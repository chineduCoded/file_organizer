#[cfg(test)]
mod tests {
    use tempfile::Builder;
    use tokio::fs;
    use stash::{image_classifier::ImageClassifier, registry::Classifier, metadata::{FileCategory, ImageSubcategory}};

    // ---------------------------
    // Unit tests
    // ---------------------------

    #[test]
    fn test_name() {
        let classifier = ImageClassifier;
        assert_eq!(classifier.name(), "ImageClassifier");
    }

    #[test]
    fn test_confidence_scores() {
        let c = ImageClassifier;

        // Common formats
        assert_eq!(c.confidence("jpg", "image/jpeg"), 100);
        assert_eq!(c.confidence("png", "image/png"), 100);

        // RAW formats
        assert_eq!(c.confidence("nef", "image/x-nikon"), 95);

        // Newer formats
        assert_eq!(c.confidence("heic", "image/heic"), 85);

        // MIME type fallback
        assert_eq!(c.confidence("xyz", "image/xyz"), 90);

        // Not an image
        assert_eq!(c.confidence("txt", "text/plain"), 0);
    }

    // ---------------------------
    // Async integration tests
    // ---------------------------

    #[tokio::test]
    async fn test_extract_metadata_jpeg() {
        let tmp = Builder::new().suffix(".jpg").tempfile().unwrap();
        fs::write(tmp.path(), b"fakejpegdata").await.unwrap();

        let classifier = ImageClassifier;
        let meta = classifier.extract_metadata(tmp.path()).await.unwrap();

        assert!(matches!(meta.category, FileCategory::Images(ImageSubcategory::Jpeg)));
        assert_eq!(meta.mime_type.unwrap(), "image/jpeg");
        assert_eq!(meta.file_size.unwrap(), 12);
        assert!(meta.year.is_some());
    }

    #[tokio::test]
    async fn test_extract_metadata_heic() {
        let tmp = Builder::new().suffix(".heic").tempfile().unwrap();
        fs::write(tmp.path(), b"fakeheic").await.unwrap();

        let classifier = ImageClassifier;
        let meta = classifier.extract_metadata(tmp.path()).await.unwrap();

        assert!(matches!(meta.category, FileCategory::Images(ImageSubcategory::Heic)));
        assert_eq!(meta.mime_type.unwrap(), "image/heic");
    }

    #[tokio::test]
    async fn test_extract_metadata_unknown_extension() {
        let tmp = Builder::new().suffix(".foo").tempfile().unwrap();
        fs::write(tmp.path(), b"data").await.unwrap();

        let classifier = ImageClassifier;
        let meta = classifier.extract_metadata(tmp.path()).await.unwrap();

        assert!(matches!(meta.category, FileCategory::Images(ImageSubcategory::Other)));
        assert_eq!(meta.mime_type.unwrap(), "application/octet-stream");
    }

    // ---------------------------
    // Optional property tests
    // ---------------------------

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_confidence_never_panics(ext in "[a-z]{0,5}", mime in ".*") {
            let c = ImageClassifier;
            let _ = c.confidence(&ext, &mime);
        }
    }
}
