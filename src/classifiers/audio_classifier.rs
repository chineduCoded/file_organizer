use std::path::Path;
use async_trait::async_trait;
use crate::{
    classifier::{detect_mime, system_time_to_year, Classifier},
    errors::Result,
    metadata::{AudioSubcategory, ClassifiedFileMetadata, FileCategory},
};

pub struct AudioClassifier;

#[async_trait]
impl Classifier for AudioClassifier {
    fn name(&self) -> &'static str {
        "AudioClassifier"
    }

    fn confidence(&self, extension: &str, mime_type: &str) -> u8 {
        // High confidence for common audio formats
        if matches!(
            extension,
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "opus"
        ) {
            return 100;
        }

        // Medium confidence for less common audio formats
        if matches!(
            extension,
            "alac" | "aiff" | "aif" | "wma" | "pcm" | "dsd" | "dff" |
            "dsf" | "ape" | "ac3" | "dts" | "amr" | "ra" | "rm" | "caf" | "weba"
        ) {
            return 80;
        }

        // Lower confidence for music/sound formats that might be ambiguous
        if matches!(extension, "mid" | "midi" | "xm" | "mod" | "s3m") {
            return 60;
        }

        // MIME type based confidence
        if mime_type.starts_with("audio/") {
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
            "mp3" => AudioSubcategory::Mp3,
            "wav" => AudioSubcategory::Wav,
            "flac" => AudioSubcategory::Flac,
            "aac" => AudioSubcategory::Aac,
            "ogg" => AudioSubcategory::Ogg,
            "m4a" => AudioSubcategory::M4a,
            "opus" => AudioSubcategory::Opus,
            "alac" => AudioSubcategory::Alac,
            "aiff" | "aif" => AudioSubcategory::Aiff,
            "wma" => AudioSubcategory::Wma,
            _ => AudioSubcategory::Other,
        };

        let mut classified = ClassifiedFileMetadata::new(
            path.to_path_buf(),
            FileCategory::Audio(subcategory),
        );
        classified.mime_type = Some(mime);
        classified.file_size = Some(size);
        classified.year = year;

        Ok(classified)
    }
}