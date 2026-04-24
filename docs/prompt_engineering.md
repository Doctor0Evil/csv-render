# Prompt Engineering for GitHub‑Ready, Validator‑Safe CSVs

This document defines reusable prompt patterns for Large Language Models (LLMs) that must generate, validate, and repair CSV files compatible with RFC 4180, GitHub’s renderer, and the `csv-core`/`csv-cli` toolchain in this repository.

The core idea: the model should think like the validator before writing any row.

---

## 1. Non‑negotiable CSV rules

When generating CSV for this project, the model must treat these rules as hard constraints:

1. Use a comma (`,`) as the only field delimiter.
2. Every row must have exactly the same number of fields as the header.
3. If a field contains a comma, a double quote, or a newline, enclose the entire field in double quotes.
4. If a double quote appears inside a quoted field, represent it as two double quotes (`""`).
5. Do not add trailing commas at the end of any row.
6. Use UTF‑8 text and avoid stray control characters.

These rules match RFC 4180 and the strict profile enforced by `csv-core`.

---

## 2. Checklist + CSV Generation Pattern

This pattern wires Tier 1 and Tier 2 invariants directly into the prompt: the model first restates the schema and constraints, then emits CSV.

### 2.1. Inputs

- A schema summary JSON produced by `csv-cli describe-schema`.
- The desired number of rows and any additional domain constraints.

### 2.2. System prompt template

You are a CSV generator that must strictly follow a given schema and validation rules.

You will receive:

1. A JSON object that summarizes a CSV schema (columns, types, constraints, relations).
2. A description of the data to generate.

You must:

- First, produce a checklist that restates the schema and constraints in your own words.
- Only after the checklist is complete and correct, produce the CSV body with no commentary.

The CSV must:

- Use a comma as the only delimiter.
- Include a single header row that exactly matches the column names defined in the schema.
- Quote any field that contains a comma, double quote, or newline, and escape internal quotes by doubling them.

Do not invent new columns or change the order of columns.

### 2.3. User prompt template

Schema summary:

```json
{{SCHEMA_JSON}}
```

Task:

Generate {{ROW_COUNT}} rows of data that satisfy this schema.

Step 1: Checklist

1. List all columns in order with their types and whether they are required.
2. For each timestamp column, state:
   - The unit (seconds, milliseconds, etc.).
   - The allowed `[min_epoch, max_epoch]` range.
   - The monotonicity requirement (`none`, `non_decreasing`, `strict_increasing`, `per_group`).
3. For each flags column, state:
   - The separator character.
   - The set of allowed flags.
   - Any mutual exclusivity rules.
   - Any implication rules.
   - Minimum and maximum number of flags allowed per row.
4. For each relation, describe:
   - The source file and column.
   - The target file and column.
   - The meaning of the foreign key constraint.

Wait for confirmation that your checklist is correct.

Step 2: CSV output

After the checklist is confirmed, output only the CSV body, with a single header row and exactly {{ROW_COUNT}} data rows.

---

## 3. Error‑Driven Repair Pattern

This pattern uses `error_report.json` from `csv-cli validate --json` as ground truth for fixing broken CSVs.

### 3.1. Inputs

- `broken.csv`: a CSV file that failed validation.
- `error_report.json`: machine‑readable diagnostics from `csv-cli validate --json` for that file.

### 3.2. System prompt template

You are a CSV repair agent that must correct CSV files until they pass a strict validator.

You will receive:

1. The content of a CSV file (`broken.csv`).
2. A JSON object (`error_report.json`) describing validation errors produced by a strict validator.

Your task:

- Interpret each error's `code`, `category`, and `details`.
- Modify the CSV minimally so that all reported errors are resolved.
- Do not change:
  - The set of columns.
  - The column order.
  - The delimiter (must remain a comma).

You must preserve the header row and output a corrected CSV (`fixed.csv`) with the same structure.

If multiple corrections are possible, choose the smallest change that could realistically occur in a human or system workflow.

### 3.3. User prompt template

Here is the broken CSV:

```text
{{BROKEN_CSV}}
```

Here is the error report JSON:

```json
{{ERROR_REPORT_JSON}}
```

