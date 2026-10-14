//! The headless setting checker (`dev-ui check`), and the error lines the window shares with it.

use std::ffi::OsStr;
use std::path::Path;
use std::process::ExitCode;

use policyspace_engine::{
    GoodId, LoadErrors, LoadedSetting, Quantity, TurnReport, World, load_setting,
};

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

/// The lines reported for a failed load: each error's `Display`, in order, then
/// `"{n} error(s) found in {folder.display()}"`. Used by `run` (printed to stderr)
/// and by the window's error view, so both show the same lines.
pub fn error_lines(errors: &LoadErrors, folder: &Path) -> Vec<String> {
    let mut lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
    lines.push(format!(
        "{} error(s) found in {}",
        errors.len(),
        folder.display()
    ));
    lines
}

/// "1 hex", "19 hexes": the count with the singular or plural word. `pub(crate)` because
/// the window's status bar uses it too.
pub(crate) fn count(n: usize, singular: &str, plural: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_lines_lists_each_error_then_a_summary() {
        let folder =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../engine/tests/settings/many_errors");
        let errors = load_setting(&folder).expect_err("many_errors has errors");
        let lines = error_lines(&errors, &folder);
        assert_eq!(
            lines,
            vec![
                "setting.toml:2:18: unsupported format_version 3 (this engine supports format version 1)".to_string(),
                "rules/a.toml:1:3: unknown section 'widget'".to_string(),
                "rules/b.txt: unexpected file (only .toml files and README.md are allowed here)".to_string(),
                "start/: missing required folder".to_string(),
                format!("4 error(s) found in {}", folder.display()),
            ]
        );
    }
}
