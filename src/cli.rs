use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "sptrace")]
#[command(version)]
#[command(about = "Offline Stored Procedure analyzer for legacy SQL systems")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Scan {
        path: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, num_args = 0..=1, default_missing_value = "mermaid", value_name = "FORMAT")]
        diagram: Option<String>,
    },
    Context {
        path: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Diff {
        before: PathBuf,
        after: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}
