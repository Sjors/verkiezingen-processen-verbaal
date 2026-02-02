#!/usr/bin/env python3
import argparse
import bisect
import json
import re
from pathlib import Path
from typing import Dict, List, Tuple


ROW_RE = re.compile(r"^\s*(\d{1,2})\.\s*(.+)$")


def norm_text(text: str) -> str:
    return re.sub(r"\s+", " ", text.strip()).lower()


def line_center_y(bbox: List[float]) -> float:
    x, y, w, h = bbox
    return y + (h / 2.0)


def line_center_x(bbox: List[float]) -> float:
    x, y, w, h = bbox
    return x + (w / 2.0)


def digits_from_text(text: str) -> List[str]:
    return re.findall(r"\d", text)


def is_numeric_text(text: str) -> bool:
    if re.search(r"[A-Za-z]", text):
        return False
    return bool(re.search(r"\d", text))


def extract_rows(lines: List[dict]) -> List[dict]:
    rows = []
    for line in lines:
        text = line.get("text", "")
        m = ROW_RE.match(text)
        if not m:
            continue
        list_number = int(m.group(1))
        list_name = m.group(2).strip()
        rows.append(
            {
                "list_number": list_number,
                "list_name": list_name,
                "bbox": line.get("bbox"),
            }
        )
    return rows


def extract_numeric_candidates(lines: List[dict], min_x: float) -> List[dict]:
    candidates = []
    for line in lines:
        text = line.get("text", "")
        bbox = line.get("bbox")
        if not bbox:
            continue
        if line_center_x(bbox) < min_x:
            continue
        if not is_numeric_text(text):
            continue
        digits = digits_from_text(text)
        if not digits:
            continue
        candidates.append({"text": text, "digits": digits, "bbox": bbox})
    return candidates


def vision_bbox_to_tl_px(bbox: List[float], width: int, height: int) -> Tuple[float, float, float, float]:
    x, y, w, h = bbox
    x1 = x * width
    y1 = (1.0 - (y + h)) * height
    x2 = (x + w) * width
    y2 = y1 + (h * height)
    return x1, y1, x2, y2


def digit_positions_px(
    lines: List[dict], width: int, height: int, min_x_px: float
) -> List[Tuple[float, float, str]]:
    positions: List[Tuple[float, float, str]] = []
    for line in lines:
        text = line.get("text", "")
        bbox = line.get("bbox")
        if not bbox:
            continue
        if not is_numeric_text(text):
            continue
        digits = digits_from_text(text)
        if not digits:
            continue
        x1, y1, x2, y2 = vision_bbox_to_tl_px(bbox, width, height)
        cx = (x1 + x2) / 2.0
        cy = (y1 + y2) / 2.0
        if cx < min_x_px:
            continue
        if len(digits) == 1:
            positions.append((cx, cy, digits[0]))
        else:
            w = x2 - x1
            for i, d in enumerate(digits):
                sub_x = x1 + (i + 0.5) * (w / len(digits))
                positions.append((sub_x, cy, d))
    return positions


def digit_positions(candidates: List[dict]) -> List[Tuple[float, float, str]]:
    positions: List[Tuple[float, float, str]] = []
    for cand in candidates:
        bbox = cand["bbox"]
        digits = cand["digits"]
        x, yb, w, h = bbox
        y = line_center_y(bbox)
        if len(digits) == 1:
            positions.append((line_center_x(bbox), y, digits[0]))
        else:
            for i, d in enumerate(digits):
                sub_x = x + (i + 0.5) * (w / len(digits))
                positions.append((sub_x, y, d))
    return positions


def kmeans_1d(values: List[float], k: int) -> List[float]:
    if not values:
        return []
    values_sorted = sorted(values)
    if len(values_sorted) < k:
        return values_sorted

    centers = []
    for i in range(k):
        idx = int((i + 0.5) / k * (len(values_sorted) - 1))
        centers.append(values_sorted[idx])

    for _ in range(10):
        groups = [[] for _ in range(k)]
        for v in values_sorted:
            nearest = min(range(k), key=lambda i: abs(v - centers[i]))
            groups[nearest].append(v)
        for i in range(k):
            if groups[i]:
                centers[i] = sum(groups[i]) / len(groups[i])

    return sorted(centers)


