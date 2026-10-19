"""Shared helpers for the Python tests: where the recordings are and how to read them."""

from pathlib import Path

import pytest

from policyspace import protocol

RECORDINGS = Path(__file__).resolve().parents[2] / "crates" / "engine" / "tests" / "recordings"
SAMPLE_RECORDING = RECORDINGS / "sample_turn0.recording.json"


def recording_text(name: str) -> str:
    """The text of the recording file `name`."""
    return (RECORDINGS / name).read_text(encoding="utf-8")


def hand_written_view(name: str) -> protocol.BusinessView:
    """The only view of the hand-written request `<name>.request.json`."""
    request = protocol.read_request(recording_text(f"{name}.request.json"))
    assert len(request.views) == 1
    return request.views[0]


@pytest.fixture
def sample_text() -> str:
    """The sample recording's text."""
    return SAMPLE_RECORDING.read_text(encoding="utf-8")
