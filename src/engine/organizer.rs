use std::{path::{Path, PathBuf}, sync::Arc};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use futures::stream::{FuturesUnordered, StreamExt};

use crate::{
    conflict_resolver::resolve_conflict, 
    errors::{FileOrganizerError, Result}, 
    file_mover::FileMover, 
    hasher::{create_hasher, FileHasher, HashAlgo}, 
    index::Db, 
    metadata::FileCategory, 
    path_builder::PathBuilder, 
    registry::ClassifierRegistry, 
    scanner::{RawFileMetadata, Scanner, ScannerExt}, 
    utils::{create_classifier_registry, default_db_path, make_progress}
};

/// Organize files in `root_dir` asynchronously and efficiently.
pub async fn organise_files(
    root_dir: &Path,
    dry_run: bool
) -> Result<()> {
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
    
    let db_path = if dry_run {
        PathBuf::from(":memory:")
    } else {
        let path = default_db_path().await?;
        tracing::debug!(target: "organizer", "Using database path: {:?}", path);
        if let Some(parent) = path.parent() {
            tracing::debug!(target: "organizer", "Database directory exists: {}", parent.exists());
        }
        path
    };

    let db = Arc::new(Db::new(&db_path).await?);
    let registry = Arc::new(create_classifier_registry());
    let mover = Arc::new(FileMover::new());
    let hasher = create_hasher(HashAlgo::Blake3);

    let files = scan_files(root_dir).await?;
    
    // Process files with concurrency control
    process_files_concurrently(files, db.clone(), registry, mover, hasher, root_dir, dry_run).await?;
    
    // Commit DB checkpoint once all files are processed
    db.save().await?;
    
    Ok(())
}

/// Scans only top-level files from the root directory (ignores subdirs)
async fn scan_files(root_dir: &Path) -> Result<Vec<RawFileMetadata>> {
    let root_dir = root_dir.to_path_buf();
    
    let result = tokio::task::spawn_blocking(move || {
        Scanner::new(root_dir.clone(), Default::default())
            .filter_ok()
            .filter(|raw| {
                // Keep only files directly under `root_dir`
                raw.path.is_file() &&
                raw.path.parent() == Some(&root_dir)
            })
            .collect::<Vec<_>>()
    })
    .await?;
    
    Ok(result)
}

/// Processes files concurrently with a semaphore for rate limiting
async fn process_files_concurrently(
    files: Vec<RawFileMetadata>,
    db: Arc<Db>,
    registry: Arc<ClassifierRegistry>,
    mover: Arc<FileMover>,
    hasher: Arc<dyn FileHasher + Send + Sync>,
    root_dir: &Path,
    dry_run: bool
) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(32)); // Max concurrent files
    let mut tasks = FuturesUnordered::new();

    let root_dir = root_dir.to_path_buf();

    let total = files.len();
    let label = if dry_run { "Organizing (dry-run)" } else { "Organizing" };
    let pb = make_progress( total as u64, label);

    for raw_file in files {
        let permit = semaphore.clone().acquire_owned().await.map_err(|e| {
            // Convert AcquireError to your error type
            crate::errors::FileOrganizerError::from(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to acquire semaphore: {}", e),
            ))
        })?;
        
        let registry_clone = registry.clone();
        let mover_clone = mover.clone();
        let hasher_clone = hasher.clone();
        let root_dir_clone = root_dir.clone();
        let pb_clone = pb.clone();

        tasks.push(tokio::spawn(async move {
            let res = process_file(
                raw_file,
                registry_clone,
                mover_clone,
                hasher_clone,
                &root_dir_clone,
                permit,
                dry_run,
            ).await;

            pb_clone.inc(1);
            res
        }));
    }

    let mut results = Vec::new();

    // Await all tasks and propagate errors
    while let Some(join_res) = tasks.next().await {
        match join_res {
            Ok(Ok(Some(entry))) => results.push(entry),
            Ok(Ok(_)) => {}
            Ok(Err(e)) => return Err(e),
            Err(join_err) => {
                pb.finish_and_clear();
                return Err(FileOrganizerError::from(join_err));
            }
        }
    }

    if dry_run {
        for (raw, category, dest, _) in &results {
            println!("Would move {:?} (category: {}) → {:?}", raw.path, category, dest);
       } 
    } else {
        db.update_files_batch(&results).await?;
    }

    let summary = if dry_run {
        format!("✅ Dry-run completed: {} files analyzed, {} planned moves", total, results.len())
    } else {
        format!("✅ Organize completed: {} files processed", total)
    };
    pb.finish_with_message(summary.clone());

    if dry_run {
        tracing::info!(target: "organizer", "Dry-run completed with {} files analyzed", total);
    } else {
        tracing::info!(target: "organizer", "Organize completed with {} files processed", total);
    }

    Ok(())
}

