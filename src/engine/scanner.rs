use std::fs::Permissions;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;
use walkdir::{DirEntry, WalkDir};

use crate::errors::{FileOrganizerError, Result, SkipReason};

#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub include_hidden: bool,
    pub include_dirs: bool,
    pub max_depth: usize,
    pub allowed_extensions: Option<Vec<String>>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub follow_symlinks: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            include_hidden: false,
            include_dirs: false,
            max_depth: usize::MAX,
            allowed_extensions: None,
            min_size: None,
            max_size: None,
            follow_symlinks: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawFileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub permissions: Permissions,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
}

impl RawFileMetadata {
    pub fn is_newer_than(&self, other: &RawFileMetadata) -> bool {
        // 1. File does not exist in DB
        if other.path.as_os_str().is_empty() {
            return true;
        }

        // 2. Size changed
        if self.size != other.size {
            return true;
        }

        // 3. Modified time is newer
        match (self.modified, other.modified) {
            (Some(m1), Some(m2)) if m1 > m2 => return true,
            _ => {}
        }

        // 4. If size + modified same, we assume unchanged
        // (Only hash if explicitly forced in conflict resolution)
        false
    }
}


pub struct Scanner {
    inner: walkdir::IntoIter,
    config: ScanConfig,
}

impl Scanner {
    pub fn new<P: Into<PathBuf>>(root: P, mut config: ScanConfig) -> Self {
        // Normalize allowed extensions to lowercase
        if let Some(ref mut exts) = config.allowed_extensions {
            *exts = exts.iter().map(|e| e.to_lowercase()).collect();
        }

        let walker = WalkDir::new(root.into())
            .max_depth(config.max_depth)
            .follow_links(config.follow_symlinks);

        Self {
            inner: walker.into_iter(),
            config,
        }
    }

    fn process_entry(&self, entry: &DirEntry) -> Result<RawFileMetadata> {
        // hidden
        if !self.config.include_hidden && is_hidden(entry) {
            return Err(FileOrganizerError::Skipped(SkipReason::Hidden));
        }

        let metadata = entry.metadata().map_err(|_| FileOrganizerError::Skipped(SkipReason::MetadataUnreadable))?;

        // skip dirs
        if metadata.is_dir() && !self.config.include_dirs {
            return Err(FileOrganizerError::Skipped(SkipReason::IsDir));
        }

        // skip by extension
        if let Some(ref exts) = self.config.allowed_extensions {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                if !exts.contains(&ext.to_lowercase()) {
                    return Err(FileOrganizerError::Skipped(SkipReason::WrongExtension));
                }
            }
        }

        // size filtering
        if metadata.is_file() {
            let size = metadata.len();
            if let Some(min) = self.config.min_size {
                if size < min {
                    return Err(FileOrganizerError::Skipped(SkipReason::TooSmall));
                }
            }
            if let Some(max) = self.config.max_size {
                if size > max {
                    return Err(FileOrganizerError::Skipped(SkipReason::TooLarge));
                }
            }
        }

        Ok(RawFileMetadata {
            path: entry.path().to_path_buf(),
            size: metadata.len(),
            created: metadata.created().ok(),
            modified: metadata.modified().ok(),
            accessed: metadata.accessed().ok(),
            permissions: metadata.permissions(),
            is_file: metadata.is_file(),
            is_dir: metadata.is_dir(),
            is_symlink: metadata.file_type().is_symlink(),
        })
    }
}

impl Iterator for Scanner {
    type Item = Result<RawFileMetadata>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(entry) = self.inner.next() {
            match entry {
                Ok(e) => return Some(self.process_entry(&e)),
                Err(err) => return Some(Err(FileOrganizerError::Io(io::Error::new(io::ErrorKind::Other, err)))),
            }
        }
        None
    }
}

/// UNIX hidden detection (dotfiles)
#[cfg(unix)]
fn is_hidden(entry: &DirEntry) -> bool {
    entry.file_name().to_string_lossy().starts_with('.')
}

/// Windows hidden detection (dotfile OR FILE_ATTRIBUTE_HIDDEN)
#[cfg(windows)]
fn is_hidden(entry: &DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

    if entry.file_name().to_string_lossy().starts_with('.') {
        return true;
    }
    if let Ok(metadata) = entry.metadata() {
        return (metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN) != 0;
    }
    false
}

/// Extension trait for filtering results
pub trait ScannerExt: Iterator<Item = Result<RawFileMetadata>> + Sized {
    fn filter_ok(self) -> impl Iterator<Item = RawFileMetadata>;
    fn filter_skipped(self) -> impl Iterator<Item = SkipReason>;
    fn filter_err(self) -> impl Iterator<Item = io::Error>;
}

impl<I> ScannerExt for I
where
    I: Iterator<Item = Result<RawFileMetadata>>,
{
    fn filter_ok(self) -> impl Iterator<Item = RawFileMetadata> {
        self.filter_map(|res| match res {
            Ok(file) => Some(file),
            _ => None,
        })
    }

    fn filter_skipped(self) -> impl Iterator<Item = SkipReason> {
        self.filter_map(|res| match res {
            Err(FileOrganizerError::Skipped(reason)) => Some(reason),
            _ => None,
        })
    }

    fn filter_err(self) -> impl Iterator<Item = io::Error> {
        self.filter_map(|res| match res {
            Err(FileOrganizerError::Io(err)) => Some(err),
            _ => None,
        })
    }
}
