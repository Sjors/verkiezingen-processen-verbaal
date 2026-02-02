# OCR

This project uses small, preprocessed cell crops in tmp/ocr to compare OCR engines.
The gallery output is a Markdown table of the crops with the recognized digit and confidence.

## Generate the test gallery

Use the TrOCR script to produce Vision-like JSON, then render a gallery.

Activate the Python venv once per shell:

```bash
source .venv/bin/activate
```

With the venv active, you can run the scripts directly (they are executable):

```bash
./scripts/ocr-gallery.py --help
```

```bash
./.venv/bin/python scripts/ocr-trocr-cells.py \
  --image-dir tmp/ocr/0307-adventkerk-28_0-cells-prep5 \
  --out tmp/ocr/0307-adventkerk-28_0-cells-prep5-trocr.json \
  --model microsoft/trocr-large-printed \
  --num-beams 48 \
  --length-penalty 0.0 \
  --max-new-tokens 1 \
  --use-fast-processor \
  --device mps

./scripts/ocr-gallery.py \
  --image-dir tmp/ocr/0307-adventkerk-28_0-cells-prep5 \
  --ocr-json tmp/ocr/0307-adventkerk-28_0-cells-prep5-trocr.json

PARSeq runs by default unless you supply --parseq-json.
```

The gallery writes to gallery.md by default (no --out required).

## Prepare tiles from a PDF

If the PDF is a scanned document, prefer extracting the embedded 200 DPI JPEGs instead of
rendering to PNG. This keeps the original scan quality and makes Vision OCR more reliable.

```bash
pdfimages -list 2025-TK/0307/tk25-sb-adventkerk-28_0.pdf
pdfimages -j 2025-TK/0307/tk25-sb-adventkerk-28_0.pdf tmp/ocr/adventkerk
```

Pick the extracted image that contains the bijlage table (use `pdfimages -list` to map
pages to filenames). For example:

`tmp/ocr/adventkerk-009.jpg`

1) OCR the extracted JPEG with Vision:

```bash
swift scripts/ocr-vision.swift \
  --image tmp/ocr/BIJLAGE.jpg \
  --out tmp/ocr/BIJLAGE.vision.json
```

2) Detect the table grid lines on the extracted JPEG.
For single-image OCR output, the Vision JSON page number is always 1:

```bash
./.venv/bin/python scripts/detect-table-lines.py \
  --image tmp/ocr/BIJLAGE.jpg \
  --vision-json tmp/ocr/BIJLAGE.vision.json \
  --page 1 \
  --out-json tmp/ocr/BIJLAGE.lines-table.json \
  --out-image tmp/ocr/BIJLAGE.lines-table.png
```

3) Crop and preprocess the grid cells (less aggressive vertical trim):

```bash
./.venv/bin/python scripts/crop-grid-cells.py \
  --vision-json tmp/ocr/BIJLAGE.vision.json \
  --grid-json tmp/ocr/BIJLAGE.lines-table.json \
  --image tmp/ocr/BIJLAGE.jpg \
  --out-dir tmp/ocr/BIJLAGE-cells-prep5 \
  --map-json tmp/ocr/BIJLAGE-cells-prep5-map.json \
  --pad 0.18 \
  --row-inner 0.04 \
  --col-inner 0.05 \
  --page 1
```

4) Generate the gallery from the JPEG-based crops:

```bash
./scripts/ocr-gallery.py \
  --image-dir tmp/ocr/BIJLAGE-cells-prep5 \
  --ms-num-beams 20 \
  --max-new-tokens 1
```

If you OCR a full PDF instead of a single image, pass the real PDF page number to
`detect-table-lines.py --page` and `crop-grid-cells.py --page`.

## TrOCR models

Available Hugging Face model ids (large variants only):

- microsoft/trocr-large-handwritten
- microsoft/trocr-large-printed

## TrOCR tweaks

Most useful parameters:

- --ms-num-beams: beam search width for Microsoft OCR (higher may improve accuracy, slower).
- --length-penalty: adjust preference for longer/shorter outputs.
- --max-new-tokens: limit output length (defaults to 2 for single digits).
- --use-fast-processor: use the fast image processor backend.
- --ms-min-conf: filter low-confidence predictions for Microsoft OCR.
- --ambiguous-min-conf: minimum confidence for ambiguous glyph mappings (e.g., I→1).
- --device: cpu|mps|cuda.
- --empty-threshold: skip OCR when the tile has almost no ink (default 0.01; set to 0 to disable).
  Use this in ocr-trocr-cells.py; crop-grid-cells.py no longer blanks images.
- --empty-trim: trim fraction per side for empty checks (default 0.13, for pad 0.18).

## Vision OCR (optional)

For Apple Vision tests on the same crops:

```bash
swift scripts/ocr-vision.swift \
  --image-dir-input tmp/ocr/0307-adventkerk-28_0-cells-prep5 \
  --out tmp/ocr/0307-adventkerk-28_0-cells-prep5-vision.json \
  --candidates 10 \
  --level accurate \
  --no-language-correction \
  --min-text-height 0.01

./scripts/ocr-gallery.py \
  --image-dir tmp/ocr/0307-adventkerk-28_0-cells-prep5 \
  --ocr-json tmp/ocr/0307-adventkerk-28_0-cells-prep5-vision.json

To compare printed vs handwritten models in one gallery:

./scripts/ocr-gallery.py \
  --image-dir tmp/ocr/0307-adventkerk-28_0-cells-prep5 \
  --ms-num-beams 20 \
  --max-new-tokens 1

The combined view shows: left = printed model, right = handwritten model.
The higher-confidence value is bolded.

`max-new-tokens` limits the length of the generated string; it does not list candidates.
```

Vision tweaks:

- --level: accurate|fast.
- --candidates: number of candidate strings per observation.
- --min-text-height: minimum text height in normalized units.
- --no-language-correction: disable language correction.

## Gallery tweaks

The gallery always displays the first digit from OCR output.
- --cols: number of columns in the table (default 5).
- --out: output file path (default gallery.md).
- --parseq-threshold: use PARSeq at or below this confidence (default 0.6).
- --parseq-empty-conf: minimum PARSeq confidence to accept an explicit no-digit result (default 0.9).
- --parseq-min-conf: minimum PARSeq confidence to accept a digit (default 0.5).
- --parseq-beam-size: beam size for PARSeq decoding (default 20).
- PARSeq uses original color crops when available (auto-created next to the prep dir).
