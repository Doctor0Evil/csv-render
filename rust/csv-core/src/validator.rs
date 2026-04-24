use std::fs::File;
use std::fmt;
use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};

use csv::ReaderBuilder;

use crate::error::ValidationError;
use crate::schema::{NeurorightsConfig, Schema};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsvErrorCategory {
    Encoding,
    Structure,
    Quoting,
    ColumnCount,
}

#[derive(Debug, Clone)]
pub struct CsvError {
    pub row_index: usize,
    pub column_index: Option<usize>,
    pub category: CsvErrorCategory,
    pub message: String,
}

impl fmt::Display for CsvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(col) = self.column_index {
            write!(
                f,
                "row {}, column {}: {:?} - {}",
                self.row_index, col, self.category, self.message
            )
        } else {
            write!(
                f,
                "row {}: {:?} - {}",
                self.row_index, self.category, self.message
            )
        }
    }
}

impl std::error::Error for CsvError {}

#[derive(Debug, Clone)]
pub struct CsvSchema {
    pub column_count: usize,
    pub header: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CsvRecord {
    pub fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CsvValidatorConfig {
    pub delimiter: u8,
    pub enforce_utf8: bool,
}

impl Default for CsvValidatorConfig {
    fn default() -> Self {
        Self {
            delimiter: b',',
            enforce_utf8: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CsvValidator {
    config: CsvValidatorConfig,
}

impl CsvValidator {
    pub fn new(config: CsvValidatorConfig) -> Self {
        CsvValidator { config }
    }

    pub fn validate_reader<R: Read>(
        &self,
        reader: R,
    ) -> Result<(CsvSchema, Vec<CsvRecord>), CsvError> {
        let mut buf_reader = std::io::BufReader::new(reader);
        let mut first_line = String::new();

        let bytes_read = buf_reader
            .read_line(&mut first_line)
            .map_err(|e| CsvError {
                row_index: 0,
                column_index: None,
                category: CsvErrorCategory::Encoding,
                message: format!("failed to read header line: {}", e),
            })?;

        if bytes_read == 0 {
            return Err(CsvError {
                row_index: 0,
                column_index: None,
                category: CsvErrorCategory::Structure,
                message: "empty CSV: no header row".to_string(),
            });
        }

        if self.config.enforce_utf8 && !first_line.is_char_boundary(first_line.len()) {
            return Err(CsvError {
                row_index: 0,
                column_index: None,
                category: CsvErrorCategory::Encoding,
                message: "header not valid UTF-8 boundary".to_string(),
            });
        }

        let first_line = first_line.trim_end_matches(|c| c == '\n' || c == '\r').to_string();
        let header_fields = self.parse_line(&first_line, 0)?;

        if header_fields.is_empty() {
            return Err(CsvError {
                row_index: 0,
                column_index: None,
                category: CsvErrorCategory::Structure,
                message: "header row has zero columns".to_string(),
            });
        }

        let schema = CsvSchema {
            column_count: header_fields.len(),
            header: header_fields,
        };

        let mut records = Vec::new();
        let mut row_index = 1usize;
        let mut line = String::new();

        loop {
            line.clear();
            let n = buf_reader
                .read_line(&mut line)
                .map_err(|e| CsvError {
                    row_index,
                    column_index: None,
                    category: CsvErrorCategory::Encoding,
                    message: format!("failed to read line: {}", e),
                })?;
            if n == 0 {
                break;
            }

            let trimmed = line.trim_end_matches(|c| c == '\n' || c == '\r');
            if trimmed.is_empty() {
                row_index += 1;
                continue;
            }

            let fields = self.parse_line(trimmed, row_index)?;

            if fields.len() != schema.column_count {
                return Err(CsvError {
                    row_index,
                    column_index: None,
                    category: CsvErrorCategory::ColumnCount,
                    message: format!(
                        "expected {} columns based on header, found {}",
                        schema.column_count,
                        fields.len()
                    ),
                });
            }

            records.push(CsvRecord { fields });
            row_index += 1;
        }

        Ok((schema, records))
    }

    fn parse_line(&self, line: &str, row_index: usize) -> Result<Vec<String>, CsvError> {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum State {
            StartField,
            InUnquoted,
            InQuoted,
            AfterQuoteInQuoted,
        }

        let delimiter = self.config.delimiter as char;
        let mut state = State::StartField;
        let mut fields: Vec<String> = Vec::new();
        let mut current = String::new();
        let mut col_index = 0usize;

        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            match state {
                State::StartField => {
                    if ch == delimiter {
                        fields.push(String::new());
                        col_index += 1;
                    } else if ch == '"' {
                        state = State::InQuoted;
                    } else {
                        current.push(ch);
                        state = State::InUnquoted;
                    }
                }
                State::InUnquoted => {
                    if ch == delimiter {
                        fields.push(current.clone());
                        current.clear();
                        col_index += 1;
                        state = State::StartField;
                    } else if ch == '"' {
                        return Err(CsvError {
                            row_index,
                            column_index: Some(col_index),
                            category: CsvErrorCategory::Quoting,
                            message: "unexpected quote in unquoted field".to_string(),
                        });
                    } else {
                        current.push(ch);
                    }
                }
                State::InQuoted => {
                    if ch == '"' {
                        state = State::AfterQuoteInQuoted;
                    } else {
                        current.push(ch);
                    }
                }
                State::AfterQuoteInQuoted => {
                    if ch == '"' {
                        current.push('"');
                        state = State::InQuoted;
                    } else if ch == delimiter {
                        fields.push(current.clone());
                        current.clear();
                        col_index += 1;
                        state = State::StartField;
                    } else {
                        return Err(CsvError {
                            row_index,
                            column_index: Some(col_index),
                            category: CsvErrorCategory::Quoting,
                            message: format!(
                                "unexpected character '{}' after closing quote",
                                ch
                            ),
                        });
                    }
                }
            }
        }

        match state {
            State::StartField => {
                fields.push(String::new());
            }
            State::InUnquoted => {
                fields.push(current);
            }
            State::InQuoted | State::AfterQuoteInQuoted => {
                return Err(CsvError {
                    row_index,
                    column_index: Some(col_index),
                    category: CsvErrorCategory::Quoting,
                    message: "unterminated quoted field at end of line".to_string(),
                });
            }
        }

        Ok(fields)
    }
}

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
