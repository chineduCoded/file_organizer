use tracing_subscriber::fmt;

use crate::errors::FileOrganizerError;

pub fn init_tracing() {
    let _ = fmt::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_file(true)
        .with_line_number(true)
        .compact()
        .try_init();
}

pub fn humanize(e: &FileOrganizerError) -> String {
    use FileOrganizerError::*;
    match e {
        Io(err) => format!("I/O error: {}", err),
        Config(msg) => format!("Configuration error: {}", msg),
        Index(msg) => format!("Indexing error: {}", msg),
        Move(msg) => format!("Moving error: {}", msg),
        Scan(msg) => format!("Scanning error: {}", msg),
        Watch(msg) => format!("Watching error: {}", msg),
        InvalidPath(path) => format!("Invalid path: {}", path.display()),
        Classify(msg) => format!("Classify error: {}", msg),
        NoMatchingRule(msg) => format!("No matching rule error: {}", msg),
        Json { path, source } => {
            format!("JSON error in {}: {}", path.display(), source)
        }
        Regex { pattern, source } => {
            format!("Regex error in pattern `{}`: {}", pattern, source)
        }
        InvalidRule(msg) => format!("Invalid rule: {}", msg),
        MimeDetection(msg) => format!("MIME detection error: {}", msg),
    }
}

pub fn map_exit_code(e: &FileOrganizerError) -> u8 {
    use FileOrganizerError::*;
    match e {
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
        InvalidRule(_) => 13,
        MimeDetection(_) => 14
    }
}

