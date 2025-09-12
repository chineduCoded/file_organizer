#[cfg(test)]
mod tests {
    use std::path::Path;
    use file_organizer::{
        executable_classifier::ExecutableClassifier,
        metadata::{FileCategory, ExecutableSubcategory},
        registry::Classifier,
    };

    #[tokio::test]
    async fn test_confidence_levels() {
        let classifier = ExecutableClassifier;

        // High confidence for executables
        assert_eq!(classifier.confidence("exe", "application/x-msdownload"), 100);
        assert_eq!(classifier.confidence("apk", "application/vnd.android.package-archive"), 100);

        // Medium confidence for scripts
        assert_eq!(classifier.confidence("sh", "text/x-shellscript"), 80);

        // Lower confidence for config/log
        assert_eq!(classifier.confidence("log", "text/plain"), 60);

        // MIME-based confidence
        assert_eq!(classifier.confidence("unknown", "application/x-executable"), 90);

        // No confidence
        assert_eq!(classifier.confidence("txt", "text/plain"), 0);
    }

    #[tokio::test]
    async fn test_extract_metadata_exe() {
        let path = Path::new("test_program.exe");
        tokio::fs::write(&path, "dummy exe").await.unwrap();

        let classifier = ExecutableClassifier;
        let metadata = classifier.extract_metadata(&path).await.unwrap();

        assert!(matches!(metadata.category, FileCategory::Executables(ExecutableSubcategory::WindowsApp)));
        assert_eq!(metadata.file_size, Some(9));

        tokio::fs::remove_file(&path).await.unwrap();
    }

    #[tokio::test]
    async fn test_extract_metadata_sh() {
        let path = Path::new("script.sh");
        tokio::fs::write(&path, "#!/bin/bash\necho hello").await.unwrap();

        let classifier = ExecutableClassifier;
        let metadata = classifier.extract_metadata(&path).await.unwrap();

        assert!(matches!(metadata.category, FileCategory::Executables(ExecutableSubcategory::Script)));
        assert_eq!(metadata.file_size, Some(22));

        tokio::fs::remove_file(&path).await.unwrap();
    }
}