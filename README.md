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

By default, crops are written to `{election}/{municipality}/crops/`. The crop
directory is ignored by git.

The crop command currently writes lossless PNG crops for table 2.2
`Uitgebrachte stemmen`. It uses `pdftotext` to locate the table page, then
extracts the embedded page image with `pdfimages` and crops those native pixels
directly. It does not render through `pdftoppm`, so the crop keeps the original
scan resolution.

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
