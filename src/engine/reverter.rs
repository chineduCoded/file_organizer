use std::{collections::HashSet, path::Path, sync::Arc};
use crate::{
    conflict_resolver::resolve_conflict,
    index::{Db, DbFileEntry},
    file_mover::FileMover,
    errors::Result,
    utils::default_db_path,
};

/// Reverts previously organized files back to their original locations.
pub async fn revert_files(root_dir: &Path) -> Result<()> {
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

    // Commit all changes at once
    tx.commit().await?;
    tracing::info!("Revert completed with {} files processed.", files.len());

    Ok(())
}
