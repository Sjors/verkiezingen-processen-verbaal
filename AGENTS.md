# Agents Instructions

This project downloads and archives "Proces verbaal" (vote counting) documents published by Dutch municipalities after elections.

## Project Structure

```
2025-TK/           # 2025 Tweede Kamer (parliament) elections
  README.md        # Election-level notes and Kiesraad link
   TODO.md          # Problematic municipalities only
  0855/            # Municipality code
      .lock          # Lock file while processing
         .todo          # URL for municipality (one line; when no SHA256SUMS)
    config.txt     # Download configuration
    README.md      # Municipality-specific notes
    *.pdf          # Downloaded documents
    SHA256SUMS     # File checksums
```

## Setting up a new election

When a new election occurs:

1. **Create the election folder**
   ```bash
   mkdir 2025-TK
   ```

2. **Create the election README.md**
   Include the Kiesraad results page URL:
   ```markdown
   # 2025-TK Tweede Kamerverkiezing

   - Election date: 29 oktober 2025
   - Kiesraad results: https://www.kiesraad.nl/verkiezingen/tweede-kamer/uitslagen/uitslagen-per-gemeente-tweede-kamer
   ```

3. **Initialize TODO.md (problems only)**
   Start with just the header (and the retention note if needed). Do **not** populate it with all municipalities. Add entries only when a municipality is blocked or needs follow-up.

4. **Populate initial .todo files (once per election cycle)**
   Run this **once** after the election folder exists to seed `.todo` files:
   ```bash
   ./scripts/populate_todo_urls.py
   ```
   Do **not** run this repeatedly during normal batch work unless explicitly asked to repair `.todo`/TODO.md mismatches.

5. **Copy configs from previous election**
   For each municipality that participated in a previous election, copy only the `config.txt` and `README.md`:
   ```bash
   for code in $(ls 2024-EP/); do
     if [[ -d "2024-EP/$code" ]]; then
       mkdir -p "2025-TK/$code"
       cp "2024-EP/$code/config.txt" "2025-TK/$code/" 2>/dev/null || true
       cp "2024-EP/$code/README.md" "2025-TK/$code/" 2>/dev/null || true
     fi
   done
   ```
   Do NOT copy PDF files, URL lists, or SHA256SUMS.

## TODO Tracking

`{election}/TODO.md` only lists **problematic municipalities**. Each entry is a `## {code} {name}` heading followed by notes. **Do not** include URL lines in TODO.md.

- Store the municipality URL in `{election}/{code}/.todo` (single-line file) for **any** municipality without `SHA256SUMS` (not just problematic ones).
- For entries in TODO.md, `.todo` must exist and contain the URL.
- When a municipality is completed and `SHA256SUMS` exists, remove the `.todo` file with `rm 2025-TK/{code}/.todo`.
- A municipality not in TODO.md may be unprocessed or already completed; use `.lock` and `SHA256SUMS` for progress tracking.
- URLs can always be retrieved from https://www.kiesraad.nl/verkiezingen/tweede-kamer/uitslagen/uitslagen-per-gemeente-tweede-kamer
- `SHA256SUMS` must never include `.lock` or `.todo` entries; if it does, regenerate after removing those files.
- Always delete `.lock` and `.todo` using `rm` (never via patches).
- Do not hand-edit `SHA256SUMS`; regenerate via `./scripts/fetch-pv.py` if needed.

## Timing constraints

Municipalities are legally required to retain processen-verbaal for **3 months** after an election. After that, they may delete them.

When searching for processen-verbaal:
- **< 2 weeks after election**: Files may not be uploaded yet. Note in TODO and retry later.
- **2 weeks – 3 months after election**: Files should be available. If not found, note the URLs searched in TODO.
- **> 3 months after election**: Files may have been deleted. Note "likely deleted" in TODO and move on.

## Batch processing

When processing multiple municipalities, handle them **one at a time** (not in a loop). This makes it easier to:
- Diagnose failures for individual municipalities
- Allow the user to follow along
- Track progress via the TODO list

**Finish with a summary:** After processing multiple municipalities, end your response with a brief summary of what completed, what was partial/blocked, and any follow-up needed. Use a table with one row per municipality.

