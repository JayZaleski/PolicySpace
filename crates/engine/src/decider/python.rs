//! The project's Python: the interpreter of the virtual environment, found relative to the
//! project root; the time limit; the command that runs a named decider; and the host a
//! setting asks for (one external process per distinct decider name).

use std::collections::BTreeSet;
use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::decider::external::{ExternalCommand, ExternalDecider, ProcessConfig, StartError};
use crate::decider::host::DeciderHost;
use crate::setting::LoadedSetting;

/// Overrides the interpreter (a path).
pub const INTERPRETER_VAR: &str = "POLICYSPACE_PYTHON";
/// Overrides the time limit: seconds, `0` for none.
pub const TIME_LIMIT_VAR: &str = "POLICYSPACE_DECIDER_TIME_LIMIT";

/// Why the project's Python can't be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonError {
    /// No interpreter at `path` (the virtual environment's, or the override's).
    InterpreterNotFound {
        /// Where it was looked for.
        path: PathBuf,
    },
    /// `POLICYSPACE_DECIDER_TIME_LIMIT` isn't a non-negative number of seconds.
    BadTimeLimit {
        /// The text found.
        value: String,
    },
}

impl fmt::Display for PythonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PythonError::InterpreterNotFound { path } => write!(
                f,
                "the project's Python environment wasn't found at {}; create it with \
                 `python -m venv .venv` and `.venv/Scripts/python -m pip install -r \
                 requirements.txt` from the project root, or set {INTERPRETER_VAR} to an \
                 interpreter",
                path.display()
            ),
            PythonError::BadTimeLimit { value } => write!(
                f,
                "{TIME_LIMIT_VAR} must be a number of seconds (0 for no limit), not '{value}'"
            ),
        }
    }
}

impl std::error::Error for PythonError {}

/// Why a setting's host couldn't be built.
#[derive(Debug, Clone, PartialEq)]
pub enum HostError {
    /// The project's Python can't be used.
    Python(PythonError),
    /// A decider's process wouldn't start.
    Start {
        /// The decider name.
        decider: String,
        /// Why not.
        error: StartError,
    },
}

impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostError::Python(error) => write!(f, "{error}"),
            HostError::Start { decider, error } => write!(f, "decider '{decider}': {error}"),
        }
    }
}

impl std::error::Error for HostError {}

/// The project root: the engine crate's manifest directory joined with `../..`.
pub fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `project_root()/.venv/Scripts/python.exe` on Windows, `.venv/bin/python` elsewhere.
pub fn venv_interpreter() -> PathBuf {
    let venv = project_root().join(".venv");
    if cfg!(windows) {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python")
    }
}

/// The interpreter: the override if set, else the virtual environment's; must exist as a
/// file.
pub fn interpreter() -> Result<PathBuf, PythonError> {
    interpreter_from(env::var_os(INTERPRETER_VAR).as_deref())
}

/// [`interpreter`] with the override's value given instead of read from the environment.
pub fn interpreter_from(override_path: Option<&OsStr>) -> Result<PathBuf, PythonError> {
    let path = match override_path {
        Some(path) => PathBuf::from(path),
        None => venv_interpreter(),
    };
    if path.is_file() {
        Ok(path)
    } else {
        Err(PythonError::InterpreterNotFound { path })
    }
}

/// The time limit: 5 s unless overridden.
pub fn time_limit() -> Result<Option<Duration>, PythonError> {
    time_limit_from(env::var(TIME_LIMIT_VAR).ok().as_deref())
}

/// [`time_limit`] with the override's value given instead of read from the environment.
pub fn time_limit_from(value: Option<&str>) -> Result<Option<Duration>, PythonError> {
    let Some(text) = value else {
        return Ok(Some(ProcessConfig::DEFAULT_TIME_LIMIT));
    };
    let bad = || PythonError::BadTimeLimit {
        value: text.to_string(),
    };
    let seconds: f64 = text.trim().parse().map_err(|_| bad())?;
    if !seconds.is_finite() || seconds < 0.0 {
        return Err(bad());
    }
    if seconds == 0.0 {
        return Ok(None);
    }
    Duration::try_from_secs_f64(seconds)
        .map(Some)
        .map_err(|_| bad())
}

