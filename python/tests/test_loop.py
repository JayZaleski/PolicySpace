"""Tests for the command-line loop: started as a subprocess, as Step 07 will start it."""

import subprocess
import sys

from conftest import SAMPLE_RECORDING
from policyspace.protocol import read_recording, write_reply, write_request


def run(*arguments: str, stdin: bytes = b"") -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, "-m", "policyspace", *arguments],
        input=stdin,
        capture_output=True,
    )


def test_the_script_answers_each_request_line():
    recording = read_recording(SAMPLE_RECORDING.read_text(encoding="utf-8"))
    line = write_request(recording.request).encode()
    expected = write_reply(recording.reply).encode()
    result = run("arithmetic", stdin=line + b"\n\n" + line + b"\n")
    assert result.returncode == 0
    assert result.stdout == expected + b"\n" + expected + b"\n"
    assert result.stderr == b""


def test_an_unknown_decider_fails_at_startup():
    result = run("bogus")
    assert result.returncode == 2
    assert b"bogus" in result.stderr
    assert b"arithmetic" in result.stderr


def test_no_argument_prints_usage():
    result = run()
    assert result.returncode == 2
    assert b"usage" in result.stderr


def test_a_bad_line_fails():
    result = run("arithmetic", stdin=b"not json\n")
    assert result.returncode == 1
    assert result.stderr
    assert result.stdout == b""
