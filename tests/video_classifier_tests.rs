#[cfg(test)]
mod tests {
    use file_organizer::{video_classifier::VideoClassifier, registry::Classifier};
    use tempfile::Builder;
    use tokio::fs;
    use file_organizer::metadata::{FileCategory, VideoSubcategory};

    // ---------------------------
    // Unit tests
    // ---------------------------

    #[test]
    fn test_name() {
        let classifier = VideoClassifier;
        assert_eq!(classifier.name(), "VideoClassifier");
    }

    #[test]
    fn test_confidence_scores() {
        let c = VideoClassifier;

        // High confidence formats
        assert_eq!(c.confidence("mp4", "video/mp4"), 100);
        assert_eq!(c.confidence("mkv", "video/x-matroska"), 100);

        // Medium confidence formats
        assert_eq!(c.confidence("wmv", "video/x-ms-wmv"), 80);
        assert_eq!(c.confidence("flv", "video/x-flv"), 80);

        // MIME fallback
        assert_eq!(c.confidence("xyz", "video/xyz"), 90);

        // Non-video
        assert_eq!(c.confidence("txt", "text/plain"), 0);
    }

    // ---------------------------
    // Async integration tests
    // ---------------------------

    #[tokio::test]
    async fn test_extract_metadata_mp4() {
        let tmp = Builder::new().suffix(".mp4").tempfile().unwrap();
        fs::write(tmp.path(), b"fakevideodata").await.unwrap();

        let classifier = VideoClassifier;
        let meta = classifier.extract_metadata(tmp.path()).await.unwrap();

        assert!(matches!(meta.category, FileCategory::Videos(VideoSubcategory::Mp4)));
        assert_eq!(meta.mime_type.unwrap(), "video/mp4");
        assert_eq!(meta.file_size.unwrap(), 13); // len of "fakevideodata"
        assert!(meta.year.is_some());
    }

    #[tokio::test]
    async fn test_extract_metadata_wmv() {
        let tmp = Builder::new().suffix(".wmv").tempfile().unwrap();
        fs::write(tmp.path(), b"data").await.unwrap();

        let classifier = VideoClassifier;
        let meta = classifier.extract_metadata(tmp.path()).await.unwrap();

        assert!(matches!(meta.category, FileCategory::Videos(VideoSubcategory::Wmv)));
        assert_eq!(meta.mime_type.unwrap(), "video/x-ms-wmv");
    }

    #[tokio::test]
    async fn test_extract_metadata_unknown_extension() {
        let tmp = Builder::new().suffix(".foo").tempfile().unwrap();
        fs::write(tmp.path(), b"data").await.unwrap();

        let classifier = VideoClassifier;
        let meta = classifier.extract_metadata(tmp.path()).await.unwrap();

        assert!(matches!(meta.category, FileCategory::Videos(VideoSubcategory::Other)));
        assert_eq!(meta.mime_type.unwrap(), "application/octet-stream");
    }

    // ---------------------------
    // Optional property tests
    // ---------------------------

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_confidence_never_panics(ext in "[a-z]{0,5}", mime in ".*") {
            let c = VideoClassifier;
            let _ = c.confidence(&ext, &mime);
        }
    }
}
