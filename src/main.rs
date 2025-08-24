use clap::Parser;
use file_organizer::{cli::{Args, Commands}, utils::init_tracing};

fn main() {
    init_tracing();

    let args = Args::parse();
    
    match args.cmd {
        Commands::Organize { watch, dry_run } => {},
    }
}
