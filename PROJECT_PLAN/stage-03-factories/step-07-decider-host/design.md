# Step 07: decider-host

## Summary
This Step gives the turn its decision point. Once per turn, after endowment and before any trading, every business's view is served to the decider its setting entry names and every decision is collected, one batched exchange per decider. The deciders live outside the World, in a host the caller builds and hands to the World, so the engine owns no processes and never depends on a decider's memory. A Decider abstraction has three implementations: the external-process decider, which starts the Python script once and exchanges one line per turn across the life of the host; the replay decider, which answers from committed recordings so engine tests that don't test strategy need no Python; and a closure decider for tests. Failure has a defined meaning: a broken decider idles the businesses it controls for the turn and is reported, and the turn never halts. The World stores each business's answer for Step 08 to resolve and Step 09 to show.

## Behavior

### The host and the Decider
A **decider** answers one request (a turn's batch of views) with one reply (a batch of decisions), as Step 05 defines them. The engine defines this as an abstraction with no knowledge of processes or files, and the three implementations below fill it in.

A **host** is a table of deciders by name: the names businesses carry in the setting (Step 02). The caller builds it: the dev UI and the engine's tests, never the World. The World takes the host at creation (turn 0 has a decision point like every turn) and at every step, and reads from it only during the decision point. The World holds no decider between turns, so a host can be dropped and rebuilt, and its processes with it, without the simulation losing anything. A fresh world in the dev UI (open or reload) gets a fresh host.

**An idle host** is the host with no deciders in which every business does nothing every turn. It is for callers that don't care about business behavior: every existing engine and dev-UI test can use it, so their expectations don't change in this Step, and don't change in Step 08 either, since a business that decides nothing trades nothing and runs nothing.

### Resolving names
A business's decider name is resolved against the host when a world is created from it. A business naming a decider the host doesn't hold is an error at world creation, naming the business and the name, because a misnamed decider in a setting is an authoring mistake, not a turn-time event. The idle host is exempt: it answers for every name with nothing.

All the businesses that name one decider share it: one decider receives one request per turn holding only its businesses' views, in business ID order. Every decider in the host receives exactly one request per turn, empty when it controls no business, so a decider can count turns from its input. The views in a request are the ones Step 05 builds, scoped to each business.

### The decision point
The decision point runs at the start of the "actors act" part of the turn, after emission and endowment and before the trade round, so a view shows the stockpile as Step 05 describes it. The host:

1. Builds every decider's request from the businesses it controls.
2. Writes every request before reading any reply, so several external processes work at the same time without threads in the engine; deciders are taken in a fixed order (by name) so the exchange is deterministic.
3. Reads each reply and validates it against its request under Step 05's rules: a reply refused as a whole (wrong turn, a decision for a business not in the request, two decisions for one business) is a decider failure; otherwise each business gets its answer, nothing, valid, or refused with the validation errors.
4. Stores the answer per business in the World, where it stays until the next decision point replaces it. Step 08 resolves the valid ones; Step 09 shows them with their traces.

Nothing else in the turn changes in this Step: businesses still trade nothing and their factories still don't run.

### Failure semantics
A decider degrades the simulation in a known way and never halts it. By the decision point emission and endowment have already happened, and a halt would leave the turn half done and freeze the dev UI on a broken script.

- **A refused decision** (fails validation against its view) idles that business for the turn: it counts as having decided nothing. Other businesses, of the same decider or not, are unaffected. The refused decision and its errors are kept as the business's answer, so the developer can see what was sent and why it was refused.
- **A failed decider** (its process has died, it didn't answer within the time limit, its reply couldn't be read as a reply, or its reply was refused as a whole) idles every business it controls for the turn. The turn report records the failure: the decider's name and the reason, including the last lines of the process's error output when there are any. An external process that failed is killed, so a stale reply can't arrive next turn, and is started again at the next decision point; if it fails again, the failure is reported again. A replay or closure decider that failed is simply asked again next turn.
- **A process that won't start** when the host is built is an error at host construction, naming the decider and the reason (the interpreter wasn't found, the process exited at once, with its error output). "At once" means within a short grace window after starting, during which the host watches for an exit; the window is host configuration like the time limit. A process that exits later than that is seen as a decider failure at the next decision point instead. The caller decides what to do: the dev UI shows it as a load error, in place of the world, the way a broken setting is shown.

Tests of the turn assert that the report lists no failures.

### The external-process decider
Started once, when the host is built, with the command the caller gives it (a program and its arguments). It writes one compact request line to the process's standard input per turn and reads one reply line from its standard output, which is the exchange Step 06's `python -m policyspace <name>` already speaks. It has a **time limit** per exchange, set by whoever builds the host, with a default of a few seconds; the limit is host configuration, not setting data, because it is about the machine running the simulation, not about the world. Error output from the process is captured so a failure can quote it. When the host is dropped its processes are killed.

