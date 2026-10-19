# Step 06: business-script — Spec

## Context

Read before starting:

- `PROJECT_PLAN/stage-03-factories/step-06-business-script/design.md` — the behavior this spec implements: the package, the protocol, the loop, the arithmetic rule (margin test, maintenance fraction, offers and bids, cash budget, trace), the tests and recordings.
- `PROJECT_PLAN/stage-03-factories/step-05-business-view-and-decisions/docs_changes.md` — the decider protocol as built: every message's fields, the JSON format (names and factory ID numbers as keys, quoted factory keys, `perishes` always written, `trace` absent when none, the lossless rule, the two reading layers). This is the format Python must read and write exactly.
- `crates/engine/src/decider/view.rs`, `decision.rs`, `batch.rs`, `mod.rs` — the Rust types and their serde attributes (field order is the JSON key order; `BTreeMap` keys are sorted; `Decision.factories/offers/bids` default to empty and are always written; `FactoryOrder` fields default to 0; `Reply.decisions` defaults to empty).
- `crates/engine/src/production.rs` — the slice arithmetic the rule mirrors: `slices`, `snap`, `TOLERANCE = 1e-9`, eligibility to run (at least one lifetime-slice), wear (one slice, stopping at zero), the maintenance caps (one construction slice, then full condition).
- `crates/engine/tests/decider.rs` and `crates/engine/tests/common/mod.rs` — the integration test conventions and the shared helpers (`sample_setting`, `world_of`, `q`).
- `settings/sample/rules/technologies.toml`, `settings/sample/start/businesses.toml`, `factories.toml`, `markets.toml` — the numbers the sample recording is built from.
- `tools/check.py`, `requirements.txt`, `.gitignore`, `docs/workspace.md` (the check command and test layout).

## Approach

