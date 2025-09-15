mod test_utils;

#[cfg(test)]
mod tests {
    use stash::{archive_classifier::ArchiveClassifier, registry::Classifier};
    use stash::metadata::{ArchiveSubcategory, FileCategory};

    use crate::test_utils::create_test_file_with_ext;

    #[tokio::test]
    async fn test_confidence_levels() {
        let clf = ArchiveClassifier;

        // High confidence
        assert_eq!(clf.confidence("zip", "application/zip"), 100);
        assert_eq!(clf.confidence("tar", "application/x-tar"), 100);

        // Medium confidence
        assert_eq!(clf.confidence("iso", "application/x-iso9660-image"), 80);

        // Lower confidence
        assert_eq!(clf.confidence("deb", "application/vnd.debian.binary-package"), 60);

        // MIME-based confidence
        assert_eq!(clf.confidence("foo", "application/x-tar"), 90);

        // No confidence
        assert_eq!(clf.confidence("txt", "text/plain"), 0);
    }

    #[tokio::test]
    async fn test_extract_metadata_zip() {
        let (_dir, path) = create_test_file_with_ext("zip");

        let clf = ArchiveClassifier;
        let result = clf.extract_metadata(&path).await.unwrap();

        match result.category {
            FileCategory::Archives(sub) => assert_eq!(sub, ArchiveSubcategory::Zip),
            _ => panic!("Expected archive category"),
        }

        assert_eq!(result.mime_type.unwrap(), "application/zip");
        assert!(result.file_size.unwrap() > 0);
    }

    #[tokio::test]
    async fn test_extract_metadata_rar() {
        let (_dir, path) = create_test_file_with_ext("rar");

        let clf = ArchiveClassifier;
        let result = clf.extract_metadata(&path).await.unwrap();

        match result.category {
            FileCategory::Archives(sub) => assert_eq!(sub, ArchiveSubcategory::Rar),
            _ => panic!("Expected archive category"),
        }

        assert_eq!(result.mime_type.unwrap(), "application/x-rar-compressed");
    }
}
