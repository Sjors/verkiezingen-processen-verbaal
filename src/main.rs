use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::Engine;
use image::{GenericImageView, ImageFormat, RgbaImage};
use serde_json::json;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const DEFAULT_LLM_ENDPOINT: &str = "http://127.0.0.1:8089/v1/chat/completions";
const DEFAULT_LLM_MODEL: &str = "local";
const DEFAULT_OCR_PROMPT_PATH: &str = "prompts/ocr-votes.md";
const OCR_SKIP_MARKER: &str = "<!-- pv-ocr-votes: skip -->";
const DEFAULT_CORRECTIONS_OCR_PROMPT_PATH: &str = "prompts/ocr-corrections.md";
const DEFAULT_DOWNLOAD_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15";
const UTRECHT_GSB_CSV_URL: &str =
    "https://open.utrecht.nl/sites/default/files/open-data/osv4-3-telling-gr2026-utrecht.csv";
const UTRECHT_CSB_CSV_URL: &str =
    "https://open.utrecht.nl/sites/default/files/open-data/osv4-3-telling-gr2026-utrecht_0.csv";

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
    Corrections,
}

impl CropKind {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "votes" | "2.2" => Ok(Self::Votes),
            "corrections" | "b1-2.4" | "2.4" => Ok(Self::Corrections),
            _ => err(format!(
                "unknown crop kind {value:?}; expected votes / 2.2 or corrections / b1-2.4"
            )),
        }
    }

    fn filename_part(self) -> &'static str {
        match self {
            Self::Votes => "2.2",
            Self::Corrections => "corrections",
        }
    }

    fn directory_name(self) -> &'static str {
        match self {
            Self::Votes => "2.2",
            Self::Corrections => "corrections",
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
            Self::Corrections => {
                lower.contains("b1 - 2.4")
                    && lower.contains("lijsten met verschil")
                    && lower.contains("lijsttotaal")
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
            Self::Corrections => CropTemplate {
                x: 0.0250,
                y: 0.0550,
                width: 0.9550,
                height: 0.8750,
            },
        }
    }

    fn narrow_from_full_table_template(self) -> Option<CropTemplate> {
        match self {
            Self::Votes => Some(CropTemplate {
                x: 0.0200,
                y: 0.0200,
                width: 0.1725,
                height: 0.9600,
            }),
            Self::Corrections => None,
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
    stations: BTreeSet<String>,
    out_dir: Option<PathBuf>,
    kind: CropKind,
    page_override: Option<u32>,
    keep_page_images: bool,
    force: bool,
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
    stations: BTreeSet<String>,
    force: bool,
    max_tokens: u32,
    timeout: Duration,
}

type OcrCorrectionsOptions = OcrVotesOptions;

#[derive(Debug)]
struct OfficialCsvOptions {
    election: String,
    municipality: String,
    out_dir: Option<PathBuf>,
    gsb_url: Option<String>,
    csb_url: Option<String>,
    force: bool,
}

#[derive(Debug)]
struct OfficialCsvSource {
    label: &'static str,
    file_name: &'static str,
    url: String,
}

#[derive(Debug)]
struct CompareResultsOptions {
    election: String,
    municipality: String,
    results_dir: Option<PathBuf>,
    corrections_dir: Option<PathBuf>,
    output_path: Option<PathBuf>,
    stations: BTreeSet<String>,
    format: ReportFormat,
    debug: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReportFormat {
    Terminal,
    Markdown,
}

impl ReportFormat {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "terminal" => Ok(Self::Terminal),
            "markdown" => Ok(Self::Markdown),
            _ => err(format!(
                "unknown report format {value:?}; expected terminal or markdown"
            )),
        }
    }
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
    Skipped,
    FailedToGenerate(String),
}

#[derive(Debug)]
struct ValidationReport {
    passed: bool,
    skipped: bool,
    errors: Vec<String>,
}

#[derive(Debug)]
struct OfficialResults {
    source_path: PathBuf,
    station_order: Vec<String>,
    stations: BTreeMap<String, OfficialStationResult>,
}

#[derive(Debug)]
struct OfficialStationResult {
    location: String,
    values: BTreeMap<String, u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComparisonStatus {
    Missing,
    Incomplete,
    CorrectionInconsistent,
    InternallyInconsistent,
    FullyMatches,
    Mismatch,
}

impl ComparisonStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Incomplete => "incomplete",
            Self::CorrectionInconsistent => "correction inconsistent",
            Self::InternallyInconsistent => "internally inconsistent",
            Self::FullyMatches => "fully matches",
            Self::Mismatch => "mismatch",
        }
    }
}

#[derive(Debug)]
struct ComparisonRow {
    station: String,
    location: String,
    markdown_path: Option<PathBuf>,
    correction_path: Option<PathBuf>,
    official_values: BTreeMap<String, u32>,
    status: ComparisonStatus,
    details: String,
}

#[derive(Debug)]
struct CorrectionDocument {
    path: PathBuf,
    validation: ValidationReport,
    corrections: BTreeMap<String, Correction>,
}

#[derive(Debug)]
struct Correction {
    first: Option<u32>,
    second: Option<u32>,
    difference: i32,
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
        Some("ocr-corrections") => ocr_corrections_command(&args[1..]),
        Some("official-csvs") => official_csvs_command(&args[1..]),
        Some("compare-results") => compare_results_command(&args[1..]),
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
    let pdfs = find_pdfs(
        &municipality_dir,
        options.pdf.as_deref(),
        &options.stations,
        options.kind,
    )?;
    let out_dir = options
        .out_dir
        .clone()
        .unwrap_or_else(|| municipality_dir.join("crops"));
    fs::create_dir_all(&out_dir)?;

    let total_pdfs = pdfs.len();
    let mut eta = ProgressEta::new();
    let mut failures = Vec::new();
    for (index, pdf) in pdfs.into_iter().enumerate() {
        println!("processing {}/{} {}", index + 1, total_pdfs, pdf.display());
        io::stdout().flush()?;
        if let Err(error) = crop_pdf(&pdf, &out_dir, &options) {
            println!("failed {}: {error}", pdf.display());
            io::stdout().flush()?;
            failures.push(format!("{}: {error}", pdf.display()));
        }
        eta.maybe_print(index + 1, total_pdfs)?;
    }

    if failures.is_empty() {
        Ok(())
    } else {
        err(format!(
            "failed to crop {} PDF(s): {}",
            failures.len(),
            failures.join("; ")
        ))
    }
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
    let images = find_ocr_images(&input_dir, &options.images, &options.stations)?;
    fs::create_dir_all(&out_dir)?;

    let mut reports = Vec::new();
    let total_images = images.len();
    let mut eta = ProgressEta::new();
    for (index, image_path) in images.into_iter().enumerate() {
        let stem = file_stem_string(&image_path)?;
        println!("processing {}/{} {}", index + 1, total_images, stem);
        io::stdout().flush()?;
        let output_path = out_dir.join(format!("{stem}.md"));
        let report = process_ocr_image(
            &image_path,
            &output_path,
            &prompt,
            &options,
            true,
            validate_votes_markdown,
        )?;
        println!(
            "{} {} -> {}",
            match &report.action {
                OcrAction::Generated => "generated",
                OcrAction::Existing => "existing",
                OcrAction::Skipped => "skipped",
                OcrAction::FailedToGenerate(_) => "failed",
            },
            report.stem,
            report.output_path.display()
        );
        io::stdout().flush()?;
        reports.push(report);
        eta.maybe_print(index + 1, total_images)?;
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

fn ocr_corrections_command(args: &[String]) -> Result<()> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_ocr_corrections_help();
        return Ok(());
    }

    let options = parse_ocr_corrections_args(args)?;
    let prompt = fs::read_to_string(&options.prompt)?;
    let municipality_dir = Path::new(&options.election).join(&options.municipality);
    let input_dir = options
        .input_dir
        .clone()
        .unwrap_or_else(|| municipality_dir.join("crops").join("corrections"));
    let out_dir = options
        .out_dir
        .clone()
        .unwrap_or_else(|| municipality_dir.join("results").join("corrections"));
    let images = find_ocr_images(&input_dir, &options.images, &options.stations)?;
    fs::create_dir_all(&out_dir)?;

    let mut reports = Vec::new();
    let total_images = images.len();
    let mut eta = ProgressEta::new();
    for (index, image_path) in images.into_iter().enumerate() {
        let stem = file_stem_string(&image_path)?;
        println!("processing {}/{} {}", index + 1, total_images, stem);
        io::stdout().flush()?;
        let output_path = out_dir.join(format!("{stem}.md"));
        let report = process_ocr_image(
            &image_path,
            &output_path,
            &prompt,
            &options,
            false,
            validate_corrections_markdown,
        )?;
        println!(
            "{} {} -> {}",
            match &report.action {
                OcrAction::Generated => "generated",
                OcrAction::Existing => "existing",
                OcrAction::Skipped => "skipped",
                OcrAction::FailedToGenerate(_) => "failed",
            },
            report.stem,
            report.output_path.display()
        );
        io::stdout().flush()?;
        reports.push(report);
        eta.maybe_print(index + 1, total_images)?;
    }

    print_ocr_report("Correction OCR results", &reports);

    if reports.iter().any(|report| {
        matches!(report.action, OcrAction::FailedToGenerate(_)) || !report.validation.passed
    }) {
        err("one or more correction OCR results failed validation")
    } else {
        Ok(())
    }
}

fn official_csvs_command(args: &[String]) -> Result<()> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_official_csvs_help();
        return Ok(());
    }

    let options = parse_official_csvs_args(args)?;
    let municipality_dir = Path::new(&options.election).join(&options.municipality);
    let out_dir = options
        .out_dir
        .clone()
        .unwrap_or_else(|| municipality_dir.join("results").join("official"));
    let sources = official_csv_sources(&options)?;
    fs::create_dir_all(&out_dir)?;

    for source in sources {
        let output_path = out_dir.join(source.file_name);
        if output_path.exists() && !options.force {
            println!(
                "existing {} -> {} (use --force to overwrite)",
                source.label,
                output_path.display()
            );
            continue;
        }
        let bytes = download_to_path(&source.url, &output_path)?;
        println!(
            "downloaded {} ({} bytes) -> {}",
            source.label,
            bytes,
            output_path.display()
        );
    }

    Ok(())
}

fn compare_results_command(args: &[String]) -> Result<()> {
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        print_compare_results_help();
        return Ok(());
    }

    let options = parse_compare_results_args(args)?;
    let municipality_dir = Path::new(&options.election).join(&options.municipality);
    let results_dir = options
        .results_dir
        .clone()
        .unwrap_or_else(|| municipality_dir.join("results"));
    let corrections_dir = options
        .corrections_dir
        .clone()
        .unwrap_or_else(|| municipality_dir.join("results").join("corrections"));
    debug_log(
        options.debug,
        format!("indexing Markdown files in {}", results_dir.display()),
    );
    let markdown_by_station = find_result_markdown_by_station(&results_dir)?;
    let markdown_count: usize = markdown_by_station.values().map(Vec::len).sum();
    debug_log(
        options.debug,
        format!(
            "indexed {markdown_count} Markdown files for {} station keys",
            markdown_by_station.len()
        ),
    );
    let official_csv = compare_official_csv_path(&municipality_dir);
    debug_log(
        options.debug,
        format!(
            "indexing correction OCR files in {}",
            corrections_dir.display()
        ),
    );
    let corrections_by_station = read_corrections_by_station(&corrections_dir)?;
    debug_log(
        options.debug,
        format!(
            "indexed correction OCR files for {} station keys",
            corrections_by_station.len()
        ),
    );
    debug_log(
        options.debug,
        format!("reading official CSV {}", official_csv.display()),
    );
    let official_results = read_official_results_csv(&official_csv)?;
    debug_log(
        options.debug,
        format!(
            "loaded {} official polling stations",
            official_results.station_order.len()
        ),
    );
    debug_log(options.debug, "comparing Markdown against official CSV");
    let mut rows = compare_markdown_to_official(
        &official_results,
        &markdown_by_station,
        &corrections_by_station,
        official_csv_needs_correction_reversal(&official_csv),
    );
    if !options.stations.is_empty() {
        rows.retain(|row| options.stations.contains(&row.station));
        debug_log(
            options.debug,
            format!(
                "filtered comparison to {} requested station(s)",
                options.stations.len()
            ),
        );
    }
    debug_log(
        options.debug,
        format!("computed {} comparison rows", rows.len()),
    );
    let mismatch_report_dir = write_mismatch_reports(
        &municipality_dir,
        &rows,
        options.debug,
        !options.stations.is_empty(),
    )?;
    if let Some(output_path) = &options.output_path {
        if let Some(parent) = output_path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let markdown = render_markdown_comparison_report(
            &official_results,
            &results_dir,
            &corrections_dir,
            mismatch_report_dir.as_deref(),
            &rows,
        );
        fs::write(output_path, markdown)?;
    }
    print_comparison_report(
        &official_results,
        &results_dir,
        &corrections_dir,
        mismatch_report_dir.as_deref(),
        &rows,
        options.format,
    );
    Ok(())
}

fn compare_official_csv_path(municipality_dir: &Path) -> PathBuf {
    let official_dir = municipality_dir.join("results").join("official");
    let first_count_csv = official_dir.join("first-count-tellingsbestand.csv");
    if first_count_csv.exists() {
        first_count_csv
    } else {
        official_dir.join("gsb-tellingsbestand.csv")
    }
}

fn official_csv_needs_correction_reversal(official_csv: &Path) -> bool {
    official_csv.file_name().and_then(OsStr::to_str) == Some("gsb-tellingsbestand.csv")
}

fn is_first_count_markdown(path: &PathBuf) -> bool {
    path.file_stem()
        .and_then(OsStr::to_str)
        .is_some_and(|stem| stem.contains("_eerste_telling"))
}

fn debug_log(enabled: bool, message: impl AsRef<str>) {
    if enabled {
        eprintln!("[compare-results] {}", message.as_ref());
    }
}

