# Step 07: decider-host — Manual Changes

The developer UI and the checker's `--turns` now start the setting's business deciders as Python processes and ask them what to do every turn. What businesses do in the world doesn't change yet: their decisions are checked and stored but nothing acts on them until Step 08, so the turn reports, the panels and the sample's output are unchanged. The visible changes are the Python requirement, a new kind of load error, and decider failures in the turn log. This builds on Step 02's draft (the `decider` key) and Step 06's (the Python package and the arithmetic rule), and changes parts of both (see below).

## What users can now see or do

- **Loading a setting with businesses starts its deciders.** For each decider name its businesses carry (the sample has one, `arithmetic`), the developer UI starts `python -m policyspace <name>` with the project's Python environment and keeps it running while that setting is loaded. Reload, opening another folder and closing the window end those processes, and a reload starts fresh ones. Settings without businesses start nothing and don't need Python.
- **The Python environment is now needed to run the developer UI on the sample**, and for `cargo test` / `tools/check.py` (some Rust tests run the real decider). Without it, the load fails with a message saying where the environment was expected and the two setup commands that create it.
- **A decider that can't start is a load error.** The most likely cause for a setting author is a misspelled `decider` name: the Python package doesn't know it, so the window shows the error view (as for a broken setting) instead of the world, with a message like:

  ```
  decider 'rule': '...\.venv\Scripts\python.exe -m policyspace rule' exited (exit code: 2) as soon as it started
  its error output ends with:
    error: unknown decider 'rule' (known: arithmetic)
  1 error(s) found in crates/engine/tests/settings/production_valid
  ```

  The same lines go to stderr from `dev-ui check <folder> --turns N`, which then runs no turns and exits with code 1. **Plain `check <folder>` (without `--turns`) doesn't start the deciders, so it doesn't catch a misspelled decider name; `--turns 0` does.** (The status bar's error count counts every line of such a message, so it can say more than 1 error.)
- **A decider that fails during a turn doesn't stop the world.** If the script crashes, takes too long to answer (more than 5 seconds), or answers something that isn't a valid reply, the businesses it controls do nothing that turn, the turn goes on, and the turn log (and `check --turns` output) shows a line right after that turn's line, then the last lines the script wrote to its error output:

  ```
  Turn 3: decider 'arithmetic' failed: the process exited (exit code: 1) before answering
      stderr: Traceback (most recent call last):
      stderr: ...
  ```

  The script is started again at the next step; if it keeps failing, a line appears every turn. A failure during turns doesn't change the checker's exit code. A single business's decision that breaks the rules (a factory it doesn't own, a good with no price) is not a failure line: that business just does nothing that turn (Step 09 will show these).
- **Two environment variables for developers**: `POLICYSPACE_PYTHON` points the developer UI and the tests at another Python interpreter, and `POLICYSPACE_DECIDER_TIME_LIMIT` sets the time limit in seconds (`0` for none, useful when stepping through a decider in a debugger). Printing to standard error (`print(..., file=sys.stderr)`) is the way to debug a decider, since those lines appear in the failure lines.

## Changes to earlier drafts

- **Step 02's `decider` key** ("for now it is only stored") and **Step 06's** rewording ("the engine still doesn't check the name or run it") are both out of date. The key now names the Python decider the developer UI starts for the business; several businesses can share one; a name the project's Python package doesn't know is a load error in the window and in `check --turns`. `arithmetic` is still the only decider shipped.
- **Step 06's "How the arithmetic rule decides"** should no longer say the rule "doesn't run yet". It runs every turn now, but its decisions still have no effect until Step 08, so the manual should still present the behavior as how businesses *will* act (trades and factory runs don't appear in any report or panel yet).

## Where it goes

- **"Project overview"**: where Step 06's draft adds the setup commands, add that the developer UI needs the environment to load settings with businesses (the sample included) and that the Rust tests need it too. Mention the two environment variables briefly, or leave them to the docs link. Link [docs/workspace.md](docs/workspace.md#the-python-environment).
- **"Businesses" subsection** (Step 02's, under "Writing a setting"): update the `decider` key as above, and add `a misspelled decider name` to the errors, noting it is caught by `--turns` and the window, not by plain `check`.
- **"Checking a setting"**: add the decider-start failure to the `--turns` description and the exit-code table's meaning of 1 ("the setting has errors, or its deciders couldn't start"), with the plain-`check` caveat and the example message above.
- **"The developer UI"**: under "The world panel", the Turn log bullet gains the failure lines (with the example); under "The author's loop", say that a decider that won't start shows in the error view like setting errors, and that Reload restarts the deciders, so an edited Python decider takes effect on Reload. Remove nothing from "Not available yet".
- **"Turn reports"**: no change; the report lines are unchanged.
- Link [docs/decider-host.md](docs/decider-host.md) for how the exchange, the time limit and the failure handling work, and [docs/python.md](docs/python.md) for the decider script itself.

## Example

The two text examples above (the load error for an unknown name and a failure line in the turn log) are enough; no diagram or interactive cell is needed. An executed cell that starts Python would make `nbconvert --execute` depend on the project's environment and the engine build, so don't add one.
