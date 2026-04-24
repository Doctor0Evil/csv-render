use std::path::PathBuf;
use thiserror::Error;

/// High-level error category used for JSON reporting.
#[derive(Debug, Clone, Copy)]
pub enum ErrorCategory {
    Lexical,
    Structural,
    Semantic,
    Relational,
    Io,
}

impl ErrorCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCategory::Lexical => "lexical",
            ErrorCategory::Structural => "structural",
            ErrorCategory::Semantic => "semantic",
            ErrorCategory::Relational => "relational",
            ErrorCategory::Io => "io",
        }
    }
}

/// Error type for schema loading and parsing.
#[derive(Debug, Error)]
pub enum SchemaLoadError {
    #[error("failed to read schema file {path:?}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse YAML schema file {path:?}")]
    Yaml {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("failed to parse TOML config file {path:?}")]
    Toml {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
}

/// Structured validation error type used throughout the library.
///
/// This type is designed to support:
/// - stable `code` strings,
/// - high-level `category`,
/// - row/column coordinates where applicable,
/// - human-readable `message`,
/// - JSON `details` payloads for downstream tooling.
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("schema error: {message}")]
    Schema {
        code: &'static str,
        message: String,
        #[source]
        source: Option<SchemaLoadError>,
    },

    #[error("structural error at row {row}, column {column}: {message}")]
    Structural {
        code: &'static str,
        row: usize,
        column: usize,
        message: String,
    },

    #[error("semantic error at row {row}, column {column}: {message}")]
    Semantic {
        code: &'static str,
        row: usize,
        column: usize,
        message: String,
        details: serde_json::Value,
    },

    #[error("relational error at row {row}, column {column}: {message}")]
    Relational {
        code: &'static str,
        row: usize,
        column: usize,
        message: String,
        details: serde_json::Value,
    },

    #[error("io error ({code}) at {path:?}: {message}")]
    Io {
        code: &'static str,
        path: Option<PathBuf>,
        message: String,
        #[source]
        source: Option<std::io::Error>,
    },
}

impl ValidationError {
    pub fn category(&self) -> ErrorCategory {
        match self {
            ValidationError::Schema { .. } => ErrorCategory::Structural,
            ValidationError::Structural { .. } => ErrorCategory::Structural,
            ValidationError::Semantic { .. } => ErrorCategory::Semantic,
            ValidationError::Relational { .. } => ErrorCategory::Relational,
            ValidationError::Io { .. } => ErrorCategory::Io,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            ValidationError::Schema { code, .. } => code,
            ValidationError::Structural { code, .. } => code,
            ValidationError::Semantic { code, .. } => code,
            ValidationError::Relational { code, .. } => code,
            ValidationError::Io { code, .. } => code,
        }
    }
}
