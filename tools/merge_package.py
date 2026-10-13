"""Merge one finished package build from its worktree into the main working tree.

A plan file (a Step's spec.md or a Stage's consolidate.md) may split its build into
packages (see the Packages section in CLAUDE.md). Each package is built in its own git
worktree. This script copies the files the package owns from that worktree into the main
working tree, drops everything else (scratch edits), then removes the worktree and its
branch. Nothing is committed. Standard library only.

Usage:
    python tools/merge_package.py <plan file> <package id> <worktree path>
    python tools/merge_package.py PROJECT_PLAN/stage-01-x/step-05-y/spec.md P1 .claude/worktrees/abc
    python tools/merge_package.py ... --keep      # merge, but leave the worktree in place

The package's files are the backticked paths on its "- Owns:" line (and any indented
lines under it) in the plan's "## Packages" section. A path ending in "/" owns everything
under that folder.

Exit codes: 0 = merged; 1 = refused, nothing changed in the main tree (the reason is on
stderr); 2 = bad arguments or the plan has no such package.
"""

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent

PACKAGES_RE = re.compile(r"^##\s+Packages\s*$")
PACKAGE_RE = re.compile(r"^###\s+(\S+?):")
OWNS_RE = re.compile(r"^-\s+Owns:")
FIELD_RE = re.compile(r"^-\s+[A-Za-z ]+:")
PATH_RE = re.compile(r"`([^`]+)`")


class Refused(Exception):
    pass


def git(cwd, *args):
    out = subprocess.run(["git", "-C", str(cwd), *args],
                         capture_output=True, text=True, encoding="utf-8", timeout=60)
    if out.returncode != 0:
        raise Refused(f"git {' '.join(args)} failed in {cwd}: {out.stderr.strip()}")
    return out.stdout


def read_owns(plan_path, package):
    """Return the owned paths of `package` in the plan's Packages section, or None."""
    lines = plan_path.read_text(encoding="utf-8").splitlines()
    in_packages = in_package = in_owns = False
    owns = None
    for line in lines:
        if line.startswith("## "):
            in_packages = bool(PACKAGES_RE.match(line))
            in_package = in_owns = False
            continue
        if not in_packages:
            continue
        m = PACKAGE_RE.match(line)
        if m or line.startswith("### "):
            in_package = bool(m) and m.group(1) == package
            in_owns = False
            if in_package:
                owns = []
            continue
        if not in_package:
            continue
        if OWNS_RE.match(line):
            in_owns = True
        elif FIELD_RE.match(line) or not line.strip():
            in_owns = False
            continue
        elif not line[:1].isspace():
            in_owns = False
        if in_owns:
            owns.extend(p.strip() for p in PATH_RE.findall(line))
    return owns


def is_owned(path, owns):
    return any(path == o or (o.endswith("/") and path.startswith(o)) for o in owns)


def find_worktree(path):
    """Return (worktree path, branch) if `path` is a linked worktree of this repo."""
    target = Path(path).resolve()
    entries = git(PROJECT_ROOT, "worktree", "list", "--porcelain").split("\n\n")
    for i, entry in enumerate(entries):
        fields = dict(line.split(" ", 1) for line in entry.splitlines() if " " in line)
        if "worktree" in fields and Path(fields["worktree"]).resolve() == target:
            if i == 0:
                raise Refused(f"{path} is the main working tree, not a package worktree")
            branch = fields.get("branch", "").removeprefix("refs/heads/") or None
            return target, branch
    raise Refused(f"{path} is not a worktree of this repository (see `git worktree list`)")


def changed_files(worktree, base):
    """Every file the worktree changed since `base`, as (status letter, path)."""
    git(worktree, "add", "-A")  # the worktree's own index; the main tree's is untouched
    out = git(worktree, "diff", "--cached", "--no-renames", "--name-status", "-z", base)
    parts = out.split("\0")
    return [(parts[i][0], parts[i + 1]) for i in range(0, len(parts) - 1, 2)]


def merge(plan_path, package, worktree_arg, keep):
    if package.lower() == "integration":
        print("error: the Integration package is built in the main tree, not merged", file=sys.stderr)
        return 2
    owns = read_owns(plan_path, package)
    if owns is None:
        print(f"error: {plan_path} has no package {package!r} under '## Packages'", file=sys.stderr)
        return 2
    if not owns:
        print(f"error: package {package!r} has no backticked paths on its '- Owns:' line", file=sys.stderr)
        return 2

    try:
        worktree, branch = find_worktree(worktree_arg)
        base = git(PROJECT_ROOT, "rev-parse", "HEAD").strip()
        changes = changed_files(worktree, base)
        merged = [(s, p) for s, p in changes if is_owned(p, owns)]
        dropped = [p for s, p in changes if not is_owned(p, owns)]
        if not merged:
            raise Refused(f"package {package} changed none of its owned files; the worktree is left in place")

        busy = git(PROJECT_ROOT, "status", "--porcelain", "--", *(p for _, p in merged))
        if busy.strip():
            raise Refused("these owned files already have changes in the main tree "
                          "(another package or build touched them):\n" + busy.rstrip())
    except Refused as e:
        print(f"refused: {e}", file=sys.stderr)
        return 1

    for status, path in merged:
        dest = PROJECT_ROOT / path
        if status == "D":
            dest.unlink(missing_ok=True)
        else:
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(worktree / path, dest)

    print(f"Merged package {package} from {worktree}:")
    for status, path in merged:
        print(f"  {status} {path}")
    print("Dropped (not owned by this package):" if dropped else "Dropped: none")
    for path in dropped:
        print(f"  {path}")

    if keep:
        print(f"Kept worktree {worktree}")
        return 0
    try:
        git(PROJECT_ROOT, "worktree", "remove", "--force", str(worktree))
        if branch:
            git(PROJECT_ROOT, "branch", "-D", branch)
    except Refused as e:
        print(f"warning: merged, but cleanup failed: {e}", file=sys.stderr)
        return 0
    print(f"Removed worktree {worktree}" + (f" and branch {branch}" if branch else ""))
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("plan", type=Path, help="the plan file with the Packages section")
    parser.add_argument("package", help="the package ID, e.g. P1")
    parser.add_argument("worktree", help="the package's worktree path")
    parser.add_argument("--keep", action="store_true", help="don't remove the worktree after merging")
    args = parser.parse_args()
    if not args.plan.is_file():
        print(f"error: no plan file at {args.plan}", file=sys.stderr)
        return 2
    return merge(args.plan, args.package, args.worktree, args.keep)


if __name__ == "__main__":
    sys.exit(main())
