mod test_utils;

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, sync::Arc};
    use std::path::Path;
    use async_trait::async_trait;

    use stash::{
        errors::{FileOrganizerError, Result}, 
        metadata::ClassifiedFileMetadata,
        registry::{ClassifierRegistry, Classifier}, 
    };
    use tokio::task;

    use crate::test_utils::{create_test_file, create_test_metadata};

    // =============== Dynamic Mock Classifier ===============

    pub struct MockClassifier {
        pub name: &'static str,
        pub confidence_score: u8,
        pub metadata_fn: Arc<dyn Fn(&Path) -> Result<ClassifiedFileMetadata> + Send + Sync>,
    }

    #[async_trait]
    impl Classifier for MockClassifier {
        fn name(&self) -> &'static str {
            self.name
        }

        fn confidence(&self, _extension: &str, _mime_type: &str) -> u8 {
            self.confidence_score
        }

        async fn extract_metadata(&self, path: &Path) -> Result<ClassifiedFileMetadata> {
           (self.metadata_fn)(path)
        }
    }

    // =============== Tests ===============

    #[tokio::test]
    async fn test_classifier_registry_and_priority_sorting() {
        let mut registry = ClassifierRegistry::new();
        let file = create_test_file("test.txt", 1024);
        
        let low_priority = Arc::new(MockClassifier {
            name: "LowPriority",
            confidence_score: 100,
            metadata_fn: Arc::new(|path| Ok(create_test_metadata(path))),
        });
        
        let high_priority = Arc::new(MockClassifier {
            name: "HighPriority",
            confidence_score: 50, // Lower confidence but higher priority
            metadata_fn: Arc::new(|path| Ok(create_test_metadata(path))),
        });

        registry.register_with_priority(10, low_priority);
        registry.register_with_priority(90, high_priority);

        assert_eq!(registry.classifiers[0].1.name(), "HighPriority");
        assert_eq!(registry.classifiers[1].1.name(), "LowPriority");

        let result = registry.classify(&file).await.unwrap();
        
        // High priority should win despite lower confidence
        assert_eq!(result.mime_type.unwrap(), "text/plain");
    }

    #[tokio::test]
    async fn test_confidence_threshold() {
        let mut registry = ClassifierRegistry::new();

        let low_confidence = Arc::new(MockClassifier {
            name: "LowConfidence",
            confidence_score: 10,
            metadata_fn: Arc::new(|_| Err(FileOrganizerError::Classify("Failed".into()))),
        });

        registry.register(low_confidence);

        let file = create_test_file("test.txt", 1024);
        let result = registry.classify(&file).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fallback_to_next_classifier() {
        let mut registry = ClassifierRegistry::new();
        let file = create_test_file("fallback.bin", 2048);

        let failing_classifier = Arc::new(MockClassifier {
            name: "FailingClassifier",
            confidence_score: 100,
            metadata_fn: Arc::new(|_| Err(FileOrganizerError::Other("Failed to extract".into()))),
        });
        
        let succeeding_classifier = Arc::new(MockClassifier {
            name: "SucceedingClassifier",
            confidence_score: 50,
            metadata_fn: Arc::new(|path| Ok(create_test_metadata(path))),
        });

        registry.register_with_priority(10, failing_classifier);
        registry.register_with_priority(20, succeeding_classifier);

        let classified = registry.classify(&file).await;

        let result = classified.as_ref().unwrap();
        let expected = create_test_metadata(&file.path);

        
        // Should fall back to the succeeding classifier
        assert_eq!(result.mime_type, expected.mime_type);
    }

    #[tokio::test]
    async fn test_mime_caching() {
        let registry = ClassifierRegistry::new();
        let mime = registry.get_cached_mime("txt").await;
        
        // Second call should use cache
        let cached_mime = registry.get_cached_mime("txt").await;
        assert_eq!(mime, cached_mime);
    }

    #[tokio::test]
    async fn test_batch_classification() {
        let mut registry = ClassifierRegistry::new();

        let succeeding_classifier = Arc::new(MockClassifier {
            name: "SucceedingClassifier",
            confidence_score: 50,
            metadata_fn: Arc::new(|path| Ok(create_test_metadata(path))),
        });

        registry.register_with_priority(20, succeeding_classifier);

        // Create multiple test files
        let file1 = create_test_file("a.txt", 1024);
        let file2 = create_test_file("b.bin", 2048);
        // Classify in batch
        let results = futures::future::join_all(vec![
            registry.classify(&file1),
            registry.classify(&file2),
        ])
        .await;

        let result1 = results[0].as_ref().unwrap();
        let result2 = results[1].as_ref().unwrap();

        let expected1 = create_test_metadata(&file1.path);
        let expected2 = create_test_metadata(&file2.path);

        // ✅ Both should use the succeeding classifier
        assert_eq!(result1.mime_type, expected1.mime_type);
        assert_eq!(result2.mime_type, expected2.mime_type);
    }

    #[tokio::test]
    async fn test_concurrent_classification() {
        let mut registry = ClassifierRegistry::new();
        
        // A classifier that always succeeds with "text/plain"
        let classifier = Arc::new(MockClassifier {
            name: "ConcurrentClassifier",
            confidence_score: 80,
            metadata_fn: Arc::new(|_path| Ok(create_test_metadata(&PathBuf::from("dummy.txt")))),
        });

        registry.register_with_priority(10, classifier);

        let registry = Arc::new(registry);

        // Create multiple fake files
        let files: Vec<_> = (0..20)
            .map(|i| create_test_file(&format!("file_{}.txt", i), 1024))
            .collect();

        // Spawn concurrent tasks
        let mut handles = Vec::new();
        for file in files {
            let reg = Arc::clone(&registry);
            handles.push(task::spawn(async move {
                reg.classify(&file).await
            }));
        }

        // Collect results
        let results: Vec<_> = futures::future::join_all(handles).await;

        for res in results {
            let classified = res.unwrap().unwrap();
            assert_eq!(classified.mime_type, Some("text/plain".to_string()));
        }
    }

    /// 🔥 Stress test with 500 concurrent tasks
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_concurrent_classification_stress() {
        let mut registry = ClassifierRegistry::new();

        let classifier = Arc::new(MockClassifier {
            name: "StressClassifier",
            confidence_score: 80,
            metadata_fn: Arc::new(|_path| Ok(create_test_metadata(&std::path::PathBuf::from("dummy.txt")))),
        });

        registry.register_with_priority(10, classifier);
        let registry = Arc::new(registry);

        let files: Vec<_> = (0..500)
            .map(|i| create_test_file(&format!("stress_file_{}.txt", i), 2048))
            .collect();

        let handles: Vec<_> = files.into_iter()
            .map(|file| {
                let reg = Arc::clone(&registry);
                task::spawn(async move {
                    reg.classify(&file).await
                })
            })
            .collect();

        let results = futures::future::join_all(handles).await;

        for res in results {
            let classified = res.unwrap().unwrap();
            assert_eq!(classified.mime_type, Some("text/plain".to_string()));
        }
    }
}

