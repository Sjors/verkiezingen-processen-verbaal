#!/usr/bin/env python3
import argparse
import json
import re
from pathlib import Path
from typing import List, Tuple

import cv2


def _norm_text(text: str) -> str:
    return re.sub(r"\s+", " ", text.strip()).lower()


def _find_line(lines: List[dict], needle: str) -> dict | None:
    needle_norm = _norm_text(needle)
    for line in lines:
        text = _norm_text(line.get("text", ""))
        if needle_norm in text:
            return line
    return None


def _crop_from_vision(vision_json: Path, page_num: int) -> Tuple[float, float, float, float] | None:
    data = json.loads(vision_json.read_text(encoding="utf-8"))
    page = next((p for p in data if p.get("page") == page_num), None)
    if not page:
        return None
    lines = page.get("lines", [])
    header = _find_line(lines, "Bijlage Voorlopige telling op lijstniveau")
    total = _find_line(lines, "TOTAAL AANTAL STEMBILJETTEN IN STEMBUS")
    if not header or not total:
        return None

    def to_top_left(bbox: List[float]) -> Tuple[float, float, float, float]:
        x, y, w, h = bbox
        x1 = x
        y1 = 1.0 - (y + h)
        x2 = x + w
        y2 = y1 + h
        return x1, y1, x2, y2

    header_tl = to_top_left(header["bbox"])
    total_tl = to_top_left(total["bbox"])

    top = max(0.0, header_tl[1] - 0.02)
    bottom = min(1.0, total_tl[3] + 0.03)
    left = 0.05
    right = 0.95
    return left, top, right, bottom


def _cluster_positions(values: List[int], tol: int = 4) -> List[int]:
    if not values:
        return []
    values = sorted(values)
    clusters: List[List[int]] = [[values[0]]]
    for v in values[1:]:
        if abs(v - clusters[-1][-1]) <= tol:
            clusters[-1].append(v)
        else:
            clusters.append([v])
    return [int(sum(c) / len(c)) for c in clusters]


