---
name: comp-review
description: Review all the code changes from a completed Stage, look for chances to consolidate and optimize, and write the approved recommendations to the Stage's consolidate.md. Then run the end-of-Stage loop (build, code review, vision and roadmap update, docs, status).
model: best
argument-hint: "[stage, e.g. 01 or economy]"
disable-model-invocation: true
---

# Comprehensive Review

Review the code changes across all of a Stage's Steps, look for chances to consolidate and optimize, and write the recommendations to `consolidate.md` in the Stage folder. Then run the rest of the end-of-Stage loop (see `CLAUDE.md`): the consolidation build and its Code Review, an update of `vision.md` and `roadmap.md` to reflect the decisions made during the Stage, the documentation update, and the stage checkpoint.

**Two audiences.** Everything you say to the user (findings, recommendations, summaries, proposed changes) is in plain English: what it means for the code and the project, without code jargon. Bring in a technical term only when it's needed, and explain it when you do. `consolidate.md` is written for the `build` subagent and should be as precise and technical as that requires.

**Model.** This skill is meant to run on the best available model: Fable, or Opus if Fable isn't available. Its frontmatter switches the model only for the turn it's invoked, so in your opening message tell the user, in one line, to run `/model best` (press `s` to apply it to this session only) if the session isn't already on it.

## 1. Pick the Stage

- If the user named a Stage (`$ARGUMENTS`), use it.
- Otherwise, use the first Stage in `PROJECT_PLAN/roadmap.md` that is `in-progress`.
- Confirm the Stage with the user. If any of its Steps isn't `done`, or anything outside `PROJECT_PLAN/` has uncommitted changes, the loop was probably interrupted. Tell the user, suggest the `resume-loop` skill, and ask whether to go on.
- If the Stage already has a `consolidate.md`, ask whether to resume from it or start over.

## 2. Gather context

1. Read `PROJECT_PLAN/vision.md`, `PROJECT_PLAN/roadmap.md`, and `PROJECT_PLAN/scratchpad.md`.
   - `scratchpad.md` should be empty. If it isn't, an earlier session was probably interrupted. Show the user what's in it and ask whether to use it as context or clear it.
2. Read every Step's `design.md`, `spec.md`, `docs_changes.md`, and `manual_changes.md` in the Stage. Read `PROJECT_PLAN/bugs.md` and the `fix.md` of any bug fixed during the Stage, since its commits are part of the Stage's diff.
3. Find the Stage's code changes. Use `git log` to find the Stage's `plan:` commit, then diff from there to `HEAD`. Read the changed files in full where the diff isn't enough.
4. Read the relevant files in `docs/`, and the code that the Stage's changes connect to.

## 3. Review

Look across the Steps as a whole, not one at a time. Each Step was built with only partial knowledge of the ones that came after it. Look for:

- **Duplication:** logic, data structures, or helpers that several Steps built separately.
- **Emerging patterns:** a shared abstraction that only became visible once all the Steps existed. Recommend one only if it clearly earns its place.
- **Inconsistency:** naming, structure, error handling, or conventions that differ between Steps.
- **Leftovers:** dead code, unused parameters, temporary scaffolding, and workarounds that later Steps made unnecessary.
- **Efficiency:** wasted work, poor data structures, or hot paths that do more than they need to.
- **Tests:** duplicated or brittle tests, and gaps where Steps interact.
- **Bugs:** report any you find, even though finding bugs isn't the main goal.

Consolidation must not change behavior. A recommendation that would change how the feature behaves moves away from `design.md`: point that out to the user explicitly.

Skip changes that are purely cosmetic and changes whose benefit doesn't justify the risk.

## 4. Agree on recommendations

Present the findings to the user in plain English, grouped and ordered by value. For each one, give what it is, why it matters, the risk, and your recommendation. Go through them with the user and record each decision as `approved` or `rejected`.

Keep `scratchpad.md` up to date with bullet points of the findings and decisions. When the user corrects you, fix the bullet rather than adding a new one.

## 5. Write consolidate.md

Write `consolidate.md` in the Stage folder. Include every recommendation, including the rejected ones (with a one-line reason), so future reviews don't raise them again. Each approved recommendation must be self-contained enough for `build` to follow without this conversation.

```markdown
# Stage NN: <name> — Consolidation

## Context
Files `build` must read before starting.

## Recommendations

### C01: <title>
- **Status:** approved | rejected | applied
- **Summary:** <plain English: what and why>
- **Changes:** <files to modify, and what changes in each>
- **Tasks:** <an ordered checklist>
- **Tests:** <tests to add, change, or run to show behavior is unchanged>
- **Reason rejected:** <only if rejected>

## Packages
Only if the build splits (see Parallel builds in CLAUDE.md): each package's Owns, Tasks (recommendation IDs), and Isolated tests, then Integration.

## Done When
Every approved recommendation is applied, and all tests pass.
```

