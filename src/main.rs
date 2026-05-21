use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use image::{GenericImageView, ImageFormat};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const EXTERNAL_TOOLS: &[ExternalTool] = &[
    ExternalTool {
        name: "pdfimages",
        purpose: "extract embedded PDF page images at native resolution",
    },
    ExternalTool {
        name: "pdftotext",
        purpose: "locate table pages by text anchors",
    },
];

#[derive(Clone, Copy, Debug)]
struct ExternalTool {
    name: &'static str,
    purpose: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum CropKind {
    Votes,
}

impl CropKind {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "votes" | "2.2" => Ok(Self::Votes),
            _ => err(format!(
                "unknown crop kind {value:?}; only votes / 2.2 is supported for now"
            )),
        }
    }

    fn filename_part(self) -> &'static str {
        match self {
            Self::Votes => "2.2",
        }
    }

    fn directory_name(self) -> &'static str {
        match self {
            Self::Votes => "2.2",
        }
    }

    fn anchor_matches(self, page_text: &str) -> bool {
        let lower = page_text.to_lowercase();
        match self {
            Self::Votes => {
                lower.contains("2.2")
                    && lower.contains("uitgebrachte stemmen")
                    && lower.contains("totaal lijst")
            }
        }
    }

    fn full_table_template(self) -> CropTemplate {
        match self {
            Self::Votes => CropTemplate {
                x: 0.0550,
                y: 0.1200,
                width: 0.9100,
                height: 0.7200,
            },
        }
    }

    fn narrow_from_full_table_template(self) -> CropTemplate {
        match self {
            Self::Votes => CropTemplate {
                x: 0.0200,
                y: 0.1150,
                width: 0.1725,
                height: 0.8650,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct CropTemplate {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Debug)]
struct CropOptions {
    election: String,
    municipality: String,
    pdf: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    page_override: Option<u32>,
    keep_page_images: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("doctor") => doctor_command(&args[1..]),
        Some("crop") => crop_command(&args[1..]),
        Some("-h" | "--help") | None => {
            print_help();
            Ok(())
        }
        Some(command) => err(format!("unknown command {command:?}\n\n{}", help_text())),
    }
}

fn doctor_command(args: &[String]) -> Result<()> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_doctor_help();
        return Ok(());
    }
    if !args.is_empty() {
        return err(format!(
            "doctor does not take arguments\n\n{}",
            doctor_help_text()
        ));
    }

    let reports = check_tools(EXTERNAL_TOOLS);
    for report in &reports {
        match &report.status {
            ToolStatus::Usable { version } => {
                if let Some(version) = version {
                    println!(
                        "ok: {} ({version}) - {}",
                        report.tool.name, report.tool.purpose
                    );
                } else {
                    println!("ok: {} - {}", report.tool.name, report.tool.purpose);
                }
            }
            ToolStatus::Missing { error } => {
                println!(
                    "missing: {} - {} ({error})",
                    report.tool.name, report.tool.purpose
                );
            }
            ToolStatus::Broken { error } => {
                println!(
                    "broken: {} - {} ({error})",
                    report.tool.name, report.tool.purpose
                );
            }
        }
    }

    let failures: Vec<_> = reports
        .iter()
        .filter(|report| !report.status.is_usable())
        .map(|report| report.tool.name)
        .collect();
    if failures.is_empty() {
        println!("all required external tools are present");
        Ok(())
    } else {
        err(format!(
            "missing or unusable external tool(s): {}",
            failures.join(", ")
        ))
    }
}

fn crop_command(args: &[String]) -> Result<()> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_crop_help();
        return Ok(());
    }

    let options = parse_crop_args(args)?;
    ensure_tools(required_crop_tools(&options))?;
    let municipality_dir = Path::new(&options.election).join(&options.municipality);
    let pdfs = find_pdfs(&municipality_dir, options.pdf.as_deref())?;
    let out_dir = options
        .out_dir
        .clone()
        .unwrap_or_else(|| municipality_dir.join("crops"));
    fs::create_dir_all(&out_dir)?;

    for pdf in pdfs {
        crop_pdf(&pdf, &out_dir, &options)?;
    }

    Ok(())
}

fn required_crop_tools(options: &CropOptions) -> &'static [ExternalTool] {
    if options.page_override.is_some() {
        &EXTERNAL_TOOLS[..1]
    } else {
        EXTERNAL_TOOLS
    }
}

#[derive(Debug)]
struct ToolReport {
    tool: ExternalTool,
    status: ToolStatus,
}

#[derive(Debug)]
enum ToolStatus {
    Usable { version: Option<String> },
    Missing { error: String },
    Broken { error: String },
}

