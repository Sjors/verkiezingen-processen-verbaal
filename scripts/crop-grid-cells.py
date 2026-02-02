#!/usr/bin/env python3
import argparse
import json
import re
from pathlib import Path
from typing import List, Tuple

import cv2
import numpy as np
from PIL import Image


ROW_RE = re.compile(r"^\s*(\d{1,2})\.\s*(.+)$")


def norm_text(text: str) -> str:
    return re.sub(r"\s+", " ", text.strip()).lower()


def find_line(lines: List[dict], needle: str) -> dict | None:
    needle_norm = norm_text(needle)
    for line in lines:
        text = norm_text(line.get("text", ""))
        if needle_norm in text:
            return line
    return None


def extract_rows(lines: List[dict]) -> List[dict]:
    rows = []
    for line in lines:
        text = line.get("text", "")
        m = ROW_RE.match(text)
        if not m:
            continue
        rows.append(
            {
                "list_number": int(m.group(1)),
                "list_name": m.group(2).strip(),
                "bbox": line.get("bbox"),
            }
        )
    return rows


def vision_bbox_to_tl_px(bbox: List[float], width: int, height: int) -> Tuple[float, float, float, float]:
    x, y, w, h = bbox
    x1 = x * width
    y1 = (1.0 - (y + h)) * height
    x2 = (x + w) * width
    y2 = y1 + (h * height)
    return x1, y1, x2, y2


def select_digit_columns(vert_lines: List[int]) -> List[Tuple[int, int]]:
    vert_lines = sorted(vert_lines)
    if len(vert_lines) < 6:
        return []
    rightmost = vert_lines[-6:]
    return [(rightmost[i], rightmost[i + 1]) for i in range(5)]


def row_bounds_from_lines(horiz_lines: List[int], y: float) -> Tuple[int, int] | None:
    if len(horiz_lines) < 2:
        return None
    idx = 0
    for i, line_y in enumerate(horiz_lines):
        if line_y <= y:
            idx = i
        else:
            break
    if idx >= len(horiz_lines) - 1:
        return None
    return horiz_lines[idx], horiz_lines[idx + 1]


def crop_cell(
    image: Image.Image,
    row_bounds: Tuple[int, int],
    col_bounds: Tuple[int, int],
    pad: float,
    row_inner: float,
    col_inner: float,
) -> Image.Image:
    y1, y2 = row_bounds
    x1, x2 = col_bounds
    row_h = max(1, y2 - y1)
    col_w = max(1, x2 - x1)
    inner_row = int(row_h * row_inner)
    inner_col = int(col_w * col_inner)
    y1 = y1 + inner_row
    y2 = y2 - inner_row
    x1 = x1 + inner_col
    x2 = x2 - inner_col
    row_pad = int(row_h * pad)
    col_pad = int(col_w * pad)
    cx1 = max(0, x1 - col_pad)
    cx2 = min(image.width, x2 + col_pad)
    cy1 = max(0, y1 - row_pad)
    cy2 = min(image.height, y2 + row_pad)
    return image.crop((cx1, cy1, cx2, cy2))


def crop_cell_bounds(
    image: Image.Image,
    row_bounds: Tuple[int, int],
    col_bounds: Tuple[int, int],
    pad: float,
    row_inner: float,
    col_inner: float,
) -> Tuple[int, int, int, int]:
    y1, y2 = row_bounds
    x1, x2 = col_bounds
    row_h = max(1, y2 - y1)
    col_w = max(1, x2 - x1)
    inner_row = int(row_h * row_inner)
    inner_col = int(col_w * col_inner)
    y1 = y1 + inner_row
    y2 = y2 - inner_row
    x1 = x1 + inner_col
    x2 = x2 - inner_col
    row_pad = int(row_h * pad)
    col_pad = int(col_w * pad)
    cx1 = max(0, x1 - col_pad)
    cx2 = min(image.width, x2 + col_pad)
    cy1 = max(0, y1 - row_pad)
    cy2 = min(image.height, y2 + row_pad)
    return cx1, cy1, cx2, cy2


def color_dir_from_out(out_dir: Path) -> Path:
    name = out_dir.name
    if "-prep" in name:
        prefix = name.split("-prep")[0]
        return out_dir.with_name(f"{prefix}-color")
    return out_dir.with_name(f"{name}-color")


