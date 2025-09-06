use phf::{phf_map, phf_set};

use crate::metadata::ExecutableSubcategory;

// --- Extension → Subcategory map ---
pub static EXECUTABLE_EXTENSION_MAP: phf::Map<&'static str, ExecutableSubcategory> = phf_map! {
    // Windows applications
    "exe" => ExecutableSubcategory::WindowsApp,
    "msi" => ExecutableSubcategory::WindowsApp,
    "dll" => ExecutableSubcategory::WindowsApp,

    // macOS applications
    "app" => ExecutableSubcategory::MacApp,
    "dmg" => ExecutableSubcategory::MacApp,
    "pkg" => ExecutableSubcategory::MacApp,
    "dylib" => ExecutableSubcategory::MacApp,

    // Linux applications
    "deb" => ExecutableSubcategory::LinuxApp,
    "rpm" => ExecutableSubcategory::LinuxApp,
    "so" => ExecutableSubcategory::LinuxApp,
    "bin" => ExecutableSubcategory::LinuxApp,

    // Mobile applications
    "apk" => ExecutableSubcategory::MobileApp,
    "ipa" => ExecutableSubcategory::MobileApp,

    // Scripts
    "bat" => ExecutableSubcategory::Script,
    "cmd" => ExecutableSubcategory::Script,
    "sh"  => ExecutableSubcategory::Script,
    "ps1" => ExecutableSubcategory::Script,

    // Configuration files
    "conf" => ExecutableSubcategory::Config,
    "ini"  => ExecutableSubcategory::Config,

    // Log files
    "log"  => ExecutableSubcategory::Log,
};

// --- Just extensions (for quick membership check) ---
#[allow(dead_code)]
pub static EXECUTABLE_EXTENSIONS: phf::Set<&'static str> = phf_set! {
    "exe", "msi", "dll",
    "app", "dmg", "pkg", "dylib",
    "deb", "rpm", "so", "bin",
    "apk", "ipa",
    "bat", "cmd", "sh", "ps1",
    "conf", "ini",
    "log"
};

// --- MIME patterns (slice is enough, no phf needed) ---
pub static EXECUTABLE_MIME_PATTERNS: &[&str] = &[
    "application/x-msdownload",                 // Windows .exe
    "application/x-msi",                        // Windows .msi
    "application/x-executable",                 // Linux binaries
    "application/x-mach-binary",                // macOS binaries
    "application/vnd.android.package-archive",  // Android .apk
];
