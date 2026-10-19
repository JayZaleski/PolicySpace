# Step 07: decider-host — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-03-factories/step-07-decider-host/design.md` — the behavior this spec implements: the host and the Decider, name resolution, the decision point, the failure semantics (including the grace window and the I/O pump threads), the three deciders, finding Python, the report and the dev UI.
- `PROJECT_PLAN/stage-03-factories/step-05-business-view-and-decisions/docs_changes.md` — the decider protocol as built (`docs/decider.md` isn't in `docs/` yet; this file is its draft): the view, the decision, `Reply::validate` and `Answer`, the recordings, the JSON format, and the note that a request read from a file isn't checked for consistency.
- `PROJECT_PLAN/stage-03-factories/step-06-business-script/docs_changes.md` — the Python package: `python -m policyspace <decider>` (one request line in, one reply line out, flushed; exit 2 before reading anything for an unknown name; UTF-8, `\n` line endings), the committed recordings in `crates/engine/tests/recordings/` and how they are regenerated.
- `docs/world.md`, `docs/dev-ui.md`, `docs/workspace.md` — the World and the three-part turn, the dev UI's session and checker, the check command and test layout.
- Code: `crates/engine/src/world.rs` (`World::new`, `step`, `run_turn`, `TurnReport`, `StepError`, the accessors and the unit test at the bottom), `crates/engine/src/decider/mod.rs`, `batch.rs` (`Request`, `Reply`, `Recording`, `Answer`, `Reply::validate`, `ReplyErrors`), `decision.rs` (`Decision`, `DecisionErrors`), `view.rs`, `crates/engine/src/businesses.rs` (`Business.decider`, `Businesses::iter`), `crates/engine/src/id.rs` (`IdVec`), `crates/engine/src/setting/mod.rs` (`LoadedSetting`, `StartState`), `crates/engine/src/lib.rs`, `crates/engine/tests/common/mod.rs`, `crates/engine/tests/recordings.rs` (`sample_request`: how the engine builds the sample's request), `crates/engine/tests/decider.rs`.
- Dev UI: `crates/dev-ui/src/session.rs` (`Session`, `Loaded`, `load`, `step`, its tests), `check.rs` (`run`, `run_turns`), `text.rs` (`error_lines`, `turn_line`; its tests build worlds), `app.rs` (how `Session::new` and `Loaded` are used; it changes in one line), `test_support.rs`, `crates/dev-ui/tests/checker.rs`.
- Python: `python/policyspace/__main__.py`, `deciders.py` (`serve`).
- Settings: `settings/sample/start/businesses.toml` (three businesses, every one `decider = "arithmetic"`), `crates/engine/tests/settings/README.md` for `businesses_valid` (`farm_a` and `farm_b` name `rule`, `idle` names `other_rule`; `idle` sits on a hex with no market) and `production_valid` (`works` and `springs` name `rule`).

## Approach

- **Four new files in `crates/engine/src/decider/`**: `host.rs` (the `Decider` trait, `DeciderHost`, the decision point, `BusinessAnswer`, `DeciderFailure`, the report types, `ClosureDecider`), `external.rs` (the external-process decider), `replay.rs` (the replay decider), `python.rs` (finding the project's Python, the host built from a setting). Rationale: the design's parts, each changeable alone; the protocol files from Step 05 don't change.
- **The World takes the host explicitly, everywhere.** `World::new(setting, &mut host)` and `World::step(&mut host)`; there is no host-less form, so a caller can't forget it. Every existing caller passes `DeciderHost::idle()`; the dev UI passes the host it built. Rationale: the design's rule that the World holds no decider between turns, made visible in the signature. The edits are mechanical (about 70 call sites, all in tests except the dev UI's session, checker and text tests).
- **Names are checked at creation and at the top of every step.** `World::new` fails with `UnknownDecider` when a business names a decider the host doesn't hold; `World::step` returns `StepError::UnknownDecider` for the same, before anything changes, because a rebuilt host may lack a name. The idle host holds every name.
- **A two-phase `Decider` trait: `submit` then `collect`.** The host submits every decider's request before collecting any reply, so external processes work at the same time. Replay and closure deciders compute their reply in `submit` and hand it over in `collect`. Rationale: the design's step 2 without threads in the host.
- **Owned answers in the World.** `Reply::validate`'s `Answer<'a>` borrows the reply, and the World must keep answers across the turn, so the host turns them into an owned `BusinessAnswer` (`Nothing`, `Valid(Decision)`, `Refused { decision, errors }`). The refused form keeps the decision as the design asks. `Answer` stays as it is.
- **Failures are structured.** `DeciderFailure` is an enum with one variant per cause in the design; the report carries `(decider name, failure)` pairs and counts. The dev UI's lines are built from the enum's `Display`. Rationale: tests match variants, not strings.
- **External processes get two I/O pump threads** (stdout lines into a channel, stderr into a bounded tail), as `design.md` now allows; the host waits on the channel with the time limit. A failed process is killed and waited for, its threads joined, and the stderr tail snapshotted into the failure. A grace window after starting catches a process that exits at once.
- **Finding Python lives in the engine** (`decider::python`), relative to the engine crate's manifest directory, so the engine's tests and the dev UI start the same interpreter. `DeciderHost::for_setting` (in `python.rs`) builds the host the dev UI and the checker use. Rationale: one definition; the engine stays setting-agnostic because the setting's own decider names drive it.
- **Environment lookups are testable without touching the environment**: each lookup has a `_from(value)` form that the public form feeds from `std::env`. Rationale: tests run in parallel in one process; a test must never set a process-wide variable.
- **The Session keeps the host beside the world**, and takes a `Hosting` choice at construction (`Idle`, `FromSetting`, or a custom builder), so existing session tests use the idle host, the window uses the setting's deciders, and a test can inject a closure decider. `Loaded::World(world)` doesn't change shape, so `app.rs` changes only where it builds the session.
- **The build runs as one.** The dev UI's changes need the engine's new types, the world wiring and the host are one piece of work, and the tests cross everything; no package could be built and tested alone.

## Changes

### New

- `crates/engine/src/decider/host.rs` — `Decider`, `DeciderHost`, `UnknownDecider`, the decision point, `BusinessAnswer`, `DeciderFailure`, `ReportedFailure`, `DecisionCounts`, `ClosureDecider`; unit tests.
- `crates/engine/src/decider/external.rs` — `ExternalCommand`, `ProcessConfig`, `StartError`, `ExternalDecider`; unit tests that don't need Python (command display, config defaults, a program that doesn't exist).
- `crates/engine/src/decider/replay.rs` — `ReplayDecider`, `ReplayError`; unit tests.
- `crates/engine/src/decider/python.rs` — `interpreter`, `time_limit`, `process_config`, `script_command`, `host_for`, `PythonError`, `HostError`; unit tests of the `_from` forms.
- `crates/engine/tests/decider_host.rs` — integration tests that need no Python (Test Plan).
- `crates/engine/tests/external_decider.rs` — integration tests that run the project's Python (Test Plan).

### Modified

- `crates/engine/src/decider/mod.rs` — `pub mod external; pub mod host; pub mod python; pub mod replay;` and re-exports (Interfaces & Data). Module docs: replace the last sentence ("Nothing here runs a decider ...") with: the [`host`] module runs the exchange once per turn through a [`DeciderHost`] of named [`Decider`]s (an external process, a replayed recording, or a Rust closure), stores each business's [`BusinessAnswer`] and reports every [`DeciderFailure`]; Step 08 resolves the valid decisions.
- `crates/engine/src/world.rs` — `World::new` and `World::step` take `&mut DeciderHost`; `World::new` returns `Result`; `StepError::UnknownDecider`; the decision point in `run_turn`; `business_answers` and `business_answer`; `TurnReport` gains `decisions` and `decider_failures`; module docs (below); the unit tests at the bottom use `DeciderHost::idle()` and also assert `business_answers` is unchanged by a refused step.
- `crates/engine/src/lib.rs` — re-exports; crate docs: replace "no decider runs yet (Steps 06 and 07)" with "a [`DeciderHost`] of named deciders (a Python process the engine starts and keeps alive, a replayed recording, or a closure) answers every business's view once per turn at the decision point, and the world stores each business's answer; nothing resolves the decisions yet (Step 08)".
- `crates/engine/src/trade.rs` — its unit tests' `World::new(...)` calls pass `&mut DeciderHost::idle()`.
- `crates/engine/tests/common/mod.rs` — `world_of` builds with an idle host (`DeciderHost::idle()`), unwrapping the result; new helper `pub fn idle() -> DeciderHost`.
- Every engine integration test that calls `world.step()` — `crates/engine/tests/businesses.rs`, `decider.rs`, `factories.rs`, `growth.rs`, `market.rs`, `pops.rs`, `trade.rs`, `world.rs`, and `demand.rs`/`growth.rs` where they call `World::new` directly — passes `&mut common::idle()` (or a local `let mut host = idle();`). Expectations don't change.
- `crates/dev-ui/src/session.rs` — `Hosting`, `Session::new(hosting)`, the host beside the world, failure lines in the log; existing tests use `Hosting::Idle`; new tests (Test Plan).
- `crates/dev-ui/src/check.rs` — `run_turns` builds the host from the setting, prints its errors like load errors, prints failure lines after each turn line.
- `crates/dev-ui/src/text.rs` — `failure_lines`, `host_error_lines`; its tests build worlds with an idle host.
- `crates/dev-ui/src/app.rs` — `Session::new(Hosting::FromSetting)`; nothing else.
- `crates/dev-ui/src/test_support.rs` — `world_at` uses an idle host.
- `crates/dev-ui/src/main.rs` — module docs: the window and `check --turns` start the setting's deciders as Python processes; a decider that won't start is a load error; the turn log shows decider failures.
- `crates/dev-ui/tests/checker.rs` — two new tests (Test Plan); the existing sample `--turns` tests now run the real `arithmetic` decider and must still pass unchanged.

Nothing else changes: not the protocol types, `Quantity`, `Turn`, the loader, any setting (sample or test), the recordings, the Python package, `tools/check.py`, `docs/`, or `manual.ipynb`.

## Interfaces & Data

Every public item has a doc comment (`missing_docs` is on). Every error type implements `Display` and `std::error::Error`. "Stderr tail" below means the last `STDERR_TAIL_LINES` (20) lines of a process's error output, oldest first, each without its line ending.

### `crates/engine/src/decider/host.rs`

```rust
//! The decision point: once per turn, every business's view goes to the decider its setting
//! entry names and every decision comes back, one batched exchange per decider. The host
//! is a table of deciders by name, built by the caller (the dev UI, the checker, a test),
//! never by the World; the World takes it at creation and at every step and reads from it
//! only during the decision point, so it holds no decider between turns. Every decider
//! receives exactly one request per turn (empty when it controls no business), deciders
//! are taken in name order, and every request is submitted before any reply is collected.
//! A refused decision idles its business; a failed decider idles every business it
//! controls and is reported; the turn never halts.

/// Answers one turn's request with one reply. Implementations: the external process, the
/// replay decider and the closure decider.
pub trait Decider {
    /// Takes the turn's request. An external process writes it; the others prepare the
    /// reply. A failure here means the decider can't answer this turn.
    fn submit(&mut self, request: &Request) -> Result<(), DeciderFailure>;
    /// Returns the reply to the request last submitted. Called exactly once after every
    /// successful `submit`, never otherwise.
    fn collect(&mut self) -> Result<Reply, DeciderFailure>;
    /// Told that the reply just collected was refused as a whole (wrong turn, unknown or
    /// duplicate business). The default does nothing; the external process stops itself so
    /// that a process one reply behind is started afresh next turn, as the design asks.
    fn reject(&mut self) {}
}

/// Why a decider couldn't answer a turn. Only the process variants carry a stderr tail.
#[derive(Debug, Clone, PartialEq)]
pub enum DeciderFailure {
    /// The process exited, or closed its pipes, before answering.
    Exited { status: std::process::ExitStatus, stderr: Vec<String> },
    /// No reply within the time limit; the process was killed.
    TimedOut { limit: Duration, stderr: Vec<String> },
    /// The reply line couldn't be read as a reply; the process was killed.
    Unreadable { error: FormatError, stderr: Vec<String> },
    /// The process had failed earlier and couldn't be started again.
    WontStart { error: StartError },
    /// The reply was refused as a whole (wrong turn, unknown or duplicate business).
    Refused(ReplyErrors),
    /// The replay decider has no recording for this turn.
    NoRecording { turn: Turn },
    /// The replay decider's recording for this turn holds a different request.
    RequestDiffers { turn: Turn },
}
impl DeciderFailure {
    /// The stderr tail, empty for the variants that have none. For `WontStart` it is the
    /// `StartError::ExitedAtOnce` tail (empty for `Spawn`).
    pub fn stderr_tail(&self) -> &[String];
}
// Display, always one line, without the stderr tail: `the process exited ({status}) before
// answering` (`{status}` is ExitStatus's Display); `no reply within the time limit of
// {secs:.1} s`; `the reply couldn't be read: {error}` (a FormatError::Values joins its lines
// with "; "); `the process couldn't be started again: {StartError::summary()}`; `the reply
// was refused: {e1}; {e2}` (ReplyErrors joined with "; "); `no recording for turn {turn}`;
// `the request for turn {turn} differs from the recorded one`.

/// One decider's failure in one turn, as the turn report carries it.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportedFailure { pub decider: String, pub failure: DeciderFailure }
// Display: `decider '{decider}': {failure}`, then, when the tail is non-empty, a line
// `its error output ends with:` and each tail line prefixed by two spaces.

/// How many businesses sent a valid decision, were refused, and sent nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecisionCounts { pub valid: usize, pub refused: usize, pub nothing: usize }

/// What one business got at the decision point. Kept in the World until the next one.
#[derive(Debug, Clone, PartialEq)]
pub enum BusinessAnswer {
    /// No decision (none sent, or its decider failed): the business does nothing.
    Nothing,
    /// A decision that passed validation against the business's view.
    Valid(Decision),
    /// A decision that failed validation, kept with its errors; the business does nothing.
    Refused { decision: Decision, errors: DecisionErrors },
}

/// A business names a decider the host doesn't hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownDecider { pub business: String, pub decider: String, pub known: Vec<String> }
// Display: `business '{business}' names decider '{decider}', which the host doesn't hold
// (it holds: a, b)` or `... (it holds no deciders)`.

/// The deciders by name. `idle()` holds every name and answers nothing for names it has no
/// decider for; `new()` holds only what is added.
pub struct DeciderHost { deciders: BTreeMap<String, Box<dyn Decider>>, accepts_unknown: bool }
impl DeciderHost {
    /// An empty host that accepts only the names added to it.
    pub fn new() -> DeciderHost;
    /// The idle host: every name is accepted, and a name with no decider gets nothing.
    pub fn idle() -> DeciderHost;
    /// Adds a decider under `name`.
    /// # Panics
    /// Panics if the host already holds `name` (`a decider named '{name}' is already in the host`).
    pub fn add(&mut self, name: impl Into<String>, decider: impl Decider + 'static) -> &mut DeciderHost;
    /// `add` in builder form.
    pub fn with(self, name: impl Into<String>, decider: impl Decider + 'static) -> DeciderHost;
    /// True for the idle host.
    pub fn accepts_unknown(&self) -> bool;
    /// True if `name` may be used by a business: it was added, or the host accepts every name.
    pub fn holds(&self, name: &str) -> bool;
    /// The names added, in name order.
    pub fn names(&self) -> impl Iterator<Item = &str> + '_;

    /// The first business (in ID order) whose decider the host doesn't hold, if any.
    pub(crate) fn check(&self, businesses: &Businesses) -> Result<(), UnknownDecider>;
    /// The decision point for `world`'s current turn (below).
    pub(crate) fn decision_point(&mut self, world: &World) -> DecisionPoint;
}
impl Default for DeciderHost { /* new() */ }
impl fmt::Debug for DeciderHost { /* `DeciderHost { names: [..], accepts_unknown: .. }` */ }

/// What the decision point produced: one answer per business by index, and the failures.
pub(crate) struct DecisionPoint { pub(crate) answers: Vec<BusinessAnswer>, pub(crate) failures: Vec<ReportedFailure> }

/// A decider built from a Rust function, for tests.
pub struct ClosureDecider { f: Box<dyn FnMut(&Request) -> Reply>, pending: Option<Reply> }
impl ClosureDecider { pub fn new(f: impl FnMut(&Request) -> Reply + 'static) -> ClosureDecider; }
// submit: pending = Some((f)(request)); collect: pending.take().expect("collect follows submit").
```

`decision_point(world)`, step by step (`businesses = world.businesses()`, `goods = &world.rules().goods`):

1. `answers = vec![BusinessAnswer::Nothing; businesses.len()]`, `failures = vec![]`.
2. For each `(name, decider)` in `self.deciders` (name order): `ids` = every business whose `decider == name`, in ID order (empty allowed); `request = Request { turn: world.turn(), views: ids.map(|id| world.business_view(id)) }`; `decider.submit(&request)`; keep `(name, ids, request, Option<DeciderFailure>)` in a `Vec`. A business whose name has no decider (only possible when `accepts_unknown`) is left at `Nothing`.
3. For each kept entry in the same order, if `submit` succeeded: `decider.collect()`. Then `reply.validate(&request, goods)`: `Err(errors)` → `decider.reject()`, then `DeciderFailure::Refused(errors)`. `Ok(answers)` → for each `(id, answer)` zipped with `ids`: `Answer::Nothing` → `Nothing`; `Answer::Valid(d)` → `Valid(d.clone())`; `Answer::Refused(errors)` → `Refused { decision: the one decision in reply.decisions whose business is the view's (exactly one after validation), errors }`.
4. Any failure (from `submit`, `collect` or validation) is pushed as `ReportedFailure { decider: name, failure }`, and that decider's businesses stay `Nothing`.

`check(businesses)`: for each business in ID order, if `!self.holds(business.decider)` return `Err(UnknownDecider { business: name, decider, known: self.names().collect() })`.

### `crates/engine/src/decider/external.rs`

```rust
//! The external-process decider: a program started once with its arguments, kept alive
//! across turns, written one compact request line per turn on its standard input and read
//! one reply line from its standard output, with a time limit per exchange. Its error output
//! is captured so a failure can quote it. Two threads pump its output and error output
//! (the only way std Rust waits on a pipe with a time limit, and keeps a talkative process
//! from blocking); they do no simulation work and end with the process. A failed process
//! is killed and started again at the next submit. Dropping the decider kills the process.

/// How many lines of error output are kept.
pub const STDERR_TAIL_LINES: usize = 20;

/// A program and its arguments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCommand { pub program: OsString, pub args: Vec<OsString> }
impl ExternalCommand {
    pub fn new(program: impl Into<OsString>) -> ExternalCommand;
    pub fn arg(self, arg: impl Into<OsString>) -> ExternalCommand;
    pub fn args(self, args: impl IntoIterator<Item = impl Into<OsString>>) -> ExternalCommand;
}
// Display: the program and arguments joined by single spaces, each as `to_string_lossy`.

/// Host configuration for a process: about the machine, not the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessConfig {
    /// How long to wait for a reply; None waits forever.
    pub time_limit: Option<Duration>,
    /// How long after starting to watch for an exit before calling the start good.
    pub start_grace: Duration,
}
impl Default for ProcessConfig { /* time_limit: Some(5 s), start_grace: 500 ms */ }
impl ProcessConfig {
    pub const DEFAULT_TIME_LIMIT: Duration = Duration::from_secs(5);
    pub const DEFAULT_START_GRACE: Duration = Duration::from_millis(500);
}

/// Why a process couldn't be started.
#[derive(Debug, Clone, PartialEq)]
pub enum StartError {
    /// The program couldn't be spawned (not found, not executable).
    Spawn { command: String, error: String },           // error: the io::Error's Display
    /// It exited within the grace window.
    ExitedAtOnce { command: String, status: std::process::ExitStatus, stderr: Vec<String> },
}
impl StartError {
    /// The one-line form: `'{command}' couldn't be started: {error}` or `'{command}' exited
    /// ({status}) as soon as it started`, never the stderr tail.
    pub fn summary(&self) -> String;
    /// The stderr tail of `ExitedAtOnce`, empty for `Spawn`.
    pub fn stderr_tail(&self) -> &[String];
}
// Display: `summary()`, then for `ExitedAtOnce` with a non-empty tail a line `its error
// output ends with:` and each tail line prefixed by two spaces.

/// A running process, or none between a failure and the next submit.
struct Process {
    child: Child,
    stdin: ChildStdin,
    lines: mpsc::Receiver<io::Result<String>>,      // from the stdout pump
    stderr: Arc<Mutex<VecDeque<String>>>,            // the tail, from the stderr pump
    pumps: Vec<JoinHandle<()>>,
}

pub struct ExternalDecider { command: ExternalCommand, config: ProcessConfig, process: Option<Process> }
impl ExternalDecider {
    /// Starts the process and watches it for `config.start_grace`.
    pub fn start(command: ExternalCommand, config: ProcessConfig) -> Result<ExternalDecider, StartError>;
    pub fn command(&self) -> &ExternalCommand;
    pub fn config(&self) -> ProcessConfig;
    /// True while a process is held (it may have exited unnoticed; the next exchange finds out).
    pub fn is_running(&self) -> bool;
}
impl Drop for ExternalDecider { /* kill and wait the process, ignoring errors; the pumps end on EOF */ }
```

`spawn(&self) -> Result<Process, StartError>` (private): `Command::new(program).args(args).stdin(piped).stdout(piped).stderr(piped).spawn()` → `Spawn` on error. Take the three pipes. Start the stdout pump: `for line in BufReader::new(stdout).lines() { if tx.send(line).is_err() { break } }`; the thread ends at EOF (and drops `tx`, so the receiver sees `Disconnected`). Start the stderr pump: for each line, push onto the tail, popping the front when the length exceeds `STDERR_TAIL_LINES`. Then the grace window: loop `child.try_wait()`; `Some(status)` → join the pumps (EOF is guaranteed after exit), snapshot the tail, return `ExitedAtOnce`; `None` → sleep 10 ms until `config.start_grace` has elapsed; then return the `Process`. `start` calls `spawn` once.

`stop(&mut self) -> (ExitStatus, Vec<String>)` (private): take `self.process`; `child.kill()` (Ok if already exited), `child.wait()` (the status), join the pumps, snapshot the tail. Used by every failure path, so after any failure `self.process` is `None`.

`Decider` for `ExternalDecider`:

- `reject`: `stop()` if a process is held (its status and tail are discarded), so the next `submit` starts a fresh process.
- `submit`: if `self.process` is `None`, `self.process = Some(self.spawn()?)` mapping the error to `WontStart { error }`. Write `request.to_json()` and `\n` to `stdin` and flush; an `io::Error` → `stop()` → `Exited { status, stderr }`.
- `collect`: `lines.recv_timeout(limit)` (or `recv()` when the limit is `None`): `Ok(Ok(line))` → `Reply::from_json(line.trim_end_matches(['\r', '\n']))`, `Err(error)` → `stop()` → `Unreadable { error, stderr }`; `Ok(Err(_))` or `Err(Disconnected)` → `stop()` → `Exited { status, stderr }`; `Err(Timeout)` → `stop()` → `TimedOut { limit, stderr }`.

Pitfalls: take the pipes out of the `Child` before spawning the pumps; `Child::kill` on an exited process returns `Ok` in current Rust; never block in `Drop` beyond `kill` and `wait`. **Known limit:** the write in `submit` has no time limit. A process that has stopped reading its input blocks `submit` once a request is larger than the pipe buffer (measured: about 64 KB on Windows, roughly 100 businesses on one decider at the sample's 600 bytes per view); today's settings are far below that. A later Step can add a writer thread fed by a channel if settings grow; not this one.

### `crates/engine/src/decider/replay.rs`

```rust
//! The replay decider: answers from recordings, one per turn, so engine tests that don't
//! test strategy need no Python. The engine's request must equal the recorded one, which
//! also checks a request read from a file against an engine-built one.

/// Why a recording file couldn't be used.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplayError {
    Read { path: PathBuf, error: String },        // io::Error's Display
    Format { path: PathBuf, error: FormatError },
    DuplicateTurn { turn: Turn },                 // two recordings for one turn
}
// Display: `{path}: can't be read: {error}`; `{path}: not a recording: {error}`; `two
// recordings for turn {turn}`.

pub struct ReplayDecider { recordings: BTreeMap<Turn, Recording>, pending: Option<Result<Reply, DeciderFailure>> }
impl ReplayDecider {
    /// From recordings in memory; `DuplicateTurn` if two share a request turn.
    pub fn new(recordings: impl IntoIterator<Item = Recording>) -> Result<ReplayDecider, ReplayError>;
    /// Reads one recording per file (`Recording::from_json`), then `new`.
    pub fn from_files(paths: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<ReplayDecider, ReplayError>;
    /// The turns recorded, ascending.
    pub fn turns(&self) -> impl Iterator<Item = Turn> + '_;
}
```

`submit(request)`: `pending = Some(match recordings.get(&request.turn) { None => Err(NoRecording { turn }), Some(r) if r.request != *request => Err(RequestDiffers { turn }), Some(r) => Ok(r.reply.clone()) })`; returns `Ok(())` (the failure surfaces in `collect`, so a replay failure is reported like a process failure). `collect`: `pending.take().expect("collect follows submit")`.

### `crates/engine/src/decider/python.rs`

```rust
//! The project's Python: the interpreter of the virtual environment, found relative to the
//! project root; the time limit; the command that runs a named decider; and the host a
//! setting asks for (one external process per distinct decider name).

/// Overrides the interpreter (a path).
pub const INTERPRETER_VAR: &str = "POLICYSPACE_PYTHON";
/// Overrides the time limit: seconds, `0` for none.
pub const TIME_LIMIT_VAR: &str = "POLICYSPACE_DECIDER_TIME_LIMIT";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PythonError {
    /// No interpreter at `path` (the virtual environment's, or the override's).
    InterpreterNotFound { path: PathBuf },
    /// `POLICYSPACE_DECIDER_TIME_LIMIT` isn't a non-negative number of seconds.
    BadTimeLimit { value: String },
}
// Display: `the project's Python environment wasn't found at {path}; create it with
// `python -m venv .venv` and `.venv/Scripts/python -m pip install -r requirements.txt` from
// the project root, or set POLICYSPACE_PYTHON to an interpreter`; `POLICYSPACE_DECIDER_TIME_LIMIT
// must be a number of seconds (0 for no limit), not '{value}'`.

/// Why a setting's host couldn't be built.
#[derive(Debug, Clone, PartialEq)]
pub enum HostError { Python(PythonError), Start { decider: String, error: StartError } }
// Display: the PythonError's; `decider '{decider}': {error}`.

/// The project root: the engine crate's manifest directory joined with `../..`.
pub fn project_root() -> PathBuf;
/// `project_root()/.venv/Scripts/python.exe` on Windows, `.venv/bin/python` elsewhere.
pub fn venv_interpreter() -> PathBuf;
/// The interpreter: the override if set, else the virtual environment's; must exist as a file.
pub fn interpreter() -> Result<PathBuf, PythonError>;          // interpreter_from(env::var_os(INTERPRETER_VAR))
pub fn interpreter_from(override_path: Option<&OsStr>) -> Result<PathBuf, PythonError>;
/// The time limit: 5 s unless overridden.
pub fn time_limit() -> Result<Option<Duration>, PythonError>; // time_limit_from(env::var(TIME_LIMIT_VAR).ok().as_deref())
pub fn time_limit_from(value: Option<&str>) -> Result<Option<Duration>, PythonError>;
/// `ProcessConfig { time_limit: time_limit()?, start_grace: default }`.
pub fn process_config() -> Result<ProcessConfig, PythonError>;
/// `interpreter()? -m policyspace {decider}`.
pub fn script_command(decider: &str) -> Result<ExternalCommand, PythonError>;
/// One `ExternalDecider` per distinct decider name among `setting.start.businesses`, in
/// name order, each started with `script_command(name)` and `process_config()`, in a
/// `DeciderHost::new()`. A setting with no businesses gives an empty host without touching
/// Python. The first start failure is returned (the deciders started before it are dropped).
pub fn host_for(setting: &LoadedSetting) -> Result<DeciderHost, HostError>;
```

`time_limit_from`: `None` → `Ok(Some(5 s))`; `Some(s)`: `s.trim().parse::<f64>()` finite and `>= 0` else `BadTimeLimit`; `0` → `Ok(None)`; else `Ok(Some(Duration::from_secs_f64(v)))`. `interpreter_from`: `Some(p)` → `PathBuf::from(p)`, else `venv_interpreter()`; `Err(InterpreterNotFound { path })` unless `path.is_file()`.

### `crates/engine/src/decider/mod.rs`

```rust
pub mod external;
pub mod host;
pub mod python;
pub mod replay;

pub use external::{ExternalCommand, ExternalDecider, ProcessConfig, StartError};
pub use host::{BusinessAnswer, ClosureDecider, Decider, DeciderFailure, DeciderHost, DecisionCounts, ReportedFailure, UnknownDecider};
pub use python::{HostError, PythonError};
pub use replay::{ReplayDecider, ReplayError};
```

### `crates/engine/src/lib.rs`

After the existing `decider` re-export line, add `pub use decider::{BusinessAnswer, Decider, DeciderFailure, DeciderHost, ReportedFailure, UnknownDecider};` (keep the existing one). The rest is reached as `policyspace_engine::decider::{...}`.

### `crates/engine/src/world.rs`

```rust
pub struct TurnReport {
    // ... existing fields unchanged, then:
    /// How the businesses answered at the decision point of `turn`.
    pub decisions: DecisionCounts,
    /// Every decider that failed at the decision point of `turn`, in decider name order.
    pub decider_failures: Vec<ReportedFailure>,
}

pub enum StepError {
    ClockAtMaximum,
    /// A business names a decider the host doesn't hold. The world is unchanged.
    UnknownDecider(UnknownDecider),
}
// Display for the new variant: the UnknownDecider's Display.

pub struct World { /* ... existing fields, then: */
    /// One per business, by `BusinessId`: its answer at the current turn's decision point.
    business_answers: IdVec<BusinessKind, BusinessAnswer>,
}

impl World {
    /// Builds a world from `setting` at turn 0 and runs turn 0's start, decision point,
    /// trade round and growth, with `host` answering for the businesses. Fails, building
    /// nothing, if a business names a decider the host doesn't hold.
    pub fn new(setting: LoadedSetting, host: &mut DeciderHost) -> Result<(World, TurnReport), UnknownDecider>;
    /// Ends the current turn, advances the clock, starts the next turn with `host`.
    /// At `Turn::MAX` returns `Err(StepError::ClockAtMaximum)`; if a business names a
    /// decider `host` doesn't hold, `Err(StepError::UnknownDecider)`. Either way nothing changes.
    pub fn step(&mut self, host: &mut DeciderHost) -> Result<TurnReport, StepError>;
    /// `business`'s answer at the current turn's decision point.
    /// # Panics
    /// Panics if `business` isn't from this world's businesses.
    pub fn business_answer(&self, business: BusinessId) -> &BusinessAnswer;
    fn run_turn(&mut self, perished: Vec<(GoodId, Quantity)>, host: &mut DeciderHost) -> TurnReport;
    /// Part 2 of the turn, before `trade`: the decision point. Stores every answer and
    /// returns the counts and the failures.
    fn decide(&mut self, host: &mut DeciderHost) -> (DecisionCounts, Vec<ReportedFailure>);
}
```

`new`: `host.check(&start.businesses)?` before anything is built; `business_answers` starts as `Nothing` for every business; then `run_turn(Vec::new(), host)`. `step`: the clock check, then `host.check(&self.businesses).map_err(StepError::UnknownDecider)?`, then perish, advance, `run_turn`. `run_turn`: `emit`, `endow`, `decide`, `trade`, `grow`. `decide`: `let point = host.decision_point(self); self.business_answers = IdVec::from_vec(point.answers)`; counts from the answers.

Module docs: the first bullet's list of what the world holds gains "each business's answer at the current turn's decision point"; the "business_view" bullet adds that the decision point serving it is `decide`; part 2 of the turn begins "the decision point, then the trade round ...: every business's view goes to the decider its setting entry names, through the host the caller passes to `new` and `step` (see the [`decider::host`] module), and its answer is stored; nothing resolves the answers yet (Step 08); then on every hex each POP submits ..."; the `World::new` bullet says it takes the host and fails for a decider name the host doesn't hold; the determinism bullet adds "deciders are taken in name order and businesses in ID order within a decider".

### `crates/dev-ui/src/session.rs`

```rust
/// How a session builds the decider host when it loads a setting.
pub enum Hosting {
    /// The idle host: every business does nothing. For tests and tools that don't care.
    Idle,
    /// One Python process per decider name the setting's businesses carry (`python::host_for`).
    FromSetting,
    /// A caller-supplied builder (tests; Stage 05's UI decider).
    Custom(Box<dyn Fn(&LoadedSetting) -> Result<DeciderHost, HostError>>),
}

pub struct Session {
    hosting: Hosting,
    folder: Option<PathBuf>,
    loaded: Option<Loaded>,
    /// The host for the loaded world; None whenever `loaded` isn't a world. Dropping it
    /// (on reload, open or a failed load) ends its processes.
    host: Option<DeciderHost>,
    log: Vec<String>,
    step_error: Option<String>,
    selected: Option<HexCoord>,
}
impl Session {
    /// No folder chosen; `hosting` says how later loads build their host.
    pub fn new(hosting: Hosting) -> Session;
    // open, reload, folder, loaded, world, log, step_error, select, selected: as before.
    /// As before; each step's log gains the turn line, then `text::failure_lines` for each failure.
    pub fn step(&mut self, times: u32) -> u32;
}
```

`load(folder)`: `self.host = None` first (ends the old processes). `load_setting` → `Err` → errors as before. `Ok(setting)` → the host from `self.hosting` (`Idle` → `DeciderHost::idle()`; `FromSetting` → `python::host_for(&setting)`; `Custom(f)` → `f(&setting)`): `Err(error)` → `self.loaded = Some(Loaded::Errors(text::load_failure_lines(&error.to_string(), folder)))`, log cleared. `Ok(mut host)` → `World::new(setting, &mut host)`: `Err(error)` → `Loaded::Errors(load_failure_lines(&error.to_string(), folder))` the same way; `Ok((world, report))` → log = turn line then failure lines; `self.host = Some(host)`; `self.loaded = Some(Loaded::World(world))`. `Session` no longer derives `Default` (`Hosting` has no default); `app.rs` calls `Session::new(Hosting::FromSetting)`. Only `FromSetting` is used outside tests, so `Idle` and `Custom` (and the `HostBuilder` type alias for `Custom`'s boxed closure, which clippy otherwise flags as too complex) carry `#[cfg(test)]` rather than a dead-code allowance; Stage 05 lifts the attribute when the UI becomes a decider.

### `crates/dev-ui/src/text.rs`

```rust
/// The lines logged for one decider failure at `turn`: `Turn {turn}: decider '{name}' failed:
/// {failure}` (the DeciderFailure's one-line Display), then each stderr tail line as
/// `    stderr: {line}`.
pub fn failure_lines(turn: Turn, failure: &ReportedFailure) -> Vec<String>;
/// The lines shown when a setting loads but its deciders can't start (or a name can't be
/// resolved): `message`'s lines, then `1 error(s) found in {folder.display()}` so the status
/// bar's count and the error view read as for a broken setting.
pub fn load_failure_lines(message: &str, folder: &Path) -> Vec<String>;
```

The session uses `load_failure_lines` for both `HostError` and `UnknownDecider`.

### `crates/dev-ui/src/check.rs`

`run_turns(setting, turns)`: `let mut host = match python::host_for(&setting) { Ok(h) => h, Err(e) => { for line in load_failure_lines(&e.to_string(), path) { eprintln!("{line}") } return ExitCode::from(1) } }`; `World::new(setting, &mut host)` likewise on `Err`; after turn 0's line and after each step's line, `println!` every `failure_lines` line. `run` passes `path` through for the message. Exit codes unchanged: 0 success, 1 errors.

## Tasks

1. `decider/host.rs`: types, `DeciderHost`, `check`, `decision_point`, `ClosureDecider`, Display impls, unit tests (Test Plan). `mod.rs` and `lib.rs` re-exports and docs.
2. `world.rs`: the signatures, `business_answers`, `decide`, the report fields, `StepError::UnknownDecider`, module docs, the unit tests. `trade.rs` unit tests. Build the engine crate alone (`cargo build -p policyspace-engine`).
3. `tests/common/mod.rs` (`idle`, `world_of`) and every engine integration test's `World::new`/`step` call. `cargo test -p policyspace-engine` must pass with every existing expectation unchanged.
4. `decider/replay.rs` with unit tests; `decider/external.rs` with the Python-free unit tests; `decider/python.rs` with the `_from` unit tests.
5. `crates/engine/tests/decider_host.rs` (Test Plan).
6. `crates/engine/tests/external_decider.rs` (Test Plan). Run `.venv/Scripts/python -m pip install -r requirements.txt` first if `import policyspace` fails.
7. Dev UI: `text.rs` (`failure_lines`, `load_failure_lines`, test worlds), `session.rs` (`Hosting`, the host, the log; tests), `check.rs`, `app.rs`, `test_support.rs`, `main.rs` docs, `tests/checker.rs`.
8. Run `.venv/Scripts/python tools/check.py` until it passes. Run `cargo run -p policyspace-dev-ui -- check settings/sample --turns 2` and confirm the output is unchanged from before this Step (the arithmetic decider runs and nothing is printed for it). Run `cargo run -p policyspace-dev-ui -- check crates/engine/tests/settings/production_valid --turns 1` and confirm it exits 1 with the `unknown decider 'rule'` text on stderr.

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py`. The tests in `external_decider.rs`, the session tests marked (Python) and the checker tests on the sample with `--turns` need the virtual environment; when it is missing they fail with `PythonError::InterpreterNotFound`'s message (through `expect`/`unwrap_or_else(|e| panic!("{e}"))`), never skip. Test names are suggestions. Every test that needs a time limit shorter than the default sets it through `ProcessConfig`, never through the environment.

### Unit tests

**`decider/host.rs`** — helpers: `request(turn, names)` builds a request of minimal views (reuse `view::test_view()` with the business name and turn replaced); a `recording` decider through `ClosureDecider`.

- `the_idle_host_holds_every_name_and_the_empty_host_none`: `DeciderHost::idle().holds("x")`; `!DeciderHost::new().holds("x")`; after `add("x", ..)` both hold it; `names()` lists added names in order.
- `adding_a_name_twice_panics` (`should_panic(expected = "already in the host")`).
- `check_names_the_first_business_with_an_unknown_decider`: a hand-built `Businesses` (as `businesses.rs`'s tests do: `a` and `c` name `rule`, `b` names `other`) against a host holding only `rule` → `Err(UnknownDecider { business: "b", decider: "other", known: ["rule"] })`; its Display; the idle host → `Ok`; the empty host → `Err` for `a`, Display ends `(it holds no deciders)`.
- `failure_and_report_displays`: the Display text of every `DeciderFailure` variant except `Exited` (`WontStart` wrapping an `ExitedAtOnce` with a two-line tail displays as one line and `stderr_tail()` returns the two lines) (covered by the integration tests, since it needs a real `ExitStatus`); `ReportedFailure` with a two-line tail prints the `its error output ends with:` line and the two indented lines; `DecisionCounts::default()` is zeros.
- `a_closure_decider_answers_its_request`: `ClosureDecider::new(|r| Reply { turn: r.turn, decisions: vec![] })`; `submit` then `collect` gives that reply.

**`decider/replay.rs`** — build two recordings in memory (turns 2 and 3, one view each).

- `replay_answers_the_recorded_turn`: submit the recorded request for turn 2 → collect gives the recorded reply.
- `replay_fails_for_a_missing_turn_and_a_different_request`: turn 4 → `NoRecording { turn: 4 }`; turn 2 with a view whose cash differs → `RequestDiffers { turn: 2 }`.
- `two_recordings_for_one_turn_are_refused`: `new` → `DuplicateTurn`.
- `from_files_reports_a_missing_and_a_malformed_file`: a nonexistent path → `Read`; a `tempfile` holding `{` → `Format`; the Displays name the path.

**`decider/external.rs`**

- `a_command_displays_as_its_words`: `ExternalCommand::new("python").arg("-m").args(["policyspace", "arithmetic"])` displays `python -m policyspace arithmetic`.
- `the_default_config`: 5 s and 500 ms.
- `a_program_that_doesnt_exist_fails_to_start`: `ExternalDecider::start(ExternalCommand::new("policyspace-no-such-program-7f3a"), ProcessConfig::default())` → `Err(StartError::Spawn { command, .. })` with `command == "policyspace-no-such-program-7f3a"`; Display starts with `'policyspace-no-such-program-7f3a' couldn't be started: `.

**`decider/python.rs`**

- `the_interpreter_is_the_venvs_unless_overridden`: `venv_interpreter()` ends with `.venv/Scripts/python.exe` (Windows) or `.venv/bin/python`; `interpreter_from(Some("no/such/python"))` → `Err(InterpreterNotFound { path })` whose Display contains `no/such/python`, `python -m venv .venv` and `POLICYSPACE_PYTHON`; `interpreter_from(Some(<path of the current test executable>))` → `Ok` (any existing file passes).
- `the_time_limit_parses`: `None` → 5 s; `"0"` → `None`; `"2.5"` → 2.5 s; `" 10 "` → 10 s; `"abc"`, `"-1"`, `"inf"`, `""` → `BadTimeLimit` whose Display names the value.

### `crates/engine/tests/decider_host.rs` (`mod common;`, no Python)

Helpers: `empty_reply(turn)`; `decision_for(name, factories...)`; `answers_of(&World) -> Vec<BusinessAnswer>` by business ID order; `closure(f) -> ClosureDecider`.

- **AC 2.** Covered by every existing test passing with `common::idle()` (no new test).
- **AC 3.** `an_unknown_decider_name_fails_world_creation`: `businesses_valid` with `DeciderHost::new().with("rule", closure(empty_reply))` → `World::new` is `Err(UnknownDecider { business: "idle", decider: "other_rule", known: ["rule"] })`; with the idle host → `Ok`, every answer `Nothing`, report `decisions == { nothing: 3 }`, no failures. `a_rebuilt_host_missing_a_name_refuses_the_step`: build with a host holding `rule` and `other_rule`, then `step` with one holding only `rule` → `Err(StepError::UnknownDecider(..))`, `world.turn()` still 0, answers unchanged; `step` with the full host → `Ok`.
- **AC 4 (scoping and order).** `each_decider_receives_only_its_businesses_in_id_order_and_an_empty_request_otherwise`: `businesses_valid`; `rule` closure records every request it sees into an `Rc<RefCell<Vec<Request>>>` and answers empty; `other_rule` likewise; a third decider `spare` added to the host (no business names it). After `World::new`: `rule` saw one request at turn 0 with views `[farm_a, farm_b]` (names in that order), `other_rule` saw `[idle]`, `spare` saw one request with no views; after `step`, each saw a second request at turn 1. The report's `decisions == { nothing: 3 }` and no failures.
- **AC 4 (a refused decision).** `a_refused_decision_idles_only_its_business`: `rule` answers `[decision_for("farm_a", factories {9: (0, 0)}), Decision::empty("farm_b")]`; `other_rule` answers `[Decision::empty("idle")]`. Answers: `farm_a` → `Refused { decision: that one, errors: one line "factory #9: not in this business's view" }`, `farm_b` → `Valid(Decision::empty("farm_b"))`, `idle` → `Valid(..)`. Report: `{ valid: 2, refused: 1, nothing: 0 }`, no failures.
- **AC 4 (a refused reply).** `a_reply_refused_as_a_whole_idles_every_business_of_that_decider`: three cases, each a fresh world: `rule` answers turn 1 at turn 0; `rule` names `nobody`; `rule` names `farm_a` twice. Each time: `farm_a` and `farm_b` are `Nothing`, `idle` (whose decider answers properly) is `Valid`; the report has exactly one failure `ReportedFailure { decider: "rule", failure: Refused(errors) }` with the expected `ReplyError` (`TurnMismatch { request: 0, reply: 1 }`, `UnknownBusiness { nobody }`, `DuplicateBusiness { farm_a }`); `decisions == { valid: 1, nothing: 2 }`; the failure's Display starts with `decider 'rule': the reply was refused: `. `a_failed_closure_decider_is_asked_again_next_turn`: the wrong-turn decider is asked again at turn 1 (its request count is 2 after one step) and fails again.
- **AC 4 (nothing).** `an_empty_reply_means_every_business_does_nothing`: `rule` and `other_rule` answer empty replies → all `Nothing`, `{ nothing: 3 }`, no failures.
- **AC 1 (replay).** `the_sample_replays_its_recording_at_turn_0`: `ReplayDecider::from_files([recordings dir / "sample_turn0.recording.json"])` under `arithmetic`; `World::new` from the sample → each center business's answer is `Valid(d)` where `d` equals the recording's decision for that business; report `{ valid: 3 }`, no failures. **AC 6.** Then `step` → every answer `Nothing`, one failure `{ decider: "arithmetic", failure: NoRecording { turn: 1 } }`, `{ nothing: 3 }`. `a_replay_of_a_different_request_fails`: read the sample recording, double `center_farm`'s `cash` in its request, build a `ReplayDecider::new([edited])` and the sample world under it → turn 0 answers all `Nothing` and one failure `RequestDiffers { turn: 0 }`.
- **AC 1 (storage).** `answers_are_replaced_every_turn`: `rule` answers `Valid` for `farm_a` at even turns and empty at odd turns; after `new` `farm_a` is `Valid`, after one step `Nothing`, after another `Valid`.

### `crates/engine/tests/external_decider.rs` (`mod common;`, needs Python)

Helpers: `python() -> PathBuf` (`python::interpreter().unwrap_or_else(|e| panic!("{e}"))`); `script(code: &str, args: &[&str]) -> ExternalCommand` (`python() -c code args...`); `config(limit_ms)`; `sample_world(host) -> (World, TurnReport)`.

- **AC 1 (the real decider).** `the_sample_runs_under_the_arithmetic_decider`: `python::host_for(&load(sample))` → `Ok`; `World::new` → each center business's answer is `Valid(d)` with `d` equal to the recording's decision for that business (read `sample_turn0.recording.json`); report `{ valid: 3 }`, no failures. Step 5 times: each report `{ valid: 3 }`, no failures, every answer `Valid`.
- **AC 5 (exits before answering).** `a_process_that_exits_before_answering_idles_its_businesses_and_is_restarted`: a temp file holding `0`; `script("import sys,pathlib; p=pathlib.Path(sys.argv[1]); p.write_text(str(int(p.read_text())+1)); sys.stdin.readline(); print('boom', file=sys.stderr); sys.exit(3)", [path])` as `arithmetic`; `World::new` from the sample: answers all `Nothing`, one failure `Exited { status, stderr: ["boom"] }` with `status.code() == Some(3)`, Display contains `the process exited`; the file reads `1`. `step` → the same failure again and the file reads `2` (started again). `{ nothing: 3 }` both times.
- **AC 5 (late).** `a_process_that_answers_late_times_out`: `script("import sys,time; sys.stdin.readline(); time.sleep(30)")` with `config(300)` → failure `TimedOut { limit: 300 ms, .. }`; the whole `World::new` takes under 5 s; `step` → `TimedOut` again (so the process was killed and restarted).
- **AC 5 (one reply behind).** `a_process_whose_reply_is_refused_is_restarted`: a script that, after each request line, prints two reply lines for that turn (`{"turn": N, "decisions": []}` twice, parsing the turn from the request with `json`), under `arithmetic` on the sample: turn 0 is answered (all `Nothing`, no failure, `{ nothing: 3 }`); `step` → the stale second line answers turn 0 against turn 1, so one failure `Refused` with `TurnMismatch { request: 1, reply: 0 }`; `step` again → the fresh process answers turn 2 correctly: no failure. A start counter file (as in the exits-before-answering test) reads `2` at the end.
- **AC 5 (garbage).** `a_reply_that_isnt_a_reply_is_a_failure`: `script("import sys; sys.stdin.readline(); print('not json', flush=True); sys.stdin.readline()")` → `Unreadable { error: FormatError::Json(_), .. }`, Display starts `the reply couldn't be read: `.
- **AC 5 (won't start).** `a_script_that_exits_at_once_fails_host_construction`: `ExternalDecider::start(script("import sys; sys.exit(2)"), default)` → `Err(StartError::ExitedAtOnce { status, .. })` with code 2. `an_unknown_decider_name_is_a_host_error`: `python::host_for(&load(production_valid))` → `Err(HostError::Start { decider: "rule", error: ExitedAtOnce { status, stderr } })` with code 2 and a stderr line containing `unknown decider 'rule'`; the Display contains both.
- **AC 7 (dropping).** `dropping_a_host_ends_its_processes`: a heartbeat script that appends one `.` to the file at `sys.argv[1]` every 50 ms, forever (append, never rewrite: a rewrite truncates the file first, so a read can see it empty), started into a host; wait 300 ms, the file grows between two reads 200 ms apart; `drop(host)`; wait 300 ms; two reads 300 ms apart are equal.
- **AC 8.** The `expect` in `python()`: no separate test; `python.rs`'s unit test checks the message.

### Dev UI

**`session.rs`** — existing tests: `Session::new(Hosting::Idle)`; expectations unchanged. New:

- `a_closure_decider_failure_is_logged_each_turn`: `Hosting::Custom(Box::new(|_| Ok(DeciderHost::new().with("rule", ClosureDecider::new(|r| Reply { turn: r.turn.next().unwrap(), decisions: vec![] })).with("other_rule", ClosureDecider::new(|r| Reply { turn: r.turn, decisions: vec![] })))))` on `businesses_valid`: the log is the turn-0 line then `Turn 0: decider 'rule' failed: the reply was refused: the reply answers turn 1, but the request is for turn 0`; after `step(1)` two more lines, the last being the turn-1 failure line.
- `a_custom_host_error_is_a_load_error`: `Hosting::Custom(Box::new(|_| Err(HostError::Python(PythonError::BadTimeLimit { value: "x".into() }))))` → `Loaded::Errors(lines)` with `lines[0]` the error's Display and `lines[1]` `1 error(s) found in {folder}`; `world()` is `None`; the log is empty.
- `the_sample_loads_and_steps_under_its_python_deciders` (Python): `Hosting::FromSetting` on the sample: if `loaded()` is `Loaded::Errors(lines)`, panic with those lines (so a missing Python environment fails with its own message); `log()[0]` is the known turn-0 line; `step(2)` → the log has exactly 3 lines, all starting `Turn ` and none containing `failed`.
- `a_decider_that_wont_start_is_a_load_error` (Python): `Hosting::FromSetting` on `production_valid` → `Loaded::Errors`, a line containing `decider 'rule'` and one containing `unknown decider 'rule'`; `step(1)` returns 0.

**`text.rs`** — `failure_lines_and_load_failure_lines`: a `ReportedFailure` with `TimedOut { 2.5 s, ["a", "b"] }` at turn 7 gives `["Turn 7: decider 'x' failed: no reply within the time limit of 2.5 s", "    stderr: a", "    stderr: b"]`; `load_failure_lines("one\ntwo", folder)` gives `["one", "two", "1 error(s) found in <folder>"]`.

**`tests/checker.rs`** — `a_decider_that_wont_start_prints_an_error_and_exits_1` (Python): `check production_valid --turns 1` → exit 1, stdout holds only the `Loaded setting` line, stderr contains `unknown decider 'rule'`. `turns_without_businesses_start_no_python`: `check pops_placed --turns 1` passes even with `POLICYSPACE_PYTHON=no/such/python` in the child's environment (set it on the `Command`, not the test process) and prints no failure lines. The existing sample `--turns` tests (Python) must pass unchanged, with the child's stderr added to their exit-code assertion messages so a missing Python environment fails with its message rather than a bare exit code.

## Constraints

- Follow the existing conventions: doc comments on every public item and field; `cargo fmt`; clippy with `-D warnings`; `BTreeMap`/`BTreeSet`, never `HashMap`; `#[cfg(test)]` rather than `#[allow(dead_code)]`.
- No new dependencies in either crate: `std::process`, `std::thread`, `std::sync::mpsc` only. No thread anywhere except the two pumps per process.
- Tests never set or remove environment variables in the test process (use the `_from` forms, or set variables on a child `Command`). Tests that spawn Python use `python::interpreter()` and fail with its message when it is missing.
- Don't change the protocol types or their JSON, `Reply::validate`, `Answer`, the recordings, the Python package, any setting, `tools/check.py`, `docs/`, or `manual.ipynb` (documentation is drafted later in the loop).
- The turn line (`text::turn_line`) and every existing checker and session expectation stay byte-for-byte the same.
- Out of scope: resolving decisions, trading, running factories, the cash cap (Step 08); showing answers and traces beyond the log lines (Step 09); a ready handshake or any protocol change; deciders in other languages beyond what `ExternalCommand` allows; businesses entering or leaving; halting on a failure or a strict mode.

## Notes from the build

Recorded after Code Review, for Step 08, Step 09 and the docs:

- The fix round added `Decider::reject` (a whole-reply refusal restarts an external process, as the design asks), `StartError::summary`/`stderr_tail` (every failure logs as one line plus `stderr:` lines), and better failure messages in the dev UI tests when the Python environment is missing.
- **Known limit:** the request write in `ExternalDecider::submit` has no time limit; a process that stops reading its input blocks the turn once a request exceeds the pipe buffer (about 64 KB, roughly 100 businesses on one decider). A later Step can add a writer thread.
- Joining the pump threads after a kill assumes nothing else holds the process's pipes; a decider script that spawns its own child inheriting them, and that outlives it, would delay the join. On Windows the venv's `python.exe` launcher and its real interpreter die together (the timeout and drop tests prove it).
- `StartError::ExitedAtOnce` shows the interpreter path as built from the engine crate's manifest directory (`...\crates\engine\..\..\.venv\Scripts\python.exe`), not canonicalised.
- With the Python environment missing, `cargo test -p policyspace-dev-ui` stops at the unit-test target; `--no-fail-fast` reaches the checker tests. The engine's `external_decider` tests fail with the "Python environment wasn't found" message.
- `Hosting::Idle` and `Hosting::Custom` are `#[cfg(test)]` until Stage 05 makes the UI a decider.
- `cargo test --workspace` and the sample's `check --turns` now start Python, so the virtual environment (with `pip install -r requirements.txt`) is required for the Rust tests too.

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes with four checks, every test in the Test Plan is written and passes, the sample's checker output for `--turns 2` is unchanged, and every Acceptance Criterion in `design.md` is met.
