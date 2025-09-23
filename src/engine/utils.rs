use std::{path::PathBuf, sync::Arc, time::{Duration, SystemTime, UNIX_EPOCH}};

use chrono::{DateTime, Utc, Datelike};
use indicatif::{ProgressBar, ProgressStyle};
use tracing_subscriber::{fmt, EnvFilter, prelude::*};
use tracing_appender::rolling;

use crate::{
    archive_classifier::ArchiveClassifier, 
    audio_classifier::AudioClassifier, 
    code_classifier::CodeClassifier, 
    docs_classifier::DocumentClassifier,
    errors::{FileOrganizerError, Result}, 
    executable_classifier::ExecutableClassifier, 
    generic::GenericClassifier, 
    image_classifier::ImageClassifier, 
    registry::ClassifierRegistry, 
    video_classifier::VideoClassifier
};

/// Initialize tracing
/// - Console: clean progress & summary (no spammy per-file logs)
/// - File: detailed DEBUG logs for all operations
pub fn init_tracing() {
    let file_appender = rolling::daily("logs", "file_organizer.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    Box::leak(Box::new(guard));

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_timer(fmt::time::UtcTime::rfc_3339())
        .with_filter(EnvFilter::new("debug"));

    let console_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_timer(fmt::time::LocalTime::rfc_3339())
        .compact()
        .with_filter(EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(file_layer)
        .with(console_layer)
        .init();
}

/// Create a styled progress bar
pub fn make_progress(total: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.magenta/bright_magenta}] {pos}/{len} {msg}")
            .unwrap()
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(200));
    pb
}

pub async fn default_db_path() -> Result<PathBuf> {
    // Candidate directories in order of preference
    let candidates = [
        dirs::data_local_dir(),  // Best: platform-specific writable data dir
        dirs::home_dir(), // Good: home dir
        Some(std::env::temp_dir()), // Last resort: temp directory
    ];

    for candidate in candidates.iter().flatten() {
        let path = candidate.join("file_organizer");
        // Ensure directory exists
        if let Err(e) = tokio::fs::create_dir_all(&path).await {
            tracing::debug!("Failed to create directory {:?}: {}", path, e);
            continue; // try next fallback
        }

        // Test writability safely
        let test_file = path.join(".write_test_tmp");
        match tokio::fs::File::create(&test_file).await {
            Ok(_) => {
                let _ = tokio::fs::remove_file(&test_file); // clean up
                let db_path = path.join("file_organizer.db");
                tracing::debug!("Using database path: {:?}", db_path);
                return Ok(db_path);
            }
            Err(e) => {
                tracing::debug!("Directory {:?} not writable: {}", path, e);
                continue; // try next fallback
            }
        }

    }

    // All fallbacks failed
    Err(FileOrganizerError::Io(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "No writable directory found for the database (tried data_local, home, temp dir)",
    )))
}

/// Expands `~` and environment variables in paths, then returns an absolute path.
pub fn expand_tilde<P: AsRef<str>>(path: P) -> PathBuf {
    // Expand tilde (~) to home directory
    let expanded = shellexpand::tilde(path.as_ref());

    // Expand any environment variables, e.g., $HOME or %USERPROFILE%
    let expanded_env = shellexpand::env(&expanded).unwrap_or(expanded.clone());

    let mut path_buf = PathBuf::from(expanded_env.to_string());

    // If relative, make it absolute relative to current working dir
    if !path_buf.is_absolute() {
        if let Ok(current_dir) = std::env::current_dir() {
            path_buf = current_dir.join(path_buf);
        }
    }

    path_buf
}


pub fn detect_mime(ext: &str) -> String {
    let mime = mime_guess::from_ext(ext).first_or_octet_stream();
    mime.essence_str().to_string()
}

/// Extract UTC year from SystemTime safely.
pub fn system_time_to_year(t: SystemTime) -> Option<i32> {
    let datetime: DateTime<Utc> = t.into();
    Some(datetime.year())
}

/// Convert SystemTime → Option<i64> (seconds since epoch).
/// Returns None if pre-1970 or overflow.
pub fn to_unix(ts: Option<SystemTime>) -> Option<i64> {
    ts.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
      .map(|d| d.as_secs() as i64)
}

/// Convert i64 (from DB) → Option<SystemTime>.
/// Returns None if ts < 0 (pre-1970).
pub fn from_unix(ts: Option<i64>) -> Option<SystemTime> {
    ts.and_then(|s| {
        if s < 0 {
            None
        } else {
            Some(UNIX_EPOCH + Duration::from_secs(s as u64))
        }
    })
}

/// Creates and configures the classifier registry with priorities
pub fn create_classifier_registry() -> ClassifierRegistry {
    let mut registry = ClassifierRegistry::new();
    // Register classifiers with appropriate base priorities
    // Higher priority = more specific/specialized classifiers
    // Lower priority = more general/fallback classifiers

    // Media classifiers (very specific, high confidence)
    registry.register_with_priority(100, Arc::new(ImageClassifier));
    registry.register_with_priority(95, Arc::new(AudioClassifier));
    registry.register_with_priority(90, Arc::new(VideoClassifier));

    // Document classifier (specific but may overlap with code)
    registry.register_with_priority(85, Arc::new(DocumentClassifier));

    // Code classifier (specific but may overlap with documents/executables)
    registry.register_with_priority(80, Arc::new(CodeClassifier));

    // Archive classifier (specific but may overlap with executables)
    registry.register_with_priority(75, Arc::new(ArchiveClassifier));

    // Executable classifier (broader category, may overlap with others)
    registry.register_with_priority(70, Arc::new(ExecutableClassifier));

    // Generic fallback (lowest priority, handles everything)
    registry.register_with_priority(10, Arc::new(GenericClassifier));

    registry
}

pub fn humanize(e: &FileOrganizerError) -> String {
    match e {
        FileOrganizerError::InvalidPath(path) => format!("Invalid path: {}", path.display()),
        FileOrganizerError::Json { path, source } => {
            format!("JSON error in {}: {}", path.display(), source)
        }
        FileOrganizerError::Regex { pattern, source } => {
            format!("Regex error in `{}`: {}", pattern, source)
        }
        other => other.to_string(),
    }
}