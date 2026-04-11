use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "geocode")]
#[command(about = "Local-first geospatial analysis CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Cli(CliArgs),
}

#[derive(Debug, Args)]
pub struct CliArgs {
    #[arg(long, global = true, help = "Render structured JSON output")]
    pub json: bool,
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {
    Ask(AskArgs),
}

#[derive(Debug, Args, Clone)]
pub struct AskArgs {
    #[arg(
        long = "file",
        short = 'f',
        help = "Attach one or more dataset files to the request"
    )]
    pub files: Vec<PathBuf>,
    pub query: String,
}

#[cfg(test)]
mod tests {
    use super::{AskArgs, Cli, CliCommand, Command};

    #[test]
    fn root_defaults_to_tui_mode() {
        let cli = <Cli as clap::Parser>::parse_from(["geocode"]);
        assert!(cli.command.is_none());
    }

    #[test]
    fn cli_ask_parses_multiple_file_flags() {
        let cli = <Cli as clap::Parser>::parse_from([
            "geocode",
            "cli",
            "ask",
            "--file",
            "base.nc",
            "--file",
            "scenario.tif",
            "compare these",
        ]);

        match cli.command {
            Some(Command::Cli(args)) => match args.command {
                CliCommand::Ask(AskArgs { files, query }) => {
                    assert_eq!(files.len(), 2);
                    assert_eq!(files[0].to_string_lossy(), "base.nc");
                    assert_eq!(files[1].to_string_lossy(), "scenario.tif");
                    assert_eq!(query, "compare these");
                }
            },
            other => panic!("expected cli ask command, got {other:?}"),
        }
    }
}
