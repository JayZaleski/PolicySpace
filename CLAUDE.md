# CLAUDE.md

## Project Plan (`PROJECT_PLAN/`)

The `PROJECT_PLAN/` folder holds the current canonical plan for the project, organized into **Stages** and **Steps**.

- **Stages** develop a single system of the game up to the point where it becomes dependent on a different system. Each Stage is broken into Steps.
- **Steps** design and implement one technical feature of a system. Each Step has its own folder containing:
  - `design.md` — a plain-English description of the step.
  - `spec.md` — instructions for an AI agent to build the feature into the code.

Stage and Step folders follow this naming pattern (two-digit numbers, kebab-case names):

```
PROJECT_PLAN/
  stage-01-<name>/
    consolidate.md          # written by comp-review at the end of the Stage
    step-01-<name>/
      design.md
      spec.md
      docs_changes.md       # proposed changes to docs/
      manual_changes.md     # proposed additions to the manual
    step-02-<name>/
      ...
  stage-02-<name>/
    ...
  bugs.md                   # the bug log (see Bug fixes)
  bugs/
    bug-01-<name>/
      fix.md                # the plan build follows to fix the bug
```

Top-level plan files:

- `vision.md` — the project's overarching goals.
- `roadmap.md` — a summary of the Stages and the Steps in each, with a description of each Step and tags that help agents find related Steps that may contain useful context.
- `bugs.md` — the log of bugs found in code that's already built, and their status (see [Bug fixes](#bug-fixes)).
- `scratchpad.md` — a live record of the agent's interpretation of the current discussion. It is empty at the start of a session. During the session, the agent fills it with bullet points so the user can see how the discussion is being interpreted and correct it. The agent empties it once the session's edits are finished, so it doesn't confuse future agents.

### Roadmap format

```
## Stage 01: <name>
- Status: in-progress
- Goal: <one or two sentences>

### Step 01: <name>
- Status: designed
- Tags: #economy #taxes
- Description: <a few sentences>
```

Only the next Stage is broken into Steps. Later Stages are outlines (name, status, and goal only) until `project-plan` details them.

- **Stage status:** `outline` (not yet broken into Steps) → `planned` (Steps defined) → `in-progress` → `done`.
- **Step status:** `planned` → `designed` (`design.md` written) → `specified` (`spec.md` written) → `built` (code built and approved by the user) → `done` (docs drafted and committed).

Stage and Step names in `roadmap.md` match their folder names (e.g. `### Step 01: tax-collection` ↔ `step-01-tax-collection/`). Outline Stages have no folder. A status needs its files: `designed` needs `design.md`, `specified` needs `spec.md`, a `done` Step needs `docs_changes.md` and `manual_changes.md`, and a `done` Stage needs `consolidate.md`. `.venv/Scripts/python tools/export_status.py --check` checks all of this, and `update-status` runs it before every commit.

### Tags

Steps in `roadmap.md` are tagged using `#tag` format (lowercase, kebab-case, e.g. `#economy`, `#save-system`). Tags are an open list: reuse existing tags where they fit, and create new ones when no existing tag applies. To find related Steps, search `roadmap.md` for a tag.

## Documentation

- `docs/` — Markdown files explaining how the code works, for developers who want to modify the project. `docs/README.md` indexes them.
- `manual.ipynb` — the user manual, a Jupyter notebook. It explains the project to users and points to the technical documents in `docs/` where appropriate. `manual.html` is generated from it with `.venv/Scripts/python -m jupyter nbconvert --to html --execute --no-input manual.ipynb`. Edit the notebook, never the HTML, and regenerate the HTML after every edit. Set up the virtual environment with `python -m venv .venv` and `.venv/Scripts/python -m pip install -r requirements.txt`.

## Status Dashboard

A dashboard on the home server shows the project's progress. It reads `Z:\Project-Tracking\PolicySpace\status.json`, a snapshot of the Stages, Steps, and statuses in `roadmap.md`. `tools/export_status.py` generates the file:

