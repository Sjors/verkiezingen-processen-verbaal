# Processen-verbaal verkiezingen

Archives URLs and documents from Dutch municipal "proces-verbaal" (vote counting) files published after elections.

For agent instructions, processing workflows, and automation details, see [AGENTS.md](AGENTS.md).

## Setup

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

## Structure

```
2026-GR/           # Election folder (year-type)
  README.md        # Election-level notes
  TODO.md          # Problematic municipalities only
  {code}/          # CBS municipality code
    config.txt     # Download configuration
    README.md      # Municipality notes and stembureau counts
    *.pdf          # Downloaded documents
    SHA256SUMS     # File checksums
```

## Checking progress

```bash
./scripts/show-progress.py --election 2026-GR
```

## Rust utilities

This repository includes a Rust CLI for processing election documents. The
long-term direction is to move the repository's Python and Bash tooling into
Rust, but the current Rust surface is intentionally scoped to OCR preparation.

Build and check the external toolchain:

```bash
cargo run -- doctor
```

The current crop workflow requires Poppler's `pdfimages` and `pdftotext`.
`doctor` checks that both are available on `PATH`, and `crop` runs the same
check before it starts.

Crop one Utrecht first-count PDF:

```bash
cargo run -- crop 2026 0344 --station 40
```

Crop all first-count PDFs for a municipality:

```bash
cargo run -- crop 2026-GR 0344
```

By default, full table crops are written to
`{election}/{municipality}/crops/2.2/`, and narrow OCR-focused crops are written
to `{election}/{municipality}/crops/2.2/narrow/`. The crop directory is ignored
by git. Existing output files are treated as an error; remove old crops before
rerunning the command.

The crop command currently writes lossless PNG crops for table 2.2
`Uitgebrachte stemmen`. The full crop includes the table header and row
descriptions. The narrow crop contains only the handwritten number cells and
identifier column, including total rows E through H. It uses `pdftotext` to
locate the table page, then extracts the embedded page image with `pdfimages`
and crops those native pixels directly. It does not render through `pdftoppm`,
so the crop keeps the original scan resolution.

Second-count PDFs can also be cropped for the correction table that explains
changes from the first count:

```bash
cargo run -- crop 2026-GR 0344 --kind corrections
```

This writes `B1 - 2.4 Lijsten met verschillen` crops to
`{election}/{municipality}/crops/corrections/`.

To crop one correction table, use the same station shorthand:

```bash
cargo run -- crop 2026-GR 0344 --kind corrections --station 111
```

After cropping, run the narrow crops through a local multimodal LLM:

```bash
cargo run -- ocr-votes 2026-GR 0344
```

By default, `ocr-votes` reads PNG files from
`{election}/{municipality}/crops/2.2/narrow/` and writes Markdown tables to
`{election}/{municipality}/results/`. Each image is sent as a fresh chat
completion request. The prompt lives in
[`prompts/ocr-votes.md`](prompts/ocr-votes.md) so it can be adjusted without
editing Rust code.

The command validates every resulting Markdown file: it expects rows `E.1`
through `E.20`, `E`, `F`, `G`, and `H`, checks that values are digits, verifies
that `E.1` through `E.20` sum to `E`, and verifies that `E + F + G` equals `H`.
It prints a final PASS/FAIL table per voting-location crop.

To re-run one voting location, pass its crop stem or image path and use
`--force`. A polling station number is accepted directly:

```bash
cargo run -- ocr-votes 2026-GR 0344 --station 41 --force
```

Run correction crops through the LLM separately:

```bash
cargo run -- ocr-corrections 2026-GR 0344
```

By default, `ocr-corrections` reads from
`{election}/{municipality}/crops/corrections/` and writes Markdown tables to
`{election}/{municipality}/results/corrections/`. The prompt lives in
[`prompts/ocr-corrections.md`](prompts/ocr-corrections.md). Each correction row
has `ID`, `First`, `Second`, `Difference`, and `Note`; list numbers are
normalized to `E.1` through `E.20`, `blanco` to `F`, and `ongeldig` to `G`.

To OCR one correction table:

