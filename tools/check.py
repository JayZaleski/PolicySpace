"""Run the project's checks: formatting, lints, and tests.

Runs, in order, from the project root (so the current directory doesn't matter):
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    python -m pytest python/tests

The fourth check runs the Python tests with the interpreter running this script, so
`.venv/Scripts/python tools/check.py` uses the virtual environment. If `pytest` isn't
installed it fails with Python's "No module named pytest"; `pip install -r requirements.txt`
fixes it. All four always run, even if an earlier one fails. Standard library only.

Usage:
    python tools/check.py                    # or .venv/Scripts/python tools/check.py

Exit codes: 0 = every check passed; 1 = a check failed, or cargo wasn't found.
"""

import shutil
import subprocess
import sys
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parent.parent

CHECKS = [
    ("cargo fmt", ["cargo", "fmt", "--all", "--", "--check"]),
    ("cargo clippy", ["cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
    ("cargo test", ["cargo", "test", "--workspace"]),
    ("pytest", [sys.executable, "-m", "pytest", "python/tests"]),
]


def main() -> int:
    if shutil.which("cargo") is None:
        print("error: cargo was not found on PATH. Install Rust (https://rustup.rs) and try again.")
        return 1

    results = []
    for name, command in CHECKS:
        print(f"\n=== {name}: {' '.join(command)} ===", flush=True)
        try:
            passed = subprocess.run(command, cwd=PROJECT_ROOT).returncode == 0
        except FileNotFoundError:
            print("error: cargo was not found on PATH. Install Rust (https://rustup.rs) and try again.")
            return 1
        results.append((name, passed))

    print("\n=== Summary ===")
    for name, passed in results:
        print(f"{name}: {'passed' if passed else 'FAILED'}")
    return 0 if all(passed for _, passed in results) else 1


if __name__ == "__main__":
    sys.exit(main())
