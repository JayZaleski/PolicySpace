//! Placeholder developer UI. Step 07 replaces this with a real developer UI.
//!
//! With no argument it only prints the engine's name and version, which shows that the UI
//! crate builds against the engine's public interface. Given the path to a setting folder
//! it works as a setting checker: it loads the setting through the engine's public API,
//! and either prints the setting's name, format version and counts of what it defines
//! (so far, goods), or prints every error found and exits with a failure code.
//!
//! Exit codes: 0 = success; 1 = the setting has errors; 2 = wrong usage.

use std::process::ExitCode;

use policyspace_engine::{ENGINE_NAME, ENGINE_VERSION, load_setting};

/// "1 good", "0 goods", "5 goods": the count with the singular or plural word.
fn count(n: usize, singular: &str, plural: &str) -> String {
    let word = if n == 1 { singular } else { plural };
    format!("{n} {word}")
}

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [] => {
            println!("{ENGINE_NAME} {ENGINE_VERSION}");
            ExitCode::SUCCESS
        }
        [path] => match load_setting(path) {
            Ok(setting) => {
                println!(
                    "Loaded setting '{}' (format version {}): {}",
                    setting.manifest.name,
                    setting.manifest.format_version,
                    count(setting.rules.goods.len(), "good", "goods")
                );
                ExitCode::SUCCESS
            }
            Err(errors) => {
                for error in &errors {
                    eprintln!("{error}");
                }
                eprintln!(
                    "{} error(s) found in {}",
                    errors.len(),
                    std::path::Path::new(path).display()
                );
                ExitCode::from(1)
            }
        },
        _ => {
            eprintln!("usage: dev-ui [SETTING_FOLDER]");
            ExitCode::from(2)
        }
    }
}
