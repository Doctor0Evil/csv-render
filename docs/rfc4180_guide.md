# RFC 4180 Guide for `csv-render`

This guide explains how the `csv-render` project interprets and enforces RFC 4180, the de facto standard for Comma-Separated Values (CSV) files. It is written for both humans and AI systems that need to generate and validate CSVs which render correctly on GitHub and pass the `csv-core` Rust validator.

The focus is on two layers of correctness:

- **Structural validity**, as defined by RFC 4180 and required for GitHub’s CSV viewer.
- **Semantic validity**, as defined by the project’s schema (`governance-particle-schema.yaml`) and related configuration files.

The examples in this guide assume the governance particle schema with the header:

`host_did, consent_ledger_refs, timestamp, neurorights_flags, notes`

---

## 1. Core RFC 4180 concepts

RFC 4180 defines a simple but strict model for CSV files:

- A CSV file consists of **records**, one per line.
- Each record is made up of **fields**, separated by a **comma**.
- The first record may be a **header row** that names the columns.
- Fields may be optionally enclosed in **double quotes**.
- Fields that contain commas, double quotes, or newlines **must** be enclosed in double quotes.
- Double quotes inside a quoted field are represented by **two double quotes**.

The `csv-render` project treats these rules as non-negotiable at the structural layer. Any CSV that violates them is considered structurally invalid and is rejected before schema-based checks run.

---

## 2. Delimiters, quoting, and escaping

In this project, a CSV is always:

- **Comma-delimited** (`,` as the delimiter).
- Using **double quotes** (`"`) as the only quote character.
- Using **no alternate delimiters** (no semicolons or tabs as field separators).

### Fields that must be quoted

A field must be enclosed in double quotes if it contains any of the following:

- A comma (`,`)
- A double quote (`"`)
- A newline (line break)

Examples:

- `PlainText` — no special characters, quoting optional.
- `"Text with, comma"` — contains a comma, so it must be quoted.
- `"Text with ""quote"""` — contains a double quote; it is quoted, and the internal quote is doubled.
- `"First line\nSecond line"` — contains a newline, so it must be quoted.

### Escaping double quotes

Inside a quoted field, each literal double quote is written as two double quotes:

- Logical content: `He said "hello".`
- CSV field: `"He said ""hello"". "` 

The parser treats `""` as a single literal `"` inside the field.

---

## 3. Consistent column counts

RFC 4180 requires that each record have the same number of fields. In practice:

- The **header row** defines the number of columns.
- Every data row must have **exactly the same number** of fields, in the same order.
- There are **no trailing commas** after the last field in a row.

If the header has 5 fields, then every subsequent row must also have exactly 5 fields. For example, with the governance particle schema:

`host_did, consent_ledger_refs, timestamp, neurorights_flags, notes`

every row must have five fields corresponding to those names.

Examples:

Valid (5 fields):

```csv
host_did,consent_ledger_refs,timestamp,neurorights_flags,notes
did:example:host1,https://ledger.example/consent/abc123;https://ledger.example/consent/def456,1713825600,cognitive_liberty;mental_privacy,"Initial governance record."
```

Invalid (extra field):

```csv
host_did,consent_ledger_refs,timestamp,neurorights_flags,notes
did:example:host1,https://ledger.example/consent/abc123;https://ledger.example/consent/def456,1713825600,cognitive_liberty;mental_privacy,"Extra field","Oops"
```

Invalid (missing field):

```csv
host_did,consent_ledger_refs,timestamp,neurorights_flags,notes
did:example:host1,https://ledger.example/consent/abc123;https://ledger.example/consent/def456,1713825600,cognitive_liberty;mental_privacy
```

The `csv-core` validator rejects any row whose field count does not match the header.

---

## 4. Header row and schema alignment

The header row is both a structural and semantic boundary:

- Structurally, it defines the number and ordering of columns.
- Semantically, it ties each position to a named field in the schema.

If the schema for a table defines the columns:

`host_did, consent_ledger_refs, timestamp, neurorights_flags, notes`

then the header row must use **exactly those names**, in **exactly that order**, with the same spelling and casing. Any deviation (extra field, missing field, different name, or reordering) is reported as a **structural** mismatch by the validator.

Examples of invalid headers for this schema:

- `HostDID, consent_ledger_refs, timestamp, neurorights_flags, notes` (wrong casing)
- `host_did, timestamp, consent_ledger_refs, neurorights_flags, notes` (reordered)
- `host_did, consent_ledger_refs, timestamp, neurorights_flags` (missing `notes`)

Correct header:

