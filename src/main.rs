use std::collections::BTreeMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::Engine;
use image::{GenericImageView, ImageFormat};
use serde_json::json;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const DEFAULT_LLM_ENDPOINT: &str = "http://127.0.0.1:8089/v1/chat/completions";
const DEFAULT_LLM_MODEL: &str = "local";
const DEFAULT_OCR_PROMPT_PATH: &str = "prompts/ocr-votes.md";

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

#[derive(Debug)]
struct OcrVotesOptions {
    election: String,
    municipality: String,
    input_dir: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    prompt: PathBuf,
    endpoint: String,
    model: String,
    images: Vec<String>,
    force: bool,
    max_tokens: u32,
    timeout: Duration,
}

#[derive(Debug)]
struct ImageOcrReport {
    stem: String,
    output_path: PathBuf,
    action: OcrAction,
    validation: ValidationReport,
}

#[derive(Debug)]
enum OcrAction {
    Generated,
    Existing,
    FailedToGenerate(String),
}

#[derive(Debug)]
struct ValidationReport {
    passed: bool,
    errors: Vec<String>,
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
        Some("ocr-votes") => ocr_votes_command(&args[1..]),
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

fn ocr_votes_command(args: &[String]) -> Result<()> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_ocr_votes_help();
        return Ok(());
    }

    let options = parse_ocr_votes_args(args)?;
    let prompt = fs::read_to_string(&options.prompt)?;
    let municipality_dir = Path::new(&options.election).join(&options.municipality);
    let input_dir = options
        .input_dir
        .clone()
        .unwrap_or_else(|| municipality_dir.join("crops").join("2.2").join("narrow"));
    let out_dir = options
        .out_dir
        .clone()
        .unwrap_or_else(|| municipality_dir.join("results"));
    let images = find_ocr_images(&input_dir, &options.images)?;
    fs::create_dir_all(&out_dir)?;

    let mut reports = Vec::new();
    let total_images = images.len();
    for (index, image_path) in images.into_iter().enumerate() {
        let stem = file_stem_string(&image_path)?;
        println!("processing {}/{} {}", index + 1, total_images, stem);
        io::stdout().flush()?;
        let output_path = out_dir.join(format!("{stem}.md"));
        let report = process_ocr_image(&image_path, &output_path, &prompt, &options)?;
        println!(
            "{} {} -> {}",
            match &report.action {
                OcrAction::Generated => "generated",
                OcrAction::Existing => "existing",
                OcrAction::FailedToGenerate(_) => "failed",
            },
            report.stem,
            report.output_path.display()
        );
        io::stdout().flush()?;
        reports.push(report);
    }

    print_ocr_votes_report(&reports);

    if reports.iter().any(|report| {
        matches!(report.action, OcrAction::FailedToGenerate(_)) || !report.validation.passed
    }) {
        err("one or more voting-location OCR results failed validation")
    } else {
        Ok(())
    }
}

fn process_ocr_image(
    image_path: &Path,
    output_path: &Path,
    prompt: &str,
    options: &OcrVotesOptions,
) -> Result<ImageOcrReport> {
    let stem = file_stem_string(image_path)?;
    let (action, markdown) = if output_path.exists() && !options.force {
        (OcrAction::Existing, fs::read_to_string(output_path)?)
    } else {
        match call_llm_for_image(image_path, prompt, options) {
            Ok(markdown) => {
                fs::write(output_path, &markdown)?;
                (OcrAction::Generated, markdown)
            }
            Err(error) => {
                let message = error.to_string();
                return Ok(ImageOcrReport {
                    stem,
                    output_path: output_path.to_path_buf(),
                    action: OcrAction::FailedToGenerate(message.clone()),
                    validation: ValidationReport {
                        passed: false,
                        errors: vec![message],
                    },
                });
            }
        }
    };

    let validation = validate_votes_markdown(&markdown);
    Ok(ImageOcrReport {
        stem,
        output_path: output_path.to_path_buf(),
        action,
        validation,
    })
}