def detect_fullwidth_bars(
    region: np.ndarray,
    min_height_px: int,
    coverage_thresh: float = 0.85,
    min_len_px: int = 6,
) -> List[Tuple[int, int]]:
    med = float(np.median(region))
    mad = float(np.median(np.abs(region - med)))
    robust_std = 1.4826 * mad
    thresh = med - max(10.0, 1.1 * robust_std)
    dark = (region < thresh).astype(np.uint8) * 255
    kernel = np.ones((1, 31), np.uint8)
    dark = cv2.morphologyEx(dark, cv2.MORPH_CLOSE, kernel)
    row_cov = dark.mean(axis=1) / 255.0
    mask = row_cov >= coverage_thresh

    mask_img = (mask.astype(np.uint8) * 255).reshape(-1, 1)
    k = max(3, int(round(min_height_px * 1.5)))
    kernel_v = np.ones((k, 1), np.uint8)
    mask_img = cv2.morphologyEx(mask_img, cv2.MORPH_CLOSE, kernel_v)
    mask = mask_img[:, 0] > 0

    segments: List[Tuple[int, int]] = []
    start = None
    for i, flag in enumerate(mask):
        if flag and start is None:
            start = i
        elif not flag and start is not None:
            end = i
            if end - start >= max(min_len_px, min_height_px):
                segments.append((start, end))
            start = None
    if start is not None:
        end = len(mask)
        if end - start >= max(min_len_px, min_height_px):
            segments.append((start, end))
    return segments


def find_bar_ranges(
    image: Image.Image,
    grid_bbox: List[int] | None,
) -> List[Tuple[int, int]]:
    rgb = np.array(image.convert("RGB"))
    gray = cv2.cvtColor(rgb, cv2.COLOR_RGB2GRAY)

    bg = cv2.medianBlur(gray, 31)
    corr = cv2.addWeighted(gray, 1.8, bg, -0.8, 128)
    corr = np.clip(corr, 0, 255).astype("uint8")
    corr = cv2.normalize(corr, None, 0, 255, cv2.NORM_MINMAX)

    if grid_bbox:
        bx1, by1, bx2, by2 = [int(v) for v in grid_bbox]
    else:
        bx1, by1, bx2, by2 = 0, 0, 0, 0

    x1 = bx1 if bx2 > bx1 else 0
    x2 = bx2 if bx2 > bx1 else gray.shape[1] - 1
    y1 = by1 if by2 > by1 else 0
    y2 = by2 if by2 > by1 else gray.shape[0] - 1

    mm_to_px = gray.shape[0] / 297.0
    min_height_px = max(3, int(round(mm_to_px * 1.0)))
    region = corr[y1:y2, x1:x2]
    fullwidth = detect_fullwidth_bars(region, min_height_px)
    sep_ranges = [(y1 + s, y1 + e) for s, e in fullwidth]
    sep_ranges = sorted(sep_ranges)
    merged: List[Tuple[int, int]] = []
    merge_gap = max(2, int(round(min_height_px * 0.6)))
    for s, e in sep_ranges:
        if not merged:
            merged.append((s, e))
            continue
        ps, pe = merged[-1]
        if s <= pe + merge_gap:
            merged[-1] = (ps, max(pe, e))
        else:
            merged.append((s, e))
    sep_ranges = merged

    cleaned: List[Tuple[int, int]] = []
    for sy1, sy2 in sep_ranges:
        y0 = max(y1, sy1)
        y3 = min(y2, sy2)
        if y3 <= y0:
            continue
        cleaned.append((y0, y3))
    return cleaned


def apply_bar_whitening(
    crop: Image.Image,
    crop_y1: int,
    crop_y2: int,
    bar_ranges: List[Tuple[int, int]],
) -> Image.Image:
    if not bar_ranges:
        return crop
    arr = np.array(crop)
    height = arr.shape[0]
    for sy1, sy2 in bar_ranges:
        if sy2 <= crop_y1 or sy1 >= crop_y2:
            continue
        y0 = max(0, sy1 - crop_y1)
        y3 = min(height, sy2 - crop_y1)
        if y3 <= y0:
            continue
        arr[y0:y3, ...] = 255
    return Image.fromarray(arr, mode=crop.mode)


def hue_distance(h: np.ndarray, center: int) -> np.ndarray:
    diff = np.abs(h.astype(np.int16) - center)
    return np.minimum(diff, 180 - diff)


def detect_pen_hue(image: Image.Image, sat_min: int, val_max: int) -> int | None:
    rgb = np.array(image.convert("RGB"))
    hsv = cv2.cvtColor(rgb, cv2.COLOR_RGB2HSV)
    h = hsv[:, :, 0]
    s = hsv[:, :, 1]
    v = hsv[:, :, 2]
    mask = (s >= sat_min) & (v <= val_max)
    if mask.sum() < 200:
        return None
    hist = np.bincount(h[mask].ravel(), minlength=180)
    if hist.max() < 50:
        return None
    return int(hist.argmax())


