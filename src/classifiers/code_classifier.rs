use std::path::Path;
use async_trait::async_trait;
use crate::{
    classifier::{detect_mime, system_time_to_year, Classifier}, code_const::{CODE_MIME_PATTERNS, EXTENSION_MAP}, errors::Result, metadata::{ClassifiedFileMetadata, CodeSubcategory, FileCategory}
};

pub struct CodeClassifier;

#[async_trait]
impl Classifier for CodeClassifier {
    fn name(&self) -> &'static str {
        "CodeClassifier"
    }

    fn confidence(&self, extension: &str, mime_type: &str) -> u8 {
        // High confidence for programming languages
        if matches!(
            extension,
            "rs" | "py" | "js" | "ts" | "java" | "c" | "cpp" | "go" | "php" |
            "swift" | "kt" | "scala" | "rb" | "pl" | "lua" | "hs" | "dart"
        ) {
            return 100;
        }

        // High confidence for web technologies
        if matches!(
            extension,
            "html" | "htm" | "css" | "scss" | "sass" | "less" | "styl"
        ) {
            return 95;
        }

        // Medium confidence for configuration files
        if matches!(
            extension,
            "json" | "yaml" | "yml" | "toml" | "xml" | "ini" | "conf" | "properties"
        ) {
            return 85;
        }

        // Medium confidence for database files
        if matches!(extension, "sql" | "plsql" | "tsql") {
            return 80;
        }

        // Medium confidence for build/automation files
        if matches!(
            extension,
            "makefile" | "mk" | "dockerfile" | "dockerignore" | "gitignore"
        ) {
            return 75;
        }

        // Lower confidence for documentation files (may conflict with DocumentClassifier)
        if matches!(extension, "md" | "markdown" | "rst") {
            return 65;
        }

        // MIME type based confidence
        if CODE_MIME_PATTERNS.iter().any(|pattern| mime_type.contains(pattern)) {
            return 90;
        }

        // Generic text files
        if mime_type.starts_with("text/") {
            return 70;
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
            .and_then(system_time_to_year);

        // Determine subcategory using the extension map
        let subcategory = EXTENSION_MAP
            .get(ext.as_str())
            .cloned()
            .unwrap_or_else(|| CodeSubcategory::Other(ext.clone()));

        let mut classified = ClassifiedFileMetadata::new(
            path.to_path_buf(),
            FileCategory::Code(subcategory),
        );
        classified.mime_type = Some(mime);
        classified.file_size = Some(size);
        classified.year = year;

        Ok(classified)
    }
}