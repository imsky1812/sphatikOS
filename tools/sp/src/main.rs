//! `sp`: the Sphatik OS developer tool.
//!
//! One command for every build, flash and debug task (handbook, "Repository
//! layout and build environment").

mod build;
mod cli;

use std::process::ExitCode;

use clap::Parser;

use cli::{Cli, Command};

/// Exit code for commands that exist but are not available yet.
const EXIT_UNAVAILABLE: u8 = 2;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Build(args) => build::run(&args),
        Command::Flash(_) => {
            eprintln!("sp: not implemented yet");
            ExitCode::from(EXIT_UNAVAILABLE)
        }
        Command::Logs(_) => {
            eprintln!(
                "sp logs: needs SSH over USB to the phone, which arrives in WP 1.7. Not available yet."
            );
            ExitCode::from(EXIT_UNAVAILABLE)
        }
    }
}
