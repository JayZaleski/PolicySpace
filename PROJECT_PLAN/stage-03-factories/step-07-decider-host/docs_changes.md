# Step 07: decider-host — Docs Changes

This Step builds on earlier drafts in this Stage that are not in `docs/` yet: Step 05's `docs/decider.md`, Step 06's `docs/python.md`, Step 02's `docs/businesses.md`, Step 04's `docs/production.md`, and their edits to `world.md`, `workspace.md`, `technologies.md` and `README.md`. Changes to those files below are relative to those drafts and name the sections they define. Running the exchange gets a new file, `docs/decider-host.md`, because `decider.md` describes the protocol as data and this is the runtime around it.

## docs/decider-host.md — new file
- **Location:** whole file
- **Change:** add

# The decider host and the decision point

Source: `crates/engine/src/decider/` — `host.rs` (the `Decider` trait, `DeciderHost`, the decision point, `BusinessAnswer`, `DeciderFailure`, `ReportedFailure`, `DecisionCounts`, `UnknownDecider`, `ClosureDecider`), `external.rs` (the external-process decider), `replay.rs` (the replay decider) and `python.rs` (finding the project's Python and building the host a setting asks for). The World side is `World::new`, `World::step`, `World::decide` and `World::business_answer` in `world.rs`. The crate root re-exports `BusinessAnswer`, `Decider`, `DeciderFailure`, `DeciderHost`, `ReportedFailure` and `UnknownDecider`; everything else is under `policyspace_engine::decider` (`ClosureDecider`, `DecisionCounts`, `ExternalCommand`, `ExternalDecider`, `ProcessConfig`, `StartError`, `ReplayDecider`, `ReplayError`, `HostError`, `PythonError`), and the submodules are public as `decider::host`, `decider::external`, `decider::replay` and `decider::python`.

Once per turn the World stops at its **decision point**: every business's view (see [decider.md](decider.md#the-view)) goes to the decider its setting entry names, and every decision comes back, one batched exchange per decider. The deciders live in a **host** that the caller builds and passes to the World, so the engine owns no processes and keeps nothing of a decider's between turns. The World stores each business's answer. **Nothing resolves the answers yet:** businesses still trade nothing and their factories don't run, so the turn's other numbers are unchanged; Step 08 of Stage 03 resolves the valid decisions.

## Concepts

- **The host is outside the World.** A `DeciderHost` is a table of deciders by name, the names businesses carry in their `decider` key (see [businesses.md](businesses.md)). The dev UI, the checker and the tests build it, never the World. `World::new(setting, &mut host)` and `World::step(&mut host)` take it, and the World reads from it only inside the decision point. There is no host-less form of either call, so a caller can't forget it. Because the World holds no decider between turns, a host (and its processes) can be dropped and rebuilt without the simulation losing anything.
- **Names are resolved against the host**, at world creation and again at the top of every step (a rebuilt host may lack a name). A business naming a decider the host doesn't hold is an authoring mistake, not a turn-time event, so it is an error: `UnknownDecider` from `World::new`, `StepError::UnknownDecider` from `World::step`, and in both cases nothing is built or changed.
- **One request per decider per turn.** All the businesses that name one decider share it: it receives one `Request` holding only their views, in business ID order. Every decider in the host receives exactly one request every turn, an empty one when it controls no business, so a decider can count turns from its input.
- **The turn never halts.** By the decision point, emission and endowment have already happened, so a halt would leave the turn half done. A decision refused by validation idles its business; a decider that fails idles every business it controls and is reported in the turn report; the turn goes on either way.
- **Submit everything, then collect.** The `Decider` trait has two phases. The host submits every decider's request before collecting any reply, so several external processes work on their requests at the same time without the engine using threads for the simulation.
- **The idle host** holds every name and answers nothing: every business decides nothing every turn. It is for callers that don't care about business behavior; every engine test that isn't about deciders uses it, and their expectations don't depend on it, since a business that decides nothing does nothing.

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
2. For each decider in name order: collect the IDs of the businesses whose `decider` is that name, in ID order (possibly none); build `Request { turn: world.turn(), views }` with one `World::business_view` per business; call `submit`. Keep the name, the IDs, the request and the submit result.
3. For each kept exchange in the same order: if `submit` succeeded, call `collect`. Validate the reply with `Reply::validate(&request, goods)` (see [decider.md](decider.md#the-batch)).
   - A reply refused as a whole: call `reject` on the decider and record `DeciderFailure::Refused(errors)`.
   - Otherwise each business gets its answer: `Answer::Nothing` becomes `Nothing`, `Answer::Valid(d)` becomes `Valid(d.clone())`, and `Answer::Refused(errors)` becomes `Refused { decision, errors }` with the one decision in the reply for that business (exactly one after validation).
4. Any failure, from `submit`, `collect` or validation, is pushed as a `ReportedFailure { decider: name, failure }`, and that decider's businesses stay `Nothing`.

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
- A refused decision is kept with its errors, so a developer can see what was sent and why it was refused (Step 09 shows it). It counts as deciding nothing.
- `TurnReport` gains `decisions: DecisionCounts` (how many businesses sent a valid decision, were refused, and sent nothing; they add up to the number of businesses) and `decider_failures: Vec<ReportedFailure>` (every decider that failed this turn, in name order). Tests of the turn assert that `decider_failures` is empty.
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

**Starting.** `ExternalDecider::start(command, config) -> Result<ExternalDecider, StartError>` spawns the program with all three standard streams piped, takes the pipes, and starts two **pump threads**: one reads standard output line by line into an `mpsc` channel, the other reads standard error and keeps its last `STDERR_TAIL_LINES` lines (lossy UTF-8, line endings stripped) in a shared `VecDeque`. Then it watches the process for `start_grace`, polling `try_wait` every 10 ms: a process that exits in that window is `ExitedAtOnce` with its status and error tail. A spawn error (program not found, not executable) is `Spawn`. `StartError::summary()` is the one-line form (`'{command}' couldn't be started: {error}`, or `'{command}' exited ({status}) as soon as it started`); `Display` adds `its error output ends with:` and the indented tail. A process that exits after the grace window is found at the next exchange instead, as `Exited`.

**The exchange.** `submit` restarts the process if a failure stopped it (a start failure becomes `WontStart`), then writes and flushes the request line; a write error means the process is gone (`Exited`). `collect` waits on the channel with `recv_timeout(time_limit)` (or `recv` with no limit): a line is parsed with `Reply::from_json` (`Unreadable` if it fails); a closed channel or a read error is `Exited`; a timeout is `TimedOut`. Validating the reply is the host's job, not the decider's.

**Stopping.** Every failure path stops the process: kill it (harmless if it has already exited), wait for its status, join both pumps (they end at end of file once the process is gone), and snapshot the error tail into the failure. Afterwards no process is held (`is_running()` is false) until the next `submit` starts one, so a stale reply can never arrive in a later turn. `reject` stops it the same way. Dropping an `ExternalDecider` kills and waits for its process, so dropping a host ends all its processes.

**Why threads.** The pumps are the only threads in the engine. Standard Rust can't wait on a pipe with a time limit, and a process that writes a lot of error output would block on a full pipe if nothing read it. They do no simulation work, hold no world state and end with the process.

**Known limits.**

- The request write in `submit` has no time limit. A process that has stopped reading its input blocks the turn once a request is larger than the pipe buffer (about 64 KB on Windows, roughly 100 businesses on one decider at the sample's size of view). Today's settings are far below that; a writer thread fed by a channel would remove the limit.
- Joining the pumps after a kill assumes nothing else holds the process's pipes. A decider that starts a child process of its own which inherits the pipes and outlives it would delay the join. On Windows the virtual environment's `python.exe` launcher and the real interpreter end together, so the project's Python is not affected.

## The replay decider

`ReplayDecider` answers from recordings (see [decider.md](decider.md#recordings)), one per turn, so engine tests that don't test strategy need no Python.

- `ReplayDecider::new(recordings)` keys them by their request's turn; two for one turn is `ReplayError::DuplicateTurn`. `ReplayDecider::from_files(paths)` reads one recording per file with `Recording::from_json` and calls `new`; `ReplayError::Read` and `ReplayError::Format` name the path. `turns()` lists the recorded turns.
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
| Engine tests not about deciders, `tests/common`'s `world_of`, the dev UI's `test_support::world_at`, `text.rs` tests | `DeciderHost::idle()` (`common::idle()` in the integration tests) |
| The dev UI window | `python::host_for` for each loaded setting (`Hosting::FromSetting`; see [dev-ui.md](dev-ui.md#loading-and-stepping-session)) |
| `dev-ui check --turns N` | `python::host_for` (see [world.md](world.md#setting-checker---turns-n)) |
| Decider tests | closure, replay or external deciders built by hand, or `host_for` |

## Extending

- **A new kind of decider** (Stage 05's dev-UI decider, a decider in another language): implement `Decider`. Do the work, or start it, in `submit` and hand over the reply in `collect`; keep `submit` from blocking if the decider is slow, so other deciders can work at the same time. Override `reject` if a refused reply leaves the decider in a state it shouldn't continue from. A decider in another language that speaks the protocol on its standard streams needs no new code: build an `ExternalDecider` with its command.
- **A new cause of failure**: add a `DeciderFailure` variant with a one-line `Display`, and return it from `stderr_tail` if it carries error output. The dev UI's log lines are built from `Display` and `stderr_tail`, so they need no change.
- **Choosing deciders some other way than by setting name** (a player choosing a business's decider): build the host from that choice instead of `host_for`. The World only needs every business's name to be held.
- **Removing the write limit**: give `Process` a writer thread fed by a channel, and have `submit` send to it.

## Not yet

Resolving the stored decisions (trading, running and maintaining factories, trimming over-asking, the cash cap) is Step 08 of Stage 03, and showing each business's answer, its trace and its refusals in the dev UI is Step 09; until then only the failure lines in the turn log are visible. The dev UI as a decider is Stage 05. Businesses entering or leaving the simulation, and the rule for a new business's decider, are not planned yet. There is no strict mode that halts on a failure, and no ready handshake with a process beyond the grace window.

## Tests

- `decider/host.rs` unit tests: the idle and empty hosts' `holds`, `names` in order, adding a name twice panics, `check` names the first business with an unknown decider (and its `Display`, with and without known names), the `Display` of every failure variant except `Exited` and of a `ReportedFailure` with a tail, `WontStart`'s tail, `DecisionCounts::default()`, and a closure decider answering its request.
- `decider/replay.rs` unit tests: the recorded turn is answered; a missing turn and a different request fail; two recordings for one turn are refused; `from_files` reports a missing and a malformed file, naming the path.
- `decider/external.rs` unit tests (no Python): a command's `Display`, the default config, and a program that doesn't exist failing with `Spawn`.
- `decider/python.rs` unit tests, through the `_from` forms only: the interpreter path, a missing override (the message names the path, the setup commands and `POLICYSPACE_PYTHON`), an existing file accepted, and the time limit (`0` is none, `2.5` and ` 10 ` parse, `abc`, `-1`, `inf` and the empty string are refused).
- `crates/engine/tests/decider_host.rs` (no Python), with closure and replay deciders on `businesses_valid` (`farm_a` and `farm_b` name `rule`, `idle` names `other_rule`) and the sample: an unknown name fails world creation and the idle host accepts it; a rebuilt host missing a name refuses the step and changes nothing; each decider receives only its businesses' views in ID order, and a decider with no business an empty request, every turn; a refused decision idles only its business and is kept with its errors; a reply with a wrong turn, an unknown business or a duplicate idles every business of that decider and is reported; a failed closure decider is asked again next turn; an empty reply means nothing; the sample replays `sample_turn0.recording.json` at turn 0 with three valid answers equal to the recording's, then reports `NoRecording` at turn 1; an edited request fails with `RequestDiffers`; answers are replaced every turn.
- `crates/engine/tests/external_decider.rs` (needs the project's Python; fails with `PythonError`'s message, never skips, when the environment is missing): the sample under the real `arithmetic` decider gives three valid answers equal to the recording's at turn 0 and keeps doing so for five steps with no failure; scripts written inline with `python -c` cover a process that exits before answering (reported with its exit code and error output, and started again next turn, counted through a file), one that answers past a 300 ms limit (killed and restarted), one that answers twice per turn (its stale second line is refused at the next turn and the process is restarted), one that writes something that isn't a reply, one that exits at once (a start error), an unknown decider name (`host_for` on `production_valid` fails with the script's message), and dropping a host ending its processes (a heartbeat file stops growing).
- Every other engine test passes the idle host, with its expectations unchanged.

## docs/decider.md — modify
- **Location:** introduction, the paragraph beginning "**Nothing in the engine runs a decider or touches the turn yet.**"
- **Change:** replace
- **Supersedes:** Step 06 (business-script), the same paragraph

**The engine runs the exchange once per turn.** At the decision point, through a host of named deciders the caller builds (a Python process, a replayed recording, or a Rust closure), every business's view is served and its decision validated and stored in the World (see [decider-host.md](decider-host.md)). The other side of the protocol exists in Python: the `policyspace` package reads and writes these messages byte for byte and holds the first decider, the arithmetic rule (see [python.md](python.md)). The two sides are tied together by committed recordings that both test suites read (see [Recordings](#recordings)). Nothing resolves the decisions yet: Step 08 resolves valid decisions (trading, running and maintaining factories, trimming over-asking), and Step 09 shows decisions and traces in the dev UI.

## docs/decider.md — modify
- **Location:** "The decision", "Validation", the last sentence ("What happens to a refused decision (the business doing nothing that turn, or the run stopping) is Step 07's.")
- **Change:** replace

A refused decision idles its business for the turn and is kept with its errors; the run goes on (see [decider-host.md](decider-host.md#failures)).

## docs/decider.md — modify
- **Location:** "The batch", the paragraph "The turn has one decision point, ... because a decider may control any set of businesses and Step 07 may split one turn among several deciders."
- **Change:** replace its last clause

because a decider may control any set of businesses, and the host splits one turn among several deciders, one request each (see [decider-host.md](decider-host.md#the-decision-point)).

## docs/decider.md — modify
- **Location:** "The batch", the paragraph beginning "A **recording** is a request and its reply saved together."
- **Change:** replace
- **Supersedes:** Step 06 (business-script), the same paragraph

A **recording** is a request and its reply saved together. The Python tests read recordings to drive the deciders, and the replay decider answers from them, so engine tests that don't test strategy need no Python (see [decider-host.md](decider-host.md#the-replay-decider)). The committed recordings are described in [Recordings](#recordings).

## docs/decider.md — modify
- **Location:** "The batch", the paragraph beginning "Nothing checks that a request is consistent in itself"
- **Change:** replace

Nothing checks that a request is consistent in itself (each view's turn against the request's, or two views of one business). A request the engine builds can't have either problem. A request read from a file is never trusted on its own: the replay decider answers only when the engine's own request for that turn equals the recorded one.

## docs/decider.md — modify
- **Location:** "The Python side", the second paragraph ("**Transport is one message per line.** ..."), its last sentence ("Step 07 starts the process and drives this exchange.")
- **Change:** replace
- **Supersedes:** Step 06 (business-script), the same sentence

The engine's external-process decider starts the process once, keeps it alive across turns, and drives this exchange with a time limit per turn (see [decider-host.md](decider-host.md#the-external-process-decider)).

## docs/decider.md — modify
- **Location:** "Recordings", the first sentence ("`crates/engine/tests/recordings/` holds committed protocol messages that the engine's tests and the Python tests both read, and Step 07's replay decider will read too.")
- **Change:** replace
- **Supersedes:** Step 06 (business-script), the same sentence

`crates/engine/tests/recordings/` holds committed protocol messages that the engine's tests and the Python tests both read, and that the replay decider answers from in the engine's tests.

## docs/decider.md — modify
- **Location:** "Recordings", the `the_sample_recording_matches_the_engine` bullet, the words "builds the request the way Step 07 will"
- **Change:** replace with

builds the request the way the decision point does

and add at the end of the bullet:

`crates/engine/tests/decider_host.rs` replays the same file through the World (three valid answers at turn 0), and `crates/engine/tests/external_decider.rs` checks that the real `arithmetic` process answers the sample's turn 0 with the recorded reply.

## docs/decider.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 06 (business-script), the same section

Resolving decisions (trading, running and maintaining factories, trimming over-asking, the cash cap, keeping the trace in the business's record) is Step 08 of Stage 03, and showing decisions and traces in the dev UI is Step 09. Verbs beyond the two fractions, offers and bids (budgets, construction, retooling, trading assets) are later Stages. Per-deposit IDs and structured traces are not planned yet. Running the exchange is described in [decider-host.md](decider-host.md).

## docs/decider.md — modify
- **Location:** "Tests", at the end of the list (after the `crates/engine/tests/recordings.rs` bullet Step 06 adds)
- **Change:** add

- The host, the decision point and the three deciders have their own tests, listed in [decider-host.md](decider-host.md#tests).

## docs/python.md — modify
- **Location:** introduction, the paragraph beginning "**Nothing in the engine runs the package yet.**"
- **Change:** replace
- **Supersedes:** Step 06 (business-script), the same paragraph in its new file

**The engine runs it.** The dev UI and `dev-ui check --turns` start `python -m policyspace <name>` once for each decider name the setting's businesses carry, with the virtual environment's interpreter, and exchange one line per turn with it at the decision point (see [decider-host.md](decider-host.md)). The engine validates and stores the decisions; Step 08 of Stage 03 resolves them.

## docs/python.md — modify
- **Location:** "The loop and the command line", the last paragraph, its last sentence ("Keeping the process alive across turns, timeouts, and what the engine does when the script fails or answers late are Step 07's.")
- **Change:** replace

The engine keeps the process alive across turns, waits for each reply with a time limit (5 s unless `POLICYSPACE_DECIDER_TIME_LIMIT` says otherwise), and kills and restarts a process that exits, answers late or answers something that isn't a reply; its businesses do nothing that turn (see [decider-host.md](decider-host.md#failures)). Messages written to standard error are not lost: the engine keeps the last 20 lines and quotes them when the process fails, so `print(..., file=sys.stderr)` is the way to debug a decider. Exit code 2 for an unknown name, which happens before any input is read, makes the engine's start of the process fail, so a misnamed decider in a setting is a load error in the dev UI and the checker.

## docs/python.md — modify
- **Location:** "Setup", at the end of the section
- **Change:** add

The engine finds the interpreter at `.venv/Scripts/python.exe` (Windows) or `.venv/bin/python` under the project root, or at the path in `POLICYSPACE_PYTHON` (see [decider-host.md](decider-host.md#finding-python-deciderpython)). So the dev UI needs the environment to load a setting with businesses, and `cargo test --workspace` needs it too.

## docs/python.md — modify
- **Location:** "Extending", the "**A new decider**" bullet, at the end
- **Change:** add

The engine needs no change: a business naming the new decider makes the dev UI and the checker start `python -m policyspace <name>` for it.

## docs/python.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 06 (business-script), the same section in its new file

Resolving the decisions and the cash cap are Step 08 of Stage 03, and showing traces in the dev UI is Step 09. Strategy beyond the arithmetic rule (ranking factories by margin, protecting maintenance over running, holding inventory, memory between turns) and vectorized or parallel evaluation of a batch are not planned yet. The process is kept alive across turns, so a decider could remember between turns, but none does, and a process restarted after a failure would lose that memory. The rule assumes every offer sells; once Stage 04's live market breaks that, the rule still produces valid decisions, and the engine trims what can't be done.

## docs/world.md — modify
- **Location:** introduction, at the end of the first paragraph
- **Change:** add

The turn also asks the businesses' deciders what to do, through a decider host that the caller passes to `World::new` and `World::step`; the world holds no decider between calls (see [decider-host.md](decider-host.md)).

## docs/world.md — modify
- **Location:** "What a World holds", the table, new row after the `factories` row (which Step 03 adds)
- **Change:** add

| `business_answers: IdVec<BusinessKind, BusinessAnswer>` | `Nothing` for every business at creation | replaced at every decision point: each business's answer (nothing, a valid decision, or a refused decision with its errors; see [decider-host.md](decider-host.md#answers-and-the-report)) |

## docs/world.md — modify
- **Location:** "What a World holds", the paragraph after the table, after the `business_view` sentence (the one Step 05 adds)
- **Change:** add

`business_answer(business)` returns the business's `BusinessAnswer` from the current turn's decision point, and panics for a business from another world.

## docs/world.md — modify
- **Location:** "The turn", item 2 of the numbered list
- **Change:** replace

2. **Actors act:** the decision point, then the trade round and consumption, then growth. At the decision point every business's view goes to the decider its setting entry names, through the host passed to `new` or `step`, and its answer is stored; nothing acts on the answers yet (see [decider-host.md](decider-host.md#the-decision-point)). Then on every hex each POP submits its parameters and holdings, the hex's market computes their demand and settles every sale before any purchase, and each POP consumes its utility goods. Then, once every hex has traded, every POP grows or shrinks by the rate its outcome records (see [Trade round](#trade-round), [Growth](#growth), [trade.md](trade.md) and [growth.md](growth.md)). Later Steps resolve the businesses' decisions here, and later Stages add the live market.

## docs/world.md — modify
- **Location:** "The turn", the three bullets after "Two calls drive the clock, and both run a turn through one private function"
- **Change:** replace
- **Supersedes:** Step 02 (businesses), its replacement of the first two bullets

- `World::new(setting, &mut host) -> Result<(World, TurnReport), UnknownDecider>` first checks that the host holds every business's decider name (`host.check`), and fails, building nothing, if one is missing. Then it creates the world at turn 0 with empty hex and POP stockpiles, each business's stockpile filled from its starting inventory (each good in the group it would get if added in turn 0, `PerishesAt::for_good(perishes, Turn::ZERO)`), and every business's answer `Nothing`, and runs `run_turn` for turn 0, so a fresh world's deposits have emitted on top of that, its businesses have answered, and its POPs have already been endowed, traded, consumed and grown. The report's `perished` is empty.
- `World::step(&mut self, &mut host) -> Result<TurnReport, StepError>` first checks that the clock can advance (`Turn::next`) and that the host holds every business's decider name, so a refused step changes nothing (sizes, outcomes, business stockpiles and answers included). Then it runs the end of the current turn (`perish`: hexes in ID order, then POPs in ID order, then businesses in ID order), advances the clock, and runs `run_turn` for the new turn with what perished. The host may be a different one from the last call (a rebuilt host, say), as long as it holds every name.
- `World::run_turn(&mut self, perished, &mut host) -> TurnReport` (private) calls `emit`, `endow`, `decide`, `trade` and `grow` in that order and builds the report from their results: `emitted` and `endowed` from the first two, `decisions` and `decider_failures` from `decide`, `sold`, `bought` and `consumed` from the private `TradeTotals` that `trade` returns, `population` from `grow`, and `turn` from the clock.

## docs/world.md — modify
- **Location:** "The turn", the paragraph beginning "So between calls the world is always **after part 2**", its first sentence
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same sentence

So between calls the world is always **after part 2**: this turn's output is in the hex stockpiles (unowned deposits) and the business stockpiles (owned deposits), every business's answer for this turn is stored, this turn's trade is done, each POP's stockpile holds what the round left, each POP's size is its size after growth, and nothing has perished yet this turn.

## docs/world.md — modify
- **Location:** "The turn", the paragraph "`StepError` has one variant, ..."
- **Change:** replace

`StepError` has two variants: `ClockAtMaximum` (the clock is at `Turn::MAX`), unreachable in practice, and `UnknownDecider(UnknownDecider)` (a business names a decider the host doesn't hold, displayed as the `UnknownDecider`'s message). Either way the world is unchanged. It implements `Display` and `std::error::Error`.

## docs/world.md — modify
- **Location:** "The turn", new subsection after "Endowment", before "Trade round"
- **Change:** add

### Decision point

`World::decide(&mut self, &mut host)` (private) runs after `endow` and before `trade`. It calls the host's crate-private `decision_point(self)`, which builds each decider's request from `business_view`, exchanges it, and validates the reply (see [decider-host.md](decider-host.md#the-decision-point)). It replaces `business_answers` with the answers, counts them into a `DecisionCounts`, and returns the counts and the failures for the report. The decision point only reads the world, so the views see the stockpiles after emission and endowment; nothing in the turn acts on the answers yet.

## docs/world.md — modify
- **Location:** "Determinism", its first sentence
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same sentence

No `HashMap` or `HashSet` is used. Deposits emit in ID order, POPs are endowed in ID order, deciders are taken in name order and the businesses within a decider in ID order, the trade round runs on hexes in ID order, POPs in ID order within a hex and goods in ID order within a POP, with every sale on a hex before any purchase and then consumption, POPs grow in ID order, stockpiles perish hexes first, then POPs, then businesses, each in ID order, stockpiles are `BTreeMap`s, and every list is in good ID order, so the same setting stepped the same number of times with deciders that answer the same way gives identical worlds (POP sizes, business stockpiles and answers included), outcomes and reports.

## docs/world.md — modify
- **Location:** "TurnReport", the code block, after the `population` line
- **Change:** add

```rust
    pub decisions: DecisionCounts,          // businesses with a valid decision, refused, and nothing, at `turn`'s decision point
    pub decider_failures: Vec<ReportedFailure>, // every decider that failed at `turn`'s decision point, in name order
```

## docs/world.md — modify
- **Location:** "TurnReport", the paragraph after the code block, at the end
- **Change:** add

`decisions` adds up to the number of businesses; `decider_failures` is empty when every decider answered (see [decider-host.md](decider-host.md#answers-and-the-report)). The turn line doesn't print `decisions`; failures are printed as lines of their own after it.

## docs/world.md — modify
- **Location:** "Setting checker: `--turns N`", at the end of the section
- **Change:** add

Before creating the world, `--turns` starts the setting's deciders with `python::host_for`: one `python -m policyspace <name>` process per decider name its businesses carry, with the project's virtual environment (see [decider-host.md](decider-host.md#finding-python-deciderpython)). A setting without businesses starts no Python. If a decider can't start (the environment is missing, or the Python package doesn't know the name), or a business names a decider the host doesn't hold, the checker prints the message and `1 error(s) found in <path>` to stderr after the success line, runs no turns, and exits with 1. A decider that fails during a turn is not an error: after that turn's line it prints `Turn {n}: decider '{name}' failed: {reason}`, then each line of the process's error output as `    stderr: {line}`, and carries on. The sample's deciders answer every turn, so its output is unchanged by them.

## docs/world.md — modify
- **Location:** "Tests", first bullet, the `world.rs` item
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same item

`world.rs` (a world forced to `Turn::MAX`, for `turns_valid`, `pops_placed` and `businesses_valid`, refuses to step and leaves its turn, hex stockpiles, POP stockpiles, business stockpiles, business answers, POP outcomes and POP sizes unchanged)

## docs/world.md — modify
- **Location:** "Tests", second bullet (`crates/engine/tests/world.rs`), at the end
- **Change:** add

These tests, and every engine test not about deciders, build and step worlds with the idle host (`common::idle()`), so no business decides anything. The decision point is tested in `crates/engine/tests/decider_host.rs` and `external_decider.rs` (see [decider-host.md](decider-host.md#tests)).

## docs/world.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), the same section

Changing deposit ownership during play, starting stockpiles in `start/` for hexes or POPs (businesses have their starting inventory), deposit stock, depletion and regeneration, any effect of `movable`, per-hex, per-POP and per-business report breakdowns (per-POP trade and growth are in `World::pop_outcome`, each business's answer in `World::business_answer`), saving and loading a world, prices that change during play and rationing (Stage 05), removing POPs or changing a POP's hex or type during play, businesses acting on their decisions (they are served views and their answers are stored at the decision point, see [decider-host.md](decider-host.md), but nothing resolves them until Step 08 of Stage 03), and factories running, wearing or being maintained in the turn (the mechanics exist as pure functions, see [production.md](production.md), and Step 08 of Stage 03 calls them; until then factories are held as state, unchanged).

## docs/businesses.md — modify
- **Location:** introduction, the second paragraph ("Nothing decides or trades for a business in the running world yet. ...")
- **Change:** replace
- **Supersedes:** Step 06 (business-script), its replacement of the same paragraph

Nothing trades for a business yet. Factories (see [factories.md](factories.md)) name a business as their owner, and must sit on its hex. Every turn, at the decision point, each business is served its view and its decider's answer is validated and stored in the world (see [decider.md](decider.md) for the view and the decision, [decider-host.md](decider-host.md) for the exchange, and [python.md](python.md) for the first decider, the arithmetic rule). Business trade (Step 08) will act on the answers, trading from and into the business stockpile and running its factories. Until then a business holds its inventory, receives the output of its deposits, owns factories that don't run yet, and has an answer that nothing acts on.

## docs/businesses.md — modify
- **Location:** "Concepts", the "**Decider name.**" bullet
- **Change:** replace

- **Decider name.** Required and a valid name. All the businesses naming one decider share it: each turn it receives one request with all their views. The name is resolved against the decider host when a world is created and at every step, and a business naming a decider the host doesn't hold is an error (`UnknownDecider`). The dev UI and the checker build the host from the names themselves, starting `python -m policyspace <name>` for each, so a name the Python package doesn't know is a load error there (see [decider-host.md](decider-host.md#finding-python-deciderpython)). The loader still checks only that it is a valid name.

## docs/businesses.md — modify
- **Location:** "Data model", the code block, the `decider` line
- **Change:** replace

```rust
    pub decider: String,                    // a valid name; resolved against the decider host by the World
```

## docs/businesses.md — modify
- **Location:** "How businesses are loaded", the keys table, the `decider` row
- **Change:** replace

| `decider` | A valid name. Resolved when a world is created, not by the loader (see [decider-host.md](decider-host.md)) |

## docs/businesses.md — modify
- **Location:** "Using businesses", the code block, its first line and the `decider` line
- **Change:** replace `let (world, _report) = World::new(load_setting(path)?);` with

```rust
let mut host = DeciderHost::idle();                      // or python::host_for(&setting)? (see decider-host.md)
let (world, _report) = World::new(load_setting(path)?, &mut host)?;
```

and `    let decider = business.decider.as_str();             // not resolved yet` with

```rust
    let decider = business.decider.as_str();             // the name its host resolves
    let answer = world.business_answer(id);              // its answer at this turn's decision point
```

## docs/businesses.md — modify
- **Location:** "Not yet", its first sentence
- **Change:** replace
- **Supersedes:** Step 06 (business-script), the same sentence

The factory mechanics a business's factories will follow exist as pure functions (see [production.md](production.md)), its view and decisions exist as data (see [decider.md](decider.md)), and its decider answers every turn (see [decider-host.md](decider-host.md)). The following are later Steps of Stage 03: business trade, running its factories in the turn and the use of its cash, acting on its answer (Step 08), and showing businesses, their factories and their decisions in the dev UI (Step 09).

## docs/production.md — modify
- **Location:** "Not yet" (in the file Step 04 adds)
- **Change:** replace
- **Supersedes:** Step 06 (business-script), its replacement of the same section

Calling the mechanics in the turn, keeping the per-business records in the World, and anything in the turn report are Step 08. The fractions come from a business's `Decision` (see [decider.md](decider.md)), which every business's decider sends at the decision point and the World stores (see [decider-host.md](decider-host.md)); the first decider, the arithmetic rule, chooses them in Python with the same slice arithmetic as this module (see [python.md](python.md#orders-per-factory)). Showing the records in the dev UI is Step 09. Redistributing goods left unused after a shared cut is not planned. Building new factories, scrapping and retooling are Stage 07.

## docs/technologies.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 06 (business-script), its replacement of this section

Factories, which run technologies, exist (see [factories.md](factories.md)), and the factory mechanics of running, depreciation and maintenance exist as pure functions (see [production.md](production.md)). A business's view lists each technology its factories use once, with every number above (see [decider.md](decider.md#the-view)), and the arithmetic rule tests each technology's margin at posted prices from those numbers (see [python.md](python.md#the-margin-test-per-technology)); its decisions are collected every turn (see [decider-host.md](decider-host.md)). Business trade with factories running in the turn (Step 08) and showing businesses and factories in the dev UI (Step 09) are later Steps of this Stage. Constructing new factories, scrapping, and moving a factory to another technology are Stage 07. Several outputs, by-products and substitution between inputs are not planned.

## docs/dev-ui.md — modify
- **Location:** introduction, after the sentence listing the engine's public interface it uses (ending "... `ENGINE_NAME`, `ENGINE_VERSION`).")
- **Change:** add

It also uses the decider host: `DeciderHost`, `ReportedFailure` and `DeciderFailure`, and `decider::python` (`host_for`, `HostError`) to start the setting's deciders; its tests add `ClosureDecider`, `Reply` and `PythonError` (see [decider-host.md](decider-host.md)).

## docs/dev-ui.md — modify
- **Location:** "Module structure", the table rows for `check.rs`, `text.rs`, `session.rs` and `test_support.rs`
- **Change:** replace those four rows

| `check.rs` | The headless checker only: `run` and `run_turns`. `run_turns` starts the setting's deciders and prints each turn's line and its decider failures. Its wording comes from `text`. |
| `text.rs` | All the text the dev UI shows: the `check` output (`error_lines`, `load_failure_lines`, `count`, `turn_line`, `failure_lines`) and the window's panels (countdowns, deposit lines, stockpile lines, market lines, POP sections, goods, resource-type and POP-type rows, and the index, rate and share formatting), built from engine types. No egui types. |
| `session.rs` | `Session`, `Loaded` and `Hosting`: the current folder, what loading it gave, the decider host of the loaded world, the turn log, and the selected hex. Steps the world. No egui types. |
| `test_support.rs` | `#[cfg(test)]` helpers shared by the unit tests: `repo_path(relative)` (a path from the repository root) and `world_at(relative)` (loads a setting and returns its world at turn 0 under the idle host, panicking with the errors). |

## docs/dev-ui.md — modify
- **Location:** "Loading and stepping: `Session`", the list of what `Session` holds
- **Change:** replace

`Session` holds:

- `hosting: Hosting`: how each load builds its decider host;
- `folder: Option<PathBuf>`;
- `loaded: Option<Loaded>`;
- `host: Option<DeciderHost>`: the host of the loaded world, `None` whenever `loaded` isn't a world;
- `log: Vec<String>`: every turn line since the last load, oldest first, each followed by its decider failure lines;
- `step_error: Option<String>`: the error from the last refused step since the last load;
- `selected: Option<HexCoord>`: the selected hex.

`Hosting` is `FromSetting` (one Python process per decider name the setting's businesses carry, through `python::host_for`; the window uses it), and, in tests only, `Idle` (`DeciderHost::idle()`) and `Custom(HostBuilder)` (a boxed `Fn(&LoadedSetting) -> Result<DeciderHost, HostError>`). `Idle` and `Custom` are `#[cfg(test)]` because nothing outside the tests builds them yet; Stage 05 lifts that when the dev UI becomes a decider. `Session::new(hosting)` takes it, and `Session` has no `Default`. The app calls `Session::new(Hosting::FromSetting)`.

## docs/dev-ui.md — modify
- **Location:** "Loading and stepping: `Session`", "**Loading.**", the second bullet ("Loading calls `load_setting`. ...")
- **Change:** replace

- Loading first drops the previous host, which ends its processes, then calls `load_setting`. On failure it stores `Loaded::Errors(lines)`, where `lines` is `text::error_lines(&errors, &folder)`, and the log is emptied. On success it builds the host from `hosting` and then `World::new(setting, &mut host)` (at turn 0, with turn 0's decision point, trade round and growth already run). The log becomes `text::turn_line` of turn 0's `TurnReport`, followed by `text::failure_lines` for each of its decider failures, the host is kept, and the result is `Loaded::World(world)`.
- If the host can't be built (`HostError`: the Python environment is missing, the time-limit variable is malformed, or a decider exits as soon as it starts, such as for a name the Python package doesn't know), or `World::new` refuses a decider name (`UnknownDecider`), the load fails like a broken setting: `Loaded::Errors(text::load_failure_lines(&error.to_string(), folder))`, with the log emptied and no host. The window shows these lines in its error view.

## docs/dev-ui.md — modify
- **Location:** "Loading and stepping: `Session`", the "**Stepping.**" paragraph, its first sentence
- **Change:** replace

**Stepping.** `step(times)` calls `World::step(&mut host)` up to `times` times and pushes `text::turn_line` of each report onto the log, followed by `text::failure_lines` for each decider failure in it, so a failing script shows in the turn log as soon as it fails. A decider failure never stops stepping. It stops at the first refused step and stores that `StepError`'s `Display` in `step_error`. It does nothing without a world (and its host) and returns the number of steps taken.

## docs/dev-ui.md — modify
- **Location:** "Panel text: `text`", "Shared with `check`", the list
- **Change:** add two bullets, after the `error_lines` bullet and after the `turn_line` bullet respectively

- `load_failure_lines(message, folder)`: the lines of a setting that loaded but whose deciders couldn't start (or whose decider name the host doesn't hold): `message`'s lines (a `HostError` or `UnknownDecider` `Display`, which can span several lines when it quotes a script's error output), then `1 error(s) found in {folder}`, so the error view and the checker read as for a broken setting. The status bar counts the lines before the last, so a message that quotes error output shows more than `1 error(s)` there.

- `failure_lines(turn, failure)`: the lines logged for one decider failure: `Turn {turn}: decider '{name}' failed: {failure}` (the `DeciderFailure`'s one-line `Display`), then each line of its error-output tail as `    stderr: {line}`. For example `Turn 7: decider 'x' failed: no reply within the time limit of 2.5 s`. The log and `check --turns` print them right after the turn's line.

## docs/dev-ui.md — modify
- **Location:** "Extending the window", at the end of the list
- **Change:** add

- **A different source of deciders** (Stage 05's dev UI as a decider): add a `Hosting` variant, or use `Custom` without its `#[cfg(test)]`, and build the host in `Hosting::build`. Keep the host in the session beside the world, and drop it whenever the world is replaced, so its processes end.

## docs/dev-ui.md — modify
- **Location:** "Tests", first bullet, the parenthetical list of `session.rs` tests
- **Change:** replace

`session.rs` (open, `world()`, reload after edits in a `tempfile` folder, failed loads, stepping and the log matching `turn_line` of a separately stepped world, the sample's exact turn 0 and turn 1 lines, selection kept or cleared across reload and open, all with `Hosting::Idle`; a closure decider's failure logged at turn 0 and again at every step; a `Custom` host error shown as a load error with an empty log and no world; and, with the project's Python, the sample loading and stepping under its `arithmetic` decider with no failure lines, and `production_valid`, whose businesses name `rule`, failing to load with the script's `unknown decider 'rule'` message)

## docs/dev-ui.md — modify
- **Location:** "Tests", the `text.rs` sub-bullets, at the end
- **Change:** add

  - `failure_lines` with and without an error-output tail, and `load_failure_lines` splitting a message into lines before the summary line.

## docs/dev-ui.md — modify
- **Location:** "Tests", the `crates/dev-ui/tests/checker.rs` bullet, at the end
- **Change:** add

The sample's `--turns` tests now run the real `arithmetic` decider, so they need the project's Python; their exit-code assertions quote the child's stderr, so a missing environment fails with its message. `a_decider_that_wont_start_prints_an_error_and_exits_1` runs `production_valid --turns 1` (exit 1, only the success line on stdout, `unknown decider 'rule'` on stderr), and `turns_without_businesses_start_no_python` runs `pops_placed --turns 1` with `POLICYSPACE_PYTHON` pointing nowhere (set on the child's `Command`, never in the test process) and still passes.

## docs/setting-loader.md — modify
- **Location:** "Setting checker (dev UI)", the table, the two `check <path> --turns N` rows
- **Change:** replace

| `check <path> --turns N` | after the success line, starts the setting's deciders (one Python process per decider name its businesses carry; none without businesses), creates a world, prints turn 0's line, then steps `N` times printing one line per turn, each followed by a line for every decider that failed that turn (see [world.md](world.md#setting-checker---turns-n)); `N` is a whole number from 0 to 10,000 (ASCII digits only, leading zeros allowed) | 0 |
| `check <path> --turns N` with errors | as for `check <path>` with errors; no turns are run | 1 |
| `check <path> --turns N` whose deciders can't start | the success line, then the reason and `1 error(s) found in <path>` on stderr; no turns are run | 1 |

## docs/setting-loader.md — modify
- **Location:** "Setting checker (dev UI)", after the table
- **Change:** add

`check <path>` without `--turns` doesn't start the deciders, so it doesn't catch a decider name the Python package doesn't know; `--turns 0` does.

## docs/workspace.md — modify
- **Location:** the repository layout block, the `src/decider/` line (which Step 05 adds)
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), the same line

```
    src/decider/        the decider protocol (view, decision, batch, JSON; see decider.md) and
                        the host that runs it each turn (see decider-host.md)
```

## docs/workspace.md — modify
- **Location:** the repository layout block, the `tests/businesses.rs, ...` entry
- **Change:** replace the entry (keep the `tests/recordings/` and `tests/settings/` lines after it)
- **Supersedes:** Step 06 (business-script), the same entry

```
    tests/businesses.rs, currency.rs, decider.rs, decider_host.rs, demand.rs,
          external_decider.rs, factories.rs, goods.rs, growth.rs, map.rs, market.rs,
          pops.rs, production.rs, recordings.rs, resources.rs, setting_loader.rs,
          technologies.rs, trade.rs, world.rs
```

## docs/workspace.md — modify
- **Location:** "The Python environment" (the section Step 06 adds), at the end
- **Change:** add

The Rust side uses the environment too. The engine's `decider::python` module finds its interpreter relative to the engine crate (`.venv/Scripts/python.exe` on Windows, `.venv/bin/python` elsewhere) and starts `python -m policyspace <decider>` from it, so the dev UI needs the environment to load a setting with businesses, and `cargo test --workspace` needs it for the tests that run the real decider. Two environment variables change this for one run: `POLICYSPACE_PYTHON` points at another interpreter, and `POLICYSPACE_DECIDER_TIME_LIMIT` sets the time limit for each reply in seconds (`0` for none, to step through a decider in a debugger). See [decider-host.md](decider-host.md#finding-python-deciderpython).

## docs/workspace.md — modify
- **Location:** "The check command", the last paragraph (Step 06's "The fourth check uses the interpreter running the script ..."), at the end
- **Change:** add

The third check needs the environment as well: some engine and dev-UI tests start the project's Python as a decider, and fail (they are never skipped) with `the project's Python environment wasn't found at ...` when it is missing. With the environment missing, `cargo test -p policyspace-dev-ui` stops at its unit-test target; add `--no-fail-fast` to reach the checker tests too.

## docs/workspace.md — modify
- **Location:** "Tests", the bullet about `crates/engine/tests/*.rs` and `common/mod.rs`, the list of shared helpers
- **Change:** replace "`lines(errors)`, `world_of(path)` (a world with its turn 0 report)," with

`lines(errors)`, `idle()` (the idle decider host, for every `World::new` and `step` that doesn't test deciders), `world_of(path)` (a world built under the idle host, with its turn 0 report),

## docs/workspace.md — modify
- **Location:** "Tests", at the end of the list
- **Change:** add

- Tests never set or remove environment variables in the test process, because tests run in parallel in one process. Code that reads one has a `_from(value)` form that tests call instead (`python::interpreter_from`, `python::time_limit_from`), and tests of a child process set variables on its `Command`.

## docs/README.md — modify
- **Location:** the document table, new row after the `python.md` row (which Step 06 adds)
- **Change:** add

| [decider-host.md](decider-host.md) | The decision point: the `DeciderHost` of named deciders the caller builds and passes to the World, name resolution, one batched request per decider per turn, the two-phase `Decider` trait, stored `BusinessAnswer`s and the report's counts and failures, the failure semantics, the external-process decider (time limit, grace window, pump threads, restart), the replay and closure deciders, and finding the project's Python (`host_for`, environment variables) |

## docs/README.md — modify
- **Location:** the document table, the `world.md` row
- **Change:** replace

| [world.md](world.md) | The running `World`, the three-part turn (emission and POP endowment; the decision point, the trade round and POP growth; perishing), hex, POP and business stockpiles with perish groups and removal, per-POP trade outcomes and per-business answers, turn reports with the total population and decider failures, and the setting checker's `--turns` option |

## docs/README.md — modify
- **Location:** the document table, the `dev-ui.md` row
- **Change:** replace

| [dev-ui.md](dev-ui.md) | The developer UI: its commands, modules, the `Session` that loads and steps settings and keeps their decider host, the map math, the text it shows (turn lines, decider failure lines, panels, POP sections), the window, and the headless `check` mode |