**Parallel build.** Check whether the approved recommendations split into packages that can be built at the same time (see Parallel builds in `CLAUDE.md`). Usually a package is a group of recommendations whose files don't overlap with any other group's. Look for it actively, but split only when the rules allow it, and tell the user whether the build will run as one or as N packages plus Integration. A recommendation that touches shared files belongs to Integration as a whole: don't split one recommendation across packages.

Then empty `scratchpad.md`. Do not commit.

## 6. Build

1. Ask the user for permission to commit `consolidate.md` and build, in one question. Only if they agree:
   1. Deploy the `update-status` subagent at the **consolidate** checkpoint. This commit is the build's safe point (see `CLAUDE.md`). If it reports a problem, explain it to the user and stop.
   2. Deploy the `build` subagent with `consolidate.md` and the list of approved recommendations, in **new** mode. If `consolidate.md` has Packages, run the parallel build instead, as Parallel builds in `CLAUDE.md` describes.
   3. Once `build` finishes (Integration, for a parallel build), deploy the `build-review` subagent with `consolidate.md`, the approved list, the mode, and every build's report (the build permission covers it).
2. If `build` reports an issue, update `consolidate.md` so it always matches what gets built:
   - **Small changes** (the kind you would have decided yourself without asking): edit `consolidate.md`, tell the user briefly what changed, and tell `build` to continue.
   - **Larger changes:** propose the change to the user first. Once they approve, edit `consolidate.md` and tell `build` to continue.
   - **Changes that move away from `design.md`:** always propose to the user, however small.
   - **In a parallel build,** continue only the builder that reported the issue (with `SendMessage`), and follow Parallel builds in `CLAUDE.md` if the split was wrong.
3. **Code Review:** once `build-review` finishes, summarize the diff (`git diff HEAD` plus new files), the test results, and the `build-review` findings for the user in plain English, recommendation by recommendation, and ask them to approve or reject. Don't fix anything before they decide.
   - **Approved:** set each approved recommendation that was built to `applied`.
   - **Rejected:** offer the three choices in `CLAUDE.md` (Rejected builds):
     1. **Fix and re-review:** sort the problems as `CLAUDE.md` describes and update `consolidate.md` where needed. To drop a single recommendation, set it to `rejected` with a reason, and list "undo CNN" as a problem. Deploy one `build` in **fix** mode (no package) with the list of problems, then `build-review`, then review again.
     2. **Discard and re-plan:** discard the build as `CLAUDE.md` describes, return to section 4 to revise the recommendations with the user, and continue from section 5. The revised `consolidate.md` gets its own (revised) commit.
     3. **Discard and set aside:** discard the build, and set every approved recommendation to `rejected` with the reason "set aside after Code Review". Then continue with section 7, so the Stage can finish without consolidation.

## 7. Update the vision and roadmap

The Stage's Steps were designed one at a time, and each design settled questions the vision left open, changed details, or reshaped what later Stages should expect. Bring `vision.md` and `roadmap.md` back in line with what was actually decided, so the next `brainstorm` and `project-plan` start from the truth rather than from the plan as it stood before the Stage.

1. Gather the decisions: each Step's `design.md` (the decisions, interactions, and out-of-scope lists), roadmap changes made by `step-design` during the Stage, bugs marked `wont-fix` as design changes, and the consolidation. Compare them against `vision.md` and against the Stage's entry and the outline Stages in `roadmap.md`.
2. Propose the edits to the user in plain English, and get their approval before editing:
   - **`vision.md`:** record decisions that settle, refine, or change what the vision says, at the vision's level (goals and player experience, the *what* and *why*). Replace superseded content rather than marking it obsolete. Don't add implementation detail, and don't transcribe `design.md` into it. Write principles, not examples, as `brainstorm` does.
   - **`roadmap.md`:** update this Stage's goal and its Steps' descriptions to match what was built, and the goals of outline Stages that the decisions affect. Keep the format in `CLAUDE.md`. Don't change statuses, names, numbering, or `done` Stages, and don't break outline Stages into Steps: that is `project-plan`'s job.
3. Edit the files. Do not commit: the **stage** checkpoint commits them, so the next `brainstorm` sees a clean `vision.md`.
4. If nothing needs to change, say so to the user and move on.

## 8. Documentation and status

1. Deploy the `update-docs` subagent with the Stage folder. Once it finishes, deploy the `update-manual` subagent. `update-manual` goes second because the manual links to the final `docs/` files.
2. Summarize the documentation changes for the user.
3. Ask the user for permission to deploy the `update-status` subagent at the **stage** checkpoint. Deploy it only if they agree.
4. Tell the user the Stage is complete, and summarize the vision and roadmap changes so they know what the next `brainstorm` will start from. The next Stage starts in a fresh session with the `brainstorm` skill.
