//! The headless setting checker (`dev-ui check`).

use std::ffi::OsStr;
use std::path::Path;
use std::process::ExitCode;

use policyspace_engine::decider::python;
use policyspace_engine::{LoadedSetting, TurnReport, World, load_setting};

use crate::text::{count, error_lines, failure_lines, load_failure_lines, turn_line};

/// Loads the setting at `path` and prints the checker's output; runs `turns` turns if given.
/// Exit codes: 0 success, 1 setting errors (or a step error).
pub fn run(path: &OsStr, turns: Option<u32>) -> ExitCode {
    match load_setting(path) {
        Ok(setting) => {
            println!(
                "Loaded setting '{}' (format version {}): {}, {}, {}, {}, {}, {}, {}, {}, {}, {}",
                setting.manifest.name,
                setting.manifest.format_version,
                count(setting.rules.goods.len(), "good", "goods"),
                count(setting.start.map.len(), "hex", "hexes"),
                count(setting.rules.resources.len(), "resource", "resources"),
                count(setting.start.deposits.len(), "deposit", "deposits"),
                count(setting.start.prices.market_count(), "market", "markets"),
                count(setting.rules.pop_types.len(), "pop type", "pop types"),
                count(setting.start.pops.len(), "pop", "pops"),
                count(
                    setting.rules.technologies.len(),
                    "technology",
                    "technologies"
                ),
                count(setting.start.businesses.len(), "business", "businesses"),
                count(setting.start.factories.len(), "factory", "factories")
            );
            match turns {
                Some(turns) => run_turns(setting, turns, Path::new(path)),
                None => ExitCode::SUCCESS,
            }
        }
        Err(errors) => {
            for line in error_lines(&errors, Path::new(path)) {
                eprintln!("{line}");
            }
            ExitCode::from(1)
        }
    }
}

/// Starts the setting's deciders, builds the world, prints turn 0's line, then steps
/// `turns` times, printing each line followed by the lines of each decider failure. Deciders
/// that can't start are reported like load errors (exit code 1).
fn run_turns(setting: LoadedSetting, turns: u32, path: &Path) -> ExitCode {
    let mut host = match python::host_for(&setting) {
        Ok(host) => host,
        Err(error) => return load_failure(&error.to_string(), path),
    };
    let (mut world, report) = match World::new(setting, &mut host) {
        Ok(created) => created,
        Err(error) => return load_failure(&error.to_string(), path),
    };
    print_turn(&world, &report);
    for _ in 0..turns {
        match world.step(&mut host) {
            Ok(report) => print_turn(&world, &report),
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::from(1);
            }
        }
    }
    ExitCode::SUCCESS
}

/// Prints a turn's line, then the lines of each decider failure in it.
fn print_turn(world: &World, report: &TurnReport) {
    println!("{}", turn_line(world, report));
    for failure in &report.decider_failures {
        for line in failure_lines(report.turn, failure) {
            println!("{line}");
        }
    }
}

/// Prints a failure to start the deciders like load errors, to stderr, and returns exit
/// code 1.
fn load_failure(message: &str, path: &Path) -> ExitCode {
    for line in load_failure_lines(message, path) {
        eprintln!("{line}");
    }
    ExitCode::from(1)
}