Instructions:

1. Read all errors and group them by `code`.
2. For time‑related errors:
   - For `T2_TIME_RANGE_VIOLATION`, adjust timestamps into the allowed `[min_epoch, max_epoch]` range while preserving ordering if possible.
   - For `T2_TIME_MONOTONICITY_VIOLATION`, reorder or adjust timestamps minimally to satisfy the monotonicity rule.
3. For neurorights flag errors:
   - For `T2_FLAG_UNKNOWN`, replace unknown flags with the closest valid alternative or remove them if no reasonable mapping exists.
   - For `T2_FLAG_MUTUALLY_EXCLUSIVE`, remove or adjust one of the offending flags.
   - For `T2_FLAG_MISSING_IMPLICATION`, add the required consequent flag when safe.
   - For `T2_FLAG_CARDINALITY_MIN` and `T2_FLAG_CARDINALITY_MAX`, add or remove flags to satisfy `min`/`max`.
4. For relational errors:
   - For `T2_RELATION_MISSING_FK`, fix or drop rows that reference non‑existent foreign keys. Prefer to correct obvious typos; if no fix is apparent, drop the row.

Output only the corrected CSV (`fixed.csv`), with no explanation or commentary.

---

## 4. Governance particle schema context

The `governance_particle` table is defined declaratively in `configs/governance-particle-schema.yaml`. The key columns are:

- `host_did` (`String`, required)
- `consent_ledger_refs` (`Vec<String>`, required, semicolon‑separated inside the field)
- `timestamp` (`u64`, required; Unix seconds since epoch, with configured range and monotonicity)
- `neurorights_flags` (`NeurorightsFlags`, required; semicolon‑separated allowed flags)
- `notes` (`String`, optional; may contain commas and must be quoted as needed)

Allowed neurorights flags are defined in `configs/neurorights-flags.toml`, such as:

- `cognitive_liberty`
- `mental_privacy`
- `mental_integrity`
- `psychological_continuity`
- `agency_preservation`

When prompting an LLM, you can paste either:

- A human‑readable summary like the above, or
- The exact JSON emitted by `csv-cli describe-schema`,

into the context to make the contract explicit.

---

## 5. Zero‑shot “validator mindset” prompt for governance_particle

Use this template when asking an LLM to generate governance particle CSVs from scratch.

### 5.1 System prompt (template)

You are a CSV generation assistant.  
Your primary goal is to produce a CSV file that:

- Strictly follows RFC 4180 CSV rules.
- Renders correctly as a table on GitHub.
- Passes validation by a Rust library called `csv-core`, which enforces a declarative schema.

Schema (governance_particle):

- Columns (in order):
  1. `host_did` (String, required)
  2. `consent_ledger_refs` (Vec<String>, required, semicolon `;` separator inside the field)
  3. `timestamp` (u64, required, Unix seconds since epoch, with a configured valid range)
  4. `neurorights_flags` (NeurorightsFlags, required, semicolon `;` separator inside the field)
  5. `notes` (String, optional; may contain commas)

- Allowed neurorights flags:
  - `cognitive_liberty`
  - `mental_privacy`
  - `mental_integrity`
  - `psychological_continuity`
  - `agency_preservation`

RFC 4180‑style structural rules:

- Use `,` as the only column delimiter.
- If a field contains a comma, double quote, or newline, enclose the entire field in double quotes.
- If a double quote appears inside a quoted field, escape it as `""`.
- Every row must have exactly 5 columns, matching the header row.

### 5.2 User prompt (template)

Generate a CSV for the `governance_particle` table with N records.  
Follow this exact procedure before writing any output:

1. List the 5 column names in order.
2. For each column, state its type and constraints in one sentence.
3. For each record you will generate:
   a. Propose values for each field in plain language.  
   b. Check that:
      - `timestamp` is a valid u64 (integer, no quotes, no decimals, within the configured epoch range).  
      - `consent_ledger_refs` is a semicolon‑separated list of URIs inside a single field.  
      - `neurorights_flags` is a semicolon‑separated list of allowed flags only.  
      - `notes` is free text and will be quoted if it contains commas.
   c. Convert the proposed values into a CSV row, applying RFC 4180 quoting rules.
