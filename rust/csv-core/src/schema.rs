use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::error::SchemaLoadError;

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

#[derive(Debug, Deserialize, Clone)]
pub struct Schema {
    pub table: String,
    pub columns: Vec<Column>,
}

impl Schema {
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

#[derive(Debug, Deserialize, Clone)]
pub struct NeurorightsConfig {
    pub flags: HashMap<String, bool>,
}

impl NeurorightsConfig {
    pub fn from_toml_file<P: AsRef<Path>>(path: P) -> Result<Self, SchemaLoadError> {
        let path_ref = path.as_ref();
        let toml_str = fs::read_to_string(path_ref).map_err(|source| SchemaLoadError::Io {
            path: path_ref.to_path_buf(),
            source,
        })?;
        let cfg: NeurorightsConfig =
            toml::from_str(&toml_str).map_err(|source| SchemaLoadError::Toml {
                path: path_ref.to_path_buf(),
                source,
            })?;
        Ok(cfg)
    }

    pub fn allowed_flags(&self) -> HashSet<String> {
        self.flags
            .iter()
            .filter_map(|(k, v)| if *v { Some(k.clone()) } else { None })
            .collect()
    }
}
