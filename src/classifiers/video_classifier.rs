use std::path::Path;
use async_trait::async_trait;
use crate::{
    classifier::{detect_mime, system_time_to_year, Classifier},
    errors::Result,
    metadata::{ClassifiedFileMetadata, VideoSubcategory, FileCategory},
};

pub struct VideoClassifier;

#[async_trait]
impl Classifier for VideoClassifier {
    fn name(&self) -> &'static str {
        "VideoClassifier"
    }

    fn confidence(&self, extension: &str, mime_type: &str) -> u8 {
        // High confidence for common video formats
        if matches!(
            extension,
            "mp4" | "mkv" | "mov" | "webm" | "avi" | "m4v" | "mpg" | "mpeg"
        ) {
            return 100;
        }

        // Medium confidence for less common video formats
        if matches!(
            extension,
            "wmv" | "flv" | "3gp" | "m2ts" | "ts" | "mts" | "vob" | "ogv" | "divx"
        ) {
            return 80;
        }

        // MIME type based confidence
        if mime_type.starts_with("video/") {
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
            .and_then(system_time_to_year);

        let subcategory = match ext.as_str() {
            "mp4" | "m4v" => VideoSubcategory::Mp4,
            "avi" | "divx" => VideoSubcategory::Avi,
            "mkv" => VideoSubcategory::Mkv,
            "mov" => VideoSubcategory::Mov,
            "webm" | "ogv" => VideoSubcategory::Webm,
            "wmv" => VideoSubcategory::Wmv,
            "flv" => VideoSubcategory::Flv,
            "mpg" | "mpeg" => VideoSubcategory::Mpeg,
            "3gp" => VideoSubcategory::ThreeGp,
            "ts" | "mts" | "m2ts" => VideoSubcategory::Ts,
            "vob" => VideoSubcategory::Vob,
            _ => VideoSubcategory::Other,
        };

        let mut classified = ClassifiedFileMetadata::new(
            path.to_path_buf(),
            FileCategory::Videos(subcategory),
        );
        classified.mime_type = Some(mime);
        classified.file_size = Some(size);
        classified.year = year;

        Ok(classified)
    }
}