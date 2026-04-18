mod agent;
mod app;
mod array;
mod auth;
mod bindings;
mod capability;
mod cli;
mod engine;
mod executor;
mod http;
mod memory;
mod output;
mod paths;
mod plan;
mod policy;
mod provider;
mod runtime;
mod session;
mod tools;
mod tui;
mod update;

use std::process::ExitCode;

fn main() -> ExitCode {
    let cli: cli::Cli = cli::Cli::parse_args();

    match app::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
