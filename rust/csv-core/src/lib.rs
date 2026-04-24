//! csv-core
//!
//! Core CSV validation library for the `csv-render` project.
//!
//! Responsibilities:
//!   1. Enforce RFC 4180 structural rules (delimiter, quoting, escaping).
//!   2. Ensure consistent column counts across all records.
//!   3. Apply schema-based, semantic validation using declarative configs.
//!
//! This crate acts as the oracle for CSV correctness within the repository.
//! Tools in other languages (Python, shell, AI agents) are expected to
//! treat the results of this library as authoritative.

pub mod error;
pub mod schema;
pub mod validator;

pub use crate::error::{CsvError, CsvErrorCategory};
pub use crate::schema::{Schema, SchemaLoadError};
pub use crate::validator::{validate_csv_with_schema, ValidationOptions};
