use std::{collections::HashSet, path::Path, sync::Arc};
use tokio::fs;

use crate::{
    conflict_resolver::resolve_conflict, 
    errors::{Result, FileOrganizerError}, 
    file_mover::FileMover, 
    index::{Db, DbFileEntry}, 
    utils::{default_db_path, make_progress}
};

/// Iteratively remove empty directories under `root` (post-order).
pub async fn cleanup_empty_dirs(root: &Path) -> Result<()> {
    let mut stack = vec![(root.to_path_buf(), false)];

    while let Some((dir, visited)) = stack.pop() {
        if !visited {
            stack.push((dir.clone(), true));

            let mut entries = match fs::read_dir(&dir).await {
                Ok(e) => e,
                Err(_) => continue,
            };

            while let Some(entry) = entries.next_entry().await? {
                let path = entry.path();
                if path.is_dir() {
                    stack.push((path, false));
                }
            }
        } else {
            // Second time: check if empty and remove
            let mut entries = fs::read_dir(&dir).await?;
            if entries.next_entry().await?.is_none() && dir != root {
                if fs::remove_dir(&dir).await.is_ok() {
                    tracing::info!("Removed empty dir: {:?}", dir);
                }
            }
        }
    }

    Ok(())
}


/// Reverts previously organized files back to their original locations.
pub async fn revert_files(root_dir: &Path, cleanup: bool) -> Result<()> {
    if !root_dir.exists() {
        return Err(FileOrganizerError::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Directory {:?} does not exist", root_dir),
        )));
    }

    if !root_dir.is_dir() {
        return Err(FileOrganizerError::from(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Path {:?} is not a directory", root_dir),
        )));
    }

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

    let total = files.len();
    let pb = make_progress(total as u64, "Reverting");

    // Begin transaction for performance
    let mut tx = db.begin().await?;

    for file in &files {
        let source = file.dest_path.clone();
        let original = file.path.clone();

        if !tokio::fs::try_exists(&source).await? {
            tracing::warn!("Missing file at destination, skipping: {:?}", source);
            pb.inc(1);
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
        tracing::debug!(target: "reverter", "Reverted {:?} -> {:?}", source, final_path);

        db.update_dest_path_tx(&mut tx, &file.path, &final_path).await?;
        pb.inc(1);
    }

    tx.commit().await?;
    pb.finish_with_message(format!("♻️ Revert completed: {} files processed", total));

    tracing::info!(target: "reverter", "Revert completed with {} files processed.", total);

    if cleanup {
        if let Err(e) = cleanup_empty_dirs(root_dir).await {
            tracing::warn!(target: "reverter", "Failed to fully cleanup dirs: {:?}", e);
        }
    }

    Ok(())
}
