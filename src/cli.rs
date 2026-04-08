use std::path::PathBuf;

use clap::{ArgGroup, Args, Parser, Subcommand};

use crate::provider::ProviderKind;

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
    Ask(AskArgs),
    Provider(ProviderArgs),
    Session(SessionArgs),
}

#[derive(Debug, Args)]
pub struct AskArgs {
    #[arg(
        long = "file",
        short = 'f',
        help = "Attach one or more dataset files to the request"
    )]
    pub files: Vec<PathBuf>,
    pub query: String,
}

#[derive(Debug, Args)]
pub struct ProviderArgs {
    #[command(subcommand)]
    pub command: ProviderCommand,
}

#[derive(Debug, Subcommand)]
pub enum ProviderCommand {
    List,
    Status,
    SetApiKey(SetApiKeyArgs),
}

#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("input")
        .required(true)
        .args(["api_key", "stdin"])
))]
pub struct SetApiKeyArgs {
    pub provider: ProviderKind,
    #[arg(long, help = "API key value to persist locally")]
    pub api_key: Option<String>,
    #[arg(long, help = "Read API key from stdin")]
    pub stdin: bool,
}

#[derive(Debug, Args)]
pub struct SessionArgs {
    #[command(subcommand)]
    pub command: SessionCommand,
}

#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    Show,
    Clear,
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

#[cfg(test)]
mod tests {
    use super::{Cli, Command};

    #[test]
    fn ask_parses_multiple_file_flags() {
        let cli = <Cli as clap::Parser>::parse_from([
            "geocode",
            "ask",
            "--file",
            "base.nc",
            "--file",
            "scenario.tif",
            "compare these",
        ]);

        match cli.command {
            Command::Ask(args) => {
                assert_eq!(args.files.len(), 2);
                assert_eq!(args.files[0].to_string_lossy(), "base.nc");
                assert_eq!(args.files[1].to_string_lossy(), "scenario.tif");
                assert_eq!(args.query, "compare these");
            }
            other => panic!("expected ask command, got {other:?}"),
        }
    }
}
