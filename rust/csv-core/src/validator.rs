use std::fs::File;
use std::path::Path;

use csv::ReaderBuilder;

use crate::error::ValidationError;
use crate::schema::Schema;

/// Options controlling how validation should behave.
///
/// This struct is designed to be extended with flags (e.g., max_rows, strict
/// newline policies) without breaking callers.
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    /// If set, limit validation to at most this many data rows.
    pub max_rows: Option<usize>,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self { max_rows: None }
    }
}

/// Validate a CSV file against RFC 4180-like structural rules and a schema.
///
/// This function assumes:
/// - UTF-8 encoding.
/// - Comma delimiter.
/// - Double-quote as the quote character.
/// - First row is the header.
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

    // Check header against schema.
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

    // Validate each record.
    for (row_idx, result) in reader.records().enumerate() {
        if let Some(limit) = options.max_rows {
            if row_idx >= limit {
                break;
            }
        }

        let row_number = row_idx + 1; // +1 to account for header row
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

        // Placeholder for future semantic checks based on `schema.columns[idx].ty`.
        // For now, we only enforce structural alignment with the schema.
    }

    Ok(())
}