```csv
host_did,consent_ledger_refs,timestamp,neurorights_flags,notes
```

---

## 5. Structural vs semantic validation

`csv-render` separates validation into two layers:

1. **Structural validation** (RFC 4180)
   - Enforced by the CSV parser configuration and initial checks.
   - Ensures the file is well-formed: correct quoting, escaping, delimiters, and consistent column counts.
   - Fails on issues such as unclosed quotes, extra fields, or missing fields.

2. **Semantic validation** (schema-driven)
   - Enforced by `csv-core` using `governance-particle-schema.yaml` and additional config files.
   - Ensures that each field’s value is meaningful and type-correct:
     - `timestamp` parses as a `u64`.
     - `consent_ledger_refs` is a list of URIs split by `;`.
     - `neurorights_flags` contains only allowed flags from `neurorights-flags.toml`.
   - Fails on issues such as non-numeric timestamps, unknown neurorights flags, or missing required values.

A file like `rust/tests/broken.csv` is intentionally constructed to:

- Pass all **structural** checks (so it is RFC 4180-compliant and GitHub-renderable).
- Fail at the **semantic** layer (e.g., `not-a-timestamp`, `unknown_flag`).

This dual-layer approach allows tests to confirm that both structural and semantic validation logic are functioning correctly.

---

## 6. Empty fields and optional columns

RFC 4180 allows fields to be empty. An empty field is represented as nothing between delimiters, for example:

- `value1,,value3`

Here the second field is empty. This can also be written as `""` to make the empty field explicit:

- `value1,"",value3`

In `csv-render`:

- The schema indicates whether a column is **required** or **optional**.
- If a **required** field is empty (or effectively missing), it is treated as a **semantic error**.
- **Optional** fields may be empty; using `""` is often clearer and keeps the CSV visually aligned.

The validator distinguishes between structural correctness (field is present in the row) and semantic correctness (field satisfies type and constraint rules).

---

## 7. Newlines inside fields

RFC 4180 permits line breaks inside fields, provided the field is quoted:

- `"First line\nSecond line"`

In practice, this can be fragile for GitHub rendering and some tools, because line breaks inside fields can be misinterpreted or make diffs difficult to read.

In `csv-render`:

- Newlines inside fields are **allowed but discouraged**.
- If they are used, the field **must be quoted**.
- Most examples and tests intentionally avoid embedded newlines to keep behavior simple and predictable.

When in doubt, prefer keeping each record on a single physical line.

---

## 8. Encoding and BOM

RFC 4180 does not mandate a specific character encoding, but:

- UTF-8 is the de facto standard for modern systems.
- A Byte Order Mark (BOM) at the start of the file (`\uFEFF`) can confuse some parsers.

`csv-render` assumes:

- Files are encoded as **UTF-8 without a BOM**.
- If a BOM or other unexpected leading bytes are present, they may be treated as part of the first field, causing subtle errors.

To avoid these issues, ensure that editors and tools save CSV files as plain UTF-8 without BOM.

---

## 9. How GitHub interacts with RFC 4180

GitHub uses a built-in CSV viewer to render files as interactive tables when:

- The file is small enough (in terms of size and row count) to be rendered.
- The CSV is structurally valid and can be parsed reliably.

Common reasons GitHub falls back to showing raw text include:

- Unclosed quoted fields.
- Inconsistent column counts across rows.
- Extremely large files.

Because `csv-render` enforces RFC 4180 rules, the same problems that break parsers are also flagged by the validator. The aim is that any CSV accepted by `csv-core` will also be safe to render as a table on GitHub.

---

## 10. How `csv-core` enforces RFC 4180

The Rust `csv-core` library uses a CSV parser configured to behave in line with RFC 4180:

- Comma delimiter and double quote as the quote character.
- A required header row that is matched against the schema.
- Flexible mode disabled so that each record must have a consistent number of fields.

On top of that, `csv-core` adds:

- Schema-based checks for types (for example, parsing `u64` for `timestamp`).
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
2. The header row uses the exact column names and order defined in the schema (for example, `host_did, consent_ledger_refs, timestamp, neurorights_flags, notes`).
3. Every data row has the same number of fields as the header.
4. Any field containing a comma, double quote, or newline is fully enclosed in double quotes.
5. Any double quote inside a quoted field is escaped as `""`.
6. No row has a trailing comma.
7. Required fields are not empty.
8. Values conform to the expected types and allowed sets (for example, timestamps as integers, neurorights flags from the allowed list).

If all these checks pass, the CSV file should be RFC 4180-compliant, GitHub-ready, and able to pass the `csv-core` validator without errors.