impl ToolStatus {
    fn is_usable(&self) -> bool {
        matches!(self, Self::Usable { .. })
    }
}

fn ensure_tools(tools: &[ExternalTool]) -> Result<()> {
    let failures: Vec<_> = check_tools(tools)
        .into_iter()
        .filter(|report| !report.status.is_usable())
        .map(|report| report.tool.name)
        .collect();

    if failures.is_empty() {
        Ok(())
    } else {
        err(format!(
            "missing or unusable external tool(s): {}. Run `cargo run -- doctor` for details.",
            failures.join(", ")
        ))
    }
}

fn check_tools(tools: &[ExternalTool]) -> Vec<ToolReport> {
    tools.iter().copied().map(check_tool).collect()
}

fn check_tool(tool: ExternalTool) -> ToolReport {
    let status = match Command::new(tool.name).arg("-v").output() {
        Ok(output) if output.status.success() => ToolStatus::Usable {
            version: command_version(&output.stdout, &output.stderr),
        },
        Ok(output) => ToolStatus::Broken {
            error: command_output_summary(&output.stdout, &output.stderr)
                .unwrap_or_else(|| format!("exited with {}", output.status)),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => ToolStatus::Missing {
            error: "not found on PATH".to_owned(),
        },
        Err(error) => ToolStatus::Broken {
            error: error.to_string(),
        },
    };

    ToolReport { tool, status }
}

fn command_version(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    command_output_summary(stdout, stderr).map(|line| {
        line.strip_prefix("pdfimages ")
            .or_else(|| line.strip_prefix("pdftotext "))
            .unwrap_or(&line)
            .to_owned()
    })
}

fn command_output_summary(stdout: &[u8], stderr: &[u8]) -> Option<String> {
    String::from_utf8_lossy(stdout)
        .lines()
        .chain(String::from_utf8_lossy(stderr).lines())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_owned)
}

fn parse_crop_args(args: &[String]) -> Result<CropOptions> {
    let mut positionals = Vec::new();
    let mut pdf = None;
    let mut out_dir = None;
    let mut page_override = None;
    let mut keep_page_images = false;

    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--pdf" => {
                index += 1;
                pdf = Some(PathBuf::from(require_arg(args, index, "--pdf")?));
            }
            "--out-dir" => {
                index += 1;
                out_dir = Some(PathBuf::from(require_arg(args, index, "--out-dir")?));
            }
            "--kind" => {
                index += 1;
                let _ = CropKind::parse(require_arg(args, index, "--kind")?)?;
            }
            "--page" => {
                index += 1;
                page_override = Some(require_arg(args, index, "--page")?.parse()?);
            }
            "--keep-page-images" => {
                keep_page_images = true;
            }
            value if value.starts_with("--") => {
                return err(format!("unknown option {value:?}"));
            }
            value => positionals.push(value.to_owned()),
        }
        index += 1;
    }

    if positionals.len() != 2 {
        return err(format!(
            "crop expects <election> <municipality>\n\n{}",
            crop_help_text()
        ));
    }

    let election = normalize_election(&positionals[0]);
    Ok(CropOptions {
        election,
        municipality: positionals[1].clone(),
        pdf,
        out_dir,
        page_override,
        keep_page_images,
    })
}

fn require_arg<'a>(args: &'a [String], index: usize, option: &str) -> Result<&'a str> {
    args.get(index).map(String::as_str).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{option} needs a value"),
        )
        .into()
    })
}

fn normalize_election(value: &str) -> String {
    if value.chars().all(|ch| ch.is_ascii_digit()) {
        format!("{value}-GR")
    } else {
        value.to_owned()
    }
}

fn find_pdfs(municipality_dir: &Path, pdf_arg: Option<&Path>) -> Result<Vec<PathBuf>> {
    if let Some(pdf_arg) = pdf_arg {
        let pdf = if pdf_arg.exists() {
            pdf_arg.to_path_buf()
        } else {
            municipality_dir.join(pdf_arg)
        };
        if !pdf.exists() {
            return err(format!("PDF does not exist: {}", pdf.display()));
        }
        return Ok(vec![pdf]);
    }

    let mut pdfs = Vec::new();
    for entry in fs::read_dir(municipality_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        if file_name.ends_with(".pdf") && file_name.contains("_eerste_telling") {
            pdfs.push(path);
        }
    }
    pdfs.sort();

    if pdfs.is_empty() {
        return err(format!(
            "no *_eerste_telling.pdf files found in {}",
            municipality_dir.display()
        ));
    }
    Ok(pdfs)
}

