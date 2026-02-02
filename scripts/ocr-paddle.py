#!/usr/bin/env python3
import argparse
import json
import os
import re
from typing import List

from pdf2image import convert_from_path, pdfinfo_from_path
from paddleocr import PaddleOCR
import numpy as np


def to_jsonable(value):
    if isinstance(value, np.ndarray):
        return value.tolist()
    if isinstance(value, list):
        return [to_jsonable(item) for item in value]
    if isinstance(value, tuple):
        return [to_jsonable(item) for item in value]
    return value


def parse_pages(pages: str, total_pages: int) -> List[int]:
    if not pages:
        return list(range(1, total_pages + 1))

    result = set()
    for part in pages.split(","):
        part = part.strip()
        if not part:
            continue
        if "-" in part:
            start_str, end_str = part.split("-", 1)
            start = int(start_str)
            end = int(end_str)
            if start > end:
                start, end = end, start
            for page in range(start, end + 1):
                if 1 <= page <= total_pages:
                    result.add(page)
        else:
            page = int(part)
            if 1 <= page <= total_pages:
                result.add(page)

    return sorted(result)


def ocr_pdf(pdf_path: str, dpi: int, pages: List[int], device: str | None):
    ocr_kwargs = {"use_textline_orientation": True, "lang": "en"}
    if device:
        ocr_kwargs["device"] = device
    ocr = PaddleOCR(**ocr_kwargs)

    for page in pages:
        images = convert_from_path(pdf_path, dpi=dpi, first_page=page, last_page=page)
        if not images:
            continue
        image = np.array(images[0])
        result = ocr.predict(image)
        yield page, result


def main() -> None:
    parser = argparse.ArgumentParser(description="Run PaddleOCR on a PDF.")
    parser.add_argument("pdf", help="Path to PDF file")
    parser.add_argument("--dpi", type=int, default=300, help="Render DPI (default: 300)")
    parser.add_argument("--pages", help="Pages to OCR (e.g. '1,3-5'). Default: all pages")
    parser.add_argument("--out", help="Write JSON output to this file")
    parser.add_argument(
        "--device",
        help="Device to use: cpu, gpu, gpu:0. Defaults to PaddleOCR automatic selection.",
    )

    args = parser.parse_args()
    pdf_path = args.pdf

    if not os.path.exists(pdf_path):
        raise SystemExit(f"PDF not found: {pdf_path}")

    info = pdfinfo_from_path(pdf_path)
    total_pages = int(info.get("Pages", 0))
    if total_pages <= 0:
        raise SystemExit("Could not determine page count")

    pages = parse_pages(args.pages, total_pages)
    if not pages:
        raise SystemExit("No pages selected")

    device = args.device
    if device:
        try:
            import paddle

            if device.lower().startswith("gpu") and not paddle.device.is_compiled_with_cuda():
                print("GPU requested but Paddle is not compiled with CUDA; using CPU instead.")
                device = "cpu"
        except Exception:
            pass

    output = []
    for page, result in ocr_pdf(pdf_path, args.dpi, pages, device):
        lines = []
        if result:
            item = result[0]
            texts = item.get("rec_texts") or []
            scores = item.get("rec_scores") or []
            boxes = item.get("rec_polys") or item.get("dt_polys") or []
            for idx, text in enumerate(texts):
                conf = float(scores[idx]) if idx < len(scores) else None
                bbox = boxes[idx] if idx < len(boxes) else None
                lines.append({"text": text, "confidence": conf, "bbox": to_jsonable(bbox)})
        output.append({"page": page, "lines": lines})

    if args.out:
        out_dir = os.path.dirname(args.out)
        if out_dir:
            os.makedirs(out_dir, exist_ok=True)
        with open(args.out, "w", encoding="utf-8") as f:
            json.dump(output, f, ensure_ascii=False, indent=2)
    else:
        print(json.dumps(output, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
