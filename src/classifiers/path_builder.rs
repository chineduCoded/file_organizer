use std::path::{Path, PathBuf};
use crate::metadata::{
    ArchiveSubcategory, AudioSubcategory, ClassifiedFileMetadata, CodeSubcategory,
    DocumentSubcategory, ExecutableSubcategory, FileCategory, ImageSubcategory, VideoSubcategory,
};

// Convert each subcategory to a string
impl AsRef<str> for DocumentSubcategory {
    fn as_ref(&self) -> &str {
        match self {
            DocumentSubcategory::Pdf => "Pdf",
            DocumentSubcategory::Word => "Word",
            DocumentSubcategory::Spreadsheet => "Spreadsheet",
            DocumentSubcategory::Presentation => "Presentation",
            DocumentSubcategory::Text => "Text",
            DocumentSubcategory::OpenDocument => "OpenDocument",
            DocumentSubcategory::Technical => "Technical",
            DocumentSubcategory::Ebook => "Ebook",
            DocumentSubcategory::Other => "Other",
        }
    }
}

impl AsRef<str> for ImageSubcategory {
    fn as_ref(&self) -> &str {
        match self {
            ImageSubcategory::Jpeg => "Jpeg",
            ImageSubcategory::Png => "Png",
            ImageSubcategory::Gif => "Gif",
            ImageSubcategory::Tiff => "Tiff",
            ImageSubcategory::Svg => "Svg",
            ImageSubcategory::Raw => "Raw",
            ImageSubcategory::Webp => "Webp",
            ImageSubcategory::Bmp => "Bmp",
            ImageSubcategory::Ico => "Ico",
            ImageSubcategory::Heic => "Heic",
            ImageSubcategory::Other => "Other",
        }
    }
}

impl AsRef<str> for VideoSubcategory {
    fn as_ref(&self) -> &str {
        match self {
            VideoSubcategory::Mp4 => "Mp4",
            VideoSubcategory::Mkv => "Mkv",
            VideoSubcategory::Avi => "Avi",
            VideoSubcategory::Mov => "Mov",
            VideoSubcategory::Webm => "Webm",
            VideoSubcategory::Flv => "Flv",
            VideoSubcategory::Mpeg => "Mpeg",
            VideoSubcategory::ThreeGp => "3Gp",
            VideoSubcategory::Ts => "Ts",
            VideoSubcategory::Vob => "Vob",
            VideoSubcategory::Wmv => "Wmv",
            VideoSubcategory::Other => "Other",
        }
    }
}

impl AsRef<str> for AudioSubcategory {
    fn as_ref(&self) -> &str {
        match self {
            AudioSubcategory::Mp3 => "Mp3",
            AudioSubcategory::Wav => "Wav",
            AudioSubcategory::Flac => "Flac",
            AudioSubcategory::Ogg => "Ogg",
            AudioSubcategory::Aac => "Aac",
            AudioSubcategory::M4a => "M4a",
            AudioSubcategory::Opus => "Opus",
            AudioSubcategory::Alac => "Alac",
            AudioSubcategory::Aiff => "Aiff",
            AudioSubcategory::Wma => "Wma",
            AudioSubcategory::Other => "Other",
        }
    }
}

impl AsRef<str> for ArchiveSubcategory {
    fn as_ref(&self) -> &str {
        match self {
            ArchiveSubcategory::Zip => "Zip",
            ArchiveSubcategory::Tar => "Tar",
            ArchiveSubcategory::Rar => "Rar",
            ArchiveSubcategory::SevenZ => "7z",
            ArchiveSubcategory::Gz => "Gz",
            ArchiveSubcategory::Bz2 => "Bz2",
            ArchiveSubcategory::Xz => "Xz",
            ArchiveSubcategory::Other => "Other",
        }
    }
}

impl AsRef<str> for ExecutableSubcategory {
    fn as_ref(&self) -> &str {
        match self {
            ExecutableSubcategory::WindowsApp => "WindowsApp",
            ExecutableSubcategory::MacApp => "MacApp",
            ExecutableSubcategory::LinuxApp => "LinuxApp",
            ExecutableSubcategory::MobileApp => "MobileApp",
            ExecutableSubcategory::Script => "Script",
            ExecutableSubcategory::Config => "Config",
            ExecutableSubcategory::Log => "Log",
            ExecutableSubcategory::Other => "Other",
        }
    }
}