**Sanity check:** Do **not** run `./scripts/populate_todo_urls.py` after each batch. It should be used **once per election cycle** (or only when explicitly asked to repair `.todo`/TODO.md mismatches).

**Quick progress check:** Run `./scripts/show-progress.py --election 2025-TK` anytime to display the current finished, in-progress, and TODO counts without scanning the directories manually.

### Agent todo list (required)

In addition to `{election}/TODO.md`, maintain a separate **agent todo list** (the assistant task list) so progress is visible at a glance.

**Requirements:**
- Create the todo list before starting a batch.
- Add one item per municipality in the batch (use the CBS code and name).
- Mark each item as completed immediately after finishing that municipality.
- If a municipality is blocked/partial, still mark the item completed and note the reason in the todo item text.
- Do not end the session with all items still marked in-progress.

### Selecting municipalities (with locking for parallel agents)

To select a batch of random municipalities from the directory list, **omit**:
- directories with a `.lock`
- directories with a `SHA256SUMS`
- directories mentioned in TODO.md

Example:
```bash
./scripts/select_random_municipalities.py 2025-TK 10
```

**Note:** If other agents have already locked all remaining municipalities (or all remaining ones are already in TODO.md), the selection script will report “No eligible municipalities found.” In that case, ask the user how to proceed. If it returns fewer than N results, that's fine, just work on the smaller set.

**Random rule (important):** If the random results include **any** of the 5 biggest municipalities by population — **Amsterdam, Rotterdam, Den Haag, Utrecht, Eindhoven** — then select **only that single municipality** and drop the rest of the batch. The script prints a notice to stderr when this happens; state explicitly in your response that you did this because a top-5 municipality was present.

**Before starting work**, lock **all chosen municipalities** by creating a `.lock` file in each directory:
```bash
touch 2025-TK/{code}/.lock
```

**Safe TODO.md updates (concurrent edits):**
- Re-read the latest TODO.md immediately before each edit.
- Edit only the specific municipality block you are working on (avoid broad replacements or reformatting).
- If the block has changed (notes added, removed, or the entry disappeared), do **not** overwrite it; re-select another municipality.
- TODO.md should contain **notes only** (no URL lines). URLs live in `.todo` files.

### Processing workflow

Process each municipality sequentially. **After completing each municipality** (before moving to the next):
1. Ensure `2025-TK/{code}/.lock` exists before starting; remove it with `rm 2025-TK/{code}/.lock` when done.
2. Ensure `2025-TK/{code}/.todo` exists and contains the URL before running fetches.
3. If successful: remove the municipality entry from TODO.md (if present) and remove `2025-TK/{code}/.todo` with `rm`.
4. If failed/partial: add/update the TODO.md entry with notes (no URL lines) and ensure `2025-TK/{code}/.todo` contains the URL.
5. Mark the todo item as completed in your todo list.
6. Verify the download worked before moving to the next.

**Important**: Update TODO.md incrementally after each municipality, not in a batch at the end. This ensures progress is saved and other agents see accurate state.

## Task list for each municipality

For each municipality:

1. **Find the processen-verbaal page**
   - If `2025-TK/{code}/.todo` exists, start from that URL
   - Otherwise, locate the URL via config or the municipality website
   - Look for links like "Processen-verbaal", "Uitslag per stembureau", or similar
   - Note: Some sites have PDFs directly on the main page, others have a dedicated subpage

2. **Create the municipality folder and config**
   ```bash
   mkdir -p 2025-TK/{code}
   ```
   Create `config.txt` with:
   ```
   URL=<page containing PDF links>
   REGEX=<regex to match PDF links>
   PREFIX=<prefix for relative URLs>
   NAME=<municipality name>
   ```

3. **Run the fetch script**
   ```bash
   ./scripts/fetch-pv.py 2025-TK/{code}
   ```
   Do not call the venv binary directly (e.g. `.venv/bin/python`) — it triggers permission prompts.
   This will:
   - Generate a URL list file (`{code} {name}.txt`) if missing
   - Download all PDFs (skipping existing files)
   - Generate `SHA256SUMS` or verify existing checksums

