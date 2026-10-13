---
name: step-spec
description: Translate a Step's design.md into spec.md, the technical instructions the build subagent follows. Second step of each Step in the development loop.
model: opus
argument-hint: "[step, e.g. 03 or tax-collection]"
disable-model-invocation: true
---

# Step Spec

Translate a Step's `design.md` into `spec.md`: technical instructions that the `build` subagent can follow without any other context. This is the second step of each Step in the development loop (see `CLAUDE.md`).

**Two audiences.** Everything you say to the user (decisions, summaries, proposed changes) is in plain English: what each option means for the feature and the project, without code jargon. Bring in a technical term only when it's needed, and explain it when you do. `spec.md` itself is written for the `build` subagent and should be as precise and technical as that requires.

**Model.** This skill is meant to run on Opus. Its frontmatter switches the model only for the turn it's invoked, so in your opening message tell the user, in one line, to run `/model opus` (press `s` to apply it to this session only) if the session isn't already on it.

## 1. Pick the Step

- If the user named a Step (`$ARGUMENTS`), use it.
- Otherwise, use the first Step in the current Stage whose status is `designed`. Usually this is the Step that `step-design` just finished in this session.
- Confirm the Step with the user before going on.
- If the Step has no `design.md`, stop and tell the user to run `step-design` first.
- If anything outside `PROJECT_PLAN/` has uncommitted changes, or an earlier Step in the Stage isn't `done`, the loop was probably interrupted. Tell the user and suggest the `resume-loop` skill before going on.
- If the Step already has a `spec.md`, ask whether to revise it or start over.

## 2. Gather context

1. Read the Step's `design.md`, plus `PROJECT_PLAN/roadmap.md` and `PROJECT_PLAN/scratchpad.md`.
   - `scratchpad.md` should be empty unless `step-design` ran earlier in this session and has already cleared it. If it isn't empty, show the user what's in it and ask whether to use it as context or clear it.
2. Read the current Stage's earlier Steps' `spec.md` and `docs_changes.md` files for what was actually built.
3. Search `roadmap.md` for this Step's tags to find related Steps, and read the relevant files in `docs/`.
4. Study the code the feature will touch: its structure, conventions, and the interfaces it will use or extend.

## 3. Technical decisions

Identify the key technical decisions: architecture, data structures, where the code lives, how it connects to existing systems, and anything with real trade-offs. Bring them to the user one or two at a time, in plain English, each with options, the trade-offs, and your recommendation. Don't ask about choices with an obvious answer. Make those yourself and list them in the draft.

Keep `scratchpad.md` up to date with bullet points of the decisions as they're made. When the user corrects you, fix the bullet rather than adding a new one.

If the technical work shows that `design.md` is wrong or incomplete, propose a fix to the user. With their approval, update `design.md` so the two files stay consistent. If the roadmap needs to change, follow the same rules as `step-design`: change only not-yet-`done` Steps in the current Stage, and only with approval.

## 4. Write spec.md

Draft `spec.md` in the Step folder using the template below. Present a plain-English summary of it to the user for approval, and point them to the file if they want the full detail. Leave out sections that don't apply.

The spec must be **self-contained**: the `build` subagent sees only `spec.md` and the files it points to, never this conversation. Specify exact files, public interfaces, and data structures, and break the work into ordered tasks. Leave function bodies to `build`, but call out any non-obvious logic, edge cases, or pitfalls.

**Parallel build.** Once the Changes and Tasks are clear, check whether the work splits into packages that can be built at the same time (see Parallel builds in `CLAUDE.md` for the rules and the `## Packages` format). Look for it actively, but split only when the rules allow it. If it splits, add the `## Packages` section after Tasks, and make each package's Isolated tests concrete. In the summary for the user, say whether the build will run as one or as N packages plus Integration, and why.

