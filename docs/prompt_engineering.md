# Prompt Engineering for GitHub‑Ready CSVs

This document provides reusable prompt patterns for Large Language Models (LLMs) that must generate CSV files compatible with RFC 4180, GitHub’s renderer, and the `csv-core` validator in this repository.[web:6][web:69]

The core idea: **the model should think like the validator before writing any row.**

---

## 1. Core rules the model must follow

When generating CSV for this project, the model must treat these rules as non‑negotiable:[web:6][web:61]

1. Use a comma (`,`) as the only field delimiter.
2. Every row must have exactly the same number of fields as the header.
3. If a field contains a comma, a double quote, or a newline, enclose the entire field in double quotes.
4. If a double quote appears *inside* a quoted field, represent it as two double quotes (`""`).
5. Do not add trailing commas at the end of any row.
6. Use UTF‑8 text and avoid stray control characters.

These are distilled from RFC 4180 and the behavior of common CSV parsers.[web:6][web:9]

---

## 2. Governance particle schema (for context)

The `governance_particle` table is defined declaratively in `configs/governance-particle-schema.yaml`.[web:11] The key columns are:

- `host_did` (`String`, required)  
- `consent_ledger_refs` (`Vec<String>`, required, semicolon‑separated)  
- `timestamp` (`u64`, required; Unix seconds since epoch)  
- `neurorights_flags` (`NeurorightsFlags`, required; semicolon‑separated allowed flags)  
- `notes` (`String`, optional; may contain commas and must be quoted as needed)

Allowed neurorights flags are defined in `configs/neurorights-flags.toml`, such as:

- `cognitive_liberty`
- `mental_privacy`
- `mental_integrity`
- `psychological_continuity`
- `agency_preservation`[web:11]

When prompting an LLM, you can paste this schema snippet into the context to make the contract explicit.

---

## 3. Zero‑shot “validator mindset” prompt

Use this template when asking an LLM to generate governance particle CSVs from scratch.

> **System prompt (template)**
>
> You are a CSV generation assistant.  
> Your primary goal is to produce a CSV file that:
> - Strictly follows RFC 4180 CSV rules.  
> - Renders correctly as a table on GitHub.  
> - Passes validation by a Rust library called `csv-core`, which enforces a declarative schema.
>
> Schema (governance_particle):
>
> - Columns (in order):
>   1. `host_did` (String, required)
>   2. `consent_ledger_refs` (Vec<String>, required, semicolon `;` separator inside the field)
>   3. `timestamp` (u64, required, Unix seconds since epoch)
>   4. `neurorights_flags` (NeurorightsFlags, required, semicolon `;` separator inside the field)
>   5. `notes` (String, optional; may contain commas)
>
> - Allowed neurorights flags:
>   - `cognitive_liberty`
>   - `mental_privacy`
>   - `mental_integrity`
>   - `psychological_continuity`
>   - `agency_preservation`
>
> RFC 4180‑style structural rules:
> - Use `,` as the only column delimiter.
> - If a field contains a comma, double quote, or newline, enclose the entire field in double quotes.
> - If a double quote appears inside a quoted field, escape it as `""`.
> - Every row must have exactly 5 columns, matching the header row.

> **User prompt (template)**
>
> Generate a CSV for the `governance_particle` table with N records.  
> Follow this exact procedure before writing any output:
>
> 1. List the 5 column names in order.
> 2. For each column, state its type and constraints in one sentence.
> 3. For each record you will generate:
>    a. Propose values for each field in plain language.  
>    b. Check that:
>       - `timestamp` is a valid u64 (integer, no quotes, no decimals).  
>       - `consent_ledger_refs` is a semicolon‑separated list of URIs.  
>       - `neurorights_flags` is a semicolon‑separated list of allowed flags only.  
>       - `notes` is free‑text and will be quoted if it contains commas.
>    c. Convert the proposed values into a CSV row, applying RFC 4180 quoting rules.
> 4. After constructing all rows in your reasoning, output **only** the final CSV text, with:
>    - A single header row: `host_did,consent_ledger_refs,timestamp,neurorights_flags,notes`  
>    - Exactly N data rows.
>
> Do **not** include explanations in the final output, only the CSV.

