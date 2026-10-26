"""The decider protocol as Python data: the engine's messages, read and written in their
exact JSON form.

Every message is a dataclass with the Rust field names (`BusinessView`, `Decision`,
`Request`, `Reply`, `Recording` and their parts). A decision's offers and bids are tables of
`OrderLine`s, one per good: a quantity at the posted price, a slope, a maximum and a give-up
price, all required (the engine anchors the line at the good's posted price). Numbers are kept exactly as parsed: JSON
integers stay `int` (exact for the engine's `u64` turns) and other numbers stay `float`, and
the package never converts between them. Reading is strict like the engine's: unknown or
missing keys, wrong types, repeated keys, `NaN`/`Infinity` and booleans are refused with a
`ProtocolError`. Value ranges (fractions, negative or wrong-way line numbers) are not checked
here; that is the engine's validation.

Writing is done by `dumps`, a small JSON writer that lays numbers out the way the engine's
serde_json does (Python's own writer prints `1e-05` where the engine prints `0.00001`), with
tables in sorted order and fields in the engine's order, so a message read from the engine
comes back byte for byte.
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass, field

Number = int | float
"""A JSON number as Python parsed it: an `int` or a `float`, never a `bool`."""


class ProtocolError(ValueError):
    """Text that isn't a message of the format: the parser's message, or 'unknown key ...',
    'missing key ...', 'duplicate key ...', '... must be a number' and the like."""


# ---------------------------------------------------------------------------------------
# The messages
# ---------------------------------------------------------------------------------------


@dataclass
class PerishGroup:
    """One perish group of a good in a stockpile."""

    amount: Number
    perishes: int | None


@dataclass
class FactoryView:
    """One factory as its business sees it."""

    technology: str
    condition: Number


@dataclass
class TechnologyView:
    """A technology's numbers, as the rules give them."""

    inputs: dict[str, Number]
    output: str
    capacity: Number
    capital: dict[str, Number]
    construction: dict[str, Number]
    construction_turns: int
    lifetime: int


@dataclass
class DepositsView:
    """The deposits of one resource type a business owns."""

    count: int
    produces: str
    output: Number


@dataclass
class BusinessView:
    """What one business is told at the decision point."""

    turn: int
    business: str
    currency: str
    cash: Number
    prices: dict[str, Number]
    stockpile: dict[str, list[PerishGroup]]
    factories: dict[int, FactoryView]
    technologies: dict[str, TechnologyView]
    deposits: dict[str, DepositsView]

    def held(self, good: str) -> Number:
        """The total of `good` over every perish group; 0 if the business holds none."""
        return sum((group.amount for group in self.stockpile.get(good, [])), 0)

    def price(self, good: str) -> Number | None:
        """The posted price of `good`: 1 for the hex's currency, None if it has none."""
        if good == self.currency:
            return 1.0
        return self.prices.get(good)


@dataclass
class FactoryOrder:
    """The two fractions for one factory."""

    run: Number = 0.0
    maintenance: Number = 0.0


@dataclass
class OrderLine:
    """One order line as a decision carries it: the quantity at the posted price, the slope,
    the maximum, and the give-up price. The engine anchors it at the good's posted price."""

    quantity: Number
    slope: Number
    maximum: Number
    give_up: Number

    @staticmethod
    def flat(amount: Number, give_up: Number) -> "OrderLine":
        """A flat line: `amount` at every price the give-up allows (slope 0.0, maximum
        `amount`)."""
        return OrderLine(amount, 0.0, amount, give_up)


@dataclass
class Decision:
    """A business's answer for one turn."""

    business: str
    factories: dict[int, FactoryOrder] = field(default_factory=dict)
    offers: dict[str, OrderLine] = field(default_factory=dict)
    bids: dict[str, OrderLine] = field(default_factory=dict)
    trace: str | None = None

    @staticmethod
    def empty(business: str) -> "Decision":
        """A decision that does nothing: no orders, offers or bids, no trace."""
        return Decision(business)


