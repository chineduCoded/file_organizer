use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Commands
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    Organize {
        /// Root directory to organize
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Enable watch mode
        #[arg(short, long)]
        watch: bool,

        /// Perform a dry run without moving files
        #[arg(short, long)]
        dry_run: bool,
    },
    Revert {
        /// Root directory to revert to
        root_dir: PathBuf,
    },
}