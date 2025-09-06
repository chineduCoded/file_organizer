use std::{sync::Arc, time::SystemTime, path::PathBuf};

use chrono::{DateTime, Utc, Datelike};
use tracing_subscriber::{fmt, EnvFilter};
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
    video_classifier::VideoClassifier};

pub fn init_tracing() {
    // Example: export RUST_LOG="info,file_organizer=debug"
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info")); 

    fmt()
        .with_env_filter(filter)
        .with_target(false) // hide target module path
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .with_timer(fmt::time::LocalTime::rfc_3339()) // timestamp
        .compact()
        .init();
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