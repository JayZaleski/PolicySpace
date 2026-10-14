"""Export the project's progress to the shared dashboard folder.

Parses PROJECT_PLAN/roadmap.md (format defined in CLAUDE.md), validates it, works
out each task's progress from the plan folders and git, and atomically writes
<STATUS_ROOT>/<PROJECT>/status.json (schema_version 2: Stages, sections, and tasks;
see the README in the Project-Tracking folder). Standard library only.

Usage:
    python tools/export_status.py            # export now
    python tools/export_status.py --hook     # Claude Code PostToolUse hook mode: exports when a file
                                             # under PROJECT_PLAN/ (other than scratchpad.md) was edited
    python tools/export_status.py --check    # check roadmap.md against the plan folders; no export
    python tools/export_status.py --roadmap <path>   # parse a different roadmap (testing)

Exit codes: 0 = exported, or skipped because the share is unavailable (warning on
stderr), or --check found no problems; 1 = --check found the roadmap and the plan
folders out of step; 2 = roadmap.md doesn't match the format (nothing written).
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

# --- Per-project settings: the only lines another project needs to change. ---
STATUS_ROOT = r"Z:\Project-Tracking"  # overridden by the PROJECT_TRACKING_DIR env var
PROJECT = "PolicySpace"
# ------------------------------------------------------------------------------

SCHEMA_VERSION = 2
PROJECT_ROOT = Path(__file__).resolve().parent.parent
PLAN_DIR = PROJECT_ROOT / "PROJECT_PLAN"
ROADMAP = PLAN_DIR / "roadmap.md"

STAGE_STATUSES = ("outline", "planned", "in-progress", "done")
STEP_STATUSES = ("planned", "designed", "specified", "built", "done")
STAGE_FIELDS = {"Status": True, "Goal": True}
STEP_FIELDS = {"Status": True, "Tags": True, "Description": True}

# Files each status requires in the Step or Stage folder (later statuses need them too).
STEP_FILES = {
    "designed": ("design.md",),
    "specified": ("spec.md",),
    "done": ("docs_changes.md", "manual_changes.md"),
}
STAGE_FILES = {"done": ("consolidate.md",)}

STAGE_RE = re.compile(r"^##\s+Stage\s+(\d+):\s*(.*?)\s*$")
STEP_RE = re.compile(r"^###\s+Step\s+(\d+):\s*(.*?)\s*$")
FIELD_RE = re.compile(r"^-\s+([A-Za-z]+):\s*(.*?)\s*$")
NAME_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
TAG_RE = re.compile(r"^#([a-z0-9]+(?:-[a-z0-9]+)*)$")
RECOMMENDATION_STATUS_RE = re.compile(r"^-\s+\*\*Status:\*\*\s*(approved|rejected|applied)\b")


class RoadmapError(Exception):
    pass


def parse_roadmap(text):
    stages = []
    current = None  # the Stage or Step dict whose fields are being read
    last_field = None
    errors = []

    for lineno, line in enumerate(text.splitlines(), 1):
        if m := STAGE_RE.match(line):
            stage = {"number": int(m[1]), "name": m[2], "_line": lineno, "_fields": {}, "steps": []}
            stages.append(stage)
            current, last_field = stage, None
        elif m := STEP_RE.match(line):
            if not stages:
                errors.append(f"line {lineno}: Step heading before any Stage")
                current = None
                continue
            step = {"number": int(m[1]), "name": m[2], "_line": lineno, "_fields": {}}
            stages[-1]["steps"].append(step)
            current, last_field = step, None
        elif line.startswith("#"):
            current, last_field = None, None  # any other heading ends the block
        elif current is None:
            continue  # text outside Stage/Step blocks is ignored
        elif m := FIELD_RE.match(line):
            key, value = m[1], m[2]
            allowed = STEP_FIELDS if "steps" not in current else STAGE_FIELDS
            if key not in allowed:
                errors.append(f"line {lineno}: unknown field '{key}' (expected {', '.join(allowed)})")
            elif key in current["_fields"]:
                errors.append(f"line {lineno}: duplicate field '{key}'")
            else:
                current["_fields"][key] = value
                last_field = key
        elif line.strip() == "":
            last_field = None
        elif last_field is not None:
            current["_fields"][last_field] += " " + line.strip()  # wrapped continuation line
        else:
            errors.append(f"line {lineno}: unexpected text inside a Stage/Step block: {line.strip()!r}")

    result = []
    for i, stage in enumerate(stages, 1):
        where = f"line {stage['_line']} (Stage {stage['number']:02d})"
        check_block(stage, i, STAGE_FIELDS, STAGE_STATUSES, where, errors)
        if stage["_fields"].get("Status") == "outline" and stage["steps"]:
            errors.append(f"{where}: an outline Stage must not have Steps")
        steps = []
        for j, step in enumerate(stage["steps"], 1):
            swhere = f"line {step['_line']} (Step {stage['number']:02d}.{step['number']:02d})"
            check_block(step, j, STEP_FIELDS, STEP_STATUSES, swhere, errors)
            steps.append({
                "number": step["number"],
                "name": step["name"],
                "status": step["_fields"].get("Status"),
                "tags": parse_tags(step["_fields"].get("Tags", ""), swhere, errors),
                "description": step["_fields"].get("Description"),
            })
        result.append({
            "number": stage["number"],
            "name": stage["name"],
            "status": stage["_fields"].get("Status"),
            "goal": stage["_fields"].get("Goal"),
            "steps": steps,
        })

    if errors:
        raise RoadmapError("\n".join(errors))
    return result


def check_block(block, expected_number, fields, statuses, where, errors):
    if block["number"] != expected_number:
        errors.append(f"{where}: expected number {expected_number:02d}")
    if not NAME_RE.match(block["name"]):
        errors.append(f"{where}: name '{block['name']}' must be lowercase kebab-case")
    for key, required in fields.items():
        if required and key not in block["_fields"]:
            errors.append(f"{where}: missing '- {key}:'")
    status = block["_fields"].get("Status")
    if status is not None and status not in statuses:
        errors.append(f"{where}: status '{status}' must be one of {', '.join(statuses)}")


def parse_tags(value, where, errors):
    tags = []
    for token in value.split():
        if m := TAG_RE.match(token):
            tags.append(m[1])
        else:
            errors.append(f"{where}: tag '{token}' must look like #lowercase-kebab-case")
    return tags


def check_folders(stages, plan_dir):
    """Returns a list of places where the roadmap and the plan folders disagree."""
    problems = []
    expected = {f"stage-{s['number']:02d}-{s['name']}": s for s in stages if s["status"] != "outline"}
    for d in sorted(plan_dir.glob("stage-*")):
        if d.is_dir() and d.name not in expected:
            problems.append(f"{d.name}/: no matching Stage in roadmap.md (outline Stages have no folder)")
    for folder, stage in expected.items():
        stage_dir = plan_dir / folder
        if not stage_dir.is_dir():
            problems.append(f"{folder}/: missing folder for Stage {stage['number']:02d}")
            continue
        problems += missing_files(stage_dir, folder, stage["status"], STAGE_STATUSES, STAGE_FILES)
        steps = {f"step-{s['number']:02d}-{s['name']}": s for s in stage["steps"]}
        for d in sorted(stage_dir.glob("step-*")):
            if d.is_dir() and d.name not in steps:
                problems.append(f"{folder}/{d.name}/: no matching Step in roadmap.md")
        for step_folder, step in steps.items():
            where = f"{folder}/{step_folder}"
            if not (stage_dir / step_folder).is_dir():
                problems.append(f"{where}/: missing folder for Step {stage['number']:02d}.{step['number']:02d}")
                continue
            problems += missing_files(stage_dir / step_folder, where, step["status"], STEP_STATUSES, STEP_FILES)
    return problems


def missing_files(folder, where, status, statuses, required):
    reached = statuses[:statuses.index(status) + 1]
    return [f"{where}/{name}: missing, but the status is '{status}'"
            for s in reached for name in required.get(s, ())
            if not (folder / name).is_file()]


# --- Sections, tasks, and progress (schema_version 2) -------------------------
#
# Each Stage is exported as three sections, planning / work / wrap-up, each a list
# of tasks. A task's progress is the share of its sub-tasks that are done. Steps are
# the tasks of the work section. Sub-task completion is derived from evidence the
# development loop already leaves: roadmap statuses, files in the plan folders, the
# recommendation statuses in consolidate.md, and git (dirty files, checkpoint commits).


def task(name, subtasks, **extra):
    """subtasks: ordered (name, done) pairs. Non-step tasks get a status derived from progress."""
    done = sum(1 for _, d in subtasks if d)
    progress = round(100 * done / len(subtasks)) if subtasks else 0
    derived = "done" if done == len(subtasks) else "in-progress" if done else "planned"
    return {"name": name, "status": extra.pop("status", derived), **extra,
            "progress": progress, "subtasks": [{"name": n, "done": bool(d)} for n, d in subtasks]}


def step_task(step, step_dir):
    reached = STEP_STATUSES.index(step["status"])
    at_least = lambda s: reached >= STEP_STATUSES.index(s)
    drafts = all((step_dir / f).is_file() for f in STEP_FILES["done"])
    return task(step["name"], [
        ("design", at_least("designed")),
        ("spec", at_least("specified")),
        ("build", at_least("built")),
        ("code-review", at_least("built")),  # the build is only observable as the approved Code Review
        ("docs-draft", at_least("done") or drafts),
        ("commit", at_least("done")),
    ], number=step["number"], status=step["status"], tags=step["tags"], description=step["description"])


def planning_tasks(stage, is_current, evidence):
    started = stage["status"] != "outline"
    plan_commit = re.compile(rf"^plan: stage {stage['number']:02d}\b")
    return [
        task("brainstorm", [
            ("brainstorm", started or (is_current and "PROJECT_PLAN/vision.md" in evidence["dirty"])),
        ]),
        task("project-plan", [
            ("project-plan", started),
            ("commit", started and any(plan_commit.match(s) for s in evidence["subjects"])),
        ]),
    ]


def wrap_up_tasks(stage, stage_dir, evidence):
    finished = stage["status"] == "done"
    consolidate = stage_dir / "consolidate.md"
    reviewed = finished or consolidate.is_file()
    consolidated = finished or (consolidate.is_file() and not consolidate_has_approved(consolidate))
    dirty_docs = any(p.startswith("docs/") for p in evidence["dirty"])
    dirty_manual = any(p in ("manual.ipynb", "manual.html") for p in evidence["dirty"])
    return [
        task("comprehensive-review", [
            ("review", reviewed),
            ("consolidation-build", consolidated),  # flips with code-review: approval sets recommendations to applied
            ("code-review", consolidated),
        ]),
        task("documentation", [
            ("update-docs", finished or (consolidated and dirty_docs)),
            ("update-manual", finished or (consolidated and dirty_manual)),
            ("commit", finished),
        ]),
    ]


def consolidate_has_approved(path):
    """True if consolidate.md still has a recommendation whose Status is approved (not yet built)."""
    try:
        text = path.read_text(encoding="utf-8")
    except OSError:
        return False
    return any(m[1] == "approved" for line in text.splitlines() if (m := RECOMMENDATION_STATUS_RE.match(line)))


def build_sections(stages, plan_dir, evidence):
    """Turns the parsed Stages into the schema-2 list: each Stage with its sections and tasks."""
    current = next((s["number"] for s in stages if s["status"] != "done"), None)
    result = []
    for stage in stages:
        stage_dir = plan_dir / f"stage-{stage['number']:02d}-{stage['name']}"
        work = [step_task(step, stage_dir / f"step-{step['number']:02d}-{step['name']}") for step in stage["steps"]]
        result.append({
            "number": stage["number"],
            "name": stage["name"],
            "status": stage["status"],
            "goal": stage["goal"],
            "sections": [
                {"name": "planning", "tasks": planning_tasks(stage, stage["number"] == current, evidence)},
                {"name": "work", "tasks": work},
                {"name": "wrap-up", "tasks": wrap_up_tasks(stage, stage_dir, evidence)},
            ],
        })
    return result


# --- git ------------------------------------------------------------------------


def git(*args):
    try:
        out = subprocess.run(["git", "-C", str(PROJECT_ROOT), *args],
                             capture_output=True, text=True, encoding="utf-8", timeout=15)
    except (OSError, subprocess.SubprocessError):
        return None
    return out.stdout if out.returncode == 0 else None


def git_info():
    """Returns (last_commit, uncommitted_changes, evidence). Without git the evidence is empty."""
    evidence = {"dirty": set(), "subjects": []}
    if (git("rev-parse", "--is-inside-work-tree") or "").strip() != "true":
        return None, None, evidence
    log = git("log", "-1", "--format=%h%x00%s%x00%cI")
    last = None
    if log and log.strip():
        h, subject, date = log.strip().split("\x00")
        last = {"hash": h, "subject": subject, "date": date}
    status = git("status", "--porcelain")
    if status is not None:
        # Porcelain lines are "XY path" or "XY old -> new"; paths are repo-relative with '/'.
        evidence["dirty"] = {line[3:].split(" -> ")[-1].strip('"') for line in status.splitlines() if len(line) > 3}
    evidence["subjects"] = (git("log", "--format=%s") or "").splitlines()
    return last, (bool(status.strip()) if status is not None else None), evidence


# --- output ---------------------------------------------------------------------


def write_atomic(path, data):
    tmp = path.with_name(f"{path.name}.{os.getpid()}.tmp")
    try:
        with open(tmp, "w", encoding="utf-8", newline="\n") as f:
            f.write(data)
            f.flush()
            os.fsync(f.fileno())
        for attempt in range(10):
            try:
                os.replace(tmp, path)
                return
            except PermissionError:  # Windows: a reader has the file open
                if attempt == 9:
                    raise
                time.sleep(0.2)
    finally:
        if tmp.exists():
            tmp.unlink()


def load(roadmap_path):
    """Returns the parsed Stages, or None after reporting a format error."""
    try:
        text = roadmap_path.read_text(encoding="utf-8") if roadmap_path.exists() else ""
        return parse_roadmap(text)
    except RoadmapError as e:
        print(f"export_status: {roadmap_path} doesn't match the roadmap format in CLAUDE.md; "
              f"status.json was not updated.\n{e}", file=sys.stderr)
        return None


def check(roadmap_path):
    """Returns an exit code."""
    stages = load(roadmap_path)
    if stages is None:
        return 2
    problems = check_folders(stages, roadmap_path.parent)
    if problems:
        print("export_status: roadmap.md and the plan folders don't match:\n"
              + "\n".join(f"- {p}" for p in problems), file=sys.stderr)
        return 1
    print("export_status: roadmap.md matches the plan folders")
    return 0


def export(roadmap_path):
    """Returns an exit code."""
    stages = load(roadmap_path)
    if stages is None:
        return 2

    root = Path(os.environ.get("PROJECT_TRACKING_DIR", STATUS_ROOT))
    if not root.is_dir():
        print(f"export_status: WARNING: {root} is not available; dashboard not updated.", file=sys.stderr)
        return 0

    last_commit, uncommitted, evidence = git_info()
    snapshot = {
        "schema_version": SCHEMA_VERSION,
        "project": PROJECT,
        "generated_at": datetime.now().astimezone().isoformat(timespec="seconds"),
        "last_commit": last_commit,
        "uncommitted_changes": uncommitted,
        "stages": build_sections(stages, roadmap_path.parent, evidence),
    }
    target = root / PROJECT / "status.json"
    try:
        target.parent.mkdir(exist_ok=True)
        write_atomic(target, json.dumps(snapshot, indent=2, ensure_ascii=False) + "\n")
    except OSError as e:
        print(f"export_status: WARNING: couldn't write {target}: {e}; dashboard not updated.", file=sys.stderr)
        return 0
    print(f"export_status: wrote {target}")
    return 0


def hook_targets_plan():
    """True if the hook payload's edited file is under PROJECT_PLAN/ and isn't scratchpad.md."""
    try:
        payload = json.load(sys.stdin)
        file_path = payload["tool_input"]["file_path"]
    except (ValueError, KeyError, TypeError):
        return False
    norm = lambda p: os.path.normcase(os.path.abspath(p))
    edited, plan = norm(file_path), norm(PLAN_DIR)
    return edited.startswith(plan + os.sep) and os.path.basename(edited) != "scratchpad.md"


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--hook", action="store_true",
                        help="read a PostToolUse hook payload from stdin; export only if a plan file was edited")
    parser.add_argument("--check", action="store_true", help="check roadmap.md against the plan folders; no export")
    parser.add_argument("--roadmap", type=Path, default=ROADMAP, help="roadmap file to parse (for testing)")
    args = parser.parse_args()
    if args.hook and not hook_targets_plan():
        return 0
    return check(args.roadmap) if args.check else export(args.roadmap)


if __name__ == "__main__":
    sys.exit(main())
