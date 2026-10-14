//! Runs the dev UI binary's non-window forms: `--version` and the usage errors.
//!
//! No test here runs the binary with no arguments or with a single folder: those open a
//! window, which would hang the test run.

use std::process::Command;

use policyspace_engine::{ENGINE_NAME, ENGINE_VERSION};

/// The usage message the binary prints on wrong usage.
const USAGE: &str = "usage: dev-ui [SETTING_FOLDER]\n       dev-ui check SETTING_FOLDER [--turns N]\n       dev-ui --version";

#[test]
fn version_prints_engine_name_and_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_dev-ui"))
        .arg("--version")
        .output()
        .expect("failed to run the dev-ui binary");
    assert!(
        output.status.success(),
        "dev-ui exited with {:?}",
        output.status
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout was not UTF-8");
    assert_eq!(stdout.trim(), format!("{} {}", ENGINE_NAME, ENGINE_VERSION));
}

#[test]
fn wrong_usage_prints_usage_and_exits_2() {
    let cases: [&[&str]; 4] = [
        &["check"],
        &["--bogus"],
        &["check", "-x"],
        &["--version", "extra"],
    ];
    for args in cases {
        let output = Command::new(env!("CARGO_BIN_EXE_dev-ui"))
            .args(args)
            .output()
            .expect("failed to run the dev-ui binary");
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}");
        let stderr = String::from_utf8(output.stderr).expect("stderr was not UTF-8");
        assert_eq!(stderr.trim(), USAGE, "{args:?}");
    }
}