fn official_csv_sources(options: &OfficialCsvOptions) -> Result<Vec<OfficialCsvSource>> {
    let defaults = default_official_csv_sources(&options.election, &options.municipality);
    let gsb_url = options
        .gsb_url
        .clone()
        .or_else(|| defaults.as_ref().map(|sources| sources[0].url.clone()));
    let csb_url = options
        .csb_url
        .clone()
        .or_else(|| defaults.as_ref().map(|sources| sources[1].url.clone()));

    let (Some(gsb_url), Some(csb_url)) = (gsb_url, csb_url) else {
        return err(format!(
            "no built-in official CSV URLs for {}/{}; pass both --gsb-url and --csb-url",
            options.election, options.municipality
        ));
    };

    Ok(vec![
        OfficialCsvSource {
            label: "GSB tellingsbestand",
            file_name: "gsb-tellingsbestand.csv",
            url: gsb_url,
        },
        OfficialCsvSource {
            label: "CSB tellingsbestand",
            file_name: "csb-tellingsbestand.csv",
            url: csb_url,
        },
    ])
}

fn default_official_csv_sources(
    election: &str,
    municipality: &str,
) -> Option<Vec<OfficialCsvSource>> {
    if election == "2026-GR" && municipality == "0344" {
        Some(vec![
            OfficialCsvSource {
                label: "GSB tellingsbestand",
                file_name: "gsb-tellingsbestand.csv",
                url: UTRECHT_GSB_CSV_URL.to_owned(),
            },
            OfficialCsvSource {
                label: "CSB tellingsbestand",
                file_name: "csb-tellingsbestand.csv",
                url: UTRECHT_CSB_CSV_URL.to_owned(),
            },
        ])
    } else {
        None
    }
}

fn download_to_path(url: &str, output_path: &Path) -> Result<u64> {
    let parent = output_path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("output path has no parent: {}", output_path.display()),
        )
    })?;
    fs::create_dir_all(parent)?;

    let temp_path = output_path.with_file_name(format!(
        ".{}.tmp-{}",
        output_path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("download"),
        std::process::id()
    ));
    if temp_path.exists() {
        fs::remove_file(&temp_path)?;
    }

    let output = Command::new("curl")
        .arg("--fail")
        .arg("--silent")
        .arg("--show-error")
        .arg("--location")
        .arg("--user-agent")
        .arg(DEFAULT_DOWNLOAD_USER_AGENT)
        .arg("--output")
        .arg(&temp_path)
        .arg(url)
        .output()?;
    if !output.status.success() {
        let _ = fs::remove_file(&temp_path);
        return err(format!(
            "curl failed for {url}: {}",
            command_output_summary(&output.stdout, &output.stderr)
                .unwrap_or_else(|| format!("exited with {}", output.status))
        ));
    }

    let bytes = fs::metadata(&temp_path)?.len();
    if bytes == 0 {
        let _ = fs::remove_file(&temp_path);
        return err(format!("downloaded empty file from {url}"));
    }
    fs::rename(&temp_path, output_path)?;
    Ok(bytes)
}

fn read_official_results_csv(path: &Path) -> Result<OfficialResults> {
    let content = fs::read_to_string(path)?;
    let rows = parse_semicolon_csv(&content)?;
    let header_row = rows
        .iter()
        .find(|row| row.first().is_some_and(|cell| cell == "Lijstnummer"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{} does not contain a Lijstnummer header row",
                    path.display()
                ),
            )
        })?;
    let area_row = rows
        .iter()
        .find(|row| row.first().is_some_and(|cell| cell == "Gebiednummer"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{} does not contain a Gebiednummer row", path.display()),
            )
        })?;

    let mut station_columns = Vec::new();
    for (column, station_code) in area_row.iter().enumerate().skip(5) {
        if !station_code.is_empty() {
            station_columns.push((column, station_code.clone()));
        }
    }
    if station_columns.is_empty() {
        return err(format!(
            "{} does not contain station-level result columns",
            path.display()
        ));
    }

    let mut station_order = Vec::new();
    let mut stations = BTreeMap::new();
    for (column, station_code) in &station_columns {
        station_order.push(station_code.clone());
        stations.insert(
            station_code.clone(),
            OfficialStationResult {
                location: header_row.get(*column).cloned().unwrap_or_default(),
                values: BTreeMap::new(),
            },
        );
    }

    for row in &rows {
        if let Some(id) = official_result_id_for_row(row) {
            for (column, station_code) in &station_columns {
                let value = parse_u32_cell(row.get(*column).map(String::as_str).unwrap_or(""))?;
                stations
                    .get_mut(station_code)
                    .expect("station initialized from station_columns")
                    .values
                    .insert(id.clone(), value);
            }
        }
    }

    let expected_ids = expected_vote_ids();
    for station_code in &station_order {
        let station = stations
            .get(station_code)
            .expect("station initialized from station_order");
        let missing: Vec<_> = expected_ids
            .iter()
            .filter(|id| !station.values.contains_key(*id))
            .cloned()
            .collect();
        if !missing.is_empty() {
            return err(format!(
                "{} is missing official result row(s) for station {}: {}",
                path.display(),
                station_code,
                missing.join(", ")
            ));
        }
    }

    Ok(OfficialResults {
        source_path: path.to_path_buf(),
        station_order,
        stations,
    })
}

fn parse_semicolon_csv(content: &str) -> Result<Vec<Vec<String>>> {
    content
        .lines()
        .map(parse_semicolon_csv_record)
        .collect::<Result<Vec<_>>>()
}

fn parse_semicolon_csv_record(line: &str) -> Result<Vec<String>> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(ch);
            }
        } else if ch == '"' && field.is_empty() {
            in_quotes = true;
        } else if ch == ';' {
            fields.push(field);
            field = String::new();
        } else {
            field.push(ch);
        }
    }
    if in_quotes {
        return err(format!("unterminated quoted CSV field in {line:?}"));
    }
    fields.push(field);
    if let Some(first) = fields.first_mut() {
        *first = first.trim_start_matches('\u{feff}').to_owned();
    }
    Ok(fields)
}

fn official_result_id_for_row(row: &[String]) -> Option<String> {
    if let Some(list_number) = row.first().and_then(|cell| cell.parse::<u32>().ok()) {
        if (1..=20).contains(&list_number) && row.get(2).is_some_and(|cell| cell.is_empty()) {
            return Some(format!("E.{list_number}"));
        }
    }

    let label = row.get(1).map(String::as_str)?;
    match label {
        "geldige stembiljetten" => Some("E".to_owned()),
        "blanco stembiljetten" => Some("F".to_owned()),
        "ongeldige stembiljetten" => Some("G".to_owned()),
        "aangetroffen stembiljetten" => Some("H".to_owned()),
        _ => None,
    }
}

fn parse_u32_cell(cell: &str) -> Result<u32> {
    let value = cell.trim();
    if value.is_empty() {
        return Ok(0);
    }
    if !value.chars().all(|ch| ch.is_ascii_digit()) {
        return err(format!("expected numeric CSV cell, found {value:?}"));
    }
    Ok(value.parse()?)
}

fn find_result_markdown_by_station(results_dir: &Path) -> Result<BTreeMap<String, Vec<PathBuf>>> {
    let mut markdown_by_station: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    if !results_dir.exists() {
        return Ok(markdown_by_station);
    }

    for entry in fs::read_dir(results_dir)? {
        let path = entry?.path();
        if path.extension().and_then(OsStr::to_str) != Some("md") {
            continue;
        }
        let station = station_code_from_markdown_path(&path).unwrap_or_else(|| "?".to_owned());
        markdown_by_station.entry(station).or_default().push(path);
    }
    for paths in markdown_by_station.values_mut() {
        paths.sort();
    }
    Ok(markdown_by_station)
}

fn read_corrections_by_station(
    corrections_dir: &Path,
) -> Result<BTreeMap<String, CorrectionDocument>> {
    let mut corrections_by_station = BTreeMap::new();
    if !corrections_dir.exists() {
        return Ok(corrections_by_station);
    }

    for entry in fs::read_dir(corrections_dir)? {
        let path = entry?.path();
        if path.extension().and_then(OsStr::to_str) != Some("md") {
            continue;
        }
        let station = station_code_from_markdown_path(&path).unwrap_or_else(|| "?".to_owned());
        let markdown = fs::read_to_string(&path)?;
        let mut document = parse_correction_document(&path, &markdown);
        if corrections_by_station.contains_key(&station) {
            document.validation.passed = false;
            document.validation.errors.push(format!(
                "duplicate correction OCR for station {station}: {}",
                path.display()
            ));
        }
        corrections_by_station.insert(station, document);
    }

    Ok(corrections_by_station)
}

fn parse_correction_document(path: &Path, markdown: &str) -> CorrectionDocument {
    let validation = validate_corrections_markdown(markdown);
    let corrections = parse_correction_markdown_values(markdown);
    CorrectionDocument {
        path: path.to_path_buf(),
        validation,
        corrections,
    }
}

fn station_code_from_markdown_path(path: &Path) -> Option<String> {
    station_code_from_file_name(path.file_stem()?.to_str()?).map(str::to_owned)
}

fn compare_markdown_to_official(
    official_results: &OfficialResults,
    markdown_by_station: &BTreeMap<String, Vec<PathBuf>>,
    corrections_by_station: &BTreeMap<String, CorrectionDocument>,
    reverse_corrections: bool,
) -> Vec<ComparisonRow> {
    let mut rows = Vec::new();
    let mut seen_markdown_stations = BTreeSet::new();
    for station_code in &official_results.station_order {
        let official = official_results
            .stations
            .get(station_code)
            .expect("station initialized from station_order");
        if let Some(paths) = markdown_by_station.get(station_code) {
            seen_markdown_stations.insert(station_code.clone());
            for path in paths {
                let correction = corrections_by_station.get(station_code);
                rows.push(compare_one_markdown(
                    path,
                    station_code,
                    official,
                    correction,
                    reverse_corrections && is_first_count_markdown(path),
                ));
            }
        } else {
            rows.push(ComparisonRow {
                station: station_code.clone(),
                location: official.location.clone(),
                markdown_path: None,
                correction_path: corrections_by_station
                    .get(station_code)
                    .map(|document| document.path.clone()),
                official_values: official.values.clone(),
                status: ComparisonStatus::Missing,
                details: "no Markdown result".to_owned(),
            });
        }
    }

    for (station_code, paths) in markdown_by_station {
        if seen_markdown_stations.contains(station_code) {
            continue;
        }
        for path in paths {
            rows.push(ComparisonRow {
                station: station_code.clone(),
                location: String::new(),
                markdown_path: Some(path.clone()),
                correction_path: corrections_by_station
                    .get(station_code)
                    .map(|document| document.path.clone()),
                official_values: BTreeMap::new(),
                status: ComparisonStatus::Missing,
                details: format!(
                    "station not found in official CSV: {}",
                    display_file_name(path)
                ),
            });
        }
    }

    rows
}

fn compare_one_markdown(
    path: &Path,
    station_code: &str,
    official: &OfficialStationResult,
    correction: Option<&CorrectionDocument>,
    reverse_corrections: bool,
) -> ComparisonRow {
    let correction_path = correction.map(|document| document.path.clone());
    let markdown = match fs::read_to_string(path) {
        Ok(markdown) => markdown,
        Err(error) => {
            return ComparisonRow {
                station: station_code.to_owned(),
                location: official.location.clone(),
                markdown_path: Some(path.to_path_buf()),
                correction_path,
                official_values: official.values.clone(),
                status: ComparisonStatus::InternallyInconsistent,
                details: format!("could not read Markdown: {error}"),
            };
        }
    };
    let validation = validate_votes_markdown(&markdown);
    let values = parse_votes_markdown_values(&markdown);
    let round_two_mismatches = official_mismatches(&values, &official.values);

    let (official_values, mismatches, correction_issue_details) =
        if !reverse_corrections || round_two_mismatches.is_empty() {
            (official.values.clone(), round_two_mismatches, Vec::new())
        } else {
            let Some(correction) = correction else {
                return ComparisonRow {
                    station: station_code.to_owned(),
                    location: official.location.clone(),
                    markdown_path: Some(path.to_path_buf()),
                    correction_path: None,
                    official_values: official.values.clone(),
                    status: ComparisonStatus::Incomplete,
                    details: "missing correction OCR".to_owned(),
                };
            };
            if !correction.validation.passed {
                return ComparisonRow {
                    station: station_code.to_owned(),
                    location: official.location.clone(),
                    markdown_path: Some(path.to_path_buf()),
                    correction_path,
                    official_values: official.values.clone(),
                    status: ComparisonStatus::Incomplete,
                    details: format!(
                        "invalid correction OCR: {}",
                        correction.validation.errors.join("; ")
                    ),
                };
            }
            let (official_values, correction_errors) =
                reverse_official_values(&official.values, correction);
            if !correction_errors.is_empty() {
                return ComparisonRow {
                    station: station_code.to_owned(),
                    location: official.location.clone(),
                    markdown_path: Some(path.to_path_buf()),
                    correction_path,
                    official_values,
                    status: ComparisonStatus::Incomplete,
                    details: format!(
                        "could not apply corrections: {}",
                        correction_errors.join("; ")
                    ),
                };
            }
            let mismatches = official_mismatches(&values, &official_values);
            let correction_issue_details =
                correction_inconsistency_details(&values, &official.values, correction);
            (official_values, mismatches, correction_issue_details)
        };

    if !validation.passed {
        let mut details = validation.errors;
        details.extend(correction_issue_details);
        details.extend(mismatches);
        return ComparisonRow {
            station: station_code.to_owned(),
            location: official.location.clone(),
            markdown_path: Some(path.to_path_buf()),
            correction_path,
            official_values,
            status: ComparisonStatus::InternallyInconsistent,
            details: details.join("; "),
        };
    }

    if !correction_issue_details.is_empty() {
        let mut details = correction_issue_details;
        details.extend(mismatches);
        ComparisonRow {
            station: station_code.to_owned(),
            location: official.location.clone(),
            markdown_path: Some(path.to_path_buf()),
            correction_path,
            official_values,
            status: ComparisonStatus::CorrectionInconsistent,
            details: details.join("; "),
        }
    } else if mismatches.is_empty() {
        ComparisonRow {
            station: station_code.to_owned(),
            location: official.location.clone(),
            markdown_path: Some(path.to_path_buf()),
            correction_path,
            official_values,
            status: ComparisonStatus::FullyMatches,
            details: String::new(),
        }
    } else {
        ComparisonRow {
            station: station_code.to_owned(),
            location: official.location.clone(),
            markdown_path: Some(path.to_path_buf()),
            correction_path,
            official_values,
            status: ComparisonStatus::Mismatch,
            details: mismatches.join("; "),
        }
    }
}

