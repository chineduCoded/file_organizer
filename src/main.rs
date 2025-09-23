use std::path::Path;

use clap::Parser;
use stash::{cli::{Args, Commands, DbCommands}, index::Db, organizer::organise_files, reverter::revert_files, utils::{default_db_path, expand_tilde, init_tracing}};

fn main() -> anyhow::Result<()> {
    init_tracing();
    let args = Args::parse();

    tokio::runtime::Runtime::new()?.block_on(async {
        match args.cmd {
            Commands::Organize { path, watch, dry_run } => {
                if watch {
                    println!("Watch mode not yet implemented");
                } else {
                    let path = expand_tilde(path.to_str().unwrap());
                    println!("Expanded path: {:?}", path);
                    organise_files(Path::new(&path), dry_run).await?;

                    // Every Nth run, vacuum the DB
                    let db_path = default_db_path()?;
                    let db = Db::new(&db_path).await?;
                    if rand::random::<u8>() % 20 == 0  {
                        if let Err(e) = db.vacuum().await {
                            tracing::warn!(%e, "Auto-vacuum failed");
                        }
                    }
                }
            }
            Commands::Revert { root_dir, no_cleanup } => {
                let root_dir = expand_tilde(root_dir.to_str().unwrap());
                println!("Expanded path: {:?}", root_dir);
                revert_files(&root_dir, !no_cleanup).await?;
            }
            Commands::Db { action } => {
                match action {
                    DbCommands::Vacuum => {
                        let db_path = default_db_path()?;
                        let db = Db::new(&db_path).await?;
                        db.vacuum().await?;
                        println!("✅ Database vacuum completed.");
                    }
                    DbCommands::Status => {
                        let db_path = default_db_path()?;
                        Db::status(&db_path).await?;
                    }
                }
            }
        }
        Ok(())
    })
}
