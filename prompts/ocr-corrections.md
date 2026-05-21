Read the correction table in the image and return only one Markdown table.

Use this exact format:

| ID | First | Second | Difference | Note |
|---|---:|---:|---:|---|

Rules:

- Output no prose before or after the table.
- Use one row for each handwritten correction row.
- Map list number `1` through `20` to `E.1` through `E.20`.
- Map `blanco` to `F`.
- Map `ongeldig` to `G`.
- Map a correction for total valid votes or total candidate votes to `E`.
- Map a correction for total cast votes or `correctie uitgebrachte stemmen` to `H`.
- `First` is the value established by the stembureau.
- `Second` is the value established by the gemeentelijk stembureau.
- `Difference` is `Second - First`, as a signed integer without leading zeros.
- If a first or second value is not written, leave that cell empty but still fill `Difference`.
- Preserve any handwritten explanation in `Note`; otherwise leave `Note` empty.
- If the correction table has no correction rows, return only the header and separator.
