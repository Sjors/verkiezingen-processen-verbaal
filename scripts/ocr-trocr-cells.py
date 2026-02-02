#!/usr/bin/env python3
import argparse
import json
import os
import warnings
from pathlib import Path
from typing import List

os.environ.setdefault("TORCHDYNAMO_DISABLE", "1")
os.environ.setdefault("TORCH_DISABLE_DYNAMO", "1")

import numpy as np
import torch

try:
    import torch._dynamo as dynamo

    dynamo.config.disable = True
    dynamo.config.suppress_errors = True
    _DYNAMO_AVAILABLE = True
except Exception:
    dynamo = None
    _DYNAMO_AVAILABLE = False
from PIL import Image
from transformers import TrOCRProcessor, VisionEncoderDecoderModel
from transformers.utils import logging as hf_logging


def list_images(image_dir: Path) -> List[Path]:
    exts = {".jpg", ".jpeg", ".png"}
    return sorted([p for p in image_dir.iterdir() if p.suffix.lower() in exts])


def pick_device(requested: str | None) -> str:
    if requested:
        return requested
    if torch.cuda.is_available():
        return "cuda"
    if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        return "mps"
    return "cpu"


AMBIGUOUS_DIGITS = {
    "I": "1",
    "l": "1",
    "|": "1",
    "!": "1",
    "i": "1",
    "'": "1",
    "T": "7",
    "Q": "9",
    "g": "9",
    "q": "9",
    "Z": "2",
    "z": "2",
    "o": "0",
    "O": "0",
}


def extract_digit_and_confidence(
    decoded: str,
    scores: list[torch.Tensor],
    sequences: torch.Tensor,
    ambiguous_min_conf: float,
) -> tuple[str, float]:
    cleaned = decoded.strip()
    if not cleaned:
        return "", 0.0

    last_char = cleaned[-1]
    digit = last_char if last_char.isdigit() else ""
    ambiguous = False
    if not digit and last_char in AMBIGUOUS_DIGITS:
        digit = AMBIGUOUS_DIGITS[last_char]
        ambiguous = True
    if not digit or not scores:
        return digit, 0.0

    probs = []
    for step, step_scores in enumerate(scores):
        token_id = sequences[0, step + 1].item()
        prob = torch.softmax(step_scores, dim=-1)[0, token_id].item()
        probs.append(prob)

    conf = sum(probs) / len(probs) if probs else 0.0
    if ambiguous and conf < ambiguous_min_conf:
        return "", 0.0
    return digit, conf


def empty_ratio(image: Image.Image, dark_threshold: int, trim: float) -> float:
    gray = np.array(image.convert("L"))
    if gray.size == 0:
        return 0.0
    if trim > 0:
        h, w = gray.shape[:2]
        tx = int(w * trim)
        ty = int(h * trim)
        if w - 2 * tx > 0 and h - 2 * ty > 0:
            gray = gray[ty : h - ty, tx : w - tx]
    dark = gray <= dark_threshold
    return float(dark.sum()) / float(gray.size)


def main() -> None:
    hf_logging.set_verbosity_error()
    warnings.filterwarnings("ignore", message=".*tie_word_embeddings.*")
    warnings.filterwarnings("ignore", message=".*MISSING.*")
    parser = argparse.ArgumentParser(description="OCR digit cell crops with TrOCR.")
    parser.add_argument("--image-dir", required=True, help="Directory of cell crops")
    parser.add_argument("--out", required=True, help="Output JSON file")
    parser.add_argument(
        "--model",
        default="microsoft/trocr-base-handwritten",
        help="Hugging Face model id",
    )
    parser.add_argument(
        "--use-fast-processor",
        action="store_true",
        help="Use the fast image processor backend",
    )
    parser.add_argument("--device", choices=["cpu", "cuda", "mps"], help="Device override")
    parser.add_argument(
        "--num-beams",
        type=int,
        default=5,
        help="Beam count for generation",
    )
    parser.add_argument(
        "--length-penalty",
        type=float,
        default=1.0,
        help="Length penalty for beam search",
    )
    parser.add_argument(
        "--min-conf",
        type=float,
        default=0.1,
        help="Minimum confidence to accept a digit",
    )
    parser.add_argument(
        "--ambiguous-min-conf",
        type=float,
        default=0.4,
        help="Minimum confidence to accept ambiguous glyph mappings",
    )
    parser.add_argument(
        "--max-new-tokens",
        type=int,
        default=2,
        help="Maximum generated tokens",
    )
    parser.add_argument(
        "--empty-threshold",
        type=float,
        default=0.01,
        help="Dark pixel ratio below which a cell is treated as empty (0 disables)",
    )
    parser.add_argument(
        "--empty-dark",
        type=int,
        default=200,
        help="Grayscale threshold for counting dark pixels",
    )
    parser.add_argument(
        "--empty-trim",
        type=float,
        default=0.13,
        help="Trim fraction per side for empty check (use to ignore padded margins)",
    )
    args = parser.parse_args()

    image_dir = Path(args.image_dir)
    images = list_images(image_dir)
    if not images:
        raise SystemExit(f"No images found in {image_dir}")

    device = pick_device(args.device)
    processor = TrOCRProcessor.from_pretrained(args.model, use_fast=args.use_fast_processor)
    model = VisionEncoderDecoderModel.from_pretrained(
        args.model,
        low_cpu_mem_usage=False,
    )
    model.to(device)
    model.eval()

    generate_fn = model.generate
    if _DYNAMO_AVAILABLE:
        try:
            generate_fn = dynamo.disable(model.generate)
        except Exception:
            generate_fn = model.generate

    pages = []
    for idx, path in enumerate(images):
        image = Image.open(path).convert("RGB")
        if args.empty_threshold > 0:
            ratio = empty_ratio(image, args.empty_dark, args.empty_trim)
            if ratio < args.empty_threshold:
                pages.append(
                    {
                        "page": idx + 1,
                        "lines": [
                            {
                                "text": "",
                                "confidence": 0.0,
                                "raw_text": "",
                                "raw_confidence": 0.0,
                                "accepted": False,
                                "skipped": True,
                                "bbox": [0, 0, 0, 0],
                            }
                        ],
                    }
                )
                continue
        pixel_values = processor(images=image, return_tensors="pt").pixel_values.to(device)
        with torch.no_grad():
            output = generate_fn(
                pixel_values,
                max_new_tokens=args.max_new_tokens,
                num_beams=max(1, args.num_beams),
                length_penalty=args.length_penalty,
                early_stopping=True,
                output_scores=True,
                return_dict_in_generate=True,
            )

        decoded = processor.batch_decode(output.sequences, skip_special_tokens=True)[0].strip()
        digit, conf = extract_digit_and_confidence(
            decoded,
            output.scores,
            output.sequences,
            args.ambiguous_min_conf,
        )
        accepted = bool(digit) and conf >= args.min_conf

        pages.append(
            {
                "page": idx + 1,
                "lines": [
                    {
                        "text": digit if accepted else "",
                        "confidence": conf if accepted else 0.0,
                        "raw_text": digit,
                        "raw_confidence": conf if digit else 0.0,
                        "accepted": accepted,
                        "skipped": False,
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
