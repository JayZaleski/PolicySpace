//! Runs the dev UI binary as a setting checker and checks its output and exit codes.
//! (With no arguments, `placeholder.rs` covers the unchanged behavior.)

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run(args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dev-ui"))
        .args(args)
        .output()
        .expect("failed to run the dev-ui binary")
}

fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

#[test]
fn checks_the_sample_setting() {
    let output = run(&[&repo_path("settings/sample")]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        "Loaded setting 'Sample Setting' (format version 1): 5 goods, 19 hexes, 2 resources, 10 deposits"
    );
}

#[test]
fn says_1_good_for_one_good_and_0_goods_for_none() {
    let one = run(&[&repo_path("crates/engine/tests/settings/goods_one")]);
    assert_eq!(one.status.code(), Some(0));
    let stdout = String::from_utf8(one.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        "Loaded setting 'Goods Test' (format version 1): 1 good, 0 hexes, 0 resources, 0 deposits"
    );

    let none = run(&[&repo_path("crates/engine/tests/settings/valid_minimal")]);
    assert_eq!(none.status.code(), Some(0));
    let stdout = String::from_utf8(none.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        "Loaded setting 'Minimal Setting' (format version 1): 0 goods, 0 hexes, 0 resources, 0 deposits"
    );
}

#[test]
fn says_1_hex_for_one_hex() {
    let output = run(&[&repo_path("crates/engine/tests/settings/hexes_one")]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        "Loaded setting 'Hexes Test' (format version 1): 0 goods, 1 hex, 0 resources, 0 deposits"
    );
}

#[test]
fn says_1_resource_and_1_deposit_for_one_of_each() {
    let output = run(&[&repo_path("crates/engine/tests/settings/resources_one")]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        "Loaded setting 'Resources Test' (format version 1): 1 good, 1 hex, 1 resource, 1 deposit"
    );
}

#[test]
fn reports_every_error_of_a_broken_setting() {
    let path = repo_path("crates/engine/tests/settings/many_errors");
    let output = run(&[&path]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    let expected = [
        "setting.toml:2:18: unsupported format_version 3 (this engine supports format version 1)",
        "rules/a.toml:1:3: unknown section 'widget'",
        "rules/b.txt: unexpected file (only .toml files and README.md are allowed here)",
        "start/: missing required folder",
    ];
    let lines: Vec<&str> = stderr.lines().collect();
    assert_eq!(lines.len(), expected.len() + 1, "{stderr}");
    assert_eq!(&lines[..expected.len()], &expected);
    assert_eq!(
        lines[expected.len()],
        format!("4 error(s) found in {}", path.display())
    );
}

#[test]
fn two_arguments_print_usage_and_exit_2() {
    let output = run(&[Path::new("a"), Path::new("b")]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.trim(), "usage: dev-ui [SETTING_FOLDER]");
}