```markdown
# Step NN: <name> — Spec

## Context
Files `build` must read before starting: this Step's design.md, relevant docs/ files, and key existing code.

## Approach
A short overview of the technical approach and the key decisions, each with a one-line rationale.

## Changes
Each file to create or modify, and what changes in it.

## Interfaces & Data
Public functions, classes, and signatures; data structures; formats; events.

## Tasks
An ordered checklist. Each task is small and can be checked on its own.

## Packages
Only if the build splits (see Parallel builds in CLAUDE.md): each package's Owns, Tasks, and Isolated tests, then Integration.

## Test Plan
The tests to write, each mapped to an Acceptance Criterion in design.md, plus how to run them.

## Constraints
Conventions to follow, code not to touch, and what's out of scope.

## Done When
All tasks complete, all tests in the Test Plan pass, and every Acceptance Criterion is met.
```

Once the user approves:

1. Set the Step's status in `roadmap.md` to `specified`.
2. Do not commit.
3. Empty `scratchpad.md`.

## 5. Build

Summarize the spec. Ask the user for permission to commit the spec and build, in one question. Only if they agree:

1. Deploy the `update-status` subagent at the **spec** checkpoint. This commit is the build's safe point (see `CLAUDE.md`). If it reports a problem, explain it to the user and stop.
2. Deploy the `build` subagent on this Step's `spec.md` in **new** mode. If the spec has Packages, run the parallel build instead, as Parallel builds in `CLAUDE.md` describes: the package builds at once, a merge after each, then Integration.
3. Once `build` finishes (Integration, for a parallel build), deploy the `build-review` subagent with the spec, the mode, and every build's report (the build permission covers it).

### Handling issues during the build

If `build` reports an issue with the spec, fix `spec.md` so that it always matches what gets built:

- **Small changes** (the kind you would have decided yourself in section 3 without asking): edit `spec.md`, tell the user briefly what changed, and tell `build` to continue.
- **Larger changes** (the kind you would have brought to the user in section 3): propose the change to the user first. Once they approve, edit `spec.md` and tell `build` to continue.
- **Changes that move away from `design.md`**: always propose to the user, however small. Once they approve, update both `design.md` and `spec.md`, then tell `build` to continue.

In a parallel build, tell only the builder that reported the issue to continue (with `SendMessage`); the others keep going. If the fix changes another package's files or the interfaces it uses, tell that builder too. If the issue shows the split was wrong, follow Parallel builds in `CLAUDE.md`.

## 6. Code Review

Once `build-review` finishes, summarize the diff (`git diff HEAD` plus new files) and the test results for the user in plain English: what was built, how it meets each Acceptance Criterion, anything `build` flagged, the `build-review` findings (each sorted as in Rejected builds in `CLAUDE.md`), and anything that looks off to you. The findings inform the user's decision: don't fix anything before they decide. Ask the user to approve or reject.

- **Approved:** set the Step's status in `roadmap.md` to `built`.
- **Rejected:** offer the three choices in `CLAUDE.md` (Rejected builds):
  1. **Fix and re-review:** sort the problems as `CLAUDE.md` describes, update `spec.md` (and `design.md`) where needed, deploy one `build` in **fix** mode (no package, even if the spec has Packages) with the list of problems, then `build-review`, then review again.
  2. **Discard and re-spec:** discard the build as `CLAUDE.md` describes. Set the Step's status back to `designed`, return to section 3 to revise `spec.md` (and `design.md` if needed) with the user, then continue from section 4. The new spec gets its own (revised) spec commit.
  3. **Discard and set aside:** discard the build, and set the Step's status back to `designed`. Keep `design.md` and `spec.md` as they are. Tell the user the Step can be picked up later with `step-spec` (or `step-design` to rethink it), and stop here.

## 7. Wrap up

Once the build is approved:

1. Make sure `spec.md` and `design.md` reflect every change the user approved during the build, Code Review, and any fix rounds, so `step-docs-draft` works from what was actually built. Edit them where they don't, and tell the user briefly what changed.
2. Continue the Step loop as `CLAUDE.md` describes: `step-docs-draft`, then `update-status` at the **step** checkpoint.
