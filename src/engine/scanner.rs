use std::fs::Permissions;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::ffi::OsStr;
use walkdir::{WalkDir, DirEntry, IntoIter};

/// Represents collected metadata for a file system entry.
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub file_name: String,
    pub extension: Option<String>,
    pub size: u64,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub permissions: Permissions,
    pub is_file: bool,
    pub is_dir: bool,
    pub is_symlink: bool,
}

/// Configuration for directory scanning with filters
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub include_hidden: bool,
    pub include_dirs: bool,
    pub max_depth: Option<usize>,
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
            max_depth: None,
            allowed_extensions: None,
            min_size: None,
            max_size: None,
            follow_symlinks: false,
        }
    }
}

/// Scanner struct that implements Iterator for lazy, memory-efficient scanning
pub struct Scanner {
    inner: IntoIter,
    config: ScanConfig,
}

/// Scanner for traversing directories and collecting file metadata.
impl Scanner {
    /// Creates a new Scanner for the given path with configuration
    pub fn new<P: AsRef<Path>>(path: P, config: ScanConfig) -> Self {
        let mut walk = WalkDir::new(path);

        if config.follow_symlinks {
            walk = walk.follow_links(true);
        }

        if let Some(depth) = config.max_depth {
            walk = walk.max_depth(depth);
        }

        Scanner { 
            inner: walk.into_iter(), 
            config 
        }
    }

    /// Create a scanner with default configuration
    pub fn with_defaults<P: AsRef<Path>>(path: P) -> Self {
        Self::new(path, ScanConfig::default())
    }

    fn is_hidden(entry: &DirEntry) -> bool {
        entry.file_name()
            .to_str()
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
    }

    fn is_extension_allowed(&self, ext: Option<&OsStr>) -> bool {
        match &self.config.allowed_extensions {
            Some(allowed) => {
                if let Some(ext_str) = ext.and_then(|e| e.to_str()) {
                    allowed.iter().any(|a| a.eq_ignore_ascii_case(ext_str))
                } else {
                    false
                }
            },
            None => true,
        }
    }

    /// Helper method to process an entry with filtering
    fn process_entry(&self, entry: &DirEntry) -> Result<Option<FileMetadata>, io::Error> {
        let file_type = entry.file_type();
        let metadata = entry.path().symlink_metadata()?;

        if !self.config.include_hidden && Self::is_hidden(entry) {
            return Ok(None);
        }

        if metadata.is_dir() && !self.config.include_dirs {
            return Ok(None);
        }

        if metadata.is_file() {
            let ext = entry.path().extension();
            if !self.is_extension_allowed(ext) {
                return Ok(None);
            }

            let size = metadata.len();
            if let Some(min) = self.config.min_size {
                if size < min {
                    return Ok(None);
                }
            }
            if let Some(max) = self.config.max_size {
                if size > max {
                    return Ok(None);
                }
            }
        }

        let file_metadata = FileMetadata {
            path: entry.path().to_path_buf(),
            file_name: entry.file_name().to_string_lossy().into_owned(),
            extension: entry.path().extension().and_then(|e| e.to_str().map(|s| s.to_owned())),
            size: metadata.len(),
            created: metadata.created().ok(),
            modified: metadata.modified().ok(),
            accessed: metadata.accessed().ok(),
            permissions: metadata.permissions(),
            is_file: file_type.is_file(),
            is_dir: file_type.is_dir(),
            is_symlink: file_type.is_symlink(),
        };

        Ok(Some(file_metadata))
    }
}

impl Iterator for Scanner {
    type Item = FileMetadata;
    
    fn next(&mut self) -> Option<Self::Item> {
       while let Some(entry) = self.inner.next() {
            match entry {
                Ok(e) => match self.process_entry(&e) {
                    Ok(Some(meta)) => return Some(meta),
                    Ok(None) => continue,
                    Err(_) => continue,
                }
                Err(_) => continue,
            }
        }
        None
    }
}

