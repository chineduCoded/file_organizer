use std::path::Path;

use async_trait::async_trait;

use crate::{
    errors::Result, metadata::{ClassifiedFileMetadata, DocumentSubcategory, FileCategory}, registry::Classifier, utils::{detect_mime, system_time_to_year}
};

pub struct DocumentClassifier;

#[async_trait]
impl Classifier for DocumentClassifier {
    fn name(&self) -> &'static str {
        "DocumentClassifier"
    }

    fn confidence(&self, extension: &str, mime_type: &str) -> u8 {
        // High confidence for specific document extensions
        if matches!(
            extension,
            "pdf" | "doc" | "docx" | "ppt" | "pptx" | "xls" | "xlsx" | 
            "odt" | "ods" | "odp" | "docm" | "dotx" | "dotm" | "xlsm" |
            "xltx" | "xltm" | "pptm" | "potx" | "potm" | "ppsx" | "ppsm" | "epub"
        ) {
            return 100;
        }

        // Medium confidence for text-based formats that might overlap with code
        if matches!(
            extension,
            "txt" | "rtf" | "md" | "markdown" | "tex" | "ltx" | "sty" | "cls" | "bib" |
            "odg" | "odf" | "csv"
        ) {
            return 80;
        }

        // MIME type based confidence
        if mime_type.starts_with("application/vnd.") ||
           mime_type.contains("word") ||
           mime_type.contains("spreadsheet") ||
           mime_type.contains("presentation") ||
           mime_type.contains("opendocument") ||
           mime_type.contains("officedocument") ||
           mime_type == "application/pdf" ||
           mime_type == "application/epub+zip" {
            return 90;
        }

        // Lower confidence for generic text types
        if mime_type.starts_with("text/") {
            return 70;
        }

        0
    }

    async fn extract_metadata(&self, path: &Path) -> Result<ClassifiedFileMetadata> {
        let raw = tokio::fs::metadata(path).await?;
        let size = raw.len();
        
        // Get file extension for MIME detection
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        
        let mime = detect_mime(ext);

        let year = raw
            .modified()
            .ok()
            .or_else(|| raw.created().ok())
            .and_then(|t| system_time_to_year(t));

        // Subcategory
        let subcategory = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| match ext.to_ascii_lowercase().as_str() {
                "pdf" => DocumentSubcategory::Pdf,
                "doc" | "docx" | "docm" | "dotx" | "dotm" | "odt" | "rtf" => {
                    DocumentSubcategory::Word
                }
                "xls" | "xlsx" | "xlsm" | "xltx" | "xltm" | "ods" | "csv" => {
                    DocumentSubcategory::Spreadsheet
                }
                "ppt" | "pptx" | "pptm" | "potx" | "potm" | "ppsx" | "ppsm" | "odp" => {
                    DocumentSubcategory::Presentation
                }
                "txt" | "md" | "markdown" => DocumentSubcategory::Text,
                "tex" | "ltx" | "sty" | "cls" | "bib" => DocumentSubcategory::Technical,
                "odg" | "odf" => DocumentSubcategory::OpenDocument,
                "epub" => DocumentSubcategory::Ebook,
                _ => DocumentSubcategory::Other,
            })
            .unwrap_or(DocumentSubcategory::Other);

        let mut classified = ClassifiedFileMetadata::new(
            path.to_path_buf(),
            FileCategory::Documents(subcategory),
        );
        classified.mime_type = Some(mime);
        classified.file_size = Some(size);
        classified.year = year;

        Ok(classified)
    }
}