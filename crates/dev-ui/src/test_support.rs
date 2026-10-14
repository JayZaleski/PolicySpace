//! Helpers shared by the dev UI's unit tests.

use std::path::{Path, PathBuf};

use policyspace_engine::{World, load_setting};

/// A path in the repository (relative to the repository root).
pub fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// A world at turn 0 for the setting at `relative` (from the repository root). Panics with
/// the load errors if the setting doesn't load.
pub fn world_at(relative: &str) -> World {
    let setting = load_setting(repo_path(relative)).unwrap_or_else(|e| panic!("should load:\n{e}"));
    World::new(setting).0
}
