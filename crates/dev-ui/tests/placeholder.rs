//! Runs the placeholder dev UI binary and checks its output.

use std::process::Command;

use policyspace_engine::{ENGINE_NAME, ENGINE_VERSION};

#[test]
fn prints_engine_name_and_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_dev-ui"))
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
