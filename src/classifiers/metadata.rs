use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Datelike};

use crate::scanner::RawFileMetadata;

#[derive(Debug, Clone)]
pub enum FileCategory {
    Documents(DocumentSubcategory),
    Images(ImageSubcategory),
    Videos(VideoSubcategory),
    Audio(AudioSubcategory),
    Archives(ArchiveSubcategory),
    Executables(ExecutableSubcategory),
    Code(CodeSubcategory),
    Others,
}

impl Default for FileCategory {
    fn default() -> Self {
        FileCategory::Others
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExtraMetadataValue {
    String(String),
    Int(i32),
    Float(f64),
    Bool(bool),
    StringArray(Vec<String>),
    Null,
}

#[derive(Debug, Clone)]
pub struct ClassifiedFileMetadata {
    pub path: PathBuf,
    pub category: FileCategory,
    pub year: Option<i32>,
    pub created_date: Option<String>,
    pub modified_date: Option<String>,
    pub file_size: Option<u64>,
    pub mime_type: Option<String>,

    pub extra: HashMap<String, ExtraMetadataValue>,
}

impl ClassifiedFileMetadata {
    pub fn new(path: PathBuf, category: FileCategory) -> Self {
        Self {
            path,
            category,
            year: None,
            created_date: None,
            modified_date: None,
            file_size: None,
            mime_type: None,
            extra: HashMap::new(),
        }
    }
}

impl From<RawFileMetadata> for ClassifiedFileMetadata {
    fn from(raw: RawFileMetadata) -> Self {
        let mime = raw.path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| mime_guess::from_ext(ext).first_or_octet_stream().essence_str().to_string());

        // Convert SystemTime -> RFC3339 string when present
        let created_date = raw
            .created
            .map(|t| DateTime::<Utc>::from(t).to_rfc3339());
        let modified_date = raw
            .modified
            .map(|t| DateTime::<Utc>::from(t).to_rfc3339());

        // Prefer modified timestamp for year, fall back to created
        let year = raw
            .modified
            .or(raw.created)
            .map(|t| DateTime::<Utc>::from(t).year());

        Self {
            path: raw.path,
            category: FileCategory::default(),
            year,
            created_date,
            modified_date,
            file_size: Some(raw.size),
            mime_type: mime,
            extra: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DocumentSubcategory {
    Pdf,
    Word,
    Spreadsheet,
    Presentation,
    Text,
    OpenDocument,
    Ebook,
    Technical,
    Other,
}

#[derive(Debug, Clone)]
pub enum ImageSubcategory {
    Jpeg,
    Png,
    Gif,
    Svg,
    Raw,
    Tiff,
    Webp,
    Bmp,
    Ico,
    Heic,
    Other,
}

#[derive(Debug, Clone)]
pub enum VideoSubcategory {
    Mp4,
    Avi,
    Mkv,
    Mov,
    Webm,
    Wmv,
    Flv,
    Mpeg,
    ThreeGp,
    Ts,
    Vob,
    Other,
}

#[derive(Debug, Clone)]
pub enum AudioSubcategory {
    Mp3,
    Wav,
    Flac,
    Aac,
    Ogg,
    M4a,
    Opus,
    Alac,
    Aiff,
    Wma,
    Other,
}

#[derive(Debug, Clone)]
pub enum ArchiveSubcategory {
    Zip,
    Tar,
    Gz,
    Rar,
    SevenZ,
    Bz2,
    Xz,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutableSubcategory {
    WindowsApp,    // .exe, .msi, .dll
    MacApp,        // .app, .dmg, .pkg, .dylib
    LinuxApp,      // .deb, .rpm, .so, .bin
    MobileApp,     // .apk, .ipa
    Script,        // .bat, .cmd, .sh, .ps1
    Config,        // .conf, .ini
    Log,           // .log
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeSubcategory {
    // Programming Languages
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Java,
    C,
    Cpp,
    Go,
    Php,
    Swift,
    Kotlin,
    Scala,
    Ruby,
    Perl,
    Lua,
    Haskell,
    Dart,
    
    // Web Technologies
    Html,
    Css,
    Scss,
    Sass,
    Less,
    Stylus,
    
    // Configuration & Data
    Json,
    Yaml,
    Toml,
    Xml,
    Ini,
    Properties,
    
    // Database
    Sql,
    Plsql,
    Tsql,
    
    // Build & Automation
    Makefile,
    Dockerfile,
    DockerIgnore,
    GitIgnore,
    
    // Documentation
    Markdown,
    RestructuredText,
    
    Other(String),
}

impl CodeSubcategory {
    pub fn language_name(&self) -> &str {
        match self {
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
            CodeSubcategory::Html => "HTML",
            CodeSubcategory::Css => "CSS",
            CodeSubcategory::Scss => "SCSS",
            CodeSubcategory::Sass => "SASS",
            CodeSubcategory::Less => "Less",
            CodeSubcategory::Stylus => "Stylus",
            CodeSubcategory::Json => "JSON",
            CodeSubcategory::Yaml => "YAML",
            CodeSubcategory::Toml => "TOML",
            CodeSubcategory::Xml => "XML",
            CodeSubcategory::Ini => "INI",
            CodeSubcategory::Properties => "Properties",
            CodeSubcategory::Sql => "SQL",
            CodeSubcategory::Plsql => "PL/SQL",
            CodeSubcategory::Tsql => "T-SQL",
            CodeSubcategory::Makefile => "Makefile",
            CodeSubcategory::Dockerfile => "Dockerfile",
            CodeSubcategory::DockerIgnore => "Docker Ignore",
            CodeSubcategory::GitIgnore => "Git Ignore",
            CodeSubcategory::Markdown => "Markdown",
            CodeSubcategory::RestructuredText => "reStructuredText",
            CodeSubcategory::Other(name) => name.as_str(),
        }
    }
    
    pub fn is_programming_language(&self) -> bool {
        matches!(
            self,
            CodeSubcategory::Rust
                | CodeSubcategory::Python
                | CodeSubcategory::JavaScript
                | CodeSubcategory::TypeScript
                | CodeSubcategory::Java
                | CodeSubcategory::C
                | CodeSubcategory::Cpp
                | CodeSubcategory::Go
                | CodeSubcategory::Php
                | CodeSubcategory::Swift
                | CodeSubcategory::Kotlin
                | CodeSubcategory::Scala
                | CodeSubcategory::Ruby
                | CodeSubcategory::Perl
                | CodeSubcategory::Lua
                | CodeSubcategory::Haskell
                | CodeSubcategory::Dart
        )
    }
    
    pub fn is_configuration(&self) -> bool {
        matches!(
            self,
            CodeSubcategory::Json
                | CodeSubcategory::Yaml
                | CodeSubcategory::Toml
                | CodeSubcategory::Xml
                | CodeSubcategory::Ini
                | CodeSubcategory::Properties
        )
    }
}

