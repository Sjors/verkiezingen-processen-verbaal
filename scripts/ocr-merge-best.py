#!/usr/bin/env python3
import argparse
import json
from pathlib import Path
from typing import Any, Dict, List, Tuple


def load_json(path: Path) -> List[Dict[str, Any]]:
    return json.loads(path.read_text(encoding="utf-8"))


def pick_best_line(candidates: List[Tuple[str, Dict[str, Any]]]) -> Dict[str, Any]:
    best_line: Dict[str, Any] | None = None
    best_score = -1.0
    best_source = ""

    for source, line in candidates:
        if not line:
            continue
        text = line.get("text", "") or ""
        raw_text = line.get("raw_text", "") or ""
        score = float(line.get("confidence", 0.0) or 0.0)
        if not text and raw_text:
            score = float(line.get("raw_confidence", score) or score)

        if score > best_score:
            best_score = score
            best_line = line
            best_source = source

    if best_line is None:
        return {"text": "", "confidence": 0.0, "bbox": [0, 0, 0, 0]}

    out = dict(best_line)
    text = out.get("text", "") or ""
    raw_text = out.get("raw_text", "") or ""
    if not text and raw_text:
        out["text"] = raw_text
        out["confidence"] = float(out.get("raw_confidence", out.get("confidence", 0.0)) or 0.0)
    out["source"] = best_source
    return out


def main() -> None:
    parser = argparse.ArgumentParser(description="Merge OCR JSON by highest confidence per cell.")
    parser.add_argument("--inputs", nargs="+", required=True, help="OCR JSON files to merge")
    parser.add_argument("--out", required=True, help="Output merged JSON")
    args = parser.parse_args()

    input_paths = [Path(p) for p in args.inputs]
    datasets = [load_json(p) for p in input_paths]
    max_len = max((len(d) for d in datasets), default=0)

    merged = []
    for idx in range(max_len):
        candidates = []
        for path, data in zip(input_paths, datasets):
            if idx >= len(data):
                continue
            lines = data[idx].get("lines", [])
            line = lines[0] if lines else {}
            candidates.append((path.stem, line))
        merged.append({"page": idx + 1, "lines": [pick_best_line(candidates)]})

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(merged, ensure_ascii=False, indent=2), encoding="utf-8")


if __name__ == "__main__":
    main()
