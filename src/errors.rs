use std::{io, path::PathBuf};
use thiserror::Error;

pub type Result<T, E = FileOrganizerError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum FileOrganizerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Indexing error: {0}")]
    Index(String),

    #[error("Moving error: {0}")]
    Move(String),

    #[error("Scanning error: {0}")]
    Scan(String),

    #[error("Watching error: {0}")]
    Watch(String),

    #[error("Invalid path: {0}")]
    InvalidPath(PathBuf),

    #[error("Classify error: {0}")]
    Classify(String),
}

/// Fine-grained, per-file outcome (never aborts the whole run).
#[derive(Debug)]
pub enum FileOutcome {
    Ok(FileReport),
    Err(FileErrorReport)
}

#[derive(Debug)]
pub struct FileReport {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub action: MoveAction,
}

#[derive(Debug)]
pub enum MoveAction {
    Moved,
    Skipped,
    Renamed(PathBuf),
}

#[derive(Debug)]
pub struct FileErrorReport {
    pub path: PathBuf,
    pub stage: &'static str, // "scan" | "classify" | "move" | "index"
    pub error: FileOrganizerError,
}