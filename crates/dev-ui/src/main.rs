//! The developer UI: a tool for developers, not a player UI. It uses only the engine's
//! public interface, and the engine knows nothing about it.
//!
//! Forms:
//! - `dev-ui [SETTING_FOLDER]` opens the window (with that setting loaded, if given). The
//!   window draws the setting's hex map, which can be panned and zoomed, and shows load
//!   errors; a setting can be opened and reloaded from it. The window also inspects hexes
//!   (deposits and stockpile), lists the world's goods, resource types and turn log, and
//!   steps turns.
//! - `dev-ui check SETTING_FOLDER [--turns N]` is the headless setting checker. It loads the
//!   setting through the engine's public API, and either prints the setting's name, format
//!   version and counts of what it defines (so far, goods, hexes, resources, and deposits),
//!   or prints every error found and exits with a failure code. With `--turns N` (a whole
//!   number from 0 to 10,000), a setting that loads is then run as a world: the checker
//!   prints turn 0's line, steps the world `N` times, and prints a line for each turn:
//!   `Turn 3: perished 10 copper; emitted 10 copper, 24 grain`. It never opens a window.
//!   Exit codes: 0 = success; 1 = the setting has errors; 2 = wrong usage.
//! - `dev-ui --version` prints the engine's name and version.
//!
//! Any other arguments print the usage message to stderr and exit with code 2.

mod app;
mod check;
mod cli;
mod geometry;
mod session;
#[cfg(test)]
mod test_support;
mod text;

use std::process::ExitCode;

use policyspace_engine::{ENGINE_NAME, ENGINE_VERSION};

use crate::cli::{Command, USAGE, parse_args};

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let Some(command) = parse_args(&args) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match command {
        Command::Version => {
            println!("{ENGINE_NAME} {ENGINE_VERSION}");
            ExitCode::SUCCESS
        }
        Command::Check { path, turns } => check::run(&path, turns),
        Command::Window { folder } => app::run(folder),
    }
}
