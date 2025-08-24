// use clap::Parser;
// use file_organizer::{cli::{Args, Commands}, utils::init_tracing};

// fn main() {
    // init_tracing();

    // let args = Args::parse();
    
    // match args.cmd {
    //     Commands::Organize { watch, dry_run } => {},
    // }
// }

use file_organizer::{classifier::Classifier, config::RulesConfig};

fn main() -> anyhow::Result<()> {
    let rules = RulesConfig::load_from_file("rules/default_rules.json")?;
    let classifier = Classifier::new(rules);

    let files = [
        "holiday.jpg", 
        "report_2025.pdf", 
        "invoice_12345.csv", 
        "unknown.xyz",
        "presentation.pptx",
        "data_sheet.xlsx",
        "music_song.mp3",
        "archive.zip",
        "document.docx",
        "video.mp4",
    ];
    
    for f in files {
        match classifier.classify(f) {
            Some(dest) => println!("{f} → {dest}"),
            None => println!("{f} → Others"),
        }
    }

    Ok(())
}

