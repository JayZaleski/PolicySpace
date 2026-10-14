//! What the window has loaded: the current setting folder and what loading it gave. The
//! session also steps the world, keeps the turn log, and remembers the selected hex.
//!
//! This is separate from drawing so it can be tested without a window.

use std::path::{Path, PathBuf};

use policyspace_engine::{HexCoord, HexId, World, load_setting};

use crate::check::{error_lines, turn_line};

/// How many steps Step ×10 takes.
pub const STEP_MANY: u32 = 10;

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
    /// Every turn line since the last load, oldest first (`check::turn_line`). Empty
    /// unless a world is loaded.
    log: Vec<String>,
    /// The error from the last refused step, if any, since the last load.
    step_error: Option<String>,
    /// The selected hex's coordinates. Kept across a failed load (see `reload`).
    selected: Option<HexCoord>,
}

impl Session {
    /// No folder chosen.
    pub fn new() -> Session {
        Session::default()
    }

    /// Makes `folder` the current folder and loads it (replacing whatever was loaded).
    /// Clears the selection.
    pub fn open(&mut self, folder: PathBuf) {
        self.load(&folder);
        self.folder = Some(folder);
        self.selected = None;
    }

    /// Loads the current folder again, from disk. Does nothing if no folder was chosen.
    /// Keeps the selection unless the load succeeded and the new map has no hex at the
    /// selected coordinates; a failed load doesn't clear it.
    pub fn reload(&mut self) {
        let Some(folder) = self.folder.clone() else {
            return;
        };
        self.load(&folder);
        if let (Some(coord), Some(Loaded::World(world))) = (self.selected, &self.loaded)
            && world.map().id(coord).is_none()
        {
            self.selected = None;
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

    /// Steps the world up to `times` times, adding each report's line to the log. Stops at
    /// the first refused step, storing its error (`StepError`'s `Display`). Does nothing
    /// without a world. Returns the number of steps taken.
    pub fn step(&mut self, times: u32) -> u32 {
        let Some(Loaded::World(world)) = &mut self.loaded else {
            return 0;
        };
        let mut taken = 0;
        for _ in 0..times {
            match world.step() {
                Ok(report) => {
                    self.log.push(turn_line(world, &report));
                    taken += 1;
                }
                Err(error) => {
                    self.step_error = Some(error.to_string());
                    break;
                }
            }
        }
        taken
    }

    /// The turn log, oldest first.
    pub fn log(&self) -> &[String] {
        &self.log
    }

    /// The last refused step's error, if any, since the last load.
    pub fn step_error(&self) -> Option<&str> {
        self.step_error.as_deref()
    }

    /// Selects `hex` (a hex of the current world's map), or clears the selection with None.
    /// Without a world, `Some` is ignored and `None` clears.
    pub fn select(&mut self, hex: Option<HexId>) {
        match (hex, &self.loaded) {
            (Some(hex), Some(Loaded::World(world))) => {
                self.selected = Some(world.map().coord(hex));
            }
            (Some(_), _) => {}
            (None, _) => self.selected = None,
        }
    }

    /// The selected hex in the current world's map; None when nothing is selected, no
    /// world is loaded, or the selected coordinates aren't on the map.
    pub fn selected(&self) -> Option<HexId> {
        match (self.selected, &self.loaded) {
            (Some(coord), Some(Loaded::World(world))) => world.map().id(coord),
            _ => None,
        }
    }

    /// Loads the setting in `folder`: a world at turn 0 with a log of turn 0's line, or the
    /// lines describing its errors with an empty log. Clears the step error either way.
    fn load(&mut self, folder: &Path) {
        self.step_error = None;
        match load_setting(folder) {
            Ok(setting) => {
                let (world, report) = World::new(setting);
                self.log = vec![turn_line(&world, &report)];
                self.loaded = Some(Loaded::World(world));
            }
            Err(errors) => {
                self.log.clear();
                self.loaded = Some(Loaded::Errors(error_lines(&errors, folder)));
            }
        }
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

    /// A world for the setting at `relative`, built and stepped `steps` times on its own.
    fn separate_world(relative: &str, steps: u32) -> (World, Vec<String>) {
        let setting = load_setting(repo_path(relative)).unwrap();
        let (mut world, report) = World::new(setting);
        let mut lines = vec![turn_line(&world, &report)];
        for _ in 0..steps {
            let report = world.step().unwrap();
            lines.push(turn_line(&world, &report));
        }
        (world, lines)
    }

    /// The hex at (q, r) in the session's world.
    fn hex_at(session: &Session, q: i32, r: i32) -> HexId {
        world(session)
            .map()
            .id(HexCoord::new(q, r))
            .expect("hex exists")
    }

    /// A setting folder with a manifest and hexes (0, 0) and (1, 0).
    fn two_hex_setting(root: &Path) {
        fs::create_dir_all(root.join("rules")).unwrap();
        fs::create_dir_all(root.join("start")).unwrap();
        fs::write(
            root.join("rules/README.md"),
            "rules
",
        )
        .unwrap();
        fs::write(
            root.join("setting.toml"),
            "name = \"Edited\"
format_version = 1
",
        )
        .unwrap();
        fs::write(
            root.join("start/map.toml"),
            "[[hex]]
q = 0
r = 0

[[hex]]
q = 1
r = 0
",
        )
        .unwrap();
    }

    #[test]
    fn open_gives_a_log_of_turn_0_and_nothing_selected() {
        let mut session = Session::new();
        session.open(repo_path("settings/sample"));
        assert_eq!(
            session.log(),
            ["Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain"]
        );
        assert_eq!(session.step_error(), None);
        assert_eq!(session.selected(), None);
    }

    #[test]
    fn step_adds_one_log_line_per_turn() {
        let mut session = Session::new();
        session.open(repo_path("settings/sample"));
        assert_eq!(session.step(1), 1);
        assert_eq!(world(&session).turn().get(), 1);
        assert_eq!(session.log().len(), 2);
        assert_eq!(
            session.log().last().map(String::as_str),
            Some(
                "Turn 1: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain"
            )
        );
        assert_eq!(session.step(STEP_MANY), 10);
        assert_eq!(world(&session).turn().get(), 11);
        assert_eq!(session.log().len(), 12);
        assert!(session.log().last().unwrap().starts_with("Turn 11: "));
        assert_eq!(session.step_error(), None);
    }

    #[test]
    fn the_log_matches_the_checkers_turn_lines() {
        let mut session = Session::new();
        session.open(repo_path("crates/engine/tests/settings/turns_valid"));
        assert_eq!(session.step(4), 4);
        let (_, expected) = separate_world("crates/engine/tests/settings/turns_valid", 4);
        assert_eq!(session.log(), expected.as_slice());
    }

    #[test]
    fn stepping_without_a_world_does_nothing() {
        let mut session = Session::new();
        assert_eq!(session.step(1), 0);
        assert!(session.log().is_empty());

        session.open(repo_path("crates/engine/tests/settings/many_errors"));
        assert_eq!(session.step(STEP_MANY), 0);
        assert!(session.log().is_empty());
        assert_eq!(session.step_error(), None);
    }

    #[test]
    fn reload_after_stepping_starts_a_new_log_at_turn_0() {
        let mut session = Session::new();
        session.open(repo_path("settings/sample"));
        session.step(STEP_MANY);
        session.reload();
        assert_eq!(world(&session).turn().get(), 0);
        assert_eq!(session.log().len(), 1);
        assert_eq!(session.step_error(), None);
    }

    #[test]
    fn reload_keeps_the_selection_and_select_none_clears_it() {
        let mut session = Session::new();
        session.open(repo_path("settings/sample"));
        let hex = hex_at(&session, 0, 0);
        session.select(Some(hex));
        assert_eq!(session.selected(), Some(hex));
        session.reload();
        assert_eq!(session.selected(), Some(hex_at(&session, 0, 0)));
        session.select(None);
        assert_eq!(session.selected(), None);
    }

    #[test]
    fn open_clears_the_selection() {
        let mut session = Session::new();
        session.open(repo_path("settings/sample"));
        session.select(Some(hex_at(&session, 0, 0)));
        session.open(repo_path("settings/sample"));
        assert_eq!(session.selected(), None);

        session.select(Some(hex_at(&session, 0, 0)));
        session.open(repo_path("crates/engine/tests/settings/turns_valid"));
        assert_eq!(session.selected(), None);
        assert_eq!(session.log().len(), 1);
    }

    #[test]
    fn reload_clears_the_selection_when_the_hex_is_gone_for_good() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        two_hex_setting(root);
        let mut session = Session::new();
        session.open(root.to_path_buf());
        session.select(Some(hex_at(&session, 1, 0)));
        assert!(session.selected().is_some());

        fs::write(
            root.join("start/map.toml"),
            "[[hex]]
q = 0
r = 0
",
        )
        .unwrap();
        session.reload();
        assert_eq!(session.selected(), None);

        fs::write(
            root.join("start/map.toml"),
            "[[hex]]
q = 0
r = 0

[[hex]]
q = 1
r = 0
",
        )
        .unwrap();
        session.reload();
        assert_eq!(session.selected(), None);
    }

    #[test]
    fn a_failed_reload_keeps_the_selection_for_the_next_good_one() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        two_hex_setting(root);
        let mut session = Session::new();
        session.open(root.to_path_buf());
        session.select(Some(hex_at(&session, 0, 0)));

        fs::write(
            root.join("setting.toml"),
            "name = \"Edited\"
format_version = 99
",
        )
        .unwrap();
        session.reload();
        assert!(matches!(session.loaded(), Some(Loaded::Errors(_))));
        assert_eq!(session.selected(), None);
        assert!(session.log().is_empty());

        fs::write(
            root.join("setting.toml"),
            "name = \"Edited\"
format_version = 1
",
        )
        .unwrap();
        session.reload();
        assert_eq!(session.selected(), Some(hex_at(&session, 0, 0)));
    }

    #[test]
    fn select_while_holding_errors_selects_nothing() {
        let mut session = Session::new();
        let hex = {
            let mut other = Session::new();
            other.open(repo_path("settings/sample"));
            hex_at(&other, 0, 0)
        };
        session.select(Some(hex));
        assert_eq!(session.selected(), None);
        session.open(repo_path("crates/engine/tests/settings/many_errors"));
        session.select(Some(hex));
        assert_eq!(session.selected(), None);
    }
}
