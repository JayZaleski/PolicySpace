"""The arithmetic rule: the first decider, a function from a batch of views to a batch of
decisions.

The rule plans each business from its view alone (posted prices, stockpile, cash, factories,
technologies) and remembers nothing between turns. Per technology it tests the margin at
posted prices: one turn at capacity earns capacity times the output's price and costs the
inputs plus one lifetime-slice of the capital and construction bundles. A technology at or
above zero margin has every factory run at capacity and maintained; any other is left to run
down. The business then offers what it holds beyond its needs and bids for what it lacks,
and, if the bids cost more than its cash plus the value of its offers, scales every
factory's run and maintenance by one common proportion. Every decision carries a trace of
the arithmetic. The slice arithmetic mirrors the engine's `production.rs` so that "holds a
slice" and the condition after wear are the engine's numbers.
"""

from __future__ import annotations

from dataclasses import dataclass

from policyspace.protocol import (
    BusinessView,
    Decision,
    FactoryOrder,
    Reply,
    Request,
    TechnologyView,
)

TOLERANCE = 1e-9
"""How close to a whole number of slices, or to 0 or 1, a condition counts as exactly that
(the engine's `production::TOLERANCE`)."""

NO_MARKET = "no market on this hex: nothing to do"


def slices(condition: float, lifetime: int) -> float:
    """A condition as a number of lifetime-slices, rounded to a whole number when within
    `TOLERANCE` of one."""
    exact = condition * lifetime
    rounded = float(round(exact))
    return rounded if abs(exact - rounded) < TOLERANCE else exact


def holds_a_slice(condition: float, lifetime: int) -> bool:
    """Whether a factory holds at least one lifetime-slice, so that it can run."""
    return slices(condition, lifetime) >= 1


def snap(condition: float) -> float:
    """A condition within `TOLERANCE` of 0 becomes 0.0, within `TOLERANCE` of 1 becomes 1.0."""
    if condition < TOLERANCE:
        return 0.0
    if condition > 1 - TOLERANCE:
        return 1.0
    return condition


def after_wear(condition: float, lifetime: int) -> float:
    """A condition after one turn's wear: one lifetime-slice lost, stopping at zero."""
    held = slices(condition, lifetime)
    return snap((held - min(held, 1.0)) / lifetime)


@dataclass
class Verdict:
    """The margin test of one technology."""

    passes: bool
    line: str


def _g(value: float) -> str:
    return format(value, ".6g")


def _first_unpriced(view: BusinessView, technology: TechnologyView) -> str | None:
    """The first good of the technology with no posted price: the output, then the inputs,
    the capital and the construction bundle (each in name order)."""
    goods = [technology.output]
    for bundle in (technology.inputs, technology.capital, technology.construction):
        goods.extend(sorted(bundle))
    return next((good for good in goods if view.price(good) is None), None)


def margin(view: BusinessView, name: str, technology: TechnologyView) -> Verdict:
    """Tests technology `name` at the view's posted prices."""
    unpriced = _first_unpriced(view, technology)
    if unpriced is not None:
        return Verdict(False, f"{name}: no posted price for {unpriced}: run down")
    capacity = float(technology.capacity)
    revenue = capacity * view.price(technology.output)
    input_cost = capacity * sum(
        technology.inputs[good] * view.price(good) for good in sorted(technology.inputs)
    )
    bundle_goods = sorted(set(technology.capital) | set(technology.construction))
    maintenance_cost = (
        sum(
            (technology.capital.get(good, 0) + technology.construction.get(good, 0))
            * view.price(good)
            for good in bundle_goods
        )
        / technology.lifetime
    )
    result = revenue - input_cost - maintenance_cost
    head = (
        f"{name}: revenue {_g(revenue)}; inputs {_g(input_cost)}; "
        f"maintenance {_g(maintenance_cost)}; margin {_g(result)}"
    )
    if result >= 0:
        return Verdict(True, head + ": run and maintain")
    return Verdict(False, head + " < 0: run down")


