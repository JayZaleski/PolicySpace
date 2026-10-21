# The decider host and the decision point

Source: `crates/engine/src/decider/` — `host.rs` (the `Decider` trait, `DeciderHost`, the decision point, `BusinessAnswer`, `DeciderFailure`, `ReportedFailure`, `DecisionCounts`, `UnknownDecider`, `ClosureDecider`), `external.rs` (the external-process decider), `replay.rs` (the replay decider) and `python.rs` (finding the project's Python and building the host a setting asks for). The World side is `World::new`, `World::step`, `World::decide` and `World::business_answer` in `world.rs`. The crate root re-exports `BusinessAnswer`, `Decider`, `DeciderFailure`, `DeciderHost`, `ReportedFailure` and `UnknownDecider`; everything else is under `policyspace_engine::decider` (`ClosureDecider`, `DecisionCounts`, `ExternalCommand`, `ExternalDecider`, `ProcessConfig`, `StartError`, `ReplayDecider`, `ReplayError`, `HostError`, `PythonError`), and the submodules are public as `decider::host`, `decider::external`, `decider::replay` and `decider::python`. The messages themselves are described in [decider.md](decider.md).

Once per turn the World stops at its **decision point**: every business's view (see [decider.md](decider.md#the-view)) goes to the decider its setting entry names, and every decision comes back, one batched exchange per decider. The deciders live in a **host** that the caller builds and passes to the World, so the engine owns no processes and keeps nothing of a decider's between turns. The World stores each business's answer, then resolves each valid decision in the trade round and in factory operation (see [trade.md](trade.md)); a business whose answer is nothing or refused trades nothing and its factories only wear.

## Concepts

- **The host is outside the World.** A `DeciderHost` is a table of deciders by name, the names businesses carry in their `decider` key (see [businesses.md](businesses.md)). The dev UI, the checker and the tests build it, never the World. `World::new(setting, &mut host)` and `World::step(&mut host)` take it, and the World reads from it only inside the decision point. There is no host-less form of either call, so a caller can't forget it. Because the World holds no decider between turns, a host (and its processes) can be dropped and rebuilt without the simulation losing anything.
- **Names are resolved against the host**, at world creation and again at the top of every step (a rebuilt host may lack a name). A business naming a decider the host doesn't hold is an authoring mistake, not a turn-time event, so it is an error: `UnknownDecider` from `World::new`, `StepError::UnknownDecider` from `World::step`, and in both cases nothing is built or changed.
- **One request per decider per turn.** All the businesses that name one decider share it: it receives one `Request` holding only their views, in business ID order. Every decider in the host receives exactly one request every turn, an empty one when it controls no business, so a decider can count turns from its input.
- **The turn never halts.** By the decision point, emission and endowment have already happened, so a halt would leave the turn half done. A decision refused by validation idles its business; a decider that fails idles every business it controls and is reported in the turn report; the turn goes on either way.
- **Submit everything, then collect.** The `Decider` trait has two phases. The host submits every decider's request before collecting any reply, so several external processes work on their requests at the same time without the engine using threads for the simulation.
- **The idle host** holds every name and answers nothing: every business decides nothing every turn, so no business trades or runs a factory, but every factory still wears one slice a turn. It is for callers that don't care about business behavior; every engine test that isn't about deciders uses it. Their POP numbers don't depend on it; factory conditions do (a fresh world's factories have already worn one slice).

## The `Decider` trait

```rust
pub trait Decider {
    fn submit(&mut self, request: &Request) -> Result<(), DeciderFailure>;
    fn collect(&mut self) -> Result<Reply, DeciderFailure>;
    fn reject(&mut self) {}
}
```

- `submit` takes the turn's request. An external process writes it to the process; the replay and closure deciders compute their reply here and keep it. An `Err` means the decider can't answer this turn.
- `collect` returns the reply to the request last submitted. The host calls it exactly once after every successful `submit`, and never otherwise.
- `reject` tells the decider that the reply it just returned was refused as a whole (wrong turn, unknown or duplicate business). The default does nothing. The external process stops itself, so that a process that has fallen one reply behind is started afresh next turn instead of answering every later turn with a stale line.

Three implementations exist: `ExternalDecider`, `ReplayDecider` and `ClosureDecider`. Stage 05 adds the dev UI itself as a decider through the same trait.

## `DeciderHost`

