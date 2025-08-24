use std::{io, path::PathBuf, process::{ExitCode, Termination}};
use thiserror::Error;

use crate::utils::{humanize, map_exit_code};

pub type Result<T, E = FileOrganizerError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum FileOrganizerError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Config error: {0}")]
    Config(#[from] anyhow::Error),

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

    #[error("No matching rule: {0}")]
    NoMatchingRule(String),

    #[error("Invalid rule: {0}")]
    InvalidRule(String),

    #[error("JSON error at {path}: {source}")]
    Json { path: PathBuf, source: serde_json::Error },

    #[error("Regex error on`{pattern}`: {source}")]
    Regex { pattern: String, source: regex::Error },
}

impl Termination for FileOrganizerError {
    fn report(self) -> ExitCode {
        eprintln!("{}", humanize(&self));
        ExitCode::from(map_exit_code(&self))
    }
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