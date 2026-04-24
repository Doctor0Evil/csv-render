# RFC 4180 Guide for csv-render

This guide summarizes the parts of RFC 4180 that are most relevant for the `csv-render` project and explains how they interact with GitHub’s CSV rendering and the `csv-core` validator.

The goal is simple: any CSV that passes the rules in this guide should parse cleanly with standard tools, render correctly as a table on GitHub, and be acceptable to the Rust validation engine.

---

## 1. What RFC 4180 Covers

RFC 4180 describes a common format and MIME type for Comma-Separated Values (CSV) files. It does not standardize every possible variation of CSV in the wild, but it captures the most widely used conventions and clarifies how fields, quotes, and line breaks should behave.

The key areas it addresses are:

- How records (rows) are separated.
- How fields (columns) are separated.
- How to handle fields that contain commas, double quotes, or newlines.
- How to represent header rows.

This guide focuses on those aspects and on how `csv-render` enforces them.

---

## 2. Records and line endings

A CSV file is a sequence of records. In RFC 4180:

- Each record is placed on its own line.
- Records are separated by line breaks.
- Line breaks are typically represented as CRLF (`\r\n`), but many tools accept LF (`\n`) as well.

In `csv-render`:

- We accept both `\n` and `\r\n` line endings, but files should be internally consistent.
- Each non-empty line is expected to represent exactly one record.
- There should be no stray line fragments caused by unclosed quotes.

Consistency of line endings helps tools and Git diff views behave predictably, but most modern systems normalize line endings automatically.

---

## 3. Fields and delimiters

Within each record, fields are separated by commas:

- The comma (`,`) is the only field delimiter.
- There must be the same number of fields in every record, including the header row.

This implies:

- If the header row has 5 fields, every subsequent row must also have exactly 5 fields.
- There should be no trailing comma at the end of any row; a trailing comma creates an extra empty field.

In `csv-render`:

- The Rust validator checks that the header and all data rows have identical field counts.
- Any mismatch (extra or missing columns) is treated as a structural error.
- We do not use semicolons or tabs as primary delimiters; semicolons may appear inside fields as data.

---

## 4. Quoting rules for fields

RFC 4180 defines when a field must be quoted and how quotes behave.

### 4.1 When to quote a field

A field must be enclosed in double quotes if it contains any of the following:

- A comma.
- A double quote.
- A line break (CRLF or LF).

If a field does not contain these characters, it may be left unquoted. For clarity and safety, it is acceptable to quote more fields than strictly necessary, but never less.

Examples:

- `hello` is fine unquoted.
- `"hello, world"` is required if the field contains a comma.
- `"Line with\nnewline"` is required if the field contains a newline.
- `"He said ""hello"""` is required if the field contains double quotes.

In `csv-render`, the validator assumes:

- Commas split fields unless they appear inside a properly quoted field.
- Newlines always terminate records unless they are inside a properly quoted field.
- Quotes are significant and must follow the escaping rules below.

### 4.2 Escaping double quotes inside fields

Double quotes inside a quoted field must be represented as two consecutive double quotes:

- Logical content: `He said "hello".`
- CSV field: `"He said ""hello""."`

This is the only valid way to escape double quotes inside CSV fields under RFC 4180.

The validator treats any mismatched or unescaped double quotes as structural errors. This includes:

- Opening a quoted field with `"` and never closing it.
- Using a bare `"` inside a quoted field without doubling it.

---

## 5. Header row and column consistency

RFC 4180 recommends that CSV files start with a header row:

- The first line of the file is a header that names each field.
- Subsequent records then provide data for those fields.

`csv-render` treats the header row as authoritative:

- The number of fields in the header row defines the expected number of columns for all data rows.
- The names in the header row must match the column names declared in the schema.

If the schema for a table expects:

- `host_did, consent_ledger_refs, timestamp, neurorights_flags, notes`

then the header row must use exactly those names, in exactly that order, with the same spelling and casing. Any deviation (extra field, missing field, different name, or reordering) will be reported as a structural mismatch.

---

## 6. Empty fields and optional columns

RFC 4180 allows fields to be empty. An empty field is represented as nothing between delimiters, for example:

- `value1,,value3`

Here the second field is empty. This can also be written as `""` to make the empty field explicit:

- `value1,"",value3`

In `csv-render`:

- The schema indicates whether a column is required or optional.
- If a required field is empty (or effectively missing), it is a semantic error.
- Optional fields may be empty; using `""` is often clearer and keeps the CSV visually aligned.

The validator distinguishes between structural correctness (field exists) and semantic correctness (field satisfies type and constraint rules).

---

## 7. Newlines inside fields

RFC 4180 permits line breaks inside fields, provided the field is quoted:

- `"First line\nSecond line"`

In practice, this is fragile for GitHub rendering and some tools, because line breaks inside fields can be misinterpreted or make diffs harder to read.

In `csv-render`:

- Newlines inside fields are allowed but discouraged.
- If they are used, the field must be quoted.
- Many examples and tests intentionally avoid embedded newlines to keep behavior simple and predictable.

When in doubt, prefer keeping each record on a single physical line.

---

## 8. Encoding and BOM

RFC 4180 does not mandate a specific character encoding, but:

- UTF-8 has become the de facto standard for modern systems.
- A Byte Order Mark (BOM) at the start of the file (`\uFEFF`) can confuse some parsers.

`csv-render` assumes:

- Files are encoded as UTF-8 without a BOM.
- If a BOM or other unexpected leading bytes are detected, they may be treated as part of the first field, causing subtle errors.

To avoid these issues, make sure your editors and tools save CSV files as plain UTF-8 without BOM.

---

## 9. How GitHub interacts with RFC 4180

GitHub uses a built-in CSV viewer to render files as interactive tables when:

- The file is small enough (in terms of size and row count) to be rendered.
- The CSV is structurally valid and can be parsed reliably.

Common reasons GitHub falls back to showing raw text include:

- Unclosed quoted fields.
- Inconsistent column counts across rows.
- Extremely large files.

Because `csv-render` enforces RFC 4180 rules, the same problems that break parsers are also flagged by the validator. The goal is that any CSV accepted by `csv-core` will also be safe to render on GitHub.

---

## 10. How csv-core enforces RFC 4180

The Rust `csv-core` library in this repository uses a CSV parser configured to behave in line with RFC 4180:

- Comma delimiter, double quote as the quote character.
- Header row required and matched against the schema.
- Flexible mode disabled so that each record must have a consistent number of fields.

On top of that, `csv-core` adds:

- Schema-based checks for types (for example, parsing `u64`).
- Constraints on allowed values, such as a fixed set of neurorights flags.
- Clear error messages that identify the row, column, and reason for failure.

This means that:

- Structural rules from RFC 4180 are enforced first.
- Semantic rules from the schema are applied second.
- Any failure at either stage is treated as a validation error and should be corrected before committing CSV files.

---

## 11. Practical checklist for authors and AI systems

Before committing or accepting a CSV file in this project, verify:

1. The file is saved as UTF-8 without BOM.
2. The header row uses the exact column names and order defined in the schema.
3. Every data row has the same number of fields as the header.
4. Any field containing a comma, double quote, or newline is fully enclosed in double quotes.
5. Any double quote inside a quoted field is escaped as `""`.
6. No row has a trailing comma.
7. Required fields are not empty.
8. Values conform to the expected types and allowed sets (for example, timestamps as integers, neurorights flags from the allowed list).

If all these checks pass, the CSV file should be both RFC 4180-compliant and GitHub-ready, and it should pass the `csv-core` validator without errors.
