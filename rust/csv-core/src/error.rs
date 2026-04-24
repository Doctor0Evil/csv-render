use std::path::PathBuf;

use thiserror::Error;

/// Top-level error type for CSV validation operations.
///
/// This error is designed to be serializable and machine-readable so that
/// other tools and AI agents can reason about the exact cause of failure.
#[derive(Debug, Error)]
pub enum ValidationError {
    /// Errors that occur while loading or parsing a schema file.
    #[error("schema error: {0}")]
    Schema(#[from] SchemaLoadError),

    /// Errors that occur while reading CSV input.
    #[error("I/O error while reading CSV: {0}")]
    Io(#[from] std::io::Error),

    /// Structural CSV issues (delimiter, quoting, column count).
    #[error("structural error at row {row}, column {column}: {message}")]
    Structural {
        row: usize,
        column: usize,
        message: String,
    },

    /// Semantic schema violations (type mismatches, enum violations, etc.).
    #[error("schema violation at row {row}, column {column}: {message}")]
    Semantic {
        row: usize,
        column: usize,
        message: String,
    },
}

/// Error type for schema loading and parsing.
#[derive(Debug, Error)]
pub enum SchemaLoadError {
    /// A schema file could not be read from disk.
    #[error("failed to read schema file {path:?}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A YAML schema file could not be parsed.
    #[error("failed to parse YAML schema {path:?}: {source}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    /// A TOML configuration file could not be parsed.
    #[error("failed to parse TOML config {path:?}: {source}")]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}
