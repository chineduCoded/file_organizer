#[cfg(test)]
mod tests {
    use file_organizer::{
        code_classifier::CodeClassifier,
        metadata::{CodeSubcategory, FileCategory},
        registry::Classifier,
    };
    use tempfile::TempDir;
    use std::{fs, path::Path};

    struct TestFile<'a>(&'a Path);

    impl<'a> Drop for TestFile<'a> {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(self.0);
        }
    }

    /// Helper to create a temporary file with the given extension
    fn create_test_file_with_ext(ext: &str) -> (TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let filename = if ext == "makefile" {
            // Special case: no extension, just name
            "Makefile".to_string()
        } else if ext == "dockerfile" {
            "Dockerfile".to_string()
        } else {
            format!("testfile.{}", ext)
        };
        let path = dir.path().join(filename);
        fs::write(&path, b"dummy code").unwrap();
        (dir, path)
    }

    #[tokio::test]
    async fn test_confidence_levels() {
        let clf = CodeClassifier;

        // High confidence (programming languages)
        assert_eq!(clf.confidence("rs", "text/plain"), 100);
        assert_eq!(clf.confidence("py", "text/x-python"), 100);

        // Web tech
        assert_eq!(clf.confidence("html", "text/html"), 95);
        assert_eq!(clf.confidence("css", "text/css"), 95);

        // Config files
        assert_eq!(clf.confidence("json", "application/json"), 85);
        assert_eq!(clf.confidence("toml", "text/plain"), 85);

        // Database files
        assert_eq!(clf.confidence("sql", "application/sql"), 80);

        // Build/automation
        assert_eq!(clf.confidence("makefile", "text/x-makefile"), 75);
        assert_eq!(clf.confidence("dockerfile", "text/plain"), 75);

        // Documentation files
        assert_eq!(clf.confidence("md", "text/markdown"), 65);

        // MIME pattern match
        assert_eq!(clf.confidence("unknown", "application/javascript"), 90);

        // Generic text fallback
        assert_eq!(clf.confidence("foo", "text/plain"), 70);

        // No confidence
        assert_eq!(clf.confidence("exe", "application/octet-stream"), 0);
    }

    #[tokio::test]
    async fn test_extract_metadata_rust_file() {
        let (_dir, path) = create_test_file_with_ext("rs");
        let clf = CodeClassifier;

        let result = clf.extract_metadata(&path).await.unwrap();

        match result.category {
            FileCategory::Code(CodeSubcategory::Rust) => {}
            other => panic!("Expected Rust, got {:?}", other),
        }

        assert_eq!(result.mime_type.unwrap(), "text/x-rust");
        assert!(result.file_size.unwrap() > 0);
    }

    #[tokio::test]
    async fn test_extract_metadata_dockerfile() {
        let path = Path::new("Dockerfile");
        tokio::fs::write(&path, "FROM rust:latest").await.unwrap();
        let _guard = TestFile(path);

        let classifier = CodeClassifier;
        let metadata = classifier.extract_metadata(&path).await.unwrap();

        assert!(matches!(metadata.category, FileCategory::Code(CodeSubcategory::Dockerfile)));
    }


    #[tokio::test]
    async fn test_extract_metadata_unknown_extension() {
        let (_dir, path) = create_test_file_with_ext("xyzlang");
        let clf = CodeClassifier;

        let result = clf.extract_metadata(&path).await.unwrap();

        match result.category {
            FileCategory::Code(CodeSubcategory::Other(ext)) => {
                assert_eq!(ext, "xyzlang");
            }
            _ => panic!("Expected Other subcategory"),
        }

        assert_eq!(result.mime_type.unwrap(), "application/octet-stream");
    }
}