fn reverse_official_values(
    official_values: &BTreeMap<String, u32>,
    correction: &CorrectionDocument,
) -> (BTreeMap<String, u32>, Vec<String>) {
    let mut values = official_values.clone();
    let mut errors = Vec::new();
    for (id, correction) in &correction.corrections {
        let Some(round_two) = official_values.get(id).copied() else {
            errors.push(format!("{id} is not present in official CSV"));
            continue;
        };
        if let Some(second) = correction.second {
            if second != round_two {
                errors.push(format!("{id} second={second}, official={round_two}"));
                continue;
            }
        }
        let round_one = if let Some(first) = correction.first {
            first
        } else {
            let reversed = round_two as i64 - correction.difference as i64;
            if reversed < 0 {
                errors.push(format!("{id} reverses below zero"));
                continue;
            }
            reversed as u32
        };
        values.insert(id.clone(), round_one);
    }
    recompute_aggregate_vote_totals(&mut values);
    (values, errors)
}

fn correction_inconsistency_details(
    markdown_values: &BTreeMap<String, u32>,
    round_two_values: &BTreeMap<String, u32>,
    correction: &CorrectionDocument,
) -> Vec<String> {
    let mut details = Vec::new();
    for (id, correction_row) in &correction.corrections {
        let (Some(first), Some(markdown_value)) =
            (correction_row.first, markdown_values.get(id).copied())
        else {
            continue;
        };
        if first == markdown_value {
            continue;
        }
        let second = correction_row
            .second
            .or_else(|| round_two_values.get(id).copied());
        if second != Some(markdown_value) {
            continue;
        }

        let mut detail = format!(
            "correction {id} first={first}, Markdown={markdown_value}, second={}",
            optional_u32(second)
        );
        if let Some(second) = second {
            if let Some(candidate_id) = possible_correction_target(
                markdown_values,
                round_two_values,
                correction,
                id,
                first,
                second,
            ) {
                detail.push_str(&format!(", maybe belongs to {candidate_id}"));
            }
        }
        details.push(detail);
    }
    details
}

fn possible_correction_target(
    markdown_values: &BTreeMap<String, u32>,
    round_two_values: &BTreeMap<String, u32>,
    correction: &CorrectionDocument,
    correction_id: &str,
    first: u32,
    second: u32,
) -> Option<String> {
    expected_vote_ids().into_iter().find(|candidate_id| {
        candidate_id != correction_id
            && !correction.corrections.contains_key(candidate_id)
            && markdown_values.get(candidate_id).copied() == Some(first)
            && round_two_values.get(candidate_id).copied() == Some(second)
    })
}

fn recompute_aggregate_vote_totals(values: &mut BTreeMap<String, u32>) {
    let candidate_total: u32 = (1..=20)
        .map(|index| {
            values
                .get(&format!("E.{index}"))
                .copied()
                .unwrap_or_default()
        })
        .sum();
    values.insert("E".to_owned(), candidate_total);

    let ballot_total = ["E", "F", "G"]
        .into_iter()
        .map(|id| values.get(id).copied().unwrap_or_default())
        .sum();
    values.insert("H".to_owned(), ballot_total);
}

fn official_mismatches(
    markdown_values: &BTreeMap<String, u32>,
    official_values: &BTreeMap<String, u32>,
) -> Vec<String> {
    expected_vote_ids()
        .into_iter()
        .filter_map(|id| {
            let markdown_value = markdown_values.get(&id).copied();
            let official_value = official_values.get(&id).copied();
            if markdown_value == official_value {
                None
            } else {
                Some(format!(
                    "{id}: md={}, official={}",
                    optional_u32(markdown_value),
                    optional_u32(official_value)
                ))
            }
        })
        .collect()
}

fn optional_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "missing".to_owned())
}

fn parse_votes_markdown_values(markdown: &str) -> BTreeMap<String, u32> {
    let mut values = BTreeMap::new();
    let expected_ids = expected_vote_ids();
    for line in markdown.lines().map(str::trim) {
        let cells = markdown_cells(line);
        if cells.len() != 2 {
            continue;
        }
        if expected_ids.contains(&cells[0]) {
            if let Ok(value) = cells[1].parse::<u32>() {
                values.insert(cells[0].clone(), value);
            }
        }
    }
    values
}

fn parse_correction_markdown_values(markdown: &str) -> BTreeMap<String, Correction> {
    let mut corrections = BTreeMap::new();
    for line in markdown.lines().map(str::trim).skip(2) {
        if line.is_empty() {
            continue;
        }
        let cells = markdown_cells(line);
        if cells.len() != 5 {
            continue;
        }
        let Some(id) = normalize_correction_id(&cells[0]) else {
            continue;
        };
        let Some(difference) = parse_i32_cell(&cells[3]) else {
            continue;
        };
        corrections.insert(
            id,
            Correction {
                first: parse_optional_u32_cell(&cells[1]),
                second: parse_optional_u32_cell(&cells[2]),
                difference,
            },
        );
    }
    corrections
}

fn expected_vote_ids() -> Vec<String> {
    (1..=20)
        .map(|index| format!("E.{index}"))
        .chain(["E", "F", "G", "H"].into_iter().map(str::to_owned))
        .collect()
}

fn normalize_correction_id(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("blanco") || trimmed.eq_ignore_ascii_case("blank") {
        return Some("F".to_owned());
    }
    if trimmed.eq_ignore_ascii_case("ongeldig") || trimmed.eq_ignore_ascii_case("invalid") {
        return Some("G".to_owned());
    }
    if trimmed.eq_ignore_ascii_case("totaal geldige stemmen")
        || trimmed.eq_ignore_ascii_case("totaal stemmen op kandidaten")
    {
        return Some("E".to_owned());
    }
    if trimmed.eq_ignore_ascii_case("totaal uitgebrachte stemmen")
        || trimmed.eq_ignore_ascii_case("correctie uitgebrachte stemmen")
    {
        return Some("H".to_owned());
    }
    if matches!(trimmed, "E" | "F" | "G" | "H") {
        return Some(trimmed.to_owned());
    }
    if let Some(number) = trimmed.strip_prefix("E.") {
        if let Ok(number) = number.parse::<u32>() {
            if (1..=20).contains(&number) {
                return Some(format!("E.{number}"));
            }
        }
        return None;
    }
    if let Ok(number) = trimmed.parse::<u32>() {
        if (1..=20).contains(&number) {
            return Some(format!("E.{number}"));
        }
    }
    None
}

fn parse_optional_u32_cell(cell: &str) -> Option<u32> {
    let value = cell.trim();
    if value.is_empty() || value == "-" {
        return None;
    }
    value.parse::<u32>().ok()
}

fn parse_i32_cell(cell: &str) -> Option<i32> {
    cell.trim().parse::<i32>().ok()
}

fn display_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(str::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

fn write_mismatch_reports(
    municipality_dir: &Path,
    rows: &[ComparisonRow],
    debug: bool,
    preserve_other_reports: bool,
) -> Result<Option<PathBuf>> {
    let report_dir = municipality_dir.join("results").join("mismatches");
    if preserve_other_reports {
        prepare_filtered_mismatch_report_dir(&report_dir, rows)?;
    }

    let report_rows: Vec<_> = rows
        .iter()
        .filter(|row| {
            matches!(
                row.status,
                ComparisonStatus::Incomplete
                    | ComparisonStatus::CorrectionInconsistent
                    | ComparisonStatus::InternallyInconsistent
                    | ComparisonStatus::Mismatch
            )
        })
        .collect();
    if report_rows.is_empty() {
        debug_log(debug, "no mismatch reports to write");
        return Ok(None);
    }

    if preserve_other_reports {
        debug_log(
            debug,
            format!(
                "updating selected mismatch reports in {}",
                report_dir.display()
            ),
        );
    } else {
        debug_log(
            debug,
            format!(
                "clearing mismatch report directory {}",
                report_dir.display()
            ),
        );
        reset_mismatch_report_dir(&report_dir)?;
    }
    debug_log(
        debug,
        format!("writing {} mismatch reports", report_rows.len()),
    );
    for (index, row) in report_rows.iter().enumerate() {
        debug_log(
            debug,
            format!(
                "[{}/{}] station {}: {}",
                index + 1,
                report_rows.len(),
                row.station,
                row.status.label()
            ),
        );
        write_mismatch_report(municipality_dir, &report_dir, row)?;
    }
    debug_log(
        debug,
        format!("wrote mismatch reports to {}", report_dir.display()),
    );
    Ok(Some(report_dir))
}

fn prepare_filtered_mismatch_report_dir(report_dir: &Path, rows: &[ComparisonRow]) -> Result<()> {
    fs::create_dir_all(report_dir)?;
    for row in rows {
        let base_name = format!("station-{}", safe_file_part(&row.station));
        for suffix in [".md", ".png", "-corrections.png"] {
            let path = report_dir.join(format!("{base_name}{suffix}"));
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}

fn reset_mismatch_report_dir(report_dir: &Path) -> Result<()> {
    if report_dir.exists() {
        for entry in fs::read_dir(report_dir)? {
            let path = entry?.path();
            if path.extension().and_then(OsStr::to_str).is_some_and(|ext| {
                ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("png")
            }) {
                fs::remove_file(path)?;
            }
        }
    } else {
        fs::create_dir_all(report_dir)?;
    }
    Ok(())
}

fn write_mismatch_report(
    municipality_dir: &Path,
    report_dir: &Path,
    row: &ComparisonRow,
) -> Result<()> {
    let base_name = format!("station-{}", safe_file_part(&row.station));
    let report_path = report_dir.join(format!("{base_name}.md"));
    let highlighted_image_name = format!("{base_name}.png");
    let highlighted_image_path = report_dir.join(&highlighted_image_name);

    let image_note = if let Some(markdown_path) = &row.markdown_path {
        let full_crop_path = full_crop_path_for_markdown(municipality_dir, markdown_path)?;
        if full_crop_path.exists() {
            let highlight_rows = highlight_rows_for_row(row);
            let markdown_values = fs::read_to_string(markdown_path)
                .map(|markdown| parse_votes_markdown_values(&markdown))
                .unwrap_or_default();
            write_highlighted_table_image(
                &full_crop_path,
                &highlighted_image_path,
                &highlight_rows,
                &markdown_values,
                &row.official_values,
            )?;
            format!(
                "\nLegend: yellow/red = official CSV mismatch; blue = internal consistency issue. The right margin shows OCR and official values for official mismatches.\n\n![Highlighted table rows]({highlighted_image_name})\n"
            )
        } else {
            format!(
                "\nFull table crop not found: `{}`\n",
                full_crop_path.display()
            )
        }
    } else {
        "\nNo source Markdown file is available for this row.\n".to_owned()
    };

    let mut content = String::new();
    content.push_str(&format!("# Station {}", row.station));
    if !row.location.is_empty() {
        content.push_str(&format!(" - {}", row.location));
    }
    content.push_str("\n\n");
    content.push_str(&format!("- Status: `{}`\n", row.status.label()));
    if let Some(markdown_path) = &row.markdown_path {
        content.push_str(&format!("- Markdown: `{}`\n", markdown_path.display()));
    }
    if let Some(correction_path) = &row.correction_path {
        content.push_str(&format!(
            "- Correction OCR: `{}`\n",
            correction_path.display()
        ));
    }
    if row.status == ComparisonStatus::CorrectionInconsistent {
        content.push_str("\n## Correction Inconsistency\n\n");
        content.push_str(
            "The correction table does not line up with the first-count Markdown. Inspect the correction crop before treating this as a plain official-result mismatch.\n\n",
        );
        for detail in correction_issue_details_from_row(row) {
            content.push_str(&format!("- {detail}\n"));
        }
    }
    content.push_str("\n## Details\n\n");
    for detail in row.details.split("; ").filter(|detail| !detail.is_empty()) {
        content.push_str(&format!("- {detail}\n"));
    }
    content.push_str(&image_note);
    content.push_str(&correction_note_for_report(
        municipality_dir,
        report_dir,
        &base_name,
        row,
    )?);
    fs::write(report_path, content)?;
    Ok(())
}

fn correction_issue_details_from_row(row: &ComparisonRow) -> Vec<&str> {
    row.details
        .split("; ")
        .filter(|detail| detail.starts_with("correction "))
        .collect()
}

fn correction_note_for_report(
    municipality_dir: &Path,
    report_dir: &Path,
    base_name: &str,
    row: &ComparisonRow,
) -> Result<String> {
    if row.markdown_path.is_none() && row.correction_path.is_none() {
        return Ok(String::new());
    }

    let mut content = String::new();
    content.push_str("\n## Corrections\n\n");
    if let Some(correction_path) = &row.correction_path {
        match fs::read_to_string(correction_path) {
            Ok(markdown) => {
                content.push_str("```markdown\n");
                content.push_str(markdown.trim_end());
                content.push_str("\n```\n");
            }
            Err(error) => {
                content.push_str(&format!(
                    "Could not read correction OCR `{}`: {error}\n",
                    correction_path.display()
                ));
            }
        }
    } else {
        content.push_str("No correction OCR Markdown is available for this station.\n");
    }

    if let Some(crop_path) = correction_crop_path_for_row(municipality_dir, row)? {
        if crop_path.exists() {
            let image_name = format!("{base_name}-corrections.png");
            let output_path = report_dir.join(&image_name);
            write_correction_table_report_image(&crop_path, &output_path, row)?;
            content.push_str(&format!("\n![Correction table]({image_name})\n"));
        } else {
            content.push_str(&format!(
                "\nCorrection table crop not found: `{}`\n",
                crop_path.display()
            ));
        }
    }

    Ok(content)
}

fn write_correction_table_report_image(
    crop_path: &Path,
    output_path: &Path,
    row: &ComparisonRow,
) -> Result<()> {
    let issue_ids = correction_issue_ids_from_row(row);
    if issue_ids.is_empty() {
        fs::copy(crop_path, output_path)?;
        return Ok(());
    }

    let mut image = image::open(crop_path)?.to_rgba8();
    let correction_ids = row
        .correction_path
        .as_ref()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|markdown| correction_ids_in_markdown(&markdown))
        .unwrap_or_default();
    let bands = correction_table_row_bands(&image, correction_ids.len());
    for issue_id in issue_ids {
        if let Some(row_index) = correction_ids.iter().position(|id| id == &issue_id) {
            if let Some((top, bottom)) = bands.get(row_index).copied() {
                highlight_image_band(&mut image, top, bottom, CORRECTION_HIGHLIGHT);
                draw_correction_error_label(&mut image, top, bottom);
            }
        }
    }
    image.save_with_format(output_path, ImageFormat::Png)?;
    Ok(())
}

fn correction_ids_in_markdown(markdown: &str) -> Vec<String> {
    markdown
        .lines()
        .map(str::trim)
        .skip(2)
        .filter_map(|line| {
            let cells = markdown_cells(line);
            if cells.len() == 5 {
                normalize_correction_id(&cells[0])
            } else {
                None
            }
        })
        .collect()
}

fn correction_issue_ids_from_row(row: &ComparisonRow) -> BTreeSet<String> {
    row.details
        .split("; ")
        .filter_map(|detail| {
            detail
                .strip_prefix("correction ")
                .and_then(|rest| rest.split_once(' '))
                .map(|(id, _)| id.to_owned())
        })
        .collect()
}

fn correction_table_row_bands(image: &RgbaImage, row_count: usize) -> Vec<(u32, u32)> {
    let (_width, height) = image.dimensions();
    let lines = detect_correction_table_lines(image);
    if lines.len() > row_count {
        return lines
            .windows(2)
            .take(row_count)
            .map(|pair| padded_band(pair[0], pair[1], height))
            .collect();
    }

    let row_top = scaled(0.200, height);
    let row_height = scaled(0.037, height).max(1);
    (0..row_count)
        .map(|index| {
            let top = row_top + row_height * index as u32;
            padded_band(top, top + row_height, height)
        })
        .collect()
}

fn detect_correction_table_lines(image: &RgbaImage) -> Vec<u32> {
    let (width, height) = image.dimensions();
    let x_start = scaled(0.040, width);
    let x_end = scaled(0.940, width).max(x_start + 1);
    let y_start = scaled(0.170, height);
    let y_end = scaled(0.960, height);
    let threshold = ((x_end - x_start) as f32 * 0.60).round() as u32;
    let mut line_rows = Vec::new();
    for y in y_start..y_end {
        let mut dark_pixels = 0;
        for x in x_start..x_end {
            let pixel = image.get_pixel(x, y).0;
            if pixel[0] < 150 && pixel[1] < 150 && pixel[2] < 150 {
                dark_pixels += 1;
            }
        }
        if dark_pixels >= threshold {
            line_rows.push(y);
        }
    }
    grouped_line_centers(&line_rows)
}

fn draw_correction_error_label(image: &mut RgbaImage, top: u32, bottom: u32) {
    let (width, _height) = image.dimensions();
    let scale = (width / 320).clamp(4, 8);
    let label = "ERROR";
    let text_width = bitmap_text_width(label, scale);
    let x = width
        .saturating_sub(text_width)
        .saturating_sub(scaled(0.060, width));
    let row_height = bottom.saturating_sub(top);
    let y = top + row_height.saturating_sub(bitmap_text_height(scale)) / 2;
    draw_bitmap_text(image, label, x, y, scale, [160, 0, 160]);
}

fn correction_crop_path_for_row(
    municipality_dir: &Path,
    row: &ComparisonRow,
) -> Result<Option<PathBuf>> {
    if let Some(correction_path) = &row.correction_path {
        let stem = correction_path
            .file_stem()
            .and_then(OsStr::to_str)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Correction Markdown path has no valid file stem: {}",
                        correction_path.display()
                    ),
                )
            })?;
        return Ok(Some(
            municipality_dir
                .join("crops")
                .join("corrections")
                .join(format!("{stem}.png")),
        ));
    }
    find_correction_crop_for_station(municipality_dir, &row.station)
}

