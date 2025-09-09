use sqlx;
use std::{io, path::PathBuf, process::{ExitCode, Termination}};
use serde::Serialize;
use thiserror::Error;
use tokio::task::JoinError;

use crate::utils::humanize;

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

    #[error("MIME detection error: {0}")]
    MimeDetection(String),

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

    #[error("Regex error on `{pattern}`: {source}")]
    Regex { pattern: String, source: regex::Error },

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Skipped: {0}")]
    Skipped(SkipReason),
    #[error("Task join error: {0}")]
    Join(#[from] JoinError),

    #[error("Concurrency error: {0}")]
    Concurrency(String),

    #[error("Other: {0}")]
    Other(String),
}

impl FileOrganizerError {
    pub fn exit_code(&self) -> u8 {
        use FileOrganizerError::*;
        match self {
            Io(_) => 2,
            Config(_) => 3,
            Index(_) => 4,
            Move(_) => 5,
            Scan(_) => 6,
            Watch(_) => 7,
            InvalidPath(_) => 8,
            Classify(_) => 9,
            NoMatchingRule(_) => 10,
            Json { .. } => 11,
            Regex { .. } => 12,
            Database(_) => 13,
            InvalidRule(_) => 14,
            MimeDetection(_) => 15,
            Skipped(_) => 16,
            Join(_) => 17,
            Concurrency(_) => 18,
            Other(_) => 19,
        }
    }
}

impl Termination for FileOrganizerError {
    fn report(self) -> ExitCode {
        eprintln!("{}", humanize(&self));
        ExitCode::from(self.exit_code())
    }
}

impl From<tokio::sync::AcquireError> for FileOrganizerError {
    fn from(err: tokio::sync::AcquireError) -> Self {
        FileOrganizerError::Concurrency(err.to_string())
    }
}

/// Why a file was skipped
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub enum SkipReason {
    Hidden,
    IsDir,
    WrongExtension,
    TooSmall,
    TooLarge,
    MetadataUnreadable,
}

impl SkipReason {
    pub const VARIANTS: [SkipReason; 6] = [
        SkipReason::Hidden,
        SkipReason::IsDir,
        SkipReason::WrongExtension,
        SkipReason::TooSmall,
        SkipReason::TooLarge,
        SkipReason::MetadataUnreadable,
    ];

    #[inline]
    pub fn as_index(&self) -> usize {
        match self {
            SkipReason::Hidden => 0,
            SkipReason::IsDir => 1,
            SkipReason::WrongExtension => 2,
            SkipReason::TooSmall => 3,
            SkipReason::TooLarge => 4,
            SkipReason::MetadataUnreadable => 5,
        }
    }
}

impl std::fmt::Display for SkipReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            SkipReason::Hidden => "File skipped because it is hidden",
            SkipReason::IsDir => "Skipped directory (not scanning dirs)",
            SkipReason::WrongExtension => "File skipped due to unsupported extension",
            SkipReason::TooSmall => "File skipped because it is smaller than minimum size",
            SkipReason::TooLarge => "File skipped because it is larger than maximum size",
            SkipReason::MetadataUnreadable => "File skipped because metadata could not be read",
        };
        write!(f, "{}", msg)
    }
}



