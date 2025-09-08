use std::path::Path;

use clap::Parser;
use file_organizer::{cli::{Args, Commands}, utils::init_tracing, organizer::organise_files, reverter::revert_files};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    
    let args = Args::parse();
    
    match args.cmd {
        Commands::Organize { path, watch, dry_run } => {
            if watch {
                // TODO: Implement file watching
                println!("Watch mode not yet implemented");
            } else {
                organise_files(Path::new(&path), dry_run).await?;
            }
        }
        Commands::Revert { root_dir, no_cleanup } => {
            // Pass `!no_cleanup` so default = true
            revert_files(&root_dir, !no_cleanup).await?;
        }
    }
    
    Ok(())
}