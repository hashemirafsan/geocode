use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "geocode")]
#[command(about = "Local-first geospatial analysis CLI")]
pub struct Cli {
    #[arg(long, global = true, help = "Render structured JSON output")]
    pub json: bool,
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Inspect(FileArgs),
    Mean(VariableArgs),
    Compare(CompareArgs),
}

#[derive(Debug, Args)]
pub struct FileArgs {
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct VariableArgs {
    pub file: PathBuf,
    #[arg(
        long,
        help = "Variable name for formats that require explicit selection"
    )]
    pub var: Option<String>,
}

#[derive(Debug, Args)]
pub struct CompareArgs {
    pub file_a: PathBuf,
    pub file_b: PathBuf,
    #[arg(
        long,
        help = "Variable name for formats that require explicit selection"
    )]
    pub var: Option<String>,
}
