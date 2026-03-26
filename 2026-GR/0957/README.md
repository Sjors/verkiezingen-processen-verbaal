# Roermond

Proces-verbalen reproduceerbaar te downloaden vanaf de Google Drive-links op
`https://www.roermond.nl/uitslag-verkiezingen-gemeenteraad-2026`.

## Downloadmethode

Roermond publiceerde de documenten in drie publieke Google Drive-mappen in plaats
van als directe PDF-links op de website. Deze zijn opnieuw opgehaald en
geverifieerd met:

```bash
./scripts/scrapers/googledrive.py \
  per-kandidaat=https://drive.google.com/drive/folders/1_G0uhO3ssfArx4XU16gvbq6BK3IH0GH3 \
  op-partijniveau=https://drive.google.com/drive/folders/1GG81l9a1O3nes35mSGnaGRls4BjeEeU_ \
  hertellingen=https://drive.google.com/drive/folders/1sXceJd5KREdM8pWR17bGjFsos1giwnK4 \
  tmp/roermond_recheck
```

De publieke mapweergave (`embeddedfolderview`) levert de bestandsnamen en Drive
file IDs op. De scriptdownload via `uc?export=download&id=...` geeft dezelfde
bestanden als de eerder handmatig binnengehaalde set: `SHA256SUMS` kwam exact
overeen.

De bestanden zijn georganiseerd in drie submappen:

- `per-kandidaat/` — 35 PDF's
- `op-partijniveau/` — 35 PDF's
- `hertellingen/` — 35 PDF's

De bronlinks staan in `0957 Roermond.txt`.

## Stembureaus per verkiezing

| Verkiezing | Stembureaus |
|------------|-------------|
| 2026-GR    | 35          |