def needs(view: BusinessView, orders: dict[int, FactoryOrder]) -> dict[str, float]:
    """What the planned orders use, per good: the inputs of each run and the capital and
    construction goods of each maintenance, summed over every factory."""
    totals: dict[str, float] = {}
    for number in sorted(orders):
        order = orders[number]
        technology = view.technologies[view.factories[number].technology]
        for good, amount in technology.inputs.items():
            used = float(technology.capacity) * order.run * amount
            totals[good] = totals.get(good, 0.0) + used
        for bundle in (technology.capital, technology.construction):
            for good, amount in bundle.items():
                used = order.maintenance * amount
                totals[good] = totals.get(good, 0.0) + used
    return totals


def trade(
    view: BusinessView, wanted: dict[str, float]
) -> tuple[dict[str, float], dict[str, float]]:
    """The offers and bids that bring the stockpile to `wanted`: the surplus is offered, the
    shortfall bid for. The currency and goods with no posted price are skipped, and amounts
    below `TOLERANCE` (rounding specks) are dropped."""
    offers: dict[str, float] = {}
    bids: dict[str, float] = {}
    for good in sorted(set(wanted) | set(view.stockpile)):
        if good == view.currency or view.price(good) is None:
            continue
        held = float(view.held(good))
        need = wanted.get(good, 0.0)
        if need - held >= TOLERANCE:
            bids[good] = need - held
        elif held - need >= TOLERANCE:
            offers[good] = held - need
    return offers, bids


def _value(view: BusinessView, table: dict[str, float]) -> float:
    return sum(amount * view.price(good) for good, amount in sorted(table.items()))


def decide_business(view: BusinessView) -> Decision:
    """The arithmetic rule for one business."""
    if not view.prices:
        return Decision(view.business, trace=NO_MARKET)

    lines: list[str] = []
    verdicts: dict[str, Verdict] = {}
    for name in sorted(view.technologies):
        verdicts[name] = margin(view, name, view.technologies[name])
        lines.append(verdicts[name].line)

    orders: dict[int, FactoryOrder] = {}
    status: dict[int, str] = {}
    for number in sorted(view.factories):
        factory = view.factories[number]
        technology = view.technologies[factory.technology]
        if verdicts[factory.technology].passes:
            runs = holds_a_slice(factory.condition, technology.lifetime)
            maintenance = min(
                1.0 - after_wear(factory.condition, technology.lifetime),
                1.0 / technology.construction_turns,
            )
            orders[number] = FactoryOrder(1.0 if runs else 0.0, maintenance)
            status[number] = "run" if runs else "shut down"
        else:
            orders[number] = FactoryOrder(0.0, 0.0)
            status[number] = "run down"

    offers, bids = trade(view, needs(view, orders))
    cash = float(view.cash)
    sales = _value(view, offers)
    budget = cash + sales
    cost = _value(view, bids)
    if cost > budget and cost > 0:
        scale = budget / cost
        orders = {
            number: FactoryOrder(order.run * scale, order.maintenance * scale)
            for number, order in orders.items()
        }
        offers, bids = trade(view, needs(view, orders))
        budget_line = (
            f"budget {_g(budget)} = cash {_g(cash)} + offers {_g(sales)}; "
            f"bids cost {_g(cost)}: scaled by {_g(scale)}"
        )
    else:
        budget_line = (
            f"budget {_g(budget)} = cash {_g(cash)} + offers {_g(sales)}; "
            f"bids cost {_g(cost)}: within budget"
        )

    for number in sorted(view.factories):
        factory = view.factories[number]
        order = orders[number]
        head = f"factory #{number} ({factory.technology}, condition {_g(factory.condition)}): "
        if status[number] == "run down":
            lines.append(head + "run down")
        elif status[number] == "shut down":
            lines.append(head + f"shut down, maintenance {_g(order.maintenance)}")
        else:
            lines.append(head + f"run {_g(order.run)}, maintenance {_g(order.maintenance)}")
    lines.append(budget_line)

    return Decision(view.business, orders, offers, bids, "\n".join(lines))


def decide(request: Request) -> Reply:
    """The arithmetic rule over a batch: one decision per view, in the request's order."""
    return Reply(request.turn, [decide_business(view) for view in request.views])