def compute_column_centers(rows: List[dict], candidates: List[dict], fallback_min_x: float) -> List[float]:
    if not rows:
        return []
    row_ys = [line_center_y(r["bbox"]) for r in rows if r.get("bbox")]
    if not row_ys:
        return []
    y_min = min(row_ys) - 0.05
    y_max = max(row_ys) + 0.05

    positions = digit_positions(candidates)
    xs = [x for x, y, _ in positions if y_min <= y <= y_max and x >= fallback_min_x]
    if len(xs) < 5:
        min_x = min(xs) if xs else fallback_min_x
        max_x = max(xs) if xs else 0.95
        step = (max_x - min_x) / 4.0 if max_x > min_x else 0.05
        return [min_x + i * step for i in range(5)]

    centers = kmeans_1d(xs, 5)
    if len(centers) < 5:
        min_x = min(xs)
        max_x = max(xs)
        step = (max_x - min_x) / 4.0 if max_x > min_x else 0.05
        return [min_x + i * step for i in range(5)]
    return centers


def assign_to_columns(digits: List[Tuple[float, str]], centers: List[float]) -> List[str]:
    cols = [None] * 5
    col_dist = [None] * 5
    for x, d in digits:
        if not centers:
            continue
        idx = min(range(len(centers)), key=lambda i: abs(x - centers[i]))
        dist = abs(x - centers[idx])
        if idx >= 5:
            continue
        if cols[idx] is None or (col_dist[idx] is not None and dist < col_dist[idx]):
            cols[idx] = d
            col_dist[idx] = dist
    return [c or "" for c in cols]


def digits_for_y(positions: List[Tuple[float, float, str]], y: float, tol: float) -> List[Tuple[float, str]]:
    return [(x, d) for x, yy, d in positions if abs(yy - y) <= tol]


def first_non_empty_index(cols: List[str]) -> int | None:
    for i, value in enumerate(cols):
        if value:
            return i
    return None


def shift_columns(cols: List[str], shift: int) -> List[str]:
    if shift <= 0:
        return cols
    return cols[shift:] + [""] * shift


def find_line(lines: List[dict], needle: str) -> dict | None:
    needle_norm = norm_text(needle)
    for line in lines:
        text = norm_text(line.get("text", ""))
        if needle_norm in text:
            return line
    return None


def extract_total_digits(
    lines: List[dict],
    positions: List[Tuple[float, float, str]],
    label: str,
    centers: List[float],
) -> List[str] | None:
    label_line = find_line(lines, label)
    if not label_line:
        return None
    bbox = label_line.get("bbox")
    if not bbox:
        return None
    y = line_center_y(bbox)
    tol = max(0.015, bbox[3] * 2.0)
    digits = digits_for_y(positions, y, tol)
    if not digits:
        return None
    return assign_to_columns(digits, centers)


def select_digit_columns(vert_lines: List[int]) -> List[Tuple[int, int]]:
    vert_lines = sorted(vert_lines)
    if len(vert_lines) < 6:
        return []
    rightmost = vert_lines[-6:]
    return [(rightmost[i], rightmost[i + 1]) for i in range(5)]


def row_bounds_from_lines(horiz_lines: List[int], y: float) -> Tuple[int, int] | None:
    if len(horiz_lines) < 2:
        return None
    idx = bisect.bisect_right(horiz_lines, y) - 1
    if idx < 0 or idx >= len(horiz_lines) - 1:
        return None
    return horiz_lines[idx], horiz_lines[idx + 1]


def digits_for_row_columns(
    positions: List[Tuple[float, float, str]],
    row_bounds: Tuple[int, int],
    col_bounds: List[Tuple[int, int]],
) -> List[str]:
    y1, y2 = row_bounds
    row_h = max(1, y2 - y1)
    row_pad = max(3, int(row_h * 0.25))

    row_candidates: List[Tuple[float, float, str, int]] = []
    for idx, (x, y, d) in enumerate(positions):
        if y1 - row_pad <= y <= y2 + row_pad:
            row_candidates.append((x, y, d, idx))

    cols: List[str] = [""] * 5
    used: set[int] = set()
    for i, (x1, x2) in enumerate(col_bounds):
        col_w = max(1, x2 - x1)
        col_pad = max(3, int(col_w * 0.25))
        candidates = []
        for x, y, d, idx in row_candidates:
            if idx in used:
                continue
            if x1 - col_pad <= x <= x2 + col_pad:
                candidates.append((abs(x - ((x1 + x2) / 2.0)), idx, d))
        if candidates:
            candidates.sort(key=lambda v: v[0])
            _, idx, d = candidates[0]
            cols[i] = d
            used.add(idx)

    remaining = sorted(
        [(x, idx, d) for x, _, d, idx in row_candidates if idx not in used],
        key=lambda v: v[0],
    )
    if remaining:
        for i in range(len(cols)):
            if cols[i] == "" and remaining:
                _, idx, d = remaining.pop(0)
                cols[i] = d
                used.add(idx)

    return cols


