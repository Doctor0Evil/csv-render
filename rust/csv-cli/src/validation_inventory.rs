// csv-cli/src/validation_inventory.rs

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use crate::scan::{load_repo_index, RepoIndex};

#[derive(Debug, Serialize)]
struct InventoryRow {
    id: String,
    path: String,
    columns: String,
    row_count: u64,
    digest_hex8: String,
    algorithm: String,
}

fn normalize_newlines(mut bytes: Vec<u8>) -> Vec<u8> {
    // Normalize CRLF (\r\n) to LF (\n), keep lone \n as is.
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                out.push(b'\n');
                i += 2;
            } else {
                // Lone \r -> convert to \n for consistency.
                out.push(b'\n');
                i += 1;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

/// Compute SHA-256 digest and return first 8 hex characters.
fn digest_hex8(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let result = hasher.finalize();
    let full_hex = hex::encode(result);
    full_hex[..8].to_string()
}

/// Extract header columns and row count from a normalized UTF-8 CSV.
/// Assumes RFC-4180 style structure.
fn analyze_csv(path: &Path) -> Result<(Vec<String>, u64), Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut lines = reader.lines();
    let mut row_count: u64 = 0;

    let first = match lines.next() {
        Some(line) => line?,
        None => {
            return Ok((Vec::new(), 0));
        }
    };
    row_count += 1;

    // Very simple header split: delegate full quoting rules to csv-core if available.
    // Here we fallback to Rust's csv crate for robust parsing.
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(first.as_bytes());

    let header = rdr.headers()?;
    let columns: Vec<String> = header.iter().map(|s| s.to_string()).collect();

    // Count remaining lines (approximate row count including header).
    for line in lines {
        let l = line?;
        if !l.trim().is_empty() {
            row_count += 1;
        }
    }

    Ok((columns, row_count))
}

/// Generate inventory rows for all indexed CSV files.
fn build_inventory(
    repo_root: &Path,
    index: &RepoIndex,
) -> Result<Vec<InventoryRow>, Box<dyn std::error::Error>> {
    let root = if let Some(ref r) = index.root {
        repo_root.join(r)
    } else {
        repo_root.to_path_buf()
    };

    let mut rows = Vec::new();

    for entry in &index.csv_files {
        let rel_path = PathBuf::from(&entry.path);
        let abs_path = root.join(&rel_path);

        let raw_bytes = fs::read(&abs_path)?;
        let norm_bytes = normalize_newlines(raw_bytes);

        // Ensure UTF-8 decoding; this will fail if encoding is wrong.
        let _ = std::str::from_utf8(&norm_bytes)?;

        let digest = digest_hex8(&norm_bytes);
        let (columns, row_count) = analyze_csv(&abs_path)?;

        let columns_joined = if columns.is_empty() {
            "".to_string()
        } else {
            // Join with a pipe; this avoids embedding commas into the inventory CSV
            // and keeps columns human-readable.
            columns.join("|")
        };

        rows.push(InventoryRow {
            id: entry.id.clone(),
            path: entry.path.clone(),
            columns: columns_joined,
            row_count,
            digest_hex8: digest,
            algorithm: "sha256".to_string(),
        });
    }

    Ok(rows)
}

/// Emit the inventory as a GitHub-safe CSV to stdout.
fn write_inventory_csv(rows: &[InventoryRow]) -> Result<(), Box<dyn std::error::Error>> {
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(std::io::stdout());

    for row in rows {
        wtr.serialize(row)?;
    }
    wtr.flush()?;
    Ok(())
}

/// Public entry point: run `validation-inventory` subcommand.
pub fn run_validation_inventory(repo_root: &Path, index_path: &Path) -> i32 {
    let index = match load_repo_index(index_path) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to load repo index `{}`: {}", index_path.display(), e);
            return 1;
        }
    };

    let rows = match build_inventory(repo_root, &index) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to build validation inventory: {}", e);
            return 1;
        }
    };

    if let Err(e) = write_inventory_csv(&rows) {
        eprintln!("Failed to write validation inventory CSV: {}", e);
        return 1;
    }

    0
}
