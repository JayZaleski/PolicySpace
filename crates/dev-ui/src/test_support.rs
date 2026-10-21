//! Helpers shared by the dev UI's unit tests.

use std::path::{Path, PathBuf};

use policyspace_engine::decider::python;
use policyspace_engine::{DeciderHost, World, load_setting};

/// A path in the repository (relative to the repository root).
pub fn repo_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

/// A world at turn 0 for the setting at `relative` (from the repository root), built with
/// `host` answering for the businesses. Panics with the load errors if the setting doesn't
/// load, and with the error if the host doesn't hold a name the setting uses.
pub fn world_with(relative: &str, host: &mut DeciderHost) -> World {
    let setting = load_setting(repo_path(relative)).unwrap_or_else(|e| panic!("should load:\n{e}"));
    World::new(setting, host)
        .unwrap_or_else(|e| panic!("the host should hold every decider name: {e}"))
        .0
}

/// A world at turn 0 for the setting at `relative` (from the repository root), under the
/// idle host. Panics with the load errors if the setting doesn't load.
pub fn world_at(relative: &str) -> World {
    world_with(relative, &mut DeciderHost::idle())
}

/// A world at turn 0 for the setting at `relative`, under the setting's own Python deciders
/// (`python::host_for`), with the host so a test can step the world. Panics with the load
/// errors or the host's error. Tests using it need the project's virtual environment, like
/// `session.rs`'s `the_sample_loads_and_steps_under_its_python_deciders`.
pub fn python_world(relative: &str) -> (World, DeciderHost) {
    let setting = load_setting(repo_path(relative)).unwrap_or_else(|e| panic!("should load:\n{e}"));
    let mut host =
        python::host_for(&setting).unwrap_or_else(|e| panic!("the host should start: {e}"));
    let world = World::new(setting, &mut host)
        .unwrap_or_else(|e| panic!("the host should hold every decider name: {e}"))
        .0;
    (world, host)
}