def init_handwriting_ocr(engine: str, model_name: str, device: str | None = None):
    if engine == "easyocr":
        import easyocr

        reader = easyocr.Reader(["en"], gpu=False, verbose=False)
        return {"engine": "easyocr", "reader": reader}

    from transformers import TrOCRProcessor, VisionEncoderDecoderModel
    import torch

    if device is None:
        if torch.backends.mps.is_available():
            device = "mps"
        else:
            device = "cpu"

    processor = TrOCRProcessor.from_pretrained(model_name)
    model = VisionEncoderDecoderModel.from_pretrained(model_name)
    model.to(device)
    model.eval()
    return {"engine": "trocr", "processor": processor, "model": model, "device": device}


def ocr_handwritten_digit(image, ctx: dict, min_conf: float) -> str:
    import numpy as np
    from PIL import Image, ImageOps

    if ctx["engine"] == "easyocr":
        rgb = image.convert("RGB")
        results = ctx["reader"].readtext(
            np.array(rgb),
            detail=1,
            allowlist="0123456789",
        )
        if not results:
            return ""
        text, conf = results[0][1], results[0][2]
        if conf < min_conf:
            return ""
        m = re.search(r"\d", text)
        return m.group(0) if m else ""

    import torch

    if image.mode != "L":
        gray = image.convert("L")
    else:
        gray = image

    if np.mean(np.asarray(gray)) > 245:
        return ""

    gray = ImageOps.autocontrast(gray)
    rgb = gray.convert("RGB")

    w, h = rgb.size
    scale = max(1.0, 64.0 / min(w, h))
    if scale > 1.0:
        rgb = rgb.resize((int(w * scale), int(h * scale)), Image.Resampling.LANCZOS)

    processor = ctx["processor"]
    model = ctx["model"]
    device = ctx["device"]

    pixel_values = processor(images=rgb, return_tensors="pt").pixel_values.to(device)
    with torch.no_grad():
        outputs = model.generate(
            pixel_values,
            max_length=4,
            return_dict_in_generate=True,
            output_scores=True,
        )

    text = processor.batch_decode(outputs.sequences, skip_special_tokens=True)[0]
    if outputs.scores:
        confs = []
        seq = outputs.sequences[0]
        for i, score in enumerate(outputs.scores):
            token_id = int(seq[i + 1]) if i + 1 < len(seq) else int(seq[-1])
            prob = torch.softmax(score, dim=-1)[0, token_id].item()
            confs.append(prob)
        avg_conf = sum(confs) / len(confs) if confs else 0.0
        if avg_conf < min_conf:
            return ""
    m = re.search(r"\d", text)
    return m.group(0) if m else ""


def handwriting_digits_for_row(
    image,
    row_bounds: Tuple[int, int],
    col_bounds: List[Tuple[int, int]],
    ctx: dict,
    pad_frac: float,
    min_ink: float,
    min_conf: float,
) -> List[str]:
    from PIL import Image
    import numpy as np
    import cv2

    y1, y2 = row_bounds
    row_h = max(1, y2 - y1)
    row_pad = int(row_h * pad_frac)

    cols: List[str] = []
    for x1, x2 in col_bounds:
        col_w = max(1, x2 - x1)
        col_pad = int(col_w * pad_frac)
        cx1 = max(0, x1 - col_pad)
        cx2 = min(image.width, x2 + col_pad)
        cy1 = max(0, y1 - row_pad)
        cy2 = min(image.height, y2 + row_pad)
        if cx2 <= cx1 or cy2 <= cy1:
            cols.append("")
            continue
        crop = image.crop((cx1, cy1, cx2, cy2))
        gray = crop.convert("L")
        arr = np.asarray(gray)
        _, bw = cv2.threshold(arr, 0, 255, cv2.THRESH_BINARY_INV + cv2.THRESH_OTSU)
        ink_ratio = float(bw.mean() / 255.0)
        if ink_ratio < min_ink:
            cols.append("")
        else:
                cols.append(ocr_handwritten_digit(crop, ctx, min_conf))
    return cols


