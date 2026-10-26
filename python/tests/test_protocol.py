"""Tests for the protocol module: the engine-style float layout and JSON writer, strict
reading, defaults and key order, and the lossless round trips of the engine's files."""

import pytest

from conftest import RECORDINGS, recording_text
from policyspace.protocol import (
    Decision,
    FactoryOrder,
    OrderLine,
    ProtocolError,
    Reply,
    dumps,
    format_float,
    read_decision,
    read_recording,
    read_reply,
    read_request,
    read_view,
    write_decision,
    write_recording,
    write_reply,
    write_request,
    write_view,
)


@pytest.mark.parametrize(
    ("value", "expected"),
    [
        (1e-5, "0.00001"),
        (1.23e-5, "0.0000123"),
        (1e-6, "1e-6"),
        (1e-4, "0.0001"),
        (1e16, "1e+16"),
        (1e15, "1000000000000000.0"),
        (1.5e16, "1.5e+16"),
        (123.0, "123.0"),
        (1e21, "1e+21"),
        (12345678901234567.0, "1.2345678901234568e+16"),
        (0.1, "0.1"),
        (100.0, "100.0"),
        (5e-324, "5e-324"),
        (1.0 / 3.0, "0.3333333333333333"),
        (123456789.123456789, "123456789.12345679"),
        (1e-300, "1e-300"),
        (0.0, "0.0"),
        (-0.0, "-0.0"),
        (-2.5, "-2.5"),
        (1.7976931348623157e308, "1.7976931348623157e+308"),
    ],
)
def test_format_float_matches_the_engine(value, expected):
    assert format_float(value) == expected


@pytest.mark.parametrize("value", [float("inf"), float("-inf"), float("nan")])
def test_format_float_refuses_non_finite_values(value):
    with pytest.raises(ProtocolError):
        format_float(value)


def test_dumps_writes_ints_strings_and_tables():
    data = {"turn": 18446744073709551615, "s": 'a"b\\c\nd', "l": [], "d": {}}
    expected = r'{"turn":18446744073709551615,"s":"a\"b\\c\nd","l":[],"d":{}}'
    assert dumps(data) == expected


def test_dumps_pretty_follows_serde_json():
    data = {"a": [1, {"b": None}], "c": {}}
    expected = '{\n  "a": [\n    1,\n    {\n      "b": null\n    }\n  ],\n  "c": {}\n}'
    assert dumps(data, pretty=True) == expected


def test_dumps_refuses_booleans():
    with pytest.raises(ProtocolError):
        dumps(True)


LINE = '{"quantity":8,"slope":0,"maximum":8,"give_up":0.5}'
"""A well-formed line as JSON text."""


@pytest.mark.parametrize(
    "text",
    [
        '{"business": ',
        '{"business":"a","offer":{}}',
        '{"business":"a","offers":{"g":' + LINE + ',"g":' + LINE + "}}",
        '{"business":"a","offers":{"g":{"quantity":NaN,"slope":0,"maximum":1,"give_up":1}}}',
        '{"business":"a","factories":{"x":{}}}',
        '{"business":"a","factories":{"0":{"run":true}}}',
    ],
)
def test_reading_refuses_bad_decisions(text):
    with pytest.raises(ProtocolError):
        read_decision(text)


def test_reading_names_what_is_wrong_with_a_line():
    def decision(line, side="offers"):
        return '{"business":"a","' + side + '":{"g":' + line + "}}"

    with pytest.raises(ProtocolError, match="missing key 'give_up' in offer 'g'"):
        read_decision(decision('{"quantity":8,"slope":0,"maximum":8}'))
    with pytest.raises(ProtocolError, match="unknown key 'anchor' in bid 'g'"):
        read_decision(decision('{"quantity":8,"slope":0,"maximum":8,"give_up":1,"anchor":2}', "bids"))
    with pytest.raises(ProtocolError, match="offer 'g' quantity must be a number"):
        read_decision(decision('{"quantity":"8","slope":0,"maximum":8,"give_up":1}'))
    with pytest.raises(ProtocolError, match="offer 'g' slope must be a number"):
        read_decision(decision('{"quantity":8,"slope":true,"maximum":8,"give_up":1}'))
    with pytest.raises(ProtocolError, match="offer 'g' must be a table"):
        read_decision(decision("8"))
    with pytest.raises(ProtocolError, match="offers must be a table"):
        read_decision('{"business":"a","offers":8}')
    with pytest.raises(ProtocolError, match="duplicate key 'g'"):
        read_decision('{"business":"a","bids":{"g":' + LINE + ',"g":' + LINE + "}}")


