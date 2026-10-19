"""The command line: `python -m policyspace <decider>` answers one request line on standard
input with one reply line on standard output, until input ends."""

from __future__ import annotations

import sys

from policyspace.deciders import DECIDERS, serve
from policyspace.protocol import ProtocolError


def main(argv: list[str]) -> int:
    """Runs the decider named in `argv`. Returns the exit code: 0 at end of input, 1 for a
    line that isn't a request, 2 for a bad command line."""
    known = ", ".join(sorted(DECIDERS))
    if len(argv) != 1:
        sys.stderr.write("usage: python -m policyspace <decider>\n")
        sys.stderr.write(f"known deciders: {known}\n")
        return 2
    name = argv[0]
    if name not in DECIDERS:
        sys.stderr.write(f"error: unknown decider '{name}' (known: {known})\n")
        return 2
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8", newline="\n")
    try:
        serve(DECIDERS[name], sys.stdin, sys.stdout)
    except ProtocolError as error:
        sys.stderr.write(f"error: {error}\n")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
