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
        #[arg(short, long)]
        watch: bool,

        #[arg(short, long)]
        dry_run: bool,
    },
}