fn find_ocr_images(input_dir: &Path, requested: &[String]) -> Result<Vec<PathBuf>> {
    let images = if requested.is_empty() {
        let mut images = Vec::new();
        for entry in fs::read_dir(input_dir)? {
            let path = entry?.path();
            if path.extension().and_then(OsStr::to_str) == Some("png") {
                images.push(path);
            }
        }
        images.sort();
        images
    } else {
        let mut images = Vec::new();
        for request in requested {
            images.push(resolve_ocr_image(input_dir, request)?);
        }
        images
    };

    if images.is_empty() {
        return err(format!(
            "no .png narrow crops found in {}",
            input_dir.display()
        ));
    }
    Ok(images)
}

fn resolve_ocr_image(input_dir: &Path, request: &str) -> Result<PathBuf> {
    let requested_path = PathBuf::from(request);
    let candidates = if requested_path.exists() {
        vec![requested_path]
    } else if requested_path.extension().is_some() {
        vec![input_dir.join(requested_path)]
    } else {
        vec![
            input_dir.join(format!("{request}.png")),
            input_dir.join(request),
        ]
    };

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    err(format!(
        "could not find requested narrow crop {request:?} in {}",
        input_dir.display()
    ))
}

fn call_llm_for_image(
    image_path: &Path,
    prompt: &str,
    options: &OcrVotesOptions,
) -> Result<String> {
    let image = fs::read(image_path)?;
    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(image)
    );
    let body = json!({
        "model": options.model,
        "messages": [{
            "role": "user",
            "content": [
                { "type": "text", "text": prompt },
                { "type": "image_url", "image_url": { "url": data_url } }
            ]
        }],
        "temperature": 0,
        "max_tokens": options.max_tokens,
    });

    let response = http_post_json(&options.endpoint, &body.to_string(), options.timeout)?;
    let json: serde_json::Value = serde_json::from_str(&response)?;
    let content = json
        .pointer("/choices/0/message/content")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "LLM response did not contain choices[0].message.content",
            )
        })?
        .trim();
    if content.is_empty() {
        return err("LLM returned empty content");
    }
    Ok(format!("{content}\n"))
}

fn http_post_json(endpoint: &str, body: &str, timeout: Duration) -> Result<String> {
    let endpoint = parse_http_endpoint(endpoint)?;
    let mut stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port))?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;

    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        endpoint.path,
        endpoint.host_header,
        body.len(),
        body
    );
    stream.write_all(request.as_bytes())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response)?;
    let response = String::from_utf8(response)?;
    let (headers, body) = response.split_once("\r\n\r\n").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "HTTP response did not include headers",
        )
    })?;
    let status_line = headers.lines().next().unwrap_or_default();
    if !status_line.contains(" 200 ") {
        return err(format!(
            "LLM endpoint returned {status_line}: {}",
            body.chars().take(1000).collect::<String>()
        ));
    }
    if headers
        .lines()
        .any(|line| line.eq_ignore_ascii_case("transfer-encoding: chunked"))
    {
        decode_chunked_body(body)
    } else {
        Ok(body.to_owned())
    }
}

struct HttpEndpoint {
    host: String,
    host_header: String,
    port: u16,
    path: String,
}

fn parse_http_endpoint(endpoint: &str) -> Result<HttpEndpoint> {
    let Some(rest) = endpoint.strip_prefix("http://") else {
        return err("only http:// LLM endpoints are supported");
    };
    let (authority, path) = rest
        .split_once('/')
        .map(|(authority, path)| (authority, format!("/{path}")))
        .unwrap_or((rest, "/".to_owned()));
    let (host, port) = if let Some((host, port)) = authority.rsplit_once(':') {
        (host.to_owned(), port.parse::<u16>()?)
    } else {
        (authority.to_owned(), 80)
    };
    if host.is_empty() {
        return err(format!("invalid LLM endpoint {endpoint:?}"));
    }
    let host_header = if port == 80 {
        host.clone()
    } else {
        format!("{host}:{port}")
    };
    Ok(HttpEndpoint {
        host,
        host_header,
        port,
        path,
    })
}

