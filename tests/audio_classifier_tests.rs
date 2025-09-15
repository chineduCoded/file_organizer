mod test_utils;

#[cfg(test)]
mod tests {
    use stash::{
        metadata::{AudioSubcategory, FileCategory},
        registry::Classifier,
        audio_classifier::AudioClassifier,
    };

    use crate::test_utils::create_test_file_with_ext;

    #[tokio::test]
    async fn test_confidence_levels() {
        let clf = AudioClassifier;

        // High confidence formats
        assert_eq!(clf.confidence("mp3", "audio/mpeg"), 100);
        assert_eq!(clf.confidence("flac", "audio/flac"), 100);

        // Medium confidence formats
        assert_eq!(clf.confidence("alac", "audio/alac"), 80);
        assert_eq!(clf.confidence("wma", "audio/x-ms-wma"), 80);

        // Lower confidence
        assert_eq!(clf.confidence("mid", "audio/midi"), 60);

        // MIME fallback
        assert_eq!(clf.confidence("unknown", "audio/ogg"), 90);

        // No confidence
        assert_eq!(clf.confidence("exe", "application/octet-stream"), 0);
    }

    #[tokio::test]
    async fn test_extract_metadata_mp3() {
        let (_dir, path) = create_test_file_with_ext("mp3");

        let clf = AudioClassifier;
        let result = clf.extract_metadata(&path).await.unwrap();

        match result.category {
            FileCategory::Audio(sub) => assert_eq!(sub, AudioSubcategory::Mp3),
            _ => panic!("Expected Audio category"),
        }

        assert_eq!(result.mime_type.unwrap(), "audio/mpeg"); // depends on detect_mime()
        assert!(result.file_size.unwrap() > 0);
    }

    #[tokio::test]
    async fn test_extract_metadata_wav() {
        let (_dir, path) = create_test_file_with_ext("wav");

        let clf = AudioClassifier;
        let result = clf.extract_metadata(&path).await.unwrap();

        match result.category {
            FileCategory::Audio(sub) => assert_eq!(sub, AudioSubcategory::Wav),
            _ => panic!("Expected Audio category"),
        }

        assert_eq!(result.mime_type.unwrap(), "audio/wav");
        assert!(result.file_size.unwrap() > 0);
    }
}