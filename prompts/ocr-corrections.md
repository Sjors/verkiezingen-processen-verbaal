Read the correction table in the image and return only one Markdown table.

Start your response immediately with the Markdown table. Do not think step by
step. Do not describe the image. Do not add commentary.

Use this exact format:

| ID | First | Second | Difference | Note |
|---|---:|---:|---:|---|

Rules:

- Output no prose before or after the table.
- Use one row for each handwritten correction row.
- Only read handwritten values from the table rows. Do not turn explanatory
  text in the Note column into extra rows unless that same physical row has a
  handwritten difference, first count, or second count.
- If a row has a blank list-number cell but has a handwritten difference, treat
  it as a continuation row for the most recent non-empty ID and repeat that ID.
- Omit rows that are clearly crossed out or marked as void/cancelled, such as
  `vervalt`.
- Map list number `1` through `20` to `E.1` through `E.20`.
- Map `blanco` to `F`.
- Map `ongeldig` to `G`.
- Map a correction for total valid votes or total candidate votes to `E`.
- Map a correction for total cast votes or `correctie uitgebrachte stemmen` to `H`.
- `First` is the value established by the stembureau.
- `Second` is the value established by the gemeentelijk stembureau.
- `Difference` is `Second - First`, as a signed integer without leading zeros.
- Always fill `Difference` for rows you output. If both `First` and `Second`
  are written, compute the difference from those values.
- If a first or second value is not written, leave that cell empty but still fill `Difference`.
- Preserve any handwritten explanation in `Note`; otherwise leave `Note` empty.
- If the correction table has no correction rows, return only the header and separator.
