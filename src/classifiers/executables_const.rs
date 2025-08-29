use std::collections::HashMap;

use lazy_static::lazy_static;

use crate::metadata::ExecutableSubcategory;


lazy_static! {
    pub static ref EXECUTABLE_EXTENSION_MAP: HashMap<&'static str, ExecutableSubcategory> = {
        let mut m = HashMap::new();
        
        // Windows applications
        m.insert("exe", ExecutableSubcategory::WindowsApp);
        m.insert("msi", ExecutableSubcategory::WindowsApp);
        m.insert("dll", ExecutableSubcategory::WindowsApp);
        
        // macOS applications
        m.insert("app", ExecutableSubcategory::MacApp);
        m.insert("dmg", ExecutableSubcategory::MacApp);
        m.insert("pkg", ExecutableSubcategory::MacApp);
        m.insert("dylib", ExecutableSubcategory::MacApp);
        
        // Linux applications
        m.insert("deb", ExecutableSubcategory::LinuxApp);
        m.insert("rpm", ExecutableSubcategory::LinuxApp);
        m.insert("so", ExecutableSubcategory::LinuxApp);
        m.insert("bin", ExecutableSubcategory::LinuxApp);
        
        // Mobile applications
        m.insert("apk", ExecutableSubcategory::MobileApp);
        m.insert("ipa", ExecutableSubcategory::MobileApp);
        
        // Scripts
        m.insert("bat", ExecutableSubcategory::Script);
        m.insert("cmd", ExecutableSubcategory::Script);
        m.insert("sh", ExecutableSubcategory::Script);
        m.insert("ps1", ExecutableSubcategory::Script);
        
        // Configuration files
        m.insert("conf", ExecutableSubcategory::Config);
        m.insert("ini", ExecutableSubcategory::Config);
        
        // Log files
        m.insert("log", ExecutableSubcategory::Log);
        
        m
    };
     pub static ref EXECUTABLE_EXTENSIONS: Vec<&'static str> = {
        EXECUTABLE_EXTENSION_MAP.keys().copied().collect()
     };

    pub static ref EXECUTABLE_MIME_PATTERNS: Vec<&'static str> = vec![
        // The most common and reliable executable MIME types
        "application/x-msdownload",          // Windows .exe
        "application/x-msi",                 // Windows .msi
        "application/x-executable",          // Linux binaries
        "application/x-mach-binary",         // macOS binaries
        "application/vnd.android.package-archive", // Android .apk
    ];
}