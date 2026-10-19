//! What the window has loaded: the current setting folder and what loading it gave. The
//! session also steps the world, keeps the turn log, and remembers the selected hex.
//!
//! This is separate from drawing so it can be tested without a window.

use std::path::{Path, PathBuf};

use policyspace_engine::decider::python::{self, HostError};
use policyspace_engine::{DeciderHost, HexCoord, HexId, LoadedSetting, World, load_setting};

use crate::text::{error_lines, failure_lines, load_failure_lines, turn_line};

/// How many steps Step ×10 takes.
pub const STEP_MANY: u32 = 10;

/// What the last load of the current folder gave.
// The world is big, but there is only ever one `Loaded` value at a time, so boxing it would
// only add indirection.
#[allow(clippy::large_enum_variant)]
pub enum Loaded {
    /// The setting loaded; this is its world (at turn 0 after a load, later after steps).
    World(World),
    /// The setting has errors: the lines from `text::error_lines`.
    Errors(Vec<String>),
}

/// A function that builds the decider host for a setting.
#[cfg(test)]
pub type HostBuilder = Box<dyn Fn(&LoadedSetting) -> Result<DeciderHost, HostError>>;

/// How a session builds the decider host when it loads a setting.
pub enum Hosting {
    /// The idle host: every business does nothing. For tests and tools that don't care.
    /// Only the tests build it, so the window's build leaves it out.
    #[cfg(test)]
    Idle,
    /// One Python process per decider name the setting's businesses carry
    /// (`python::host_for`).
    FromSetting,
    /// A caller-supplied builder (tests; Stage 05's UI decider). Only the tests build it, so
    /// the window's build leaves it out.
    #[cfg(test)]
    Custom(HostBuilder),
}

impl Hosting {
    /// Builds the host for `setting`.
    fn build(&self, setting: &LoadedSetting) -> Result<DeciderHost, HostError> {
        match self {
            #[cfg(test)]
            Hosting::Idle => Ok(DeciderHost::idle()),
            Hosting::FromSetting => python::host_for(setting),
            #[cfg(test)]
            Hosting::Custom(build) => build(setting),
        }
    }
}

/// The current folder (if any) and what loading it gave.
pub struct Session {
    /// How later loads build their decider host.
    hosting: Hosting,
    /// The current folder, once one was opened.
    folder: Option<PathBuf>,
    /// What loading the current folder gave.
    loaded: Option<Loaded>,
    /// The host for the loaded world; None whenever `loaded` isn't a world. Dropping it
    /// (on reload, open or a failed load) ends its processes.
    host: Option<DeciderHost>,
    /// Every turn line since the last load, oldest first (`text::turn_line`), each followed
    /// by its decider failure lines (`text::failure_lines`). Empty unless a world is loaded.
    log: Vec<String>,
    /// The error from the last refused step, if any, since the last load.
    step_error: Option<String>,
    /// The selected hex's coordinates. Kept across a failed load (see `reload`).
    selected: Option<HexCoord>,
}

