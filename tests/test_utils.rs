use std::{collections::HashMap, os::unix::fs::PermissionsExt, path::{Path, PathBuf}};

use stash::{metadata::{ClassifiedFileMetadata, FileCategory}, scanner::RawFileMetadata};
use tempfile::TempDir;

// =============== Classifier Registry Tests ===============

// Helper function to create a RawFileMetadata for testing

#[allow(dead_code)]
pub fn create_test_file(path: &str, size: u64) -> RawFileMetadata {
    RawFileMetadata {
        path: PathBuf::from(path),
        size,
        created: None,
        modified: None,
        accessed: None,
        permissions: std::fs::Permissions::from_mode(0o644),
        is_file: true,
        is_dir: false,
        is_symlink: false,   
    }
}

/// Helper: create a dummy file with the given extension
#[allow(dead_code)]
pub fn create_test_file_with_ext(ext: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(format!("testfile.{}", ext));
    std::fs::write(&path, b"dummy content").unwrap();
    (dir, path)
}

/// Helper: create ClassifiedFileMetadata for a given path
#[allow(dead_code)]
pub fn create_test_metadata(path: &Path) -> ClassifiedFileMetadata {
    let mime_type = match path.extension().and_then(|ext| ext.to_str()) {
        Some("txt") => "text/plain",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        _ => "application/octet-stream",
    };

    ClassifiedFileMetadata {
        path: path.to_path_buf(),
        category: FileCategory::Others,
        year: None,
        created_date: None,
        modified_date: None,
        file_size: Some(0),
        mime_type: Some(mime_type.into()),
        extra: HashMap::new(),
    }
}
    