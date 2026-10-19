"""The deciders by name, and the loop that carries one.

`DECIDERS` maps the names a setting's businesses carry (`decider = "arithmetic"`) to
functions from a `Request` to a `Reply`. `serve` is the line-per-turn loop: it reads one
compact request per line, runs the decider and writes one compact reply per line. Later
rules are added as names.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import TextIO

from policyspace import arithmetic
from policyspace.protocol import Reply, Request, read_request, write_reply

DECIDERS: dict[str, Callable[[Request], Reply]] = {"arithmetic": arithmetic.decide}


def serve(decider: Callable[[Request], Reply], stdin: TextIO, stdout: TextIO) -> None:
    """For each line of `stdin` (blank lines skipped): reads a request, runs `decider`,
    writes one compact reply line to `stdout` and flushes. Returns at end of input.

    Raises `ProtocolError` for a line that isn't a request."""
    for line in stdin:
        if not line.strip():
            continue
        reply = decider(read_request(line))
        stdout.write(write_reply(reply) + "\n")
        stdout.flush()