/// Process a single file: classify → resolve conflicts → move → update DB
async fn process_file(
    raw: RawFileMetadata,
    registry: Arc<ClassifierRegistry>,
    mover: Arc<FileMover>,
    hasher: Arc<dyn FileHasher + Send + Sync>,
    root_dir: &Path,
    _permit: OwnedSemaphorePermit,
    dry_run: bool,
) -> Result<Option<(RawFileMetadata, String, PathBuf, String)>> {
    let classified = registry.classify(&raw).await?;
    let mut destination = PathBuilder::new(&classified)
        .base(&root_dir.join("Organized"))
        .build();

    destination.push(raw.path.file_name().unwrap());

    if dry_run {
        tracing::info!(target: "organizer", "Would move {:?} to {:?}", raw.path, destination);
        return Ok(Some(
            (raw, classified.category.to_string(), destination, "dry-run".into())
        ));
    }

    let entry = handle_file_movement(raw, &classified.category, destination, mover, hasher).await?;
    Ok(Some(entry))
}

/// Checks if a file should be skipped (unchanged since last processing)
#[allow(dead_code)]
async fn should_skip_file(raw: &RawFileMetadata, db: &Db) -> Result<bool> {
    if let Some(existing) = db.lookup(&raw.path).await? {
        if !raw.is_newer_than(&existing) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Handles file movement with conflict resolution
async fn handle_file_movement(
    raw: RawFileMetadata,
    category: &FileCategory,
    destination: PathBuf,
    mover: Arc<FileMover>,
    hasher: Arc<dyn FileHasher + Send + Sync>,
) -> Result<(RawFileMetadata, String, PathBuf, String)> {
    let source_hash = hex::encode(hasher.hash_file(&raw.path).await?);
    let category_str = category.to_string();

    let destination_exists = tokio::fs::try_exists(&destination).await?;

    if !destination_exists {
        mover.move_file(&raw.path, &destination).await?;
        Ok((raw, category_str, destination, source_hash))
    } else {
        let dest_hash = hex::encode(hasher.hash_file(&destination).await?);

        if source_hash == dest_hash {
            tracing::debug!("Skipping identical file: {:?}", raw.path);
            Ok((raw, category_str, destination, source_hash))
        } else {
            let resolved_path = resolve_conflict(&destination, false).await?;
            mover.move_file(&raw.path, &resolved_path).await?;
            Ok((raw, category_str, resolved_path, source_hash))
        }
    }
}
/// Handles file conflicts by comparing hashes and resolving 
#[allow(dead_code)]
async fn handle_conflict(
    raw: RawFileMetadata,
    category: &FileCategory,
    destination: PathBuf,
    mover: Arc<FileMover>,
    hasher: Arc<dyn FileHasher + Send + Sync>,
    source_hash: String,
    db: Arc<Db>,
) -> Result<(RawFileMetadata, String, PathBuf, String)> {
    let destination_hash = get_destination_hash(&destination, &db, &hasher).await?;
    let category_str = category.to_string();

    if source_hash == destination_hash {
        Ok((raw, category_str, destination, source_hash))
    } else {
        let resolved_path = resolve_conflict(&destination, false).await?;
        mover.move_file(&raw.path, &resolved_path).await?;
        Ok((raw, category_str, resolved_path, source_hash))
    }
}

/// Gets the hash of the destination file, checking database first for optimization
#[allow(dead_code)]
async fn get_destination_hash(
    destination: &std::path::PathBuf,
    db: &Db,
    hasher: &Arc<dyn FileHasher + Send + Sync>,
) -> Result<String> {
    if let Some(metadata) = db.lookup_full(destination).await? {
        if let Some(h) = metadata.hash {
            return Ok(h);
        }
    }

    Ok(hex::encode(hasher.hash_file(destination).await?))
}