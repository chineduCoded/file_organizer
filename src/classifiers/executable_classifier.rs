use std::path::Path;
use async_trait::async_trait;
use crate::{
    classifiers::executables_const::{EXECUTABLE_EXTENSION_MAP, EXECUTABLE_MIME_PATTERNS}, errors::Result, metadata::{ClassifiedFileMetadata, ExecutableSubcategory, FileCategory}, registry::Classifier, utils::{detect_mime, system_time_to_year}
};

pub struct ExecutableClassifier;

#[async_trait]
impl Classifier for ExecutableClassifier {
    fn name(&self) -> &'static str {
        "ExecutableClassifier"
    }

    fn confidence(&self, extension: &str, mime_type: &str) -> u8 {
        // High confidence for binary executables
        if matches!(
            extension,
            "exe" | "msi" | "dll" | "app" | "dmg" | "pkg" | "dylib" | 
            "deb" | "rpm" | "so" | "bin" | "apk" | "ipa"
        ) {
            return 100;
        }

        // Medium confidence for scripts
        if matches!(extension, "bat" | "cmd" | "sh" | "ps1") {
            return 80;
        }

        // Lower confidence for config and log files
        if matches!(extension, "conf" | "ini" | "log") {
            return 60;
        }

        // MIME type based confidence
        if EXECUTABLE_MIME_PATTERNS.iter().any(|pattern| mime_type.contains(pattern)) {
            return 90;
        }

        // No confidence for other types
        0
    }

    async fn extract_metadata(&self, path: &Path) -> Result<ClassifiedFileMetadata> {
        let raw = tokio::fs::metadata(path).await?;
        let size = raw.len();
        
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();

        let mime = detect_mime(&ext);

        let year = raw
            .modified()
            .ok()
            .or_else(|| raw.created().ok())
            .and_then(|t| system_time_to_year(t));

        let subcategory = EXECUTABLE_EXTENSION_MAP
            .get(ext.as_str())
            .cloned()
            .unwrap_or(ExecutableSubcategory::Other);

        let mut classified = ClassifiedFileMetadata::new(
            path.to_path_buf(),
            FileCategory::Executables(subcategory),
        );
        classified.mime_type = Some(mime);
        classified.file_size = Some(size);
        classified.year = year;

        Ok(classified)
    }
}