4. **Sanity check against previous elections**
   Compare the number of stembureaus with previous years:
   ```bash
   wc -l 2023-TK/{code}*.txt 2024-EP/{code}*.txt 2025-TK/{code}/{code}*.txt
   ```
   - Some variation (±20%) is normal due to population changes
   - Large drops (>50%) may indicate missing files or wrong regex
   - File naming conventions may differ between years

5. **Update README.md with stembureau counts**
   Create or update `README.md` in the municipality folder:
   ```markdown
   # {name}

   ## Stembureaus per verkiezing

   | Verkiezing | Stembureaus |
   |------------|-------------|
   | 2023-TK    | XX          |
   | 2024-EP    | XX          |
   | 2025-TK    | XX          |
   ```
      Omit rows with a count of 0 (it means that election was not downloaded).

6. **Verify and clear tracking**
   - Check the downloaded PDFs are correct proces-verbaal documents
   - If only summary PDFs are found (no per-stembureau PVs), treat as partial: keep the TODO.md entry and `.todo`, and remove any `SHA256SUMS` created by the partial fetch
   - Remove the municipality from TODO.md (if present)
   - Remove `2025-TK/{code}/.todo` on success with `rm 2025-TK/{code}/.todo`

7. **Commit** (only when requested by user)
   ```bash
   git add 2025-TK/{code} && git commit -m "{code} {name}"
   ```

## Common config patterns

| Municipality Type | REGEX Pattern | Notes |
|-------------------|---------------|-------|
| Standard PDFs | `proces.*verbaal.*\.pdf` | Case-insensitive |
| TYPO3/Fileadmin | `fileadmin/.*\.pdf` | Common CMS |
| Drupal | `sites/.*/files/.*\.pdf` | Common CMS |
| sim-cdn.nl | `sim-cdn\.nl/.*/uploads/.*\.pdf` | CDN hosting |
| SDU/dsresource | `dsresource\?objectid=.*type=pdf` | See below |

### dsresource URLs (SDU CMS)

Some municipalities use SDU's CMS which serves files via URLs like:
```
https://www.gemeente.nl/dsresource?objectid=abc123-def456&type=pdf
```

The `fetch-pv.py` script handles this automatically by extracting the filename from the `Content-Disposition` header.

Example config:
```
URL=https://www.vlaardingen.nl/Bestuur/Verkiezingen/Tweede_Kamerverkiezing_2025
REGEX=dsresource\?objectid=.*type=pdf
PREFIX=https://www.vlaardingen.nl/
NAME=Vlaardingen
```

### Centraal Tellen (Central Counting)

Some municipalities use "centraal tellen" (central counting) where votes from all polling stations are counted together at a central location the day after the election. In this case:

- There is typically **one combined PDF** for all stembureaus instead of individual files
- The stembureau count in README.md should note this: `1 (centraal tellen)` or similar
- The number of stembureaus will appear much lower than previous years

This is a valid approach and not an error - just document it in the README.md.

### StackStorage

Some municipalities host files on StackStorage (e.g., `technischbeheerassen.stackstorage.com`). These sites use JavaScript to render file listings and cannot be scraped automatically.

For StackStorage municipalities:
1. Note the storage URLs in config.txt as comments
2. Add a note in TODO.md that manual download is required and ensure `.todo` contains the URL
3. See https://github.com/Sjors/verkiezingen-processen-verbaal/issues/55 for potential automation

## Mijn Stembureau API

Some municipalities use the "Mijn Stembureau" platform (e.g., `mijnstembureau-{gemeente}.nl`). These require a different approach:

1. **Create config.txt with a note**
   ```
   # Uses Mijn Stembureau API
   # Download with: ./scripts/fetch-mijnstembureau.py <url> <dir>
   NAME=Losser
   ```

2. **Run the dedicated script**
   ```bash
   ./scripts/fetch-mijnstembureau.py https://mijnstembureau-losser.nl 2025-TK/0168
   ```