def _extract_line_masks(gray: cv2.Mat) -> Tuple[cv2.Mat, cv2.Mat, cv2.Mat]:
    height, width = gray.shape[:2]

    blur = cv2.GaussianBlur(gray, (3, 3), 0)
    bin_img = cv2.adaptiveThreshold(
        blur,
        255,
        cv2.ADAPTIVE_THRESH_MEAN_C,
        cv2.THRESH_BINARY_INV,
        35,
        15,
    )

    horiz_size = max(20, width // 30)
    vert_size = max(20, height // 30)
    horiz_kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (horiz_size, 1))
    vert_kernel = cv2.getStructuringElement(cv2.MORPH_RECT, (1, vert_size))

    horiz = cv2.erode(bin_img, horiz_kernel)
    horiz = cv2.dilate(horiz, horiz_kernel)
    vert = cv2.erode(bin_img, vert_kernel)
    vert = cv2.dilate(vert, vert_kernel)

    return bin_img, horiz, vert


def _detect_grid_lines(gray: cv2.Mat) -> Tuple[List[int], List[int]]:
    height, width = gray.shape[:2]
    _, horiz, vert = _extract_line_masks(gray)

    horiz_contours, _ = cv2.findContours(horiz, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
    vert_contours, _ = cv2.findContours(vert, cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)

    horiz_positions: List[int] = []
    vert_positions: List[int] = []

    max_h = max(8, int(height * 0.02))
    max_w = max(8, int(width * 0.02))

    for cnt in horiz_contours:
        x, y, w, h = cv2.boundingRect(cnt)
        if w >= int(width * 0.6) and h <= max_h:
            horiz_positions.append(y + h // 2)

    for cnt in vert_contours:
        x, y, w, h = cv2.boundingRect(cnt)
        if h >= int(height * 0.6) and w <= max_w:
            vert_positions.append(x + w // 2)

    horiz_positions = _cluster_positions(horiz_positions, tol=4)
    vert_positions = _cluster_positions(vert_positions, tol=4)

    return horiz_positions, vert_positions


def _detect_table_bbox(gray: cv2.Mat) -> Tuple[int, int, int, int] | None:
    height, width = gray.shape[:2]

    _, horiz, vert = _extract_line_masks(gray)
    grid = cv2.bitwise_or(horiz, vert)
    nz = cv2.findNonZero(grid)
    if nz is None or len(nz) < 500:
        return None

    x, y, w, h = cv2.boundingRect(nz)
    if w < width * 0.2 or h < height * 0.2:
        return None
    return x, y, x + w, y + h


def main() -> None:
    parser = argparse.ArgumentParser(description="Detect table region via line detection.")
    parser.add_argument("--image", required=True, help="Path to the image of the bijlage page")
    parser.add_argument("--out-json", required=True, help="Output JSON file")
    parser.add_argument("--out-image", required=True, help="Output image with overlay")
    parser.add_argument("--vision-json", help="Vision OCR JSON to auto-crop the table region")
    parser.add_argument("--page", type=int, help="Page number in Vision JSON (1-based)")
    parser.add_argument(
        "--crop-norm",
        nargs=4,
        type=float,
        metavar=("X1", "Y1", "X2", "Y2"),
        help="Crop region in normalized coordinates",
    )
    args = parser.parse_args()

    image_path = Path(args.image)
    if not image_path.exists():
        raise SystemExit(f"Image not found: {image_path}")

    crop_norm = None
    if args.crop_norm:
        crop_norm = tuple(args.crop_norm)
    elif args.vision_json and args.page:
        crop_norm = _crop_from_vision(Path(args.vision_json), args.page)

    img = cv2.imread(str(image_path))
    if img is None:
        raise SystemExit(f"Failed to read image: {image_path}")
    height, width = img.shape[:2]

    offset_x = 0
    offset_y = 0
    crop = img
    if crop_norm:
        x1 = max(0, int(crop_norm[0] * width))
        y1 = max(0, int(crop_norm[1] * height))
        x2 = min(width, int(crop_norm[2] * width))
        y2 = min(height, int(crop_norm[3] * height))
        if x2 <= x1 or y2 <= y1:
            raise SystemExit("Invalid crop bounds computed from Vision JSON")
        crop = img[y1:y2, x1:x2]
        offset_x = x1
        offset_y = y1

    gray = cv2.cvtColor(crop, cv2.COLOR_BGR2GRAY)
    table_bbox = _detect_table_bbox(gray)

    output = {
        "image": str(image_path),
        "width": width,
        "height": height,
        "crop_norm": crop_norm,
        "table_bbox_px": None,
        "table_bbox_norm": None,
    }

    overlay = img.copy()
    if table_bbox:
        x1, y1, x2, y2 = table_bbox
        x1 += offset_x
        x2 += offset_x
        y1 += offset_y
        y2 += offset_y

        output["table_bbox_px"] = [x1, y1, x2, y2]
        output["table_bbox_norm"] = [x1 / width, y1 / height, x2 / width, y2 / height]

        # Draw rectangle in yellow
        cv2.rectangle(overlay, (x1, y1), (x2, y2), (0, 255, 255), 3)

        # Detect grid lines inside the table
        table_crop = cv2.cvtColor(img[y1:y2, x1:x2], cv2.COLOR_BGR2GRAY)
        horiz_positions, vert_positions = _detect_grid_lines(table_crop)

        output["grid_lines_px"] = {
            "horizontal": [y1 + y for y in horiz_positions],
            "vertical": [x1 + x for x in vert_positions],
        }
        output["grid_lines_norm"] = {
            "horizontal": [(y1 + y) / height for y in horiz_positions],
            "vertical": [(x1 + x) / width for x in vert_positions],
        }

        for y in horiz_positions:
            cv2.line(overlay, (x1, y1 + y), (x2, y1 + y), (0, 255, 255), 2)
        for x in vert_positions:
            cv2.line(overlay, (x1 + x, y1), (x1 + x, y2), (0, 255, 255), 2)
    else:
        output["grid_lines_px"] = None
        output["grid_lines_norm"] = None

    out_json = Path(args.out_json)
    out_json.parent.mkdir(parents=True, exist_ok=True)
    out_json.write_text(json.dumps(output, ensure_ascii=False, indent=2), encoding="utf-8")

    out_image = Path(args.out_image)
    out_image.parent.mkdir(parents=True, exist_ok=True)
    cv2.imwrite(str(out_image), overlay)


if __name__ == "__main__":
    main()
