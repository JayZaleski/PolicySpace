# The Python package and its deciders

Source: `python/` — `pyproject.toml` (the project file), `policyspace/__init__.py` (package docstring), `policyspace/protocol.py` (the messages, the strict reader, the engine-style writer), `policyspace/arithmetic.py` (the arithmetic rule), `policyspace/deciders.py` (the registry of decider names and the line-per-turn loop) and `policyspace/__main__.py` (the command line). Tests are in `python/tests/`. The recorded messages the tests read are in `crates/engine/tests/recordings/` (see [decider.md](decider.md#recordings)).

`policyspace` is the project's Python side: a client for the decider protocol and the deciders the project ships. A decider is a function from one turn's batch of views (a `Request`) to a batch of decisions (a `Reply`), exactly as [decider.md](decider.md) defines them. The package can run a decider as a long-lived process that answers one request line with one reply line per turn.

**The engine runs it.** The dev UI and `dev-ui check --turns` start `python -m policyspace <name>` once for each decider name the setting's businesses carry, with the virtual environment's interpreter, and exchange one line per turn with it at the decision point (see [decider-host.md](decider-host.md)). The engine validates and stores the decisions, then acts on the valid ones: the offers and bids go through the hex's trade round and the fractions run and maintain the factories (see [trade.md](trade.md)).

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

The engine finds the interpreter at `.venv/Scripts/python.exe` (Windows) or `.venv/bin/python` under the project root, or at the path in `POLICYSPACE_PYTHON` (see [decider-host.md](decider-host.md#finding-python-deciderpython)). So the dev UI needs the environment to load a setting with businesses, and `cargo test --workspace` needs it too.

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

Standard input and output are reconfigured to UTF-8, and output to `\n` line endings, so the process writes the same bytes on Windows as elsewhere. Nothing else in the package prints. The engine keeps the process alive across turns, waits for each reply with a time limit (5 s unless `POLICYSPACE_DECIDER_TIME_LIMIT` says otherwise), and kills and restarts a process that exits, answers late or answers something that isn't a reply; its businesses do nothing that turn (see [decider-host.md](decider-host.md#failures)). Messages written to standard error are not lost: the engine keeps the last 20 lines and quotes them when the process fails, so `print(..., file=sys.stderr)` is the way to debug a decider. Exit code 2 for an unknown name, which happens before any input is read, makes the engine's start of the process fail, so a misnamed decider in a setting is a load error in the dev UI and the checker.

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

The slice arithmetic mirrors `production.rs` exactly, so "holds a slice" and the condition after wear are the engine's numbers: `slices(condition, lifetime)` is the same rule as the engine's public `slices_held` (condition × lifetime, rounded when within `TOLERANCE = 1e-9` of a whole number), and `holds_a_slice`, `snap` (to 0 or 1 within the tolerance) and `after_wear` (one slice lost, stopping at zero) follow the engine's private helpers. The margin test counts only the steady slice, not a catch-up, so a factory below full is restored on the strength of its steady-state margin.

### Offers and bids

`needs(view, orders)` sums, per good, capacity × run × inputs[g] for each factory plus maintenance × (capital[g] + construction[g]). `trade(view, needs)` compares each good's need with what the business holds (`view.held`, over every perish group): it **bids the shortfall and offers the surplus**. The currency and goods with no posted price are skipped, and amounts below `TOLERANCE` are dropped, so every decision passes the engine's validation. Offering every priced surplus means the business sells last turn's output and unneeded unimproved goods without having to tell them apart, and a business with nothing to run still sells what it holds.

The rule's bids should match exactly what its fractions use, since anything bought and not used stays in the stockpile, and labor left over perishes at the end of the turn. At present the toolmaker's maintenance fraction comes out a hair under 1/24 while its labor bid is for the full slice, so about 2.2e-16 `unskilled_labor` perishes each turn and prints on the sample's turn lines (logged as bug 01 in `PROJECT_PLAN/bugs.md`; the fix belongs in the rule).

### The cash budget

Sales settle before purchases and the dummy market buys everything offered, so before buying the business will have

budget = cash + Σ offers[g] × price(g).

If the bids cost more than that, the rule multiplies **every factory's run and maintenance fractions by one common proportion**, budget / cost, and recomputes needs, offers and bids once from the scaled orders. Scaling needs down can only shrink bids and grow offers, so one pass lands within the budget, up to rounding. One proportion for every factory and both fractions mirrors the factory mechanics' proportional sharing of a shortage. The engine's cash cap is the safety net, not the normal path: if the bids still cost more than the cash after the sales, the engine scales every bid by one proportion and the factories are cut by the resulting shortage (see [trade.md](trade.md#the-cash-cap)).

**A known gap:** if a setting puts the hex's currency itself in a technology's inputs or bundles, the margin test prices it at 1, but the budget doesn't reserve that cash, because the currency is never bid. The sample never does this, and the engine's cash cap covers it. A later rule could subtract the currency's own need from the budget.

### The trace

Every decision carries a trace, the only account of why the business did what it did. The dev UI shows it, collapsed, in each business's section on the hex panel. Its lines are joined by `\n`: one line per technology in name order (the margin arithmetic and the verdict), one per factory in number order (its final, possibly scaled fractions), then the budget line. Numbers are formatted with `.6g`. The farm in the sample at turn 0:

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

The engine's side of the same process is tested in `crates/engine/tests/external_decider.rs` and `business_trade.rs`, and the dev UI's in its `session.rs`, `text/businesses.rs` and `checker.rs` tests, all of which start the real `arithmetic` decider from the virtual environment (see [decider-host.md](decider-host.md#tests)).

## Extending

- **A new decider**: write a function from `Request` to `Reply` in its own module, add it to `DECIDERS` under the name businesses will carry, and test it against recordings. Keep the shape batch in, batch out, and build decisions through `protocol`'s dataclasses so they are written in the engine's form. Answer every business in the request, in order, with an empty decision where there is nothing to do. The engine needs no change: a business naming the new decider makes the dev UI and the checker start `python -m policyspace <name>` for it.
- **A change to the protocol** (a new field or verb in [decider.md](decider.md#extending)): add the field to the dataclass, its reader and writer in the same key position as the Rust struct, and regenerate the recordings. The round-trip tests fail until both sides agree.
- **A runtime dependency**: avoid one if the standard library will do. If one is needed, add it to `pyproject.toml`'s `dependencies`.

## Not yet

Strategy beyond the arithmetic rule (ranking factories by margin, protecting maintenance over running, holding inventory, memory between turns) and vectorized or parallel evaluation of a batch are not planned yet. The process is kept alive across turns, so a decider could remember between turns, but none does, and a process restarted after a failure would lose that memory. The rule assumes every offer sells; once Stage 04's live market breaks that, the rule still produces valid decisions, and the engine trims what can't be done.
