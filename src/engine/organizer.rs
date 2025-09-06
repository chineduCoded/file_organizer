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
    utils::{create_classifier_registry, default_db_path}
};

/// Organize files in `root_dir` asynchronously and efficiently.
pub async fn organise_files(
    root_dir: &Path,
    dry_run: bool
) -> Result<()> {
    let db_path = if dry_run {
        PathBuf::from(":memory:")
    } else {
        default_db_path()
    };

    let db = Arc::new(Db::new(&db_path).await?);
    let registry = Arc::new(create_classifier_registry());
    let mover = Arc::new(FileMover::new());
    let hasher = create_hasher(HashAlgo::Blake3);

    let files = scan_files(root_dir).await?;
    
    // Process files with concurrency control
    process_files_concurrently(files, db.clone(), registry, mover, hasher, dry_run).await?;
    
    // Commit DB checkpoint once all files are processed
    db.save().await?;
    
    Ok(())
}

/// Scans files from the root directory using a blocking task
async fn scan_files(root_dir: &Path) -> Result<Vec<RawFileMetadata>> {
    let root_dir = root_dir.to_path_buf();
    
    let result = tokio::task::spawn_blocking(move || {
        Scanner::new(root_dir, Default::default())
            .filter_ok()
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
    dry_run: bool
) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(32)); // Max concurrent files
    let mut tasks = FuturesUnordered::new();

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

        tasks.push(tokio::spawn(async move {
            process_file(
                raw_file,
                registry_clone,
                mover_clone,
                hasher_clone,
                permit,
                dry_run,
            ).await
        }));
    }

    let mut results = Vec::new();

    // Await all tasks and propagate errors
    while let Some(join_res) = tasks.next().await {
        match join_res {
            Ok(Ok(Some(entry))) => results.push(entry),
            Ok(Ok(None)) => {}
            Ok(Err(e)) => return Err(e),
            Err(join_err) => {
                return Err(FileOrganizerError::from(join_err));
            }
        }
    }

    db.update_files_batch(&results).await?;

    Ok(())
}

/// Process a single file: classify → resolve conflicts → move → update DB
async fn process_file(
    raw: RawFileMetadata,
    registry: Arc<ClassifierRegistry>,
    mover: Arc<FileMover>,
    hasher: Arc<dyn FileHasher + Send + Sync>,
    _permit: OwnedSemaphorePermit,
    dry_run: bool,
) -> Result<Option<(RawFileMetadata, String, PathBuf, String)>> {
    // if should_skip_file(&raw, &db).await? {
    //     tracing::debug!("Skipping unchanged file: {:?}", raw.path);
    //     return Ok(());
    // }

    let classified = registry.classify(&raw).await?;
    let destination = PathBuilder::new(&classified).build();

    if dry_run {
        tracing::info!("Would move {:?} to {:?}", raw.path, destination);
        return Ok(None);
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
        // ✅ Destination doesn't exist → move file
        mover.move_file(&raw.path, &destination).await?;
        Ok((raw, category_str, destination, source_hash))
    } else {
        // ⚡ Conflict resolution
        let resolved_path = resolve_conflict(&destination, false).await?;
        mover.move_file(&raw.path, &resolved_path).await?;
        Ok((raw, category_str, resolved_path, source_hash))
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
    db: Arc<Db>, // still needed for lookup optimization
) -> Result<(RawFileMetadata, String, PathBuf, String)> {
    let destination_hash = get_destination_hash(&destination, &db, &hasher).await?;
    let category_str = category.to_string();

    if source_hash == destination_hash {
        // ✅ Files are identical → no move, just return DB entry
        Ok((raw, category_str, destination, source_hash))
    } else {
        // ⚡ Files differ → resolve conflict + move
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