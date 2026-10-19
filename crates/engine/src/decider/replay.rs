//! The replay decider: answers from recordings, one per turn, so engine tests that don't
//! test strategy need no Python. The engine's request must equal the recorded one, which
//! also checks a request read from a file against an engine-built one.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use crate::decider::host::{Decider, DeciderFailure};
use crate::decider::{FormatError, Recording, Reply, Request};
use crate::turn::Turn;

/// Why a recording file couldn't be used.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayError {
    /// The file couldn't be read.
    Read {
        /// The file.
        path: PathBuf,
        /// The operating system's message.
        error: String,
    },
    /// The file isn't a recording.
    Format {
        /// The file.
        path: PathBuf,
        /// Why not.
        error: FormatError,
    },
    /// Two recordings for one turn.
    DuplicateTurn {
        /// The turn recorded twice.
        turn: Turn,
    },
}

impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplayError::Read { path, error } => {
                write!(f, "{}: can't be read: {error}", path.display())
            }
            ReplayError::Format { path, error } => {
                write!(f, "{}: not a recording: {error}", path.display())
            }
            ReplayError::DuplicateTurn { turn } => write!(f, "two recordings for turn {turn}"),
        }
    }
}

impl std::error::Error for ReplayError {}

/// A decider that replays recordings: the reply recorded for the request's turn, provided
/// the request equals the recorded one.
pub struct ReplayDecider {
    recordings: BTreeMap<Turn, Recording>,
    pending: Option<Result<Reply, DeciderFailure>>,
}

impl ReplayDecider {
    /// From recordings in memory; `DuplicateTurn` if two share a request turn.
    pub fn new(
        recordings: impl IntoIterator<Item = Recording>,
    ) -> Result<ReplayDecider, ReplayError> {
        let mut by_turn = BTreeMap::new();
        for recording in recordings {
            let turn = recording.request.turn;
            if by_turn.insert(turn, recording).is_some() {
                return Err(ReplayError::DuplicateTurn { turn });
            }
        }
        Ok(ReplayDecider {
            recordings: by_turn,
            pending: None,
        })
    }

    /// Reads one recording per file, then `new`.
    pub fn from_files(
        paths: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> Result<ReplayDecider, ReplayError> {
        let mut recordings = Vec::new();
        for path in paths {
            let path = path.as_ref();
            let text = fs::read_to_string(path).map_err(|error| ReplayError::Read {
                path: path.to_path_buf(),
                error: error.to_string(),
            })?;
            let recording = Recording::from_json(&text).map_err(|error| ReplayError::Format {
                path: path.to_path_buf(),
                error,
            })?;
            recordings.push(recording);
        }
        ReplayDecider::new(recordings)
    }

    /// The turns recorded, ascending.
    pub fn turns(&self) -> impl Iterator<Item = Turn> + '_ {
        self.recordings.keys().copied()
    }
}

impl Decider for ReplayDecider {
    fn submit(&mut self, request: &Request) -> Result<(), DeciderFailure> {
        let turn = request.turn;
        self.pending = Some(match self.recordings.get(&turn) {
            None => Err(DeciderFailure::NoRecording { turn }),
            Some(recording) if recording.request != *request => {
                Err(DeciderFailure::RequestDiffers { turn })
            }
            Some(recording) => Ok(recording.reply.clone()),
        });
        Ok(())
    }

    fn collect(&mut self) -> Result<Reply, DeciderFailure> {
        self.pending.take().expect("collect follows submit")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decider::Decision;
    use crate::decider::view::test_view;

    fn recording(turn: u64, business: &str) -> Recording {
        let mut view = test_view();
        view.turn = Turn::new(turn);
        view.business = business.to_string();
        Recording {
            request: Request {
                turn: Turn::new(turn),
                views: vec![view],
            },
            reply: Reply {
                turn: Turn::new(turn),
                decisions: vec![Decision::empty(business)],
            },
        }
    }

    fn decider() -> ReplayDecider {
        ReplayDecider::new([recording(3, "b"), recording(2, "a")]).unwrap()
    }

    #[test]
    fn replay_answers_the_recorded_turn() {
        let mut decider = decider();
        assert_eq!(
            decider.turns().collect::<Vec<_>>(),
            [Turn::new(2), Turn::new(3)]
        );
        let recorded = recording(2, "a");
        decider.submit(&recorded.request).unwrap();
        assert_eq!(decider.collect().unwrap(), recorded.reply);
    }

    #[test]
    fn replay_fails_for_a_missing_turn_and_a_different_request() {
        let mut decider = decider();
        let mut missing = recording(2, "a").request;
        missing.turn = Turn::new(4);
        decider.submit(&missing).unwrap();
        assert_eq!(
            decider.collect(),
            Err(DeciderFailure::NoRecording { turn: Turn::new(4) })
        );
        let mut different = recording(2, "a").request;
        different.views[0].cash = crate::test_support::q(99.0);
        decider.submit(&different).unwrap();
        assert_eq!(
            decider.collect(),
            Err(DeciderFailure::RequestDiffers { turn: Turn::new(2) })
        );
    }

    #[test]
    fn two_recordings_for_one_turn_are_refused() {
        let error = ReplayDecider::new([recording(2, "a"), recording(2, "b")])
            .err()
            .expect("duplicate");
        assert_eq!(error, ReplayError::DuplicateTurn { turn: Turn::new(2) });
        assert_eq!(error.to_string(), "two recordings for turn 2");
    }

    #[test]
    fn from_files_reports_a_missing_and_a_malformed_file() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.json");
        let error = ReplayDecider::from_files([&missing]).err().unwrap();
        assert!(matches!(&error, ReplayError::Read { path, .. } if *path == missing));
        assert!(
            error
                .to_string()
                .starts_with(&format!("{}: can't be read: ", missing.display()))
        );

        let bad = dir.path().join("bad.json");
        fs::write(&bad, "{").unwrap();
        let error = ReplayDecider::from_files([&bad]).err().unwrap();
        assert!(matches!(&error, ReplayError::Format { path, .. } if *path == bad));
        assert!(
            error
                .to_string()
                .starts_with(&format!("{}: not a recording: ", bad.display()))
        );

        let good = dir.path().join("good.json");
        fs::write(&good, recording(5, "a").to_json()).unwrap();
        let decider = ReplayDecider::from_files([&good]).unwrap();
        assert_eq!(decider.turns().collect::<Vec<_>>(), [Turn::new(5)]);
    }
}
