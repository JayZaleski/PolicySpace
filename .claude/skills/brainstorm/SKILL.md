---
name: brainstorm
description: Collaboratively brainstorm with the user and update PROJECT_PLAN/vision.md. First step of each Stage in the development loop.
model: best
disable-model-invocation: true
---

# Brainstorm

Work with the user to refine the project's vision, then record the result in `PROJECT_PLAN/vision.md`. This is the first step of each Stage in the development loop (see `CLAUDE.md`).

**Model.** This skill is meant to run on the best available model: Fable, or Opus if Fable isn't available. Its frontmatter switches the model only for the turn it's invoked, so in your opening message tell the user, in one line, to run `/model best` (press `s` to apply it to this session only) if the session isn't already on it.

## 1. Gather context

1. Read `PROJECT_PLAN/vision.md`, `PROJECT_PLAN/roadmap.md`, and `PROJECT_PLAN/scratchpad.md`.
   - `scratchpad.md` should be empty. If it isn't, an earlier session was probably interrupted. Show the user what's in it and ask whether to use it as context or clear it.
2. Run `git status` and `git diff -- PROJECT_PLAN/vision.md`.
   - If the project is not a git repository, warn the user: `project-plan` detects vision changes with git diff, so git should be initialized and `vision.md` committed before brainstorming.
   - If `vision.md` already has uncommitted changes, tell the user. A previous brainstorm was probably not followed by `project-plan` and `update-status`. Ask whether to continue on top of those changes or stop.
3. Work out where the project stands from `roadmap.md`: which Stages are done, and which Stage comes next. If a Stage is still `in-progress`, or anything outside `PROJECT_PLAN/` has uncommitted changes, the loop was probably interrupted. Tell the user, suggest the `resume-loop` skill, and ask whether to go on. If `vision.md` is empty, this is the first brainstorm. Start from the fundamentals: what the game is, who it's for, and what makes it worth building.

## 2. Brainstorm

Open with a short recap of the current vision and the project's progress (a few sentences, not a full summary), then ask what the user wants to explore.

Run the session as a collaborator, not just an interviewer:

- Ask one or two questions at a time. Don't send a questionnaire.
- Offer your own ideas and alternatives when they would help.
- Push back on ideas that seem vague, overscoped, or in conflict with each other.
- Point out tensions with the existing vision and with systems that earlier Stages already built.
- Briefly revisit the whole vision, then go deep on what the **next Stage** needs. Remember that a Stage develops one system up to the point it depends on another system.

Keep the discussion at the level of goals and player experience: the *what* and *why*. Leave the technical *how* to the Step `design.md` and `spec.md` files. If the user goes deep into implementation, note the idea and move back to the vision.

**Examples illustrate; they aren't requirements.** When the user gives an example to explain an idea, work out the principle or reason behind it, and ask if it isn't clear. Record the principle, not the example: a future agent can't tell an illustration from a required outcome, and may build the example instead of the rule. If the user means an example as the actual outcome, confirm that and record it as a decision.

As the discussion goes on, keep `PROJECT_PLAN/scratchpad.md` up to date with bullet points showing how you're interpreting it: decisions made, ideas accepted or rejected, open questions. Update it after each meaningful exchange, so the user can read it alongside the conversation and correct you. When the user corrects you, fix the bullet rather than adding a new one.

Keep going until the user says they're ready to write.

## 3. Update vision.md

1. Base the edits on `scratchpad.md`. Summarize the changes you plan to make to `vision.md` and get the user's approval.
2. Edit `vision.md` in place. It has no fixed structure: organize it however best fits the content, and reorganize it when that makes it clearer. Remove content that has been superseded instead of marking it obsolete, since git keeps the history.
3. Write principles and reasons, not the examples the user gave to explain them. Keep an example only when the principle can't be stated clearly without one, and then label it as an illustration ("for example"), never as the outcome.
4. Edit only `vision.md` and `scratchpad.md`. Do not touch `roadmap.md` or any other plan files.
5. **Do not commit.** `project-plan` finds the changes by diffing `vision.md` against the last commit.
6. Once the edits to `vision.md` are finished, empty `scratchpad.md` so it doesn't confuse future agents.

## 4. Wrap up

Summarize what changed. Tell the user the next step: start a fresh session and invoke the `project-plan` skill.
