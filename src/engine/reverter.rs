use std::{collections::HashSet, path::Path, sync::Arc};
use tokio::fs;

use crate::{
    conflict_resolver::resolve_conflict, errors::{FileOrganizerError, Result}, file_mover::FileMover, hasher::{create_hasher, FileHasher, HashAlgo}, index::{Db, DbFileEntry}, utils::{default_db_path, make_progress}
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
pub async fn revert_files(
    root_dir: &Path, 
    cleanup: bool,
) -> Result<()> {
    validate_dir(&root_dir).await?;

    let db_path = default_db_path().await?; 
    let db = Arc::new(Db::new(&db_path).await?);
    let mover = Arc::new(FileMover::new());
    let hasher = create_hasher(HashAlgo::Blake3);

    // Deduplicate by dest_path
    let mut seen = HashSet::new();
    let files: Vec<DbFileEntry> = db.get_all_files()
        .await?
        .into_iter()
        .filter(|f| f.dest_path.starts_with(root_dir)) // Only revert inside root_dir
        .filter(|f| seen.insert(f.dest_path.clone()))
        .collect();

    let total = files.len();
    let pb = make_progress(total as u64, "Reverting");

    let mut moved: usize = 0;

    for file in &files {
        let source = file.dest_path.clone();
        let original = file.path.clone();

        if !tokio::fs::try_exists(&source).await? {
            tracing::warn!("Missing file at destination, skipping: {:?}", source);
            pb.inc(1);
            continue;
        }

        if source == original {
            tracing::debug!("Already at original path, skipping: {:?}", source);
            pb.inc(1);
            continue;
        }

        if should_skip_file(&source, &original, hasher.clone(), &pb).await? {
            continue;
        }

        // If original already exists, resolve conflict
        let final_path = if tokio::fs::try_exists(&original).await? {
            resolve_conflict(&original, true).await?
        } else {
            original
        };

        if let Some(parent) = final_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Move file back
        mover.move_file(&source, &final_path).await?;
        tracing::debug!(target: "reverter", "Reverted {:?} -> {:?}", source, final_path);

        let mut tx = db.begin().await?;
        db.update_dest_path_tx(&mut tx, &file.path, &final_path).await?;
        tx.commit().await?;

        moved += 1;
        pb.inc(1);
    }

    pb.finish_with_message(format!(
        "♻️ Revert completed: {} moved, {} skipped, {} candidates.",
        moved,
        total - moved,
        total
    ));
    tracing::info!(
        target: "reverter", "Revert completed: {} moved, {} skipped, {} candidates.", 
        moved, 
        total - moved,
        total
    );

    if cleanup {
        if let Err(e) = cleanup_empty_dirs(root_dir).await {
            tracing::warn!(target: "reverter", "Failed to fully cleanup dirs: {:?}", e);
        }
    }

    Ok(())
}

/// Checks if a directory exists and is valid.
pub async fn validate_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err(FileOrganizerError::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Directory {:?} does not exist", path),
        )));
    }

    if !path.is_dir() {
        return Err(FileOrganizerError::from(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Path {:?} is not a directory", path),
        )));
    }

    Ok(())
}

/// Checks if source and original files are identical based on their hashes.
/// Returns true if the file should be skipped (identical).
pub async fn should_skip_file(
    source: &Path,
    original: &Path,
    hasher: Arc<dyn FileHasher + Send + Sync>,
    pb: &indicatif::ProgressBar,
) -> Result<bool> {
    if !tokio::fs::try_exists(original).await? {
        return Ok(false);
    }

    let source_hash = hex::encode(hasher.hash_file(source).await?);
    let original_hash = hex::encode(hasher.hash_file(original).await?);

    if source_hash == original_hash {
        tracing::debug!("Skipping identical file: {:?}", source);
        pb.inc(1);
        Ok(true)
    } else {
        Ok(false)
    }
}

