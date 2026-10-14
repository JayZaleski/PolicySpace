//! What the window has loaded: the current setting folder and what loading it gave.
//!
//! This is separate from drawing so it can be tested without a window.

use std::path::{Path, PathBuf};

use policyspace_engine::{World, load_setting};

use crate::check::error_lines;

/// What the last load of the current folder gave.
// The world is big, but there is only ever one `Loaded` value at a time, so boxing it would
// only add indirection.
#[allow(clippy::large_enum_variant)]
pub enum Loaded {
    /// The setting loaded; its world is at turn 0.
    World(World),
    /// The setting has errors: the lines from `check::error_lines`.
    Errors(Vec<String>),
}

/// The current folder (if any) and what loading it gave.
#[derive(Default)]
pub struct Session {
    /// The current folder, once one was opened.
    folder: Option<PathBuf>,
    /// What loading the current folder gave.
    loaded: Option<Loaded>,
}

impl Session {
    /// No folder chosen.
    pub fn new() -> Session {
        Session::default()
    }

    /// Makes `folder` the current folder and loads it (replacing whatever was loaded).
    pub fn open(&mut self, folder: PathBuf) {
        self.loaded = Some(load(&folder));
        self.folder = Some(folder);
    }

    /// Loads the current folder again, from disk. Does nothing if no folder was chosen.
    pub fn reload(&mut self) {
        if let Some(folder) = &self.folder {
            self.loaded = Some(load(folder));
        }
    }

    /// The current folder.
    pub fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    /// What the last load gave; None if no folder was chosen.
    pub fn loaded(&self) -> Option<&Loaded> {
        self.loaded.as_ref()
    }
}

/// Loads the setting in `folder`: a world at turn 0, or the lines describing its errors.
fn load(folder: &Path) -> Loaded {
    match load_setting(folder) {
        // Step 08 will show the turn 0 report; for now the map doesn't need it.
        Ok(setting) => Loaded::World(World::new(setting).0),
        Err(errors) => Loaded::Errors(error_lines(&errors, folder)),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A setting folder in the repository (relative to the repository root).
    fn repo_path(relative: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }

    /// The world a session holds, or a failure if it holds errors or nothing.
    fn world(session: &Session) -> &World {
        match session.loaded() {
            Some(Loaded::World(world)) => world,
            Some(Loaded::Errors(lines)) => panic!("expected a world, got errors: {lines:?}"),
            None => panic!("expected a world, got nothing"),
        }
    }

    #[test]
    fn a_new_session_has_nothing_and_reload_does_nothing() {
        let mut session = Session::new();
        assert!(session.folder().is_none());
        assert!(session.loaded().is_none());
        session.reload();
        assert!(session.folder().is_none());
        assert!(session.loaded().is_none());
    }

    #[test]
    fn open_loads_a_world_at_turn_0() {
        let mut session = Session::new();
        let folder = repo_path("settings/sample");
        session.open(folder.clone());
        assert_eq!(session.folder(), Some(folder.as_path()));
        let world = world(&session);
        assert_eq!(world.map().len(), 19);
        assert_eq!(world.turn().get(), 0);
    }

    #[test]
    fn open_on_a_broken_setting_gives_the_checkers_lines_and_keeps_the_folder() {
        let mut session = Session::new();
        let folder = repo_path("crates/engine/tests/settings/many_errors");
        session.open(folder.clone());
        assert_eq!(session.folder(), Some(folder.as_path()));
        let errors = load_setting(&folder).expect_err("many_errors has errors");
        match session.loaded() {
            Some(Loaded::Errors(lines)) => assert_eq!(lines, &error_lines(&errors, &folder)),
            _ => panic!("expected errors"),
        }
    }

    #[test]
    fn reload_sees_edits_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("rules")).unwrap();
        fs::create_dir_all(root.join("start")).unwrap();
        fs::write(root.join("rules/README.md"), "rules\n").unwrap();
        let valid_manifest = "name = \"Edited\"\nformat_version = 1\n";
        fs::write(root.join("setting.toml"), valid_manifest).unwrap();
        fs::write(root.join("start/map.toml"), "[[hex]]\nq = 0\nr = 0\n").unwrap();

        let mut session = Session::new();
        session.open(root.to_path_buf());
        assert_eq!(world(&session).map().len(), 1);

        fs::write(
            root.join("start/map.toml"),
            "[[hex]]\nq = 0\nr = 0\n\n[[hex]]\nq = 1\nr = 0\n",
        )
        .unwrap();
        session.reload();
        assert_eq!(world(&session).map().len(), 2);

        fs::write(
            root.join("setting.toml"),
            "name = \"Edited\"\nformat_version = 99\n",
        )
        .unwrap();
        session.reload();
        assert!(matches!(session.loaded(), Some(Loaded::Errors(_))));
        assert_eq!(session.folder(), Some(root));

        fs::write(root.join("setting.toml"), valid_manifest).unwrap();
        session.reload();
        assert_eq!(world(&session).map().len(), 2);
    }

    #[test]
    fn open_on_a_different_folder_replaces_what_was_loaded() {
        let mut session = Session::new();
        session.open(repo_path("settings/sample"));
        assert_eq!(world(&session).map().len(), 19);
        session.open(repo_path("crates/engine/tests/settings/hexes_one"));
        assert_eq!(world(&session).map().len(), 1);
        session.open(repo_path("crates/engine/tests/settings/many_errors"));
        assert!(matches!(session.loaded(), Some(Loaded::Errors(_))));
        session.open(repo_path("settings/sample"));
        assert_eq!(world(&session).map().len(), 19);
    }
}