fn decode_chunked_body(body: &str) -> Result<String> {
    let mut rest = body;
    let mut decoded = String::new();
    loop {
        let Some((size_hex, after_size)) = rest.split_once("\r\n") else {
            return err("invalid chunked HTTP response");
        };
        let size = usize::from_str_radix(size_hex.trim(), 16)?;
        if size == 0 {
            return Ok(decoded);
        }
        if after_size.len() < size + 2 {
            return err("truncated chunked HTTP response");
        }
        decoded.push_str(&after_size[..size]);
        rest = &after_size[size + 2..];
    }
}

fn validate_votes_markdown(markdown: &str) -> ValidationReport {
    let mut errors = Vec::new();
    let lines: Vec<_> = markdown
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() != 26 {
        errors.push(format!(
            "expected 26 non-empty table lines, found {}",
            lines.len()
        ));
    }
    if lines
        .iter()
        .any(|line| !line.starts_with('|') || !line.ends_with('|'))
    {
        errors.push("all non-empty lines must be Markdown table rows".to_owned());
    }

    let header = lines.first().copied().unwrap_or_default();
    let header_cells = markdown_cells(header);
    if header_cells != ["ID", "Value"] {
        errors.push(format!(
            "expected header `| ID | Value |`, found {header:?}"
        ));
    }
    if lines.get(1).is_none_or(|line| !is_markdown_separator(line)) {
        errors.push("expected Markdown separator row after header".to_owned());
    }

    let expected_ids: Vec<String> = (1..=20)
        .map(|index| format!("E.{index}"))
        .chain(["E", "F", "G", "H"].into_iter().map(str::to_owned))
        .collect();
    let mut values = BTreeMap::new();
    for (index, expected_id) in expected_ids.iter().enumerate() {
        let line_number = index + 3;
        let Some(line) = lines.get(index + 2) else {
            errors.push(format!("missing row {line_number} for {expected_id}"));
            continue;
        };
        let cells = markdown_cells(line);
        if cells.len() != 2 {
            errors.push(format!(
                "row {line_number} should have exactly 2 cells, found {}",
                cells.len()
            ));
            continue;
        }
        if cells[0] != *expected_id {
            errors.push(format!(
                "row {line_number} expected ID {expected_id}, found {}",
                cells[0]
            ));
        }
        if !cells[1].chars().all(|ch| ch.is_ascii_digit()) {
            errors.push(format!(
                "row {line_number} value for {} is not digits only: {}",
                cells[0], cells[1]
            ));
            continue;
        }
        match cells[1].parse::<u32>() {
            Ok(value) => {
                values.insert(cells[0].clone(), value);
            }
            Err(error) => {
                errors.push(format!(
                    "row {line_number} value for {} could not be parsed: {error}",
                    cells[0]
                ));
            }
        }
    }

    let candidate_sum: Option<u32> = (1..=20)
        .map(|index| values.get(&format!("E.{index}")).copied())
        .sum();
    if let (Some(candidate_sum), Some(e_value)) = (candidate_sum, values.get("E")) {
        if candidate_sum != *e_value {
            errors.push(format!(
                "E.1 through E.20 sum to {candidate_sum}, but E is {e_value}"
            ));
        }
    }
    if let (Some(e), Some(f), Some(g), Some(h)) = (
        values.get("E"),
        values.get("F"),
        values.get("G"),
        values.get("H"),
    ) {
        let total = e + f + g;
        if total != *h {
            errors.push(format!("E + F + G is {total}, but H is {h}"));
        }
    }

    ValidationReport {
        passed: errors.is_empty(),
        errors,
    }
}