```bash
cargo run -- ocr-corrections 2026-GR 0344 --station 111 --force
```

Fetch official Utrecht tellingsbestand CSV files for OCR cross-checks:

```bash
cargo run -- official-csvs 2026-GR 0344
```

This writes `gsb-tellingsbestand.csv` and `csb-tellingsbestand.csv` to
`{election}/{municipality}/results/official/`. For now, the command only has
built-in URLs for Utrecht. Other municipalities require both URLs explicitly:

```bash
cargo run -- official-csvs 2026-GR 9999 \
  --gsb-url https://example.invalid/gsb.csv \
  --csb-url https://example.invalid/csb.csv
```

Current limitations:

- Built-in CSV discovery only exists for Utrecht (`2026-GR 0344`).
- Other municipalities fail unless both URLs are passed manually.
- Utrecht's published `gsb-tellingsbestand.csv` is the OSV4-3
  central/candidate-count file from the municipal counting board. It is not the
  first count on list level from the `_eerste_telling` stembureau PDFs.
- `official-csvs` only downloads the files; use `compare-results` to compare OCR
  Markdown to the matching station-level CSV.

Compare OCR Markdown results against the official station-level CSV:

```bash
cargo run -- compare-results 2026-GR 0344
```

To recheck one polling station and rewrite only that station's mismatch report:

```bash
cargo run -- compare-results 2026-GR 0344 --station 111
```

This prefers
`{election}/{municipality}/results/official/first-count-tellingsbestand.csv`.
If that file is absent, it falls back to the GSB CSV written by `official-csvs`
at `{election}/{municipality}/results/official/gsb-tellingsbestand.csv`. The GSB
CSV is the OSV4-3 central/candidate count. For `_eerste_telling` Markdown files,
`compare-results` reverses the station's correction OCR from
`{election}/{municipality}/results/corrections/` when the uncorrected round-2
CSV differs from the Markdown. If the matching correction OCR is then missing or
invalid, the row is marked `incomplete` instead of `mismatch`. It prints a
terminal-friendly table by default; use
`--format markdown` for a Markdown table. Use `--debug` to print progress logs
to stderr while it writes mismatch reports.

When candidate-level OCR files are present in
`{election}/{municipality}/results/candidates/`, `compare-results` also compares
those B1 section 3.5 candidate counts against the station-level OSV4-3 GSB CSV.
Stations without candidate OCR are marked `not checked` for the candidate
status and do not count as failures.

Rows have one of six statuses per station: `missing`, `incomplete`,
`correction inconsistent`, `internally inconsistent`, `fully matches`, or
`mismatch`. `correction inconsistent` means the correction table conflicts with
the first-count Markdown, for example when a correction row's first value does
not match the first-count table. Terminal reasons are kept short, with longer
failure details written to
`{election}/{municipality}/results/mismatches/` as one Markdown report per
failing polling station. Those reports include a highlighted copy of the full
table crop when the crop is available, with OCR and official CSV values printed
in the right margin for official mismatches. They also include the correction OCR
Markdown and correction crop when present, because a remaining mismatch can come
from OCR errors in either table. Yellow/red highlights mark rows that differ
from the corrected official CSV; blue highlights mark rows implicated only by
internal consistency checks.

The CSV is the source of truth for the polling station list; Markdown files in
`{election}/{municipality}/results/` are matched onto that list when present. The
CSB CSV is still downloaded for archiving, but Utrecht's CSB CSV currently
contains only municipality totals and is not used for station-level comparison.

## Hashes and timestamps

Each election has a signed manifest built from per-municipality `SHA256SUMS` files:

```bash
./scripts/build-election-manifest.py 2026-GR
gpg --yes --clearsign 2026-GR/2026-GR
```

To verify:

```bash
./scripts/build-election-manifest.py 2026-GR
gpg --verify 2026-GR/2026-GR.asc
ots verify 2026-GR/2026-GR.asc.ots
```

Timestamps can be verified at [opentimestamps.org](https://opentimestamps.org) or with [ots-client](https://github.com/opentimestamps/opentimestamps-client) and a Bitcoin node.
