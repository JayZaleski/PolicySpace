---
name: project-plan
description: Update PROJECT_PLAN/roadmap.md to reflect changes in vision.md, break the next Stage into Steps, and create its folders. Second step of each Stage in the development loop.
model: opus
disable-model-invocation: true
---

# Project Plan

Turn the changes from the latest brainstorm into an updated `PROJECT_PLAN/roadmap.md`. Break the next Stage into Steps and create its folders. This is the second step of each Stage in the development loop (see `CLAUDE.md`, which also defines the roadmap format, statuses, and tags).

**Model.** This skill is meant to run on Opus. Its frontmatter switches the model only for the turn it's invoked, so in your opening message tell the user, in one line, to run `/model opus` (press `s` to apply it to this session only) if the session isn't already on it.

## 1. Gather context

1. Read `PROJECT_PLAN/vision.md`, `PROJECT_PLAN/roadmap.md`, and `PROJECT_PLAN/scratchpad.md`.
   - `scratchpad.md` should be empty. If it isn't, an earlier session was probably interrupted. Show the user what's in it and ask whether to use it as context or clear it.
2. Find what changed in the vision with `git diff -- PROJECT_PLAN/vision.md`.
   - If the project is not a git repository, or `vision.md` has never been committed, treat the whole of `vision.md` as new and tell the user.
   - If there is no diff and `roadmap.md` already has content, tell the user that nothing in the vision has changed and ask whether to continue anyway.
3. Find the next Stage: the first Stage in `roadmap.md` that isn't `done`. If an earlier Stage is still `in-progress`, or has Steps that aren't `done`, or anything outside `PROJECT_PLAN/` has uncommitted changes, the loop was probably interrupted. Tell the user and suggest the `resume-loop` skill before going further.
4. Skim the existing Stage folders as needed to understand what has already been built.

## 2. Draft the plan

Work out how the vision changes affect the roadmap:

- **Done Stages and Steps are history.** Never edit or renumber them. If the vision change affects work that's already built, add new Steps to the next Stage to rework it, and point this out to the user.
- **The next Stage** gets a name, a goal, and a full breakdown into Steps:
  - A Stage develops one system up to the point where it depends on a different system. If the Stage's scope runs past that point, split the Stage.
  - Each Step designs and implements one technical feature. Size it so it can be designed, specified, and built in a single session.
  - Order the Steps so that each one depends only on Steps before it.
  - Give each Step a description of a few sentences and tags. Check the tags already used in `roadmap.md` and reuse them where they fit. Create new tags only when no existing tag applies.
- **Later Stages** stay as outlines (name, status `outline`, and goal). Add, remove, reorder, or rewrite them to match the vision.

Present the draft to the user and refine it with them. Keep `scratchpad.md` up to date with bullet points showing how you're interpreting the discussion (Stage boundaries, Step breakdown, decisions, open questions). When the user corrects you, fix the bullet rather than adding a new one.

## 3. Write the plan

Once the user approves:

1. Update `roadmap.md` in the format defined in `CLAUDE.md`. Set the next Stage's status to `planned` and each of its Steps to `planned`.
2. Create the folders for the next Stage: `PROJECT_PLAN/stage-NN-<name>/step-NN-<name>/`. Leave them empty, since `step-design` fills them. Folder names must match the names in `roadmap.md`.
3. Edit only `roadmap.md`, `scratchpad.md`, and the new folders. Do not edit `vision.md`.
4. Do not commit.
5. Empty `scratchpad.md`.

## 4. Wrap up

1. Summarize the plan changes.
2. Ask the user for permission to deploy the `update-status` subagent to commit the changes and update the status dashboard. Deploy it only if they agree.
3. Tell the user the next step: start a fresh session and invoke the `step-design` skill on the first Step.
