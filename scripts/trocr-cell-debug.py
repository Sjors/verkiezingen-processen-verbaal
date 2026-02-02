#!/usr/bin/env python3
import argparse
from pathlib import Path

import warnings

import torch
from PIL import Image
from transformers import TrOCRProcessor, VisionEncoderDecoderModel
from transformers.utils import logging as hf_logging


SETTINGS = [
    (1, -2.0, 1),
    (5, -2.0, 1),
    (20, -2.0, 1),
    (5, 0.0, 1),
    (20, 0.0, 1),
    (96, 0.0, 1),
]


def pick_device(requested: str | None) -> str:
    if requested:
        return requested
    if hasattr(torch.backends, "mps") and torch.backends.mps.is_available():
        return "mps"
    if torch.cuda.is_available():
        return "cuda"
    return "cpu"


def main() -> None:
    hf_logging.set_verbosity_error()
    warnings.filterwarnings("ignore", message="`resume_download` is deprecated")

    parser = argparse.ArgumentParser(description="Print raw TrOCR outputs for a cell.")
    parser.add_argument("--image-dir", required=True, help="Directory of cell crops")
    parser.add_argument("--row", type=int, required=True, help="List row number")
    parser.add_argument("--col", type=int, required=True, help="Column number")
    parser.add_argument(
        "--printed-model",
        default="microsoft/trocr-large-printed",
        help="Printed TrOCR model id",
    )
    parser.add_argument(
        "--hand-model",
        default="microsoft/trocr-large-handwritten",
        help="Handwritten TrOCR model id",
    )
    parser.add_argument(
        "--device",
        choices=["cpu", "cuda", "mps"],
        help="Device override",
    )
    parser.add_argument(
        "--use-fast-processor",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Use fast processor backend",
    )
    args = parser.parse_args()

    image_dir = Path(args.image_dir)
    filename = f"list-{args.row:02d}-col-{args.col}.png"
    image_path = image_dir / filename
    if not image_path.exists():
        raise SystemExit(f"Missing image: {image_path}")

    image = Image.open(image_path).convert("RGB")
    device = pick_device(args.device)
    print(f"device={device}")
    print(f"image={image_path}")

    models = [
        ("printed", args.printed_model),
        ("handwritten", args.hand_model),
    ]

    torch.set_grad_enabled(False)
    for label, model_id in models:
        processor = TrOCRProcessor.from_pretrained(model_id, use_fast=args.use_fast_processor)
        model = VisionEncoderDecoderModel.from_pretrained(model_id, low_cpu_mem_usage=False)
        model.to(device)
        model.eval()
        print(f"\n## {label} ({model_id})")
        pixel_values = processor(images=image, return_tensors="pt").pixel_values.to(device)
        for num_beams, length_penalty, max_new_tokens in SETTINGS:
            generate_kwargs = {
                "pixel_values": pixel_values,
                "max_new_tokens": max_new_tokens,
                "num_beams": num_beams,
                "output_scores": True,
                "return_dict_in_generate": True,
            }
            if num_beams > 1:
                generate_kwargs["length_penalty"] = length_penalty
                generate_kwargs["early_stopping"] = True

            output = model.generate(**generate_kwargs)
            decoded = processor.batch_decode(output.sequences, skip_special_tokens=True)[0]
            conf = 0.0
            if output.scores:
                probs = []
                for step, step_scores in enumerate(output.scores):
                    token_id = output.sequences[0, step + 1].item()
                    prob = torch.softmax(step_scores, dim=-1)[0, token_id].item()
                    probs.append(prob)
                if probs:
                    conf = sum(probs) / len(probs)
            print(
                f"beams={num_beams} len_pen={length_penalty} max_new={max_new_tokens} -> {decoded!r} (conf={conf:.3f})"
            )


if __name__ == "__main__":
    main()
