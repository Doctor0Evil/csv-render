# csv-render

`csv-render` is a lab-grade toolkit for generating, validating, and rendering **GitHub-ready** CSV files. It combines a Rust validation engine, Python utilities, and declarative schemas to enforce RFC 4180 rules and project-specific data contracts. [docs](https://docs.rs/csv)

The core idea: every CSV in this repository must be structurally valid (RFC 4180), semantically correct (schema-validated), and small and clean enough to render as interactive tables on GitHub. [inventivehq](https://inventivehq.com/blog/handling-special-characters-in-csv-files)

## Features

This project enforces a strict CSV discipline using three cooperating layers: a Rust workspace, Python tools, and configuration files. [theproductguy](https://theproductguy.in/blogs/csv-with-commas-quotes)

The **Rust layer** lives in `rust/` and provides:

- `csv-core`: a library that validates CSVs against RFC 4180 and declarative schemas using Rust’s `csv` crate and strong typing. [docs](https://docs.rs/csv)
- `csv-cli`: a command-line tool that exposes validation and conversion features, designed to be called from CI and from AI workflows. [gist.github](https://gist.github.com/domnikl/ccb8d0b82056fbe5cf7f4f145ac7f44b)

The **Python layer** in `python/` offers:

- A validation script for quick checks and integrations with existing Python-based pipelines.
- A converter from CSV to Markdown to produce GitHub-friendly tables when needed.

The **configuration layer** in `configs/` holds YAML and TOML files describing schemas and enumerations. These declarative files are the single source of truth for column names, types, constraints, and domain-specific rules. [docs](https://docs.rs/csv)

### RFC 4180 and GitHub-readiness

`csv-render` treats RFC 4180 as a hard requirement, including:

- Quoting fields containing commas, double quotes, or newlines. [inventivehq](https://inventivehq.com/blog/handling-special-characters-in-csv-files)
- Escaping internal double quotes by doubling them (`""`). [theproductguy](https://theproductguy.in/blogs/csv-with-commas-quotes)
- Using a consistent delimiter (comma) and consistent column counts across all rows. [docs](https://docs.rs/csv)

On top of this, the project is tuned for GitHub’s renderer by:

- Normalizing encoding to UTF‑8.
- Keeping files and examples small enough to render as tables in the GitHub UI. [github](https://github.com/orgs/community/discussions/75164)

## Repository layout

The repository is organized around a polyglot toolchain with Rust at the core and Python for scripting. [docs](https://docs.rs/csv)

```text
csv-render/
├── .github/
│   └── workflows/
│       ├── csv-lint.yml              # Python-based CSV validation CI
│       └── rust-ci.yml               # Rust: fmt, clippy, tests, CSV validation
│
├── python/
│   ├── src/
│   │   └── csv_validator.py          # Python CSV validation script
│   ├── converters/
│   │   └── csv_to_markdown.py        # CSV → Markdown table converter
│   └── requirements.txt              # Python dependencies
│
├── rust/
│   ├── Cargo.toml                    # Workspace manifest
│   ├── rustfmt.toml                  # Rust formatting config
│   ├── clippy.toml                   # Rust lint configuration
│   ├── csv-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                # Public API and module wiring
│   │       ├── error.rs              # Error and diagnostics types
│   │       ├── validator.rs          # RFC 4180 and structural checks
│   │       └── schema.rs             # Schema loading and type validation
│   ├── csv-cli/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── main.rs               # CLI entry point (clap)
│   ├── tests/
│   │   ├── valid.csv                 # Known-good CSV
│   │   └── broken.csv                # Known-bad CSV
│   └── examples/
│       └── governance_particle.csv   # Governance particle dataset
│
├── configs/
│   ├── governance-particle-schema.yaml  # Schema for governance_particle.csv
│   └── neurorights-flags.toml          # Enum-like configuration for flags
│
├── docs/
│   ├── prompt_engineering.md        # LLM prompting strategies
│   ├── rfc4180_guide.md             # CSV rules and best practices
│   └── github_rendering_tips.md     # GitHub-specific CSV guidance
│
├── examples/
│   ├── valid_example.csv            # General valid CSV example
│   └── broken_example.csv           # General invalid CSV example
│
├── README.md
└── .gitignore
```

This layout lets Rust, Python, and AI tooling all read the same schemas and examples while keeping concerns separated and discoverable. [gist.github](https://gist.github.com/domnikl/ccb8d0b82056fbe5cf7f4f145ac7f44b)

## Rust workspace

The Rust workspace under `rust/` ties everything together using the `csv` crate and standard Rust tooling. [docs](https://docs.rs/csv)

- `csv-core` is a library crate that performs robust CSV validation:
  - Streams rows using `csv::Reader`.
  - Checks encoding, delimiter, RFC 4180 quoting and escaping rules, and column counts. [theproductguy](https://theproductguy.in/blogs/csv-with-commas-quotes)
  - Applies schema-based validations by loading YAML/TOML from `configs/`.

- `csv-cli` is a binary crate that:
  - Accepts commands like `validate` and `lint`.
  - Takes `--schema` and input paths.
  - Can emit machine-readable JSON diagnostics for downstream automation.

Both crates share configuration via `rustfmt.toml` and `clippy.toml`, and are subject to the same CI rules. [github](https://github.com/orgs/community/discussions/63210)

### Running the Rust tools

From the `rust/` directory:

```bash
cargo test
cargo run -p csv-cli -- validate \
  --schema ../configs/governance-particle-schema.yaml \
  ../rust/examples/governance_particle.csv
```

These commands ensure that the validator is correct and that the governance particle CSV complies with the declared schema. [docs](https://docs.rs/csv)

## Python tools

The Python tooling under `python/` offers a lightweight complement to the Rust core. [docs](https://docs.rs/csv)

- `src/csv_validator.py` provides scripting-friendly CSV checks, which can be extended to call into the Rust CLI or re-use the same schema files.
- `converters/csv_to_markdown.py` turns valid CSV files into GitHub-ready Markdown tables, useful for documentation and visual inspection.

Install dependencies with:

```bash
pip install -r python/requirements.txt
```

The `csv-lint.yml` GitHub Actions workflow uses these scripts to run checks in CI for teams or tools that already rely on Python. [gist.github](https://gist.github.com/domnikl/ccb8d0b82056fbe5cf7f4f145ac7f44b)

## Declarative schemas and neurorights configuration

The `configs/` directory contains declarative files that define the project’s data contracts. [docs](https://docs.rs/csv)

- `governance-particle-schema.yaml` specifies:
  - Column names.
  - Expected types (e.g., string, integer, list types).
  - Required/optional fields.
  - Domain-specific rules like separators for list fields.

- `neurorights-flags.toml` lists the allowed flags and options for neurorights information, making it easy to extend or tighten those definitions without changing code.

Both Rust and Python tools can load these files to validate CSVs and to inform AI prompts about the exact structure that must be produced. [docs](https://docs.rs/csv)

## Documentation and AI prompt engineering

The `docs/` directory contains materials meant for both humans and AI systems.

- `rfc4180_guide.md` explains the core CSV rules, including field quoting, escaping, and column count consistency. [inventivehq](https://inventivehq.com/blog/handling-special-characters-in-csv-files)
- `github_rendering_tips.md` documents GitHub-specific behaviors and limits relevant to CSVs.
- `prompt_engineering.md` provides concrete prompt templates for LLMs, including:
  - Zero-shot instructions for generating RFC 4180-compliant CSV output.
  - Step-by-step “act like a validator” checklists that mirror `csv-core` logic.
  - Examples of valid and invalid CSVs and how they differ.

These documents are designed to be directly copy-pasteable into AI system prompts and to align with the behavior actually enforced by the Rust validator. [theproductguy](https://theproductguy.in/blogs/csv-with-commas-quotes)

## Continuous Integration

GitHub Actions workflows in `.github/workflows/` keep the repository consistent and trustworthy. [shift](https://shift.click/blog/github-actions-rust/)

- `rust-ci.yml`:
  - Runs `cargo fmt --all -- --check` to enforce formatting. [github](https://github.com/orgs/community/discussions/63210)
  - Runs `cargo clippy --all --all-features --tests -- -D warnings` to enforce idiomatic Rust. [github](https://github.com/marketplace/actions/rust-clippy-check)
  - Runs `cargo test --all-features` to execute unit and integration tests. [gist.github](https://gist.github.com/domnikl/ccb8d0b82056fbe5cf7f4f145ac7f44b)
  - Invokes `csv-cli validate` against curated CSV files and schema configurations.

- `csv-lint.yml`:
  - Sets up Python.
  - Installs dependencies from `python/requirements.txt`.
  - Runs Python-based validators and converters as needed.

This CI setup ensures that any change—whether authored by humans or AI—must pass the same strict checks before merging. [github](https://github.com/orgs/community/discussions/63210)

## Getting started

To start using `csv-render`:

1. Clone the repository and install Rust and Python toolchains.
2. Explore the schemas in `configs/` and the example CSVs in `rust/examples/` and `examples/`.
3. Run the Rust validator against your CSVs using `csv-cli`, or integrate the Python validator into existing scripts.
4. Use the prompt templates in `docs/prompt_engineering.md` to guide AI systems to generate CSVs that pass validation on the first try.

This README is the top-level map for the project; the next steps will populate each file and crate in the structure with concrete implementations and documentation.
