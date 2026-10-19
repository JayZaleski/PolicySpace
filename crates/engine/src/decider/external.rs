//! The external-process decider: a program started once with its arguments, kept alive
//! across turns, written one compact request line per turn on its standard input and read
//! one reply line from its standard output, with a time limit per exchange. Its error output
//! is captured so a failure can quote it. Two threads pump its output and error output
//! (the only way std Rust waits on a pipe with a time limit, and keeps a talkative process
//! from blocking); they do no simulation work and end with the process. A failed process
//! is killed and started again at the next submit. Dropping the decider kills the process.

use std::collections::VecDeque;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::decider::host::{Decider, DeciderFailure};
use crate::decider::{Reply, Request};

/// How many lines of error output are kept.
pub const STDERR_TAIL_LINES: usize = 20;

/// A program and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCommand {
    /// The program to run.
    pub program: OsString,
    /// Its arguments, in order.
    pub args: Vec<OsString>,
}

impl ExternalCommand {
    /// A command that runs `program` with no arguments.
    pub fn new(program: impl Into<OsString>) -> ExternalCommand {
        ExternalCommand {
            program: program.into(),
            args: Vec::new(),
        }
    }

    /// Adds one argument.
    pub fn arg(mut self, arg: impl Into<OsString>) -> ExternalCommand {
        self.args.push(arg.into());
        self
    }

    /// Adds several arguments.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<OsString>>) -> ExternalCommand {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }
}

impl fmt::Display for ExternalCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.program.to_string_lossy())?;
        for arg in &self.args {
            write!(f, " {}", arg.to_string_lossy())?;
        }
        Ok(())
    }
}

/// Host configuration for a process: about the machine, not the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessConfig {
    /// How long to wait for a reply; None waits forever.
    pub time_limit: Option<Duration>,
    /// How long after starting to watch for an exit before calling the start good.
    pub start_grace: Duration,
}

impl ProcessConfig {
    /// The default time limit per exchange: 5 seconds.
    pub const DEFAULT_TIME_LIMIT: Duration = Duration::from_secs(5);
    /// The default start grace window: 500 milliseconds.
    pub const DEFAULT_START_GRACE: Duration = Duration::from_millis(500);
}

impl Default for ProcessConfig {
    fn default() -> ProcessConfig {
        ProcessConfig {
            time_limit: Some(ProcessConfig::DEFAULT_TIME_LIMIT),
            start_grace: ProcessConfig::DEFAULT_START_GRACE,
        }
    }
}

/// Why a process couldn't be started.
#[derive(Debug, Clone, PartialEq)]
pub enum StartError {
    /// The program couldn't be spawned (not found, not executable).
    Spawn {
        /// The command line.
        command: String,
        /// The operating system's message.
        error: String,
    },
    /// It exited within the grace window.
    ExitedAtOnce {
        /// The command line.
        command: String,
        /// How it exited.
        status: ExitStatus,
        /// The tail of its error output.
        stderr: Vec<String>,
    },
}

impl StartError {
    /// The one-line form: `'{command}' couldn't be started: {error}` or `'{command}' exited
    /// ({status}) as soon as it started`, never the stderr tail.
    pub fn summary(&self) -> String {
        match self {
            StartError::Spawn { command, error } => {
                format!("'{command}' couldn't be started: {error}")
            }
            StartError::ExitedAtOnce {
                command, status, ..
            } => format!("'{command}' exited ({status}) as soon as it started"),
        }
    }

