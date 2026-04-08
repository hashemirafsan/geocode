mod agent;
mod app;
mod cli;
mod engine;
mod output;
mod provider;
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
