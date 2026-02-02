import argparse
import math

import torch
from PIL import Image


def decode(tokens, itos, eos_id):
    out = []
    for tid in tokens:
        if tid == eos_id:
            break
        out.append(itos[tid])
    return "".join(out)


def main() -> None:
    parser = argparse.ArgumentParser(description="Show top-N PARSeq candidates for one image.")
    parser.add_argument("--image", required=True, help="Path to a single crop image")
    parser.add_argument("--top-n", type=int, default=10, help="Number of candidates to show")
    parser.add_argument("--model", default="parseq", help="Torch Hub model entry")
    parser.add_argument("--beam-size", type=int, default=10, help="Beam size for decoding")
    parser.add_argument("--step-topk", type=int, default=20, help="Top-k tokens to expand per step")
    args = parser.parse_args()

    model = torch.hub.load("baudm/parseq", args.model, pretrained=True)
    model.eval()
    from strhub.data.module import SceneTextDataModule

    img_transform = SceneTextDataModule.get_transform(model.hparams.img_size)
    img = Image.open(args.image).convert("RGB")
    img = img_transform(img).unsqueeze(0)
    with torch.no_grad():
        logits = model(img)

    probs = logits.softmax(-1)[0]
    itos = model.tokenizer._itos
    bos_id = model.tokenizer.bos_id
    pad_id = model.tokenizer.pad_id
    eos_id = model.tokenizer.eos_id

    beam = [([], 0.0, [])]
    finished = []

    for step_probs in probs:
        new_beam = []
        topk = torch.topk(step_probs, k=min(args.step_topk, step_probs.numel()))
        for tokens, logp, confs in beam:
            if tokens and tokens[-1] == eos_id:
                finished.append((tokens, logp, confs))
                continue
            for token_id, p in zip(topk.indices.tolist(), topk.values.tolist()):
                if token_id in (bos_id, pad_id):
                    continue
                new_tokens = tokens + [token_id]
                new_logp = logp + math.log(max(p, 1e-12))
                new_confs = confs + [p]
                new_beam.append((new_tokens, new_logp, new_confs))
        new_beam.sort(key=lambda x: x[1], reverse=True)
        beam = new_beam[: args.beam_size]

    candidates = finished + beam

    best = {}
    for tokens, logp, confs in candidates:
        text = decode(tokens, itos, eos_id)
        if not text:
            continue
        avg_conf = sum(confs) / len(confs) if confs else 0.0
        if text not in best or logp > best[text][1]:
            best[text] = (avg_conf, logp)

    final = sorted(
        [(t, v[0], v[1]) for t, v in best.items()], key=lambda x: x[2], reverse=True
    )[: max(1, args.top_n)]
    for text, avg_conf, logp in final:
        print(f"{text}\tavg_conf={avg_conf:.3f}\tlogp={logp:.3f}")


if __name__ == "__main__":
    main()
