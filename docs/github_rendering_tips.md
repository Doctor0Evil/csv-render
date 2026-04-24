# GitHub Rendering Tips for CSV Files

This guide explains how GitHub renders CSV files, what can cause rendering to fail, and how to structure files so they are both GitHub‑friendly and compatible with the `csv-render` validation pipeline.

The goal is to make every CSV in this repository show up as a clean, interactive table on GitHub, instead of falling back to raw text.

---

## 1. How GitHub renders CSV files

GitHub has built-in support for rendering `.csv` and `.tsv` files as interactive tables:

- When you click a CSV file in the repository browser, GitHub attempts to parse it.
- If parsing succeeds and the file is not too large, GitHub shows a table view with:
  - The first row treated as the header.
  - Row numbers on the left.
  - Scrollable, filterable content depending on the UI.

If parsing fails or the file exceeds certain thresholds, GitHub will instead show the file as plain text or only allow “View raw”:

- Extremely large CSVs may skip table rendering.
- Structurally malformed CSVs (unclosed quotes, mismatched columns) can cause the viewer to fail.

`csv-render` is designed so that any CSV passing the validator is structurally sound and safe to render.

---

## 2. File size and complexity

GitHub does not publicly guarantee a fixed maximum size for rendering CSVs, but practical experience suggests:

- Small to moderately sized files (hundreds of kilobytes, thousands of rows) generally render fine.
- Very large files (multi‑megabyte or millions of rows) are more likely to be shown as raw text or might be slow to load.
- Git LFS or very large binary blobs are not rendered as tables.

Practical tips for this repository:

- Keep example and test CSVs small and focused.
- Store very large datasets outside the main `examples/` directory, or provide a small “sample” CSV alongside the full data.
- If you need to share large tables, consider:
  - A smaller CSV that demonstrates the structure.
  - A separate data hosting solution or compressed artifacts.

---

## 3. Structural correctness and RFC 4180

GitHub’s renderer is sensitive to structural errors, many of which are directly covered by RFC 4180:

- **Unclosed quoted fields**: A field starting with `"` must end with a matching `"` on the same physical line, unless the content intentionally spans multiple lines and is properly closed.
- **Mismatched column counts**: Each row must have the same number of fields as the header row.
- **Stray commas**: A comma outside of quoted fields is always treated as a delimiter, creating extra columns.

To keep CSVs renderable:

- Quote any field that contains commas, double quotes, or newlines.
- Escape any double quotes inside a quoted field by doubling them (`""`).
- Run the `csv-core` validator or `csv-cli` before committing CSVs so structural issues are caught early.

If GitHub is showing a CSV as raw text instead of a table, check for these issues first.

---

## 4. Delimiters and locale issues

GitHub’s CSV viewer expects:

- `.csv` files to be comma‑separated.
- `.tsv` files to be tab‑separated.

Common pitfalls:

- Files that use semicolons (`;`) as the main separator but have a `.csv` extension may confuse GitHub’s renderer.
- Locale-specific exports from some spreadsheet tools may use semicolons or other delimiters by default.

In `csv-render`:

- Commas are the only allowed primary delimiter for `.csv` files.
- Semicolons are allowed inside fields as data (for example, as internal list separators) but not as column separators.
- If you need tab‑separated files, use `.tsv` and handle them separately; this project focuses on strict `.csv` semantics.

---

## 5. Encoding and line endings

GitHub expects text files to be:

- UTF‑8 encoded.
- Free of byte order marks (BOM) where possible.

To avoid subtle issues:

- Save all CSV files as UTF‑8 without BOM.
- Use either Unix line endings (`\n`) or Windows line endings (`\r\n`), but avoid mixing styles within the same file.
- Ensure your editor is configured to use UTF‑8 and consistent line endings for the repository.

The `csv-core` validator assumes UTF‑8 and consistent line breaks; mismatches can lead to parsing errors or confusing diffs.

---

## 6. Headers and row interpretation

GitHub assumes the first row in a CSV is the header:

- The first line defines column names.
- The table view uses these names in the header row.

In this project:

- The header row must match the schema in `configs/*.yaml` exactly (names and order).
- The header row must not contain extra commas or unescaped characters.

If GitHub’s column headings look wrong (for example, shifted or truncated):

- Check the header line for stray commas or misquoted fields.
- Run the validator to check for header/schema mismatches.

---

## 7. Commas, quotes, and notes fields

A common reason for GitHub rendering failures is free‑text fields that contain commas (for example, notes or descriptions). To remain GitHub‑friendly:

- Always quote fields that contain commas, even if they are human-readable text.
- Apply the same quoting rigor to fields with double quotes or newlines.
- Prefer keeping long free‑text notes relatively short in example CSVs, and use Markdown or separate documentation files for extended prose.

In the governance particle examples:

- The `notes` column is designed to contain commas and other punctuation.
- Every `notes` value is wrapped in double quotes, and any internal quotes are doubled.

This makes the files robust under both GitHub’s viewer and the Rust CSV parser.

---

## 8. Integration with Markdown documentation

Sometimes you want to show a CSV as a table directly inside a Markdown document:

- GitHub can render CSV files as tables when you view them directly.
- For README or docs, use Markdown tables or convert CSVs to Markdown using a tool (such as the Python `csv_to_markdown.py` script in this repo).

Best practices:

- Keep the canonical source of truth as a `.csv` file.
- Generate Markdown tables from CSV when you need to embed data in docs.
- Avoid manually duplicating table content in multiple places; re‑generate it from the CSV when it changes.

This approach keeps CSVs easy to validate and documentation easy to maintain.

---

## 9. Using csv-render to keep CSVs GitHub‑ready

To ensure CSVs render correctly on GitHub:

1. **Author or generate CSVs** according to RFC 4180 and the project’s schemas.
2. **Run the Rust CLI**:
   - `cargo run -p csv-cli -- validate --schema configs/your-schema.yaml path/to/file.csv`
3. **Optionally run Python tools**:
   - Use `python/src/csv_validator.py` for quick structural checks.
   - Use `python/converters/csv_to_markdown.py` to generate Markdown tables.
4. **Commit only CSVs that pass validation**.
5. **Inspect the file on GitHub** to confirm it renders as an interactive table.

If GitHub still renders the file as raw text after validation, investigate:

- File size and row count.
- Repository settings or Git LFS usage.
- Any unusual characters or encodings not covered by the basic checks.

---

## 10. Quick checklist for GitHub‑friendly CSVs

Before you push:

- [ ] File extension is `.csv`.
- [ ] Content is UTF‑8 without BOM.
- [ ] Uses commas as the only field delimiter.
- [ ] Header row matches the schema’s column names and order.
- [ ] Every row has the same number of fields as the header.
- [ ] All fields containing commas, quotes, or newlines are quoted.
- [ ] Any quotes inside quoted fields are escaped as `""`.
- [ ] File size and row count are reasonable for GitHub’s table viewer.
- [ ] The file passes `csv-cli validate` and any relevant Python checks.

Following this checklist makes it very likely that your CSV will appear as a clean, interactive table in GitHub and integrate smoothly with the `csv-render` validation ecosystem.
