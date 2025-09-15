#[cfg(test)]
mod tests {
    use tempfile::Builder;
    use tokio::fs;
    use stash::{
        registry::Classifier,
        docs_classifier::DocumentClassifier, 
        metadata::{DocumentSubcategory, FileCategory}
    };

    // ---------------------------
    // Unit tests
    // ---------------------------

    #[test]
    fn test_name() {
        let classifier = DocumentClassifier;
        assert_eq!(classifier.name(), "DocumentClassifier");
    }

    #[test]
    fn test_confidence_scores() {
        let c = DocumentClassifier;

        // High confidence extensions
        assert_eq!(c.confidence("pdf", "application/pdf"), 100);
        assert_eq!(c.confidence("docx", "application/vnd.openxmlformats-officedocument.wordprocessingml.document"), 100);

        // Medium confidence
        assert_eq!(c.confidence("txt", "text/plain"), 80);
        assert_eq!(c.confidence("md", "text/markdown"), 80);

        // MIME-driven confidence
        assert_eq!(c.confidence("foo", "application/vnd.ms-excel"), 90);
        assert_eq!(c.confidence("foo", "application/epub+zip"), 90);

        // Lower text-based MIME
        assert_eq!(c.confidence("foo", "text/plain"), 70);

        // No confidence
        assert_eq!(c.confidence("exe", "application/octet-stream"), 0);
    }

    // ---------------------------
    // Async integration tests
    // ---------------------------

    #[tokio::test]
    async fn test_extract_metadata_pdf() {
        let tmp = Builder::new().suffix(".pdf").tempfile().unwrap();
        fs::write(tmp.path(), b"%PDF-1.4").await.unwrap();

        let classifier = DocumentClassifier;
        let meta = classifier.extract_metadata(tmp.path()).await.unwrap();

        assert!(matches!(meta.category, FileCategory::Documents(DocumentSubcategory::Pdf)));
        assert_eq!(meta.mime_type.unwrap(), "application/pdf");
        assert_eq!(meta.file_size.unwrap(), 8);
        assert!(meta.year.is_some());
    }

    #[tokio::test]
    async fn test_extract_metadata_docx() {
        let tmp = Builder::new().suffix(".docx").tempfile().unwrap();
        fs::write(tmp.path(), b"docxdata").await.unwrap();

        let classifier = DocumentClassifier;
        let meta = classifier.extract_metadata(tmp.path()).await.unwrap();

        assert!(matches!(meta.category, FileCategory::Documents(DocumentSubcategory::Word)));
        assert_eq!(meta.mime_type.unwrap(), "application/vnd.openxmlformats-officedocument.wordprocessingml.document");
    }

    #[tokio::test]
    async fn test_extract_metadata_txt() {
        let tmp = Builder::new().suffix(".txt").tempfile().unwrap();
        fs::write(tmp.path(), b"hello world").await.unwrap();

        let classifier = DocumentClassifier;
        let meta = classifier.extract_metadata(tmp.path()).await.unwrap();

        assert!(matches!(meta.category, FileCategory::Documents(DocumentSubcategory::Text)));
        assert_eq!(meta.mime_type.unwrap(), "text/plain");
    }

    #[tokio::test]
    async fn test_extract_metadata_unknown() {
        let tmp = Builder::new().suffix(".foo").tempfile().unwrap();
        fs::write(tmp.path(), b"somecontent").await.unwrap();

        let classifier = DocumentClassifier;
        let meta = classifier.extract_metadata(tmp.path()).await.unwrap();

        assert!(matches!(meta.category, FileCategory::Documents(DocumentSubcategory::Other)));
        assert_eq!(meta.mime_type.unwrap(), "application/octet-stream");
    }

    // ---------------------------
    // Optional property tests
    // ---------------------------

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn test_confidence_never_panics(ext in "[a-z]{0,6}", mime in ".*") {
            let c = DocumentClassifier;
            let _ = c.confidence(&ext, &mime);
        }
    }
}
