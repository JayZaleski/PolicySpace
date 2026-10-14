---
name: step-design
description: Collaboratively design a Step and write its design.md, a plain-English description of the feature. First step of each Step in the development loop.
model: best
argument-hint: "[step, e.g. 03 or tax-collection]"
disable-model-invocation: true
---

# Step Design

Work with the user to design one Step, then write its `design.md`: a plain-English description of the feature. This is the first step of each Step in the development loop (see `CLAUDE.md`).

**Model.** This skill is meant to run on the best available model: Fable, or Opus if Fable isn't available. Its frontmatter switches the model only for the turn it's invoked, so in your opening message tell the user, in one line, to run `/model best` (press `s` to apply it to this session only) if the session isn't already on it.

## 1. Pick the Step

- If the user named a Step (`$ARGUMENTS`), use it.
- Otherwise, use the first Step in the current Stage (the first Stage in `PROJECT_PLAN/roadmap.md` that isn't `done`) whose status is `planned`.
- Confirm the Step with the user before going on.
- If an earlier Step in the Stage isn't `done`, or anything outside `PROJECT_PLAN/` has uncommitted changes, the loop was probably interrupted. Tell the user and suggest the `resume-loop` skill before going on.
- If the Step already has a `design.md`, ask whether to revise it or start over.

## 2. Gather context

1. Read `PROJECT_PLAN/vision.md`, `PROJECT_PLAN/roadmap.md`, and `PROJECT_PLAN/scratchpad.md`.
   - `scratchpad.md` should be empty. If it isn't, an earlier session was probably interrupted. Show the user what's in it and ask whether to use it as context or clear it.
2. Read the files from the current Stage's earlier Steps: `design.md`, `spec.md`, and `docs_changes.md`. `docs/` isn't updated until the end of the Stage, so these files describe what the Stage has built so far.
3. Search `roadmap.md` for this Step's tags to find related Steps in earlier Stages. Read their `design.md` files, and the relevant files in `docs/`. Check `PROJECT_PLAN/bugs.md` for open bugs in the systems this Step touches.
4. Look through the relevant existing code to understand what the feature will build on.

## 3. Design

Open with a short summary of the Step as `roadmap.md` describes it, and what it builds on. Then work through the design with the user as a collaborator:

- Ask one or two questions at a time. Don't send a questionnaire.
- Offer your own ideas, alternatives, and edge cases.
- Push back on ideas that seem vague, overscoped, or in conflict with the vision or existing systems.
- Keep the Step to one technical feature, sized to be specified and built in a single session.

Keep the design in plain English: how the feature behaves and why. Leave code-level detail (files, classes, functions, data structures) to `spec.md`. Naming existing systems and Steps is fine.

As the discussion goes on, keep `scratchpad.md` up to date with bullet points showing how you're interpreting it: decisions made, rules and edge cases, things that are out of scope, open questions. When the user corrects you, fix the bullet rather than adding a new one.

### Roadmap changes

If the design shows that the roadmap should change (for example, the Step should be split, or a missing Step is needed), propose the change to the user. With their approval, you may edit the current Stage's not-yet-`done` Steps in `roadmap.md`, and create, rename, or renumber their folders to match. Never change `done` Steps or other Stages. Bigger changes should go through `brainstorm` and `project-plan`.

## 4. Write design.md

Once the user is ready, summarize the design and get their approval. Then write `design.md` in the Step folder using this template. Leave out sections that don't apply.

```markdown
# Step NN: <name>

## Summary
One paragraph: what this Step adds and why.

## Behavior
How the feature works, from the player's view and the system's view. Rules, edge cases, examples.

## Player-facing Changes
What the user sees or does differently.

## Interactions
The existing systems and Steps this Step touches or depends on.

## Out of Scope
What this Step deliberately doesn't do.

## Acceptance Criteria
Plain-English checks that the feature works.
```

Then:

1. Set the Step's status in `roadmap.md` to `designed`. If this is the first Step of the Stage, also set the Stage's status to `in-progress`.
2. Do not commit.
3. Empty `scratchpad.md`.

## 5. Wrap up

Summarize the design. Tell the user the next step: invoke the `step-spec` skill.