This pattern encourages explicit “pre‑flight” reasoning (like a validator) while keeping the final output clean.[web:70][web:73]

---

## 4. Few‑shot examples based on this repository

You can improve reliability by showing the model a good and a bad example side‑by‑side and telling it to imitate the good one and avoid the bad one.[web:70]

### 4.1 Valid example (good)

Paste `rust/examples/governance_particle.csv` as a reference:

```csv
host_did,consent_ledger_refs,timestamp,neurorights_flags,notes
did:example:host1,"https://ledger.example/consent/abc123;https://ledger.example/consent/def456",1713825600,"cognitive_liberty;mental_privacy","Initial governance record, fully compliant."
did:example:host2,"https://ledger.example/consent/xyz789",1713829200,"mental_integrity;psychological_continuity","Second record with multiple neurorights flags."
did:example:host3,"https://ledger.example/consent/qwe111;https://ledger.example/consent/rty222",1713832800,"cognitive_liberty;agency_preservation","Record with two consent ledger references and neurorights flags."
did:example:host4,"https://ledger.example/consent/zzz000",1713836400,mental_privacy,"Notes field with an embedded, comma that is properly quoted."
```

Tell the model:

- “Your output must be structurally identical in style to this CSV.”  
- “You may change values, but **must not** change column order, delimiter, or quoting behavior.”

### 4.2 Invalid example (bad)

Paste `rust/tests/broken.csv` and label what is wrong:

```csv
host_did,consent_ledger_refs,timestamp,neurorights_flags,notes
did:example:host1,https://ledger.example/consent/abc123;https://ledger.example/consent/def456,not-a-timestamp,cognitive_liberty;mental_privacy,"This row has an invalid timestamp."
did:example:host2,https://ledger.example/consent/xyz789,1713829200,unknown_flag,"This row uses an invalid neurorights flag."
did:example:host3,https://ledger.example/consent/qwe111;https://ledger.example/consent/rty222,1713829300,mental_privacy,""
did:example:host4,https://ledger.example/consent/zzz000,1713829400,mental_privacy,"Unquoted, note with stray comma"
```

Then instruct:

- “Never output a non‑numeric `timestamp`.”  
- “Never use neurorights flags that are not in the allowed list.”  
- “Always close quoted fields on the same line.”

This gives the model clear negative patterns to avoid while still keeping the CSV structurally valid for testing.[web:6][web:9]

---

## 5. Post‑generation verification loop

For high‑stakes workflows, you can integrate a verifier loop in which the LLM calls out to `csv-cli` (or you do so externally) and then self‑corrects based on its output.

### 5.1 Prompting the model to interpret validator errors

When `csv-cli` returns an error message like:

```json
{"status":"error","message":"schema violation at row 2, column 2: value \"not-a-timestamp\" in column 'timestamp' could not be parsed as u64"}
```

You can prompt:

> The CSV you produced failed validation with this error:  
> `schema violation at row 2, column 2: value "not-a-timestamp" in column 'timestamp' could not be parsed as u64`  
> 
> 1. Explain in one sentence what is wrong.  
> 2. Produce a corrected CSV that fixes **only** this problem, preserving all other columns and rows.  
> 3. Apply RFC 4180 rules when quoting fields.

This pattern teaches the model to read validator feedback and apply minimal, targeted fixes instead of regenerating everything from scratch.[web:73]

---

## 6. Tips for using these prompts in practice

- Always pin the schema and allowed flags in the system or developer prompt so the model cannot “invent” columns or flags.
- Keep the final output channel “CSV‑only” to avoid accidental commentary breaking the file.
- When using tools like GitHub Actions to validate AI‑generated CSV, pair these prompts with the `csv-cli validate` step so failures are caught early in CI.

These templates are meant to evolve alongside `csv-core` and the schema files; as you add new columns or rules, update this document so humans and AI share the same understanding of what a valid CSV looks like.[web:6][web:11][web:69]