impl Session {
    /// No folder chosen; `hosting` says how later loads build their host.
    pub fn new(hosting: Hosting) -> Session {
        Session {
            hosting,
            folder: None,
            loaded: None,
            host: None,
            log: Vec::new(),
            step_error: None,
            selected: None,
        }
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
        if let (Some(coord), Some(world)) = (self.selected, self.world())
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

    /// The loaded world; None when nothing is loaded or the setting has errors.
    pub fn world(&self) -> Option<&World> {
        match &self.loaded {
            Some(Loaded::World(world)) => Some(world),
            _ => None,
        }
    }

    /// Steps the world up to `times` times, adding each report's line to the log, followed
    /// by the lines of each decider failure. Stops at the first refused step, storing its
    /// error (`StepError`'s `Display`). Does nothing without a world. Returns the number of
    /// steps taken.
    pub fn step(&mut self, times: u32) -> u32 {
        let (Some(Loaded::World(world)), Some(host)) = (&mut self.loaded, &mut self.host) else {
            return 0;
        };
        let mut taken = 0;
        for _ in 0..times {
            match world.step(host) {
                Ok(report) => {
                    self.log.push(turn_line(world, &report));
                    for failure in &report.decider_failures {
                        self.log.extend(failure_lines(report.turn, failure));
                    }
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
        match (hex, self.world()) {
            (Some(hex), Some(world)) => {
                self.selected = Some(world.map().coord(hex));
            }
            (Some(_), _) => {}
            (None, _) => self.selected = None,
        }
    }

    /// The selected hex in the current world's map; None when nothing is selected, no
    /// world is loaded, or the selected coordinates aren't on the map.
    pub fn selected(&self) -> Option<HexId> {
        let coord = self.selected?;
        self.world()?.map().id(coord)
    }

    /// Loads the setting in `folder`: a world at turn 0 with a log of turn 0's line (and its
    /// decider failures), or the lines describing its errors with an empty log. A setting
    /// whose deciders can't start, or whose businesses name a decider the host doesn't
    /// hold, gives errors too. Ends the old host's processes first, and clears the step
    /// error either way.
    fn load(&mut self, folder: &Path) {
        self.step_error = None;
        self.host = None;
        let setting = match load_setting(folder) {
            Ok(setting) => setting,
            Err(errors) => {
                self.log.clear();
                self.loaded = Some(Loaded::Errors(error_lines(&errors, folder)));
                return;
            }
        };
        let mut host = match self.hosting.build(&setting) {
            Ok(host) => host,
            Err(error) => {
                self.fail(&error.to_string(), folder);
                return;
            }
        };
        match World::new(setting, &mut host) {
            Ok((world, report)) => {
                self.log = vec![turn_line(&world, &report)];
                for failure in &report.decider_failures {
                    self.log.extend(failure_lines(report.turn, failure));
                }
                self.host = Some(host);
                self.loaded = Some(Loaded::World(world));
            }
            Err(error) => self.fail(&error.to_string(), folder),
        }
    }

    /// Records a load that failed after the setting itself loaded fine.
    fn fail(&mut self, message: &str, folder: &Path) {
        self.log.clear();
        self.loaded = Some(Loaded::Errors(load_failure_lines(message, folder)));
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use policyspace_engine::Reply;
    use policyspace_engine::decider::ClosureDecider;
    use policyspace_engine::decider::python::PythonError;

    use super::*;
    use crate::test_support::repo_path;

    /// A valid manifest.
    const MANIFEST: &str = "name = \"Edited\"\nformat_version = 1\n";
    /// A manifest with an unsupported format version.
    const BAD_MANIFEST: &str = "name = \"Edited\"\nformat_version = 99\n";
    /// One good, the currency of the test maps' hexes.
    const COIN_GOOD: &str = "[[good]]\nname = \"coin\"\n";
    /// A map with the hex (0, 0).
    const ONE_HEX_MAP: &str = "[[hex]]\nq = 0\nr = 0\ncurrency = \"coin\"\n";
    /// A map with the hexes (0, 0) and (1, 0).
    const TWO_HEX_MAP: &str = "[[hex]]\nq = 0\nr = 0\ncurrency = \"coin\"\n\n[[hex]]\nq = 1\nr = 0\ncurrency = \"coin\"\n";

    /// The world a session holds, or a failure if it holds errors or nothing.
    fn world(session: &Session) -> &World {
        if let Some(world) = session.world() {
            return world;
        }
        match session.loaded() {
            Some(Loaded::Errors(lines)) => panic!("expected a world, got errors: {lines:?}"),
            _ => panic!("expected a world, got nothing"),
        }
    }

    #[test]
    fn world_is_none_without_a_world_and_the_world_otherwise() {
        let mut session = Session::new(Hosting::Idle);
        assert!(session.world().is_none());
        session.open(repo_path("crates/engine/tests/settings/many_errors"));
        assert!(session.world().is_none());
        session.open(repo_path("settings/sample"));
        let world = session.world().expect("the sample loads");
        assert_eq!(world.map().len(), 19);
        assert_eq!(world.turn().get(), 0);
    }

    #[test]
    fn a_new_session_has_nothing_and_reload_does_nothing() {
        let mut session = Session::new(Hosting::Idle);
        assert!(session.folder().is_none());
        assert!(session.loaded().is_none());
        session.reload();
        assert!(session.folder().is_none());
        assert!(session.loaded().is_none());
    }

    #[test]
    fn open_loads_a_world_at_turn_0() {
        let mut session = Session::new(Hosting::Idle);
        let folder = repo_path("settings/sample");
        session.open(folder.clone());
        assert_eq!(session.folder(), Some(folder.as_path()));
        let world = world(&session);
        assert_eq!(world.map().len(), 19);
        assert_eq!(world.turn().get(), 0);
    }

    #[test]
    fn open_on_a_broken_setting_gives_the_checkers_lines_and_keeps_the_folder() {
        let mut session = Session::new(Hosting::Idle);
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
        fs::write(root.join("rules/goods.toml"), COIN_GOOD).unwrap();
        fs::write(root.join("setting.toml"), MANIFEST).unwrap();
        fs::write(root.join("start/map.toml"), ONE_HEX_MAP).unwrap();

        let mut session = Session::new(Hosting::Idle);
        session.open(root.to_path_buf());
        assert_eq!(world(&session).map().len(), 1);

        fs::write(root.join("start/map.toml"), TWO_HEX_MAP).unwrap();
        session.reload();
        assert_eq!(world(&session).map().len(), 2);

        fs::write(root.join("setting.toml"), BAD_MANIFEST).unwrap();
        session.reload();
        assert!(matches!(session.loaded(), Some(Loaded::Errors(_))));
        assert_eq!(session.folder(), Some(root));

        fs::write(root.join("setting.toml"), MANIFEST).unwrap();
        session.reload();
        assert_eq!(world(&session).map().len(), 2);
    }

    #[test]
    fn open_on_a_different_folder_replaces_what_was_loaded() {
        let mut session = Session::new(Hosting::Idle);
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
        let mut host = DeciderHost::idle();
        let (mut world, report) = World::new(setting, &mut host).unwrap();
        let mut lines = vec![turn_line(&world, &report)];
        for _ in 0..steps {
            let report = world.step(&mut host).unwrap();
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
        fs::write(root.join("rules/README.md"), "rules\n").unwrap();
        fs::write(root.join("rules/goods.toml"), COIN_GOOD).unwrap();
        fs::write(root.join("setting.toml"), MANIFEST).unwrap();
        fs::write(root.join("start/map.toml"), TWO_HEX_MAP).unwrap();
    }

    #[test]
    fn open_gives_a_log_of_turn_0_and_nothing_selected() {
        let mut session = Session::new(Hosting::Idle);
        session.open(repo_path("settings/sample"));
        assert_eq!(
            session.log(),
            [
                "Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain; population 419.4466666666666"
            ]
        );
        assert_eq!(session.step_error(), None);
        assert_eq!(session.selected(), None);
    }

    #[test]
    fn step_adds_one_log_line_per_turn() {
        let mut session = Session::new(Hosting::Idle);
        session.open(repo_path("settings/sample"));
        assert_eq!(session.step(1), 1);
        assert_eq!(world(&session).turn().get(), 1);
        assert_eq!(session.log().len(), 2);
        assert_eq!(
            session.log().last().map(String::as_str),
            Some(
                "Turn 1: perished 10 unimproved_copper, 24 unimproved_grain; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4194.466666666667 unskilled_labor; sold 2955.6459999999997 unskilled_labor; bought 5.15 copper, 1464.9479999999999 grain; consumed 1238.8206666666667 unskilled_labor, 5.15 copper, 1464.9479999999999 grain; population 429.2982444444444"
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
        let mut session = Session::new(Hosting::Idle);
        session.open(repo_path("crates/engine/tests/settings/turns_valid"));
        assert_eq!(session.step(4), 4);
        let (_, expected) = separate_world("crates/engine/tests/settings/turns_valid", 4);
        assert_eq!(session.log(), expected.as_slice());
    }

    #[test]
    fn stepping_without_a_world_does_nothing() {
        let mut session = Session::new(Hosting::Idle);
        assert_eq!(session.step(1), 0);
        assert!(session.log().is_empty());

        session.open(repo_path("crates/engine/tests/settings/many_errors"));
        assert_eq!(session.step(STEP_MANY), 0);
        assert!(session.log().is_empty());
        assert_eq!(session.step_error(), None);
    }

    #[test]
    fn reload_after_stepping_starts_a_new_log_at_turn_0() {
        let mut session = Session::new(Hosting::Idle);
        session.open(repo_path("settings/sample"));
        session.step(STEP_MANY);
        session.reload();
        assert_eq!(world(&session).turn().get(), 0);
        assert_eq!(session.log().len(), 1);
        assert_eq!(session.step_error(), None);
    }

    #[test]
    fn reload_keeps_the_selection_and_select_none_clears_it() {
        let mut session = Session::new(Hosting::Idle);
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
        let mut session = Session::new(Hosting::Idle);
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
        let mut session = Session::new(Hosting::Idle);
        session.open(root.to_path_buf());
        session.select(Some(hex_at(&session, 1, 0)));
        assert!(session.selected().is_some());

        fs::write(root.join("start/map.toml"), ONE_HEX_MAP).unwrap();
        session.reload();
        assert_eq!(session.selected(), None);

        fs::write(root.join("start/map.toml"), TWO_HEX_MAP).unwrap();
        session.reload();
        assert_eq!(session.selected(), None);
    }

    #[test]
    fn a_failed_reload_keeps_the_selection_for_the_next_good_one() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        two_hex_setting(root);
        let mut session = Session::new(Hosting::Idle);
        session.open(root.to_path_buf());
        session.select(Some(hex_at(&session, 0, 0)));

        fs::write(root.join("setting.toml"), BAD_MANIFEST).unwrap();
        session.reload();
        assert!(matches!(session.loaded(), Some(Loaded::Errors(_))));
        assert_eq!(session.selected(), None);
        assert!(session.log().is_empty());

        fs::write(root.join("setting.toml"), MANIFEST).unwrap();
        session.reload();
        assert_eq!(session.selected(), Some(hex_at(&session, 0, 0)));
    }

    #[test]
    fn select_while_holding_errors_selects_nothing() {
        let mut session = Session::new(Hosting::Idle);
        let hex = {
            let mut other = Session::new(Hosting::Idle);
            other.open(repo_path("settings/sample"));
            hex_at(&other, 0, 0)
        };
        session.select(Some(hex));
        assert_eq!(session.selected(), None);
        session.open(repo_path("crates/engine/tests/settings/many_errors"));
        session.select(Some(hex));
        assert_eq!(session.selected(), None);
    }

    /// The sample's turn-0 line, as `open_gives_a_log_of_turn_0_and_nothing_selected` has it.
    const SAMPLE_TURN_0: &str = "Turn 0: perished nothing; emitted 10 unimproved_copper, 24 unimproved_grain; endowed 4100 unskilled_labor; sold 2878.6 unskilled_labor; bought 5 copper, 1426.8 grain; consumed 1221.4 unskilled_labor, 5 copper, 1426.8 grain; population 419.4466666666666";

    #[test]
    fn a_closure_decider_failure_is_logged_each_turn() {
        let hosting = Hosting::Custom(Box::new(|_| {
            Ok(DeciderHost::new()
                .with(
                    "rule",
                    ClosureDecider::new(|r| Reply {
                        turn: r.turn.next().unwrap(),
                        decisions: vec![],
                    }),
                )
                .with(
                    "other_rule",
                    ClosureDecider::new(|r| Reply {
                        turn: r.turn,
                        decisions: vec![],
                    }),
                ))
        }));
        let mut session = Session::new(hosting);
        session.open(repo_path("crates/engine/tests/settings/businesses_valid"));
        assert_eq!(session.log().len(), 2);
        assert!(session.log()[0].starts_with("Turn 0: "));
        assert_eq!(
            session.log()[1],
            "Turn 0: decider 'rule' failed: the reply was refused: the reply answers turn 1, but the request is for turn 0"
        );
        assert_eq!(session.step(1), 1);
        assert_eq!(session.log().len(), 4);
        assert!(session.log()[2].starts_with("Turn 1: "));
        assert_eq!(
            session.log()[3],
            "Turn 1: decider 'rule' failed: the reply was refused: the reply answers turn 2, but the request is for turn 1"
        );
    }

    #[test]
    fn a_custom_host_error_is_a_load_error() {
        let hosting = Hosting::Custom(Box::new(|_| {
            Err(HostError::Python(PythonError::BadTimeLimit {
                value: "x".to_string(),
            }))
        }));
        let mut session = Session::new(hosting);
        let folder = repo_path("settings/sample");
        session.open(folder.clone());
        match session.loaded() {
            Some(Loaded::Errors(lines)) => {
                assert_eq!(lines.len(), 2);
                assert_eq!(
                    lines[0],
                    PythonError::BadTimeLimit {
                        value: "x".to_string()
                    }
                    .to_string()
                );
                assert_eq!(
                    lines[1],
                    format!("1 error(s) found in {}", folder.display())
                );
            }
            _ => panic!("expected errors"),
        }
        assert!(session.world().is_none());
        assert!(session.log().is_empty());
        assert_eq!(session.step(1), 0);
    }

    #[test]
    fn the_sample_loads_and_steps_under_its_python_deciders() {
        let mut session = Session::new(Hosting::FromSetting);
        session.open(repo_path("settings/sample"));
        if let Some(Loaded::Errors(lines)) = session.loaded() {
            panic!("the sample didn't load:\n{}", lines.join("\n"));
        }
        assert_eq!(session.log()[0], SAMPLE_TURN_0);
        assert_eq!(session.step(2), 2);
        assert_eq!(session.log().len(), 3);
        for line in session.log() {
            assert!(line.starts_with("Turn "), "{line}");
            assert!(!line.contains("failed"), "{line}");
        }
    }

    #[test]
    fn a_decider_that_wont_start_is_a_load_error() {
        let mut session = Session::new(Hosting::FromSetting);
        session.open(repo_path("crates/engine/tests/settings/production_valid"));
        match session.loaded() {
            Some(Loaded::Errors(lines)) => {
                assert!(
                    lines.iter().any(|l| l.contains("decider 'rule'")),
                    "{lines:?}"
                );
                assert!(
                    lines.iter().any(|l| l.contains("unknown decider 'rule'")),
                    "{lines:?}"
                );
            }
            _ => panic!("expected errors"),
        }
        assert_eq!(session.step(1), 0);
    }
}
