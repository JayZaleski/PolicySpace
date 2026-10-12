---
name: bugfix
description: Diagnose and fix a bug in code that's already built, outside the Stage/Step loop. Logs the bug in PROJECT_PLAN/bugs.md, writes its fix.md, then builds, reviews, and commits the fix.
model: opus
argument-hint: "[bug number, or a short description of a new bug]"
disable-model-invocation: true
---

# Bug Fix

Fix a bug in code that's already built: find the cause, write `fix.md` (the plan the `build` subagent follows), then build, review, and commit the fix. Bugs are tracked in `PROJECT_PLAN/bugs.md`, and each fix has its own folder in `PROJECT_PLAN/bugs/` (see Bug fixes in `CLAUDE.md`).

**Two audiences.** Everything you say to the user (diagnosis, options, summaries) is in plain English: what it means for the game and the code, without code jargon. Bring in a technical term only when it's needed, and explain it when you do. `fix.md` is written for the `build` subagent and should be as precise and technical as that requires.

**Model.** This skill is meant to run on Opus. Its frontmatter switches the model only for the turn it's invoked, so in your opening message tell the user, in one line, to run `/model opus` (press `s` to apply it to this session only) if the session isn't already on it.

## 1. Check for a clean point

A bug fix needs its own safe point, so it runs only at a clean point:

- nothing outside `PROJECT_PLAN/` has uncommitted changes;
- no Step in the current Stage is `specified` or `built`;
- the Stage's `consolidate.md`, if there is one, has no `approved` recommendations waiting to be built;
- no other bug is `specified` or `built` in `bugs.md`.

If it isn't a clean point, explain why to the user in plain English. Log the bug in `bugs.md` as `open` (section 2) so it isn't lost, and tell the user to run `bugfix` again once the current work is committed. If the bug blocks the current work, it isn't a separate bug: handle it inside that work (as a build issue, or at Code Review). If the state looks like an interrupted loop, suggest `resume-loop`.

If the project isn't a git repository, stop: the fix needs a safe point commit.

## 2. Log the bug

- If the user named a bug number (`$ARGUMENTS`), use its entry in `bugs.md`. If its status is `specified` or `built`, an earlier fix was interrupted: suggest `resume-loop`.
- Otherwise, ask the user to describe the bug: what happens, what should happen, and how to make it happen. Add a new entry at the end of `bugs.md`, numbered after the last one:

```markdown
## Bug NN: <kebab-case-name>
- Status: open
- Affects: Step SS.TT <name> (or `unknown`)
- Found: <date>, <what you were doing>
- Description: <what happens, what should happen, how to reproduce>
```

Create `bugs.md` with a `# Bugs` heading if it doesn't exist.

## 3. Diagnose

1. Read `PROJECT_PLAN/scratchpad.md`. It should be empty. If it isn't, show the user what's in it and ask whether to use it as context or clear it.
2. Read the affected Step's `design.md` and `spec.md`, related Steps (search `roadmap.md` for its tags), the relevant `docs/` files, and the code.
3. Reproduce the bug if you can: run the code or the tests. Find the cause.
4. Decide what kind of problem it is, and tell the user in plain English:
   - **A bug:** the code doesn't do what `design.md` says. Continue.
   - **A design gap or change:** the code does what `design.md` says, but the design was wrong or the user wants different behavior. This isn't a bug fix. Set the bug to `wont-fix` with the reason "design change", and point the user to the right route: a new or changed Step through `step-design` (if it fits the current Stage) or `brainstorm`. Stop here.
   - **Too big:** the fix can't be specified and built in one session. Propose turning it into a Step, the same way as a design change.

Keep `scratchpad.md` up to date with bullet points of the diagnosis and decisions. When the user corrects you, fix the bullet rather than adding a new one.

## 4. Write fix.md

Bring any real choices to the user (one or two at a time, with options and your recommendation). Make the obvious ones yourself. Then write `PROJECT_PLAN/bugs/bug-NN-<name>/fix.md`:

```markdown
# Bug NN: <name> — Fix

## Context
Files `build` must read before starting: the affected Step's design.md and spec.md, relevant docs/ files, and key code.

## Problem
What happens, what should happen, and how to reproduce it.

## Cause
What in the code causes it.

## Changes
Each file to modify, and what changes in it.

## Tasks
An ordered checklist.

## Test Plan
A regression test that fails before the fix and passes after it, plus how to run the full suite.

## Constraints
Code not to touch. The fix must not change any other behavior.

## Done When
All tasks complete, the regression test passes, and the full suite passes.
```

Summarize the fix for the user in plain English and get their approval. Then set the bug to `specified` in `bugs.md`, and empty `scratchpad.md`. Do not commit.

## 5. Build

Ask the user for permission to commit the fix plan and build, in one question. Only if they agree:

1. Deploy the `update-status` subagent at the **bug-spec** checkpoint. This commit is the build's safe point (see `CLAUDE.md`). If it reports a problem, explain it to the user and stop.
2. Deploy the `build` subagent on the bug's `fix.md` in **new** mode.
3. If `build` reports an issue, update `fix.md` under the same rules as `step-spec` (small changes yourself, larger ones with approval, anything that departs from `design.md` always with approval).
4. Once `build` finishes, deploy the `build-review` subagent (the build permission covers it).

## 6. Code Review

Summarize for the user in plain English: what changed, how the regression test shows the bug is fixed, the test results, anything `build` flagged, and the `build-review` findings (each sorted as in Rejected builds in `CLAUDE.md`). Ask the user to approve or reject.

- **Approved:** set the bug to `built` in `bugs.md`.
- **Rejected:** offer the three choices in `CLAUDE.md` (Rejected builds):
  1. **Fix and re-review:** update `fix.md` where needed, deploy `build` in **fix** mode with the list of problems, then `build-review`, then review again.
  2. **Discard and re-plan:** discard the build as `CLAUDE.md` describes, return to section 3 or 4 with the user, and continue from there. The new `fix.md` gets its own (revised) commit.
  3. **Discard and set aside:** discard the build, set the bug back to `open`, keep `fix.md`, and stop.

## 7. Docs and commit

1. If `docs/` or the manual describe the buggy behavior, fix them. If the affected Step is in the current Stage (`docs/` isn't updated until the Stage ends), update that Step's `docs_changes.md` or `manual_changes.md` instead. If you edit `manual.ipynb`, regenerate `manual.html`.
2. Ask the user for permission to deploy the `update-status` subagent at the **bug** checkpoint. Deploy it only if they agree.
3. Tell the user the bug is fixed, and what the loop was doing before (so they know where to pick up).