def remove_grid_lines(gray: np.ndarray) -> np.ndarray:
    bin_img = cv2.adaptiveThreshold(
        gray,
        255,
        cv2.ADAPTIVE_THRESH_MEAN_C,
        cv2.THRESH_BINARY_INV,
        31,
        10,
    )
    height, width = bin_img.shape[:2]
    horiz_kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (max(10, width // 12), 1))
    vert_kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (1, max(10, height // 6)))
    horiz = cv2.erode(bin_img, horiz_kernel)
    horiz = cv2.dilate(horiz, horiz_kernel)
    vert = cv2.erode(bin_img, vert_kernel)
    vert = cv2.dilate(vert, vert_kernel)
    return cv2.bitwise_or(horiz, vert)


def ink_mask(
    crop: Image.Image,
    mode: str,
    hue_center: int | None,
    hue_tol: int,
    sat_min: int,
    val_max: int,
    dark_max: int,
    remove_grid: bool,
) -> np.ndarray:
    rgb = np.array(crop.convert("RGB"))
    hsv = cv2.cvtColor(rgb, cv2.COLOR_RGB2HSV)
    h = hsv[:, :, 0]
    s = hsv[:, :, 1]
    v = hsv[:, :, 2]

    mask = np.zeros(h.shape, dtype=bool)
    local_center = hue_center

    if mode in {"blue", "auto"}:
        if mode == "blue":
            local_center = 110
        if local_center is not None:
            mask_color = (s >= sat_min) & (v <= val_max) & (hue_distance(h, local_center) <= hue_tol)
            mask |= mask_color

    if mode == "black" or (mode == "auto" and local_center is None):
        mask |= v <= dark_max

    if remove_grid:
        gray = cv2.cvtColor(rgb, cv2.COLOR_RGB2GRAY)
        line_mask = remove_grid_lines(gray)
        mask &= line_mask == 0

    return mask


def ink_ratio(
    crop: Image.Image,
    mode: str,
    hue_center: int | None,
    hue_tol: int,
    sat_min: int,
    val_max: int,
    dark_max: int,
    remove_grid: bool,
) -> float:
    mask = ink_mask(crop, mode, hue_center, hue_tol, sat_min, val_max, dark_max, remove_grid)
    return float(mask.sum()) / float(mask.size) if mask.size else 0.0


def preprocess_crop(
    crop: Image.Image,
    mode: str,
    hue_center: int | None,
    hue_tol: int,
    sat_min: int,
    val_max: int,
    dark_max: int,
    remove_grid: bool,
    style: str,
    ink_darken: int,
    grid_fade: int,
    grid_erase: bool,
) -> Image.Image:
    rgb = np.array(crop.convert("RGB"))
    hsv = cv2.cvtColor(rgb, cv2.COLOR_RGB2HSV)
    h = hsv[:, :, 0]
    s = hsv[:, :, 1]
    v = hsv[:, :, 2]

    mask = np.zeros(h.shape, dtype=bool)

    if mode in {"blue", "auto"}:
        if mode == "blue":
            hue_center = 110
        if hue_center is not None:
            mask_color = (s >= sat_min) & (v <= val_max) & (hue_distance(h, hue_center) <= hue_tol)
            mask |= mask_color

    if mode == "black" or (mode == "auto" and hue_center is None):
        mask |= v <= dark_max

    gray = cv2.cvtColor(rgb, cv2.COLOR_RGB2GRAY)
    if remove_grid:
        line_mask = remove_grid_lines(gray)
        if style == "mask":
            mask &= line_mask == 0
        else:
            gray = gray.copy()
            if grid_erase:
                gray[line_mask > 0] = 255
            else:
                gray[line_mask > 0] = np.minimum(gray[line_mask > 0] + grid_fade, 255)

    if style == "mask":
        out = np.full_like(gray, 255)
        out[mask] = 0
        return Image.fromarray(out, mode="L")

    out = gray.copy()
    out[mask] = np.minimum(out[mask], ink_darken)
    return Image.fromarray(out, mode="L")


def main() -> None:
    parser = argparse.ArgumentParser(description="Crop grid cells for OCR.")
    parser.add_argument("--vision-json", required=True, help="Vision OCR JSON file")
    parser.add_argument("--grid-json", required=True, help="Grid line JSON file")
    parser.add_argument("--image", required=True, help="Page image used for grid detection")
    parser.add_argument("--out-dir", required=True, help="Output directory for cell crops")
    parser.add_argument("--map-json", required=True, help="Output mapping JSON")
    parser.add_argument("--pad", type=float, default=0.25, help="Padding fraction for crops")
    parser.add_argument("--row-inner", type=float, default=0.04, help="Trim fraction from row bounds")
    parser.add_argument("--col-inner", type=float, default=0.05, help="Trim fraction from column bounds")
    parser.add_argument("--hue-tol", type=int, default=18, help="Hue tolerance for color masking")
    parser.add_argument("--sat-min", type=int, default=40, help="Minimum saturation for color masking")
    parser.add_argument("--val-max", type=int, default=230, help="Maximum value for color masking")
    parser.add_argument("--dark-max", type=int, default=120, help="Maximum value for dark ink masking")
    parser.add_argument(
        "--ink-darken",
        type=int,
        default=40,
        help="Target darkness for ink pixels in enhance mode",
    )
    parser.add_argument(
        "--grid-fade",
        type=int,
        default=40,
        help="How much to lighten grid lines in enhance mode",
    )
    parser.add_argument(
        "--page",
        type=int,
        default=10,
        help="Vision JSON page number for the bijlage table (default: 10)",
    )
    args = parser.parse_args()

    vision = json.loads(Path(args.vision_json).read_text(encoding="utf-8"))
    grid = json.loads(Path(args.grid_json).read_text(encoding="utf-8"))

    grid_horiz = grid.get("grid_lines_px", {}).get("horizontal", [])
    grid_vert = grid.get("grid_lines_px", {}).get("vertical", [])
    digit_cols = select_digit_columns(grid_vert)
    if not grid_horiz or not digit_cols:
        raise SystemExit("Grid lines not found in grid JSON")

    image = Image.open(args.image).convert("RGB")
    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    color_dir = color_dir_from_out(out_dir)
    color_dir.mkdir(parents=True, exist_ok=True)

    bar_ranges = find_bar_ranges(image, grid.get("table_bbox_px"))
    hue_center = detect_pen_hue(image, args.sat_min, args.val_max)

    page = next((p for p in vision if p.get("page") == args.page), None)
    if not page:
        raise SystemExit("Table page not found in vision JSON")
    lines = page.get("lines", [])

    rows = extract_rows(lines)

    mapping = []

    for row in sorted(rows, key=lambda r: r["list_number"]):
        rb = row.get("bbox")
        if not rb:
            continue
        x1, y1, x2, y2 = vision_bbox_to_tl_px(rb, image.width, image.height)
        row_bounds = row_bounds_from_lines(grid_horiz, (y1 + y2) / 2.0)
        if not row_bounds:
            continue
        for col_idx, col_bounds in enumerate(digit_cols, start=1):
            cx1, cy1, cx2, cy2 = crop_cell_bounds(
                image,
                row_bounds,
                col_bounds,
                args.pad,
                args.row_inner,
                args.col_inner,
            )
            crop_color = image.crop((cx1, cy1, cx2, cy2))
            filename = f"list-{row['list_number']:02d}-col-{col_idx}.png"
            if color_dir:
                crop_color.save(color_dir / filename)
            crop = apply_bar_whitening(crop_color, cy1, cy2, bar_ranges)
            crop = preprocess_crop(
                crop,
                "auto",
                hue_center,
                args.hue_tol,
                args.sat_min,
                args.val_max,
                args.dark_max,
                True,
                "enhance",
                args.ink_darken,
                args.grid_fade,
                True,
            )
            crop.save(out_dir / filename)
            mapping.append(
                {
                    "file": filename,
                    "row_type": "list",
                    "list_number": row["list_number"],
                    "col_index": col_idx,
                }
            )

    totals = [
        ("totaal_geldige_stemmen_op_partijen", "TOTAAL GELDIGE STEMMEN OP PARTIJEN"),
        ("blanco", "BLANCO"),
        ("ongeldig", "ONGELDIG"),
        ("totaal_stembiljetten_in_stembus", "TOTAAL AANTAL STEMBILJETTEN IN STEMBUS"),
    ]
    for key, label in totals:
        line = find_line(lines, label)
        if not line or not line.get("bbox"):
            continue
        x1, y1, x2, y2 = vision_bbox_to_tl_px(line["bbox"], image.width, image.height)
        row_bounds = row_bounds_from_lines(grid_horiz, (y1 + y2) / 2.0)
        if not row_bounds:
            continue
        for col_idx, col_bounds in enumerate(digit_cols, start=1):
            cx1, cy1, cx2, cy2 = crop_cell_bounds(
                image,
                row_bounds,
                col_bounds,
                args.pad,
                args.row_inner,
                args.col_inner,
            )
            crop_color = image.crop((cx1, cy1, cx2, cy2))
            filename = f"total-{key}-col-{col_idx}.png"
            if color_dir:
                crop_color.save(color_dir / filename)
            crop = apply_bar_whitening(crop_color, cy1, cy2, bar_ranges)
            crop = preprocess_crop(
                crop,
                "auto",
                hue_center,
                args.hue_tol,
                args.sat_min,
                args.val_max,
                args.dark_max,
                True,
                "enhance",
                args.ink_darken,
                args.grid_fade,
                True,
            )
            crop.save(out_dir / filename)
            mapping.append(
                {
                    "file": filename,
                    "row_type": "total",
                    "label": key,
                    "col_index": col_idx,
                }
            )

    map_path = Path(args.map_json)
    map_path.parent.mkdir(parents=True, exist_ok=True)
    map_path.write_text(json.dumps(mapping, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
