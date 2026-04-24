use std::fs::File;
use std::path::{Path, PathBuf};

use csv::ReaderBuilder;

use crate::error::ValidationError;
use crate::schema::{NeurorightsConfig, Schema};

#[derive(Debug, Clone)]
pub struct ValidationOptions {
    pub max_rows: Option<usize>,
    pub neurorights_flags_path: Option<PathBuf>,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            max_rows: None,
            neurorights_flags_path: None,
        }
    }
}

pub fn validate_csv_with_schema<P: AsRef<Path>>(
    csv_path: P,
    schema: &Schema,
    options: &ValidationOptions,
) -> Result<(), ValidationError> {
    let file = File::open(csv_path.as_ref())?;
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .delimiter(b',')
        .flexible(false)
        .from_reader(file);

    let neurorights_allowed = if let Some(ref path) = options.neurorights_flags_path {
        let cfg = NeurorightsConfig::from_toml_file(path)?;
        Some(cfg.allowed_flags())
    } else {
        None
    };

    let headers = reader.headers().map_err(|err| ValidationError::Structural {
        row: 0,
        column: 0,
        message: format!("failed to read header row: {err}"),
    })?;

    if headers.len() != schema.columns.len() {
        return Err(ValidationError::Structural {
            row: 0,
            column: 0,
            message: format!(
                "header column count mismatch: csv={:?}, schema={:?}",
                headers.len(),
                schema.columns.len()
            ),
        });
    }

    for (idx, col) in schema.columns.iter().enumerate() {
        let header_name = headers.get(idx).unwrap_or_default();
        if header_name != col.name {
            return Err(ValidationError::Structural {
                row: 0,
                column: idx,
                message: format!(
                    "header name mismatch at index {idx}: csv={:?}, schema={:?}",
                    header_name, col.name
                ),
            });
        }
    }

    for (row_idx, result) in reader.records().enumerate() {
        if let Some(limit) = options.max_rows {
            if row_idx >= limit {
                break;
            }
        }

        let row_number = row_idx + 1;
        let record = result.map_err(|err| ValidationError::Structural {
            row: row_number,
            column: 0,
            message: format!("failed to read record: {err}"),
        })?;

        if record.len() != schema.columns.len() {
            return Err(ValidationError::Structural {
                row: row_number,
                column: 0,
                message: format!(
                    "column count mismatch at row {row_number}: csv={:?}, schema={:?}",
                    record.len(),
                    schema.columns.len()
                ),
            });
        }

        for (idx, col) in schema.columns.iter().enumerate() {
            let value = record.get(idx).unwrap_or_default();
            let value_trimmed = value.trim();

            if value_trimmed.is_empty() {
                if col.required {
                    return Err(ValidationError::Semantic {
                        row: row_number,
                        column: idx,
                        message: format!(
                            "required field '{}' is empty at row {}",
                            col.name, row_number
                        ),
                    });
                }
                continue;
            }

            match col.ty.as_str() {
                "String" => {}
                "u64" => {
                    if value_trimmed.parse::<u64>().is_err() {
                        return Err(ValidationError::Semantic {
                            row: row_number,
                            column: idx,
                            message: format!(
                                "value {:?} in column '{}' could not be parsed as u64",
                                value_trimmed, col.name
                            ),
                        });
                    }
                }
                "Vec<String>" => {
                    if let Some(sep) = &col.separator {
                        let _items: Vec<&str> = value_trimmed.split(sep).collect();
                    }
                }
                "NeurorightsFlags" => {
                    if let Some(ref allowed) = neurorights_allowed {
                        for token in value_trimmed.split(';') {
                            let t = token.trim();
                            if t.is_empty() {
                                continue;
                            }
                            if !allowed.contains(t) {
                                return Err(ValidationError::Semantic {
                                    row: row_number,
                                    column: idx,
                                    message: format!(
                                        "neurorights flag {:?} is not allowed at row {}",
                                        t, row_number
                                    ),
                                });
                            }
                        }
                    }
                }
                other => {
                    return Err(ValidationError::Semantic {
                        row: row_number,
                        column: idx,
                        message: format!(
                            "unsupported column type {:?} for column '{}' in schema",
                            other, col.name
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}
