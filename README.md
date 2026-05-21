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
cargo run -- crop 2026 0344 \
  --pdf Utrecht_40_Speeltuin_Noordsepark_GR26_eerste_telling.pdf
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
`--force`:

```bash
cargo run -- ocr-votes 2026-GR 0344 \
  --image Utrecht_41_Buurtcentrum_De_Uithoek_GR26_eerste_telling \
  --force
```

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
