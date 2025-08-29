// use clap::Parser;
// use file_organizer::{cli::{Args, Commands}, utils::init_tracing};

// fn main() {
    // init_tracing();

    // let args = Args::parse();
    
    // match args.cmd {
    //     Commands::Organize { watch, dry_run } => {},
    // }
// }

use std::sync::Arc;

use tempfile::tempdir;

use file_organizer::{
    archive_classifier::ArchiveClassifier, audio_classifier::AudioClassifier, classifier::{ClassifierRegistry, GenericClassifier}, code_classifier::CodeClassifier, docs_classifier::DocumentClassifier, executable_classifier::ExecutableClassifier, image_classifier::ImageClassifier, path_builder::PathBuilder, scanner::{ScanConfig, Scanner, ScannerExt}, video_classifier::VideoClassifier
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Create a temporary directory
    let dir = tempdir()?;
    let root = dir.path();

    // 2. Create some dummy files inside
    let files: Vec<(&str, &[u8])> = vec![
        // Documents
        ("report.pdf", &b"%PDF-1.4\n%This is a dummy PDF file\n1 0 obj\n<<>>\nendobj\n"[..]),
        ("notes.docx", &b"PK\x03\x04\x14\x00\x00\x00\x08\x00Dummy DOCX content"[..]),
        ("table.csv", &b"col1,col2,col3\n1,2,3\n4,5,6\n7,8,9"[..]),
        
        // Audio files
        ("song.mp3", &b"ID3\x03\x00\x00\x00\x00\x00\x00Dummy MP3 audio data"[..]),
        ("track.wav", &b"RIFF\x24\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x02\x00\x44\xac\x00\x00\x10\xb1\x02\x00\x04\x00\x10\x00data\x00\x00\x00\x00"[..]),
        
        // Images
        ("photo.jpg", &b"\xFF\xD8\xFF\xE0\x00\x10JFIF\x00\x01\x01\x01\x00H\x00H\x00\x00Dummy JPEG data"[..]),
        ("image.png", &b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR\x00\x00\x00\x01\x00\x00\x00\x01\x08\x02\x00\x00\x00\x90wS\xde\x00\x00\x00\x01sRGB\x00\xae\xce\x1c\xe9\x00\x00\x00\x04gAMA\x00\x00\xb1\x8f\x0b\xfc\x61\x05\x00\x00\x00\x09pHYs\x00\x00\x0e\xc3\x00\x00\x0e\xc3\x01\xc7\x6f\xa8\x64\x00\x00\x00\x0cIDAT\x18\x57\x63\xf8\x0f\x00\x00\x00\x00\x01\x00\x01\x00\x1d\xae\xa3\xb4\x00\x00\x00\x00IEND\xaeB`\x82"[..]),
        
        // Videos
        ("video.mp4", &b"\x00\x00\x00\x18ftypmp42\x00\x00\x00\x00mp42isom\x00\x00\x00\x08freeDummy MP4 video data"[..]),
        ("movie.avi", &b"RIFF\x00\x00\x00\x00AVI LIST\x00\x00\x00\x00Dummy AVI video data"[..]),
        
        // Archives
        ("archive.zip", &b"PK\x03\x04\x14\x00\x00\x00\x08\x00Dummy ZIP archive content"[..]),
        ("compressed.tar.gz", &b"\x1f\x8b\x08\x00\x00\x00\x00\x00\x00\x03Dummy tar.gz compressed data"[..]),
        
        // Code files
        ("main.rs", &b"fn main() {\n    println!(\"Hello, world!\");\n}"[..]),
        ("script.py", &b"def hello():\n    print(\"Hello from Python!\")\n\nif __name__ == \"__main__\":\n    hello()"[..]),
        
        // Executables/Configs
        ("config.yaml", &b"database:\n  host: localhost\n  port: 5432\n  name: mydb\n\nserver:\n  port: 8080\n  debug: true"[..]),
        ("script.sh", &b"#!/bin/bash\n\necho \"Hello from Bash!\"\nls -la\n"[..]),
        
        // Other types
        ("data.json", &b"{\n  \"name\": \"test\",\n  \"value\": 42,\n  \"items\": [1, 2, 3]\n}"[..]),
        ("readme.md", &b"# Project Readme\n\nThis is a sample project with various file types.\n\n## Features\n- Multiple file formats\n- Test data generation\n- File classification testing"[..]),
    ];

    for (name, content) in files {
        std::fs::write(root.join(name), content)?;
    }

    println!("Created temp test files in: {}", root.display());

    // 3. Setup scanner
    let config = ScanConfig {
        include_hidden: false,
        include_dirs: false,
        max_depth: 3,
        allowed_extensions: None, // let all files through for testing
        ..Default::default()
    };

    let scanner = Scanner::new(root, config);

    // 4. Setup classifier registry
    let mut registry = ClassifierRegistry::new();
    // Register classifiers with appropriate base priorities
    // Higher priority = more specific/specialized classifiers
    // Lower priority = more general/fallback classifiers

    // Media classifiers (very specific, high confidence)
    registry.register_with_priority(100, Arc::new(ImageClassifier));    // High confidence for images
    registry.register_with_priority(95, Arc::new(AudioClassifier));     // High confidence for audio
    registry.register_with_priority(90, Arc::new(VideoClassifier));     // High confidence for video

    // Document classifier (specific but may overlap with code)
    registry.register_with_priority(85, Arc::new(DocumentClassifier));  // High confidence for documents

    // Code classifier (specific but may overlap with documents/executables)
    registry.register_with_priority(80, Arc::new(CodeClassifier));      // High confidence for code

    // Archive classifier (specific but may overlap with executables)
    registry.register_with_priority(75, Arc::new(ArchiveClassifier));   // Medium-high confidence for archives

    // Executable classifier (broader category, may overlap with others)
    registry.register_with_priority(70, Arc::new(ExecutableClassifier)); // Medium confidence for executables

    // Generic fallback (lowest priority, handles everything)
    registry.register_with_priority(10, Arc::new(GenericClassifier));   // Lowest confidence, fallback only

    // 5. Run pipeline
    let mut count = 0;
    
    for raw in scanner.filter_ok() {
        count += 1;
        match registry.classify(&raw).await {
            Ok(classified) => {
                let dest = PathBuilder::new(&classified).build();
                println!(
                    "{:?} -> {:?}, dest = {:?}",
                    classified.path.file_name().unwrap(),
                    classified.category,
                    dest
                );
            }
            Err(err) => {
                eprintln!("❌ Failed to classify {:?}: {}", raw.path, err);
            }
        }
    }

    println!("Total files classified: {}", count);
    Ok(())
}