fn markdown_cells(line: &str) -> Vec<String> {
    let mut cells: Vec<_> = line.split('|').collect();
    if line.starts_with('|') && !cells.is_empty() {
        cells.remove(0);
    }
    if line.ends_with('|') && !cells.is_empty() {
        cells.pop();
    }
    cells
        .into_iter()
        .map(str::trim)
        .map(str::to_owned)
        .collect()
}

fn is_markdown_separator(line: &str) -> bool {
    let cells = markdown_cells(line);
    cells.len() == 2
        && cells.iter().all(|cell| {
            let stripped = cell.trim_matches(':');
            stripped.len() >= 3 && stripped.chars().all(|ch| ch == '-')
        })
}

fn print_ocr_votes_report(reports: &[ImageOcrReport]) {
    println!();
    println!("Voting location OCR results:");
    println!("| Status | Action | Voting location | Details |");
    println!("|---|---|---|---|");
    for report in reports {
        let status = if report.validation.passed {
            "PASS"
        } else {
            "FAIL"
        };
        let action = match &report.action {
            OcrAction::Generated => "generated".to_owned(),
            OcrAction::Existing => "existing".to_owned(),
            OcrAction::FailedToGenerate(_) => "failed".to_owned(),
        };
        let details = match &report.action {
            OcrAction::FailedToGenerate(error) => escape_markdown_table_cell(error),
            _ if report.validation.errors.is_empty() => report.output_path.display().to_string(),
            _ => escape_markdown_table_cell(&report.validation.errors.join("; ")),
        };
        println!(
            "| {status} | {action} | {} | {details} |",
            escape_markdown_table_cell(&report.stem)
        );
    }
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

fn parse_ocr_votes_args(args: &[String]) -> Result<OcrVotesOptions> {
    let mut positionals = Vec::new();
    let mut input_dir = None;
    let mut out_dir = None;
    let mut prompt = PathBuf::from(DEFAULT_OCR_PROMPT_PATH);
    let mut endpoint =
        env::var("PV_LLM_ENDPOINT").unwrap_or_else(|_| DEFAULT_LLM_ENDPOINT.to_owned());
    let mut model = env::var("PV_LLM_MODEL").unwrap_or_else(|_| DEFAULT_LLM_MODEL.to_owned());
    let mut images = Vec::new();
    let mut force = false;
    let mut max_tokens = 4096;
    let mut timeout = Duration::from_secs(300);

    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--input-dir" => {
                index += 1;
                input_dir = Some(PathBuf::from(require_arg(args, index, "--input-dir")?));
            }
            "--out-dir" => {
                index += 1;
                out_dir = Some(PathBuf::from(require_arg(args, index, "--out-dir")?));
            }
            "--prompt" => {
                index += 1;
                prompt = PathBuf::from(require_arg(args, index, "--prompt")?);
            }
            "--endpoint" => {
                index += 1;
                endpoint = require_arg(args, index, "--endpoint")?.to_owned();
            }
            "--model" => {
                index += 1;
                model = require_arg(args, index, "--model")?.to_owned();
            }
            "--image" => {
                index += 1;
                images.push(require_arg(args, index, "--image")?.to_owned());
            }
            "--force" => {
                force = true;
            }
            "--max-tokens" => {
                index += 1;
                max_tokens = require_arg(args, index, "--max-tokens")?.parse()?;
            }
            "--timeout-seconds" => {
                index += 1;
                let seconds: u64 = require_arg(args, index, "--timeout-seconds")?.parse()?;
                timeout = Duration::from_secs(seconds);
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
            "ocr-votes expects <election> <municipality>\n\n{}",
            ocr_votes_help_text()
        ));
    }
    if max_tokens == 0 {
        return err("--max-tokens must be greater than zero");
    }

    Ok(OcrVotesOptions {
        election: normalize_election(&positionals[0]),
        municipality: positionals[1].clone(),
        input_dir,
        out_dir,
        prompt,
        endpoint,
        model,
        images,
        force,
        max_tokens,
        timeout,
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

fn file_stem_string(path: &Path) -> Result<String> {
    path.file_stem()
        .and_then(OsStr::to_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("path has no valid file stem: {}", path.display()),
            )
            .into()
        })
}