fn crop_pdf(pdf: &Path, out_dir: &Path, options: &CropOptions) -> Result<()> {
    let kind = CropKind::Votes;
    let full_table_path = full_table_output_path_for(pdf, out_dir, kind)?;
    let narrow_path = narrow_output_path_for(pdf, out_dir, kind)?;
    ensure_output_absent(&full_table_path)?;
    ensure_output_absent(&narrow_path)?;

    let page_by_kind = if options.page_override.is_some() {
        BTreeMap::new()
    } else {
        locate_section_pages(pdf, &[kind])?
    };

    let page = options
        .page_override
        .or_else(|| page_by_kind.get(&kind).copied())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "could not locate {} page in {}",
                    kind.filename_part(),
                    pdf.display()
                ),
            )
        })?;

    let extracted = extract_native_page_image(pdf, page, out_dir, options.keep_page_images)?;
    let full_crop = write_crop(
        &extracted.image_path,
        &full_table_path,
        kind.full_table_template(),
    )?;

    println!(
        "{} page {} {} -> {} ({}x{} at {},{} from {}x{})",
        pdf.display(),
        page,
        kind.filename_part(),
        full_table_path.display(),
        full_crop.width,
        full_crop.height,
        full_crop.x,
        full_crop.y,
        full_crop.source_width,
        full_crop.source_height
    );

    let narrow_crop = write_crop(
        &full_table_path,
        &narrow_path,
        kind.narrow_from_full_table_template(),
    )?;
    println!(
        "{} {} narrow -> {} ({}x{} at {},{} from {}x{})",
        pdf.display(),
        kind.filename_part(),
        narrow_path.display(),
        narrow_crop.width,
        narrow_crop.height,
        narrow_crop.x,
        narrow_crop.y,
        narrow_crop.source_width,
        narrow_crop.source_height
    );

    if !options.keep_page_images {
        fs::remove_dir_all(extracted.temp_dir)?;
    }

    Ok(())
}

fn ensure_output_absent(path: &Path) -> Result<()> {
    if path.exists() {
        return err(format!(
            "output already exists: {}. Remove existing crops before rerunning.",
            path.display()
        ));
    }
    Ok(())
}

fn locate_section_pages(pdf: &Path, kinds: &[CropKind]) -> Result<BTreeMap<CropKind, u32>> {
    let output = Command::new("pdftotext")
        .arg("-layout")
        .arg(pdf)
        .arg("-")
        .output()?;
    if !output.status.success() {
        return err(format!(
            "pdftotext failed for {}: {}",
            pdf.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut pages = BTreeMap::new();
    for (index, page_text) in text.split('\x0c').enumerate() {
        let page_number = index as u32 + 1;
        for kind in kinds {
            if !pages.contains_key(kind) && kind.anchor_matches(page_text) {
                pages.insert(*kind, page_number);
            }
        }
    }
    if kinds.contains(&CropKind::Votes) && !pages.contains_key(&CropKind::Votes) {
        for (index, page_text) in text.split('\x0c').enumerate() {
            let lower = page_text.to_lowercase();
            if lower.contains("2.1")
                && lower.contains("toegelaten kiezers")
                && lower.contains("tel het aantal geldige stempassen")
            {
                pages.insert(CropKind::Votes, index as u32 + 2);
                break;
            }
        }
    }
    Ok(pages)
}

struct ExtractedImage {
    temp_dir: PathBuf,
    image_path: PathBuf,
}

fn extract_native_page_image(
    pdf: &Path,
    page: u32,
    out_dir: &Path,
    keep_page_image: bool,
) -> Result<ExtractedImage> {
    let temp_dir = if keep_page_image {
        let stem = pdf.file_stem().and_then(OsStr::to_str).unwrap_or("page");
        out_dir
            .join("_native_pages")
            .join(format!("{}-page-{page}", safe_file_part(stem)))
    } else {
        env::temp_dir().join(format!(
            "pv-crop-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ))
    };
    fs::create_dir_all(&temp_dir)?;

    let root = temp_dir.join("page");
    let output = Command::new("pdfimages")
        .arg("-f")
        .arg(page.to_string())
        .arg("-l")
        .arg(page.to_string())
        .arg("-png")
        .arg(pdf)
        .arg(&root)
        .output()?;
    if !output.status.success() {
        return err(format!(
            "pdfimages failed for {} page {}: {}",
            pdf.display(),
            page,
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let image_path = largest_image_in(&temp_dir)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "pdfimages did not extract a page image for {} page {}; refusing to rasterize because native resolution was requested",
                pdf.display(),
                page
            ),
        )
    })?;

    Ok(ExtractedImage {
        temp_dir,
        image_path,
    })
}

fn largest_image_in(dir: &Path) -> Result<Option<PathBuf>> {
    let mut largest = None;
    let mut largest_area = 0_u64;

    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(extension) = path.extension().and_then(OsStr::to_str) else {
            continue;
        };
        if !matches!(extension, "png" | "jpg" | "jpeg" | "ppm" | "pbm") {
            continue;
        }
        let (width, height) = image::image_dimensions(&path)?;
        let area = width as u64 * height as u64;
        if area > largest_area {
            largest_area = area;
            largest = Some(path);
        }
    }

    Ok(largest)
}

fn full_table_output_path_for(pdf: &Path, out_dir: &Path, kind: CropKind) -> Result<PathBuf> {
    let stem = pdf.file_stem().and_then(OsStr::to_str).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "PDF path has no valid file stem",
        )
    })?;
    Ok(out_dir
        .join(kind.directory_name())
        .join(format!("{stem}.png")))
}