- A hook in `.claude/settings.json` runs it whenever `roadmap.md` is edited, so every status change reaches the dashboard.
- `update-status` runs it again after committing, to record the new commit.
- It parses `roadmap.md`, so keep the roadmap in the format above. If the hook reports a format error, fix `roadmap.md`.
- If the shared drive isn't available, it prints a warning and the work continues. The next export catches up.
- Never edit `status.json` by hand.

## Development Loop

Work proceeds Stage by Stage, and within each Stage, Step by Step. **Skills** are invoked by the user. **Subagents** are deployed by the agent; where noted, the agent must get the user's permission before deploying.

### For each Stage

1. User launches a fresh session.
2. **Brainstorm** — user invokes the `brainstorm` skill to edit `vision.md`.
3. **Update the Project Plan** — user invokes the `project-plan` skill to update `roadmap.md` based on changes in `vision.md`.
4. **Update Status** — agent deploys the `update-status` subagent to commit git changes and update the status dashboard. *Requires user permission.*
5. **For each Step:**
   1. User launches a fresh session.
   2. **Create `design.md`** — user invokes the `step-design` skill to develop the Step's `design.md`.
   3. **Create `spec.md`** — user invokes the `step-spec` skill to translate `design.md` into `spec.md`.
   4. **Build and Test** — agent deploys the `update-status` subagent at the **spec** checkpoint to commit `design.md` and `spec.md` as a safe point, then deploys the `build` subagent to write code based on `spec.md`, plus any requested tests, and then the `build-review` subagent to check the build independently. *Requires user permission (one question covers all three).*
   5. **Code Review** — agent summarizes the diff, test results, and `build-review` findings for the user in plain English, and waits for the user's approval before continuing. On approval, the agent sets the Step's status to `built`. If the user rejects the build, see [Rejected builds](#rejected-builds).
   6. **Draft Documentation Changes** — agent deploys the `step-docs-draft` subagent to write `docs_changes.md` (proposed changes to `docs/`) and `manual_changes.md` (a general description of what the manual should add) in the Step folder.
   7. **Update Status** — agent deploys the `update-status` subagent. *Requires user permission.*
6. User launches a fresh session.
7. **Comprehensive Review** — user invokes the `comp-review` skill to review changes across the Stage's Steps and look for opportunities to consolidate and optimize the code. Recommendations are written to `consolidate.md` in the Stage folder.
8. **Implement Consolidation** — agent deploys the `update-status` subagent at the **consolidate** checkpoint to commit `consolidate.md` as a safe point, then deploys the `build` subagent to incorporate the approved recommendations, plus any requested tests, and then the `build-review` subagent. *Requires user permission (one question covers all three).*
9. **Code Review** — agent summarizes the diff, test results, and `build-review` findings for the user, and waits for the user's approval before continuing. If the user rejects the build, see [Rejected builds](#rejected-builds).
10. **Update Documentation** — agent deploys the `update-docs` and `update-manual` subagents:
   - `update-docs` uses the Stage's `docs_changes.md` files and `consolidate.md` to update `docs/`.
   - `update-manual` uses the Stage's `manual_changes.md` files to write the changes into `manual.ipynb`, then regenerates `manual.html`.
11. **Update Status** — agent deploys the `update-status` subagent. *Requires user permission.*

Each session starts fresh, so all state needed to continue the loop must live in files (the plan, Step folders, and git history), not in conversation context.

### Safe points

Before every build, the plan file it follows (`spec.md`, `consolidate.md`, or a bug's `fix.md`) is committed. That commit is the build's **safe point**: every code change since it belongs to the build. `build` never commits, so until the next checkpoint commit, `git diff HEAD` (plus untracked files) shows exactly what the build did.

### Rejected builds

If the user rejects a build at Code Review, offer three choices:

1. **Fix and re-review** — the user says what's wrong. Sort each problem in plain English:
   - The code doesn't match the plan: `build` fixes the code. The plan doesn't change.
   - The plan was wrong: change the plan under the skill's usual rules (small changes yourself, larger ones with approval).
   - The design was wrong: always propose the change to `design.md` to the user first.

   Then deploy `build` in **fix** mode with the list of problems (choosing this option is the user's permission), then `build-review`, and review again.
2. **Discard and re-plan** — discard the build (see below), then revise the plan with the user, commit it again at the same checkpoint, and rebuild.
3. **Discard and set aside** — discard the build and stop, so the work can be rethought later.

**Discarding a build** removes every code change since the safe point, but keeps edits in `PROJECT_PLAN/` (plan changes made while handling build issues are still useful):

1. List what will be lost: `git status --short -- . ':!PROJECT_PLAN'` and `git clean -nd -- . ':!PROJECT_PLAN'`. Show the list to the user in plain English, and point out anything that doesn't look like it came from the build.
2. Only after the user confirms: `git restore --staged --worktree -- . ':!PROJECT_PLAN'` and `git clean -fd -- . ':!PROJECT_PLAN'`. Never use `git clean -x`, and never reset or rewrite commits.

### Bug fixes

A bug is code that doesn't do what its Step's `design.md` says. When the user reports one, log it in `PROJECT_PLAN/bugs.md` as `open`, whatever the current session is doing. The user fixes it with the `bugfix` skill, which diagnoses it, writes `PROJECT_PLAN/bugs/bug-NN-<name>/fix.md`, and runs a short build loop: **bug-spec** checkpoint (safe point), `build`, `build-review`, Code Review, then the **bug** checkpoint.

- **Bug status:** `open` → `specified` (`fix.md` written) → `built` (fix approved at Code Review) → `fixed` (committed). A bug that turns out to be a design change, or too big for one session, becomes `wont-fix` with the reason, and goes through `step-design` or `brainstorm` as a Step instead.
- **Only at a clean point:** nothing outside `PROJECT_PLAN/` is uncommitted, and no Step, consolidation, or other bug is between its safe point and its final commit. A bug that blocks the current work isn't a separate bug: handle it inside that work.
- Bugs aren't in `roadmap.md` and don't appear on the status dashboard.

### Models

Each part of the loop runs on a model suited to its work: the strongest models for thinking-heavy work and independent review, Sonnet for building and docs.

| Part of the loop | Model | Effort |
|---|---|---|
| `brainstorm` | `best` (Fable, or Opus where Fable isn't available) | — |
| `project-plan`, `step-design`, `step-spec`, `comp-review`, `bugfix`, `resume-loop` | `opus` | — |
| `build` | `sonnet` | high |
| `build-review` | `opus` | high |
| `step-docs-draft`, `update-docs`, `update-manual` | `sonnet` | medium |
| `update-status` | `sonnet` | low |

- **The skill and agent frontmatter (`model:`, `effort:`) is the source of truth.** To change a model, edit the frontmatter, then update this table.
- **A skill's model lasts only for the turn it's invoked.** The rest of the session, including Code Review and handling build issues, runs on the session model. Sessions in this project start on Opus (`"model": "opus"` in `.claude/settings.json`), so usually only `brainstorm` needs a switch: `/model best` (press `s` to apply it to this session only). Each skill reminds the user if the session isn't on its model.
- **When deploying a loop subagent, never pass a `model` override.** It would replace the agent's frontmatter.
- **Every subagent report starts with a `Model:` line.** If it doesn't match the table, tell the user: something is overriding the frontmatter (an Agent tool `model` parameter, or `CLAUDE_CODE_SUBAGENT_MODEL_FORCE`).

### Interrupted sessions

If a session ends partway through the loop, the user invokes the `resume-loop` skill in a fresh session. It works out where the loop stopped from the roadmap, the Stage and Step files, and git, and continues from there. Every skill checks at the start that the previous part of the loop finished, and points the user to `resume-loop` if it didn't.
