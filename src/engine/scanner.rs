use std::fs::Permissions;
use std::io;
use std::path::PathBuf;
use std::time::SystemTime;
use walkdir::{DirEntry, WalkDir};

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
pub struct FileMetadata {
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

pub struct Scanner {
    inner: walkdir::IntoIter,
    config: ScanConfig,
}

impl Scanner {
    pub fn new<P: Into<PathBuf>>(root: P, config: ScanConfig) -> Self {
        let walker = WalkDir::new(root.into())
            .max_depth(config.max_depth)
            .follow_links(config.follow_symlinks);
        Self {
            inner: walker.into_iter(),
            config,
        }
    }

    fn process_entry(&self, entry: &DirEntry) -> io::Result<FileMetadata> {
        let metadata = entry.path().symlink_metadata()?;

        // filter hidden
        if !self.config.include_hidden && is_hidden(entry) {
            return Err(io::Error::new(io::ErrorKind::Other, "Hidden file skipped"));
        }

        // filter dirs
        if metadata.is_dir() && !self.config.include_dirs {
            return Err(io::Error::new(io::ErrorKind::Other, "Dir skipped"));
        }

        // filter extensions
        if let Some(ref exts) = self.config.allowed_extensions {
            if let Some(ext) = entry.path().extension().and_then(|e| e.to_str()) {
                if !exts.iter().any(|x| x.eq_ignore_ascii_case(ext)) {
                    return Err(io::Error::new(io::ErrorKind::Other, "Extension skipped"));
                }
            }
        }

        // filter size
        if metadata.is_file() {
            let size = metadata.len();
            if let Some(min) = self.config.min_size {
                if size < min {
                    return Err(io::Error::new(io::ErrorKind::Other, "Too small"));
                }
            }
            if let Some(max) = self.config.max_size {
                if size > max {
                    return Err(io::Error::new(io::ErrorKind::Other, "Too large"));
                }
            }
        }

        Ok(FileMetadata {
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
    type Item = io::Result<FileMetadata>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(entry) = self.inner.next() {
            match entry {
                Ok(e) => return Some(self.process_entry(&e)),
                Err(err) => return Some(Err(io::Error::new(io::ErrorKind::Other, err))),
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

/// Extension trait for filtering scanner results
pub trait ScannerExt: Iterator<Item = io::Result<FileMetadata>> + Sized {
    /// Keep only successful entries, discarding errors
    fn filter_ok(self) -> Box<dyn Iterator<Item = FileMetadata>>;
    /// Keep only errors (for logging/debugging)
    fn filter_errs(self) -> Box<dyn Iterator<Item = io::Error>>;
}

impl<I> ScannerExt for I
where
    I: Iterator<Item = io::Result<FileMetadata>> + 'static,
{
    fn filter_ok(self) -> Box<dyn Iterator<Item = FileMetadata>> {
        Box::new(self.filter_map(|res| res.ok()))
    }

    fn filter_errs(self) -> Box<dyn Iterator<Item = io::Error>> {
        Box::new(self.filter_map(|res| res.err()))
    }
}