| Method | Meaning |
|---|---|
| `DeciderHost::new()` (also `Default`) | an empty host that accepts only the names added to it |
| `DeciderHost::idle()` | the idle host: every name is accepted, and a name with no decider gets nothing |
| `add(name, decider) -> &mut DeciderHost` | adds a decider under `name`; panics with `a decider named '{name}' is already in the host` if the name is taken |
| `with(name, decider) -> DeciderHost` | `add` in builder form |
| `holds(name)` | true if `name` was added, or the host accepts every name |
| `accepts_unknown()` | true for the idle host |
| `names()` | the names added, in name order |

Deciders are kept in a `BTreeMap<String, Box<dyn Decider>>`, so they are always taken in name order. `Debug` prints the names and `accepts_unknown`, not the deciders. An idle host can also hold deciders (`DeciderHost::idle().with(..)`): the names it holds get their decider, every other name gets nothing.

Two crate-private methods serve the World:

- `check(&Businesses) -> Result<(), UnknownDecider>` returns the first business in ID order whose decider the host doesn't hold. `UnknownDecider { business, decider, known }` displays as `business 'idle' names decider 'other_rule', which the host doesn't hold (it holds: rule)`, or `... (it holds no deciders)` for an empty host.
- `decision_point(&World) -> DecisionPoint` runs the exchange (below) and returns one `BusinessAnswer` per business, by business index, and the failures in decider name order.

## The decision point

`World::run_turn` calls `emit`, `endow`, `decide`, `trade` and `grow` in that order, so the decision point is the start of part 2 of the turn ("actors act"), after emission and endowment and before the trade round (see [world.md](world.md#the-turn)). The view therefore shows the stockpile as [decider.md](decider.md#concepts) describes it. `World::decide` calls `host.decision_point(self)`, stores the answers, counts them, and returns the counts and failures for the report.

`decision_point`, step by step:

1. Every business starts at `BusinessAnswer::Nothing`.
2. One pass over `businesses.iter()` groups the business IDs by decider name in a `BTreeMap`, each list in business ID order.
3. For each decider in name order: take its list from the map (possibly none); build `Request { turn: world.turn(), views }` with one `World::business_view` per business; call `submit`. Keep the name, the IDs, the request and the submit result.
4. For each kept exchange in the same order: if `submit` succeeded, call `collect`. Validate the reply with `Reply::validate(&request, goods)` (see [decider.md](decider.md#the-batch)).
   - A reply refused as a whole: call `reject` on the decider and record `DeciderFailure::Refused(errors)`.
   - Otherwise each business gets its answer: `Answer::Nothing` becomes `Nothing`, `Answer::Valid(d)` becomes `Valid(d.clone())`, and `Answer::Refused { decision, errors }` becomes `Refused { decision: decision.clone(), errors }`; the refused decision comes with the answer, so nothing searches the reply for it.
5. Any failure, from `submit`, `collect` or validation, is pushed as a `ReportedFailure { decider: name, failure }`, and that decider's businesses stay `Nothing`.

A business whose decider name the host accepts but has no decider for (only possible with the idle host) stays `Nothing` and is in no request.

## Answers and the report

```rust
pub enum BusinessAnswer {
    Nothing,                                              // none sent, or its decider failed
    Valid(Decision),                                      // passed validation against its view
    Refused { decision: Decision, errors: DecisionErrors }, // failed validation; the business does nothing
}
pub struct DecisionCounts { pub valid: usize, pub refused: usize, pub nothing: usize }
pub struct ReportedFailure { pub decider: String, pub failure: DeciderFailure }
```

- The World keeps one `BusinessAnswer` per business in `business_answers: IdVec<BusinessKind, BusinessAnswer>`, `Nothing` for every business at creation and replaced at every decision point. `World::business_answer(business)` reads it (and panics for a business from another world). Answers are owned: `Reply::validate`'s `Answer<'a>` borrows the reply, which doesn't outlive the decision point.
- A refused decision is kept with its errors, so a developer can see what was sent and why it was refused: the dev UI shows the errors, the offers and bids asked for, and the trace (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)). It counts as deciding nothing.
- `TurnReport` has `decisions: DecisionCounts` (how many businesses sent a valid decision, were refused, and sent nothing; they add up to the number of businesses) and `decider_failures: Vec<ReportedFailure>` (every decider that failed this turn, in name order). Tests of the turn assert that `decider_failures` is empty.
- `ReportedFailure` displays as `decider '{decider}': {failure}`, then, when the failure has an error-output tail, a line `its error output ends with:` and each tail line indented by two spaces.

## Failures

A failed decider idles every business it controls for the turn. `DeciderFailure` has one variant per cause; its `Display` is always one line, and `stderr_tail()` returns the last lines of the process's error output for the variants that have them (empty otherwise).