fn find_correction_crop_for_station(
    municipality_dir: &Path,
    station: &str,
) -> Result<Option<PathBuf>> {
    let corrections_dir = municipality_dir.join("crops").join("corrections");
    if !corrections_dir.exists() {
        return Ok(None);
    }
    for entry in fs::read_dir(corrections_dir)? {
        let path = entry?.path();
        if path.extension().and_then(OsStr::to_str) == Some("png")
            && station_code_from_markdown_path(&path).as_deref() == Some(station)
        {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

fn full_crop_path_for_markdown(municipality_dir: &Path, markdown_path: &Path) -> Result<PathBuf> {
    let stem = markdown_path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "Markdown path has no valid file stem: {}",
                    markdown_path.display()
                ),
            )
        })?;
    Ok(municipality_dir
        .join("crops")
        .join("2.2")
        .join(format!("{stem}.png")))
}

#[derive(Debug)]
struct HighlightRows {
    official: BTreeSet<String>,
    internal: BTreeSet<String>,
}

fn highlight_rows_for_row(row: &ComparisonRow) -> HighlightRows {
    HighlightRows {
        official: official_problem_ids_for_row(row),
        internal: internal_problem_ids_from_details(&row.details),
    }
}

fn official_problem_ids_for_row(row: &ComparisonRow) -> BTreeSet<String> {
    let Some(markdown_path) = &row.markdown_path else {
        return official_problem_ids_from_details(&row.details);
    };
    if row.official_values.is_empty() {
        return official_problem_ids_from_details(&row.details);
    }
    let Ok(markdown) = fs::read_to_string(markdown_path) else {
        return official_problem_ids_from_details(&row.details);
    };
    let markdown_values = parse_votes_markdown_values(&markdown);
    let ids: BTreeSet<String> = expected_vote_ids()
        .into_iter()
        .filter(|id| markdown_values.get(id) != row.official_values.get(id))
        .collect();
    if ids.is_empty() {
        official_problem_ids_from_details(&row.details)
    } else {
        ids
    }
}

fn official_problem_ids_from_details(details: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for detail in details.split("; ") {
        if let Some((id, _)) = detail.split_once(": md=") {
            insert_vote_id(&mut ids, id);
        }
    }
    ids
}

fn internal_problem_ids_from_details(details: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for detail in details.split("; ") {
        if detail.starts_with("E.1 through E.20 sum to ") {
            ids.insert("E".to_owned());
            continue;
        }
        if detail.starts_with("E + F + G is ") {
            ids.insert("H".to_owned());
            continue;
        }
        if let Some(rest) = detail.strip_prefix("row ") {
            if let Some((_, rest)) = rest.split_once(" expected ID ") {
                if let Some((expected, found)) = rest.split_once(", found ") {
                    insert_vote_id(&mut ids, expected);
                    insert_vote_id(&mut ids, found);
                }
            } else if let Some((_, rest)) = rest.split_once(" value for ") {
                if let Some((id, _)) = rest
                    .split_once(" is not digits only")
                    .or_else(|| rest.split_once(" could not be parsed"))
                {
                    insert_vote_id(&mut ids, id);
                }
            }
            continue;
        }
        if let Some(rest) = detail.strip_prefix("missing row ") {
            if let Some((_, id)) = rest.split_once(" for ") {
                insert_vote_id(&mut ids, id);
            }
        }
    }
    ids
}

fn insert_vote_id(ids: &mut BTreeSet<String>, value: &str) {
    let id = value.trim();
    if id == "E" || id == "F" || id == "G" || id == "H" {
        ids.insert(id.to_owned());
    } else if let Some(number) = id.strip_prefix("E.") {
        if number.parse::<u32>().is_ok() {
            ids.insert(id.to_owned());
        }
    }
}

fn write_highlighted_table_image(
    input_path: &Path,
    output_path: &Path,
    highlight_rows: &HighlightRows,
    ocr_values: &BTreeMap<String, u32>,
    official_values: &BTreeMap<String, u32>,
) -> Result<()> {
    let mut image = image::open(input_path)?.to_rgba8();
    let bands = result_row_bands(&image);
    for id in &highlight_rows.internal {
        if let Some((top, bottom)) = bands.get(id).copied() {
            highlight_image_band(&mut image, top, bottom, INTERNAL_HIGHLIGHT);
        }
    }
    for id in &highlight_rows.official {
        if let Some((top, bottom)) = bands.get(id).copied() {
            highlight_image_band(&mut image, top, bottom, OFFICIAL_HIGHLIGHT);
        }
    }
    draw_ocr_and_official_values(
        &mut image,
        &bands,
        &highlight_rows.official,
        ocr_values,
        official_values,
    );
    image.save_with_format(output_path, ImageFormat::Png)?;
    Ok(())
}

fn draw_ocr_and_official_values(
    image: &mut RgbaImage,
    bands: &BTreeMap<String, (u32, u32)>,
    problem_ids: &BTreeSet<String>,
    ocr_values: &BTreeMap<String, u32>,
    official_values: &BTreeMap<String, u32>,
) {
    let values: Vec<_> = problem_ids
        .iter()
        .filter_map(|id| {
            let (top, bottom) = bands.get(id).copied()?;
            let ocr_value = ocr_values
                .get(id)
                .map(u32::to_string)
                .unwrap_or_else(|| "-".to_owned());
            let official_value = official_values
                .get(id)
                .map(u32::to_string)
                .unwrap_or_else(|| "-".to_owned());
            Some((top, bottom, ocr_value, official_value))
        })
        .collect();
    if values.is_empty() {
        return;
    }

    let (width, height) = image.dimensions();
    let value_scale = (height / 360).clamp(5, 8);
    let header_scale = value_scale.saturating_sub(2).max(3);
    let right_margin = scaled(0.035, width).max(40);
    let column_gap = value_scale * 6;
    let ocr_color = [0, 80, 180];
    let official_color = [180, 0, 0];
    let ocr_header = "OCR";
    let official_header = "OFFICIAL";
    let corrected_header = "(CORRECTED)";
    let max_ocr_width = values
        .iter()
        .map(|(_, _, value, _)| bitmap_text_width(value, value_scale))
        .chain([bitmap_text_width(ocr_header, header_scale)])
        .max()
        .unwrap_or(0);
    let max_official_width = values
        .iter()
        .map(|(_, _, _, value)| bitmap_text_width(value, value_scale))
        .chain([
            bitmap_text_width(official_header, header_scale),
            bitmap_text_width(corrected_header, header_scale),
        ])
        .max()
        .unwrap_or(0);
    let official_x = width.saturating_sub(right_margin + max_official_width);
    let ocr_x = official_x.saturating_sub(column_gap + max_ocr_width);
    let min_top = values.iter().map(|(top, _, _, _)| *top).min().unwrap_or(0);
    let header_y = min_top
        .saturating_sub(bitmap_text_height(header_scale) * 2 + value_scale * 3)
        .max(scaled(0.080, height));
    draw_bitmap_text(image, ocr_header, ocr_x, header_y, header_scale, ocr_color);
    let official_header_x = official_x
        + max_official_width.saturating_sub(bitmap_text_width(official_header, header_scale));
    draw_bitmap_text(
        image,
        official_header,
        official_header_x,
        header_y,
        header_scale,
        official_color,
    );
    draw_bitmap_text(
        image,
        corrected_header,
        official_x,
        header_y + bitmap_text_height(header_scale) + header_scale,
        header_scale,
        official_color,
    );

    for (top, bottom, ocr_value, official_value) in values {
        let center = top + (bottom.saturating_sub(top) / 2);
        let y = center.saturating_sub(bitmap_text_height(value_scale) / 2);
        let ocr_value_x =
            ocr_x + max_ocr_width.saturating_sub(bitmap_text_width(&ocr_value, value_scale));
        let official_value_x = official_x
            + max_official_width.saturating_sub(bitmap_text_width(&official_value, value_scale));
        draw_bitmap_text(image, &ocr_value, ocr_value_x, y, value_scale, ocr_color);
        draw_bitmap_text(
            image,
            &official_value,
            official_value_x,
            y,
            value_scale,
            official_color,
        );
    }
}

fn draw_bitmap_text(image: &mut RgbaImage, text: &str, x: u32, y: u32, scale: u32, color: [u8; 3]) {
    let mut cursor_x = x;
    for ch in text.chars() {
        if ch == ' ' {
            cursor_x += scale * 4;
            continue;
        }
        if let Some(pattern) = bitmap_glyph(ch) {
            draw_bitmap_glyph(image, &pattern, cursor_x, y, scale, color);
            cursor_x += scale * 6;
        } else {
            cursor_x += scale * 4;
        }
    }
}

fn draw_bitmap_glyph(
    image: &mut RgbaImage,
    pattern: &[&str; 7],
    x: u32,
    y: u32,
    scale: u32,
    color: [u8; 3],
) {
    let (width, height) = image.dimensions();
    for (row, line) in pattern.iter().enumerate() {
        for (column, pixel) in line.chars().enumerate() {
            if pixel != '1' {
                continue;
            }
            let start_x = x + column as u32 * scale;
            let start_y = y + row as u32 * scale;
            for draw_y in start_y..(start_y + scale).min(height) {
                for draw_x in start_x..(start_x + scale).min(width) {
                    image.put_pixel(
                        draw_x,
                        draw_y,
                        image::Rgba([color[0], color[1], color[2], 255]),
                    );
                }
            }
        }
    }
}

