// rust/csv-cli/src/index.rs
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexError {
    #[error("failed to read index file {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse index file {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_yaml::Error,
    },

    #[error("duplicate csv_files.id `{id}` in repo-index.yaml")]
    DuplicateId { id: String },

    #[error("duplicate csv_files.path `{path}` in repo-index.yaml")]
    DuplicatePath { path: String },

    #[error("missing indexed CSV file at `{path}`")]
    MissingFile { path: PathBuf },
}

#[derive(Debug, Deserialize)]
pub struct RepoIndex {
    pub version: u32,
    #[serde(default)]
    pub root: Option<PathBuf>,
    pub csv_files: Vec<CsvIndexEntry>,
    #[serde(default)]
    pub rules: IndexRules,
}

#[derive(Debug, Deserialize)]
pub struct CsvIndexEntry {
    pub id: String,
    pub path: PathBuf,
    pub schema: PathBuf,
    #[serde(default = "default_status")]
    pub status: String,
}

fn default_status() -> String {
    "current".to_string()
}

#[derive(Debug, Deserialize)]
pub struct IndexRules {
    #[serde(default = "default_true")]
    pub fail_on_unregistered_csv: bool,
    #[serde(default = "default_true")]
    pub fail_on_duplicate_ids: bool,
    #[serde(default = "default_true")]
    pub fail_on_duplicate_paths: bool,
    #[serde(default = "default_glob_include")]
    pub include_globs: Vec<String>,
    #[serde(default = "default_glob_exclude")]
    pub exclude_globs: Vec<String>,
}

fn default_true() -> bool {
    true
}

fn default_glob_include() -> Vec<String> {
    vec!["**/*.csv".to_string()]
}

fn default_glob_exclude() -> Vec<String> {
    vec!["target/**".to_string(), ".git/**".to_string()]
}

impl RepoIndex {
    pub fn load(path: &Path) -> Result<Self, IndexError> {
        let text = std::fs::read_to_string(path).map_err(|source| IndexError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut index: RepoIndex =
            serde_yaml::from_str(&text).map_err(|source| IndexError::Parse {
                path: path.to_path_buf(),
                source,
            })?;
        if index.root.is_none() {
            index.root = Some(PathBuf::from("."));
        }
        index.check_invariants(path)?;
        Ok(index)
    }

    fn check_invariants(&self, index_path: &Path) -> Result<(), IndexError> {
        let mut ids = HashSet::new();
        let mut paths = HashSet::new();

        for entry in &self.csv_files {
            if !ids.insert(entry.id.clone()) && self.rules.fail_on_duplicate_ids {
                return Err(IndexError::DuplicateId {
                    id: entry.id.clone(),
                });
            }

            let effective_path = self.resolve_path(&entry.path);
            let path_str = effective_path.to_string_lossy().into_owned();

            if !paths.insert(path_str.clone()) && self.rules.fail_on_duplicate_paths {
                return Err(IndexError::DuplicatePath { path: path_str });
            }

            if !effective_path.is_file() {
                return Err(IndexError::MissingFile {
                    path: effective_path,
                });
            }

            // The index file itself must be loadable, but schema parsing errors
            // are handled later by csv-core's schema loader.
            let _schema_path = self.resolve_path(&entry.schema);
        }

        tracing::info!(
            "validated repo-index invariants for {} CSV files (index at {})",
            self.csv_files.len(),
            index_path.display()
        );
        Ok(())
    }

    pub fn resolve_path(&self, rel: &Path) -> PathBuf {
        match &self.root {
            Some(root) => root.join(rel),
            None => rel.to_path_buf(),
        }
    }

    pub fn by_id(&self) -> HashMap<String, &CsvIndexEntry> {
        self.csv_files
            .iter()
            .map(|e| (e.id.clone(), e))
            .collect::<HashMap<_, _>>()
    }
}
