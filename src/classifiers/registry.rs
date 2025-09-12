use std::{
    collections::HashMap,
    ffi::OsStr,
    path::Path,
    sync::Arc,
};

use async_trait::async_trait;
use futures::future::join_all;
use tokio::sync::RwLock;

use crate::{
    errors::{FileOrganizerError, Result},
    metadata::ClassifiedFileMetadata,
    scanner::RawFileMetadata, utils::detect_mime,
};

#[async_trait]
pub trait Classifier: Send + Sync {
    fn name(&self) -> &'static str;
    fn confidence(&self, extension: &str, mime_type: &str) -> u8;
    async fn extract_metadata(&self, path: &Path) -> Result<ClassifiedFileMetadata>;
}

#[derive(Default, Clone)]
pub struct ClassifierRegistry {
    pub classifiers: Arc<Vec<(u8, Arc<dyn Classifier>)>>, // (priority, classifier)
    pub mime_cache: Arc<RwLock<HashMap<String, String>>>,
}

impl ClassifierRegistry {
    pub fn new() -> Self {
        Self {
            classifiers: Arc::new(Vec::new()),
            mime_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register_with_priority(&mut self, priority: u8, classifier: Arc<dyn Classifier>) {
        let classifiers = Arc::get_mut(&mut self.classifiers)
            .expect("Cannot mutate classifiers after sharing");
        
        classifiers.push((priority, classifier));
        
        // Sort by priority (highest first)
        classifiers.sort_by(|a, b| b.0.cmp(&a.0));
    }

    // Keep the original register method for backward compatibility
    pub fn register(&mut self, classifier: Arc<dyn Classifier>) {
        self.register_with_priority(50, classifier); // Default priority
    }

    pub async fn classify_batch(
        &self,
        files: Vec<RawFileMetadata>,
    ) -> Vec<Result<ClassifiedFileMetadata>> {
        let tasks: Vec<_> = files
            .into_iter()
            .map(|raw| {
                let registry = self.clone();
                tokio::spawn(async move { registry.classify(&raw).await })
            })
            .collect();

        let join_results = join_all(tasks).await;
        join_results
            .into_iter()
            .map(|jh| match jh {
                Ok(inner) => inner,
                Err(e) => Err(FileOrganizerError::Classify(e.to_string())),
            })
            .collect()
    }

    pub async fn classify(&self, raw: &RawFileMetadata) -> Result<ClassifiedFileMetadata> {
        let ext = raw
            .path
            .extension()
            .and_then(OsStr::to_str)
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();

        let mime = self.get_cached_mime(&ext).await;

        // Collect all classifiers with their confidence scores
        let mut candidates = Vec::new();
        for (priority, classifier) in &*self.classifiers {
            let confidence = classifier.confidence(&ext, &mime);
            if confidence > 0 {
                // Combine priority and confidence for weighted score
                let weighted_score = (*priority as u16) * (confidence as u16);
                candidates.push((classifier, weighted_score, confidence));
            }
        }

        // Sort by weighted score (highest first)
        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        // Try classifiers in weighted score order
        for (classifier, weighted_score, confidence) in candidates {
            tracing::debug!(
                "Trying {} with weighted score {} (confidence: {}) for {:?}",
                classifier.name(),
                weighted_score,
                confidence,
                raw.path
            );

            match classifier.extract_metadata(&raw.path).await {
                Ok(mut metadata) => {
                    metadata.file_size = Some(raw.size);
                    metadata.mime_type = Some(mime.clone());
                    return Ok(metadata);
                }
                Err(e) => {
                    tracing::debug!("Classifier {} failed: {}", classifier.name(), e);
                }
            }
        }

        Err(FileOrganizerError::Classify(format!(
            "No classifier found for {:?}",
            raw.path
        )))
    }

    pub async fn get_cached_mime(&self, ext: &str) -> String {
        let read_cache = self.mime_cache.read().await;
        if let Some(mime) = read_cache.get(ext) {
            return mime.clone();
        }
        drop(read_cache);

        let mime = detect_mime(ext);
        let mut write_cache = self.mime_cache.write().await;
        write_cache.insert(ext.to_string(), mime.clone());
        mime
    }
}