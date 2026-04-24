use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use serde::Serialize;

use csv_core::error::{ErrorCategory, ValidationError};

/// Top-level JSON report structure matching docs/errorreport-schema.md.
#[derive(Debug, Serialize)]
pub struct ErrorReport {
    pub version: u32,
    pub file: String,
    pub schema_id: Option<String>,
    pub summary: ErrorSummary,
    pub errors: Vec<ErrorEntry>,
}

#[derive(Debug, Serialize)]
pub struct ErrorSummary {
    pub total_rows: usize,
    pub errors: usize,
    pub categories: CategoryCounts,
}

#[derive(Debug, Default, Serialize)]
pub struct CategoryCounts {
    pub lexical: usize,
    pub structural: usize,
    pub semantic: usize,
    pub relational: usize,
    pub io: usize,
}

#[derive(Debug, Serialize)]
pub struct ErrorEntry {
    pub row: Option<usize>,
    pub column: Option<usize>,
    pub column_name: Option<String>,
    pub category: String,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Build an ErrorReport from a list of ValidationError and some metadata.
///
/// `total_rows` should be the number of data rows processed (excluding header).
pub fn build_error_report(
    file: &str,
    schema_id: Option<&str>,
    total_rows: usize,
    errors: Vec<(ValidationError, Option<String>)>,
) -> ErrorReport {
    let mut counts = CategoryCounts::default();
    let mut entries = Vec::with_capacity(errors.len());

    for (err, column_name) in errors.into_iter() {
        let category = err.category();
        match category {
            ErrorCategory::Lexical => counts.lexical += 1,
            ErrorCategory::Structural => counts.structural += 1,
            ErrorCategory::Semantic => counts.semantic += 1,
            ErrorCategory::Relational => counts.relational += 1,
            ErrorCategory::Io => counts.io += 1,
        }

        let (row, column, message, details) = match &err {
            ValidationError::Schema { message, .. } => (None, None, message.clone(), None),
            ValidationError::Structural {
                row,
                column,
                message,
                ..
            } => (Some(*row), Some(*column), message.clone(), None),
            ValidationError::Semantic {
                row,
                column,
                message,
                details,
                ..
            } => (Some(*row), Some(*column), message.clone(), Some(details.clone())),
            ValidationError::Relational {
                row,
                column,
                message,
                details,
                ..
            } => (Some(*row), Some(*column), message.clone(), Some(details.clone())),
            ValidationError::Io {
                message, .. 
            } => (None, None, message.clone(), None),
        };

        let entry = ErrorEntry {
            row,
            column,
            column_name: column_name.clone(),
            category: category.as_str().to_string(),
            code: err.code().to_string(),
            message,
            details,
        };

        entries.push(entry);
    }

    let summary = ErrorSummary {
        total_rows,
        errors: entries.len(),
        categories: counts,
    };

    ErrorReport {
        version: 1,
        file: file.to_string(),
        schema_id: schema_id.map(|s| s.to_string()),
        summary,
        errors: entries,
    }
}

/// Write the report as pretty-printed JSON to the given writer.
pub fn write_error_report_json<W: Write>(
    report: &ErrorReport,
    mut out: W,
) -> io::Result<()> {
    let value = serde_json::to_value(report).expect("serializable report");
    let json = serde_json::to_string_pretty(&value)?;
    out.write_all(json.as_bytes())
}

/// Convenience helper to write a report directly to a file path.
pub fn write_error_report_json_to_path(
    report: &ErrorReport,
    path: &Path,
) -> io::Result<()> {
    let file = File::create(path)?;
    write_error_report_json(report, file)
}
