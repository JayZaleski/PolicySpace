//! Helpers shared by the engine's integration tests.

// Each test file uses only some of these helpers.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use policyspace_engine::{LoadErrors, LoadedSetting, load_setting};

/// The folder of the test setting `name` (`tests/settings/<name>`).
pub fn test_setting(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/settings")
        .join(name)
}

/// The folder of the sample setting.
pub fn sample_setting() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../settings/sample")
}

/// Loads the setting at `path`, panicking with its errors if it doesn't load.
pub fn load(path: &Path) -> LoadedSetting {
    load_setting(path).unwrap_or_else(|errors| panic!("{} should load:\n{errors}", path.display()))
}

/// Each error as one line of text, in the loader's order.
pub fn lines(errors: &LoadErrors) -> Vec<String> {
    errors.iter().map(|e| e.to_string()).collect()
}

/// Loads the test setting `name`, which must fail, and returns its error lines.
pub fn failures_of(name: &str) -> Vec<String> {
    match load_setting(test_setting(name)) {
        Ok(_) => panic!("{name} must not load"),
        Err(errors) => lines(&errors),
    }
}