4. After constructing all rows in your reasoning, output only the final CSV text, with:
   - A single header row: `host_did,consent_ledger_refs,timestamp,neurorights_flags,notes`
   - Exactly N data rows.

Do not include explanations in the final output, only the CSV.

---

## 6. Few‑shot examples

Few‑shot examples help anchor both structure and semantics.

### 6.1 Valid example (good)

Use a known‑good file such as `examples/governance_particle.csv` as a reference:

```csv
host_did,consent_ledger_refs,timestamp,neurorights_flags,notes
did:example:host1,"https://ledger.example/consent/abc123;https://ledger.example/consent/def456",1713825600,"cognitive_liberty;mental_privacy","Initial governance record, fully compliant."
did:example:host2,"https://ledger.example/consent/xyz789",1713829200,"mental_integrity;psychological_continuity","Second record with multiple neurorights flags."
did:example:host3,"https://ledger.example/consent/qwe111;https://ledger.example/consent/rty222",1713832800,"cognitive_liberty;agency_preservation","Record with two consent ledger references and neurorights flags."
did:example:host4,"https://ledger.example/consent/zzz000",1713836400,"mental_privacy","Notes field with an embedded, comma that is properly quoted."
```

Tell the model:

- Your output must be structurally identical in style to this CSV.
- You may change values, but must not change column order, delimiter, or quoting behavior.

### 6.2 Invalid example (bad)

Use a deliberately broken file such as `rust/tests/broken.csv` and label what is wrong:

```csv
host_did,consent_ledger_refs,timestamp,neurorights_flags,notes
did:example:host1,https://ledger.example/consent/abc123;https://ledger.example/consent/def456,not-a-timestamp,"cognitive_liberty;mental_privacy","This row has an invalid timestamp."
did:example:host2,https://ledger.example/consent/xyz789,1713829200,"unknown_flag","This row uses an invalid neurorights flag."
did:example:host3,https://ledger.example/consent/qwe111;https://ledger.example/consent/rty222,1713829300,"mental_privacy",""
did:example:host4,https://ledger.example/consent/zzz000,1713829400,mental_privacy,"Unquoted, note with stray comma"
```

Then instruct the model:

- Never output a non‑numeric `timestamp`.
- Never use neurorights flags that are not in the allowed list.
- Always quote fields that contain commas, double quotes, or newlines.
- Always close quoted fields on the same line.

This provides clear negative patterns to avoid while keeping the CSV structurally close to valid inputs for testing and repair.

---

## 7. Post‑generation verification and repair loop

For high‑stakes workflows, use an external loop that couples prompts with the real validator.

### 7.1. External control loop

1. Ask the model to generate CSV using the checklist pattern.
2. Save the CSV to `candidate.csv`.
3. Run `csv-cli validate --schema configs/governance-particle-schema.yaml --json candidate.csv > error_report.json`.
4. If the process exits successfully and `error_report.json` has `summary.errors == 0`, accept the file.
5. Otherwise, feed `candidate.csv` and `error_report.json` into the repair prompt pattern to produce `fixed.csv`.
6. Repeat validation on `fixed.csv` until no errors remain or a maximum number of iterations is reached.

### 7.2. Prompt to interpret validator errors

When `csv-cli` returns errors in `error_report.json`, you can prompt:

The CSV you produced failed validation with the following error report:

```json
{{ERROR_REPORT_JSON}}
```

1. Explain in one or two sentences what each error `code` means.
2. Produce a corrected CSV that fixes all reported problems while preserving:
   - The header row.
   - The set and order of columns.
   - The comma delimiter.

Output only the corrected CSV.

---

## 8. Practical tips

- Always pin schema and allowed flags in the system or developer prompt so the model cannot invent columns or flags.
- Keep the final output channel “CSV‑only” to avoid commentary breaking the file.
- In CI (GitHub Actions), pair these prompts with a `csv-cli validate --json` step so AI‑generated CSVs are mechanically checked before merge.
- When you extend schemas or add new Tier 2 contracts (time, flags, relations), update this document and the prompt templates so humans and AI share the same definition of “valid CSV” in this repository.
