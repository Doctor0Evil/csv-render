use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SchemaLoadError {
    #[error("failed to read schema file {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse YAML schema file {path}: {source}")]
    ParseYaml {
        path: String,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("failed to parse TOML config file {path}: {source}")]
    ParseToml {
        path: String,
        #[source]
        source: toml::de::Error,
    },

    #[error("schema has no columns defined")]
    EmptySchema,

    #[error("duplicate column name `{0}` in schema")]
    DuplicateColumn(String),
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawColumn {
    pub name: String,
    pub r#type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub separator: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RawSchema {
    pub table: String,
    pub columns: Vec<RawColumn>,
}

#[derive(Debug, Clone)]
pub enum ColumnType {
    String,
    U64,
    StringList { separator: char },
    Enum { allowed: Vec<String> },
}

#[derive(Debug, Clone)]
pub struct Column {
    pub index: usize,
    pub name: String,
    pub col_type: ColumnType,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct TypedSchema {
    pub table: String,
    pub columns: Vec<Column>,
    pub by_name: HashMap<String, usize>,
}

impl TypedSchema {
    pub fn load(path: &Path) -> Result<Self, SchemaLoadError> {
        let text = fs::read_to_string(path).map_err(|source| SchemaLoadError::Io {
            path: path.display().to_string(),
            source,
        })?;

        let raw: RawSchema =
            serde_yaml::from_str(&text).map_err(|source| SchemaLoadError::ParseYaml {
                path: path.display().to_string(),
                source,
            })?;

        if raw.columns.is_empty() {
            return Err(SchemaLoadError::EmptySchema);
        }

        let mut by_name = HashMap::new();
        let mut columns = Vec::with_capacity(raw.columns.len());

        for (idx, rc) in raw.columns.into_iter().enumerate() {
            if by_name.insert(rc.name.clone(), idx).is_some() {
                return Err(SchemaLoadError::DuplicateColumn(rc.name));
            }

            let col_type = match rc.r#type.as_str() {
                "String" => ColumnType::String,
                "U64" => ColumnType::U64,
                "StringList" => ColumnType::StringList {
                    separator: rc
                        .separator
                        .unwrap_or_else(|| ";".to_string())
                        .chars()
                        .next()
                        .unwrap_or(';'),
                },
                other => ColumnType::Enum {
                    allowed: vec![other.to_string()],
                },
            };

            columns.push(Column {
                index: idx,
                name: rc.name,
                col_type,
                required: rc.required,
            });
        }

        Ok(TypedSchema {
            table: raw.table,
            columns,
            by_name,
        })
    }

    pub fn arity(&self) -> usize {
        self.columns.len()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct NeurorightsConfig {
    pub flags: HashMap<String, bool>,
}

impl NeurorightsConfig {
    pub fn from_toml_file(path: &Path) -> Result<Self, SchemaLoadError> {
        let toml_str = fs::read_to_string(path).map_err(|source| SchemaLoadError::Io {
            path: path.display().to_string(),
            source,
        })?;

        let cfg: NeurorightsConfig =
            toml::from_str(&toml_str).map_err(|source| SchemaLoadError::ParseToml {
                path: path.display().to_string(),
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
