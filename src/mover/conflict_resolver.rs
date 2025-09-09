use std::path::{Path, PathBuf};

use tokio::fs;

use crate::errors::Result;

/// Renames conflicting destination by appending counter (file.txt → file_1.txt).
pub async fn resolve_conflict(path: &Path, overwrite: bool) -> Result<PathBuf> {
    if overwrite {
        if tokio::fs::try_exists(path).await? {
            tokio::fs::remove_file(path).await?;
        }
        return Ok(path.to_path_buf())
    }

    // Non-overwrite: keep original if free
    if !tokio::fs::try_exists(path).await? {
        return Ok(path.to_path_buf());
    }
    
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();

    let mut counter = 1;
    loop {
        let candidate = parent.join(format!("{}_{}{}", stem, counter, ext));
        if !fs::try_exists(&candidate).await? {
            return Ok(candidate);
        }
        counter += 1;
    }
}