fn bitmap_text_width(text: &str, scale: u32) -> u32 {
    let mut width = 0;
    for ch in text.chars() {
        width += if ch == ' ' {
            scale * 4
        } else if bitmap_glyph(ch).is_some() {
            scale * 6
        } else {
            scale * 4
        };
    }
    width.saturating_sub(scale)
}

fn bitmap_text_height(scale: u32) -> u32 {
    scale * 7
}

fn bitmap_glyph(ch: char) -> Option<[&'static str; 7]> {
    match ch.to_ascii_uppercase() {
        '0' => Some([
            "01110", "10001", "10011", "10101", "11001", "10001", "01110",
        ]),
        '1' => Some([
            "00100", "01100", "00100", "00100", "00100", "00100", "01110",
        ]),
        '2' => Some([
            "01110", "10001", "00001", "00010", "00100", "01000", "11111",
        ]),
        '3' => Some([
            "11110", "00001", "00001", "01110", "00001", "00001", "11110",
        ]),
        '4' => Some([
            "00010", "00110", "01010", "10010", "11111", "00010", "00010",
        ]),
        '5' => Some([
            "11111", "10000", "10000", "11110", "00001", "00001", "11110",
        ]),
        '6' => Some([
            "01110", "10000", "10000", "11110", "10001", "10001", "01110",
        ]),
        '7' => Some([
            "11111", "00001", "00010", "00100", "01000", "01000", "01000",
        ]),
        '8' => Some([
            "01110", "10001", "10001", "01110", "10001", "10001", "01110",
        ]),
        '9' => Some([
            "01110", "10001", "10001", "01111", "00001", "00001", "01110",
        ]),
        'A' => Some([
            "01110", "10001", "10001", "11111", "10001", "10001", "10001",
        ]),
        'C' => Some([
            "01111", "10000", "10000", "10000", "10000", "10000", "01111",
        ]),
        'D' => Some([
            "11110", "10001", "10001", "10001", "10001", "10001", "11110",
        ]),
        'E' => Some([
            "11111", "10000", "10000", "11110", "10000", "10000", "11111",
        ]),
        'F' => Some([
            "11111", "10000", "10000", "11110", "10000", "10000", "10000",
        ]),
        'I' => Some([
            "11111", "00100", "00100", "00100", "00100", "00100", "11111",
        ]),
        'L' => Some([
            "10000", "10000", "10000", "10000", "10000", "10000", "11111",
        ]),
        'O' => Some([
            "01110", "10001", "10001", "10001", "10001", "10001", "01110",
        ]),
        'R' => Some([
            "11110", "10001", "10001", "11110", "10100", "10010", "10001",
        ]),
        'T' => Some([
            "11111", "00100", "00100", "00100", "00100", "00100", "00100",
        ]),
        '(' => Some([
            "00010", "00100", "01000", "01000", "01000", "00100", "00010",
        ]),
        ')' => Some([
            "01000", "00100", "00010", "00010", "00010", "00100", "01000",
        ]),
        '-' => Some([
            "00000", "00000", "00000", "11111", "00000", "00000", "00000",
        ]),
        _ => None,
    }
}

fn result_row_bands(image: &RgbaImage) -> BTreeMap<String, (u32, u32)> {
    let (_width, height) = image.dimensions();
    let mut bands = BTreeMap::new();
    let lines = detect_candidate_row_lines(image);
    if lines.len() >= 21 {
        let row_height = median_spacing(&lines[..21]).max(1);
        for index in 0..20 {
            bands.insert(
                format!("E.{}", index + 1),
                padded_band(lines[index], lines[index + 1], height),
            );
        }
        let e20_bottom = lines[20];
        bands.insert(
            "E".to_owned(),
            padded_band(e20_bottom + row_height, e20_bottom + row_height * 2, height),
        );
        bands.insert(
            "F".to_owned(),
            padded_band(
                e20_bottom + row_height * 2,
                e20_bottom + row_height * 3,
                height,
            ),
        );
        bands.insert(
            "G".to_owned(),
            padded_band(
                e20_bottom + row_height * 3,
                e20_bottom + row_height * 4,
                height,
            ),
        );
        bands.insert(
            "H".to_owned(),
            padded_band(
                e20_bottom + row_height * 5,
                e20_bottom + row_height * 6,
                height,
            ),
        );
    } else {
        insert_fallback_row_bands(&mut bands, height);
    }
    bands
}

fn detect_candidate_row_lines(image: &RgbaImage) -> Vec<u32> {
    let (width, height) = image.dimensions();
    let x_start = scaled(0.020, width);
    let x_end = scaled(0.200, width).max(x_start + 1);
    let y_start = scaled(0.115, height);
    let y_end = scaled(0.790, height);
    let threshold = ((x_end - x_start) as f32 * 0.35).round() as u32;
    let mut line_rows = Vec::new();
    for y in y_start..y_end {
        let mut dark_pixels = 0;
        for x in x_start..x_end {
            let pixel = image.get_pixel(x, y).0;
            if pixel[0] < 170 && pixel[1] < 170 && pixel[2] < 170 {
                dark_pixels += 1;
            }
        }
        if dark_pixels >= threshold {
            line_rows.push(y);
        }
    }
    grouped_line_centers(&line_rows)
}

fn grouped_line_centers(rows: &[u32]) -> Vec<u32> {
    let mut centers = Vec::new();
    let mut current_start = None;
    let mut previous = None;
    for row in rows {
        match (current_start, previous) {
            (Some(start), Some(prev)) if *row <= prev + 2 => {
                previous = Some(*row);
                current_start = Some(start);
            }
            (Some(start), Some(prev)) => {
                centers.push((start + prev) / 2);
                current_start = Some(*row);
                previous = Some(*row);
            }
            _ => {
                current_start = Some(*row);
                previous = Some(*row);
            }
        }
    }
    if let (Some(start), Some(prev)) = (current_start, previous) {
        centers.push((start + prev) / 2);
    }
    centers
}

fn median_spacing(lines: &[u32]) -> u32 {
    let mut spacings: Vec<_> = lines
        .windows(2)
        .filter_map(|pair| pair[1].checked_sub(pair[0]))
        .collect();
    spacings.sort_unstable();
    spacings
        .get(spacings.len().saturating_sub(1) / 2)
        .copied()
        .unwrap_or(1)
}

fn padded_band(top: u32, bottom: u32, height: u32) -> (u32, u32) {
    let padding = ((bottom.saturating_sub(top)) / 10).max(2);
    (
        top.saturating_sub(padding).min(height),
        (bottom + padding).min(height),
    )
}

fn insert_fallback_row_bands(bands: &mut BTreeMap<String, (u32, u32)>, height: u32) {
    let e_top = scaled(0.124, height);
    let e_row_height = scaled(0.0322, height).max(1);
    for index in 0..20 {
        let top = e_top + e_row_height * index;
        bands.insert(
            format!("E.{}", index + 1),
            padded_band(top, top + e_row_height, height),
        );
    }
    let e20_bottom = e_top + e_row_height * 20;
    bands.insert(
        "E".to_owned(),
        padded_band(
            e20_bottom + e_row_height,
            e20_bottom + e_row_height * 2,
            height,
        ),
    );
    bands.insert(
        "F".to_owned(),
        padded_band(
            e20_bottom + e_row_height * 2,
            e20_bottom + e_row_height * 3,
            height,
        ),
    );
    bands.insert(
        "G".to_owned(),
        padded_band(
            e20_bottom + e_row_height * 3,
            e20_bottom + e_row_height * 4,
            height,
        ),
    );
    bands.insert(
        "H".to_owned(),
        padded_band(
            e20_bottom + e_row_height * 5,
            e20_bottom + e_row_height * 6,
            height,
        ),
    );
}

#[derive(Clone, Copy)]
struct HighlightStyle {
    fill: [u8; 3],
    fill_alpha: u16,
    border: [u8; 3],
    border_alpha: u16,
}

const OFFICIAL_HIGHLIGHT: HighlightStyle = HighlightStyle {
    fill: [255, 230, 0],
    fill_alpha: 72,
    border: [220, 0, 0],
    border_alpha: 180,
};

const INTERNAL_HIGHLIGHT: HighlightStyle = HighlightStyle {
    fill: [70, 170, 255],
    fill_alpha: 72,
    border: [0, 80, 220],
    border_alpha: 180,
};

const CORRECTION_HIGHLIGHT: HighlightStyle = HighlightStyle {
    fill: [255, 0, 180],
    fill_alpha: 76,
    border: [160, 0, 160],
    border_alpha: 220,
};

fn highlight_image_band(image: &mut RgbaImage, top: u32, bottom: u32, style: HighlightStyle) {
    let (width, height) = image.dimensions();
    let top = top.min(height);
    let bottom = bottom.min(height);
    let width = width as usize;
    let buffer = image.as_mut();
    for y in top..bottom {
        let row_start = y as usize * width * 4;
        for x in 0..width {
            let index = row_start + x * 4;
            blend_pixel(&mut buffer[index..index + 4], style.fill, style.fill_alpha);
        }
    }
    for y in top.saturating_sub(2)..(top + 2).min(height) {
        let row_start = y as usize * width * 4;
        for x in 0..width {
            let index = row_start + x * 4;
            blend_pixel(
                &mut buffer[index..index + 4],
                style.border,
                style.border_alpha,
            );
        }
    }
    for y in bottom.saturating_sub(2)..(bottom + 2).min(height) {
        let row_start = y as usize * width * 4;
        for x in 0..width {
            let index = row_start + x * 4;
            blend_pixel(
                &mut buffer[index..index + 4],
                style.border,
                style.border_alpha,
            );
        }
    }
}

fn blend_pixel(pixel: &mut [u8], color: [u8; 3], alpha: u16) {
    for channel in 0..3 {
        pixel[channel] = (((pixel[channel] as u16) * (255 - alpha)
            + (color[channel] as u16) * alpha)
            / 255) as u8;
    }
}

fn print_comparison_report(
    official_results: &OfficialResults,
    results_dir: &Path,
    corrections_dir: &Path,
    mismatch_report_dir: Option<&Path>,
    rows: &[ComparisonRow],
    format: ReportFormat,
) {
    match format {
        ReportFormat::Terminal => print_terminal_comparison_report(
            official_results,
            results_dir,
            corrections_dir,
            mismatch_report_dir,
            rows,
        ),
        ReportFormat::Markdown => print_markdown_comparison_report(
            official_results,
            results_dir,
            corrections_dir,
            mismatch_report_dir,
            rows,
        ),
    }
}

fn print_terminal_comparison_report(
    official_results: &OfficialResults,
    results_dir: &Path,
    corrections_dir: &Path,
    mismatch_report_dir: Option<&Path>,
    rows: &[ComparisonRow],
) {
    let width = terminal_width();
    let status_width = [
        "Status",
        ComparisonStatus::Missing.label(),
        ComparisonStatus::Incomplete.label(),
        ComparisonStatus::CorrectionInconsistent.label(),
        ComparisonStatus::InternallyInconsistent.label(),
        ComparisonStatus::Mismatch.label(),
        ComparisonStatus::FullyMatches.label(),
    ]
    .into_iter()
    .map(char_width)
    .max()
    .unwrap_or("Status".len());
    let station_width = rows
        .iter()
        .map(|row| char_width(&row.station))
        .chain([char_width("Station")])
        .max()
        .unwrap_or(char_width("Station"));
    let fixed_width = status_width + station_width + 6;
    let flexible_width = width.saturating_sub(fixed_width).max(64);
    let location_width = ((flexible_width * 45) / 100).clamp(24, 48);
    let reason_width = flexible_width.saturating_sub(location_width).max(28);
    let use_color = io::stdout().is_terminal() && env::var_os("NO_COLOR").is_none();

    println!(
        "Official CSV        {}",
        official_results.source_path.display()
    );
    println!("Markdown directory  {}", results_dir.display());
    println!("Corrections dir     {}", corrections_dir.display());
    if let Some(mismatch_report_dir) = mismatch_report_dir {
        println!("Mismatch reports   {}", mismatch_report_dir.display());
    }
    println!();
    println!("Summary");
    let counts = comparison_counts(rows);
    for status in [
        ComparisonStatus::Missing,
        ComparisonStatus::Incomplete,
        ComparisonStatus::CorrectionInconsistent,
        ComparisonStatus::InternallyInconsistent,
        ComparisonStatus::Mismatch,
        ComparisonStatus::FullyMatches,
    ] {
        println!(
            "  {:<23} {}",
            status.label(),
            counts.get(status.label()).copied().unwrap_or(0)
        );
    }
    println!();
    println!(
        "{}  {}  {}  {}",
        pad_left("Station", station_width),
        pad_right("Location", location_width),
        pad_right("Status", status_width),
        pad_right("Reason", reason_width)
    );
    println!(
        "{}  {}  {}  {}",
        "-".repeat(station_width),
        "-".repeat(location_width),
        "-".repeat(status_width),
        "-".repeat(reason_width)
    );

    for row in rows {
        println!(
            "{}  {}  {}  {}",
            pad_left(&row.station, station_width),
            terminal_cell(&row.location, location_width),
            format_status(row.status, status_width, use_color),
            terminal_cell(&terminal_reason(row), reason_width)
        );
    }
}

fn terminal_reason(row: &ComparisonRow) -> String {
    let reason = report_reason(row);
    if reason.is_empty() {
        return reason;
    }
    shorten_reason(&reason)
}

fn report_reason(row: &ComparisonRow) -> String {
    if row.details.is_empty()
        || row.status == ComparisonStatus::FullyMatches
        || (row.status == ComparisonStatus::Missing && row.details == "no Markdown result")
    {
        return String::new();
    }
    row.details.clone()
}