/// `ProcessConfig { time_limit: time_limit()?, start_grace: default }`.
pub fn process_config() -> Result<ProcessConfig, PythonError> {
    Ok(ProcessConfig {
        time_limit: time_limit()?,
        start_grace: ProcessConfig::DEFAULT_START_GRACE,
    })
}

/// `interpreter()? -m policyspace {decider}`.
pub fn script_command(decider: &str) -> Result<ExternalCommand, PythonError> {
    Ok(ExternalCommand::new(interpreter()?)
        .arg("-m")
        .arg("policyspace")
        .arg(decider))
}

/// One `ExternalDecider` per distinct decider name among `setting.start.businesses`, in
/// name order, each started with `script_command(name)` and `process_config()`, in a
/// `DeciderHost::new()`. A setting with no businesses gives an empty host without touching
/// Python. The first start failure is returned (the deciders started before it are dropped).
pub fn host_for(setting: &LoadedSetting) -> Result<DeciderHost, HostError> {
    let names: BTreeSet<&str> = setting
        .start
        .businesses
        .iter()
        .map(|(_, _, business)| business.decider.as_str())
        .collect();
    let mut host = DeciderHost::new();
    if names.is_empty() {
        return Ok(host);
    }
    let config = process_config().map_err(HostError::Python)?;
    for name in names {
        let command = script_command(name).map_err(HostError::Python)?;
        let decider =
            ExternalDecider::start(command, config).map_err(|error| HostError::Start {
                decider: name.to_string(),
                error,
            })?;
        host.add(name, decider);
    }
    Ok(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interpreter_is_the_venvs_unless_overridden() {
        let expected = if cfg!(windows) {
            ".venv/Scripts/python.exe"
        } else {
            ".venv/bin/python"
        };
        assert!(venv_interpreter().ends_with(expected));

        let error = interpreter_from(Some(OsStr::new("no/such/python"))).unwrap_err();
        assert_eq!(
            error,
            PythonError::InterpreterNotFound {
                path: PathBuf::from("no/such/python")
            }
        );
        let text = error.to_string();
        assert!(text.contains("no/such/python"), "{text}");
        assert!(text.contains("python -m venv .venv"), "{text}");
        assert!(text.contains("POLICYSPACE_PYTHON"), "{text}");

        let exe = env::current_exe().unwrap();
        assert_eq!(interpreter_from(Some(exe.as_os_str())), Ok(exe.clone()));
    }

    #[test]
    fn the_time_limit_parses() {
        assert_eq!(time_limit_from(None), Ok(Some(Duration::from_secs(5))));
        assert_eq!(time_limit_from(Some("0")), Ok(None));
        assert_eq!(
            time_limit_from(Some("2.5")),
            Ok(Some(Duration::from_millis(2500)))
        );
        assert_eq!(
            time_limit_from(Some(" 10 ")),
            Ok(Some(Duration::from_secs(10)))
        );
        for bad in ["abc", "-1", "inf", ""] {
            let error = time_limit_from(Some(bad)).unwrap_err();
            assert_eq!(
                error,
                PythonError::BadTimeLimit {
                    value: bad.to_string()
                }
            );
            assert!(error.to_string().contains(&format!("not '{bad}'")));
        }
    }

    #[test]
    fn a_host_error_displays_its_cause() {
        let error = HostError::Python(PythonError::BadTimeLimit {
            value: "x".to_string(),
        });
        assert_eq!(
            error.to_string(),
            PythonError::BadTimeLimit {
                value: "x".to_string()
            }
            .to_string()
        );
        let start = HostError::Start {
            decider: "rule".to_string(),
            error: StartError::Spawn {
                command: "py".to_string(),
                error: "gone".to_string(),
            },
        };
        assert_eq!(
            start.to_string(),
            "decider 'rule': 'py' couldn't be started: gone"
        );
    }
}
