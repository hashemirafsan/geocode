mod agent;
mod app;
mod bindings;
mod capability;
mod cli;
mod engine;
mod executor;
mod memory;
mod output;
mod plan;
mod policy;
mod provider;
mod runtime;
mod session;
mod tools;

use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = cli::Cli::parse_args();

    match app::run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
