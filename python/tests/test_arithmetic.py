"""Tests for the arithmetic rule: the sample's recorded reply, the hand-written cases the
sample doesn't reach, the cash budget, and the invariants every decision keeps."""

import dataclasses
import os

import pytest

from conftest import SAMPLE_RECORDING, hand_written_view, recording_text
from policyspace.arithmetic import after_wear, decide, decide_business
from policyspace.protocol import (
    Decision,
    FactoryOrder,
    OrderLine,
    Recording,
    Request,
    read_recording,
    read_request,
    write_recording,
)

HAND_WRITTEN = [
    "negative_margin",
    "unpriced_input",
    "cash_short",
    "revival",
    "no_market",
    "seller",
]


def sample_decisions() -> dict[str, Decision]:
    recording = read_recording(SAMPLE_RECORDING.read_text(encoding="utf-8"))
    return {d.business: d for d in decide(recording.request).decisions}


def quantities(table: dict[str, OrderLine]) -> dict[str, float]:
    """The quantity of each line."""
    return {good: line.quantity for good, line in table.items()}


def test_the_sample_reply_is_the_recordings():
    recording = read_recording(SAMPLE_RECORDING.read_text(encoding="utf-8"))
    reply = decide(recording.request)
    if os.environ.get("POLICYSPACE_UPDATE_RECORDINGS"):
        text = write_recording(Recording(recording.request, reply), pretty=True) + "\n"
        SAMPLE_RECORDING.write_text(text, encoding="utf-8", newline="\n")
        recording = read_recording(text)
    assert reply == recording.reply
    assert reply.turn == 0
    assert [d.business for d in reply.decisions] == [
        "center_farm",
        "center_mine",
        "center_toolmaker",
    ]


def test_the_sample_numbers():
    decisions = sample_decisions()
    recording = read_recording(SAMPLE_RECORDING.read_text(encoding="utf-8"))
    views = {v.business: v for v in recording.request.views}
    for decision in decisions.values():
        for table in (decision.offers, decision.bids):
            for good, line in table.items():
                assert line.slope == 0.0
                assert line.maximum == line.quantity
                assert line.give_up == views[decision.business].price(good)

    farm = decisions["center_farm"]
    assert farm.factories == {0: FactoryOrder(1.0, 0.125)}
    assert quantities(farm.bids) == {"tools": 0.5, "unimproved_grain": 4.0, "unskilled_labor": 7.0}
    assert farm.offers == {}

    mine = decisions["center_mine"]
    assert mine.factories == {1: FactoryOrder(1.0, 0.375)}
    assert quantities(mine.bids) == {"tools": 0.75, "unskilled_labor": 4.0}
    assert mine.offers == {}

    toolmaker = decisions["center_toolmaker"]
    order = toolmaker.factories[2]
    assert order.run == 1.0
    assert order.maintenance == min(1.0 - after_wear(1.0, 24), 1 / 3)
    assert order.maintenance == pytest.approx(1 / 24, abs=1e-9)
    assert quantities(toolmaker.bids)["copper"] == 8.0
    assert toolmaker.bids["unskilled_labor"].quantity == pytest.approx(6.25, abs=1e-9)
    assert toolmaker.bids["tools"].quantity == pytest.approx(0.125, abs=1e-9)
    assert set(toolmaker.bids) == {"copper", "unskilled_labor", "tools"}
    assert toolmaker.offers == {}

    for decision in decisions.values():
        assert "run and maintain" in decision.trace
        assert "within budget" in decision.trace
    assert "margin 0.25" in toolmaker.trace


def test_a_negative_margin_runs_down():
    view = hand_written_view("negative_margin")
    decision = decide_business(view)
    assert decision.factories == {0: FactoryOrder(0.0, 0.0)}
    assert quantities(decision.offers) == {"metal": 3.0, "ore": 5.0}
    assert quantities(decision.bids) == {}
    assert "run down" in decision.trace
    assert "margin -32.4 < 0" in decision.trace


