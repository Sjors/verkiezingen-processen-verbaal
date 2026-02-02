#!/usr/bin/env python3
import argparse
import json
import subprocess
import sys
from pathlib import Path

from PIL import Image


def list_images(image_dir: Path) -> list[Path]:
    exts = {".jpg", ".jpeg", ".png"}
    return sorted([p for p in image_dir.iterdir() if p.suffix.lower() in exts])


def main() -> None:
    parser = argparse.ArgumentParser(description="Render an OCR gallery as Markdown.")
    parser.add_argument("--image-dir", required=True, help="Directory of images")
    parser.add_argument("--ocr-json", help="OCR JSON (Vision-like) output")
    parser.add_argument(
        "--ocr-json-print",
        help="OCR JSON for the printed model (left side in combined display)",
    )
    parser.add_argument(
        "--ocr-json-hand",
        help="OCR JSON for the handwritten model (right side in combined display)",
    )
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
        "--ms-num-beams",
        type=int,
        default=20,
        help="Beam search width when running Microsoft OCR",
    )
    parser.add_argument(
        "--length-penalty",
        type=float,
        default=0.0,
        help="Length penalty when running OCR",
    )
    parser.add_argument(
        "--ms-min-conf",
        type=float,
        default=0.1,
        help="Minimum confidence to accept a digit when running Microsoft OCR",
    )
    parser.add_argument(
        "--ambiguous-min-conf",
        type=float,
        default=0.4,
        help="Minimum confidence for ambiguous glyph mappings when running OCR",
    )
    parser.add_argument(
        "--empty-threshold",
        type=float,
        default=0.01,
        help="Empty threshold for OCR skip (pre-OCR)",
    )
    parser.add_argument(
        "--empty-trim",
        type=float,
        default=0.13,
        help="Trim fraction per side for empty check (pre-OCR)",
    )
    parser.add_argument(
        "--device",
        choices=["cpu", "cuda", "mps"],
        help="Device override when running OCR",
    )
    parser.add_argument(
        "--use-fast-processor",
        action=argparse.BooleanOptionalAction,
        default=True,
        help="Use fast processor backend when running OCR",
    )
    parser.add_argument(
        "--hand-empty-conf",
        type=float,
        default=0.4,
        help="If print is empty and hand confidence is below this, show '-'",
    )
    parser.add_argument("--out", default="gallery.md", help="Output Markdown file")
    parser.add_argument("--cols", type=int, default=5, help="Number of columns in the table")
    parser.add_argument("--parseq-json", help="PARSeq OCR JSON output")
    parser.add_argument(
        "--parseq-model",
        default="parseq",
        help="PARSeq torch.hub entry (parseq, parseq_tiny, parseq_patch16_224)",
    )
    parser.add_argument(
        "--parseq-beam-size",
        type=int,
        default=20,
        help="Beam size for PARSeq decoding",
    )
    parser.add_argument(
        "--parseq-threshold",
        type=float,
        default=0.7,
        help="Use PARSeq when confidence is at or below this threshold",
    )
    parser.add_argument(
        "--parseq-empty-conf",
        type=float,
        default=0.9,
        help="Minimum PARSeq confidence to accept an explicit no-digit result",
    )
    parser.add_argument(
        "--parseq-min-conf",
        type=float,
        default=0.4,
        help="Minimum PARSeq confidence to accept a digit",
    )
    parser.add_argument(
        "--parseq-bias",
        type=float,
        default=0.2,
        help="Confidence bias added when preferring PARSeq over base OCR",
    )
    args = parser.parse_args()

    image_dir = Path(args.image_dir)
    images = list_images(image_dir)
    if not images:
        raise SystemExit(f"No images found in {image_dir}")

    if not args.ocr_json:
        default_print = image_dir.with_name(f"{image_dir.name}-trocr-printed.json")
        default_hand = image_dir.with_name(f"{image_dir.name}-trocr-handwritten.json")
        ocr_print_path = Path(args.ocr_json_print) if args.ocr_json_print else default_print
        ocr_hand_path = Path(args.ocr_json_hand) if args.ocr_json_hand else default_hand
    else:
        ocr_print_path = None
        ocr_hand_path = None

    if not args.ocr_json:
        script_path = Path(__file__).resolve().parent / "ocr-trocr-cells.py"

        def run_trocr(model_id: str, out_path: Path) -> None:
            cmd = [
                sys.executable,
                str(script_path),
                "--image-dir",
                str(image_dir),
                "--out",
                str(out_path),
                "--model",
                model_id,
                "--num-beams",
                str(args.ms_num_beams),
                "--length-penalty",
                str(args.length_penalty),
                "--min-conf",
                str(args.ms_min_conf),
                "--ambiguous-min-conf",
                str(args.ambiguous_min_conf),
                "--empty-threshold",
                str(args.empty_threshold),
                "--empty-trim",
                str(args.empty_trim),
            ]
            if args.use_fast_processor:
                cmd.append("--use-fast-processor")
            if args.device:
                cmd.extend(["--device", args.device])
            subprocess.run(cmd, check=True)

        run_trocr(args.printed_model, ocr_print_path)
        run_trocr(args.hand_model, ocr_hand_path)

    parseq_path = Path(args.parseq_json) if args.parseq_json else image_dir.with_name(
        f"{image_dir.name}-parseq.json"
    )

    if not args.parseq_json:
        script_path = Path(__file__).resolve().parent / "ocr-parseq-cells.py"
        parseq_beam_max_steps = 3
        cmd = [
            sys.executable,
            str(script_path),
            "--image-dir",
            str(image_dir),
            "--out",
            str(parseq_path),
            "--model",
            args.parseq_model,
            "--beam-size",
            str(args.parseq_beam_size),
            "--beam-max-steps",
            str(parseq_beam_max_steps),
            "--parseq-min-conf",
            str(args.parseq_min_conf),
        ]
        if args.use_fast_processor:
            cmd.append("--use-fast-processor")
        if args.device:
            cmd.extend(["--device", args.device])
        subprocess.run(cmd, check=True)
    elif parseq_path and not parseq_path.exists():
        raise SystemExit(f"Missing PARSeq output at {parseq_path}")

    parseq_data = None
    if parseq_path and parseq_path.exists():
        parseq_data = json.loads(parseq_path.read_text(encoding="utf-8"))

    ocr_data = None
    ocr_print = None
    ocr_hand = None
    if args.ocr_json:
        ocr_data = json.loads(Path(args.ocr_json).read_text(encoding="utf-8"))
    else:
        ocr_print = json.loads(ocr_print_path.read_text(encoding="utf-8"))
        ocr_hand = json.loads(ocr_hand_path.read_text(encoding="utf-8"))

    rows = []
    for idx, img in enumerate(images):
        parseq_lines = parseq_data[idx].get("lines", []) if parseq_data and idx < len(parseq_data) else []
        if ocr_print is not None and ocr_hand is not None:
            lines_print = ocr_print[idx].get("lines", []) if idx < len(ocr_print) else []
            lines_hand = ocr_hand[idx].get("lines", []) if idx < len(ocr_hand) else []
            rows.append((img, lines_print, lines_hand, parseq_lines))
        else:
            lines = ocr_data[idx].get("lines", []) if ocr_data and idx < len(ocr_data) else []
            rows.append((img, lines, parseq_lines))

    out_path = Path(args.out)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    def relpath(path: Path) -> str:
        try:
            return path.resolve().relative_to(out_path.parent.resolve()).as_posix()
        except ValueError:
            return path.as_posix()

    blank_html = "&nbsp;"

    def is_blank_image(path: Path, min_value: int = 250) -> bool:
        try:
            img = Image.open(path).convert("L")
        except Exception:
            return False
        lo, _hi = img.getextrema()
        return lo >= min_value

    def is_skipped(lines: list[dict]) -> bool:
        if not lines:
            return False
        return bool(lines[0].get("skipped"))

    def best_digit_from_lines(lines: list[dict]) -> tuple[str, float]:
        best_digit = "-"
        best_conf = 0.0
        for line in lines:
            text = line.get("text", "")
            conf = float(line.get("confidence", 0.0) or 0.0)
            raw_text = line.get("raw_text", "")
            raw_conf = float(line.get("raw_confidence", conf) or 0.0)
            candidates = []
            if text:
                candidates.append((text, conf))
            if raw_text and raw_text != text:
                candidates.append((raw_text, raw_conf))
            for cand_text, cand_conf in candidates:
                digits = "".join([c for c in cand_text if c.isdigit()])
                if not digits:
                    continue
                digit = digits[:1]
                if cand_conf > best_conf:
                    best_conf = cand_conf
                    best_digit = digit
        return best_digit, best_conf

    def parseq_digit_from_text(text: str) -> str:
        cleaned = text.strip()
        if not cleaned:
            return ""
        if len(cleaned) > 3:
            return ""
        if len(cleaned) == 1 and cleaned.isdigit():
            return cleaned
        if cleaned[0] in {"[", "|"}:
            if len(cleaned) == 2 and cleaned[1].isdigit():
                return cleaned[1]
            if len(cleaned) == 3 and cleaned[1].isdigit() and cleaned[2] in {"]", "|"}:
                return cleaned[1]
        return ""

    def parseq_digit_from_lines(lines: list[dict]) -> tuple[str, float]:
        best_digit = ""
        best_conf = 0.0
        for line in lines:
            text = line.get("text", "")
            conf = float(line.get("confidence", 0.0) or 0.0)
            raw_text = line.get("raw_text", "")
            raw_conf = float(line.get("raw_confidence", conf) or 0.0)
            candidates = []
            if text:
                candidates.append((text, conf))
            if raw_text and raw_text != text:
                candidates.append((raw_text, raw_conf))
            for cand_text, cand_conf in candidates:
                digit = parseq_digit_from_text(cand_text)
                if digit and cand_conf > best_conf:
                    best_conf = cand_conf
                    best_digit = digit
        return best_digit, best_conf

    def format_conf(conf: float) -> str:
        return f"{conf:.2f}" if conf > 0.0 else ""

    def bracket_conf(conf_text: str) -> str:
        return f"({conf_text})" if conf_text else ""

    def square_conf(conf_text: str) -> str:
        return f"[{conf_text}]" if conf_text else ""

    def format_meta(value_text: str, conf_text: str) -> str:
        if not conf_text:
            return f"<div>{value_text}</div>"
        return (
            "<div style=\"display:flex;align-items:baseline;\">"
            f"<span>{value_text}</span>"
            f"<span style=\"margin-left:auto;text-align:right;font-size:0.85em;\">{conf_text}</span>"
            "</div>"
        )

    def bold_if(text: str, enabled: bool) -> str:
        if enabled and text not in {"-", ""}:
            return f"**{text}**"
        return text

    def parseq_fallback(
        parseq_lines: list[dict],
        base_conf: float,
        blank: bool,
        skipped: bool,
        bypass_threshold: bool,
        parseq_bias: float,
    ) -> tuple[str, float] | None:
        if not parseq_lines:
            return None
        if skipped:
            return None
        if blank:
            return None
        if is_skipped(parseq_lines):
            return None
        if not bypass_threshold and base_conf > args.parseq_threshold:
            return None
        parseq_digit, parseq_conf = parseq_digit_from_lines(parseq_lines)
        if parseq_digit not in {"", "-"} and parseq_conf > 0.0:
            if parseq_conf < args.parseq_min_conf:
                return None
            if parseq_conf + parseq_bias < base_conf:
                return None
            return parseq_digit, parseq_conf
        raw_text = parseq_lines[0].get("raw_text", "") if parseq_lines else ""
        raw_conf = float(parseq_lines[0].get("raw_confidence", 0.0) or 0.0) if parseq_lines else 0.0
        if raw_conf <= 0.0 and not raw_text:
            return None
        if any(char.isdigit() for char in raw_text):
            return None
        if raw_conf + parseq_bias < base_conf:
            return None
        if raw_conf < args.parseq_empty_conf:
            return None
        return "-", raw_conf

    cols = max(1, args.cols)
    with out_path.open("w", encoding="utf-8") as f:
        header_cells = ["#"] + [str(i) for i in range(1, cols + 1)]
        f.write("| " + " | ".join(header_cells) + " |\n")
        f.write("| " + " | ".join(["---"] * len(header_cells)) + " |\n")

        row_cells = []
        row_index = 0
        for row in rows:
            img = row[0]
            blank = is_blank_image(img)
            if len(row) == 4:
                lines_print = row[1]
                lines_hand = row[2]
                parseq_lines = row[3]
                if blank:
                    display_text = blank_html
                    label = blank_html
                    meta = f"<div>{blank_html}</div>"
                else:
                    print_skipped = is_skipped(lines_print)
                    hand_skipped = is_skipped(lines_hand)
                    print_digit, print_conf = best_digit_from_lines(lines_print)
                    hand_digit, hand_conf = best_digit_from_lines(lines_hand)
                    base_conf = max(print_conf, hand_conf)
                    if not print_skipped and not hand_skipped and print_digit != hand_digit:
                        base_conf = max(0.0, base_conf - 0.2)
                    bypass_threshold = print_digit in {"1", "7"} or hand_digit in {"1", "7"}
                    parseq_choice = parseq_fallback(
                        parseq_lines,
                        base_conf,
                        blank,
                        print_skipped and hand_skipped,
                        bypass_threshold,
                        args.parseq_bias,
                    )
                    if parseq_choice:
                        display_text = parseq_choice[0]
                        conf_str = square_conf(format_conf(parseq_choice[1]))
                        label = f"{display_text} {conf_str}" if conf_str else display_text
                        meta = format_meta(display_text, conf_str)
                    elif print_skipped and hand_skipped:
                        display_text = blank_html
                        label = blank_html
                        meta = f"<div>{blank_html}</div>"
                    elif print_skipped and not hand_skipped:
                        display_text = hand_digit if hand_digit != "-" else blank_html
                        conf_str = bracket_conf(format_conf(hand_conf))
                        label = f"{display_text} {conf_str}" if conf_str else display_text
                        meta = format_meta(display_text, conf_str)
                    elif hand_skipped and not print_skipped:
                        display_text = print_digit if print_digit != "-" else blank_html
                        conf_str = bracket_conf(format_conf(print_conf))
                        label = f"{display_text} {conf_str}" if conf_str else display_text
                        meta = format_meta(display_text, conf_str)
                    elif print_digit == hand_digit:
                        display_text = print_digit
                        best_conf = max(print_conf, hand_conf)
                        conf_str = bracket_conf(format_conf(best_conf))
                        label = f"{display_text} {conf_str}" if conf_str else display_text
                        meta = format_meta(display_text, conf_str)
                    else:
                        if print_conf > hand_conf:
                            print_display = bold_if(print_digit, True)
                            hand_display = bold_if(hand_digit, False)
                        elif hand_conf > print_conf:
                            print_display = bold_if(print_digit, False)
                            hand_display = bold_if(hand_digit, True)
                        else:
                            print_display = bold_if(print_digit, False)
                            hand_display = bold_if(hand_digit, False)
                        print_conf_str = format_conf(print_conf)
                        hand_conf_str = format_conf(hand_conf)
                        display_text = f"{print_display} / {hand_display}"
                        conf_text = " / ".join([c for c in [print_conf_str, hand_conf_str] if c])
                        conf_text = bracket_conf(conf_text)
                        label = f"{display_text} {conf_text}" if conf_text else display_text
                        meta = format_meta(display_text, conf_text)
            else:
                lines = row[1]
                parseq_lines = row[2]
                if blank or is_skipped(lines):
                    display_text = blank_html
                    display_conf = 0.0
                else:
                    display_text, display_conf = best_digit_from_lines(lines)
                parseq_choice = parseq_fallback(
                    parseq_lines,
                    display_conf,
                    blank,
                    is_skipped(lines),
                    display_text in {"1", "7"},
                    args.parseq_bias,
                )
                if parseq_choice:
                    display_text = parseq_choice[0]
                    conf_str = square_conf(format_conf(parseq_choice[1]))
                else:
                    conf_str = bracket_conf(format_conf(display_conf))
                label = f"{display_text} {conf_str}" if conf_str else f"{display_text}"
                meta = format_meta(display_text, conf_str)
            cell = f"![{label}]({relpath(img)})<br>{meta}"
            row_cells.append(cell)
            if len(row_cells) == cols:
                row_index += 1
                f.write("| " + " | ".join([str(row_index)] + row_cells) + " |\n")
                row_cells = []

        if row_cells:
            while len(row_cells) < cols:
                row_cells.append(" ")
            row_index += 1
            f.write("| " + " | ".join([str(row_index)] + row_cells) + " |\n")


if __name__ == "__main__":
    main()