def main() -> None:
    parser = argparse.ArgumentParser(description="Extract bijlage tabel uit Vision OCR JSON.")
    parser.add_argument("--vision-json", required=True, help="Vision OCR JSON file")
    parser.add_argument("--grid-json", help="Grid line JSON from detect-table-lines.py")
    parser.add_argument("--image", help="Page image used for grid detection")
    parser.add_argument("--handwriting", action="store_true", help="Use handwriting OCR for digit cells")
    parser.add_argument("--cell-ocr-json", help="Per-cell OCR JSON from ocr-vision.swift")
    parser.add_argument("--cell-map-json", help="Cell mapping JSON from crop-grid-cells.py")
    parser.add_argument(
        "--handwriting-model",
        default="microsoft/trocr-base-handwritten",
        help="Handwriting OCR model name",
    )
    parser.add_argument(
        "--handwriting-engine",
        choices=["trocr", "easyocr"],
        default="trocr",
        help="Handwriting OCR engine",
    )
    parser.add_argument(
        "--handwriting-pad",
        type=float,
        default=0.25,
        help="Padding as fraction of cell size for handwriting crops",
    )
    parser.add_argument(
        "--handwriting-min-ink",
        type=float,
        default=0.015,
        help="Minimum ink ratio to consider a cell non-empty",
    )
    parser.add_argument(
        "--handwriting-min-conf",
        type=float,
        default=0.5,
        help="Minimum average token confidence to accept handwriting OCR",
    )
    parser.add_argument(
        "--handwriting-override",
        action="store_true",
        help="Override Vision digits with handwriting OCR when present",
    )
    parser.add_argument("--out", help="Output JSON file")
    parser.add_argument("--csv", help="Output CSV file")
    args = parser.parse_args()

    vision_path = Path(args.vision_json)
    if not vision_path.exists():
        raise SystemExit(f"Vision JSON not found: {vision_path}")

    with vision_path.open("r", encoding="utf-8") as f:
        data = json.load(f)

    table_pages = []
    results_rows = []
    totals = {}

    grid = None
    if args.grid_json:
        grid_path = Path(args.grid_json)
        if grid_path.exists():
            grid = json.loads(grid_path.read_text(encoding="utf-8"))

    cell_digits = {}
    if args.cell_ocr_json and args.cell_map_json:
        ocr_path = Path(args.cell_ocr_json)
        map_path = Path(args.cell_map_json)
        if ocr_path.exists() and map_path.exists():
            ocr_pages = json.loads(ocr_path.read_text(encoding="utf-8"))
            mapping = json.loads(map_path.read_text(encoding="utf-8"))
            mapping_sorted = sorted(mapping, key=lambda m: m["file"])
            for idx, entry in enumerate(mapping_sorted):
                if idx >= len(ocr_pages):
                    break
                lines = ocr_pages[idx].get("lines", [])
                digits = []
                for line in lines:
                    digits.extend(re.findall(r"\d", line.get("text", "")))
                digit = digits[0] if digits else ""
                if entry["row_type"] == "list":
                    key = ("list", entry["list_number"], entry["col_index"])
                else:
                    key = ("total", entry["label"], entry["col_index"])
                cell_digits[key] = digit

    handwriting = None
    if args.handwriting:
        if not args.image:
            raise SystemExit("--handwriting requires --image")
        handwriting = {
            "image": Path(args.image),
            "engine": args.handwriting_engine,
            "model_name": args.handwriting_model,
            "pad": args.handwriting_pad,
            "min_ink": args.handwriting_min_ink,
            "min_conf": args.handwriting_min_conf,
            "override": args.handwriting_override,
        }

    for page in data:
        lines = page.get("lines", [])
        header = find_line(lines, "Voorlopige telling op lijstniveau")
        if not header:
            continue
        page_num = page.get("page")
        table_pages.append(page_num)

        # use a conservative min_x so we don't miss digits that are slightly left-shifted
        min_x = 0.45

        rows = extract_rows(lines)

        use_grid = False
        grid_horiz = []
        grid_vert = []
        digit_cols: List[Tuple[int, int]] = []
        grid_width = None
        grid_height = None
        if grid and grid.get("grid_lines_px"):
            grid_horiz = grid["grid_lines_px"].get("horizontal", [])
            grid_vert = grid["grid_lines_px"].get("vertical", [])
            grid_width = grid.get("width")
            grid_height = grid.get("height")
            digit_cols = select_digit_columns(grid_vert)
            use_grid = bool(grid_horiz and digit_cols and grid_width and grid_height)

        if use_grid:
            positions_px = digit_positions_px(lines, grid_width, grid_height, min_x_px=0)
            handwriting_ctx = None
            if handwriting:
                from PIL import Image

                image_path = handwriting["image"]
                if not image_path.exists():
                    raise SystemExit(f"Image not found: {image_path}")
                image = Image.open(image_path).convert("RGB")
                if image.width != grid_width or image.height != grid_height:
                    raise SystemExit("Image size does not match grid JSON dimensions")
                ctx = init_handwriting_ocr(handwriting["engine"], handwriting["model_name"])
                handwriting_ctx = {
                    "image": image,
                    "ctx": ctx,
                    "pad": handwriting["pad"],
                    "min_ink": handwriting["min_ink"],
                    "min_conf": handwriting["min_conf"],
                    "override": handwriting["override"],
                }
            for r in sorted(rows, key=lambda v: v["list_number"]):
                rb = r.get("bbox")
                if not rb:
                    digits = [""] * 5
                else:
                    x1, y1, x2, y2 = vision_bbox_to_tl_px(rb, grid_width, grid_height)
                    row_y = (y1 + y2) / 2.0
                    row_bounds = row_bounds_from_lines(grid_horiz, row_y)
                    if not row_bounds:
                        digits = [""] * 5
                    else:
                        digits = digits_for_row_columns(positions_px, row_bounds, digit_cols)
                        if cell_digits:
                            for i in range(5):
                                key = ("list", r["list_number"], i + 1)
                                if key in cell_digits and cell_digits[key]:
                                    digits[i] = cell_digits[key]
                        if handwriting_ctx:
                            h_digits = handwriting_digits_for_row(
                                handwriting_ctx["image"],
                                row_bounds,
                                digit_cols,
                                handwriting_ctx["ctx"],
                                handwriting_ctx["pad"],
                                handwriting_ctx["min_ink"],
                                handwriting_ctx["min_conf"],
                            )
                            for i in range(5):
                                if handwriting_ctx["override"]:
                                    digits[i] = h_digits[i] or digits[i]
                                elif not digits[i] and h_digits[i]:
                                    digits[i] = h_digits[i]
                results_rows.append(
                    {
                        "page": page_num,
                        "list_number": r["list_number"],
                        "list_name": r["list_name"],
                        "columns": digits,
                    }
                )

            def totals_for_label(label_key: str, label_text: str) -> List[str] | None:
                label_line = find_line(lines, label_text)
                if not label_line or not label_line.get("bbox"):
                    return None
                x1, y1, x2, y2 = vision_bbox_to_tl_px(label_line["bbox"], grid_width, grid_height)
                row_bounds = row_bounds_from_lines(grid_horiz, (y1 + y2) / 2.0)
                if not row_bounds:
                    return None
                digits = digits_for_row_columns(positions_px, row_bounds, digit_cols)
                if cell_digits:
                    for i in range(5):
                        key = ("total", label_key, i + 1)
                        if key in cell_digits and cell_digits[key]:
                            digits[i] = cell_digits[key]
                if handwriting_ctx:
                    h_digits = handwriting_digits_for_row(
                        handwriting_ctx["image"],
                        row_bounds,
                        digit_cols,
                        handwriting_ctx["ctx"],
                        handwriting_ctx["pad"],
                        handwriting_ctx["min_ink"],
                        handwriting_ctx["min_conf"],
                    )
                    for i in range(5):
                        if handwriting_ctx["override"]:
                            digits[i] = h_digits[i] or digits[i]
                        elif not digits[i] and h_digits[i]:
                            digits[i] = h_digits[i]
                return digits

            totals["totaal_geldige_stemmen_op_partijen"] = totals_for_label(
                "totaal_geldige_stemmen_op_partijen",
                "TOTAAL GELDIGE STEMMEN OP PARTIJEN",
            )
            totals["blanco"] = totals_for_label("blanco", "BLANCO")
            totals["ongeldig"] = totals_for_label("ongeldig", "ONGELDIG")
            totals["totaal_stembiljetten_in_stembus"] = totals_for_label(
                "totaal_stembiljetten_in_stembus",
                "TOTAAL AANTAL STEMBILJETTEN IN STEMBUS",
            )
        else:
            candidates = extract_numeric_candidates(lines, min_x=min_x)
            positions = digit_positions(candidates)
            centers = compute_column_centers(rows, candidates, min_x)

            for r in sorted(rows, key=lambda v: v["list_number"]):
                rb = r.get("bbox")
                if not rb:
                    digits = [""] * 5
                else:
                    y = line_center_y(rb)
                    tol = max(0.015, rb[3] * 2.0)
                    digits = assign_to_columns(digits_for_y(positions, y, tol), centers)
                results_rows.append(
                    {
                        "page": page_num,
                        "list_number": r["list_number"],
                        "list_name": r["list_name"],
                        "columns": digits,
                    }
                )

            totals["totaal_geldige_stemmen_op_partijen"] = extract_total_digits(
                lines, positions, "TOTAAL GELDIGE STEMMEN OP PARTIJEN", centers
            )
            totals["blanco"] = extract_total_digits(lines, positions, "BLANCO", centers)
            totals["ongeldig"] = extract_total_digits(lines, positions, "ONGELDIG", centers)
            totals["totaal_stembiljetten_in_stembus"] = extract_total_digits(
                lines, positions, "TOTAAL AANTAL STEMBILJETTEN IN STEMBUS", centers
            )

            # If the first row is shifted to the right, shift all rows/totals left.
            row1 = next((r for r in results_rows if r.get("list_number") == 1), None)
            if row1:
                idx = first_non_empty_index(row1.get("columns") or [])
                if idx and idx > 0:
                    for r in results_rows:
                        r["columns"] = shift_columns(r.get("columns") or ["", "", "", "", ""], idx)
                    for key in list(totals.keys()):
                        cols = totals.get(key) or ["", "", "", "", ""]
                        totals[key] = shift_columns(cols, idx)

    output = {
        "table_pages": table_pages,
        "rows": results_rows,
        "totals": totals,
    }

    if args.out:
        Path(args.out).parent.mkdir(parents=True, exist_ok=True)
        with open(args.out, "w", encoding="utf-8") as f:
            json.dump(output, f, ensure_ascii=False, indent=2)
    else:
        print(json.dumps(output, ensure_ascii=False, indent=2))

    if args.csv:
        Path(args.csv).parent.mkdir(parents=True, exist_ok=True)
        with open(args.csv, "w", encoding="utf-8") as f:
            f.write("list_number,list_name,col1,col2,col3,col4,col5\n")
            for row in results_rows:
                list_name = row["list_name"].replace('"', '""')
                cols = row.get("columns") or [""] * 5
                f.write(
                    f"{row['list_number']},\"{list_name}\",{cols[0]},{cols[1]},{cols[2]},{cols[3]},{cols[4]}\n"
                )

            totals_order = [
                "totaal_geldige_stemmen_op_partijen",
                "blanco",
                "ongeldig",
                "totaal_stembiljetten_in_stembus",
            ]
            labels = {
                "totaal_geldige_stemmen_op_partijen": "TOTAAL GELDIGE STEMMEN OP PARTIJEN",
                "blanco": "BLANCO",
                "ongeldig": "ONGELDIG",
                "totaal_stembiljetten_in_stembus": "TOTAAL AANTAL STEMBILJETTEN IN STEMBUS",
            }
            for key in totals_order:
                cols = totals.get(key) or [""] * 5
                f.write(
                    f"\"\",\"{labels[key]}\",{cols[0]},{cols[1]},{cols[2]},{cols[3]},{cols[4]}\n"
                )


if __name__ == "__main__":
    main()
