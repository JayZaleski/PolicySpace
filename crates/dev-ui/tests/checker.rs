//! Runs the dev UI binary's `check` form (the setting checker) and checks its output and
//! exit codes. (`cli.rs` covers `--version` and the usage errors.)

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Runs `dev-ui` with the arguments exactly as given.
fn run_raw(args: &[&Path]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_dev-ui"))
        .args(args)
        .output()
        .expect("failed to run the dev-ui binary")
}

/// Runs `dev-ui check` with the arguments.
fn run(args: &[&Path]) -> Output {
    let mut all = vec![Path::new("check")];
    all.extend_from_slice(args);
    run_raw(&all)
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
        "Loaded setting 'Sample Setting' (format version 1): 6 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops"
    );
}

#[test]
fn says_1_good_for_one_good_and_0_goods_for_none() {
    let one = run(&[&repo_path("crates/engine/tests/settings/goods_one")]);
    assert_eq!(one.status.code(), Some(0));
    let stdout = String::from_utf8(one.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        "Loaded setting 'Goods Test' (format version 1): 1 good, 0 hexes, 0 resources, 0 deposits, 0 markets, 0 pop types, 0 pops"
    );

    let none = run(&[&repo_path("crates/engine/tests/settings/valid_minimal")]);
    assert_eq!(none.status.code(), Some(0));
    let stdout = String::from_utf8(none.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        "Loaded setting 'Minimal Setting' (format version 1): 0 goods, 0 hexes, 0 resources, 0 deposits, 0 markets, 0 pop types, 0 pops"
    );
}

#[test]
fn says_1_hex_for_one_hex() {
    let output = run(&[&repo_path("crates/engine/tests/settings/hexes_one")]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        "Loaded setting 'Hexes Test' (format version 1): 1 good, 1 hex, 0 resources, 0 deposits, 0 markets, 0 pop types, 0 pops"
    );
}

#[test]
fn says_1_resource_and_1_deposit_for_one_of_each() {
    let output = run(&[&repo_path("crates/engine/tests/settings/resources_one")]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        "Loaded setting 'Resources Test' (format version 1): 1 good, 1 hex, 1 resource, 1 deposit, 1 market, 0 pop types, 0 pops"
    );
}

#[test]
fn says_1_pop_type_for_one_pop_type() {
    let output = run(&[&repo_path("crates/engine/tests/settings/pops_one")]);
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.trim(),
        "Loaded setting 'Pops One' (format version 1): 1 good, 1 hex, 0 resources, 0 deposits, 0 markets, 1 pop type, 1 pop"
    );
}

#[test]
fn counts_pops_and_prints_the_endowment_for_a_setting_with_pops() {
    let output = run_with(
        "crates/engine/tests/settings/pops_placed",
        &["--turns", "1"],
    );
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    assert_eq!(
        stdout_lines(&output),
        vec![
            "Loaded setting 'Pops Placed' (format version 1): 3 goods, 3 hexes, 1 resource, 1 deposit, 0 markets, 3 pop types, 4 pops",
            "Turn 0: perished nothing; emitted 2 grain; endowed 43 labor, 3 gold; sold nothing; bought nothing; consumed 35 labor",
            "Turn 1: perished 8 labor, 2 grain; emitted 2 grain; endowed 43 labor, 3 gold; sold nothing; bought nothing; consumed 35 labor",
        ]
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
    let output = run_raw(&[Path::new("a"), Path::new("b")]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.trim(), USAGE);
}

const USAGE: &str = "usage: dev-ui [SETTING_FOLDER]
       dev-ui check SETTING_FOLDER [--turns N]
       dev-ui --version";

const SAMPLE_LINE: &str = "Loaded setting 'Sample Setting' (format version 1): 6 goods, 19 hexes, 2 resources, 10 deposits, 18 markets, 2 pop types, 16 pops";

/// Runs with the setting folder (relative to the repository) and then extra arguments.
fn run_with(folder: &str, extra: &[&str]) -> Output {
    let mut args = vec![repo_path(folder)];
    args.extend(extra.iter().map(PathBuf::from));
    let args: Vec<&Path> = args.iter().map(PathBuf::as_path).collect();
    run(&args)
}

fn stdout_lines(output: &Output) -> Vec<String> {
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect()
}

#[test]
fn turns_prints_a_line_for_each_turn() {
    let output = run_with("settings/sample", &["--turns", "2"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty());
    let steady = "perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain";
    assert_eq!(
        stdout_lines(&output),
        vec![
            SAMPLE_LINE.to_string(),
            "Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain"
                .to_string(),
            format!("Turn 1: {steady}"),
            format!("Turn 2: {steady}"),
        ]
    );
}

#[test]
fn turns_zero_only_runs_turn_0() {
    let output = run_with("settings/sample", &["--turns", "0"]);
    assert_eq!(output.status.code(), Some(0));
    let lines = stdout_lines(&output);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], SAMPLE_LINE);
    assert!(lines[1].starts_with("Turn 0: "));
}

#[test]
fn empty_lists_print_nothing_and_decimals_print_as_is() {
    let output = run_with(
        "crates/engine/tests/settings/valid_minimal",
        &["--turns", "1"],
    );
    assert_eq!(output.status.code(), Some(0));
    let lines = stdout_lines(&output);
    assert_eq!(
        &lines[1..],
        [
            "Turn 0: perished nothing; emitted nothing; endowed nothing; sold nothing; bought nothing; consumed nothing",
            "Turn 1: perished nothing; emitted nothing; endowed nothing; sold nothing; bought nothing; consumed nothing"
        ]
    );

    let output = run_with(
        "crates/engine/tests/settings/turns_valid",
        &["--turns", "3"],
    );
    assert_eq!(output.status.code(), Some(0));
    let lines = stdout_lines(&output);
    assert_eq!(
        lines.last().unwrap(),
        "Turn 3: perished 6.5 daily, 5 stew; emitted 6.5 daily, 5 stew, 1.5 gold; endowed nothing; sold nothing; bought nothing; consumed nothing"
    );
}

#[test]
fn bad_turn_arguments_print_usage_and_exit_2() {
    let cases: [&[&str]; 9] = [
        &["--turns"],
        &["--turns", "10001"],
        &["--turns", "-1"],
        &["--turns", "+5"],
        &["--turns", "abc"],
        &["--turns", "99999999999"],
        &["--turns", "1", "extra"],
        &["--steps", "1"],
        &["--turns", ""],
    ];
    for extra in cases {
        let output = run_with("settings/sample", extra);
        assert_eq!(output.status.code(), Some(2), "{extra:?}");
        assert!(output.stdout.is_empty(), "{extra:?}");
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(stderr.trim(), USAGE, "{extra:?}");
    }
}

#[test]
fn leading_zeros_are_fine() {
    let output = run_with("settings/sample", &["--turns", "007"]);
    assert_eq!(output.status.code(), Some(0));
    // The success line, then turns 0 to 7.
    assert_eq!(stdout_lines(&output).len(), 9);
}

#[test]
fn a_broken_setting_with_turns_prints_the_same_errors_and_runs_no_turns() {
    let path = repo_path("crates/engine/tests/settings/many_errors");
    let plain = run(&[&path]);
    let with_turns = run_with(
        "crates/engine/tests/settings/many_errors",
        &["--turns", "2"],
    );
    assert_eq!(with_turns.status.code(), Some(1));
    assert!(with_turns.stdout.is_empty());
    assert_eq!(with_turns.stderr, plain.stderr);
}