| Variant | Cause | Display |
|---|---|---|
| `Exited { status, stderr }` | the process exited, or closed its pipes, before answering | `the process exited ({status}) before answering` |
| `TimedOut { limit, stderr }` | no reply within the time limit; the process was killed | `no reply within the time limit of 2.5 s` |
| `Unreadable { error, stderr }` | the reply line isn't a `Reply` (a `FormatError`); the process was killed | `the reply couldn't be read: {error}` (a `FormatError::Values` joined with `; `) |
| `WontStart { error }` | the process had failed earlier and couldn't be started again | `the process couldn't be started again: {StartError::summary()}` |
| `Refused(ReplyErrors)` | the reply was refused as a whole | `the reply was refused: {e1}; {e2}` |
| `NoRecording { turn }` | the replay decider has no recording for this turn | `no recording for turn {turn}` |
| `RequestDiffers { turn }` | the replay decider's recording for this turn holds a different request | `the request for turn {turn} differs from the recorded one` |

What happens next turn: an external process that failed has been killed and is started again at its next `submit`; if that fails, the next turn reports `WontStart`. A replay or closure decider is simply asked again. Every failure is reported again each turn it happens.

## The external-process decider

`ExternalDecider` runs a program for the life of the host and exchanges one line per turn with it: the compact request (`Request::to_json`) and `\n` on its standard input, one reply line from its standard output. That is exactly what `python -m policyspace <decider>` speaks (see [python.md](python.md#the-loop-and-the-command-line-deciderspy-__main__py)), but any program that speaks the protocol on its standard streams works.

```rust
pub struct ExternalCommand { pub program: OsString, pub args: Vec<OsString> }   // new, arg, args; Display: words joined by spaces
pub struct ProcessConfig { pub time_limit: Option<Duration>, pub start_grace: Duration }
// Default: time_limit Some(5 s) (DEFAULT_TIME_LIMIT), start_grace 500 ms (DEFAULT_START_GRACE)
pub enum StartError {
    Spawn { command: String, error: String },                         // couldn't be spawned
    ExitedAtOnce { command: String, status: ExitStatus, stderr: Vec<String> }, // exited in the grace window
}
pub const STDERR_TAIL_LINES: usize = 20;
```

`ProcessConfig` is host configuration, about the machine running the simulation, not about the world, so it is never setting data. `time_limit: None` waits forever.

**Starting.** `ExternalDecider::start(command, config) -> Result<ExternalDecider, StartError>` spawns the program with all three standard streams piped, takes the pipes, and starts two **pump threads**: one reads standard output line by line into an `mpsc` channel, the other reads standard error and keeps its last `STDERR_TAIL_LINES` lines (lossy UTF-8, line endings stripped) in a shared `VecDeque`. Then it watches the process for `start_grace`, polling `try_wait` every 10 ms: a process that exits in that window is `ExitedAtOnce` with its status and error tail. A spawn error (program not found, not executable) is `Spawn`. `StartError::summary()` is the one-line form (`'{command}' couldn't be started: {error}`, or `'{command}' exited ({status}) as soon as it started`); `Display` adds `its error output ends with:` and the indented tail. A process that exits after the grace window is found at the next exchange instead, as `Exited`. `command()`, `config()` and `is_running()` read the decider's state.

**The exchange.** `submit` restarts the process if a failure stopped it (a start failure becomes `WontStart`), then writes and flushes the request line; a write error means the process is gone (`Exited`). `collect` waits on the channel with `recv_timeout(time_limit)` (or `recv` with no limit): a line is parsed with `Reply::from_json` (`Unreadable` if it fails); a closed channel or a read error is `Exited`; a timeout is `TimedOut`. Validating the reply is the host's job, not the decider's.

**Stopping.** Every failure path stops the process: kill it (harmless if it has already exited), wait for its status, join both pumps (they end at end of file once the process is gone), and snapshot the error tail into the failure. Afterwards no process is held (`is_running()` is false) until the next `submit` starts one, so a stale reply can never arrive in a later turn. `reject` stops it the same way. Dropping an `ExternalDecider` kills and waits for its process, so dropping a host ends all its processes.

**Why threads.** The pumps are the only threads in the engine. Standard Rust can't wait on a pipe with a time limit, and a process that writes a lot of error output would block on a full pipe if nothing read it. They do no simulation work, hold no world state and end with the process.

**Known limits.**

- The request write in `submit` has no time limit. A process that has stopped reading its input blocks the turn once a request is larger than the pipe buffer (about 64 KB on Windows, roughly 100 businesses on one decider at the sample's size of view). Today's settings are far below that; a writer thread fed by a channel would remove the limit.
- Joining the pumps after a kill assumes nothing else holds the process's pipes. A decider that starts a child process of its own which inherits the pipes and outlives it would delay the join. On Windows the virtual environment's `python.exe` launcher and the real interpreter end together, so the project's Python is not affected.

## The replay decider

`ReplayDecider` answers from recordings (see [decider.md](decider.md#recordings)), one per turn, so engine tests that don't test strategy need no Python.

- `ReplayDecider::new(recordings)` keys them by their request's turn; two for one turn is `ReplayError::DuplicateTurn`. `ReplayDecider::from_files(paths)` reads one recording per file with `Recording::from_json` and calls `new`; `ReplayError::Read` and `ReplayError::Format` name the path. `turns()` lists the recorded turns, ascending.
- `submit` looks up the recording for the request's turn and checks that **the engine's request equals the recorded one**. A missing turn is `NoRecording`, a different request `RequestDiffers`; otherwise the recorded reply is returned by `collect`. The failure is kept until `collect`, so a replay failure is reported like a process failure.

Failing rather than answering nothing keeps a drifted or missing recording from passing silently. A test that wants businesses idle after the recorded turns uses the idle host instead. Comparing against the engine-built request is also what makes it safe to read a request from a file: a recorded request is never trusted on its own.

## The closure decider

`ClosureDecider::new(|request| reply)` answers each request with a Rust function. Tests use it to send back a wrong turn, an unknown or duplicate business, a refused decision, or nothing, and to record the requests they receive, without a process or a file.

## Finding Python: `decider::python`

The module that ties the external-process decider to the project's Python, used by the dev UI, the checker and the engine's own tests, so they all start the same interpreter:

| Item | Meaning |
|---|---|
| `project_root()` | the engine crate's manifest directory joined with `../..` |
| `venv_interpreter()` | `.venv/Scripts/python.exe` under the project root on Windows, `.venv/bin/python` elsewhere |
| `interpreter()` | the interpreter: the `POLICYSPACE_PYTHON` override if set, else the virtual environment's; `PythonError::InterpreterNotFound` unless it is a file |
| `time_limit()` | 5 s, or `POLICYSPACE_DECIDER_TIME_LIMIT` in seconds (`0` for no limit); anything that isn't a finite number of at least 0 is `PythonError::BadTimeLimit` |
| `process_config()` | `ProcessConfig { time_limit: time_limit()?, start_grace: 500 ms }` |
| `script_command(name)` | `{interpreter} -m policyspace {name}` |
| `host_for(setting)` | the host a setting asks for (below) |

`INTERPRETER_VAR` and `TIME_LIMIT_VAR` name the two variables. They let a developer point at another environment, or lift the time limit to step through a decider script in a debugger. `interpreter_from(Option<&OsStr>)` and `time_limit_from(Option<&str>)` take the value instead of reading the environment, so tests never have to set a process-wide variable (tests run in parallel in one process).

`PythonError::InterpreterNotFound` displays as `the project's Python environment wasn't found at {path}; create it with `python -m venv .venv` and `.venv/Scripts/python -m pip install -r requirements.txt` from the project root, or set POLICYSPACE_PYTHON to an interpreter`.

`host_for(setting) -> Result<DeciderHost, HostError>` starts one `ExternalDecider` per distinct decider name among the setting's businesses, in name order, each with `script_command(name)` and `process_config()`, in a `DeciderHost::new()`. The setting alone decides which scripts run. A setting with no businesses gives an empty host without touching Python, so settings without businesses need no Python environment. The first failure is returned and the deciders already started are dropped (their processes end): `HostError::Python(PythonError)`, or `HostError::Start { decider, error: StartError }`, displayed as `decider '{decider}': {error}`. A name the Python package doesn't know makes `python -m policyspace` exit with code 2 before reading anything, which shows up here as `ExitedAtOnce` with `error: unknown decider 'rule' (known: arithmetic)` in its tail. `StartError` shows the interpreter's path as built from the manifest directory (`...\crates\engine\..\..\.venv\Scripts\python.exe`), not canonicalised.

## Who builds which host

| Caller | Host |
|---|---|
| Engine tests not about deciders, `tests/common`'s `world_of`, the dev UI's `test_support::world_at`, `text` tests | `DeciderHost::idle()` (`common::idle()` in the integration tests) |
| The dev UI window | `python::host_for` for each loaded setting (`Hosting::FromSetting`; see [dev-ui.md](dev-ui.md#loading-and-stepping-session)) |
| `dev-ui check --turns N` | `python::host_for` (see [world.md](world.md#setting-checker---turns-n)) |
| Decider tests | closure, replay or external deciders built by hand, or `host_for` (the dev UI's `test_support::python_world`) |

## Extending

- **A new kind of decider** (Stage 05's dev-UI decider, a decider in another language): implement `Decider`. Do the work, or start it, in `submit` and hand over the reply in `collect`; keep `submit` from blocking if the decider is slow, so other deciders can work at the same time. Override `reject` if a refused reply leaves the decider in a state it shouldn't continue from. A decider in another language that speaks the protocol on its standard streams needs no new code: build an `ExternalDecider` with its command.
- **A new cause of failure**: add a `DeciderFailure` variant with a one-line `Display`, and return it from `stderr_tail` if it carries error output. The dev UI's log lines are built from `Display` and `stderr_tail`, so they need no change.
- **Choosing deciders some other way than by setting name** (a player choosing a business's decider): build the host from that choice instead of `host_for`. The World only needs every business's name to be held.
- **Removing the write limit**: give `Process` a writer thread fed by a channel, and have `submit` send to it.

## Not yet

The dev UI shows each business's answer, its refusals and trace, and its outcome (see [dev-ui.md](dev-ui.md#businesses-factories-and-technologies)). Taking over a business and sending its decisions from the window is Stage 05. Businesses entering or leaving the simulation, and the rule for a new business's decider, are not planned yet. There is no strict mode that halts on a failure, and no ready handshake with a process beyond the grace window.

## Tests

- `decider/host.rs` unit tests: the idle and empty hosts' `holds`, `names` in order, adding a name twice panics, `check` names the first business with an unknown decider (and its `Display`, with and without known names), the `Display` of every failure variant except `Exited` and of a `ReportedFailure` with a tail, `WontStart`'s tail, `DecisionCounts::default()`, and a closure decider answering its request.
- `decider/replay.rs` unit tests: the recorded turn is answered; a missing turn and a different request fail; two recordings for one turn are refused; `from_files` reports a missing and a malformed file, naming the path.
- `decider/external.rs` unit tests (no Python): a command's `Display`, the default config, and a program that doesn't exist failing with `Spawn`.
- `decider/python.rs` unit tests, through the `_from` forms only: the interpreter path, a missing override (the message names the path, the setup commands and `POLICYSPACE_PYTHON`), an existing file accepted, and the time limit (`0` is none, `2.5` and ` 10 ` parse, `abc`, `-1`, `inf` and the empty string are refused).
- `crates/engine/tests/decider_host.rs` (no Python), with closure and replay deciders on `businesses_valid` (`farm_a` and `farm_b` name `rule`, `idle` names `other_rule`) and the sample: an unknown name fails world creation and the idle host accepts it; a rebuilt host missing a name refuses the step and changes nothing; each decider receives only its businesses' views in ID order, and a decider with no business an empty request, every turn; a refused decision idles only its business and is kept with its errors; a reply with a wrong turn, an unknown business or a duplicate idles every business of that decider and is reported; a failed closure decider is asked again next turn; an empty reply means nothing; the sample replays `sample_turn0.recording.json` at turn 0 with three valid answers equal to the recording's, then reports `NoRecording` at turn 1; an edited request fails with `RequestDiffers`; answers are replaced every turn.
- `crates/engine/tests/external_decider.rs` (needs the project's Python; fails with `PythonError`'s message, never skips, when the environment is missing): the sample under the real `arithmetic` decider gives three valid answers equal to the recording's at turn 0 and keeps doing so for five steps with no failure; scripts written inline with `python -c` cover a process that exits before answering (reported with its exit code and error output, and started again next turn, counted through a file), one that answers past a 300 ms limit (killed and restarted), one that answers twice per turn (its stale second line is refused at the next turn and the process is restarted), one that writes something that isn't a reply, one that exits at once (a start error), an unknown decider name (`host_for` on `production_valid` fails with the script's message), and dropping a host ending its processes (a heartbeat file stops growing).
- Every other engine test passes the idle host. Their expectations don't depend on the host, except factory conditions after `World::new` or a step, which show the idle businesses' wear. `crates/engine/tests/business_trade.rs` resolves closure deciders' answers and, for the sample, the real `arithmetic` decider's (see [trade.md](trade.md#tests)).
