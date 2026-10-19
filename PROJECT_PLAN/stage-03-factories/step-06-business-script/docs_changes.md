# Step 06: business-script — Docs Changes

This Step builds on Step 05's proposed `docs/decider.md`, which is not in `docs/` yet. Changes to that file below are relative to Step 05's draft and name the sections it defines. The Python package and the arithmetic rule get a new file, `docs/python.md`.

## docs/python.md — new file
- **Location:** whole file
- **Change:** add

# The Python package and its deciders

Source: `python/` — `pyproject.toml` (the project file), `policyspace/__init__.py` (package docstring), `policyspace/protocol.py` (the messages, the strict reader, the engine-style writer), `policyspace/arithmetic.py` (the arithmetic rule), `policyspace/deciders.py` (the registry of decider names and the line-per-turn loop) and `policyspace/__main__.py` (the command line). Tests are in `python/tests/`. The recorded messages the tests read are in `crates/engine/tests/recordings/` (see [decider.md](decider.md#recordings)).

`policyspace` is the project's Python side: a client for the decider protocol and the deciders the project ships. A decider is a function from one turn's batch of views (a `Request`) to a batch of decisions (a `Reply`), exactly as [decider.md](decider.md) defines them. The package can run a decider as a long-lived process that answers one request line with one reply line per turn.

**Nothing in the engine runs the package yet.** The engine builds views and validates decisions, and the Python tests drive the deciders with recorded requests. Step 07 of Stage 03 starts the script from the engine and drives the exchange, and Step 08 resolves the decisions it sends.

## Setup

The package lives in its own folder, `python/`, and is installed in editable form into the project's virtual environment. `requirements.txt` lists `jupyter`, `pytest` and `-e ./python`, so the usual setup command, run from the project root, installs it:

```
python -m venv .venv
.venv/Scripts/python -m pip install -r requirements.txt
```

The first install fetches the build backend named in `pyproject.toml` (`setuptools>=68`), so it needs network access once. Afterwards `.venv/Scripts/python -m policyspace ...` and `import policyspace` work from any directory, and edits to the source take effect without reinstalling.

- Python 3.12 or later (`requires-python`). The package has **no runtime dependencies**: the standard library's `json` is enough, and it is lossless for this format (see [Numbers](#numbers)). `pytest` is the only test dependency.
- `pyproject.toml` uses the setuptools backend, declares the one package `policyspace`, and sets pytest's `testpaths = ["tests"]`.
- `.gitignore` ignores `*.egg-info/` (written by the editable install), `.pytest_cache/` and `__pycache__/`.

## The package

Three concerns, one module each, so each can change alone:

| Module | Concern |
|---|---|
| `protocol.py` | Reading and writing the protocol's messages in the engine's exact JSON form |
| `arithmetic.py` | The arithmetic rule: `decide(Request) -> Reply` |
| `deciders.py`, `__main__.py` | Decider names, the line-per-turn loop, the command line |

The rule knows nothing about input and output, and the loop knows nothing about strategy. A new decider is a new function from `Request` to `Reply` added to the registry (see [Extending](#extending)).

## The protocol: `protocol.py`

### The messages

Every message of [decider.md](decider.md) is a `@dataclass` with the Rust type's name and field names: `BusinessView` with `PerishGroup`, `FactoryView`, `TechnologyView` and `DepositsView`; `Decision` with `FactoryOrder`; `Request`, `Reply` and `Recording`. Tables are `dict`s keyed by name, and factory tables are keyed by `int`. Defaults follow the engine: a `Decision`'s `factories`, `offers` and `bids` default to empty and its `trace` to `None`, a `FactoryOrder`'s fractions to `0.0`, and `Decision.empty(business)` is the explicit "do nothing".

`BusinessView` has two helpers the rule uses:

- `held(good)`: the good's total over every perish group, 0 if the business holds none (the view sends groups, not totals);
- `price(good)`: 1 for the hex's currency, the posted price otherwise, `None` if the hex posts none.

`Number = int | float` is a JSON number as Python parsed it, never a `bool`.

### Reading and writing

For each message there is a `read_*` and a `write_*`: `read_view`/`write_view`, `read_decision`/`write_decision`, `read_request`/`write_request`, `read_reply`/`write_reply`, `read_recording`/`write_recording`. Each `write_*` takes `pretty=False` (one compact line, as the engine's `to_json`) or `pretty=True` (as `to_json_pretty`). Neither writes a trailing newline; callers that write files add one. Underneath, `*_from_data` and `*_to_data` convert between the dataclasses and plain JSON data, and `loads` and `dumps` convert between plain data and text.

**Reading is as strict as the engine's.** `loads` is `json.loads` with a pairs hook that refuses a key written twice and a constant hook that refuses `NaN`, `Infinity` and `-Infinity`; a parse error becomes a `ProtocolError` (a `ValueError`) with the parser's message. The `*_from_data` functions then refuse unknown keys (`unknown key 'cach' in view`), missing required keys, wrong types, a `bool` where a number is expected (`bool` is a subclass of `int` in Python, so it is checked first), and factory keys that aren't strings of decimal digits. The keys that may be left out are the ones the engine allows: a decision's `factories`, `offers`, `bids` and `trace`, a factory order's `run` and `maintenance`, and a reply's `decisions`. Value ranges are not checked (a run of 2 or a negative amount reads fine): that is the engine's validation (see [decider.md](decider.md#validation)).

**Writing mirrors the engine.** `*_to_data` builds each dict in the Rust field order, tables with their keys sorted (string keys by Python's default order, which equals the engine's byte order; factory keys numerically, written as decimal strings), `perishes` always written (`null` for "never"), and a decision's `trace` key only when there is one. Dict iteration order is never relied on for output.

### Numbers

The lossless rule of [decider.md](decider.md#the-lossless-rule) holds on the Python side too: every number read from the engine is written back unchanged, byte for byte.

- Python's `json` reads JSON integers as `int`, which holds the largest `u64` turn exactly, and every other number as a correctly rounded `float`. The package never converts between the two (no `float(int)`, no rounding).
- Python's own JSON writer lays some floats out differently from serde_json (`1e-05` where the engine writes `0.00001`), so `dumps` is a small writer of its own. It writes `None`, `int`, `float`, `str`, `list` and `dict` (in the dict's own order) compactly or in serde_json's pretty layout (two-space indent, `"key": value`, `{}` and `[]` for empty containers). Strings use `json.dumps(s, ensure_ascii=False)`, whose escapes match serde_json's. A `bool` raises `ProtocolError`, since the protocol has none.
- `format_float(x)` reproduces serde_json's float layout from Python's shortest round-trip digits (`repr`): a whole number up to 16 digits as `123.0`; a number between 1e-5 and 1e16 in plain decimal (`0.00001`, `0.0000123`, `123456789.12345679`); anything else in scientific form with a signed positive exponent (`1e-6`, `5e-324`, `1e+16`, `1.5e+16`). Zero is `0.0` or `-0.0`, and non-finite values raise `ProtocolError`.

The authority is an engine-written file, `exactness.request.json`, holding the extreme and awkward values of the format. A test reads it and writes it back, and must get the same text (see [Tests](#tests)). If the engine's layout ever changes, that test fails before anything drifts.

## The loop and the command line: `deciders.py`, `__main__.py`

`DECIDERS: dict[str, Callable[[Request], Reply]]` maps the decider names businesses carry in a setting (`decider = "arithmetic"`, see [businesses.md](businesses.md)) to their functions. It holds one name, `arithmetic`.

`serve(decider, stdin, stdout)` is the line-per-turn loop: for each line of input (blank lines skipped) it reads a request, calls the decider, writes the reply as one compact line followed by `\n`, and flushes. It returns at end of input, and raises `ProtocolError` for a line that isn't a request. The engine's compact writer puts one message on one line, so one line is one turn.

`python -m policyspace <decider>` (`main(argv)` in `__main__.py`) runs it on standard input and output:

| Situation | Effect | Exit code |
|---|---|---|
| Not exactly one argument | `usage: python -m policyspace <decider>` and the known names, to stderr | 2 |
| An unknown name | `error: unknown decider 'x' (known: arithmetic)` to stderr, before reading any input | 2 |
| A line that isn't a request | `error: ...` with the `ProtocolError`'s message, to stderr | 1 |
| End of input | | 0 |

Standard input and output are reconfigured to UTF-8, and output to `\n` line endings, so the process writes the same bytes on Windows as elsewhere. Nothing else in the package prints. Keeping the process alive across turns, timeouts, and what the engine does when the script fails or answers late are Step 07's.

## The arithmetic rule: `arithmetic.py`

`decide(request)` returns one decision per view, in the request's order, from `decide_business(view)`. The rule plans each business from its view alone (posted prices, stockpile, cash, factories, their technologies and condition) and remembers nothing between turns. It is the simplest rule that behaves sensibly under the dummy market (see [market.md](market.md)), where every offer sells at the posted price.

### No market

A view with no prices (a hex without a market) gets an empty decision with the trace `no market on this hex: nothing to do`, before anything else.

### The margin test, per technology

`margin(view, name, technology) -> Verdict` tests each technology the business's factories use, once. One turn at capacity, at the hex's posted prices:

- revenue = capacity × price(output);
- input cost = capacity × Σ inputs[g] × price(g);
- maintenance cost = Σ (capital[g] + construction[g]) × price(g) / lifetime, over the goods of both bundles: one lifetime-slice of each, the steady maintenance that holds a factory at its condition (see [production.md](production.md));
- margin = revenue − input cost − maintenance cost.

The technology **passes** when the margin is at least 0 (break-even runs). It fails when the margin is negative, or when any good it involves has no posted price: the output, an input, or a good in either bundle, checked in that order (each bundle in name order), with the first unpriced good named in the trace. A market that can't trade a good means the business could neither sell the output nor buy what running and maintenance need. The hex's currency counts as priced at 1.

Goods the business produces itself (its deposits' output) are valued at the posted price, so the test depends only on the technology and the prices, never on what the business happens to hold.

### Orders per factory

Every factory of a technology that fails is **left to run down**: run 0, maintenance 0. Every factory of one that passes is **run at capacity and maintained**:

- **run** is 1 if the factory holds at least one lifetime-slice, else 0. A shut-down factory would make nothing.
- **maintenance** is min(1 − condition after this turn's wear, 1 / construction_turns): what brings the factory back to full after wear (wear comes before maintenance in a turn), capped at one construction slice, the most the engine allows in a turn. A full factory therefore gets exactly one lifetime-slice; a factory below full catches up over a few turns; a shut-down factory is maintained and so revives slice by slice, and runs once it holds a full slice again.

The slice arithmetic mirrors `production.rs` exactly, so "holds a slice" and the condition after wear are the engine's numbers: `slices(condition, lifetime)` (condition × lifetime, rounded when within `TOLERANCE = 1e-9` of a whole number), `holds_a_slice`, `snap` (to 0 or 1 within the tolerance) and `after_wear` (one slice lost, stopping at zero). The margin test counts only the steady slice, not a catch-up, so a factory below full is restored on the strength of its steady-state margin.

### Offers and bids

`needs(view, orders)` sums, per good, capacity × run × inputs[g] for each factory plus maintenance × (capital[g] + construction[g]). `trade(view, needs)` compares each good's need with what the business holds (`view.held`, over every perish group): it **bids the shortfall and offers the surplus**. The currency and goods with no posted price are skipped, and amounts below `TOLERANCE` are dropped, so every decision passes the engine's validation. Offering every priced surplus means the business sells last turn's output and unneeded unimproved goods without having to tell them apart, and a business with nothing to run still sells what it holds.

### The cash budget

Sales settle before purchases and the dummy market buys everything offered, so before buying the business will have

budget = cash + Σ offers[g] × price(g).

If the bids cost more than that, the rule multiplies **every factory's run and maintenance fractions by one common proportion**, budget / cost, and recomputes needs, offers and bids once from the scaled orders. Scaling needs down can only shrink bids and grow offers, so one pass lands within the budget, up to rounding. One proportion for every factory and both fractions mirrors the factory mechanics' proportional sharing of a shortage. The engine's cash cap (Step 08) is the safety net, not the normal path.

**A known gap:** if a setting puts the hex's currency itself in a technology's inputs or bundles, the margin test prices it at 1, but the budget doesn't reserve that cash, because the currency is never bid. The sample never does this, and the engine's cash cap covers it. A later rule could subtract the currency's own need from the budget.

### The trace

Every decision carries a trace, the only account of why the business did what it did (Step 09 shows it in the dev UI). Its lines are joined by `\n`: one line per technology in name order (the margin arithmetic and the verdict), one per factory in number order (its final, possibly scaled fractions), then the budget line. Numbers are formatted with `.6g`. The farm in the sample at turn 0:

```
farm: revenue 24; inputs 12; maintenance 7; margin 5: run and maintain
factory #0 (farm, condition 1): run 1, maintenance 0.125
budget 100 = cash 100 + offers 0; bids cost 15: within budget
```

The farm's technology makes 12 grain (price 2) from 1 unimproved grain (0.5) and 0.5 labor (1) per unit, and its bundles (4 tools at 12, 8 labor at 1) last 8 turns: revenue 24, inputs 12, maintenance (48 + 8) / 8 = 7. Running at capacity needs 12 unimproved grain (it holds 8, so it bids 4), 6 labor plus 0.125 × 8 for maintenance (bid 7), and 0.125 × 4 = 0.5 tools (bid 0.5), costing 15 out of 100 cash. The other forms of a factory line are `factory #N (t, condition c): run down` and `...: shut down, maintenance m`, and a cut plan's budget line ends `: scaled by s`. Tests rely on the phrases `run and maintain`, `run down`, `shut down`, `scaled by`, `within budget` and `no market`; the rest of the wording may change.

## Tests

Run them with `.venv/Scripts/python -m pytest python/tests`, or as the fourth check of `tools/check.py` (see [workspace.md](workspace.md#the-check-command)). `conftest.py` locates `crates/engine/tests/recordings/` and provides `recording_text(name)`, `hand_written_view(name)` (the only view of `<name>.request.json`) and the `sample_text` fixture.

- `test_protocol.py`: `format_float` against a table of engine layouts, and refusing non-finite values; `dumps` compact and pretty, and refusing booleans; strict reading (bad syntax, unknown, missing and duplicate keys, `NaN`, bad factory keys, booleans, string and float turns); defaults and the empty decision's exact text; sorted tables; **the exactness request comes back byte for byte**; the sample recording round trips in pretty form; every request file round trips.
- `test_arithmetic.py`: the sample's reply equals the recording's, and its numbers (the three factories run at capacity, the farm bids grain and labor and maintains one slice, the mine catches up from 0.75, the toolmaker runs at a margin of 0.25); one test per hand-written request (negative margin, unpriced input, cash short, revival, no market, seller), plus an unpriced bundle good and the currency in a bundle, built in code with `dataclasses.replace`; and an invariant over every request: no offer or bid names the currency or an unpriced good, every amount is above 0, every fraction is from 0 to 1, every factory in the view has an order, and every trace is non-empty.
- `test_loop.py`: runs `python -m policyspace` as a subprocess. Two request lines with a blank line between give two reply lines with no `\r`; an unknown name and a missing name exit 2; a bad line exits 1 with nothing on stdout.

When `POLICYSPACE_UPDATE_RECORDINGS` is set, the sample test rewrites the reply half of `sample_turn0.recording.json` with the rule's reply before comparing. See [decider.md](decider.md#recordings) for when and how to regenerate.

## Extending

- **A new decider**: write a function from `Request` to `Reply` in its own module, add it to `DECIDERS` under the name businesses will carry, and test it against recordings. Keep the shape batch in, batch out, and build decisions through `protocol`'s dataclasses so they are written in the engine's form. Answer every business in the request, in order, with an empty decision where there is nothing to do.
- **A change to the protocol** (a new field or verb in [decider.md](decider.md#extending)): add the field to the dataclass, its reader and writer in the same key position as the Rust struct, and regenerate the recordings. The round-trip tests fail until both sides agree.
- **A runtime dependency**: avoid one if the standard library will do. If one is needed, add it to `pyproject.toml`'s `dependencies`.

## Not yet

Starting the script from the engine, keeping it alive, timeouts, failure semantics and the replay decider are Step 07 of Stage 03. Resolving the decisions and the cash cap are Step 08, and showing traces in the dev UI is Step 09. Strategy beyond the arithmetic rule (ranking factories by margin, protecting maintenance over running, holding inventory, memory between turns) and vectorized or parallel evaluation of a batch are not planned yet. The rule assumes every offer sells; once Stage 04's live market breaks that, the rule still produces valid decisions, and the engine trims what can't be done.

## docs/decider.md — modify
- **Location:** introduction, the paragraph beginning "**Nothing runs a decider or touches the turn yet.**"
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), the same paragraph in its new file

**Nothing in the engine runs a decider or touches the turn yet.** `World::business_view` builds a view on demand, and everything else is types and functions with their own tests. The other side of the protocol exists in Python: the `policyspace` package reads and writes these messages byte for byte and holds the first decider, the arithmetic rule (see [python.md](python.md)). The two sides are tied together by committed recordings that both test suites read (see [Recordings](#recordings)). Step 07 runs the exchange (building the request, transport, what a refused decision or a missing reply means, replaying recordings), Step 08 resolves valid decisions (trading, running and maintaining factories, trimming over-asking), and Step 09 shows decisions and traces in the dev UI.

## docs/decider.md — modify
- **Location:** "The batch", the paragraph beginning "A **recording** is a request and its reply saved together."
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), the same paragraph in its new file

A **recording** is a request and its reply saved together. The Python tests read recordings to drive the deciders, and Step 07's replay decider will answer from them, so engine tests that don't test strategy need no Python. The committed recordings are described in [Recordings](#recordings).

## docs/decider.md — modify
- **Location:** new section after "JSON" (after its subsection "Where serde_json lives"), before "Extending"
- **Change:** add

## The Python side

The deciders are Python processes, and `python/policyspace/protocol.py` is this format's other implementation (see [python.md](python.md#the-protocol-protocolpy)). It holds the same messages as dataclasses with the same field names, reads them as strictly (unknown, missing and repeated keys, wrong types, `NaN` and booleans refused), and leaves value checks to the engine's validation. Its writer reproduces serde_json's layout, both compact and pretty, so a message the engine writes comes back from Python byte for byte.

**Transport is one message per line.** `to_json` writes a message on one line, so a request is one line in and a reply is one line out. `python -m policyspace <decider>` answers each request line with one reply line until its input ends (see [python.md](python.md#the-loop-and-the-command-line-deciderspy-__main__py)). A reply from it answers every business in the request, in the request's order, with an empty decision for a business on a hex without a market. Step 07 starts the process and drives this exchange.

## Recordings

`crates/engine/tests/recordings/` holds committed protocol messages that the engine's tests and the Python tests both read, and Step 07's replay decider will read too. Its `README.md` describes the files. There are three kinds:

| File | Written by | Form | Purpose |
|---|---|---|---|
| `sample_turn0.recording.json` | the engine (`request` half) and the Python test (`reply` half) | pretty, trailing newline | the sample's turn-0 request for its three center businesses, and the arithmetic rule's reply |
| `exactness.request.json` | the engine | compact, one line plus a newline | extreme and awkward numbers (`Turn::MAX`, `f64::MAX`, `5e-324`, `1/3`, `1e-5`, `1e16` and others Python's own writer lays out differently); Python must write it back byte for byte |
| `negative_margin`, `unpriced_input`, `cash_short`, `revival`, `no_market`, `seller` (`.request.json`) | hand | pretty | one business each, for cases the sample doesn't reach; the Python tests assert what the rule does with them |

`.gitattributes` pins the folder to LF line endings (`crates/engine/tests/recordings/** text eol=lf`), because the tests compare these files byte for byte and git on Windows would otherwise check them out with CRLF.

`crates/engine/tests/recordings.rs` keeps the files honest, through the public API and `tests/common` only:

- `the_sample_recording_matches_the_engine` loads the sample, builds the request the way Step 07 will (`world.turn()` and one `business_view` per business in `businesses()` order), and checks the file's request equals it, the file is in the engine's canonical pretty form (which also proves Python's pretty writer matches), and the file's reply validates against the request with one `Answer::Valid` per view, in the request's order. If the sample setting, the view or the rule changes, this test fails, so the recording can't drift from the engine.
- `the_exactness_request_is_the_engines` builds the exactness request by hand and checks the file equals its `to_json()` plus a newline. The literal `123456789.123456789` is written `123_456_789.123_456_79`, the same `f64`, because clippy's `excessive_precision` lint rejects the longer form.
- `hand_written_requests_parse` reads every `*.request.json` in name order: each parses, has at least one view, and every view's turn equals the request's. It expects exactly seven files (six when regenerating, when it skips the exactness file another test is writing).

### Regenerating

When the sample setting, the protocol or the rule changes, regenerate the two machine-written files under one environment variable, `POLICYSPACE_UPDATE_RECORDINGS`. With it set, the engine's tests rewrite the request half of the sample recording (keeping the reply they find) and the exactness request, and the Python sample test rewrites the reply half. Without it every test only checks. From the project root:

```
set POLICYSPACE_UPDATE_RECORDINGS=1      (PowerShell: $env:POLICYSPACE_UPDATE_RECORDINGS = "1")
cargo test -p policyspace-engine --test recordings -- --test-threads=1   # rewrites the engine's halves
.venv/Scripts/python -m pytest python/tests                              # rewrites the rule's reply
cargo test -p policyspace-engine --test recordings -- --test-threads=1   # confirms
```

Then unset the variable. The `cargo test` runs single-threaded so that the tests that write files don't run at the same time as each other. After the first step the reply check may fail (the reply is stale or empty): that is the signal to run the Python step. Hand-written requests are edited by hand and never regenerated.

## docs/decider.md — modify
- **Location:** "Extending", at the end of the list
- **Change:** add

- **Any change to the format** has a second implementation to update: `python/policyspace/protocol.py` (the dataclass, its reader and its writer, with the field in the same key position; see [python.md](python.md#extending)). Then regenerate the recordings (see [Regenerating](#regenerating)); the round-trip and fixture tests fail until both sides agree.

## docs/decider.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), the same section in its new file

Building a request from every business in the turn, the decision point in the turn, starting the Python process and driving the exchange, resolving decider names, what a refused decision or a missing reply means, checking requests read from recordings, and the replay decider are Step 07 of Stage 03. The first decider exists in Python but nothing calls it from the engine (see [python.md](python.md)). Resolving decisions (trading, running and maintaining factories, trimming over-asking, the cash cap, keeping the trace in the business's record) is Step 08, and showing decisions and traces in the dev UI is Step 09. Verbs beyond the two fractions, offers and bids (budgets, construction, retooling, trading assets) are later Stages. Per-deposit IDs and structured traces are not planned yet.

## docs/decider.md — modify
- **Location:** "Tests", new bullet after the `crates/engine/tests/decider.rs` bullet (and its sub-bullets), before "No test setting was added"
- **Change:** add

- `crates/engine/tests/recordings.rs` checks the committed recordings against the engine: the sample recording's request and canonical form, and its reply's validity; the exactness request; and that every hand-written request parses (see [Recordings](#recordings)). The Python tests read the same files (see [python.md](python.md#tests)).

## docs/README.md — modify
- **Location:** the document table, the `decider.md` row (the row Step 05 adds after `production.md`)
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), the same row

| [decider.md](decider.md) | The decider protocol as data: the `BusinessView` a business is served (built by `World::business_view`), the `Decision` it answers with (run and maintenance fractions per factory, offers, bids, a trace), validation against the view and its errors, the `Request`/`Reply`/`Recording` batch and reply validation, the lossless JSON format (name-keyed tables, quoted factory numbers, `Quantity` and `Turn` forms, two-layer reading, duplicate keys), one message per line as transport, and the committed recordings both test suites share and how to regenerate them |

## docs/README.md — modify
- **Location:** the document table, new row after the `decider.md` row
- **Change:** add

| [python.md](python.md) | The Python package `policyspace`: setup in the virtual environment, the protocol client (dataclasses, strict reading, the engine-style writer and `format_float`), the decider registry, the line-per-turn loop and `python -m policyspace <decider>`, the arithmetic rule (margin test, orders per factory, offers and bids, the cash budget, the trace), and its tests |

## docs/README.md — modify
- **Location:** the `workspace.md` row
- **Change:** replace

| [workspace.md](workspace.md) | The Cargo workspace and the Python package beside it, the crates and their dependency rule, the pinned toolchain, the Python environment, the check command, and test layout including the shared test helpers and recordings |

## docs/workspace.md — modify
- **Location:** title and the start of the file, before "## Layout"
- **Change:** add (after the title line)

The project is a Rust workspace with two crates, plus a Python package for the business deciders that the engine will run as a separate process (see [python.md](python.md)).

## docs/workspace.md — modify
- **Location:** the repository layout block, the `tests/businesses.rs, ...` entry and the `tests/settings/` line
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), the same entry

```
    tests/businesses.rs, currency.rs, decider.rs, demand.rs, factories.rs, goods.rs,
          growth.rs, map.rs, market.rs, pops.rs, production.rs, recordings.rs,
          resources.rs, setting_loader.rs, technologies.rs, trade.rs, world.rs
    tests/recordings/   committed decider messages shared with the Python tests (see decider.md)
    tests/settings/     small setting folders used by engine tests
```

## docs/workspace.md — modify
- **Location:** the repository layout block, between the `dev-ui/` entries and `settings/sample/`, and at the end
- **Change:** add

```
python/                 the policyspace Python package (see python.md)
  pyproject.toml        project file: setuptools, no runtime dependencies, pytest settings
  policyspace/          protocol.py, arithmetic.py, deciders.py, __main__.py
  tests/                the Python tests: conftest.py, test_protocol.py, test_arithmetic.py,
                        test_loop.py
```
before `settings/sample/`, and
```
requirements.txt        the virtual environment: jupyter, pytest, the package (editable)
.gitattributes          keeps tests/recordings/ at LF line endings
```
after `tools/check.py`.

## docs/workspace.md — modify
- **Location:** "Crates and the dependency rule", new bullet after the engine-dependency bullet (the one Step 05 replaces)
- **Change:** add

- **The Python package** `policyspace` in `python/` depends on nothing at runtime beyond the standard library, and on `pytest` for its tests. It talks to the engine only through the decider protocol's JSON (see [decider.md](decider.md)), never by importing or linking it, so the two sides meet only at the message format and the committed recordings both test. Add a Python dependency only when a Step needs it.

Note for `update-docs`: the stale sentence "The engine's only external dependency is `toml` ..." in this section is already replaced by Step 05's proposal (adding `serde` and `serde_json`); apply that one.

## docs/workspace.md — modify
- **Location:** new section after "Toolchain", before "The check command"
- **Change:** add

## The Python environment

The project's Python lives in a virtual environment, `.venv`, created and filled from the project root:

```
python -m venv .venv
.venv/Scripts/python -m pip install -r requirements.txt
```

`requirements.txt` installs `jupyter` (for the manual), `pytest`, and the `policyspace` package in editable form (`-e ./python`; the path is relative to the project root, where the command runs). The first install fetches the package's build backend, so it needs network access once. Python 3.12 or later is required. `.gitignore` excludes `.venv/`, `__pycache__/`, `*.egg-info/` and `.pytest_cache/`.

## docs/workspace.md — modify
- **Location:** "The check command", whole section body
- **Change:** replace

`python tools/check.py` (or `.venv/Scripts/python tools/check.py`) is the one command that checks the whole project. It is a standard-library-only script that works from any directory. It runs, in order, from the project root:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `python -m pytest python/tests`, with the interpreter that runs the script

It runs all four even if an earlier one fails, prints a pass/fail summary, and exits 0 only if all passed. If `cargo` isn't found it prints a message and exits 1. Fix formatting with `cargo fmt --all`. There is no CI yet; this command is the gate.

The fourth check uses the interpreter running the script (`sys.executable`), so run the script with `.venv/Scripts/python` to test against the virtual environment. Under an interpreter without pytest the check fails with Python's `No module named pytest`; `.venv/Scripts/python -m pip install -r requirements.txt` fixes it.

## docs/workspace.md — modify
- **Location:** "Tests", at the end of the list
- **Change:** add

- `crates/engine/tests/recordings/` holds committed decider protocol messages (a sample recording, an engine-written exactness request and hand-written requests) that `crates/engine/tests/recordings.rs` and the Python tests both read. They are compared byte for byte, so `.gitattributes` keeps them at LF line endings. The sample recording and the exactness request are regenerated with `POLICYSPACE_UPDATE_RECORDINGS` set (see [decider.md](decider.md#recordings)).
- The Python tests are in `python/tests/`, run by pytest from the virtual environment (`.venv/Scripts/python -m pytest python/tests`, or the check command). `conftest.py` locates the recordings folder and reads its files (see [python.md](python.md#tests)).

## docs/production.md — modify
- **Location:** "The model", "A safety net, not the normal path", the sentence "A business isn't supposed to send orders its goods can't cover: the arithmetic rule in Step 06 asks only for what it can supply." (in the file Step 04 adds)
- **Change:** replace
- **Supersedes:** Step 04 (factory-mechanics), the same sentence in its new file

A business isn't supposed to send orders its goods can't cover: the arithmetic rule bids for the goods its plan needs and scales the plan down to the cash it will have (see [python.md](python.md#the-cash-budget)).

## docs/production.md — modify
- **Location:** "Not yet" (in the file Step 04 adds)
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), its replacement of the same section

Calling the mechanics in the turn, keeping the per-business records in the World, and anything in the turn report are Step 08. The fractions come from a business's `Decision` (see [decider.md](decider.md)), whose format and validation exist, and the first decider, the arithmetic rule, chooses them in Python with the same slice arithmetic as this module (see [python.md](python.md#orders-per-factory)); collecting decisions in the turn is Step 07. Showing the records in the dev UI is Step 09. Redistributing goods left unused after a shared cut is not planned. Building new factories, scrapping and retooling are Stage 07.

## docs/businesses.md — modify
- **Location:** introduction, the second paragraph ("Nothing decides or trades for a business yet. ...")
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), its replacement of the same paragraph

Nothing decides or trades for a business in the running world yet. Factories (see [factories.md](factories.md)) name a business as their owner, and must sit on its hex. The view a business would be served and the decision it would answer with exist as data, with validation and JSON (see [decider.md](decider.md)), and the first decider, the arithmetic rule, exists in Python (see [python.md](python.md)). The decider host (Step 07) will run it for each business, and business trade (Step 08) will trade from and into the business stockpile. Until then a business only holds its inventory, receives the output of its deposits, and owns factories that don't run yet.

## docs/businesses.md — modify
- **Location:** the sample setting bullet, the clause "every one with decider `arithmetic` (the name Step 06's rule will be registered as)" (from Step 02's draft)
- **Change:** replace
- **Supersedes:** Step 02 (businesses), the same clause

every one with decider `arithmetic`, the name of the arithmetic rule in the Python package's registry (see [python.md](python.md#the-loop-and-the-command-line-deciderspy-__main__py)):

## docs/businesses.md — modify
- **Location:** "Not yet", its first sentence
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), the same sentence

The factory mechanics a business's factories will follow exist as pure functions (see [production.md](production.md)), its view and decisions exist as data (see [decider.md](decider.md)), and the first decider exists in Python (see [python.md](python.md)). The following are later Steps of Stage 03: running the exchange and resolving decider names against the Python registry (Step 07), business trade, running its factories in the turn and the use of its cash (Step 08), and showing businesses, their factories and their decisions in the dev UI (Step 09).

## docs/technologies.md — modify
- **Location:** "Not yet"
- **Change:** replace
- **Supersedes:** Step 05 (business-view-and-decisions), its replacement of this section

Factories, which run technologies, exist (see [factories.md](factories.md)), and the factory mechanics of running, depreciation and maintenance exist as pure functions (see [production.md](production.md)). A business's view lists each technology its factories use once, with every number above (see [decider.md](decider.md#the-view)), and the arithmetic rule tests each technology's margin at posted prices from those numbers (see [python.md](python.md#the-margin-test-per-technology)). The decider host (Step 07), business trade with factories running in the turn (Step 08), and showing businesses and factories in the dev UI (Step 09) are later Steps of this Stage. Constructing new factories, scrapping, and moving a factory to another technology are Stage 07. Several outputs, by-products and substitution between inputs are not planned.
