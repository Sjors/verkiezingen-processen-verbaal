#!/usr/bin/env python3
import argparse
import json
from pathlib import Path
from typing import List

import easyocr


def list_images(image_dir: Path) -> List[Path]:
    exts = {".jpg", ".jpeg", ".png"}
    return sorted([p for p in image_dir.iterdir() if p.suffix.lower() in exts])


def main() -> None:
    parser = argparse.ArgumentParser(description="OCR digit cell crops with EasyOCR.")
    parser.add_argument("--image-dir", required=True, help="Directory of cell crops")
    parser.add_argument("--out", required=True, help="Output JSON file")
    parser.add_argument("--min-conf", type=float, default=0.3, help="Minimum confidence to accept a digit")
    args = parser.parse_args()

    image_dir = Path(args.image_dir)
    images = list_images(image_dir)
    if not images:
        raise SystemExit(f"No images found in {image_dir}")

    reader = easyocr.Reader(["en"], gpu=False, verbose=False)

    pages = []
    for idx, path in enumerate(images):
        results = reader.readtext(
            str(path),
            detail=1,
            allowlist="0123456789",
        )
        text = ""
        conf = 0.0
        if results:
            results.sort(key=lambda r: r[2], reverse=True)
            text, conf = results[0][1], float(results[0][2])
            if conf < args.min_conf:
                text = ""
                conf = 0.0

        pages.append(
            {
                "page": idx + 1,
                "lines": [
                    {
                        "text": text,
                        "confidence": conf,
                        "bbox": [0, 0, 0, 0],
                    }
                ],
            }
        )

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(pages, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
