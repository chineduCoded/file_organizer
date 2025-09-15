use tempfile::tempdir;
use tokio::fs;

use stash::{
    reverter::{cleanup_empty_dirs, validate_dir, should_skip_file},
    hasher::{create_hasher, HashAlgo},
};

#[tokio::test]
async fn test_validate_dir_success() {
    let dir = tempdir().unwrap();
    let path = dir.path();

    let res = validate_dir(path).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_validate_dir_not_found() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("nonexistent");

    let res = validate_dir(&path).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_validate_dir_not_a_directory() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("file.txt");
    fs::write(&file_path, b"hello").await.unwrap();

    let res = validate_dir(&file_path).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_cleanup_empty_dirs_removes_nested() {
    let dir = tempdir().unwrap();
    let nested = dir.path().join("a/b/c");
    fs::create_dir_all(&nested).await.unwrap();

    cleanup_empty_dirs(dir.path()).await.unwrap();

    // root still exists
    assert!(dir.path().exists());
    // nested dirs removed
    assert!(!nested.exists());
}

#[tokio::test]
async fn test_should_skip_file_identical() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let original = dir.path().join("original.txt");

    fs::write(&source, b"same").await.unwrap();
    fs::write(&original, b"same").await.unwrap();

    let hasher = create_hasher(HashAlgo::Blake3);
    let pb = indicatif::ProgressBar::hidden();

    let result = should_skip_file(&source, &original, hasher, &pb).await.unwrap();
    assert!(result);
}

#[tokio::test]
async fn test_should_skip_file_different() {
    let dir = tempdir().unwrap();
    let source = dir.path().join("source.txt");
    let original = dir.path().join("original.txt");

    fs::write(&source, b"foo").await.unwrap();
    fs::write(&original, b"bar").await.unwrap();

    let hasher = create_hasher(HashAlgo::Blake3);
    let pb = indicatif::ProgressBar::hidden();

    let result = should_skip_file(&source, &original, hasher, &pb).await.unwrap();
    assert!(!result);
}

// NOTE: Full integration test for `revert_files` requires inserting entries in the DB
// and simulating moved files. That’s heavier and best tested in integration tests
// with a mock DB or actual SQLite instance.