3. **Add a note to README.md**
   ```markdown
   # Losser

   Uses [Mijn Stembureau](https://mijnstembureau-losser.nl/) platform.

   Download with:
   ```bash
   ./scripts/fetch-mijnstembureau.py https://mijnstembureau-losser.nl 2025-TK/0168
   ```
   ```

The script:
- Fetches election data from the API
- Selects the most recent election (or use `--election "2025"` to filter)
- Downloads all proces-verbaal PDFs
- Generates SHA256SUMS

## Troubleshooting

- **No URLs found**: Check the REGEX pattern, try a simpler pattern like `\.pdf`
- **403 Forbidden**: Some sites block automated requests, may need manual download
- **Missing PREFIX**: Check if links are relative or absolute in the HTML source
- **Mijn Stembureau API error**: Check if API structure changed, update script if needed

## Ad hoc scripts and manual steps

Ad hoc scripts are OK for debugging, but if they prove necessary for a municipality or recur across sites:

- Prefer updating the main scripts (e.g., `scripts/fetch-pv.py`) to handle the pattern generally.
- If a one-off workaround remains, document the exact steps and rationale in README.md (at the election level or the municipality folder) so future runs are reproducible.

## Command patterns for auto-approval

To minimize permission prompts in VS Code, use these consistent command patterns.
Always run commands from the repo root and use only relative paths (both in the command and arguments). Do not use absolute paths.

**Fetching pages**: Do NOT use `curl ... | grep` pipelines - they cannot be auto-approved. Instead, download to a temp file and inspect separately. Use friendly filenames (letters/numbers/underscores; avoid hyphens or leading dashes) to prevent `sed` safety blocks. Use a Safari UA to avoid WAF blocks:
```bash
mkdir -p tmp
ua='Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15'
curl -sL -A "$ua" "https://www.gemeente.nl/verkiezingen" -o tmp/gemeente.html
grep -i "pdf" tmp/gemeente.html
```
This also avoids repeated downloads when trying different grep patterns.

**Quote-safe grep**: When searching for PDFs, avoid unescaped quotes in the pattern. Prefer a fixed-string search or wrap patterns with double quotes inside **single quotes**. If you end up at a `dquote>` prompt, press Ctrl-C to reset.
```bash
rg -i --fixed-strings ".pdf" tmp/gemeente.html
grep -oi 'https:[^"]*\.pdf' tmp/gemeente.html | head
```

**Running fetch scripts (auto-approved):**
```bash
./scripts/fetch-pv.py 2025-TK/{code}
./scripts/fetch-mijnstembureau.py https://mijnstembureau-{gemeente}.nl 2025-TK/{code}
```
**Never** use `.venv/bin/python` directly; always run the scripts from the repo root to avoid permission prompts.

## Municipality Codes

Dutch municipalities have a 4-digit CBS code. Find them at:
https://nl.wikipedia.org/wiki/Lijst_van_Nederlandse_gemeenten

### BES Islands (Caribbean Netherlands)

The three Caribbean "bijzondere gemeenten" participate in **Tweede Kamer elections only** (not EU elections):

| Folder | Name | Website |
|--------|------|---------|
| `BES-Bonaire` | Bonaire | https://bonairestemt.nl/ |
| `BES-Saba` | Saba | https://www.sabagov.nl/ |
| `BES-SintEustatius` | Sint Eustatius | https://www.statiagovernment.com/ |

### Municipality Name Aliases

Some municipalities have official names that differ from common usage:

| Official Name | Common Name |
|--------------|-------------|
| 's-Gravenhage | Den Haag |
| 's-Hertogenbosch | Den Bosch |

Some municipalities include province disambiguation:
- Bergen (Noord-Holland) vs Bergen (Limburg)

## What to Download

We want **Proces verbaal** documents - the official vote counting forms from each polling station (stembureau). These are typically:

- Named like `{gemeente}_{nummer}_{locatie}_TK25.pdf`
- One per polling station
- Multi-page PDFs containing detailed vote counts

We do NOT want:
- `osv4-3` summary CSV/XML files
- "Uitslagen na controle" (post-verification results)
- Corrigendum documents (corrections) — though we do archive these if present