/// Convenience function to create a scanner with default configuration
pub fn scan_dir<P: AsRef<Path>>(path: P) -> Scanner {
    Scanner::with_defaults(path)
}

/// Convenience function to create a scanner with custom configuration
pub fn scan_dir_with_config<P: AsRef<Path>>(path: P, config: ScanConfig) -> Scanner {
    Scanner::new(path, config)
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};
    use std::fs::{self, File};
    use std::io::Write;

    // Helper to create test directory structure
    fn create_test_structure() -> TempDir {
        let dir = tempdir().unwrap();
        let path = dir.path();

        // Create visible file
        File::create(path.join("file1.txt"))
            .unwrap()
            .write_all(b"Hello")
            .unwrap();

        // Create hidden file
        File::create(path.join(".hidden.txt"))
            .unwrap()
            .write_all(b"Hidden")
            .unwrap();

        // Create subdirectory
        let subdir = path.join("subdir");
        fs::create_dir(&subdir).unwrap();

        // Create file in subdirectory
        File::create(subdir.join("file2.rs"))
            .unwrap()
            .write_all(b"pub fn main() {}")
            .unwrap();

        // Create large file
        File::create(path.join("large.bin"))
            .unwrap()
            .write_all(&[0; 1024])
            .unwrap();

        dir
    }

    #[test]
    fn test_default_scan() {
        let temp_dir = create_test_structure();
        let scanner = scan_dir(temp_dir.path());
        let mut results: Vec<_> = scanner.collect();

        // Should only find non-hidden files recursively
        assert_eq!(results.len(), 3);
        results.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        
        let first = &results[0];
        assert_eq!(first.file_name, "file1.txt");
        assert!(first.is_file);
        assert!(!first.is_dir);

        let second = &results[1];
        assert_eq!(second.file_name, "file2.rs");
        assert!(second.is_file);
        assert!(!second.is_dir);

        let third = &results[2];
        assert_eq!(third.file_name, "large.bin");
        assert!(third.is_file);
        assert!(!third.is_dir);
    }

    #[test]
    fn test_non_recursive_scan() {
        let temp_dir = create_test_structure();
        let config = ScanConfig {
            max_depth: Some(1), // restrict to top-level only
            ..Default::default()
        };
        let scanner = scan_dir_with_config(temp_dir.path(), config);
        let mut results: Vec<_> = scanner.collect();

        // Should only find non-hidden files in the current directory (file1.txt, large.bin)
        assert_eq!(results.len(), 2);
        results.sort_by(|a, b| a.file_name.cmp(&b.file_name));

        let first = &results[0];
        assert_eq!(first.file_name, "file1.txt");
        assert!(first.is_file);
        assert!(!first.is_dir);

        let second = &results[1];
        assert_eq!(second.file_name, "large.bin");
        assert!(second.is_file);
        assert!(!second.is_dir);

        // Make sure subdir/file2.rs is not included
        assert!(!results.iter().any(|e| e.file_name == "file2.rs"));
    }


    #[test]
    fn test_include_hidden() {
        let temp_dir = create_test_structure();
        let config = ScanConfig {
            include_hidden: true,
            ..Default::default()
        };
        let mut scanner = scan_dir_with_config(temp_dir.path(), config);
        
        // Should find both visible and hidden files
        assert!(scanner.any(|f| f.file_name == ".hidden.txt"));
    }

    #[test]
    fn test_extension_filter() {
        let temp_dir = create_test_structure();
        let config = ScanConfig {
            allowed_extensions: Some(vec!["rs".to_string()]),
            ..Default::default()
        };
        let scanner = scan_dir_with_config(temp_dir.path(), config);
        
        let results: Vec<_> = scanner.collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_name, "file2.rs");
    }

    #[test]
    fn test_size_filter() {
        let temp_dir = create_test_structure();
        let config = ScanConfig {
            min_size: Some(1000),
            max_size: Some(2000),
            ..Default::default()
        };
        let scanner = scan_dir_with_config(temp_dir.path(), config);
        
        let results: Vec<_> = scanner.collect();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].file_name, "large.bin");
        assert_eq!(results[0].size, 1024);
    }

    #[test]
    fn test_include_directories() {
        let temp_dir = create_test_structure();
        let config = ScanConfig {
            include_dirs: true,
            max_depth: Some(1),
            ..Default::default()
        };
        let scanner = scan_dir_with_config(temp_dir.path(), config);
        
        let mut results: Vec<_> = scanner.collect();
        results.sort_by(|a, b| a.file_name.cmp(&b.file_name));
        
        assert!(results.iter().any(|f| f.is_dir && f.file_name == "subdir"));
        assert_eq!(results.len(), 3); // file1.txt, large.bin, subdir

        // Ensure subdir contents (file2.rs) are NOT included
        assert!(!results.iter().any(|e| e.file_name == "file2.rs"));
    }

    #[test]
    fn test_recursive_scan_with_dirs() {
        let temp_dir = create_test_structure();
        let config = ScanConfig {
            include_dirs: true, // include directories along with files
            ..Default::default() // recursive by default
        };
        let scanner = scan_dir_with_config(temp_dir.path(), config);
        let mut results: Vec<_> = scanner.collect();

        // Should find top-level files (file1.txt, large.bin)
        // plus subdir itself, and the file inside subdir (file2.rs)
        assert_eq!(results.len(), 4);
        results.sort_by(|a, b| a.file_name.cmp(&b.file_name));

        let first = &results[0];
        assert_eq!(first.file_name, "file1.txt");
        assert!(first.is_file);

        let second = &results[1];
        assert_eq!(second.file_name, "file2.rs");
        assert!(second.is_file);

        let third = &results[2];
        assert_eq!(third.file_name, "large.bin");
        assert!(third.is_file);

        let fourth = &results[3];
        assert_eq!(fourth.file_name, "subdir");
        assert!(fourth.is_dir);

        // Ensure hidden file is still excluded
        assert!(!results.iter().any(|e| e.file_name == ".hidden.txt"));
    }


    #[test]
    fn test_max_depth() {
        let temp_dir = create_test_structure();
        let config = ScanConfig {
            max_depth: Some(1),
            ..Default::default()
        };
        let mut scanner = scan_dir_with_config(temp_dir.path(), config);
        
        // Should not find file2.rs in subdir
        assert!(!scanner.any(|f| f.file_name == "file2.rs"));
    }

    #[test]
    fn test_symlink() {
        let temp_dir = create_test_structure();
        let path = temp_dir.path();
        
        // Create symlink (may not work on Windows without permissions)
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            symlink(path.join("file1.txt"), path.join("link.txt")).unwrap();
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::symlink_file;
            symlink_life(path.join("file1.txt"), path.join("link.txt")).unwrap();
        }

        let config = ScanConfig {
            follow_symlinks: true,
            ..Default::default()
        };
        let mut scanner = scan_dir_with_config(path, config);
        
        #[cfg(unix)]
        assert!(scanner.any(|f| f.file_name == "link.txt"));
    }

    #[test]
    fn test_broken_symlink() {
        let dir = tempdir().unwrap();
        let target = dir.path().join("nonexistent.txt");
        let symlink = dir.path().join("broken_link");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &symlink).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&target, &symlink).unwrap();

        let config = ScanConfig::default();
        let entries: Vec<_> = scan_dir_with_config(dir.path(), config).collect();

        // Broken symlink should still appear, marked as symlink
        let broken = entries.iter().find(|e| e.file_name == "broken_link")
            .expect("broken symlink should be present");

        assert!(broken.is_symlink);
        assert!(!broken.is_file);
        assert!(!broken.is_dir);
    }
}