- **A Python package `policyspace` in `python/`**, installed in editable form into `.venv` by `requirements.txt` (`-e ./python`), with `pytest` as its only (test) dependency and the standard library's `json` as its only runtime need. Rationale: the design; Step 07's test setup then only has to find the virtual environment.
- **Three modules, three concerns.** `protocol.py` reads and writes the messages (dataclasses mirroring the Rust types, plus the JSON reader and writer). `arithmetic.py` is the rule: a function from a `Request` to a `Reply`. `deciders.py` holds the name registry and the line-per-turn loop; `__main__.py` is the command line. Rationale: each can change alone, as the design asks.
- **Numbers are kept as parsed and written back by a writer that mirrors the engine.** Python's `json` reads JSON integers as `int` (exact for `u64`) and other numbers as `float` (correctly rounded); the package never converts between them. Python's own JSON writer formats some floats differently from serde_json (`1e-05` for the engine's `0.00001`, `1e-06` for `1e-6`), so `protocol.dumps` is a small writer of its own: the shortest round-trip digits from `repr`, laid out by the engine's rules (`format_float` below). An engine-written fixture (`exactness.request.json`) is the oracle that proves byte identity. Rationale: decided with the user; the design's "written back unchanged" holds literally, and drift shows up as a failing test.
- **Dataclasses, not raw dicts.** Every message is a `@dataclass` with the Rust field names; tables are `dict`s (factories keyed by `int`); `to_data`/`from_data` convert to and from plain JSON data, building dicts in the engine's key order with sorted table keys (string keys by Python's default order, which equals the engine's byte order; factory keys numerically). Reading is strict like the engine: unknown or missing keys, wrong types, duplicate keys, `NaN`/`Infinity` and booleans are refused with a `ProtocolError`. Value ranges (fractions, negative amounts) are not checked: that is the engine's validation.
- **The rule mirrors the engine's slice arithmetic** (condition × lifetime snapped to a whole number within 1e-9, wear in slices, `snap` near 0 and 1) so that "holds a slice" and the condition after wear are the engine's numbers. Rationale: the sample's toolmaker has lifetime 24, whose slice isn't exact in binary; mirroring removes any doubt about which side's rounding the recording reflects. Everything the rule writes is a `float`.
- **An unpriced good anywhere in a technology's arithmetic fails the margin test** (output, inputs, capital or construction bundle), as `design.md` now says. The hex's currency counts as priced at 1 and is never offered or bid.
- **A business on a hex with no market gets an empty decision with a trace line** (no factories, offers or bids): every decision carries a trace, and the engine treats the rest as "do nothing". Every other decision lists every factory in the view with explicit fractions (0 and 0 for a technology left to run down).
- **Recordings live in `crates/engine/tests/recordings/`**, beside `tests/settings/`, with a README. The engine's tests keep the sample recording honest and parse the hand-written requests; the Python tests read the same files by path; Step 07's replay decider reads the same folder.
- **The sample recording is regenerated in two steps under one environment variable**, decided with the user: with `POLICYSPACE_UPDATE_RECORDINGS` set, `cargo test` rewrites the request half (keeping the reply it finds) and `pytest` rewrites the reply half; without it both only check. The Rust test also checks the file is in the engine's canonical pretty form, which proves Python's pretty writer matches too.
- **`tools/check.py` gains a fourth check**, `python -m pytest python/tests` with the interpreter running the script.
- **The build runs as one.** The recording file ties the Rust fixture test to the Python rule, and the Python tests need the hand-written files the Rust test parses; no package could be built and tested alone.

## Changes

### New

- `python/pyproject.toml` — the project file: setuptools backend, package metadata, pytest configuration.
- `python/policyspace/__init__.py` — package docstring only (what the package is: the protocol client and the project's deciders).
- `python/policyspace/protocol.py` — messages, reader, writer, `format_float`.
- `python/policyspace/arithmetic.py` — the arithmetic rule.
- `python/policyspace/deciders.py` — the registry `DECIDERS` and `serve`.
- `python/policyspace/__main__.py` — the command line.
- `python/tests/conftest.py`, `test_protocol.py`, `test_arithmetic.py`, `test_loop.py` — the Python tests (Test Plan).
- `crates/engine/tests/recordings.rs` — the Rust fixture tests (Test Plan).
- `crates/engine/tests/recordings/README.md` — what each file is, which side writes it, and the regeneration steps.
- `crates/engine/tests/recordings/sample_turn0.recording.json` — the sample's turn-0 request (engine-written) and the rule's reply (Python-written), pretty, trailing newline.
- `crates/engine/tests/recordings/exactness.request.json` — engine-written, compact (one line plus a trailing newline), the lossless oracle.
- `crates/engine/tests/recordings/negative_margin.request.json`, `unpriced_input.request.json`, `cash_short.request.json`, `revival.request.json`, `no_market.request.json`, `seller.request.json` — hand-written requests (contents under Interfaces & Data), pretty, trailing newline.

### Modified

- `requirements.txt` — becomes `jupyter`, `pytest`, `-e ./python` (one per line; the `-e` path is relative to the directory `pip` runs in, which is the project root in `CLAUDE.md`'s setup command).
- `tools/check.py` — a fourth entry in `CHECKS`: `("pytest", [sys.executable, "-m", "pytest", "python/tests"])`. Docstring: four checks, the fourth runs the Python tests from the interpreter running the script (so `.venv/Scripts/python tools/check.py` uses the virtual environment); if `pytest` isn't installed the check fails with Python's "No module named pytest", and `pip install -r requirements.txt` fixes it. "All three always run" becomes "All four".
- `.gitignore` — add `*.egg-info/` and `.pytest_cache/`.

Nothing else changes: not `crates/engine/src/`, `crates/engine/tests/decider.rs`, the dev UI, any setting, `CLAUDE.md`, `docs/` or `manual.ipynb`.

## Interfaces & Data

### `python/pyproject.toml`

```toml
[build-system]
requires = ["setuptools>=68"]
build-backend = "setuptools.build_meta"

[project]
name = "policyspace"
version = "0.1.0"
description = "PolicySpace's decider client and the project's deciders."
requires-python = ">=3.12"
dependencies = []

[tool.setuptools]
packages = ["policyspace"]

[tool.pytest.ini_options]
testpaths = ["tests"]
```

The build-system requirement is fetched by pip the first time (network access once). Installing with `.venv/Scripts/python -m pip install -r requirements.txt` from the project root installs `pytest` and the package in editable form; afterwards `.venv/Scripts/python -c "import policyspace"` works from any directory.

### `python/policyspace/protocol.py`

```python
Number = int | float          # a JSON number as Python parsed it; never a bool

class ProtocolError(ValueError):
    """Text that isn't a message of the format: the parser's message, or
    'unknown key ...', 'missing key ...', 'duplicate key ...', '... must be a number' etc."""

@dataclass
class PerishGroup:   amount: Number; perishes: int | None
@dataclass
class FactoryView:   technology: str; condition: Number
@dataclass
class TechnologyView:
    inputs: dict[str, Number]; output: str; capacity: Number
    capital: dict[str, Number]; construction: dict[str, Number]
    construction_turns: int; lifetime: int
@dataclass
class DepositsView:  count: int; produces: str; output: Number
@dataclass
class BusinessView:
    turn: int; business: str; currency: str; cash: Number
    prices: dict[str, Number]
    stockpile: dict[str, list[PerishGroup]]
    factories: dict[int, FactoryView]
    technologies: dict[str, TechnologyView]
    deposits: dict[str, DepositsView]
    def held(self, good: str) -> Number: ...   # the sum of the good's groups, 0 if not held
    def price(self, good: str) -> Number | None: ...  # 1 for the currency, else prices.get(good)
@dataclass
class FactoryOrder:  run: Number = 0.0; maintenance: Number = 0.0
@dataclass
class Decision:
    business: str
    factories: dict[int, FactoryOrder] = field(default_factory=dict)
    offers: dict[str, Number] = field(default_factory=dict)
    bids: dict[str, Number] = field(default_factory=dict)
    trace: str | None = None
    @staticmethod
    def empty(business: str) -> "Decision": ...
@dataclass
class Request:   turn: int; views: list[BusinessView]
@dataclass
class Reply:     turn: int; decisions: list[Decision]
@dataclass
class Recording: request: Request; reply: Reply

def loads(text: str) -> object            # json.loads with duplicate keys and NaN/Infinity refused (ProtocolError)
def dumps(data: object, pretty: bool = False) -> str   # the engine-style writer (below)
def format_float(value: float) -> str     # the engine's float layout (below)

def read_view(text) -> BusinessView;  write_view(view, pretty=False) -> str
def read_decision(text) -> Decision;  write_decision(decision, pretty=False) -> str
def read_request(text) -> Request;    write_request(request, pretty=False) -> str
def read_reply(text) -> Reply;        write_reply(reply, pretty=False) -> str
def read_recording(text) -> Recording; write_recording(recording, pretty=False) -> str
```

Each `read_*` is `from_data(loads(text))` for its type; each `write_*` is `dumps(to_data(x), pretty)`. `to_data`/`from_data` may be module functions or methods; what matters:

- **Key order on write** is the Rust field order: `BusinessView`: turn, business, currency, cash, prices, stockpile, factories, technologies, deposits. `PerishGroup`: amount, perishes. `FactoryView`: technology, condition. `TechnologyView`: inputs, output, capacity, capital, construction, construction_turns, lifetime. `DepositsView`: count, produces, output. `Decision`: business, factories, offers, bids, then trace only when not `None`. `FactoryOrder`: run, maintenance (both always written). `Request`: turn, views. `Reply`: turn, decisions (always written). `Recording`: request, reply. Tables are written with keys sorted: `str` keys by Python's default order; factory keys numerically, written as decimal strings (`"0"`, `"10"`). `perishes` is always written (`null` for `None`).
- **Reading** is strict: every key of a message must be one of its fields (`ProtocolError("unknown key 'cach' in view")`); `Decision.factories/offers/bids/trace`, `FactoryOrder.run/maintenance` and `Reply.decisions` may be absent (defaults as above); every other key is required. Types: names and `trace` must be `str`; `turn`, `count`, `construction_turns`, `lifetime` must be `int`; quantities and fractions must be `int` or `float`; `perishes` `int` or `None`; tables `dict`; `views`, `decisions` and perish-group lists `list`. A `bool` is refused wherever a number is expected (`bool` is an `int` subclass: check `isinstance(x, bool)` first). Factory keys must be strings of decimal digits (`"0"`, not `"x"` or `"-1"`), converted to `int`. Nothing else is checked: a run of 2 or a negative amount reads fine (the engine refuses it).
- `loads`: `json.loads(text, object_pairs_hook=..., parse_constant=...)`; the pairs hook refuses a repeated key (`ProtocolError("duplicate key 'grain'")`), `parse_constant` refuses `NaN`, `Infinity`, `-Infinity`; `json.JSONDecodeError` is re-raised as `ProtocolError` with its message.

**`dumps(data, pretty)`** writes plain data (`None`, `int`, `float`, `str`, `list`, `dict` with `str` keys, in the dict's own order; it never sorts) as JSON text:

- `None` → `null`; `int` → `str(i)`; `float` → `format_float(f)`; `bool` → raise `ProtocolError` (the protocol has no booleans); `str` → `json.dumps(s, ensure_ascii=False)` (the same escapes as serde_json: `"`, `\`, `\n`, `\r`, `\t`, `\b`, `\f`, other control characters as `\u00XX`, nothing else escaped, UTF-8 raw).
- Compact (`pretty=False`): `{"k":v,"k2":v2}`, `[a,b]`, no spaces, no trailing newline.
- Pretty (`pretty=True`), the layout of serde_json's `to_string_pretty`: two-space indent; an empty dict is `{}` and an empty list `[]`; otherwise the opening bracket, then each entry on its own line at one deeper indent as `"key": value` (one space after the colon) or the value, entries separated by `,` at line end, then the closing bracket on its own line at the parent's indent. No trailing newline (callers writing files add one).

**`format_float(x)`** reproduces serde_json's (ryu's) layout from Python's shortest round-trip digits. Refuse a non-finite value (`ProtocolError`). Take `r = repr(x)`; note the sign and strip it; split off an exponent `e` if `r` contains `e` (else `e = 0`); split the mantissa at `.` into `ip` and `fp` (`fp = ""` if no dot); `digits = ip + fp`, `k = e - len(fp)` (so the value is `int(digits) × 10^k`); strip trailing zeros from `digits`, adding one to `k` for each; strip leading zeros. If `digits` is empty the value is zero: return `"0.0"` or `"-0.0"`. Let `length = len(digits)`, `kk = length + k` (the value is in `[10^(kk-1), 10^kk)`). Then, with the sign in front:

1. if `k >= 0` and `kk <= 16`: `digits + "0" * k + ".0"` (`123.0`, `100.0`, `1000000000000000.0`);
2. else if `0 < kk <= 16`: `digits[:kk] + "." + digits[kk:]` (`123456789.12345679`, `2.5`);
3. else if `-5 < kk <= 0`: `"0." + "0" * (-kk) + digits` (`0.1`, `0.0001`, `0.00001`, `0.0000123`, `0.3333333333333333`);
4. else scientific: `digits[0]`, then `"." + digits[1:]` if `length > 1`, then `"e"`, then the exponent `kk - 1` written as `+N` when it is positive and `-N` when negative (`1e-6`, `5e-324`, `1e-300`, `1e+16`, `1.5e+16`, `1.2345678901234568e+16`, `1e+21`, `1.7976931348623157e+308`).

These expected strings were checked against serde_json 1 with `float_roundtrip`; the fixture test below is the authority if they ever disagree.

### `python/policyspace/arithmetic.py`

```python
TOLERANCE = 1e-9   # the engine's production::TOLERANCE

def decide(request: Request) -> Reply
    """The arithmetic rule over a batch: Reply(turn=request.turn, decisions=[decide_business(v) for v in request.views])."""
def decide_business(view: BusinessView) -> Decision
```

Helpers (names are suggestions; keep them small and testable):

- `slices(condition, lifetime) -> float`: `condition * lifetime`, replaced by its rounded value when within `TOLERANCE` of a whole number. `holds_a_slice(condition, lifetime)`: `slices(...) >= 1`. `snap(c)`: `0.0` if `c < TOLERANCE`, `1.0` if `c > 1 - TOLERANCE`, else `c`. `after_wear(condition, lifetime)`: `held = slices(condition, lifetime)`; `snap((held - min(held, 1.0)) / lifetime)`. (Exactly `production.rs`'s `slices`, `snap`, `condition_of` and `wear`.)
- `margin(view, name, technology) -> Verdict`: a small dataclass/tuple with `passes: bool`, the numbers and a trace line. Prices through `view.price(good)` (the currency is 1). If `technology.output`, any key of `inputs`, `capital` or `construction` has no price: `passes = False`, line `"{name}: no posted price for {good}: run down"` (the first unpriced good in the order output, inputs, capital, construction). Otherwise `revenue = capacity × price(output)`; `input_cost = capacity × Σ inputs[g] × price(g)`; `maintenance_cost = Σ (capital[g] + construction[g]) × price(g) / lifetime` (over the union of the two bundles' goods); `margin = revenue − input_cost − maintenance_cost`; `passes = margin >= 0` (break-even runs). Line: `"{name}: revenue {R}; inputs {I}; maintenance {M}; margin {margin}: run and maintain"` or `"...: margin {margin} < 0: run down"`.
- The plan for a factory `id` of technology `t` with condition `c`: if the verdict fails: `run = 0.0, maintenance = 0.0`, line `"factory #{id} ({t}, condition {c}): run down"`. If it passes: `run = 1.0 if holds_a_slice(c, lifetime) else 0.0`; `maintenance = min(1.0 - after_wear(c, lifetime), 1.0 / construction_turns)`; line `"factory #{id} ({t}, condition {c}): run {run}, maintenance {maintenance}"`, with `"shut down"` in place of `"run 0.0"` when it doesn't hold a slice.
- `needs(view, orders: dict[int, FactoryOrder]) -> dict[str, float]`: for each factory, `capacity × run × inputs[g]` plus `maintenance × (capital[g] + construction[g])`, summed per good over every factory. Goods with a need of 0 may be left out.
- `trade(view, needs) -> (offers, bids)`: for every good in the union of `needs` and `view.stockpile` keys: `held = view.held(good)`, `need = needs.get(good, 0)`; a good that is the currency or has no posted price is skipped; `bid = need − held` if positive; `offer = held − need` if positive; an amount below `TOLERANCE` is dropped (rounding specks). All amounts `float`.
- The budget: `budget = cash + Σ offers[g] × price(g)`; `cost = Σ bids[g] × price(g)`. If `cost > budget` (and `cost > 0`): `scale = budget / cost`; every factory's `run` and `maintenance` are multiplied by `scale`; `needs`, `offers` and `bids` are recomputed once from the scaled orders (one pass suffices: scaling needs down can only shrink bids and grow offers, so the new cost is within the new budget, up to rounding; the engine's cash cap in Step 08 is the safety net). Line `"budget {B} = cash {cash} + offers {O}; bids cost {C}: scaled by {scale}"`, else `"...: within budget"`.
- No market (`view.prices` empty): `Decision(business, trace="no market on this hex: nothing to do")` with empty tables, before any of the above.
- The trace is the lines joined by `"\n"`: technology lines in technology-name order, then factory lines in factory-number order (showing the final, scaled fractions), then the budget line. Numbers in trace lines are written with `format(x, ".6g")`. The exact wording may vary slightly, but every line's numbers, and the phrases `run and maintain`, `run down`, `shut down`, `scaled by`, `within budget` and `no market` are relied on by tests.
- Output: `Decision(business=view.business, factories={id: FactoryOrder(run, maintenance) for every factory in the view, in factory-number order}, offers, bids, trace)`.

### `python/policyspace/deciders.py` and `__main__.py`

```python
DECIDERS: dict[str, Callable[[Request], Reply]] = {"arithmetic": arithmetic.decide}

def serve(decider: Callable[[Request], Reply], stdin: TextIO, stdout: TextIO) -> None:
    """For each line of stdin (blank lines skipped): read_request, decider, write one compact
    reply line, flush. Returns at end of input. Raises ProtocolError for a line that isn't a request."""
```

`__main__.py`: `main(argv: list[str]) -> int`, called as `sys.exit(main(sys.argv[1:]))`.
- Not exactly one argument: usage `usage: python -m policyspace <decider>` and the known names to stderr, return 2.
- Unknown name: `error: unknown decider 'x' (known: arithmetic)` to stderr, return 2. Nothing is read from stdin before this check.
- Otherwise `sys.stdin.reconfigure(encoding="utf-8")`, `sys.stdout.reconfigure(encoding="utf-8", newline="\n")` (so Windows doesn't write `\r\n`), then `serve(...)`. A `ProtocolError` is written to stderr (`error: ...`) and returns 1. End of input returns 0.

### `crates/engine/tests/recordings/` and `recordings.rs`

The Rust tests (`mod common;`, the engine's public API only):

- `recordings_dir()`: `Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recordings")`. `updating()`: `std::env::var_os("POLICYSPACE_UPDATE_RECORDINGS").is_some()`.
- `sample_request(world: &World) -> Request`: `turn: world.turn()`, one `world.business_view(id)` per business in `world.businesses().iter()` order (ID order: `center_farm`, `center_mine`, `center_toolmaker`).
- `the_sample_recording_matches_the_engine`: build the sample world and request. If updating: the reply is the existing file's reply if the file reads as a `Recording`, else `Reply { turn: request.turn, decisions: vec![] }`; write `Recording { request, reply }.to_json_pretty() + "\n"` to `sample_turn0.recording.json`. Then (always) read the file (a missing file fails with a message naming the two regeneration steps); `Recording::from_json` succeeds; `recording.request == request`; the file's text equals `recording.to_json_pretty() + "\n"` (canonical form); `recording.reply.validate(&request, &world.rules().goods)` is `Ok` with every answer `Answer::Valid`; the reply has exactly one decision per view, in the request's order (same business names). When updating with a stale or empty reply this last check fails: that is the signal to run the Python step.
- `the_exactness_request_is_the_engines`: a hand-built `Request { turn: Turn::MAX, views: vec![view] }` where `view` is `BusinessView { turn: Turn::MAX, business: "exact", currency: "coin", cash: q(f64::MAX), prices: {a: 1e-5, b: 1.23e-5, c: 1e-6, d: 1e-4, e: 1e16, f: 1e15, g: 1.5e16, h: 123.0, i: 1e21, j: 12345678901234567.0, k: 0.1, l: 100.0, m: 5e-324, n: 1.0/3.0, o: 123456789.123456789, p: 1e-300}, stockpile: {coin: [(f64::MAX, None)], a: [(5e-324, Some(Turn::MAX)), (0.0, Some(Turn::ZERO))]}, factories: {0: (t, 1.0/3.0), u32::MAX: (t, 0.0)}, technologies: {t: inputs {a: 2.5}, output b, capacity 1e-5, capital {c: 7.0}, construction {}, construction_turns u32::MAX, lifetime 1}, deposits: {z: (count u32::MAX, produces a, output 0.0)} }`. `text = request.to_json() + "\n"`. If updating, write it to `exactness.request.json`; always read the file and assert it equals `text`.
- `hand_written_requests_parse`: every `*.request.json` in the folder, in name order (`exactness` included): `Request::from_json` succeeds, `views` is non-empty, every view's `turn` equals the request's.

`README.md` in the folder: the three kinds of file (the sample recording, written half by each side; the exactness request, written by the engine, compact; the hand-written requests, pretty, edited by hand), who reads each, and the regeneration steps:

```
set POLICYSPACE_UPDATE_RECORDINGS=1      (PowerShell: $env:POLICYSPACE_UPDATE_RECORDINGS = "1")
cargo test -p policyspace-engine --test recordings     # rewrites the engine's halves
.venv/Scripts/python -m pytest python/tests            # rewrites the rule's reply
cargo test -p policyspace-engine --test recordings     # confirms
```

The hand-written requests, all at turn 5 with currency `coin`, each with one view, pretty JSON (every quantity as a float with `.0`, every table in sorted order; `deposits: {}` unless stated). Two technologies are reused:

- `smelter`: inputs `{labor: 1.0, ore: 2.0}`, output `metal`, capacity 10.0, capital `{tools: 2.0}`, construction `{labor: 4.0}`, construction_turns 2, lifetime 10.
- `bakery`: inputs `{flour: 1.0, labor: 1.0}`, output `bread`, capacity 10.0, capital `{oven: 1.0}`, construction `{labor: 2.0}`, construction_turns 2, lifetime 4 (lifetime **10** in `revival`).

| File | Business | Prices | Stockpile (cash = coin) | Factories | Expected (asserted in Python) |
|---|---|---|---|---|---|
| `negative_margin` | `loser` | labor 1, metal 4, ore 3, tools 10 | coin 20 (never), metal 3 (never), ore 5 (perishes 7) | `0`: smelter, 1.0 | margin 40 − 70 − 2.4 < 0: factory 0 run 0, maintenance 0; offers `{metal: 3, ore: 5}`; no bids; trace has `run down` |
| `unpriced_input` | `stuck` | labor 1, metal 4, tools 10 | same | `0`: smelter, 1.0 | factory 0 at 0/0; offers `{metal: 3}` only (ore unpriced); trace names `ore` |
| `cash_short` | `strapped` | bread 5, flour 1, labor 1, oven 20 | coin 25.5 | `0` and `1`: bakery (lifetime 4), 1.0 each | margin 50 − 20 − 5.5 ≥ 0; plan run 1, maintenance 0.25 each; needs flour 20, labor 21, oven 0.5, cost 51 > budget 25.5; scale 0.5: both factories run 0.5, maintenance 0.125; bids `{flour: 10, labor: 10.5, oven: 0.25}` (cost 25.5); no offers; trace has `scaled by 0.5` |
| `revival` | `reviver` | as `cash_short` | coin 100 | `0`: bakery (lifetime 10), 0.05 | margin 50 − 20 − 2.2 ≥ 0; 0.05 × 10 = 0.5 slices: no run; after wear 0; maintenance min(1, 1/2) = 0.5; factory 0 run 0, maintenance 0.5; bids `{labor: 1, oven: 0.5}`; trace has `shut down` |
| `no_market` | `remote` | (none) | coin 10, metal 3 | `0`: smelter, 1.0 | empty factories, offers and bids; trace has `no market` |
| `seller` | `trader` | grain 2, metal 4 | coin 5, grain 4 (perishes 6), metal 1 (perishes 6) + 2 (never); deposits `{field: count 1, produces grain, output 4.0}` | none (empty `factories` and `technologies`) | offers `{grain: 4, metal: 3}`; no bids; empty factories; trace has `within budget` |

### `tools/check.py`

`CHECKS` gains `("pytest", [sys.executable, "-m", "pytest", "python/tests"])` after `cargo test`. Everything else (run all, summary, exit code, the cargo precheck) stays.

## Tasks

1. `python/pyproject.toml`, `python/policyspace/__init__.py`, `requirements.txt`, `.gitignore`. From the project root run `.venv/Scripts/python -m pip install -r requirements.txt` (fetches `pytest` and the build backend once) and check `.venv/Scripts/python -c "import policyspace"` works from another directory.
2. `protocol.py`: `ProtocolError`, `loads`, `format_float`, `dumps`, the dataclasses, `to_data`/`from_data`, the `read_*`/`write_*` functions.
3. `python/tests/conftest.py` and the `test_protocol.py` tests that don't need fixture files (the `format_float` table, refusals, defaults, sorted tables, the empty decision's text).
4. `crates/engine/tests/recordings/README.md`, the six hand-written request files, and `crates/engine/tests/recordings.rs`. Run `cargo test -p policyspace-engine --test recordings` with `POLICYSPACE_UPDATE_RECORDINGS` set once: it writes `exactness.request.json` and `sample_turn0.recording.json` (with an empty reply; the reply check fails until Task 6). Then the remaining `test_protocol.py` tests (exactness, the recording's pretty round trip).
5. `arithmetic.py` and `test_arithmetic.py` (sample expectations written from the numbers in the Test Plan; the hand-written cases; the in-code cases).
6. With `POLICYSPACE_UPDATE_RECORDINGS` set, run `.venv/Scripts/python -m pytest python/tests`: the sample test writes the reply into the recording. Unset it; `cargo test -p policyspace-engine --test recordings` and `pytest` must both pass.
7. `deciders.py`, `__main__.py`, `test_loop.py`.
8. `tools/check.py`. Run `.venv/Scripts/python tools/check.py` until it passes (`cargo fmt`, clippy, every Rust test, every Python test).

## Test Plan

Run everything with `.venv/Scripts/python tools/check.py`; the Python tests alone with `.venv/Scripts/python -m pytest python/tests`; the Rust fixture tests alone with `cargo test -p policyspace-engine --test recordings`. In Python compare numbers exactly (`==`) where the spec's arithmetic is exact (whole and half values, powers of two) and with `pytest.approx(abs=1e-9)` elsewhere. Test names are suggestions.

### `python/tests/test_protocol.py`

- **AC 5 (lossless).** `the_exactness_request_comes_back_byte_for_byte`: `text = exactness.request.json`; `write_request(read_request(text)) == text.rstrip("\n")`. `the_sample_recording_round_trips`: `r = read_recording(text)`; `write_recording(r, pretty=True) + "\n" == text`; `read_recording(write_recording(r)) == r`; and for each view and decision in it, `read_view(write_view(v)) == v`, `read_decision(write_decision(d)) == d`.
- **AC 5 (the formatter).** `format_float_matches_the_engine`: the table in Interfaces & Data (`1e-5` → `0.00001`, `1.23e-5` → `0.0000123`, `1e-6` → `1e-6`, `1e-4` → `0.0001`, `1e16` → `1e+16`, `1e15` → `1000000000000000.0`, `1.5e16` → `1.5e+16`, `123.0` → `123.0`, `1e21` → `1e+21`, `12345678901234567.0` → `1.2345678901234568e+16`, `0.1` → `0.1`, `100.0` → `100.0`, `5e-324` → `5e-324`, `1/3` → `0.3333333333333333`, `123456789.123456789` → `123456789.12345679`, `1e-300` → `1e-300`, `0.0` → `0.0`, `-0.0` → `-0.0`, `-2.5` → `-2.5`, `1.7976931348623157e308` → `1.7976931348623157e+308`); `inf` and `nan` raise `ProtocolError`. `dumps_writes_ints_strings_and_tables`: `dumps({"turn": 18446744073709551615, "s": "a\"b\\c\nd", "l": [], "d": {}})` == `{"turn":18446744073709551615,"s":"a\"b\\c\nd","l":[],"d":{}}` (with the JSON escapes); pretty form of `{"a": [1, {"b": None}], "c": {}}` equals the serde_json layout written out in the test; `dumps(True)` raises.
- **Reading is strict.** `reading_refuses_bad_text`: each of these raises `ProtocolError`: `{"business": `; `{"business":"a","offer":{}}` (unknown key); `{"turn":4}` as a view (missing keys); `{"business":"a","offers":{"g":1,"g":2}}` (duplicate); `{"business":"a","offers":{"g":NaN}}`; `{"business":"a","factories":{"x":{}}}`; `{"business":"a","factories":{"0":{"run":true}}}`; `{"turn":"4","decisions":[]}` as a reply; `{"turn":4.0,"decisions":[]}`.
- **Defaults and order.** `decision_defaults_and_empty_form`: `read_decision('{"business":"a"}') == Decision.empty("a")`; `read_decision('{"business":"a","factories":{"0":{"run":1}}}').factories[0] == FactoryOrder(1, 0.0)`; `write_decision(Decision.empty("a")) == '{"business":"a","factories":{},"offers":{},"bids":{}}'`; `read_reply('{"turn":4}') == Reply(4, [])`; a decision with `trace="x"` writes `,"trace":"x"}` at the end. `tables_are_written_sorted`: a decision built with factories `{10: .., 2: ..}` and offers `{"b":1.0,"a":2.0}` writes `"factories":{"2":...,"10":...}` and `"offers":{"a":2.0,"b":1.0}`.

### `python/tests/test_arithmetic.py`

- **AC 2 (the sample).** `the_sample_reply_is_the_recordings`: `rec = read_recording(...)`; `reply = decide(rec.request)`; if `POLICYSPACE_UPDATE_RECORDINGS` is set, write `write_recording(Recording(rec.request, reply), pretty=True) + "\n"` to the file first; then `reply == rec.reply` (exact), `reply.turn == 0`, decisions named `center_farm`, `center_mine`, `center_toolmaker` in that order. `the_sample_numbers`: farm: `factories == {0: FactoryOrder(1.0, 0.125)}`, `bids == {"tools": 0.5, "unimproved_grain": 4.0, "unskilled_labor": 7.0}`, `offers == {}`; mine: `{1: FactoryOrder(1.0, 0.375)}`, bids `{"tools": 0.75, "unskilled_labor": 4.0}`, no offers; toolmaker: factory 2 run 1.0, maintenance `== min(1.0 - after_wear(1.0, 24), 1/3)` and `approx(1/24)`, bids copper 8.0, labor `approx(6.25)`, tools `approx(0.125)`, no offers; every trace contains `run and maintain` and `within budget`; the toolmaker's trace contains `margin 0.25`.
- **AC 3.** `a_negative_margin_runs_down`, `an_unpriced_input_runs_down`, `a_shut_down_factory_is_revived_but_not_run`, `no_market_means_an_empty_decision`, `a_business_with_no_factories_only_sells`: the expectations in the table above, from the files. `an_unpriced_bundle_good_runs_down`: the `unpriced_input` view with `ore` priced again but `tools` removed from `prices` (built in code with `dataclasses.replace`): factory 0 at 0/0, trace names `tools`. `the_currency_in_a_bundle_is_priced_at_one_and_never_bid`: the `revival` view with construction `{coin: 2.0}` instead of labor: margin counts 2 × 1 / 10, bids are `{oven: 0.5}` only.
- **AC 4.** `a_cash_short_plan_is_scaled_uniformly`: from `cash_short`: both orders `FactoryOrder(0.5, 0.125)`; bids `{flour: 10.0, labor: 10.5, oven: 0.25}`; `Σ bids × price <= cash + Σ offers × price + 1e-9`; trace contains `scaled by 0.5`.
- **Invariants.** `every_decision_is_well_formed`: for the sample and every hand-written request, for every decision: no offer or bid names the currency or an unpriced good; every amount `> 0`; every fraction in `[0, 1]`; the factory keys equal the view's unless the view has no prices; `trace` is a non-empty string.

### `python/tests/test_loop.py`

- **AC 1.** `the_script_answers_each_request_line`: `subprocess.run([sys.executable, "-m", "policyspace", "arithmetic"], input=line + b"\n\n" + line + b"\n", capture_output=True)` where `line` is the sample request compact (`write_request(rec.request).encode()`): return code 0, `stdout == expected + b"\n" + expected + b"\n"` with `expected = write_reply(rec.reply).encode()` (so no `\r`, and the blank line was skipped), `stderr == b""`. `an_unknown_decider_fails_at_startup`: name `bogus`, no stdin: return code 2, stderr mentions `bogus` and `arithmetic`. `no_argument_prints_usage`: return code 2, stderr contains `usage`. `a_bad_line_fails`: input `b"not json\n"`: return code 1, stderr non-empty, stdout empty.

### `crates/engine/tests/recordings.rs`

- **AC 6.** `the_sample_recording_matches_the_engine`, `the_exactness_request_is_the_engines`, `hand_written_requests_parse` as specified above.

### Existing tests

- **AC 7.** Every existing Rust test passes unchanged; `tools/check.py` reports four checks passed.

## Constraints

- Python: standard library only at runtime; `pytest` only in tests. Python 3.12+ (the venv is 3.14). No `print` in the package except the loop's stdout. Module and function docstrings in the same plain style as the Rust docs. Keep type hints on public functions. Lines up to 100 columns; no formatter or linter is added in this Step.
- `protocol.py` never changes a number it read (no `float(int)`, no rounding); `arithmetic.py` writes only `float`s.
- Dict iteration order is never relied on for output: `to_data` sorts tables, and the rule iterates views in request order, technologies and factories in sorted order.
- Rust: the existing conventions (doc comments where clippy wants them, `cargo fmt`, `BTreeMap`); the fixture tests use only the engine's public API and `tests/common`. Nothing under `crates/engine/src/` changes; `crates/engine/tests/decider.rs` doesn't change.
- Do not change the sample setting, any test setting, the dev UI, `CLAUDE.md`, `docs/` or `manual.ipynb` (docs are drafted later in the loop; `docs/workspace.md`'s check-command section becomes stale and `step-docs-draft` will propose the fix).
- Out of scope: starting the script from the engine, keeping it alive, timeouts, failure semantics (Step 07); resolving decisions, the cash cap, the per-business record (Step 08); any strategy beyond the arithmetic rule; vectorization; packaging beyond the editable install.

## Done When

All tasks complete, `.venv/Scripts/python tools/check.py` passes with four checks, every test in the Test Plan is written and passes, `sample_turn0.recording.json` and `exactness.request.json` are committed in their canonical forms, and every Acceptance Criterion in `design.md` is met.
