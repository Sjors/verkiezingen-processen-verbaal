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
