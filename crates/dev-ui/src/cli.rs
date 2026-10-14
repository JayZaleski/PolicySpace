//! The command line: which form the program was run in, and the usage message.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// What the command line asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    /// Open the window, with this setting folder loaded if given.
    Window {
        /// The setting folder to load, if one was given.
        folder: Option<PathBuf>,
    },
    /// Run the headless setting checker, then `turns` turns if given.
    Check {
        /// The setting folder to check.
        path: OsString,
        /// How many turns to run after a successful check, if asked.
        turns: Option<u32>,
    },
    /// Print the engine's name and version.
    Version,
}

/// The usage message (printed to stderr on wrong usage).
pub const USAGE: &str = "usage: dev-ui [SETTING_FOLDER]\n       dev-ui check SETTING_FOLDER [--turns N]\n       dev-ui --version";

/// The most turns `--turns` accepts.
pub const MAX_TURNS: u32 = 10_000;

/// Parses the arguments (without the program name), or None if they're wrong.
pub fn parse_args(args: &[OsString]) -> Option<Command> {
    match args {
        [] => Some(Command::Window { folder: None }),
        [flag] if flag == "--version" => Some(Command::Version),
        [word, folder] if word == "check" && is_folder(folder) => Some(Command::Check {
            path: folder.clone(),
            turns: None,
        }),
        [word, folder, flag, n] if word == "check" && is_folder(folder) && flag == "--turns" => {
            Some(Command::Check {
                path: folder.clone(),
                turns: Some(parse_turns(n)?),
            })
        }
        [folder] if is_folder(folder) => Some(Command::Window {
            folder: Some(PathBuf::from(folder)),
        }),
        _ => None,
    }
}

/// True if the argument can be a setting folder: it isn't the word `check` and doesn't
/// start with `-` (so a flag is never taken for a folder).
fn is_folder(arg: &OsStr) -> bool {
    arg != "check" && !arg.as_encoded_bytes().starts_with(b"-")
}

/// Parses the number after `--turns`: non-empty ASCII digits only, at most `MAX_TURNS`.
fn parse_turns(n: &OsStr) -> Option<u32> {
    let n = n.to_str()?;
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let value: u32 = n.parse().ok()?;
    (value <= MAX_TURNS).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Parses string arguments.
    fn parse(args: &[&str]) -> Option<Command> {
        let args: Vec<OsString> = args.iter().map(OsString::from).collect();
        parse_args(&args)
    }

    #[test]
    fn no_arguments_open_the_window_with_nothing_loaded() {
        assert_eq!(parse(&[]), Some(Command::Window { folder: None }));
    }

    #[test]
    fn one_folder_opens_the_window_with_it() {
        assert_eq!(
            parse(&["settings/sample"]),
            Some(Command::Window {
                folder: Some(PathBuf::from("settings/sample"))
            })
        );
    }

    #[test]
    fn version_is_recognised() {
        assert_eq!(parse(&["--version"]), Some(Command::Version));
    }

    #[test]
    fn check_takes_a_folder_and_optional_turns() {
        assert_eq!(
            parse(&["check", "x"]),
            Some(Command::Check {
                path: OsString::from("x"),
                turns: None
            })
        );
        assert_eq!(
            parse(&["check", "x", "--turns", "7"]),
            Some(Command::Check {
                path: OsString::from("x"),
                turns: Some(7)
            })
        );
    }

    #[test]
    fn wrong_arguments_are_rejected() {
        let cases: [&[&str]; 10] = [
            &["check"],
            &["--bogus"],
            &["-x"],
            &["check", "-x"],
            &["check", "--turns", "2"],
            &["a", "b"],
            &["--version", "x"],
            &["check", "x", "--turns"],
            &["check", "x", "--turns", "10001"],
            &["check", "x", "--steps", "1"],
        ];
        for case in cases {
            assert!(parse(case).is_none(), "{case:?}");
        }
    }
}
