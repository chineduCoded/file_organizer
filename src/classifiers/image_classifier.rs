use std::path::Path;
use async_trait::async_trait;
use crate::{
    classifier::{detect_mime, system_time_to_year, Classifier},
    errors::Result,
    metadata::{ClassifiedFileMetadata, ImageSubcategory, FileCategory},
};

pub struct ImageClassifier;

#[async_trait]
impl Classifier for ImageClassifier {
    fn name(&self) -> &'static str {
        "ImageClassifier"
    }

    fn confidence(&self, extension: &str, mime_type: &str) -> u8 {
        // High confidence for common image formats
        if matches!(
            extension,
            "jpg" | "jpeg" | "png" | "gif" | "svg" | "webp" | "bmp" | "ico"
        ) {
            return 100;
        }

        // High confidence for RAW image formats
        if matches!(
            extension,
            "raw" | "cr2" | "nef" | "arw" | "dng" | "tiff" | "tif"
        ) {
            return 95;
        }

        // Medium confidence for newer image formats
        if matches!(extension, "heic" | "heif") {
            return 85;
        }

        // MIME type based confidence
        if mime_type.starts_with("image/") {
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
            "jpg" | "jpeg" => ImageSubcategory::Jpeg,
            "png" => ImageSubcategory::Png,
            "gif" => ImageSubcategory::Gif,
            "svg" => ImageSubcategory::Svg,
            "raw" | "cr2" | "nef" | "arw" | "dng" => ImageSubcategory::Raw,
            "tiff" | "tif" => ImageSubcategory::Tiff,
            "webp" => ImageSubcategory::Webp,
            "bmp" => ImageSubcategory::Bmp,
            "ico" => ImageSubcategory::Ico,
            "heic" | "heif" => ImageSubcategory::Heic,
            _ => ImageSubcategory::Other,
        };

        let mut classified = ClassifiedFileMetadata::new(
            path.to_path_buf(),
            FileCategory::Images(subcategory),
        );
        classified.mime_type = Some(mime);
        classified.file_size = Some(size);
        classified.year = year;

        Ok(classified)
    }
}