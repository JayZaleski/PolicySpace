//! Placeholder developer UI. Step 07 replaces this with a real developer UI.
//!
//! With no argument it only prints the engine's name and version, which shows that the UI
//! crate builds against the engine's public interface. Given the path to a setting folder
//! it works as a setting checker: it loads the setting through the engine's public API,
//! and either prints the setting's name, format version and counts of what it defines
//! (so far, goods, hexes, resources, and deposits), or prints every error found and exits with a failure code.
//!
//! Usage: `dev-ui [SETTING_FOLDER [--turns N]]`. With `--turns N` (a whole number from 0
//! to 10,000), a setting that loads is then run as a world: the checker prints turn 0's
//! line, steps the world `N` times, and prints a line for each turn:
//! `Turn 3: perished 10 copper; emitted 10 copper, 24 grain`.
//!
//! Exit codes: 0 = success; 1 = the setting has errors; 2 = wrong usage.

use std::ffi::OsString;
use std::process::ExitCode;

use policyspace_engine::{
    ENGINE_NAME, ENGINE_VERSION, GoodId, LoadedSetting, Quantity, TurnReport, World, load_setting,
};

/// The most turns `--turns` accepts.
const MAX_TURNS: u32 = 10_000;

/// The usage message.
const USAGE: &str = "usage: dev-ui [SETTING_FOLDER [--turns N]]";

/// What the command line asks for.
enum Command {
    /// No arguments: print the engine's name and version.
    Version,
    /// Check a setting, then run it for `turns` turns (if given).
    Check { path: OsString, turns: Option<u32> },
}

/// Parses the arguments, or None if they're wrong.
fn parse_args(args: &[OsString]) -> Option<Command> {
    match args {
        [] => Some(Command::Version),
        [path] => Some(Command::Check {
            path: path.clone(),
            turns: None,
        }),
        [path, flag, n] if flag == "--turns" => Some(Command::Check {
            path: path.clone(),
            turns: Some(parse_turns(n)?),
        }),
        _ => None,
    }
}

/// Parses the number after `--turns`: non-empty ASCII digits only, at most `MAX_TURNS`.
fn parse_turns(n: &OsString) -> Option<u32> {
    let n = n.to_str()?;
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: u32 = n.parse().ok()?;
    (value <= MAX_TURNS).then_some(value)
}

/// "1 good", "0 goods", "5 goods": the count with the singular or plural word.
fn count(n: usize, singular: &str, plural: &str) -> String {
    let word = if n == 1 { singular } else { plural };
    format!("{n} {word}")
}

/// "10 copper, 24 grain", or "nothing" for an empty list.
fn list(world: &World, entries: &[(GoodId, Quantity)]) -> String {
    if entries.is_empty() {
        return "nothing".to_string();
    }
    entries
        .iter()
        .map(|(good, amount)| format!("{amount} {}", world.rules().goods.name(*good)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The line printed for a turn.
fn turn_line(world: &World, report: &TurnReport) -> String {
    format!(
        "Turn {}: perished {}; emitted {}",
        report.turn,
        list(world, &report.perished),
        list(world, &report.emitted)
    )
}

/// Builds the world, prints turn 0's line, then steps `turns` times, printing each line.
fn run_turns(setting: LoadedSetting, turns: u32) -> ExitCode {
    let (mut world, report) = World::new(setting);
    println!("{}", turn_line(&world, &report));
    for _ in 0..turns {
        match world.step() {
            Ok(report) => println!("{}", turn_line(&world, &report)),
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}

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
        Command::Check { path, turns } => match load_setting(&path) {
            Ok(setting) => {
                println!(
                    "Loaded setting '{}' (format version {}): {}, {}, {}, {}",
                    setting.manifest.name,
                    setting.manifest.format_version,
                    count(setting.rules.goods.len(), "good", "goods"),
                    count(setting.start.map.len(), "hex", "hexes"),
                    count(setting.rules.resources.len(), "resource", "resources"),
                    count(setting.start.deposits.len(), "deposit", "deposits")
                );
                match turns {
                    Some(turns) => run_turns(setting, turns),
                    None => ExitCode::SUCCESS,
                }
            }
            Err(errors) => {
                for error in &errors {
                    eprintln!("{error}");
                }
                eprintln!(
                    "{} error(s) found in {}",
                    errors.len(),
                    std::path::Path::new(&path).display()
                );
                ExitCode::from(1)
            }
        },
    }
}