fn shorten_reason(details: &str) -> String {
    const MAX_TERMINAL_DETAILS: usize = 2;
    let shortened: Vec<_> = details.split("; ").map(shorten_detail).collect();
    if shortened.len() <= MAX_TERMINAL_DETAILS {
        shortened.join("; ")
    } else {
        format!(
            "{}; +{}",
            shortened[..MAX_TERMINAL_DETAILS].join("; "),
            shortened.len() - MAX_TERMINAL_DETAILS
        )
    }
}

fn shorten_detail(detail: &str) -> String {
    if let Some(shortened) = shorten_mismatch_detail(detail) {
        return shortened;
    }
    if let Some(shortened) = shorten_correction_inconsistency_detail(detail) {
        return shortened;
    }
    if let Some(shortened) = shorten_candidate_sum_detail(detail) {
        return shortened;
    }
    if let Some(shortened) = shorten_ballot_sum_detail(detail) {
        return shortened;
    }
    if let Some(shortened) = shorten_row_count_detail(detail) {
        return shortened;
    }
    if let Some(shortened) = shorten_row_id_detail(detail) {
        return shortened;
    }
    if let Some(shortened) = shorten_missing_row_detail(detail) {
        return shortened;
    }
    if let Some(error) = detail.strip_prefix("could not read Markdown: ") {
        return format!("read error: {error}");
    }
    if let Some(file) = detail.strip_prefix("station not found in official CSV: ") {
        return format!("not in CSV: {file}");
    }
    if detail == "missing correction OCR" {
        return "missing correction OCR".to_owned();
    }
    if let Some(error) = detail.strip_prefix("invalid correction OCR: ") {
        return format!("bad correction OCR: {error}");
    }
    if let Some(error) = detail.strip_prefix("could not apply corrections: ") {
        return format!("bad correction: {error}");
    }
    detail.to_owned()
}

fn shorten_mismatch_detail(detail: &str) -> Option<String> {
    let (id, rest) = detail.split_once(": md=")?;
    let (markdown, official) = rest.split_once(", official=")?;
    Some(format!("{id} {markdown}->{official}"))
}

fn shorten_correction_inconsistency_detail(detail: &str) -> Option<String> {
    let rest = detail.strip_prefix("correction ")?;
    let (id, rest) = rest.split_once(" first=")?;
    let (first, rest) = rest.split_once(", Markdown=")?;
    let (markdown, rest) = rest.split_once(", second=")?;
    let second = rest
        .split_once(", maybe belongs to ")
        .map(|(second, candidate)| format!("{second}, maybe {candidate}"))
        .unwrap_or_else(|| rest.to_owned());
    Some(format!("correction {id} {first}->{second}, md={markdown}"))
}

fn shorten_candidate_sum_detail(detail: &str) -> Option<String> {
    let rest = detail.strip_prefix("E.1 through E.20 sum to ")?;
    let (sum, expected) = rest.split_once(", but E is ")?;
    Some(format!("sum(E.1-E.20) {sum}!={expected}"))
}

fn shorten_ballot_sum_detail(detail: &str) -> Option<String> {
    let rest = detail.strip_prefix("E + F + G is ")?;
    let (sum, expected) = rest.split_once(", but H is ")?;
    Some(format!("E+F+G {sum}!={expected}"))
}

fn shorten_row_count_detail(detail: &str) -> Option<String> {
    let rest = detail.strip_prefix("expected ")?;
    let (expected, found) = rest.split_once(" non-empty table lines, found ")?;
    Some(format!("rows {found}/{expected}"))
}

fn shorten_row_id_detail(detail: &str) -> Option<String> {
    let rest = detail.strip_prefix("row ")?;
    let (row, rest) = rest.split_once(" expected ID ")?;
    let (expected, found) = rest.split_once(", found ")?;
    Some(format!("row {row} {expected}->{found}"))
}

fn shorten_missing_row_detail(detail: &str) -> Option<String> {
    let rest = detail.strip_prefix("missing row ")?;
    let (row, id) = rest.split_once(" for ")?;
    Some(format!("missing row {row} {id}"))
}

fn print_markdown_comparison_report(
    official_results: &OfficialResults,
    results_dir: &Path,
    corrections_dir: &Path,
    mismatch_report_dir: Option<&Path>,
    rows: &[ComparisonRow],
) {
    print!(
        "{}",
        render_markdown_comparison_report(
            official_results,
            results_dir,
            corrections_dir,
            mismatch_report_dir,
            rows,
        )
    );
}

fn render_markdown_comparison_report(
    official_results: &OfficialResults,
    results_dir: &Path,
    corrections_dir: &Path,
    mismatch_report_dir: Option<&Path>,
    rows: &[ComparisonRow],
) -> String {
    let mut output = String::new();
    output.push_str(&format!(
        "Official CSV: {}\n",
        official_results.source_path.display()
    ));
    output.push_str(&format!("Markdown directory: {}\n", results_dir.display()));
    output.push_str(&format!(
        "Corrections directory: {}\n",
        corrections_dir.display()
    ));
    if let Some(mismatch_report_dir) = mismatch_report_dir {
        output.push_str(&format!(
            "Mismatch reports: {}\n",
            mismatch_report_dir.display()
        ));
    }
    output.push('\n');
    output.push_str("| Station | Location | Status | Reason |\n");
    output.push_str("|---:|---|---|---|\n");
    for row in rows {
        output.push_str(&format!(
            "| {} | {} | {} | {} |",
            escape_markdown_table_cell(&row.station),
            escape_markdown_table_cell(&row.location),
            row.status.label(),
            escape_markdown_table_cell(&report_reason(row))
        ));
        output.push('\n');
    }
    let counts = comparison_counts(rows);
    output.push('\n');
    output.push_str("Summary:\n");
    for status in [
        ComparisonStatus::Missing,
        ComparisonStatus::Incomplete,
        ComparisonStatus::CorrectionInconsistent,
        ComparisonStatus::InternallyInconsistent,
        ComparisonStatus::Mismatch,
        ComparisonStatus::FullyMatches,
    ] {
        output.push_str(&format!(
            "- {}: {}",
            status.label(),
            counts.get(status.label()).copied().unwrap_or(0)
        ));
        output.push('\n');
    }
    output
}

fn comparison_counts(rows: &[ComparisonRow]) -> BTreeMap<&'static str, usize> {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for row in rows {
        *counts.entry(row.status.label()).or_default() += 1;
    }
    counts
}

fn terminal_width() -> usize {
    env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width >= 80)
        .unwrap_or(120)
}

fn format_status(status: ComparisonStatus, width: usize, use_color: bool) -> String {
    let padded = pad_right(status.label(), width);
    if !use_color {
        return padded;
    }
    let color = match status {
        ComparisonStatus::Missing => "\x1b[33m",
        ComparisonStatus::Incomplete => "\x1b[36m",
        ComparisonStatus::CorrectionInconsistent => "\x1b[95m",
        ComparisonStatus::InternallyInconsistent => "\x1b[35m",
        ComparisonStatus::Mismatch => "\x1b[31m",
        ComparisonStatus::FullyMatches => "\x1b[32m",
    };
    format!("{color}{padded}\x1b[0m")
}

fn terminal_cell(value: &str, width: usize) -> String {
    let value = if value.trim().is_empty() {
        "-"
    } else {
        value.trim()
    };
    pad_right(&truncate_text(&collapse_whitespace(value), width), width)
}

fn truncate_text(value: &str, width: usize) -> String {
    if char_width(value) <= width {
        return value.to_owned();
    }
    if width <= 3 {
        return value.chars().take(width).collect();
    }
    let mut shortened: String = value.chars().take(width - 3).collect();
    shortened.push_str("...");
    shortened
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn pad_right(value: &str, width: usize) -> String {
    let padding = width.saturating_sub(char_width(value));
    format!("{value}{}", " ".repeat(padding))
}

fn pad_left(value: &str, width: usize) -> String {
    let padding = width.saturating_sub(char_width(value));
    format!("{}{value}", " ".repeat(padding))
}

fn char_width(value: &str) -> usize {
    value.chars().count()
}

fn process_ocr_image(
    image_path: &Path,
    output_path: &Path,
    prompt: &str,
    options: &OcrVotesOptions,
    skip_round_two: bool,
    validate_markdown: fn(&str) -> ValidationReport,
) -> Result<ImageOcrReport> {
    let stem = file_stem_string(image_path)?;
    if skip_round_two && should_skip_ocr_stem(&stem) {
        let markdown = skip_markdown("round two scan");
        fs::write(output_path, &markdown)?;
        return Ok(ImageOcrReport {
            stem,
            output_path: output_path.to_path_buf(),
            action: OcrAction::Skipped,
            validation: validate_markdown(&markdown),
        });
    }

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
                        skipped: false,
                        errors: vec![message],
                    },
                });
            }
        }
    };

    let validation = validate_markdown(&markdown);
    Ok(ImageOcrReport {
        stem,
        output_path: output_path.to_path_buf(),
        action,
        validation,
    })
}

fn should_skip_ocr_stem(stem: &str) -> bool {
    stem.ends_with("_GR26") || !stem.contains("_eerste_telling")
}

fn skip_markdown(reason: &str) -> String {
    format!("{OCR_SKIP_MARKER}\n\nSkipped: {reason}.\n")
}

