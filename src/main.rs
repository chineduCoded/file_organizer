use std::path::Path;

use clap::Parser;
use stash::{cli::{Args, Commands}, utils::init_tracing, organizer::organise_files, reverter::revert_files};

fn main() -> anyhow::Result<()> {
    init_tracing();
    let args = Args::parse();

    tokio::runtime::Runtime::new()?.block_on(async {
        match args.cmd {
            Commands::Organize { path, watch, dry_run } => {
                if watch {
                    println!("Watch mode not yet implemented");
                } else {
                    organise_files(Path::new(&path), dry_run).await?;
                }
            }
            Commands::Revert { root_dir, no_cleanup } => {
                revert_files(&root_dir, !no_cleanup).await?;
            }
        }
        Ok(())
    })
}
