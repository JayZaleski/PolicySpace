//! The headless setting checker (`dev-ui check`).

use std::ffi::OsStr;
use std::path::Path;
use std::process::ExitCode;

use policyspace_engine::{LoadedSetting, World, load_setting};

use crate::text::{count, error_lines, turn_line};

/// Loads the setting at `path` and prints the checker's output; runs `turns` turns if given.
/// Exit codes: 0 success, 1 setting errors (or a step error).
pub fn run(path: &OsStr, turns: Option<u32>) -> ExitCode {
    match load_setting(path) {
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
            for line in error_lines(&errors, Path::new(path)) {
                eprintln!("{line}");
            }
            ExitCode::from(1)
        }
    }
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
