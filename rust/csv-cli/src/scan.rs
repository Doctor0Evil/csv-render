// rust/csv-cli/src/scan.rs
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use globwalk::GlobWalkerBuilder;
use thiserror::Error;

use crate::index::{RepoIndex, IndexError};

#[derive(Debug, Error)]
pub enum ScanError {
    #[error(transparent)]
    Index(#[from] IndexError),

    #[error("failed while walking repository: {0}")]
    Walk(#[from] globwalk::WalkError),

    #[error("found unregistered CSV files: {0:?}")]
    Unregistered(Vec<PathBuf>),
}

pub fn scan_repo(index_path: &Path) -> Result<(), ScanError> {
    let index = RepoIndex::load(index_path)?;
    let root = index.root.clone().unwrap_or_else(|| PathBuf::from("."));

    let mut walker = GlobWalkerBuilder::from_patterns(
        &root,
        index.rules.include_globs.iter().map(String::as_str),
    );

    for pat in &index.rules.exclude_globs {
        walker = walker.ignore(pat);
    }

    let walker = walker.build()?;

    let mut discovered: HashSet<PathBuf> = HashSet::new();
    for dir_entry in walker {
        let path = dir_entry?.into_path();
        if path.extension().and_then(|e| e.to_str()) == Some("csv") {
            discovered.insert(path);
        }
    }

    let indexed_paths: HashSet<PathBuf> = index
        .csv_files
        .iter()
        .map(|e| index.resolve_path(&e.path))
        .collect();

    let unregistered: Vec<PathBuf> = discovered
        .difference(&indexed_paths)
        .cloned()
        .collect();

    if !unregistered.is_empty() && index.rules.fail_on_unregistered_csv {
        return Err(ScanError::Unregistered(unregistered));
    }

    Ok(())
}