impl AsRef<str> for CodeSubcategory {
    fn as_ref(&self) -> &str {
        match self {
            // Programming Languages
            CodeSubcategory::Rust => "Rust",
            CodeSubcategory::Python => "Python",
            CodeSubcategory::JavaScript => "JavaScript",
            CodeSubcategory::TypeScript => "TypeScript",
            CodeSubcategory::Java => "Java",
            CodeSubcategory::C => "C",
            CodeSubcategory::Cpp => "C++",
            CodeSubcategory::Go => "Go",
            CodeSubcategory::Php => "PHP",
            CodeSubcategory::Swift => "Swift",
            CodeSubcategory::Kotlin => "Kotlin",
            CodeSubcategory::Scala => "Scala",
            CodeSubcategory::Ruby => "Ruby",
            CodeSubcategory::Perl => "Perl",
            CodeSubcategory::Lua => "Lua",
            CodeSubcategory::Haskell => "Haskell",
            CodeSubcategory::Dart => "Dart",
            
            // Web Technologies
            CodeSubcategory::Html => "HTML",
            CodeSubcategory::Css => "CSS",
            CodeSubcategory::Scss => "SCSS",
            CodeSubcategory::Sass => "SASS",
            CodeSubcategory::Less => "Less",
            CodeSubcategory::Stylus => "Stylus",
            
            // Configuration & Data
            CodeSubcategory::Json => "JSON",
            CodeSubcategory::Yaml => "YAML",
            CodeSubcategory::Toml => "TOML",
            CodeSubcategory::Xml => "XML",
            CodeSubcategory::Ini => "INI",
            CodeSubcategory::Properties => "Properties",
            
            // Database
            CodeSubcategory::Sql => "SQL",
            CodeSubcategory::Plsql => "PL/SQL",
            CodeSubcategory::Tsql => "T-SQL",
            
            // Build & Automation
            CodeSubcategory::Makefile => "Makefile",
            CodeSubcategory::Dockerfile => "Dockerfile",
            CodeSubcategory::DockerIgnore => "DockerIgnore",
            CodeSubcategory::GitIgnore => "GitIgnore",
            
            // Documentation
            CodeSubcategory::Markdown => "Markdown",
            CodeSubcategory::RestructuredText => "reStructuredText",
            
            // Other with custom string
            CodeSubcategory::Other(name) => name.as_str(),
        }
    }
}

/// Builder for constructing a destination path
pub struct PathBuilder<'a> {
    meta: &'a ClassifiedFileMetadata,
    base: Option<&'a Path>,
}

impl<'a> PathBuilder<'a> {
    pub fn new(meta: &'a ClassifiedFileMetadata) -> Self {
        Self { meta, base: None }
    }

    pub fn base(mut self, base: &'a Path) -> Self {
        self.base = Some(base);
        self
    }

    pub fn build(self) -> PathBuf {
        let mut path = self.base.unwrap_or(Path::new("Organized")).to_path_buf();

        match &self.meta.category {
            FileCategory::Documents(_) => path.push("Documents"),
            FileCategory::Images(_) => path.push("Images"),
            FileCategory::Videos(_) => path.push("Videos"),
            FileCategory::Audio(_) => path.push("Audio"),
            FileCategory::Archives(_) => path.push("Archives"),
            FileCategory::Executables(_) => path.push("Executables"),
            FileCategory::Code(_) => path.push("Code"),
            FileCategory::Others => path.push("Others"),
        }

        // Push the subcategory string if it exists
        match &self.meta.category {
            FileCategory::Documents(sub) => path.push(sub.as_ref()),
            FileCategory::Images(sub) => path.push(sub.as_ref()),
            FileCategory::Videos(sub) => path.push(sub.as_ref()),
            FileCategory::Audio(sub) => path.push(sub.as_ref()),
            FileCategory::Archives(sub) => path.push(sub.as_ref()),
            FileCategory::Executables(sub) => path.push(sub.as_ref()),
            FileCategory::Code(sub) => path.push(sub.as_ref()),
            FileCategory::Others => {}
        }

        // Append year if available
        if let Some(year) = self.meta.year {
            path.push(year.to_string());
        }

        path
    }
}