def test_an_unpriced_input_runs_down():
    view = hand_written_view("unpriced_input")
    decision = decide_business(view)
    assert decision.factories == {0: FactoryOrder(0.0, 0.0)}
    assert quantities(decision.offers) == {"metal": 3.0}
    assert quantities(decision.bids) == {}
    assert "no posted price for ore" in decision.trace


def test_an_unpriced_bundle_good_runs_down():
    view = hand_written_view("unpriced_input")
    prices = dict(view.prices)
    prices["ore"] = 3.0
    del prices["tools"]
    decision = decide_business(dataclasses.replace(view, prices=prices))
    assert decision.factories == {0: FactoryOrder(0.0, 0.0)}
    assert "no posted price for tools" in decision.trace


def test_a_shut_down_factory_is_revived_but_not_run():
    view = hand_written_view("revival")
    decision = decide_business(view)
    assert decision.factories == {0: FactoryOrder(0.0, 0.5)}
    assert quantities(decision.bids) == {"labor": 1.0, "oven": 0.5}
    assert quantities(decision.offers) == {}
    assert "shut down" in decision.trace


def test_no_market_means_an_empty_decision():
    view = hand_written_view("no_market")
    decision = decide_business(view)
    assert decision.business == "remote"
    assert decision.factories == {}
    assert quantities(decision.offers) == {}
    assert quantities(decision.bids) == {}
    assert "no market" in decision.trace


def test_a_business_with_no_factories_only_sells():
    view = hand_written_view("seller")
    decision = decide_business(view)
    assert decision.factories == {}
    assert quantities(decision.offers) == {"grain": 4.0, "metal": 3.0}
    assert quantities(decision.bids) == {}
    assert "within budget" in decision.trace


def test_the_currency_in_a_bundle_is_priced_at_one_and_never_bid():
    view = hand_written_view("revival")
    technology = view.technologies["bakery"]
    technology = dataclasses.replace(technology, construction={"coin": 2.0})
    decision = decide_business(dataclasses.replace(view, technologies={"bakery": technology}))
    assert quantities(decision.bids) == {"oven": 0.5}
    assert "maintenance 2.2" in decision.trace


def test_a_cash_short_plan_is_scaled_uniformly():
    view = hand_written_view("cash_short")
    decision = decide_business(view)
    assert decision.factories == {0: FactoryOrder(0.5, 0.125), 1: FactoryOrder(0.5, 0.125)}
    assert quantities(decision.bids) == {"flour": 10.0, "labor": 10.5, "oven": 0.25}
    assert quantities(decision.offers) == {}
    cost = sum(line.quantity * view.price(good) for good, line in decision.bids.items())
    sales = sum(line.quantity * view.price(good) for good, line in decision.offers.items())
    assert cost <= view.cash + sales + 1e-9
    assert "scaled by 0.5" in decision.trace


def all_requests() -> list[Request]:
    requests = [read_recording(SAMPLE_RECORDING.read_text(encoding="utf-8")).request]
    requests += [read_request(recording_text(f"{name}.request.json")) for name in HAND_WRITTEN]
    return requests


def test_every_decision_is_well_formed():
    for request in all_requests():
        reply = decide(request)
        assert reply.turn == request.turn
        assert [d.business for d in reply.decisions] == [v.business for v in request.views]
        for view, decision in zip(request.views, reply.decisions):
            for table in (decision.offers, decision.bids):
                for good, line in table.items():
                    assert good != view.currency
                    assert view.price(good) is not None
                    assert line.quantity > 0
                    assert line.slope == 0.0
                    assert line.maximum == line.quantity
                    assert line.give_up == float(view.price(good))
                    for number in (line.quantity, line.slope, line.maximum, line.give_up):
                        assert isinstance(number, float)
            for order in decision.factories.values():
                assert 0.0 <= order.run <= 1.0
                assert 0.0 <= order.maintenance <= 1.0
                assert isinstance(order.run, float)
                assert isinstance(order.maintenance, float)
            if view.prices:
                assert set(decision.factories) == set(view.factories)
            else:
                assert decision.factories == {}
            assert isinstance(decision.trace, str)
            assert decision.trace
