use std::{path::PathBuf, sync::Arc, time::{Duration, SystemTime}};

use chrono::{DateTime, Utc, Datelike};
use indicatif::{ProgressBar, ProgressStyle};
use tracing_subscriber::{fmt, EnvFilter, prelude::*};
use tracing_appender::rolling;
use dirs::data_local_dir;

use crate::{
    archive_classifier::ArchiveClassifier, 
    audio_classifier::AudioClassifier, 
    code_classifier::CodeClassifier, 
    docs_classifier::DocumentClassifier,
    errors::FileOrganizerError, 
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

pub fn default_db_path() -> PathBuf {
    let mut path = data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("file_organizer");
    std::fs::create_dir_all(&path).ok();
    path.push("file_organizer.db");

    path
}

pub fn detect_mime(ext: &str) -> String {
    let mime = mime_guess::from_ext(ext).first_or_octet_stream();
    mime.essence_str().to_string()
}

pub fn system_time_to_year(t: SystemTime) -> Option<i32> {
    let datetime: DateTime<Utc> = t.into();
    Some(datetime.year())
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
        other => other.to_string(), // fall back to #[error(..)]
    }
}