use std::collections::{HashMap, HashSet};

use lazy_static::lazy_static;

use crate::metadata::CodeSubcategory;

lazy_static! {
    pub static ref EXTENSION_MAP: HashMap<&'static str, CodeSubcategory> = {
        let mut m = HashMap::new();
        
        // Programming Languages
        m.insert("rs", CodeSubcategory::Rust);
        m.insert("py", CodeSubcategory::Python);
        m.insert("js", CodeSubcategory::JavaScript);
        m.insert("ts", CodeSubcategory::TypeScript);
        m.insert("java", CodeSubcategory::Java);
        m.insert("c", CodeSubcategory::C);
        m.insert("cpp", CodeSubcategory::Cpp);
        m.insert("go", CodeSubcategory::Go);
        m.insert("php", CodeSubcategory::Php);
        m.insert("swift", CodeSubcategory::Swift);
        m.insert("kt", CodeSubcategory::Kotlin);
        m.insert("kts", CodeSubcategory::Kotlin);
        m.insert("scala", CodeSubcategory::Scala);
        m.insert("rb", CodeSubcategory::Ruby);
        m.insert("pl", CodeSubcategory::Perl);
        m.insert("pm", CodeSubcategory::Perl);
        m.insert("lua", CodeSubcategory::Lua);
        m.insert("hs", CodeSubcategory::Haskell);
        m.insert("dart", CodeSubcategory::Dart);
        
        // Web Technologies
        m.insert("html", CodeSubcategory::Html);
        m.insert("htm", CodeSubcategory::Html);
        m.insert("css", CodeSubcategory::Css);
        m.insert("scss", CodeSubcategory::Scss);
        m.insert("sass", CodeSubcategory::Sass);
        m.insert("less", CodeSubcategory::Less);
        m.insert("styl", CodeSubcategory::Stylus);
        
        // Configuration & Data
        m.insert("json", CodeSubcategory::Json);
        m.insert("yaml", CodeSubcategory::Yaml);
        m.insert("yml", CodeSubcategory::Yaml);
        m.insert("toml", CodeSubcategory::Toml);
        m.insert("xml", CodeSubcategory::Xml);
        m.insert("ini", CodeSubcategory::Ini);
        m.insert("conf", CodeSubcategory::Ini);
        m.insert("properties", CodeSubcategory::Properties);
        
        // Database
        m.insert("sql", CodeSubcategory::Sql);
        m.insert("plsql", CodeSubcategory::Plsql);
        m.insert("tsql", CodeSubcategory::Tsql);
        
        // Build & Automation
        m.insert("makefile", CodeSubcategory::Makefile);
        m.insert("mk", CodeSubcategory::Makefile);
        m.insert("dockerfile", CodeSubcategory::Dockerfile);
        m.insert("dockerignore", CodeSubcategory::DockerIgnore);
        m.insert("gitignore", CodeSubcategory::GitIgnore);
        
        // Documentation
        m.insert("md", CodeSubcategory::Markdown);
        m.insert("markdown", CodeSubcategory::Markdown);
        m.insert("rst", CodeSubcategory::RestructuredText);
        
        m
    };
    
    pub static ref CODE_EXTENSIONS: HashSet<&'static str> = {
        EXTENSION_MAP.keys().copied().collect()
    };
    
    pub static ref CODE_MIME_PATTERNS: Vec<&'static str> = vec![
        "text/x-", "application/x-", "application/json", "application/xml",
        "text/javascript", "application/javascript", "text/css", "text/html",
        "application/yaml", "application/x-toml", "application/sql"
    ];
}