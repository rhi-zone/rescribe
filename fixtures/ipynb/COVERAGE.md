# Jupyter Notebook (ipynb) Fixture Coverage

A fixture suite is complete when all items below are checked.
See `fixtures/spec.md` for category definitions.

Jupyter Notebook format (nbformat 4) is defined at
https://nbformat.readthedocs.io/en/latest/format_description.html.
A notebook has a `metadata` object, `nbformat`/`nbformat_minor` version fields,
and a `cells` array. Each cell has a `cell_type`, `source`, and optionally `outputs`.

## Cell types

- [x] code cell — `code-cell`
- [x] markdown cell — `markdown-cell`
- [x] raw cell — `raw-cell`
- [ ] (nbformat 3 only) heading cell — handled via markdown `heading-cell`

## Code cell features

- [x] source as single string — `code-cell`
- [x] source as array of strings (joined) — `rare-source-array`
- [x] execution_count stored as ipynb:execution_count — `code-cell-with-language`
- [x] language from kernelspec metadata — `code-cell-with-language`
- [ ] language from language_info metadata — (missing)
- [ ] code cell with no outputs — `code-cell`
- [x] code cell with outputs — `code-cell` (implicitly via output tests)

## Markdown cell features

- [x] markdown cell → paragraph — `markdown-cell`
- [x] markdown cell with ATX heading → heading node — `heading-cell`
- [ ] markdown cell with bold / emphasis — (missing)
- [ ] markdown cell with link — (missing)
- [ ] markdown cell with image — (missing)
- [ ] markdown cell with code span — (missing)
- [ ] markdown cell with list — (missing)
- [ ] markdown cell with table — (missing)
- [ ] markdown cell with blockquote — (missing)
- [ ] source as array of strings — (missing; `rare-source-array` tests code cell only)

## Raw cell features

- [x] raw cell with no format metadata → raw_block with format=text — `raw-cell`
- [ ] raw cell with format metadata (e.g., "html", "latex") — (missing)

## Output types

- [x] display_data with text/html → raw_block — `output-html`
- [x] display_data with image/png → image node — `output-image-png`
- [x] display_data with image/jpeg → image node — `output-image-jpeg`
- [x] stream output → code_block with ipynb:output_type=stream — `rare-output-stream`
- [x] stream name (stdout / stderr) stored as ipynb:stream_name — `rare-output-stream`
- [x] error output → code_block with ipynb:output_type=error — `output-error`
- [ ] execute_result output — (missing)
- [ ] display_data with text/plain — (missing)
- [ ] display_data with image/svg+xml — (missing)
- [ ] display_data with application/json — (missing)
- [ ] display_data with multiple MIME types (priority ordering) — (missing)
- [ ] multiple outputs on a single cell — (missing)

## Notebook metadata

- [ ] kernelspec.display_name — (missing)
- [x] kernelspec.language (used for code cell language) — `code-cell-with-language`
- [x] kernelspec.name — `code-cell-with-language`
- [ ] language_info.name (fallback language) — (missing)
- [ ] notebook-level title (if any) — (missing)

## Document-level structure

- [x] empty notebook (no cells) — `adv-empty`
- [x] single cell — `code-cell`, `markdown-cell`, `raw-cell`
- [x] multiple cells in sequence — `multi-cell`

## Composition (integration)

- [x] heading cell + code cell — `multi-cell`
- [ ] markdown cell with rich content + code cell + output — (missing)
- [ ] multiple output types on one cell — (missing)

## Adversarial

- [x] empty cells array — `adv-empty`
- [ ] cell with missing cell_type — (missing)
- [ ] cell with unknown cell_type — (missing)
- [ ] cell with missing source — (missing)
- [ ] output with unknown output_type — (missing)
- [ ] malformed JSON — (missing)
- [ ] nbformat version mismatch — (missing)

## Pathological

- [ ] notebook with 1000 cells — (missing)
- [ ] code cell with 1 MB of source — (missing)
- [ ] cell with 100 outputs — (missing)
