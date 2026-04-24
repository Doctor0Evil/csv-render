use std::path::PathBuf;

use clap::{Parser, Subcommand};

use csv_core::{validate_csv_with_schema, Schema, ValidationError, ValidationOptions};

/// csv-cli
///
/// Command-line interface for the csv-core validation library.
#[derive(Debug, Parser)]
#[command(name = "csv-cli")]
#[command(about = "CSV validation and linting for the csv-render project")]
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

        /// Maximum number of data rows to validate (0 = all).
        #[arg(long, default_value_t = 0)]
        max_rows: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Validate {
            schema,
            csv_path,
            max_rows,
        } => {
            let schema_loaded = Schema::from_yaml_file(schema);
            match schema_loaded {
                Ok(schema_value) => {
                    let options = if max_rows > 0 {
                        ValidationOptions {
                            max_rows: Some(max_rows),
                        }
                    } else {
                        ValidationOptions::default()
                    };

                    validate_csv_with_schema(csv_path, &schema_value, &options)
                }
                Err(e) => Err(ValidationError::Schema(e)),
            }
        }
    };

    match result {
        Ok(()) => {
            println!(r#"{{"status":"ok"}}"#);
            std::process::exit(0);
        }
        Err(err) => {
            eprintln!(r#"{{"status":"error","message":"{err}"}}"#);
            std::process::exit(1);
        }
    }
}