def test_reading_refuses_a_view_with_missing_keys():
    with pytest.raises(ProtocolError, match="missing key"):
        read_view('{"turn":4}')


def test_reading_names_the_unknown_key():
    with pytest.raises(ProtocolError, match="unknown key 'cach' in view"):
        read_view('{"cach": 1}')


@pytest.mark.parametrize("text", ['{"turn":"4","decisions":[]}', '{"turn":4.0,"decisions":[]}'])
def test_reading_refuses_a_bad_turn(text):
    with pytest.raises(ProtocolError):
        read_reply(text)


def test_decision_defaults_and_empty_form():
    assert read_decision('{"business":"a"}') == Decision.empty("a")
    one = read_decision('{"business":"a","factories":{"0":{"run":1}}}')
    assert one.factories[0] == FactoryOrder(1, 0.0)
    assert (
        write_decision(Decision.empty("a"))
        == '{"business":"a","factories":{},"offers":{},"bids":{}}'
    )
    assert read_reply('{"turn":4}') == Reply(4, [])
    assert write_decision(Decision("a", trace="x")).endswith(',"trace":"x"}')
    one_line = read_decision('{"business":"a","offers":{"g":' + LINE + "}}")
    assert one_line.offers == {"g": OrderLine(8, 0, 8, 0.5)}
    line = one_line.offers["g"]
    assert [type(n) for n in (line.quantity, line.slope, line.maximum, line.give_up)] == [
        int,
        int,
        int,
        float,
    ]
    assert read_decision(write_decision(one_line)) == one_line


def test_tables_are_written_sorted():
    decision = Decision(
        "a",
        factories={10: FactoryOrder(1.0, 0.0), 2: FactoryOrder(0.5, 0.25)},
        offers={"b": OrderLine.flat(1.0, 0.5), "a": OrderLine(2.0, 0.5, 4.0, 1.0)},
        bids={"y": OrderLine.flat(3.0, 2.0)},
    )
    text = write_decision(decision)
    factories = (
        '"factories":{"2":{"run":0.5,"maintenance":0.25},"10":{"run":1.0,"maintenance":0.0}}'
    )
    assert factories in text
    a = '"a":{"quantity":2.0,"slope":0.5,"maximum":4.0,"give_up":1.0}'
    b = '"b":{"quantity":1.0,"slope":0.0,"maximum":1.0,"give_up":0.5}'
    assert '"offers":{' + a + "," + b + "}" in text
    y = '"y":{"quantity":3.0,"slope":0.0,"maximum":3.0,"give_up":2.0}'
    assert '"bids":{' + y + "}" in text


def test_a_flat_line_has_no_slope_and_a_maximum_of_its_amount():
    assert OrderLine.flat(8.0, 0.5) == OrderLine(8.0, 0.0, 8.0, 0.5)


def test_the_exactness_request_comes_back_byte_for_byte():
    text = recording_text("exactness.request.json")
    assert write_request(read_request(text)) == text.rstrip("\n")


def test_the_sample_recording_round_trips():
    text = recording_text("sample_turn0.recording.json")
    recording = read_recording(text)
    assert write_recording(recording, pretty=True) + "\n" == text
    assert read_recording(write_recording(recording)) == recording
    for view in recording.request.views:
        assert read_view(write_view(view)) == view
    for decision in recording.reply.decisions:
        assert read_decision(write_decision(decision)) == decision
    assert read_reply(write_reply(recording.reply)) == recording.reply


def test_every_request_file_round_trips():
    for path in sorted(RECORDINGS.glob("*.request.json")):
        request = read_request(path.read_text(encoding="utf-8"))
        assert read_request(write_request(request)) == request
