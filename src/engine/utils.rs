use crate::errors::FileOrganizerError;

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
    }
}