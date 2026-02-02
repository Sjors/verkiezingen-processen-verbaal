#!/usr/bin/env python3
import argparse
import json
import math
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


def pick_digit(label: str) -> tuple[str, bool]:
    cleaned = label.strip()
    if not cleaned:
        return "", False

    for char in cleaned:
        if char.isdigit():
            return char, False
    return "", False


def beam_candidates(
    probs: torch.Tensor,
    itos: list[str],
    bos_id: int,
    pad_id: int,
    eos_id: int,
    beam_size: int,
    step_topk: int,
    max_steps: int,
) -> list[tuple[str, float, float]]:
    steps = probs.shape[0] if max_steps <= 0 else min(max_steps, probs.shape[0])
    beam: list[tuple[list[int], float, list[float]]] = [([], 0.0, [])]
    finished: list[tuple[list[int], float, list[float]]] = []

    for step in range(steps):
        new_beam: list[tuple[list[int], float, list[float]]] = []
        step_probs = probs[step]
        topk = torch.topk(step_probs, k=min(step_topk, step_probs.numel()))
        for tokens, logp, confs in beam:
            if tokens and tokens[-1] == eos_id:
                finished.append((tokens, logp, confs))
                continue
            for token_id, prob in zip(topk.indices.tolist(), topk.values.tolist()):
                if token_id in (bos_id, pad_id):
                    continue
                new_tokens = tokens + [token_id]
                new_logp = logp + math.log(max(prob, 1e-12))
                new_confs = confs + [prob]
                new_beam.append((new_tokens, new_logp, new_confs))
        if not new_beam:
            break
        new_beam.sort(key=lambda x: x[1], reverse=True)
        beam = new_beam[: max(1, beam_size)]

    candidates = finished + beam

    def decode(tokens: list[int]) -> str:
        out = []
        for tid in tokens:
            if tid == eos_id:
                break
            out.append(itos[tid])
        return "".join(out)

    best: dict[str, tuple[float, float]] = {}
    for tokens, logp, confs in candidates:
        text = decode(tokens)
        if not text:
            continue
        avg_conf = sum(confs) / len(confs) if confs else 0.0
        if text not in best or logp > best[text][1]:
            best[text] = (avg_conf, logp)

    return sorted(
        [(text, vals[0], vals[1]) for text, vals in best.items()],
        key=lambda x: x[2],
        reverse=True,
    )


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
    warnings.filterwarnings("ignore", message=".*tie_word_embeddings.*")
    warnings.filterwarnings("ignore", message=".*MISSING.*")
    parser = argparse.ArgumentParser(description="OCR digit cell crops with PARSeq.")
    parser.add_argument("--image-dir", required=True, help="Directory of cell crops")
    parser.add_argument("--out", required=True, help="Output JSON file")
    parser.add_argument(
        "--model",
        default="parseq",
        help="Torch Hub model entry (parseq, parseq_tiny, parseq_patch16_224)",
    )
    parser.add_argument(
        "--use-fast-processor",
        action="store_true",
        help="Unused (kept for CLI compatibility)",
    )
    parser.add_argument("--device", choices=["cpu", "cuda", "mps"], help="Device override")
    parser.add_argument(
        "--num-beams",
        type=int,
        default=1,
        help="Unused (kept for CLI compatibility)",
    )
    parser.add_argument(
        "--length-penalty",
        type=float,
        default=0.0,
        help="Unused (kept for CLI compatibility)",
    )
    parser.add_argument(
        "--parseq-min-conf",
        type=float,
        default=0.5,
        help="Minimum PARSeq confidence to accept a digit",
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
        default=1,
        help="Unused (kept for CLI compatibility)",
    )
    parser.add_argument(
        "--beam-size",
        type=int,
        default=20,
        help="Beam size for PARSeq decoding",
    )
    parser.add_argument(
        "--beam-topk",
        type=int,
        default=20,
        help="Top-k tokens to expand per step",
    )
    parser.add_argument(
        "--beam-max-steps",
        type=int,
        default=2,
        help="Max decoding steps (0 uses model output length)",
    )
    parser.add_argument(
        "--empty-threshold",
        type=float,
        default=0.02,
        help="Dark pixel ratio below which a cell is treated as empty",
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
    model = torch.hub.load("baudm/parseq", args.model, pretrained=True)
    model.to(device)
    model.eval()

    if _DYNAMO_AVAILABLE:
        try:
            model = dynamo.disable(model)
        except Exception:
            pass

    from strhub.data.module import SceneTextDataModule

    img_transform = SceneTextDataModule.get_transform(model.hparams.img_size)

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
        img = img_transform(image).unsqueeze(0).to(device)
        with torch.no_grad():
            logits = model(img)

        probs = logits.softmax(-1)[0]
        itos = model.tokenizer._itos
        candidates = beam_candidates(
            probs,
            itos,
            model.tokenizer.bos_id,
            model.tokenizer.pad_id,
            model.tokenizer.eos_id,
            args.beam_size,
            args.beam_topk,
            args.beam_max_steps,
        )

        raw_text = candidates[0][0] if candidates else ""
        raw_conf = candidates[0][1] if candidates else 0.0
        digit = ""
        conf = 0.0
        for text, avg_conf, _logp in candidates:
            digit, _ambiguous = pick_digit(text)
            if digit:
                conf = avg_conf
                raw_text = text
                raw_conf = avg_conf
                break
        accepted = bool(digit) and conf >= args.parseq_min_conf

        pages.append(
            {
                "page": idx + 1,
                "lines": [
                    {
                        "text": digit if accepted else "",
                        "confidence": conf if accepted else 0.0,
                        "raw_text": raw_text,
                        "raw_confidence": raw_conf,
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
