use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "geocode")]
#[command(about = "Local-first geospatial analysis CLI")]
#[command(disable_version_flag = true)]
pub struct Cli {
    #[arg(long, global = true, help = "Render structured JSON output")]
    pub json: bool,
    #[arg(long, short = 'V', global = true, help = "Show version details")]
    pub version: bool,
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
    Inspect(InspectArgs),
    Mean(MeanArgs),
    Compare(CompareArgs),
    Ask(AskArgs),
    Doctor,
    SelfUpdate,
    Version,
    #[command(hide = true)]
    Cli(LegacyCliArgs),
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

#[derive(Debug, Args, Clone)]
pub struct InspectArgs {
    pub file: PathBuf,
}

#[derive(Debug, Args, Clone)]
pub struct MeanArgs {
    pub file: PathBuf,
    #[arg(long, help = "Select variable for NetCDF datasets")]
    pub var: Option<String>,
}

#[derive(Debug, Args, Clone)]
pub struct CompareArgs {
    pub file_a: PathBuf,
    pub file_b: PathBuf,
    #[arg(long, help = "Select variable for NetCDF datasets")]
    pub var: Option<String>,
}

#[derive(Debug, Args)]
pub struct LegacyCliArgs {
    #[command(subcommand)]
    pub command: LegacyCliCommand,
}

#[derive(Debug, Subcommand)]
pub enum LegacyCliCommand {
    Ask(AskArgs),
}

#[cfg(test)]
mod tests {
    use super::{AskArgs, Cli, Command, LegacyCliCommand};

    #[test]
    fn root_defaults_to_tui_mode() {
        let cli = <Cli as clap::Parser>::parse_from(["geocode"]);
        assert!(cli.command.is_none());
        assert!(!cli.version);
    }

    #[test]
    fn version_flag_parses_as_global_flag() {
        let cli = <Cli as clap::Parser>::parse_from(["geocode", "--version"]);
        assert!(cli.version);
        assert!(cli.command.is_none());
    }

    #[test]
    fn cli_ask_parses_multiple_file_flags() {
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
            Some(Command::Ask(AskArgs { files, query })) => {
                assert_eq!(files.len(), 2);
                assert_eq!(files[0].to_string_lossy(), "base.nc");
                assert_eq!(files[1].to_string_lossy(), "scenario.tif");
                assert_eq!(query, "compare these");
            }
            other => panic!("expected ask command, got {other:?}"),
        }
    }

    #[test]
    fn legacy_cli_ask_alias_still_parses() {
        let cli = <Cli as clap::Parser>::parse_from([
            "geocode",
            "cli",
            "ask",
            "--file",
            "base.nc",
            "compare this",
        ]);

        match cli.command {
            Some(Command::Cli(args)) => match args.command {
                LegacyCliCommand::Ask(AskArgs { files, query }) => {
                    assert_eq!(files.len(), 1);
                    assert_eq!(files[0].to_string_lossy(), "base.nc");
                    assert_eq!(query, "compare this");
                }
            },
            other => panic!("expected cli ask command, got {other:?}"),
        }
    }
}
