use phf::{phf_map, phf_set};

use crate::metadata::CodeSubcategory;

// --- Extensions → Subcategory mapping ---
pub static EXTENSION_MAP: phf::Map<&'static str, CodeSubcategory> = phf_map! {
    // Programming Languages
    "rs" => CodeSubcategory::Rust,
    "py" => CodeSubcategory::Python,
    "js" => CodeSubcategory::JavaScript,
    "ts" => CodeSubcategory::TypeScript,
    "java" => CodeSubcategory::Java,
    "c" => CodeSubcategory::C,
    "cpp" => CodeSubcategory::Cpp,
    "go" => CodeSubcategory::Go,
    "php" => CodeSubcategory::Php,
    "swift" => CodeSubcategory::Swift,
    "kt" => CodeSubcategory::Kotlin,
    "kts" => CodeSubcategory::Kotlin,
    "scala" => CodeSubcategory::Scala,
    "rb" => CodeSubcategory::Ruby,
    "pl" => CodeSubcategory::Perl,
    "pm" => CodeSubcategory::Perl,
    "lua" => CodeSubcategory::Lua,
    "hs" => CodeSubcategory::Haskell,
    "dart" => CodeSubcategory::Dart,

    // Web Technologies
    "html" => CodeSubcategory::Html,
    "htm" => CodeSubcategory::Html,
    "css" => CodeSubcategory::Css,
    "scss" => CodeSubcategory::Scss,
    "sass" => CodeSubcategory::Sass,
    "less" => CodeSubcategory::Less,
    "styl" => CodeSubcategory::Stylus,

    // Config & Data
    "json" => CodeSubcategory::Json,
    "yaml" => CodeSubcategory::Yaml,
    "yml" => CodeSubcategory::Yaml,
    "toml" => CodeSubcategory::Toml,
    "xml" => CodeSubcategory::Xml,
    "ini" => CodeSubcategory::Ini,
    "conf" => CodeSubcategory::Ini,
    "properties" => CodeSubcategory::Properties,

    // Database
    "sql" => CodeSubcategory::Sql,
    "plsql" => CodeSubcategory::Plsql,
    "tsql" => CodeSubcategory::Tsql,

    // Build & Automation
    "makefile" => CodeSubcategory::Makefile,
    "mk" => CodeSubcategory::Makefile,
    "dockerfile" => CodeSubcategory::Dockerfile,
    "dockerignore" => CodeSubcategory::DockerIgnore,
    "gitignore" => CodeSubcategory::GitIgnore,

    // Documentation
    "md" => CodeSubcategory::Markdown,
    "markdown" => CodeSubcategory::Markdown,
    "rst" => CodeSubcategory::RestructuredText,
};

// --- Just extensions (for quick membership checks) ---
pub static CODE_EXTENSIONS: phf::Set<&'static str> = phf_set! {
    "rs", "py", "js", "ts", "java", "c", "cpp", "go", "php", "swift",
    "kt", "kts", "scala", "rb", "pl", "pm", "lua", "hs", "dart",
    "html", "htm", "css", "scss", "sass", "less", "styl",
    "json", "yaml", "yml", "toml", "xml", "ini", "conf", "properties",
    "sql", "plsql", "tsql",
    "makefile", "mk", "dockerfile", "dockerignore", "gitignore",
    "md", "markdown", "rst",
};

// --- MIME patterns: no need for phf, just a slice ---
pub static CODE_MIME_PATTERNS: &[&str] = &[
    "text/x-", "application/x-", "application/json", "application/xml",
    "text/javascript", "application/javascript", "text/css", "text/html",
    "application/yaml", "application/x-toml", "application/sql",
];
