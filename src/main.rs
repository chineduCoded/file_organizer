use clap::Parser;
use file_organizer::cli::{Args, Commands};

fn main() {
    let args = Args::parse();
    
    match args.cmd {
        Commands::Organize { watch, dry_run } => {},
    }
}
