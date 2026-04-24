// csv-cli/src/scan.rs

use globwalk::GlobWalkerBuilder;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RepoIndex {
    pub version: u32,
    pub root: Option<String>,
    pub csv_files: Vec<CsvFileEntry>,
    pub rules: Option<RepoRules>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CsvFileEntry {
    pub id: String,
    pub path: String,
    pub schema: Option<String>,
    pub status: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RepoRules {
    pub fail_on_unregistered_csv: Option<bool>,
    pub fail_on_duplicate_paths: Option<bool>,
    pub fail_on_duplicate_ids: Option<bool>,
    pub include_globs: Option<Vec<String>>,
    pub exclude_globs: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct ScanResult {
    pub unregistered_csvs: Vec<PathBuf>,
    pub missing_csvs: Vec<PathBuf>,
    pub duplicate_ids: Vec<String>,
    pub duplicate_paths: Vec<PathBuf>,
}

impl ScanResult {
    pub fn is_clean(&self) -> bool {
        self.unregistered_csvs.is_empty()
            && self.missing_csvs.is_empty()
            && self.duplicate_ids.is_empty()
            && self.duplicate_paths.is_empty()
    }
}

/// Load configs/repo-index.yaml
pub fn load_repo_index(path: &Path) -> Result<RepoIndex, Box<dyn std::error::Error>> {
    let yaml = fs::read_to_string(path)?;
    let index: RepoIndex = serde_yaml::from_str(&yaml)?;
    Ok(index)
}

/// Scan the repository according to repo-index.yaml and return a ScanResult.
pub fn scan_repo(root: &Path, index: &RepoIndex) -> Result<ScanResult, Box<dyn std::error::Error>> {
    let repo_root = if let Some(ref r) = index.root {
        root.join(r)
    } else {
        root.to_path_buf()
    };

    let rules = index.rules.as_ref();

    let include_globs = rules
        .and_then(|r| r.include_globs.clone())
        .unwrap_or_else(|| vec!["**/*.csv".to_string()]);
    let exclude_globs = rules
        .and_then(|r| r.exclude_globs.clone())
        .unwrap_or_else(|| vec!["target/**".to_string(), ".git/**".to_string()]);

    // Build a walker for all CSV files in the repo.
    let mut builder = GlobWalkerBuilder::from_patterns(&repo_root, include_globs);
    for ex in &exclude_globs {
        builder = builder.ignore(ex);
    }
    let walker = builder.build()?;

    let mut discovered_csvs: HashSet<PathBuf> = HashSet::new();

    for entry in walker {
        let entry = entry?;
        let path = entry.path().to_path_buf();
        if path.is_file() {
            // Normalize to path relative to repo_root for comparison.
            let rel = pathdiff::diff_paths(&path, &repo_root).unwrap_or(path.clone());
            discovered_csvs.insert(rel);
        }
    }

    // Build maps from the index.
    let mut id_counts: HashMap<String, usize> = HashMap::new();
    let mut path_counts: HashMap<PathBuf, usize> = HashMap::new();
    let mut indexed_paths: HashSet<PathBuf> = HashSet::new();

    for entry in &index.csv_files {
        *id_counts.entry(entry.id.clone()).or_insert(0) += 1;
        let p = PathBuf::from(&entry.path);
        *path_counts.entry(p.clone()).or_insert(0) += 1;
        indexed_paths.insert(p);
    }

    let duplicate_ids: Vec<String> = id_counts
        .into_iter()
        .filter_map(|(id, count)| if count > 1 { Some(id) } else { None })
        .collect();

    let duplicate_paths: Vec<PathBuf> = path_counts
        .into_iter()
        .filter_map(|(p, count)| if count > 1 { Some(p) } else { None })
        .collect();

    // Unregistered: discovered on disk but not listed in index.
    let unregistered_csvs: Vec<PathBuf> = discovered_csvs
        .difference(&indexed_paths)
        .cloned()
        .collect();

    // Missing: listed in index but not found on disk.
    let missing_csvs: Vec<PathBuf> = indexed_paths
        .difference(&discovered_csvs)
        .cloned()
        .collect();

    Ok(ScanResult {
        unregistered_csvs,
        missing_csvs,
        duplicate_ids,
        duplicate_paths,
    })
}

/// Run scan and print a human-readable report, returning an exit code.
pub fn run_scan_command(repo_root: &Path, index_path: &Path) -> i32 {
    let index = match load_repo_index(index_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to load repo index: {}", e);
            return 1;
        }
    };

    let rules = index.rules.as_ref();
    let fail_on_unregistered = rules
        .and_then(|r| r.fail_on_unregistered_csv)
        .unwrap_or(true);
    let fail_on_duplicate_ids = rules
        .and_then(|r| r.fail_on_duplicate_ids)
        .unwrap_or(true);
    let fail_on_duplicate_paths = rules
        .and_then(|r| r.fail_on_duplicate_paths)
        .unwrap_or(true);

    let result = match scan_repo(repo_root, &index) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Scan failed: {}", e);
            return 1;
        }
    };

    let mut exit_code = 0;

    if !result.duplicate_ids.is_empty() {
        eprintln!("Duplicate CSV ids found in repo-index.yaml:");
        for id in &result.duplicate_ids {
            eprintln!("  - {}", id);
        }
        if fail_on_duplicate_ids {
            exit_code = 1;
        }
    }

    if !result.duplicate_paths.is_empty() {
        eprintln!("Duplicate CSV paths found in repo-index.yaml:");
        for p in &result.duplicate_paths {
            eprintln!("  - {}", p.display());
        }
        if fail_on_duplicate_paths {
            exit_code = 1;
        }
    }

    if !result.unregistered_csvs.is_empty() {
        eprintln!("Unregistered CSV files discovered in repository:");
        for p in &result.unregistered_csvs {
            eprintln!("  - {}", p.display());
        }
        if fail_on_unregistered {
            exit_code = 1;
        }
    }

    if !result.missing_csvs.is_empty() {
        eprintln!("CSV files listed in repo-index.yaml but missing on disk:");
        for p in &result.missing_csvs {
            eprintln!("  - {}", p.display());
        }
        // Missing files are almost always a problem; treat as failure.
        exit_code = 1;
    }

    if exit_code == 0 {
        println!("csv-cli scan: OK (all CSV files match repo-index.yaml)");
    }

    exit_code
}