fn find_ocr_images(
    input_dir: &Path,
    requested: &[String],
    stations: &BTreeSet<String>,
) -> Result<Vec<PathBuf>> {
    let images = if requested.is_empty() && stations.is_empty() {
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
        let mut images = BTreeSet::new();
        for request in requested {
            images.insert(resolve_ocr_image(input_dir, request)?);
        }
        for station in stations {
            images.insert(resolve_station_ocr_image(input_dir, station)?);
        }
        images.into_iter().collect()
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
    if is_station_number(request) {
        return resolve_station_ocr_image(input_dir, &normalize_station_number(request));
    }

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

fn resolve_station_ocr_image(input_dir: &Path, station: &str) -> Result<PathBuf> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(input_dir)? {
        let path = entry?.path();
        if path.extension().and_then(OsStr::to_str) != Some("png") {
            continue;
        }
        if station_code_from_markdown_path(&path).as_deref() == Some(station) {
            matches.push(path);
        }
    }
    matches.sort();
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => err(format!(
            "could not find crop for station {station} in {}",
            input_dir.display()
        )),
        _ => err(format!(
            "multiple crops found for station {station} in {}: {}",
            input_dir.display(),
            matches
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
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
    if markdown.trim_start().starts_with(OCR_SKIP_MARKER) {
        return ValidationReport {
            passed: true,
            skipped: true,
            errors: Vec::new(),
        };
    }

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
        skipped: false,
        errors,
    }
}

fn validate_corrections_markdown(markdown: &str) -> ValidationReport {
    if markdown.trim_start().starts_with(OCR_SKIP_MARKER) {
        return ValidationReport {
            passed: true,
            skipped: true,
            errors: Vec::new(),
        };
    }

    let mut errors = Vec::new();
    let lines: Vec<_> = markdown
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.len() < 2 {
        errors.push(format!(
            "expected at least 2 non-empty table lines, found {}",
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
    if header_cells != ["ID", "First", "Second", "Difference", "Note"] {
        errors.push(format!(
            "expected header `| ID | First | Second | Difference | Note |`, found {header:?}"
        ));
    }
    if lines
        .get(1)
        .is_none_or(|line| !is_corrections_separator(line))
    {
        errors.push("expected Markdown separator row after header".to_owned());
    }

    let mut seen = BTreeSet::new();
    for (index, line) in lines.iter().enumerate().skip(2) {
        let line_number = index + 1;
        let cells = markdown_cells(line);
        if cells.len() != 5 {
            errors.push(format!(
                "row {line_number} should have exactly 5 cells, found {}",
                cells.len()
            ));
            continue;
        }
        let Some(id) = normalize_correction_id(&cells[0]) else {
            errors.push(format!(
                "row {line_number} has unknown correction ID {}",
                cells[0]
            ));
            continue;
        };
        if !seen.insert(id.clone()) {
            errors.push(format!("row {line_number} duplicates correction ID {id}"));
        }
        let first = parse_optional_u32_cell(&cells[1]);
        if first.is_none() && !cells[1].trim().is_empty() && cells[1].trim() != "-" {
            errors.push(format!(
                "row {line_number} first count is not numeric: {}",
                cells[1]
            ));
        }
        let second = parse_optional_u32_cell(&cells[2]);
        if second.is_none() && !cells[2].trim().is_empty() && cells[2].trim() != "-" {
            errors.push(format!(
                "row {line_number} second count is not numeric: {}",
                cells[2]
            ));
        }
        let Some(difference) = parse_i32_cell(&cells[3]) else {
            errors.push(format!(
                "row {line_number} difference is not signed integer: {}",
                cells[3]
            ));
            continue;
        };
        if let (Some(first), Some(second)) = (first, second) {
            let expected = second as i32 - first as i32;
            if expected != difference {
                errors.push(format!(
                    "row {line_number} {id} has difference {difference}, but second-first is {expected}"
                ));
            }
        }
    }

    ValidationReport {
        passed: errors.is_empty(),
        skipped: false,
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

fn is_corrections_separator(line: &str) -> bool {
    let cells = markdown_cells(line);
    cells.len() == 5
        && cells.iter().all(|cell| {
            let stripped = cell.trim_matches(':');
            stripped.len() >= 3 && stripped.chars().all(|ch| ch == '-')
        })
}

struct ProgressEta {
    started: Instant,
    last_printed: Option<Instant>,
}

impl ProgressEta {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            last_printed: None,
        }
    }

    fn maybe_print(&mut self, completed: usize, total: usize) -> Result<()> {
        if total <= 1 || completed == 0 {
            return Ok(());
        }

        let now = Instant::now();
        let should_print = completed == 1
            || completed == total
            || self
                .last_printed
                .map(|last| now.duration_since(last) >= Duration::from_secs(30 * 60))
                .unwrap_or(true);
        if !should_print {
            return Ok(());
        }

        let elapsed = now.duration_since(self.started);
        let remaining_count = total.saturating_sub(completed);
        let remaining_millis =
            elapsed.as_millis().saturating_mul(remaining_count as u128) / completed as u128;
        let remaining = Duration::from_millis(remaining_millis.min(u64::MAX as u128) as u64);
        println!(
            "progress {completed}/{total}; elapsed {}; ETA remaining {}",
            format_duration(elapsed),
            format_duration(remaining)
        );
        io::stdout().flush()?;
        self.last_printed = Some(now);
        Ok(())
    }
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    if hours > 0 {
        format!("{hours}h {minutes:02}m {seconds:02}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds:02}s")
    } else {
        format!("{seconds}s")
    }
}

fn print_ocr_votes_report(reports: &[ImageOcrReport]) {
    print_ocr_report("Voting location OCR results", reports);
}

fn print_ocr_report(title: &str, reports: &[ImageOcrReport]) {
    println!();
    println!("{title}:");
    println!("| Status | Action | Item | Details |");
    println!("|---|---|---|---|");
    for report in reports {
        let status = if report.validation.skipped {
            "SKIP"
        } else if report.validation.passed {
            "PASS"
        } else {
            "FAIL"
        };
        let action = match &report.action {
            OcrAction::Generated => "generated".to_owned(),
            OcrAction::Existing => "existing".to_owned(),
            OcrAction::Skipped => "skipped".to_owned(),
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
    let mut stations = BTreeSet::new();
    let mut out_dir = None;
    let mut kind = CropKind::Votes;
    let mut page_override = None;
    let mut keep_page_images = false;
    let mut force = false;

    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--pdf" => {
                index += 1;
                pdf = Some(PathBuf::from(require_arg(args, index, "--pdf")?));
            }
            "--station" => {
                index += 1;
                stations.insert(normalize_station_number(require_arg(
                    args,
                    index,
                    "--station",
                )?));
            }
            "--out-dir" => {
                index += 1;
                out_dir = Some(PathBuf::from(require_arg(args, index, "--out-dir")?));
            }
            "--kind" => {
                index += 1;
                kind = CropKind::parse(require_arg(args, index, "--kind")?)?;
            }
            "--page" => {
                index += 1;
                page_override = Some(require_arg(args, index, "--page")?.parse()?);
            }
            "--keep-page-images" => {
                keep_page_images = true;
            }
            "--force" => {
                force = true;
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
        stations,
        out_dir,
        kind,
        page_override,
        keep_page_images,
        force,
    })
}

fn parse_ocr_votes_args(args: &[String]) -> Result<OcrVotesOptions> {
    parse_ocr_args(
        args,
        DEFAULT_OCR_PROMPT_PATH,
        "ocr-votes",
        ocr_votes_help_text(),
    )
}

fn parse_ocr_corrections_args(args: &[String]) -> Result<OcrCorrectionsOptions> {
    parse_ocr_args(
        args,
        DEFAULT_CORRECTIONS_OCR_PROMPT_PATH,
        "ocr-corrections",
        ocr_corrections_help_text(),
    )
}

fn parse_ocr_args(
    args: &[String],
    default_prompt: &str,
    command_name: &str,
    help_text: &str,
) -> Result<OcrVotesOptions> {
    let mut positionals = Vec::new();
    let mut input_dir = None;
    let mut out_dir = None;
    let mut prompt = PathBuf::from(default_prompt);
    let mut endpoint =
        env::var("PV_LLM_ENDPOINT").unwrap_or_else(|_| DEFAULT_LLM_ENDPOINT.to_owned());
    let mut model = env::var("PV_LLM_MODEL").unwrap_or_else(|_| DEFAULT_LLM_MODEL.to_owned());
    let mut images = Vec::new();
    let mut stations = BTreeSet::new();
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
            "--station" => {
                index += 1;
                stations.insert(normalize_station_number(require_arg(
                    args,
                    index,
                    "--station",
                )?));
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
            "{command_name} expects <election> <municipality>\n\n{help_text}"
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
        stations,
        force,
        max_tokens,
        timeout,
    })
}

fn parse_official_csvs_args(args: &[String]) -> Result<OfficialCsvOptions> {
    let mut positionals = Vec::new();
    let mut out_dir = None;
    let mut gsb_url = None;
    let mut csb_url = None;
    let mut force = false;

    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--out-dir" => {
                index += 1;
                out_dir = Some(PathBuf::from(require_arg(args, index, "--out-dir")?));
            }
            "--gsb-url" => {
                index += 1;
                gsb_url = Some(require_arg(args, index, "--gsb-url")?.to_owned());
            }
            "--csb-url" => {
                index += 1;
                csb_url = Some(require_arg(args, index, "--csb-url")?.to_owned());
            }
            "--force" => {
                force = true;
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
            "official-csvs expects <election> <municipality>\n\n{}",
            official_csvs_help_text()
        ));
    }

    Ok(OfficialCsvOptions {
        election: normalize_election(&positionals[0]),
        municipality: positionals[1].clone(),
        out_dir,
        gsb_url,
        csb_url,
        force,
    })
}

fn parse_compare_results_args(args: &[String]) -> Result<CompareResultsOptions> {
    let mut positionals = Vec::new();
    let mut results_dir = None;
    let mut corrections_dir = None;
    let mut output_path = None;
    let mut stations = BTreeSet::new();
    let mut format = ReportFormat::Terminal;
    let mut debug = false;

    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--results-dir" => {
                index += 1;
                results_dir = Some(PathBuf::from(require_arg(args, index, "--results-dir")?));
            }
            "--corrections-dir" => {
                index += 1;
                corrections_dir = Some(PathBuf::from(require_arg(
                    args,
                    index,
                    "--corrections-dir",
                )?));
            }
            "--output" => {
                index += 1;
                output_path = Some(PathBuf::from(require_arg(args, index, "--output")?));
            }
            "--station" => {
                index += 1;
                stations.insert(normalize_station_number(require_arg(
                    args,
                    index,
                    "--station",
                )?));
            }
            "--format" => {
                index += 1;
                format = ReportFormat::parse(require_arg(args, index, "--format")?)?;
            }
            "--debug" => {
                debug = true;
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
            "compare-results expects <election> <municipality>\n\n{}",
            compare_results_help_text()
        ));
    }

    Ok(CompareResultsOptions {
        election: normalize_election(&positionals[0]),
        municipality: positionals[1].clone(),
        results_dir,
        corrections_dir,
        output_path,
        stations,
        format,
        debug,
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

fn normalize_station_number(value: &str) -> String {
    let trimmed = value.trim();
    let normalized = trimmed.trim_start_matches('0');
    if normalized.is_empty() {
        "0".to_owned()
    } else {
        normalized.to_owned()
    }
}

fn is_station_number(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit())
}

fn find_pdfs(
    municipality_dir: &Path,
    pdf_arg: Option<&Path>,
    stations: &BTreeSet<String>,
    kind: CropKind,
) -> Result<Vec<PathBuf>> {
    if let Some(pdf_arg) = pdf_arg {
        let pdf_arg_text = pdf_arg.to_string_lossy();
        if is_station_number(&pdf_arg_text) {
            return Ok(vec![resolve_station_pdf(
                municipality_dir,
                &normalize_station_number(&pdf_arg_text),
                kind,
            )?]);
        }
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

    if !stations.is_empty() {
        let mut pdfs = Vec::new();
        for station in stations {
            pdfs.push(resolve_station_pdf(municipality_dir, station, kind)?);
        }
        pdfs.sort();
        return Ok(pdfs);
    }

    let mut pdfs = Vec::new();
    for entry in fs::read_dir(municipality_dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        if default_pdf_matches_kind(file_name, kind) {
            pdfs.push(path);
        }
    }
    pdfs.sort();

    if pdfs.is_empty() {
        return err(format!(
            "no default {} PDFs found in {}",
            kind.filename_part(),
            municipality_dir.display()
        ));
    }
    Ok(pdfs)
}

fn resolve_station_pdf(municipality_dir: &Path, station: &str, kind: CropKind) -> Result<PathBuf> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(municipality_dir)? {
        let path = entry?.path();
        let Some(file_name) = path.file_name().and_then(OsStr::to_str) else {
            continue;
        };
        if default_pdf_matches_kind(file_name, kind)
            && station_code_from_file_name(file_name) == Some(station)
        {
            matches.push(path);
        }
    }
    matches.sort();
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => err(format!(
            "could not find {} PDF for station {station} in {}",
            kind.filename_part(),
            municipality_dir.display()
        )),
        _ => err(format!(
            "multiple {} PDFs found for station {station} in {}: {}",
            kind.filename_part(),
            municipality_dir.display(),
            matches
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

fn default_pdf_matches_kind(file_name: &str, kind: CropKind) -> bool {
    if !file_name.ends_with(".pdf") {
        return false;
    }
    match kind {
        CropKind::Votes => file_name.contains("_eerste_telling"),
        CropKind::Corrections => {
            !file_name.contains("_eerste_telling")
                && station_code_from_file_name(file_name).is_some()
                && file_name.contains("_GR")
        }
    }
}

fn station_code_from_file_name(file_name: &str) -> Option<&str> {
    file_name
        .split('_')
        .find(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

fn crop_pdf(pdf: &Path, out_dir: &Path, options: &CropOptions) -> Result<()> {
    let kind = options.kind;
    let full_table_path = full_table_output_path_for(pdf, out_dir, kind)?;
    if !options.force {
        ensure_output_absent(&full_table_path)?;
    }
    let narrow_path = if kind.narrow_from_full_table_template().is_some() {
        Some(narrow_output_path_for(pdf, out_dir, kind)?)
    } else {
        None
    };
    if let (false, Some(narrow_path)) = (options.force, &narrow_path) {
        ensure_output_absent(narrow_path)?;
    }

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

    if let (Some(narrow_path), Some(template)) =
        (&narrow_path, kind.narrow_from_full_table_template())
    {
        let narrow_crop = write_crop(&full_table_path, narrow_path, template)?;
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
    }

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
  pv ocr-corrections <election|year> <municipality> [options]
  pv official-csvs <election|year> <municipality> [options]
  pv compare-results <election|year> <municipality> [options]

Commands:
  doctor          Check required external tools
  crop            Crop Utrecht-style table regions at native page resolution
  ocr-votes       Run narrow table 2.2 crops through a local multimodal LLM
  ocr-corrections Run correction table crops through a local multimodal LLM
  official-csvs   Fetch official GSB and CSB CSV tellingsbestanden
  compare-results Compare OCR Markdown results against the downloaded GSB CSV

Run `pv <command> --help` for command options."
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

fn print_ocr_corrections_help() {
    println!("{}", ocr_corrections_help_text());
}

fn print_official_csvs_help() {
    println!("{}", official_csvs_help_text());
}

fn print_compare_results_help() {
    println!("{}", compare_results_help_text());
}

fn crop_help_text() -> &'static str {
    "Usage:
  pv crop <election|year> <municipality> [options]

Examples:
  cargo run -- crop 2026 0344 --station 40
  cargo run -- crop 2026 0344 --kind corrections --station 111
  cargo run -- crop 2026-GR 0344

Options:
  --pdf <path-or-number>    Crop one PDF. Relative filenames are resolved inside the municipality directory.
                            A bare number is treated as a polling station number.
  --station <number>        Crop one polling station. Repeat to crop several stations.
                            Without this option, votes crops use all *_eerste_telling.pdf files and
                            corrections crops use station-level second-count PDFs.
  --out-dir <dir>           Output directory. Default: <election>/<municipality>/crops.
  --kind <kind>             Crop kind. Default: votes. Supported: votes / 2.2,
                            corrections / b1-2.4.
  --page <number>           Override automatic section page detection.
  --keep-page-images        Keep extracted native page images under <out-dir>/_native_pages.
  --force                   Overwrite existing crop files.

The votes crop writes lossless PNG crops for table 2.2 \"Uitgebrachte stemmen\"
under <election>/<municipality>/crops/2.2/ and narrow OCR-focused crops under
<election>/<municipality>/crops/2.2/narrow/. The corrections crop writes table
B1-2.4 \"Lijsten met verschillen\" under
<election>/<municipality>/crops/corrections/. The command uses pdftotext to
locate the table page, then extracts the embedded page image with pdfimages and
crops those native pixels directly. Existing output files are treated as an
error unless --force is used."
}

fn ocr_votes_help_text() -> &'static str {
    "Usage:
  pv ocr-votes <election|year> <municipality> [options]

Examples:
  cargo run -- ocr-votes 2026-GR 0344
  cargo run -- ocr-votes 2026 0344 --station 41 --force

Options:
  --input-dir <dir>         Narrow crop directory. Default: <election>/<municipality>/crops/2.2/narrow.
  --out-dir <dir>           Markdown output directory. Default: <election>/<municipality>/results.
  --prompt <path>           Prompt markdown file. Default: prompts/ocr-votes.md.
  --endpoint <url>          OpenAI-compatible local chat endpoint. Default: http://127.0.0.1:8089/v1/chat/completions.
                            Can also be set with PV_LLM_ENDPOINT.
  --model <model>           Model name sent to the endpoint. Default: local.
                            Can also be set with PV_LLM_MODEL.
  --image <stem-path-number>
                            Process only one narrow crop. Repeat to process several specific voting locations.
                            Stems are resolved under --input-dir with a .png suffix.
                            A bare number is treated as a polling station number.
  --station <number>        Process one polling station crop. Repeat to OCR several stations.
  --force                   Re-run the LLM and overwrite existing Markdown outputs.
  --max-tokens <n>          Maximum completion tokens. Default: 4096.
  --timeout-seconds <n>     Socket read/write timeout for each LLM request. Default: 300.

The ocr-votes command is meant to run after `pv crop`. It sends each narrow
PNG crop to a local multimodal LLM in a fresh chat-completion request, writes
one Markdown table per voting location, then validates the Markdown structure
and arithmetic. Existing Markdown files are validated without rerunning the LLM
unless --force is used. The final report lists PASS/FAIL per voting location."
}

fn ocr_corrections_help_text() -> &'static str {
    "Usage:
  pv ocr-corrections <election|year> <municipality> [options]

Examples:
  cargo run -- ocr-corrections 2026-GR 0344
  cargo run -- ocr-corrections 2026 0344 --station 126 --force

Options:
  --input-dir <dir>         Correction crop directory. Default: <election>/<municipality>/crops/corrections.
  --out-dir <dir>           Markdown output directory. Default: <election>/<municipality>/results/corrections.
  --prompt <path>           Prompt markdown file. Default: prompts/ocr-corrections.md.
  --endpoint <url>          OpenAI-compatible local chat endpoint. Default: http://127.0.0.1:8089/v1/chat/completions.
                            Can also be set with PV_LLM_ENDPOINT.
  --model <model>           Model name sent to the endpoint. Default: local.
                            Can also be set with PV_LLM_MODEL.
  --image <stem-path-number>
                            Process only one correction crop. Repeat to process several specific stations.
                            A bare number is treated as a polling station number.
  --station <number>        Process one polling station correction crop. Repeat to OCR several stations.
  --force                   Re-run the LLM and overwrite existing Markdown outputs.
  --max-tokens <n>          Maximum completion tokens. Default: 4096.
  --timeout-seconds <n>     Socket read/write timeout for each LLM request. Default: 300.

The ocr-corrections command is meant to run after
`pv crop --kind corrections`. It writes one Markdown table per station with the
first count, second count, and signed difference for every correction row."
}

fn official_csvs_help_text() -> &'static str {
    "Usage:
  pv official-csvs <election|year> <municipality> [options]

Examples:
  cargo run -- official-csvs 2026-GR 0344
  cargo run -- official-csvs 2026 0344 --force
  cargo run -- official-csvs 2026-GR 9999 --gsb-url <url> --csb-url <url>

Options:
  --out-dir <dir>           Output directory. Default: <election>/<municipality>/results/official.
  --gsb-url <url>           Gemeentelijk stembureau OSV4-3 candidate-count CSV URL.
  --csb-url <url>           Centraal stembureau / final tellingsbestand CSV URL.
  --force                   Overwrite existing CSV files.

For now, built-in URLs exist only for Utrecht (`2026-GR 0344`). Other
municipalities fail unless both URLs are provided manually. The command writes
`gsb-tellingsbestand.csv` and `csb-tellingsbestand.csv`."
}

fn compare_results_help_text() -> &'static str {
    "Usage:
  pv compare-results <election|year> <municipality> [options]

Examples:
  cargo run -- compare-results 2026-GR 0344
  cargo run -- compare-results 2026-GR 0344 --station 111

Options:
  --results-dir <dir>       Markdown result directory. Default: <election>/<municipality>/results.
  --corrections-dir <dir>   Correction OCR directory. Default: <election>/<municipality>/results/corrections.
  --output <file>           Also write a Markdown report to this file.
  --station <number>        Compare and rewrite the mismatch report for one polling station.
                            Repeat to check several specific stations.
  --format <terminal|markdown>
                            Output format. Default: terminal.
  --debug                   Print progress logs to stderr while comparing and
                            writing mismatch reports.

The command prefers a first-count per-list CSV at
`<election>/<municipality>/results/official/first-count-tellingsbestand.csv`.
If that file is absent, it falls back to the station-level GSB CSV written by
`official-csvs` at
`<election>/<municipality>/results/official/gsb-tellingsbestand.csv`. The GSB
CSV is the OSV4-3 central/candidate count; for `_eerste_telling` Markdown files
the command reverses per-station corrections from
`<election>/<municipality>/results/corrections/` before comparing. Missing or
invalid correction OCR marks that row `incomplete`. Status values are `missing`,
`incomplete`, `correction inconsistent`, `internally inconsistent`,
`fully matches`, and `mismatch`. Full failure details and highlighted table crops are written under
`<election>/<municipality>/results/mismatches/`. Missing rows include official
stations without Markdown and Markdown files whose station number is not present
in the official CSV."
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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

    #[test]
    fn official_csvs_defaults_to_utrecht_urls() {
        let options = OfficialCsvOptions {
            election: "2026-GR".to_owned(),
            municipality: "0344".to_owned(),
            out_dir: None,
            gsb_url: None,
            csb_url: None,
            force: false,
        };
        let sources = official_csv_sources(&options).unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].file_name, "gsb-tellingsbestand.csv");
        assert_eq!(sources[0].url, UTRECHT_GSB_CSV_URL);
        assert_eq!(sources[1].file_name, "csb-tellingsbestand.csv");
        assert_eq!(sources[1].url, UTRECHT_CSB_CSV_URL);
    }

    #[test]
    fn official_csvs_requires_manual_urls_without_defaults() {
        let options = OfficialCsvOptions {
            election: "2026-GR".to_owned(),
            municipality: "9999".to_owned(),
            out_dir: None,
            gsb_url: None,
            csb_url: None,
            force: false,
        };
        let error = official_csv_sources(&options).unwrap_err().to_string();
        assert!(error.contains("pass both --gsb-url and --csb-url"));
    }

    #[test]
    fn parses_station_level_official_csv() {
        let mut csv = String::from(
            "\"Lijstnummer\";\"Aanduiding\";\"Volgnummer\";\"Naam kandidaat\";\"Totaal\";\"School A\";\"School B\"\n\
             \"Gebiednummer\";;;;;\"1\";\"2\"\n",
        );
        for index in 1..=20 {
            csv.push_str(&format!(
                "\"{index}\";\"Party {index}\";;;\"{}\";\"{}\";\"{}\"\n",
                index * 3,
                index,
                index * 2
            ));
        }
        csv.push_str(
            ";\"geldige stembiljetten\";;;\"63\";\"21\";\"42\"\n\
             ;\"blanco stembiljetten\";;;\"3\";\"1\";\"2\"\n\
             ;\"ongeldige stembiljetten\";;;\"3\";\"1\";\"2\"\n\
             ;\"aangetroffen stembiljetten\";;;\"69\";\"23\";\"46\"\n",
        );

        let path = write_test_file("official.csv", &csv);
        let official = read_official_results_csv(&path).unwrap();
        let _ = fs::remove_file(path);

        assert_eq!(official.station_order, ["1", "2"]);
        assert_eq!(official.stations["1"].location, "School A");
        assert_eq!(official.stations["1"].values["E.1"], 1);
        assert_eq!(official.stations["2"].values["E.20"], 40);
        assert_eq!(official.stations["2"].values["H"], 46);
    }

    #[test]
    fn compares_markdown_against_official_values() {
        let path = write_test_file("Test_1_School_A.md", &valid_votes_table());
        let official = OfficialStationResult {
            location: "School A".to_owned(),
            values: parse_votes_markdown_values(&valid_votes_table()),
        };

        let row = compare_one_markdown(&path, "1", &official, None, false);
        assert_eq!(row.status, ComparisonStatus::FullyMatches);

        let mut changed_official = official;
        changed_official.values.insert("E.1".to_owned(), 2);
        let row = compare_one_markdown(&path, "1", &changed_official, None, false);
        let _ = fs::remove_file(path);

        assert_eq!(row.status, ComparisonStatus::Mismatch);
        assert!(row.details.contains("E.1: md=1, official=2"));
    }

    #[test]
    fn highlights_exact_official_mismatches_for_internal_errors() {
        let table = valid_votes_table().replace("| E.1 | 1 |", "| E.1 | 2 |");
        let path = write_test_file("Test_1_School_A.md", &table);
        let official = OfficialStationResult {
            location: "School A".to_owned(),
            values: parse_votes_markdown_values(&valid_votes_table()),
        };

        let row = compare_one_markdown(&path, "1", &official, None, false);
        let highlight_rows = highlight_rows_for_row(&row);
        let _ = fs::remove_file(path);

        assert_eq!(row.status, ComparisonStatus::InternallyInconsistent);
        assert!(row.details.contains("E.1 through E.20 sum"));
        assert!(row.details.contains("E.1: md=2, official=1"));
        assert_eq!(highlight_rows.official, BTreeSet::from(["E.1".to_owned()]));
        assert_eq!(highlight_rows.internal, BTreeSet::from(["E".to_owned()]));
    }

    #[test]
    fn shortens_terminal_reasons() {
        assert_eq!(
            shorten_reason("E.1: md=198, official=199; E.2: md=107, official=106"),
            "E.1 198->199; E.2 107->106"
        );
        assert_eq!(
            shorten_reason("E.1 through E.20 sum to 499, but E is 0; E + F + G is 0, but H is 500"),
            "sum(E.1-E.20) 499!=0; E+F+G 0!=500"
        );
        assert_eq!(
            shorten_reason("E.1: md=1, official=2; E.2: md=3, official=4; E.3: md=5, official=6"),
            "E.1 1->2; E.2 3->4; +1"
        );
    }

    #[test]
    fn validates_and_parses_correction_markdown() {
        let markdown = "\
| ID | First | Second | Difference | Note |
|---|---:|---:|---:|---|
| 1 | 247 | 248 | +1 | |
| blanco | 4 | 3 | -1 | stembiljet blanco/niet blanco |
";

        let report = validate_corrections_markdown(markdown);
        assert!(report.passed, "{:?}", report.errors);
        let corrections = parse_correction_markdown_values(markdown);
        assert_eq!(corrections["E.1"].first, Some(247));
        assert_eq!(corrections["E.1"].second, Some(248));
        assert_eq!(corrections["E.1"].difference, 1);
        assert_eq!(corrections["F"].difference, -1);
    }

    #[test]
    fn reverses_round_two_official_values_with_corrections() {
        let correction_path = write_test_file(
            "Test_1_School_A.md",
            "\
| ID | First | Second | Difference | Note |
|---|---:|---:|---:|---|
| E.1 | 1 | 2 | 1 | |
",
        );
        let correction_markdown = fs::read_to_string(&correction_path).unwrap();
        let correction = parse_correction_document(&correction_path, &correction_markdown);
        let mut round_two_values =
            parse_votes_markdown_values(&valid_votes_table().replace("| E.1 | 1 |", "| E.1 | 2 |"));
        round_two_values.insert("E".to_owned(), 21);
        round_two_values.insert("H".to_owned(), 26);
        let official = OfficialStationResult {
            location: "School A".to_owned(),
            values: round_two_values,
        };
        let path = write_test_file("Test_1_School_A_eerste_telling.md", &valid_votes_table());

        let row = compare_one_markdown(&path, "1", &official, Some(&correction), true);
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(correction_path);

        assert_eq!(row.status, ComparisonStatus::FullyMatches);
        assert_eq!(row.official_values["E.1"], 1);
        assert_eq!(row.official_values["E"], 20);
        assert_eq!(row.official_values["H"], 25);
    }

    #[test]
    fn marks_correction_table_conflicts_separately() {
        let correction_path = write_test_file(
            "Test_1_School_A.md",
            "\
| ID | First | Second | Difference | Note |
|---|---:|---:|---:|---|
| E.16 | 13 | 23 | 10 | |
",
        );
        let correction_markdown = fs::read_to_string(&correction_path).unwrap();
        let correction = parse_correction_document(&correction_path, &correction_markdown);
        let table = valid_votes_table()
            .replace("| E.13 | 1 |", "| E.13 | 13 |")
            .replace("| E.16 | 1 |", "| E.16 | 23 |")
            .replace("| E | 20 |", "| E | 54 |")
            .replace("| H | 25 |", "| H | 59 |");
        let mut round_two_values = parse_votes_markdown_values(&table);
        round_two_values.insert("E.13".to_owned(), 23);
        recompute_aggregate_vote_totals(&mut round_two_values);
        let official = OfficialStationResult {
            location: "School A".to_owned(),
            values: round_two_values,
        };
        let path = write_test_file("Test_1_School_A_eerste_telling.md", &table);

        let row = compare_one_markdown(&path, "1", &official, Some(&correction), true);
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(correction_path);

        assert_eq!(row.status, ComparisonStatus::CorrectionInconsistent);
        assert!(
            row.details
                .contains("correction E.16 first=13, Markdown=23, second=23")
        );
        assert!(row.details.contains("maybe belongs to E.13"));
        assert!(row.details.contains("E.13: md=13, official=23"));
        assert!(row.details.contains("E.16: md=23, official=13"));
    }

    #[test]
    fn missing_correction_is_not_incomplete_when_round_two_matches() {
        let official = OfficialStationResult {
            location: "School A".to_owned(),
            values: parse_votes_markdown_values(&valid_votes_table()),
        };
        let path = write_test_file("Test_1_School_A_eerste_telling.md", &valid_votes_table());

        let row = compare_one_markdown(&path, "1", &official, None, true);
        let _ = fs::remove_file(path);

        assert_eq!(row.status, ComparisonStatus::FullyMatches);
        assert!(row.details.is_empty());
    }

    #[test]
    fn marks_first_count_comparison_incomplete_without_needed_correction() {
        let official = OfficialStationResult {
            location: "School A".to_owned(),
            values: parse_votes_markdown_values(
                &valid_votes_table().replace("| E.1 | 1 |", "| E.1 | 2 |"),
            ),
        };
        let path = write_test_file("Test_1_School_A_eerste_telling.md", &valid_votes_table());

        let row = compare_one_markdown(&path, "1", &official, None, true);
        let _ = fs::remove_file(path);

        assert_eq!(row.status, ComparisonStatus::Incomplete);
        assert_eq!(row.details, "missing correction OCR");
    }

    fn write_test_file(name: &str, content: &str) -> PathBuf {
        let counter = TEST_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "pv-test-{}-{}-{}-{name}",
            std::process::id(),
            counter,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, content).unwrap();
        path
    }
}

fn err<T>(message: impl Into<String>) -> Result<T> {
    Err(io::Error::new(io::ErrorKind::Other, message.into()).into())
}
