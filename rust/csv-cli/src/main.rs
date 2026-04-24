use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};

use csv_core::{validate_csv_with_schema, Schema, ValidationError, ValidationOptions};

mod json_report;
mod scan;

use json_report::build_error_report;

fn emit_json_report(
    csv_path: &str,
    schema_id: Option<&str>,
    total_rows: usize,
    errors: Vec<(ValidationError, Option<String>)>,
) -> std::io::Result<()> {
    let report = build_error_report(csv_path, schema_id, total_rows, errors);
    json_report::write_error_report_json(&report, std::io::stdout())
}

#[derive(Debug, Parser)]
#[command(name = "csv-cli")]
#[command(about = "CSV validation, linting, and repo scanning for the csv-render project")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Validate a CSV file against a schema.
    Validate {
        /// Path to the YAML schema file.
        #[arg(long)]
        schema: PathBuf,

        /// Path to the CSV file to validate.
        csv_path: PathBuf,

        /// Optional limit on the number of rows to validate (0 = no limit).
        #[arg(long, default_value_t = 0)]
        max_rows: usize,

        /// Emit JSON report instead of plain text.
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
    },

    /// Scan the repository for unregistered/duplicate CSVs using configs/repo-index.yaml.
    Scan {
        /// Root directory of the repository (defaults to current directory).
        #[arg(long, default_value = ".")]
        repo_root: PathBuf,

        /// Path to the repo index configuration.
        #[arg(long, default_value = "configs/repo-index.yaml")]
        index: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Commands::Validate {
            schema,
            csv_path,
            max_rows,
            json,
        } => run_validate(schema, csv_path, max_rows, json),
        Commands::Scan { repo_root, index } => run_scan(repo_root, index),
    };

    std::process::exit(exit_code);
}

fn run_scan(repo_root: PathBuf, index_path: PathBuf) -> i32 {
    scan::run_scan_command(&repo_root, &index_path)
}

fn run_validate(
    schema_path: PathBuf,
    csv_path: PathBuf,
    max_rows: usize,
    json: bool,
) -> i32 {
    let schema_loaded = Schema::from_yaml_file(&schema_path);

    let schema = match schema_loaded {
        Ok(schema_value) => schema_value,
        Err(e) => {
            let err = ValidationError::Schema {
                code: "SCHEMA_LOAD_FAILED",
                message: format!("failed to load schema: {}", e),
                source: Some(e),
            };

            if json {
                if let Err(io_err) = emit_json_report(
                    &csv_path.to_string_lossy(),
                    None,
                    0,
                    vec![(err, None)],
                ) {
                    eprintln!("failed to write JSON error report: {}", io_err);
                }
            } else {
                eprintln!("schema error: {}", err);
            }

            return 1;
        }
    };

    let options = if max_rows > 0 {
        ValidationOptions {
            max_rows: Some(max_rows),
        }
    } else {
        ValidationOptions::default()
    };

    let validation_result = validate_csv_with_schema(&csv_path, &schema, &options);

    match validation_result {
        Ok(summary) => {
            if json {
                if let Err(io_err) = emit_json_report(
                    &csv_path.to_string_lossy(),
                    schema.schema_id.as_deref(),
                    summary.total_rows,
                    Vec::new(),
                ) {
                    eprintln!("failed to write JSON report: {}", io_err);
                    return 1;
                }
            } else {
                println!(r#"{{"status":"ok","total_rows":{}}}"#, summary.total_rows);
            }
            0
        }
        Err(errors) => {
            if json {
                let items: Vec<(ValidationError, Option<String>)> =
                    errors.items.into_iter().map(|e| (e, None)).collect();
                if let Err(io_err) = emit_json_report(
                    &csv_path.to_string_lossy(),
                    schema.schema_id.as_deref(),
                    errors.total_rows,
                    items,
                ) {
                    eprintln!("failed to write JSON error report: {}", io_err);
                    return 1;
                }
            } else {
                for err in errors.items {
                    eprintln!("error: {}", err);
                }
            }
            1
        }
    }
}