    /// The stderr tail of `ExitedAtOnce`, empty for `Spawn`.
    pub fn stderr_tail(&self) -> &[String] {
        match self {
            StartError::Spawn { .. } => &[],
            StartError::ExitedAtOnce { stderr, .. } => stderr,
        }
    }
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.summary())?;
        let tail = self.stderr_tail();
        if !tail.is_empty() {
            write!(f, "\nits error output ends with:")?;
            for line in tail {
                write!(f, "\n  {line}")?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for StartError {}

/// A running process, or none between a failure and the next submit.
struct Process {
    child: Child,
    stdin: ChildStdin,
    /// Lines from the stdout pump.
    lines: mpsc::Receiver<io::Result<String>>,
    /// The tail of the error output, from the stderr pump.
    stderr: Arc<Mutex<VecDeque<String>>>,
    pumps: Vec<JoinHandle<()>>,
}

impl Process {
    /// Kills the process, waits for it, joins the pumps and snapshots the stderr tail.
    fn stop(mut self) -> (ExitStatus, Vec<String>) {
        let _ = self.child.kill();
        let status = self
            .child
            .wait()
            .expect("waiting for a killed process can't fail");
        for pump in self.pumps.drain(..) {
            let _ = pump.join();
        }
        let tail = snapshot(&self.stderr);
        (status, tail)
    }
}

fn snapshot(tail: &Mutex<VecDeque<String>>) -> Vec<String> {
    tail.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .cloned()
        .collect()
}

/// A decider that is an external program speaking the decider protocol on its standard
/// streams.
pub struct ExternalDecider {
    command: ExternalCommand,
    config: ProcessConfig,
    process: Option<Process>,
}

impl ExternalDecider {
    /// Starts the process and watches it for `config.start_grace`.
    pub fn start(
        command: ExternalCommand,
        config: ProcessConfig,
    ) -> Result<ExternalDecider, StartError> {
        let mut decider = ExternalDecider {
            command,
            config,
            process: None,
        };
        decider.process = Some(decider.spawn()?);
        Ok(decider)
    }

    /// The command this decider runs.
    pub fn command(&self) -> &ExternalCommand {
        &self.command
    }

    /// The configuration this decider was started with.
    pub fn config(&self) -> ProcessConfig {
        self.config
    }

    /// True while a process is held (it may have exited unnoticed; the next exchange finds
    /// out).
    pub fn is_running(&self) -> bool {
        self.process.is_some()
    }

    /// Starts the process and its two pumps, then watches it for the grace window.
    fn spawn(&self) -> Result<Process, StartError> {
        let mut child = Command::new(&self.command.program)
            .args(&self.command.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| StartError::Spawn {
                command: self.command.to_string(),
                error: error.to_string(),
            })?;
        let stdin = child.stdin.take().expect("stdin was piped");
        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        let (sender, lines) = mpsc::channel();
        let out_pump = thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        let tail = Arc::new(Mutex::new(VecDeque::new()));
        let pump_tail = Arc::clone(&tail);
        let err_pump = thread::spawn(move || {
            let mut reader = BufReader::new(stderr);
            let mut bytes = Vec::new();
            loop {
                bytes.clear();
                match reader.read_until(b'\n', &mut bytes) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
                let text = String::from_utf8_lossy(&bytes);
                let line = text.trim_end_matches(['\r', '\n']).to_string();
                let mut tail = pump_tail
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                tail.push_back(line);
                if tail.len() > STDERR_TAIL_LINES {
                    tail.pop_front();
                }
            }
        });
        let mut process = Process {
            child,
            stdin,
            lines,
            stderr: tail,
            pumps: vec![out_pump, err_pump],
        };

        let started = Instant::now();
        loop {
            match process.child.try_wait() {
                Ok(Some(status)) => {
                    for pump in process.pumps.drain(..) {
                        let _ = pump.join();
                    }
                    return Err(StartError::ExitedAtOnce {
                        command: self.command.to_string(),
                        status,
                        stderr: snapshot(&process.stderr),
                    });
                }
                Ok(None) => {}
                Err(_) => break,
            }
            if started.elapsed() >= self.config.start_grace {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        Ok(process)
    }

    /// Stops the process after a failure; `self.process` is `None` afterwards.
    fn stop(&mut self) -> (ExitStatus, Vec<String>) {
        self.process
            .take()
            .expect("a failure follows an exchange with a process")
            .stop()
    }
}

impl Decider for ExternalDecider {
    fn reject(&mut self) {
        if self.process.is_some() {
            let _ = self.stop();
        }
    }

    fn submit(&mut self, request: &Request) -> Result<(), DeciderFailure> {
        if self.process.is_none() {
            let process = self
                .spawn()
                .map_err(|error| DeciderFailure::WontStart { error })?;
            self.process = Some(process);
        }
        let mut line = request.to_json();
        line.push('\n');
        let process = self.process.as_mut().expect("a process was just ensured");
        let written = process
            .stdin
            .write_all(line.as_bytes())
            .and_then(|()| process.stdin.flush());
        if written.is_err() {
            let (status, stderr) = self.stop();
            return Err(DeciderFailure::Exited { status, stderr });
        }
        Ok(())
    }

    fn collect(&mut self) -> Result<Reply, DeciderFailure> {
        let limit = self.config.time_limit;
        let process = self.process.as_ref().expect("collect follows submit");
        let received = match limit {
            Some(limit) => process.lines.recv_timeout(limit),
            None => process
                .lines
                .recv()
                .map_err(|_| RecvTimeoutError::Disconnected),
        };
        match received {
            Ok(Ok(line)) => match Reply::from_json(line.trim_end_matches(['\r', '\n'])) {
                Ok(reply) => Ok(reply),
                Err(error) => {
                    let (_, stderr) = self.stop();
                    Err(DeciderFailure::Unreadable { error, stderr })
                }
            },
            Ok(Err(_)) | Err(RecvTimeoutError::Disconnected) => {
                let (status, stderr) = self.stop();
                Err(DeciderFailure::Exited { status, stderr })
            }
            Err(RecvTimeoutError::Timeout) => {
                let (_, stderr) = self.stop();
                Err(DeciderFailure::TimedOut {
                    limit: limit.expect("only a limited wait times out"),
                    stderr,
                })
            }
        }
    }
}

impl Drop for ExternalDecider {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.child.kill();
            let _ = process.child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_displays_as_its_words() {
        let command = ExternalCommand::new("python")
            .arg("-m")
            .args(["policyspace", "arithmetic"]);
        assert_eq!(command.to_string(), "python -m policyspace arithmetic");
        assert_eq!(command.args.len(), 3);
    }

    #[test]
    fn the_default_config() {
        let config = ProcessConfig::default();
        assert_eq!(config.time_limit, Some(Duration::from_secs(5)));
        assert_eq!(config.start_grace, Duration::from_millis(500));
    }

    #[test]
    fn a_program_that_doesnt_exist_fails_to_start() {
        let name = "policyspace-no-such-program-7f3a";
        let error = ExternalDecider::start(ExternalCommand::new(name), ProcessConfig::default())
            .err()
            .expect("it can't start");
        match &error {
            StartError::Spawn { command, .. } => assert_eq!(command, name),
            other => panic!("{other:?}"),
        }
        assert!(
            error
                .to_string()
                .starts_with("'policyspace-no-such-program-7f3a' couldn't be started: ")
        );
    }
}
