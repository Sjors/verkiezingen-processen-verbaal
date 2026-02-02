import math

import torch
from PIL import Image

MODEL_NAME = "parseq"
IMAGE_PATH = "tmp/ocr/0307-adventkerk-28_0-cells-prep5-jpg/list-20-col-1.png"
BEAM_SIZE = 10
STEP_TOPK = 20

model = torch.hub.load("baudm/parseq", MODEL_NAME, pretrained=True)
model.eval()
from strhub.data.module import SceneTextDataModule
img_transform = SceneTextDataModule.get_transform(model.hparams.img_size)
img = Image.open(IMAGE_PATH).convert("RGB")
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
    topk = torch.topk(step_probs, k=min(STEP_TOPK, step_probs.numel()))
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
    beam = new_beam[:BEAM_SIZE]

candidates = finished + beam


def decode(tokens):
    out = []
    for tid in tokens:
        if tid == eos_id:
            break
        out.append(itos[tid])
    return "".join(out)


best = {}
for tokens, logp, confs in candidates:
    text = decode(tokens)
    if not text:
        continue
    avg_conf = sum(confs) / len(confs) if confs else 0.0
    if text not in best or logp > best[text][1]:
        best[text] = (avg_conf, logp)

final = sorted([(t, v[0], v[1]) for t, v in best.items()], key=lambda x: x[2], reverse=True)[:10]
for text, avg_conf, logp in final:
    print(f"{text}\tavg_conf={avg_conf:.3f}\tlogp={logp:.3f}")
