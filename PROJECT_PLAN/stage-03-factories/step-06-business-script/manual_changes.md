# Step 06: business-script — Manual Changes

Nothing in the turn, the turn reports, the setting format or the developer UI changes: the arithmetic rule exists but the engine doesn't run it until Step 07, and its decisions have no effect until Step 08. The visible changes are to the project's setup and check command, plus an optional preview of how the sample's businesses will decide. This builds on Step 02's draft (the businesses' `decider` key) and Step 05's (no user-facing change).

## What users can now see or do

- **The project now has a Python part.** Beside the Rust engine and developer UI there is a Python package, `policyspace`, in `python/`. It holds the business deciders: the programs that will choose what each business does every turn. The engine will start them as a separate process and exchange one message per turn with them, which is why a decider can be written in Python at all.
- **Setup installs it.** The setup commands don't change (`python -m venv .venv`, then `.venv/Scripts/python -m pip install -r requirements.txt`), but they now also install pytest and the package. The first install needs network access once. Python 3.12 or later is needed.
- **The check command runs the Python tests too.** `tools/check.py` now runs four checks: formatting, lints, the Rust tests and the Python tests. Run it with `.venv/Scripts/python tools/check.py`, since the Python tests run under whichever Python starts the script. If pytest is missing, rerun the setup command.
- **The decider `arithmetic` now has a meaning.** Step 02's draft says a business's `decider` key "is only stored; a later Step connects it". The name `arithmetic`, which the sample's businesses carry, is now the project's first decider. The engine still doesn't check the name or run it (Step 07 does), so that sentence should say the name will be matched against the deciders the project ships, of which `arithmetic` is the only one so far.
- **How the arithmetic rule decides**, in plain terms, for setting authors choosing prices and technology numbers. Each turn, for each kind of factory a business owns:
  - it checks whether one turn at full capacity pays at the hex's posted prices: what the output sells for, minus the inputs, minus the steady upkeep (one turn's share of the factory's capital and construction goods);
  - if it pays, or breaks even, the factory runs at capacity and is maintained back to full condition; if it doesn't, or if any of the goods involved has no price on the hex, the factory is left idle and unmaintained, and slowly runs down;
  - the business sells everything it holds that its plan doesn't need and buys what the plan needs that it doesn't hold;
  - if it can't afford those purchases with its cash plus what its sales bring in, it scales all its factories down by the same proportion;
  - a business on a hex without a market does nothing.

  Every decision comes with a short written explanation (the "trace") of this arithmetic, which Step 09 will show in the developer UI. Until Steps 07 to 09 none of this is visible in a running world, so the manual should present it as how businesses *will* behave.

## Where it goes

- **"Project overview"**: mention the Python package beside the engine and developer UI, update the `tools/check.py` bullet to say it also runs the Python tests and to recommend `.venv/Scripts/python tools/check.py`, and give the setup commands once (they are currently only in `CLAUDE.md`, but anyone running the check command needs them). Link [docs/workspace.md](docs/workspace.md#the-python-environment) and [docs/python.md](docs/python.md).
- **The "Businesses" subsection** (Step 02's new subsection under "Writing a setting"): update the `decider` key's description as above, and add a short paragraph "How the arithmetic rule decides" with the bullets above, or a short separate subsection right after "Businesses" if that reads better. Say plainly that it doesn't run yet. Link [docs/python.md](docs/python.md#the-arithmetic-rule-arithmeticpy) for the exact rule.
- **"Technical documentation"**: no change needed; `docs/README.md` lists `python.md`.

## Example

A worked example would help setting authors see why prices matter: the sample's farm at turn 0. Grain sells at 2, so 12 grain bring 24; the inputs (12 unimproved grain at 0.5 and 6 labor at 1) cost 12; the upkeep (4 tools at 12 and 8 labor at 1, spread over the farm's 8-turn lifetime) costs 7 per turn. The margin is 5, so the farm runs. It holds 8 unimproved grain from its fields, so it buys 4 more, plus 7 labor and half a tool, for 15 of its 100 silver. The toolmaker shows the opposite edge: its margin is only 0.25, so a small change in the price of copper or tools would idle it. The manual could show the farm's trace as text:

```
farm: revenue 24; inputs 12; maintenance 7; margin 5: run and maintain
factory #0 (farm, condition 1): run 1, maintenance 0.125
budget 100 = cash 100 + offers 0; bids cost 15: within budget
```

An optional interactive cell could compute the margin test for each of the sample's three technologies from `settings/sample/rules/technologies.toml` and `start/markets.toml`, showing which factories would run and how close each is to breaking even. If it does, it must run under `nbconvert --execute` with nothing beyond the standard library and the installed packages (it may import `policyspace`, which is installed in the environment).
