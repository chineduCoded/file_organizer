#[cfg(test)]
mod tests {
    use std::path::Path;
    use file_organizer::metadata::{
        ArchiveSubcategory, AudioSubcategory, ClassifiedFileMetadata, CodeSubcategory,
        DocumentSubcategory, ExecutableSubcategory, FileCategory, ImageSubcategory, VideoSubcategory,
    };
    use file_organizer::path_builder::PathBuilder;

    #[test]
    fn test_document_subcategory_as_ref() {
        assert_eq!(DocumentSubcategory::Pdf.as_ref(), "Pdf");
        assert_eq!(DocumentSubcategory::Spreadsheet.as_ref(), "Spreadsheet");
        assert_eq!(DocumentSubcategory::Other.as_ref(), "Other");
    }

    #[test]
    fn test_image_subcategory_as_ref() {
        assert_eq!(ImageSubcategory::Jpeg.as_ref(), "Jpeg");
        assert_eq!(ImageSubcategory::Webp.as_ref(), "Webp");
        assert_eq!(ImageSubcategory::Other.as_ref(), "Other");
    }

    #[test]
    fn test_video_subcategory_as_ref() {
        assert_eq!(VideoSubcategory::Mp4.as_ref(), "Mp4");
        assert_eq!(VideoSubcategory::ThreeGp.as_ref(), "3Gp");
        assert_eq!(VideoSubcategory::Other.as_ref(), "Other");
    }

    #[test]
    fn test_audio_subcategory_as_ref() {
        assert_eq!(AudioSubcategory::Mp3.as_ref(), "Mp3");
        assert_eq!(AudioSubcategory::Flac.as_ref(), "Flac");
        assert_eq!(AudioSubcategory::Other.as_ref(), "Other");
    }

    #[test]
    fn test_archive_subcategory_as_ref() {
        assert_eq!(ArchiveSubcategory::Zip.as_ref(), "Zip");
        assert_eq!(ArchiveSubcategory::SevenZ.as_ref(), "7z");
        assert_eq!(ArchiveSubcategory::Other.as_ref(), "Other");
    }

    #[test]
    fn test_executable_subcategory_as_ref() {
        assert_eq!(ExecutableSubcategory::WindowsApp.as_ref(), "WindowsApp");
        assert_eq!(ExecutableSubcategory::Script.as_ref(), "Script");
        assert_eq!(ExecutableSubcategory::Other.as_ref(), "Other");
    }

    #[test]
    fn test_code_subcategory_as_ref() {
        assert_eq!(CodeSubcategory::Rust.as_ref(), "Rust");
        assert_eq!(CodeSubcategory::JavaScript.as_ref(), "JavaScript");
        assert_eq!(CodeSubcategory::Dockerfile.as_ref(), "Dockerfile");

        // Custom "Other"
        let other = CodeSubcategory::Other("CustomLang".to_string());
        assert_eq!(other.as_ref(), "CustomLang");
    }

    #[test]
    fn test_pathbuilder_documents_with_year() {
        let meta = ClassifiedFileMetadata {
            category: FileCategory::Documents(DocumentSubcategory::Pdf),
            year: Some(2024),
            ..Default::default()
        };

        let path = PathBuilder::new(&meta).build();
        assert_eq!(path, Path::new("Organized/Documents/Pdf/2024"));
    }

    #[test]
    fn test_pathbuilder_images_custom_base() {
        let meta = ClassifiedFileMetadata {
            category: FileCategory::Images(ImageSubcategory::Jpeg),
            year: None,
            ..Default::default()
        };

        let path = PathBuilder::new(&meta)
            .base(Path::new("/tmp"))
            .build();
        assert_eq!(path, Path::new("/tmp/Images/Jpeg"));
    }

    #[test]
    fn test_pathbuilder_others_category() {
        let meta = ClassifiedFileMetadata {
            category: FileCategory::Others,
            year: None,
            ..Default::default()
        };

        let path = PathBuilder::new(&meta).build();
        assert_eq!(path, Path::new("Organized/Others"));
    }
}