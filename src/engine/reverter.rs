use std::{collections::HashSet, path::Path, sync::Arc};
use tokio::fs;

use crate::{
    conflict_resolver::resolve_conflict,
    index::{Db, DbFileEntry},
    file_mover::FileMover,
    errors::Result,
    utils::default_db_path,
};

/// Iteratively remove empty directories under `root`.
async fn cleanup_empty_dirs(root: &Path) -> Result<()> {
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut entries = fs::read_dir(&dir).await?;
        let mut is_empty = true;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                is_empty = false; // contains subdir, revisit later
            } else {
                is_empty = false;
            }
        }

        // Don’t remove the original root itself
        if is_empty && dir != root {
            if fs::remove_dir(&dir).await.is_ok() {
                tracing::info!("Removed empty dir: {:?}", dir);
            }
        }
    }

    Ok(())
}


/// Reverts previously organized files back to their original locations.
pub async fn revert_files(root_dir: &Path, cleanup: bool) -> Result<()> {
    let db = Arc::new(Db::new(&default_db_path()).await?);
    let mover = Arc::new(FileMover::new());

    // Deduplicate by dest_path
    let mut seen = HashSet::new();
    let files: Vec<DbFileEntry> = db.get_all_files()
        .await?
        .into_iter()
        .filter(|f| f.path.starts_with(root_dir)) // Only revert inside root_dir
        .filter(|f| seen.insert(f.dest_path.clone()))
        .collect();

    // Begin transaction for performance
    let mut tx = db.begin().await?;

    for file in &files {
        let source = file.dest_path.clone();
        let original = file.path.clone();

        if !tokio::fs::try_exists(&source).await? {
            tracing::warn!("Missing file at destination, skipping: {:?}", source);
            continue;
        }

        // If original already exists, resolve conflict
        let final_path = if tokio::fs::try_exists(&original).await? {
            resolve_conflict(&original, true).await?
        } else {
            original
        };

        // Move file back
        mover.move_file(&source, &final_path).await?;
        tracing::info!("Reverted {:?} -> {:?}", source, final_path);

        db.update_dest_path_tx(&mut tx, &file.path, &final_path).await?;
    }

    tx.commit().await?;
    tracing::info!("Revert completed with {} files processed.", files.len());

    if cleanup {
        if let Err(e) = cleanup_empty_dirs(root_dir).await {
            tracing::warn!("Failed to fully cleanup dirs: {:?}", e);
        }
    }

    Ok(())
}
