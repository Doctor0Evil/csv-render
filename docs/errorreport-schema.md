# error_report.json Schema

This document defines the stable JSON format produced by `csv-cli validate --json`. It is the primary contract between the Rust validator, CI/tooling, and AI-based repair loops.

The schema is versioned and designed to be:

- Human-readable for debugging.
- Machine-readable for DuckDB/Python wrappers.
- Prompt-ready for LLM-based repair workflows.

---

## Top-level structure

A validation report is a single JSON object with the following shape:

```json
{
  "version": 1,
  "file": "examples/governance_particle.csv",
  "schema_id": "governance-particle-v1",
  "summary": {
    "total_rows": 250,
    "errors": 3,
    "categories": {
      "lexical": 0,
      "structural": 1,
      "semantic": 2,
      "relational": 0,
      "io": 0
    }
  },
  "errors": [
    {
      "row": 42,
      "column": 3,
      "column_name": "timestamp",
      "category": "semantic",
      "code": "T2_TIME_MONOTONICITY_VIOLATION",
      "message": "timestamp 1690000000 violates non_decreasing ordering (previous 1690000005)",
      "details": {
        "value": 1690000000,
        "previous": 1690000005,
        "monotonicity": "non_decreasing"
      }
    }
  ]
}
```

### Fields

- `version` (integer)  
  A monotonically increasing version number for the error report format. Breaking changes must increment this value.

- `file` (string)  
  Path or identifier of the CSV file that was validated.

- `schema_id` (string, optional)  
  Logical identifier of the schema used, e.g. `"governance-particle-v1"`.

- `summary` (object)  
  - `total_rows` (integer): number of data rows (excluding header) processed.
  - `errors` (integer): total number of error entries emitted.
  - `categories` (object): per-category counts keyed by category name.

- `errors` (array)  
  A list of error objects as described below.

---

## Error object

Each error entry has the following structure:

```json
{
  "row": 42,
  "column": 3,
  "column_name": "timestamp",
  "category": "semantic",
  "code": "T2_TIME_MONOTONICITY_VIOLATION",
  "message": "timestamp 1690000000 violates non_decreasing ordering (previous 1690000005)",
  "details": {
    "value": 1690000000,
    "previous": 1690000005,
    "monotonicity": "non_decreasing"
  }
}
```

### Common fields

- `row` (integer, 1-based)  
  Data row index where the error occurred. Header row is not counted.

- `column` (integer, 1-based)  
  Column index where the error occurred.

- `column_name` (string)  
  Name of the column at position `column`, as defined in the header.

- `category` (string)  
  High-level error category:
  - `"lexical"`: encoding, BOM, newline issues.
  - `"structural"`: RFC 4180 violations, column counts, quoting.
  - `"semantic"`: Tier 2 constraints (time, flags, value-level rules).
  - `"relational"`: cross-file foreign key or relational constraints.
  - `"io"`: atomic write, filesystem, or other I/O invariants.

- `code` (string)  
  Stable, machine-readable error code. See the sections below for reserved values.

- `message` (string)  
  Human-readable explanation of the error.

- `details` (object)  
  Arbitrary JSON object containing structured fields that depend on the error code. This is where additional context required for AI repair is stored.

---

## Tier 2 error codes

The following Tier 2 codes are reserved and must be emitted consistently when the corresponding conditions are violated.

### Time semantics

- `T2_TIME_PARSE`  
  Failed to parse a timestamp value as the expected numeric type.

  `details` fields:
  - `value` (string): original raw value.

- `T2_TIME_RANGE_VIOLATION`  
  Timestamp is outside the configured `[min_epoch, max_epoch]` interval.

  `details` fields:
  - `value` (integer): parsed timestamp.
  - `min_epoch` (integer).
  - `max_epoch` (integer).

- `T2_TIME_MONOTONICITY_VIOLATION`  
  Timestamp violates the configured monotonicity (non-decreasing, strictly increasing, or per-group).

  `details` fields:
  - `value` (integer): current timestamp.
  - `previous` (integer): previous timestamp in sequence or group.
  - `monotonicity` (string): one of `"none"`, `"non_decreasing"`, `"strict_increasing"`, `"per_group"`.

### Neurorights flags and compositional enums

- `T2_FLAG_UNKNOWN`  
  A flag is not listed in the allowed flag registry.

  `details` fields:
  - `flag` (string): offending flag.
  - `allowed_contract` (string): name of the active flag contract.

- `T2_FLAG_MUTUALLY_EXCLUSIVE`  
  A pair of flags that must not co-occur appears in the same record.

  `details` fields:
  - `a` (string): first flag.
  - `b` (string): second flag.
  - `allowed_contract` (string).

- `T2_FLAG_MISSING_IMPLICATION`  
  A required implication rule is not satisfied: flag A implies flag B.

  `details` fields:
  - `if` (string): required antecedent flag.
  - `then` (string): required consequent flag.
  - `allowed_contract` (string).

- `T2_FLAG_CARDINALITY_MIN`  
  The number of flags in the set is less than the configured minimum.

  `details` fields:
  - `count` (integer): number of flags present.
  - `min` (integer).
  - `allowed_contract` (string).

- `T2_FLAG_CARDINALITY_MAX`  
  The number of flags in the set is greater than the configured maximum.

  `details` fields:
  - `count` (integer).
  - `max` (integer).
  - `allowed_contract` (string).

### Cross-file relations

- `T2_RELATION_MISSING_FK`  
  Foreign key value does not exist in the referenced primary key set.

  `details` fields:
  - `relation_name` (string).
  - `foreign_value` (string).
  - `from_file` (string).
  - `from_column` (string).
  - `to_file` (string).
  - `to_column` (string).

---

## IO and Tier 1 error codes

For completeness, IO-related error codes follow a similar pattern:

- `IO_CREATE_TEMP_FAILED`
- `IO_WRITE_TEMP_FAILED`
- `IO_FSYNC_TEMP_FAILED`
- `IO_RENAME_FAILED`
- `IO_OPEN_DIR_FAILED`
- `IO_FSYNC_DIR_FAILED`

Each IO error includes:

- `path` (string): path of the file or directory.
- `operation` (string): failed operation name, e.g. `"fsync"`, `"rename"`.

---

## Usage in AI repair flows

The `error_report.json` generated by `csv-cli` is the primary input for AI-based repair prompts. A repair prompt should:

1. Provide the `broken.csv` content.
2. Provide the `error_report.json` object.
3. Instruct the model to:
   - Interpret `code` and `details` as the definition of what needs to be fixed.
   - Produce a corrected CSV that eliminates all reported errors without changing the schema.

Because the error format is stable and versioned, training data built from `(broken.csv, error_report.json, fixed.csv)` triplets remains valid even as internal implementation details evolve.