fn narrow_output_path_for(pdf: &Path, out_dir: &Path, kind: CropKind) -> Result<PathBuf> {
    let stem = pdf.file_stem().and_then(OsStr::to_str).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "PDF path has no valid file stem",
        )
    })?;
    Ok(out_dir
        .join(kind.directory_name())
        .join("narrow")
        .join(format!("{stem}.png")))
}

#[derive(Debug)]
struct WrittenCrop {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    source_width: u32,
    source_height: u32,
}

fn write_crop(
    image_path: &Path,
    output_path: &Path,
    template: CropTemplate,
) -> Result<WrittenCrop> {
    let image = image::open(image_path)?;
    let (source_width, source_height) = image.dimensions();
    let x = scaled(template.x, source_width);
    let y = scaled(template.y, source_height);
    let requested_width = scaled(template.width, source_width);
    let requested_height = scaled(template.height, source_height);
    let width = requested_width.min(source_width.saturating_sub(x));
    let height = requested_height.min(source_height.saturating_sub(y));
    if width == 0 || height == 0 {
        return err(format!(
            "computed empty crop for {} from {}x{}",
            image_path.display(),
            source_width,
            source_height
        ));
    }

    let crop = image.crop_imm(x, y, width, height);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    crop.save_with_format(output_path, ImageFormat::Png)?;

    Ok(WrittenCrop {
        x,
        y,
        width,
        height,
        source_width,
        source_height,
    })
}

fn scaled(value: f32, limit: u32) -> u32 {
    ((value * limit as f32).round() as u32).min(limit)
}

fn safe_file_part(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn print_help() {
    println!("{}", help_text());
}

fn help_text() -> &'static str {
    "Usage:
  pv doctor
  pv crop <election|year> <municipality> [options]

Commands:
  doctor  Check required external tools
  crop    Crop Utrecht-style table 2.2 regions at native page resolution

Run `pv crop --help` for crop options."
}

fn print_doctor_help() {
    println!("{}", doctor_help_text());
}

fn doctor_help_text() -> &'static str {
    "Usage:
  pv doctor

Checks whether external tools required by the Rust utilities are available on
PATH. The current crop workflow requires Poppler's pdfimages and pdftotext."
}

fn print_crop_help() {
    println!("{}", crop_help_text());
}

fn crop_help_text() -> &'static str {
    "Usage:
  pv crop <election|year> <municipality> [options]

Examples:
  cargo run -- crop 2026 0344 --pdf Utrecht_40_Speeltuin_Noordsepark_GR26_eerste_telling.pdf
  cargo run -- crop 2026-GR 0344

Options:
  --pdf <path>              Crop one PDF. Relative filenames are resolved inside the municipality directory.
                            Without this option, all *_eerste_telling.pdf files are cropped.
  --out-dir <dir>           Output directory. Default: <election>/<municipality>/crops.
  --kind <kind>             Compatibility option. Only votes / 2.2 is supported for now.
  --page <number>           Override automatic section page detection.
  --keep-page-images        Keep extracted native page images under <out-dir>/_native_pages.

The crop command currently writes lossless PNG crops for table 2.2
\"Uitgebrachte stemmen\" under <election>/<municipality>/crops/2.2/ and narrow
OCR-focused crops under <election>/<municipality>/crops/2.2/narrow/. The narrow
crop contains the handwritten number cells and identifier column, including
total rows E through H. The command uses pdftotext to locate the table page,
then extracts the embedded page image with pdfimages and crops those native
pixels directly. It does not use pdftoppm page rendering, so the crop keeps the
original scan resolution. Existing output files are treated as an error."
}

fn err<T>(message: impl Into<String>) -> Result<T> {
    Err(io::Error::new(io::ErrorKind::Other, message.into()).into())
}