fn escape_markdown_table_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn print_help() {
    println!("{}", help_text());
}

fn help_text() -> &'static str {
    "Usage:
  pv doctor
  pv crop <election|year> <municipality> [options]
  pv ocr-votes <election|year> <municipality> [options]

Commands:
  doctor     Check required external tools
  crop       Crop Utrecht-style table 2.2 regions at native page resolution
  ocr-votes  Run narrow table 2.2 crops through a local multimodal LLM

Run `pv crop --help` or `pv ocr-votes --help` for command options."
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

fn print_ocr_votes_help() {
    println!("{}", ocr_votes_help_text());
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

fn ocr_votes_help_text() -> &'static str {
    "Usage:
  pv ocr-votes <election|year> <municipality> [options]

Examples:
  cargo run -- ocr-votes 2026-GR 0344
  cargo run -- ocr-votes 2026 0344 --image Utrecht_41_Buurtcentrum_De_Uithoek_GR26_eerste_telling --force

Options:
  --input-dir <dir>         Narrow crop directory. Default: <election>/<municipality>/crops/2.2/narrow.
  --out-dir <dir>           Markdown output directory. Default: <election>/<municipality>/results.
  --prompt <path>           Prompt markdown file. Default: prompts/ocr-votes.md.
  --endpoint <url>          OpenAI-compatible local chat endpoint. Default: http://127.0.0.1:8089/v1/chat/completions.
                            Can also be set with PV_LLM_ENDPOINT.
  --model <model>           Model name sent to the endpoint. Default: local.
                            Can also be set with PV_LLM_MODEL.
  --image <stem-or-path>    Process only one narrow crop. Repeat to process several specific voting locations.
                            Stems are resolved under --input-dir with a .png suffix.
  --force                   Re-run the LLM and overwrite existing Markdown outputs.
  --max-tokens <n>          Maximum completion tokens. Default: 4096.
  --timeout-seconds <n>     Socket read/write timeout for each LLM request. Default: 300.

The ocr-votes command is meant to run after `pv crop`. It sends each narrow
PNG crop to a local multimodal LLM in a fresh chat-completion request, writes
one Markdown table per voting location, then validates the Markdown structure
and arithmetic. Existing Markdown files are validated without rerunning the LLM
unless --force is used. The final report lists PASS/FAIL per voting location."
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_votes_table() -> String {
        let mut lines = vec!["| ID | Value |".to_owned(), "|---|---|".to_owned()];
        for index in 1..=20 {
            lines.push(format!("| E.{index} | 1 |"));
        }
        lines.extend([
            "| E | 20 |".to_owned(),
            "| F | 2 |".to_owned(),
            "| G | 3 |".to_owned(),
            "| H | 25 |".to_owned(),
        ]);
        lines.join("\n")
    }

    #[test]
    fn validates_expected_votes_table() {
        let report = validate_votes_markdown(&valid_votes_table());
        assert!(report.passed, "{:?}", report.errors);
    }

    #[test]
    fn rejects_bad_candidate_sum() {
        let table = valid_votes_table().replace("| E | 20 |", "| E | 21 |");
        let report = validate_votes_markdown(&table);
        assert!(!report.passed);
        assert!(
            report
                .errors
                .iter()
                .any(|error| error.contains("E.1 through E.20 sum"))
        );
    }

    #[test]
    fn rejects_unexpected_row_order() {
        let table = valid_votes_table().replace("| E.10 | 1 |", "| E.99 | 1 |");
        let report = validate_votes_markdown(&table);
        assert!(!report.passed);
        assert!(
            report
                .errors
                .iter()
                .any(|error| error.contains("expected ID E.10"))
        );
    }
}

fn err<T>(message: impl Into<String>) -> Result<T> {
    Err(io::Error::new(io::ErrorKind::Other, message.into()).into())
}
