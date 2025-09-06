use std::path::Path;
use async_trait::async_trait;
use crate::{
    errors::Result, 
    metadata::{ArchiveSubcategory, ClassifiedFileMetadata, FileCategory}, 
    registry::Classifier, 
    utils::{detect_mime, system_time_to_year}
};

pub struct ArchiveClassifier;

#[async_trait]
impl Classifier for ArchiveClassifier {
    fn name(&self) -> &'static str {
        "ArchiveClassifier"
    }

    fn confidence(&self, extension: &str, mime_type: &str) -> u8 {
        // High confidence for common archive formats
        if matches!(
            extension,
            "zip" | "tar" | "gz" | "rar" | "7z" | "bz2" | "xz" | "tgz" | "tbz2" | "txz"
        ) {
            return 100;
        }

        // Medium confidence for less common archive formats
        if matches!(
            extension,
            "lz" | "lzma" | "z" | "lzh" | "cab" | "iso"
        ) {
            return 80;
        }

        // Lower confidence for package formats that might be handled by other classifiers
        if matches!(
            extension,
            "dmg" | "pkg" | "deb" | "rpm" | "apk" | "jar" | "war" | "ear"
        ) {
            return 60;
        }

        // MIME type based confidence
        if mime_type.contains("zip") 
            || mime_type.contains("tar")
            || mime_type.contains("compressed")
            || mime_type.contains("archive") {
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

        let subcategory = match ext.as_str() {
            "zip" => ArchiveSubcategory::Zip,
            "tar" => ArchiveSubcategory::Tar,
            "gz" | "tgz" => ArchiveSubcategory::Gz,
            "rar" => ArchiveSubcategory::Rar,
            "7z" => ArchiveSubcategory::SevenZ,
            "bz2" | "tbz2" => ArchiveSubcategory::Bz2,
            "xz" | "txz" => ArchiveSubcategory::Xz,
            _ => ArchiveSubcategory::Other,
        };

        let mut classified = ClassifiedFileMetadata::new(
            path.to_path_buf(),
            FileCategory::Archives(subcategory),
        );
        classified.mime_type = Some(mime);
        classified.file_size = Some(size);
        classified.year = year;

        Ok(classified)
    }
}