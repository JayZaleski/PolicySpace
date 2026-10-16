---
name: resume-loop
description: Work out where the development loop stopped after an interrupted session, explain it, and continue from there.
model: best
disable-model-invocation: true
---

# Resume Loop

A previous session ended partway through the development loop (see `CLAUDE.md`). Work out where it stopped, using only the files and git, explain that to the user in plain English, and continue from there.

**Model.** This skill is meant to run on the best available model: Fable, or Opus if Fable isn't available. Its frontmatter switches the model only for the turn it's invoked, so in your opening message tell the user, in one line, to run `/model best` (press `s` to apply it to this session only) if the session isn't already on it.

## 1. Gather the state

1. Read `PROJECT_PLAN/roadmap.md`, `PROJECT_PLAN/scratchpad.md`, and `PROJECT_PLAN/bugs.md` (if it exists).
2. Run `git status --short`, `git log --oneline -15`, and `git worktree list`. If the project isn't a git repository, tell the user the loop needs git (safe points and checkpoints are commits) and stop.
3. Find the current Stage: the first Stage in `roadmap.md` that isn't `done`. Read its Stage folder and the folders of its Steps.
4. Run `.venv/Scripts/python tools/export_status.py --check`. Any mismatch between `roadmap.md` and the plan folders is part of the evidence: point it out to the user.
5. Note what has uncommitted changes: `vision.md`, `roadmap.md`, `bugs.md`, the current Stage's plan files, bug `fix.md` files, `docs/`, `manual.ipynb`, and code (anything outside `PROJECT_PLAN/`, `docs/`, and `manual.*`).

If `scratchpad.md` isn't empty, an interactive skill was interrupted mid-discussion. Show the user what's in it. The skill you send them to will ask whether to use it as context.

## 2. Find where the loop stopped

Work through this table from the top, and stop at the first row that matches. "Step" means the first Step in the current Stage that isn't `done`. "Bug" means a bug in `bugs.md` that is `specified` or `built` (bug fixes run only at a clean point, so their rows come first).

| State | Where it stopped | Next |
|---|---|---|
| The bug is `specified`, and its `fix.md` has uncommitted changes | Bug fix approved, safe point not committed | **You:** bug-spec checkpoint and build, as in `bugfix` section 5 |
| The bug is `specified`, its `fix.md` is committed, and there are no code changes | Safe point committed, build never started | **You:** build in **new** mode, as in `bugfix` section 5 |
| The bug is `specified`, its `fix.md` is committed, and there are code changes | Bug fix build interrupted, or finished but not reviewed | **You:** recommend `build` in **continue** mode, then Code Review (below) |
| The bug is `built` | After the bug fix's Code Review | **You:** `bugfix` section 7 |
| `roadmap.md` is empty, or the current Stage is `outline` | Start of a Stage | User invokes `brainstorm` (or `project-plan` if `vision.md` has uncommitted changes and the scratchpad is empty) |
| `vision.md` has uncommitted changes, `roadmap.md` doesn't, and the current Stage isn't `in-progress` | During or after `brainstorm` | User invokes `brainstorm` if the scratchpad isn't empty, otherwise `project-plan` |
| The current Stage is `planned` and `roadmap.md` has uncommitted changes | After `project-plan`, before the plan commit | **You:** `update-status` at the **plan** checkpoint |
| The Step is `planned` | Before or during `step-design` | User invokes `step-design` |
| The Step is `designed` | Before or during `step-spec`, or a build that was set aside | User invokes `step-spec` |
| The Step is `specified`, and its `design.md`/`spec.md` have uncommitted changes | Spec approved, safe point not committed | **You:** spec checkpoint and build, as in `step-spec` section 5 |
| The Step is `specified`, the spec is committed, and there are no code changes | Safe point committed, build never started | **You:** build in **new** mode, as in `step-spec` section 5 |
| The Step is `specified`, the spec is committed, and there are code changes | Build interrupted, or finished but not reviewed | **You:** recommend `build` in **continue** mode, then Code Review (below) |
| The Step is `built`, and `docs_changes.md` or `manual_changes.md` is missing | After Code Review | **You:** `step-spec` section 7: sync `spec.md` and `design.md` with the build (below), then `step-docs-draft` and `update-status` at the **step** checkpoint |
| The Step is `built`, and both drafts exist | Before the step commit | **You:** `update-status` at the **step** checkpoint |
| Every Step is `done`, and there's no `consolidate.md` | Before `comp-review` | User invokes `comp-review` |
| `consolidate.md` has uncommitted changes, and every recommendation is `approved` or `rejected` | Recommendations agreed, safe point not committed | **You:** consolidate checkpoint and build, as in `comp-review` section 6 |
| `consolidate.md` has uncommitted changes otherwise, or the scratchpad isn't empty | During `comp-review` | User invokes `comp-review`, and resumes from the existing `consolidate.md` |
| `consolidate.md` is committed, it has `approved` recommendations, and there are no code changes | Safe point committed, build never started | **You:** build in **new** mode, as in `comp-review` section 6 |
| `consolidate.md` is committed, it has `approved` recommendations, and there are code changes | Consolidation build interrupted, or finished but not reviewed | **You:** recommend `build` in **continue** mode, then Code Review (below) |
| No recommendations are left `approved`, and the Stage isn't `done` | Before or during the vision and roadmap update, or the documentation update | **You:** `comp-review` section 7. Uncommitted `vision.md` or `roadmap.md` edits mean it was started: confirm them with the user, then go on to section 8 |