@dataclass
class Request:
    """One turn's views, served together."""

    turn: int
    views: list[BusinessView]


@dataclass
class Reply:
    """One turn's decisions, collected together."""

    turn: int
    decisions: list[Decision]


@dataclass
class Recording:
    """A request and its reply, saved together."""

    request: Request
    reply: Reply


# ---------------------------------------------------------------------------------------
# JSON text: reading
# ---------------------------------------------------------------------------------------


def _no_duplicates(pairs: list[tuple[str, object]]) -> dict[str, object]:
    result: dict[str, object] = {}
    for key, value in pairs:
        if key in result:
            raise ProtocolError(f"duplicate key '{key}'")
        result[key] = value
    return result


def _no_constant(name: str) -> object:
    raise ProtocolError(f"{name} is not a number of the format")


def loads(text: str) -> object:
    """Parses JSON text, refusing a repeated key and `NaN`/`Infinity`."""
    try:
        return json.loads(text, object_pairs_hook=_no_duplicates, parse_constant=_no_constant)
    except json.JSONDecodeError as error:
        raise ProtocolError(str(error)) from error


# ---------------------------------------------------------------------------------------
# JSON text: writing
# ---------------------------------------------------------------------------------------


def format_float(value: float) -> str:
    """A float as the engine writes it: the shortest digits that read back to the same
    number, laid out as serde_json does (`0.00001`, `1e-6`, `1e+16`, `100.0`)."""
    if not math.isfinite(value):
        raise ProtocolError(f"{value!r} is not a number of the format")
    text = repr(float(value))
    sign = ""
    if text.startswith("-"):
        sign = "-"
        text = text[1:]
    exponent = 0
    if "e" in text:
        text, exponent_text = text.split("e")
        exponent = int(exponent_text)
    integer_part, _, fraction_part = text.partition(".")
    digits = integer_part + fraction_part
    k = exponent - len(fraction_part)
    stripped = digits.rstrip("0")
    k += len(digits) - len(stripped)
    digits = stripped.lstrip("0")
    if not digits:
        return sign + "0.0"
    length = len(digits)
    kk = length + k
    if k >= 0 and kk <= 16:
        body = digits + "0" * k + ".0"
    elif 0 < kk <= 16:
        body = digits[:kk] + "." + digits[kk:]
    elif -5 < kk <= 0:
        body = "0." + "0" * (-kk) + digits
    else:
        body = digits[0]
        if length > 1:
            body += "." + digits[1:]
        power = kk - 1
        body += "e" + ("+" if power > 0 else "-") + str(abs(power))
    return sign + body


def _scalar(value: object) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        raise ProtocolError("the protocol has no booleans")
    if isinstance(value, int):
        return str(value)
    if isinstance(value, float):
        return format_float(value)
    if isinstance(value, str):
        return json.dumps(value, ensure_ascii=False)
    raise ProtocolError(f"cannot write a {type(value).__name__}")


def _compact(value: object) -> str:
    if isinstance(value, dict):
        entries = [f"{_scalar(key)}:{_compact(item)}" for key, item in value.items()]
        return "{" + ",".join(entries) + "}"
    if isinstance(value, list):
        return "[" + ",".join(_compact(item) for item in value) + "]"
    return _scalar(value)


def _pretty(value: object, depth: int) -> str:
    inner = "  " * (depth + 1)
    outer = "  " * depth
    if isinstance(value, dict):
        if not value:
            return "{}"
        entries = [
            f"{inner}{_scalar(key)}: {_pretty(item, depth + 1)}" for key, item in value.items()
        ]
        return "{\n" + ",\n".join(entries) + "\n" + outer + "}"
    if isinstance(value, list):
        if not value:
            return "[]"
        entries = [inner + _pretty(item, depth + 1) for item in value]
        return "[\n" + ",\n".join(entries) + "\n" + outer + "]"
    return _scalar(value)