### The replay decider
Holds recordings, one per turn. For a request it finds the recording with that turn, checks that the engine's request equals the recorded one, and returns the recorded reply. A request whose turn has no recording, or that differs from the recorded request, is a decider failure: the replay decider exists for tests, and answering silently would hide a drifted or missing recording. A test that wants businesses idle after the recorded turns uses the idle host. Comparing a recorded request against an engine-built one also closes the gap Step 05 noted: a request read from a file is never trusted on its own.

### The closure decider
A decider built from a Rust function from request to reply, so tests of the failure semantics can send back a wrong turn, an unknown business, a refused decision, or nothing, without a process or a file.

### Finding Python
A helper finds the interpreter of the project's virtual environment relative to the project root, so the dev UI and the engine's tests start the same Python. An environment variable overrides the interpreter, and another lifts the time limit, so a developer can point at a different environment or step through the script in a debugger. Engine tests that need Python fail with a clear message when the environment isn't there; they are not skipped, because the vision makes the engine's tests depend on the project's Python environment.

The dev UI and the setting checker's `--turns N` build their host from the loaded setting: every distinct decider name its businesses carry becomes an external-process decider running the project's Python with that name. The setting alone decides which script runs, and Step 06's exit-at-startup for an unknown name becomes a load error in the UI.

### The report and the dev UI
The turn report gains the decider failures of the turn (name and reason), and counts of businesses that sent a valid decision, were refused, and sent nothing. The dev UI's turn log prints each failure as a line of its own, so a dying script is visible as soon as it happens rather than in Step 09. Nothing else in the UI changes.

## Player-facing Changes
The dev UI and the checker now start the setting's deciders as Python processes when a setting is loaded, and a decider that won't start is shown as a load error. The turn log shows decider failures. Nothing changes in the setting format, the Python package, or what businesses do in the world.

## Interactions
- **Business view and decisions** (Step 05): the request, reply, validation and answers the host uses; the recordings the replay decider reads.
- **Business script** (Step 06): the process the external-process decider starts, its line-per-turn exchange, its exit at startup for an unknown name, and the committed sample recording, which the Python integration test compares against.
- **Businesses** (Step 02): the decider name resolved here; businesses grouped by it.
- **World and turn** (Stage 01 Step 06, Stage 02 Step 06): the decision point joins the turn between endowment and the trade round; world creation and stepping take the host; the report grows.
- **Dev UI** (Stage 01 Steps 07 and 08, Stage 02 Step 08): the session and the checker build the host, show start failures as load errors, and log decider failures.
- **Workspace** (Stage 01 Step 01): the virtual environment the helper finds; the check command's tests now run Python from the engine.
- **Later Steps:** Step 08 resolves the stored valid decisions and applies the cash cap. Step 09 shows each business's answer, its trace and refusals in the dev UI. Stage 05 adds the dev UI itself as a decider through the same abstraction.

## Out of Scope
- Resolving decisions: trading, running and maintaining factories, the cash cap (Step 08).
- Showing decisions and traces in the dev UI beyond the failure lines in the turn log (Step 09).
- Any change to the protocol, the Python package or the arithmetic rule.
- Deciders written in anything but Python, beyond what the command-based external process already allows.
- Businesses entering or leaving the simulation, and the setting rule for a new actor's decider.
- Threads that run any part of the simulation; parallelism comes only from several processes working on their requests at once. Each external process does get two small I/O threads that pump its output and its error output, which is the only way standard Rust can wait for a reply with a time limit and keep a talkative script from blocking on a full pipe; they do no simulation work, hold no world state, and end with the process.
- Halting the simulation on a decider failure, or any strict mode.

## Acceptance Criteria
- A world created from the sample with a host running the real Python `arithmetic` decider stores, at turn 0, one valid decision per center business equal to the committed sample recording's reply, and stepping several turns keeps producing valid decisions with no failure in the report. The same world under a replay host built from the sample recording stores the same answers at turn 0.
- Every existing engine and dev-UI test passes with an idle host, with no change to its expectations.
- A business naming a decider the host doesn't hold fails world creation with an error naming both; the idle host accepts every name.
- With closure deciders: a decision that fails validation idles only that business and is kept as refused with its errors; a reply with a wrong turn, an unknown business, or a duplicate idles every business of that decider and is reported as a failure; a decider controlling no business still receives an empty request; two deciders each receive only their own businesses' views, in business ID order.
- With an external process: a process that exits before answering, one that answers past the time limit, and one that writes something that isn't a reply each idle their businesses for that turn, appear in the report with the reason and the process's error output, and are started again at the next step; a command that can't start fails host construction with a clear error.
- The replay decider fails for a turn with no recording and for a request that differs from the recorded one.
- Dropping a host ends its processes. In the dev UI, loading a setting whose decider can't start shows a load error, and a failure during a step appears in the turn log.
- When the virtual environment is missing, the tests that need it fail with a message saying so rather than being skipped.
