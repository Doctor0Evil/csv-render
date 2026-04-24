use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::SchemaLoadError;

/// Representation of a single column in the schema.
///
/// This is intentionally simple as a first pass and can be extended with
/// richer type information and constraints.
#[derive(Debug, Deserialize, Clone)]
pub struct Column {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub separator: Option<String>,
}

/// Top-level schema structure loaded from YAML.
#[derive(Debug, Deserialize, Clone)]
pub struct Schema {
    pub table: String,
    pub columns: Vec<Column>,
}

impl Schema {
    /// Load a schema from a YAML file on disk.
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self, SchemaLoadError> {
        let path_ref = path.as_ref();
        let yaml = fs::read_to_string(path_ref).map_err(|source| SchemaLoadError::Io {
            path: path_ref.to_path_buf(),
            source,
        })?;
        let schema: Schema =
            serde_yaml::from_str(&yaml).map_err(|source| SchemaLoadError::Yaml {
                path: path_ref.to_path_buf(),
                source,
            })?;
        Ok(schema)
    }
}