def dumps(data: object, pretty: bool = False) -> str:
    """Writes plain data (`None`, `int`, `float`, `str`, `list`, `dict` with `str` keys, in
    the dict's own order) as the engine writes JSON: compact on one line, or in the layout
    of serde_json's `to_string_pretty` (two-space indent, no trailing newline)."""
    return _pretty(data, 0) if pretty else _compact(data)


# ---------------------------------------------------------------------------------------
# Plain data to and from the messages
# ---------------------------------------------------------------------------------------


def _is_number(value: object) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def _record(data: object, what: str, fields: tuple[str, ...], optional: tuple[str, ...] = ()):
    """Checks `data` is a table whose keys are all among `fields` and include every
    required one."""
    if not isinstance(data, dict):
        raise ProtocolError(f"a {what} must be a table")
    for key in data:
        if key not in fields:
            raise ProtocolError(f"unknown key '{key}' in {what}")
    for key in fields:
        if key not in data and key not in optional:
            raise ProtocolError(f"missing key '{key}' in {what}")
    return data


def _string(value: object, what: str) -> str:
    if not isinstance(value, str):
        raise ProtocolError(f"{what} must be a string")
    return value


def _whole(value: object, what: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise ProtocolError(f"{what} must be a whole number")
    return value


def _number(value: object, what: str) -> Number:
    if not _is_number(value) or (isinstance(value, float) and not math.isfinite(value)):
        raise ProtocolError(f"{what} must be a number")
    return value  # type: ignore[return-value]


def _table(value: object, what: str) -> dict:
    if not isinstance(value, dict):
        raise ProtocolError(f"{what} must be a table")
    return value


def _list(value: object, what: str) -> list:
    if not isinstance(value, list):
        raise ProtocolError(f"{what} must be a list")
    return value


def _numbers(value: object, what: str) -> dict[str, Number]:
    return {key: _number(item, f"{what} '{key}'") for key, item in _table(value, what).items()}


def _factory_id(key: str) -> int:
    if not (key.isascii() and key.isdigit()):
        raise ProtocolError(f"a factory key must be a whole number, not '{key}'")
    return int(key)


def _sorted(table: dict) -> list:
    return sorted(table.items())


def _numbers_data(table: dict[str, Number]) -> dict[str, Number]:
    return dict(_sorted(table))


def _group_from_data(data: object) -> PerishGroup:
    data = _record(data, "perish group", ("amount", "perishes"))
    perishes = data["perishes"]
    if perishes is not None:
        perishes = _whole(perishes, "perishes")
    return PerishGroup(_number(data["amount"], "amount"), perishes)


def _group_to_data(group: PerishGroup) -> dict:
    return {"amount": group.amount, "perishes": group.perishes}


def _factory_from_data(data: object) -> FactoryView:
    data = _record(data, "factory", ("technology", "condition"))
    return FactoryView(
        _string(data["technology"], "technology"), _number(data["condition"], "condition")
    )


def _factory_to_data(factory: FactoryView) -> dict:
    return {"technology": factory.technology, "condition": factory.condition}


_TECHNOLOGY_FIELDS = (
    "inputs",
    "output",
    "capacity",
    "capital",
    "construction",
    "construction_turns",
    "lifetime",
)


def _technology_from_data(data: object) -> TechnologyView:
    data = _record(data, "technology", _TECHNOLOGY_FIELDS)
    return TechnologyView(
        inputs=_numbers(data["inputs"], "inputs"),
        output=_string(data["output"], "output"),
        capacity=_number(data["capacity"], "capacity"),
        capital=_numbers(data["capital"], "capital"),
        construction=_numbers(data["construction"], "construction"),
        construction_turns=_whole(data["construction_turns"], "construction_turns"),
        lifetime=_whole(data["lifetime"], "lifetime"),
    )


def _technology_to_data(technology: TechnologyView) -> dict:
    return {
        "inputs": _numbers_data(technology.inputs),
        "output": technology.output,
        "capacity": technology.capacity,
        "capital": _numbers_data(technology.capital),
        "construction": _numbers_data(technology.construction),
        "construction_turns": technology.construction_turns,
        "lifetime": technology.lifetime,
    }


def _deposits_from_data(data: object) -> DepositsView:
    data = _record(data, "deposits", ("count", "produces", "output"))
    return DepositsView(
        _whole(data["count"], "count"),
        _string(data["produces"], "produces"),
        _number(data["output"], "output"),
    )


def _deposits_to_data(deposits: DepositsView) -> dict:
    return {"count": deposits.count, "produces": deposits.produces, "output": deposits.output}


_VIEW_FIELDS = (
    "turn",
    "business",
    "currency",
    "cash",
    "prices",
    "stockpile",
    "factories",
    "technologies",
    "deposits",
)


def view_from_data(data: object) -> BusinessView:
    """A view from plain JSON data."""
    data = _record(data, "view", _VIEW_FIELDS)
    stockpile = {
        good: [_group_from_data(group) for group in _list(groups, f"stockpile '{good}'")]
        for good, groups in _table(data["stockpile"], "stockpile").items()
    }
    return BusinessView(
        turn=_whole(data["turn"], "turn"),
        business=_string(data["business"], "business"),
        currency=_string(data["currency"], "currency"),
        cash=_number(data["cash"], "cash"),
        prices=_numbers(data["prices"], "prices"),
        stockpile=stockpile,
        factories={
            _factory_id(key): _factory_from_data(item)
            for key, item in _table(data["factories"], "factories").items()
        },
        technologies={
            name: _technology_from_data(item)
            for name, item in _table(data["technologies"], "technologies").items()
        },
        deposits={
            name: _deposits_from_data(item)
            for name, item in _table(data["deposits"], "deposits").items()
        },
    )


def view_to_data(view: BusinessView) -> dict:
    """A view as plain JSON data, in the engine's key order with tables sorted."""
    return {
        "turn": view.turn,
        "business": view.business,
        "currency": view.currency,
        "cash": view.cash,
        "prices": _numbers_data(view.prices),
        "stockpile": {
            good: [_group_to_data(group) for group in groups]
            for good, groups in _sorted(view.stockpile)
        },
        "factories": {
            str(key): _factory_to_data(item) for key, item in sorted(view.factories.items())
        },
        "technologies": {
            name: _technology_to_data(item) for name, item in _sorted(view.technologies)
        },
        "deposits": {name: _deposits_to_data(item) for name, item in _sorted(view.deposits)},
    }


def _order_from_data(data: object) -> FactoryOrder:
    data = _record(data, "factory order", ("run", "maintenance"), ("run", "maintenance"))
    return FactoryOrder(
        _number(data["run"], "run") if "run" in data else 0.0,
        _number(data["maintenance"], "maintenance") if "maintenance" in data else 0.0,
    )


def _order_to_data(order: FactoryOrder) -> dict:
    return {"run": order.run, "maintenance": order.maintenance}


_LINE_FIELDS = ("quantity", "slope", "maximum", "give_up")


def _line_from_data(data: object, what: str) -> OrderLine:
    data = _record(data, what, _LINE_FIELDS)
    return OrderLine(*(_number(data[key], f"{what} {key}") for key in _LINE_FIELDS))


def _line_to_data(line: OrderLine) -> dict:
    return {
        "quantity": line.quantity,
        "slope": line.slope,
        "maximum": line.maximum,
        "give_up": line.give_up,
    }


def _lines(value: object, what: str) -> dict[str, OrderLine]:
    return {
        good: _line_from_data(item, f"{what} '{good}'")
        for good, item in _table(value, f"{what}s").items()
    }


def _lines_data(table: dict[str, OrderLine]) -> dict:
    return {good: _line_to_data(line) for good, line in _sorted(table)}


_DECISION_FIELDS = ("business", "factories", "offers", "bids", "trace")


def decision_from_data(data: object) -> Decision:
    """A decision from plain JSON data."""
    data = _record(data, "decision", _DECISION_FIELDS, _DECISION_FIELDS[1:])
    trace = data.get("trace")
    if trace is not None:
        trace = _string(trace, "trace")
    return Decision(
        business=_string(data["business"], "business"),
        factories={
            _factory_id(key): _order_from_data(item)
            for key, item in _table(data.get("factories", {}), "factories").items()
        },
        offers=_lines(data.get("offers", {}), "offer"),
        bids=_lines(data.get("bids", {}), "bid"),
        trace=trace,
    )


def decision_to_data(decision: Decision) -> dict:
    """A decision as plain JSON data; `trace` is written only when there is one."""
    data = {
        "business": decision.business,
        "factories": {
            str(key): _order_to_data(order) for key, order in sorted(decision.factories.items())
        },
        "offers": _lines_data(decision.offers),
        "bids": _lines_data(decision.bids),
    }
    if decision.trace is not None:
        data["trace"] = decision.trace
    return data


def request_from_data(data: object) -> Request:
    """A request from plain JSON data."""
    data = _record(data, "request", ("turn", "views"))
    return Request(
        _whole(data["turn"], "turn"),
        [view_from_data(item) for item in _list(data["views"], "views")],
    )


def request_to_data(request: Request) -> dict:
    """A request as plain JSON data."""
    return {"turn": request.turn, "views": [view_to_data(view) for view in request.views]}


def reply_from_data(data: object) -> Reply:
    """A reply from plain JSON data."""
    data = _record(data, "reply", ("turn", "decisions"), ("decisions",))
    return Reply(
        _whole(data["turn"], "turn"),
        [decision_from_data(item) for item in _list(data.get("decisions", []), "decisions")],
    )


def reply_to_data(reply: Reply) -> dict:
    """A reply as plain JSON data; `decisions` is always written."""
    return {
        "turn": reply.turn,
        "decisions": [decision_to_data(decision) for decision in reply.decisions],
    }


def recording_from_data(data: object) -> Recording:
    """A recording from plain JSON data."""
    data = _record(data, "recording", ("request", "reply"))
    return Recording(request_from_data(data["request"]), reply_from_data(data["reply"]))


def recording_to_data(recording: Recording) -> dict:
    """A recording as plain JSON data."""
    return {
        "request": request_to_data(recording.request),
        "reply": reply_to_data(recording.reply),
    }


# ---------------------------------------------------------------------------------------
# Text to and from the messages
# ---------------------------------------------------------------------------------------


def read_view(text: str) -> BusinessView:
    """Reads a view from JSON text."""
    return view_from_data(loads(text))


def write_view(view: BusinessView, pretty: bool = False) -> str:
    """A view as JSON text."""
    return dumps(view_to_data(view), pretty)


def read_decision(text: str) -> Decision:
    """Reads a decision from JSON text."""
    return decision_from_data(loads(text))


def write_decision(decision: Decision, pretty: bool = False) -> str:
    """A decision as JSON text."""
    return dumps(decision_to_data(decision), pretty)


def read_request(text: str) -> Request:
    """Reads a request from JSON text."""
    return request_from_data(loads(text))


def write_request(request: Request, pretty: bool = False) -> str:
    """A request as JSON text."""
    return dumps(request_to_data(request), pretty)


def read_reply(text: str) -> Reply:
    """Reads a reply from JSON text."""
    return reply_from_data(loads(text))


def write_reply(reply: Reply, pretty: bool = False) -> str:
    """A reply as JSON text."""
    return dumps(reply_to_data(reply), pretty)


def read_recording(text: str) -> Recording:
    """Reads a recording from JSON text."""
    return recording_from_data(loads(text))


def write_recording(recording: Recording, pretty: bool = False) -> str:
    """A recording as JSON text."""
    return dumps(recording_to_data(recording), pretty)
