use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use csv_core::error::ValidationError;

/// Atomically write a CSV file by writing to a temporary file, fsyncing, and
/// renaming into place, then fsyncing the parent directory.
///
/// This function guarantees that a reader will see either the old file or the
/// new, fully-written file, but never a partially-written intermediate.
pub fn atomic_write_csv(path: &Path, bytes: &[u8]) -> Result<(), ValidationError> {
    let dir = path.parent().ok_or_else(|| ValidationError::Io {
        code: "IO_NO_PARENT_DIRECTORY",
        path: Some(path.to_path_buf()),
        message: "target path has no parent directory".to_string(),
        source: None,
    })?;

    let mut tmp = path.to_path_buf();
    tmp.set_extension("tmp");

    let mut file = File::create(&tmp).map_err(|e| ValidationError::Io {
        code: "IO_CREATE_TEMP_FAILED",
        path: Some(tmp.clone()),
        message: "failed to create temp file".to_string(),
        source: Some(e),
    })?;

    file.write_all(bytes).map_err(|e| ValidationError::Io {
        code: "IO_WRITE_TEMP_FAILED",
        path: Some(tmp.clone()),
        message: "failed to write temp file".to_string(),
        source: Some(e),
    })?;

    file.sync_all().map_err(|e| ValidationError::Io {
        code: "IO_FSYNC_TEMP_FAILED",
        path: Some(tmp.clone()),
        message: "failed to fsync temp file".to_string(),
        source: Some(e),
    })?;

    fs::rename(&tmp, path).map_err(|e| ValidationError::Io {
        code: "IO_RENAME_FAILED",
        path: Some(path.to_path_buf()),
        message: "failed to rename temp file into place".to_string(),
        source: Some(e),
    })?;

    let dir_file = File::open(dir).map_err(|e| ValidationError::Io {
        code: "IO_OPEN_DIR_FAILED",
        path: Some(dir.to_path_buf()),
        message: "failed to open parent directory".to_string(),
        source: Some(e),
    })?;

    dir_file.sync_all().map_err(|e| ValidationError::Io {
        code: "IO_FSYNC_DIR_FAILED",
        path: Some(dir.to_path_buf()),
        message: "failed to fsync parent directory".to_string(),
        source: Some(e),
    })?;

    Ok(())
}