If the state doesn't fit any row, or doesn't hang together (for example, code changes while the Step is only `designed`), don't guess. Describe what you found to the user and ask how to proceed.

## 3. Explain and confirm

Tell the user in plain English:
- where the loop stopped;
- what evidence shows it (statuses, files, commits, uncommitted changes);
- what comes next;
- any `open` bugs in `bugs.md`, which the user can fix with `bugfix` at a clean point.

For an interrupted build, also say what the code changes cover, and offer the alternative of discarding them (see Rejected builds in `CLAUDE.md`).

**Interrupted parallel build.** If the plan file has a `## Packages` section and `git worktree list` shows package worktrees (under `.claude/worktrees/`), the session ended partway through a parallel build (see Parallel builds in `CLAUDE.md`). Packages without a worktree were already merged. For each remaining worktree, say what it changed (`git -C <worktree> status --short`). Offer to merge each one with `tools/merge_package.py`, including partial work, then finish with one `build` in **continue** mode in the main tree, which checks every task. If the user would rather drop a package's work, remove its worktree with `git worktree remove --force <path>` and `git branch -D <branch>` only after they confirm. Don't restart the package builds.

Get the user's agreement before doing anything.

## 4. Continue

- **Rows where the user invokes a skill:** tell them which skill to invoke, with the Stage or Step as its argument. They can do it in this session. Stop here.
- **Rows marked "You":** carry out the work yourself by following the named skill's instructions from that point on. Read the skill's `SKILL.md`, then follow its rules: ask permission before `build` and `update-status`, handle build issues and Code Review, including rejections, the same way, and use plain English with the user.
  - **Code Review** after a resumed build works as in `step-spec` section 6, `comp-review` section 6 for consolidation, or `bugfix` section 6 for a bug fix. Deploy `build-review` first.
  - **Syncing the plan after an earlier session's Code Review:** this session doesn't know which changes the user approved during the build. Compare the code built since the spec commit (`git diff <spec commit> -- . ':!PROJECT_PLAN'` plus new files) against `spec.md` and `design.md`. List each place where they don't match, and ask the user which were approved changes. Update `spec.md` and `design.md` for those only, and point out any others as possible bugs before going on.
  - When the loop reaches a point where the user would normally start a fresh session, tell them what comes next and stop.

## Rules

- Never discard code, reset, or rewrite commits without the user's explicit confirmation. Follow the discard steps in `CLAUDE.md`.
- Never mark a Step `built` or a recommendation `applied` without a Code Review that the user approved in this session.
