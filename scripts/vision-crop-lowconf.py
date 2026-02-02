#!/usr/bin/env python3
import argparse
import json
import os
from pathlib import Path
from typing import Dict, List

from PIL import Image


def list_images(image_dir: Path) -> List[Path]:
    exts = {".jpg", ".jpeg", ".png"}
    return sorted(
        [p for p in image_dir.iterdir() if p.suffix.lower() in exts]
    )


def clamp(value: float, min_value: float, max_value: float) -> float:
    return max(min_value, min(max_value, value))


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Crop low-confidence Vision OCR boxes from page images."
    )
    parser.add_argument("--vision-json", required=True, help="Vision OCR JSON file")
    parser.add_argument("--image-dir", required=True, help="Directory with page images")
    parser.add_argument("--out-dir", required=True, help="Output directory for crops")
    parser.add_argument(
        "--max-confidence",
        type=float,
        default=0.7,
        help="Include boxes with confidence <= this threshold (default: 0.7)",
    )
    parser.add_argument(
        "--margin",
        type=float,
        default=0.15,
        help="Margin as fraction of box size (default: 0.15)",
    )
    parser.add_argument(
        "--min-size",
        type=int,
        default=20,
        help="Minimum crop width/height in pixels (default: 20)",
    )

    args = parser.parse_args()
    vision_path = Path(args.vision_json)
    image_dir = Path(args.image_dir)
    out_dir = Path(args.out_dir)

    if not vision_path.exists():
        raise SystemExit(f"Vision JSON not found: {vision_path}")
    if not image_dir.exists():
        raise SystemExit(f"Image directory not found: {image_dir}")

    out_dir.mkdir(parents=True, exist_ok=True)

    with vision_path.open("r", encoding="utf-8") as f:
        vision_data = json.load(f)

    page_map: Dict[int, List[dict]] = {}
    for page in vision_data:
        page_num = page.get("page")
        if isinstance(page_num, int):
            page_map[page_num] = page.get("lines", [])

    images = list_images(image_dir)
    if not images:
        raise SystemExit(f"No images found in {image_dir}")

    manifest = []
    for page_index, image_path in enumerate(images, start=1):
        lines = page_map.get(page_index, [])
        if not lines:
            continue

        image = Image.open(image_path).convert("RGB")
        width, height = image.size

        for line_index, line in enumerate(lines):
            conf = float(line.get("confidence", 1.0))
            if conf > args.max_confidence:
                continue

            bbox = line.get("bbox")
            if not bbox or len(bbox) != 4:
                continue

            x, y, w, h = bbox
            left = x * width
            right = (x + w) * width
            top = (1.0 - y - h) * height
            bottom = (1.0 - y) * height

            mx = w * width * args.margin
            my = h * height * args.margin

            left = clamp(left - mx, 0, width)
            right = clamp(right + mx, 0, width)
            top = clamp(top - my, 0, height)
            bottom = clamp(bottom + my, 0, height)

            if right - left < args.min_size or bottom - top < args.min_size:
                continue

            crop = image.crop((left, top, right, bottom))
            filename = f"page-{page_index:03d}-line-{line_index:04d}-conf-{conf:.2f}.png"
            out_path = out_dir / filename
            crop.save(out_path)

            manifest.append(
                {
                    "page": page_index,
                    "line_index": line_index,
                    "confidence": conf,
                    "bbox": bbox,
                    "image": image_path.name,
                    "crop": out_path.name,
                }
            )

    manifest_path = out_dir / "crops.json"
    with manifest_path.open("w", encoding="utf-8") as f:
        json.dump(manifest, f, ensure_ascii=False, indent=2)

    print(f"Crops written: {len(manifest)}")
    print(f"Manifest: {manifest_path}")


if __name__ == "__main__":
    main()
