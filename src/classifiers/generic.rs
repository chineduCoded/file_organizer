use std::path::Path;
use async_trait::async_trait;

use crate::{errors::Result, metadata::{ClassifiedFileMetadata, FileCategory}, registry::Classifier};


pub struct GenericClassifier;

#[async_trait]
impl Classifier for GenericClassifier {
    fn name(&self) -> &'static str {
        "GenericClassifier"
    }

    fn confidence(&self, _: &str, _: &str) -> u8 {
        1
    }

    async fn extract_metadata(&self, path: &Path) -> Result<ClassifiedFileMetadata> {
        Ok(ClassifiedFileMetadata::new(
            path.to_path_buf(),
            FileCategory::Others,
        ))
    }
}