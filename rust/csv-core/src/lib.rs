//! csv-core
//!
//! Core CSV validation library for the `csv-render` project.
//!
//! This crate is responsible for three things:
//!   1. Enforcing RFC 4180 structural rules (delimiter, quoting, escaping).
//!   2. Ensuring consistent column counts across all records.
//!   3. Applying schema-based, semantic validation using declarative configs.
//!
//! The crate is intended to act as an oracle for "CSV correctness" within the
//! repository. Tools in other languages (Python, shell, AI agents) are expected
//! to treat the results of this library as authoritative.

pub mod error;
pub mod schema;
pub mod validator;

pub use crate::error::ValidationError;
pub use crate::schema::{Schema, SchemaLoadError};
pub use crate::validator::{validate_csv_with_schema, ValidationOptions};
