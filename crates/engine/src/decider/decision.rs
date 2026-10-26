//! The decision: a business's answer for one turn, and its validation against the view it
//! answers.
//!
//! A decision names its business and carries, per factory, a run and a maintenance fraction
//! (a factory left out is left alone), an order line per good to sell and per good to buy,
//! and an optional free-text trace the engine stores and never acts on. A line carries a
//! quantity, a slope, a maximum and a give-up price; the engine anchors it at the good's
//! posted price in the view and checks it with Step 01's rules ([`Line::new`]). Validation
//! refuses nonsense and reports every problem at its place, in a fixed order, as the setting
//! loader does; asking for more than the business holds or can pay is a valid request (the
//! trade round trims it).

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::decider::view::BusinessView;
use crate::decider::{self, FormatError};
use crate::error_list::ErrorList;
use crate::goods::Goods;
use crate::orders::{Line, LineParams, Side};

/// A business's answer for one turn.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// The business it answers for.
    pub business: String,
    /// Orders per factory, by ID number. A factory not listed is left alone (run 0,
    /// maintenance 0). Absent in JSON means empty.
    #[serde(default, deserialize_with = "crate::decider::no_duplicate_keys")]
    pub factories: BTreeMap<u32, FactoryOrder>,
    /// Lines to sell, by good name. Absent in JSON means empty.
    #[serde(default, deserialize_with = "crate::decider::no_duplicate_keys")]
    pub offers: BTreeMap<String, OrderLine>,
    /// Lines to buy, by good name. Absent in JSON means empty.
    #[serde(default, deserialize_with = "crate::decider::no_duplicate_keys")]
    pub bids: BTreeMap<String, OrderLine>,
    /// The decider's account of its choice, free text, any number of lines. Stored for the
    /// dev UI, never acted on. Absent in JSON when None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
}

/// One order line as a decision carries it: what the business trades at the posted price
/// of the good, how that changes with the price, the most it would trade, and the last
/// price at which it still trades. The engine anchors the line at the good's posted price
/// in the view the decision answers; the decider sends no anchor. All four numbers are
/// required.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderLine {
    /// The quantity at the posted price: finite, at least 0, at most `maximum`.
    pub quantity: f64,
    /// Quantity change per unit of price: finite; at most 0 in a bid, at least 0 in an offer.
    pub slope: f64,
    /// The most the business trades at any price: finite, at least 0.
    pub maximum: f64,
    /// The last price it still trades at (an offer sells nothing below it, a bid buys
    /// nothing above it): finite, at least 0.
    pub give_up: f64,
}

impl OrderLine {
    /// A flat line: `amount` at every price the give-up allows (slope 0, maximum `amount`).
    /// "Sell this much at or above `give_up`" in an offer, "buy this much at or below it"
    /// in a bid.
    pub fn flat(amount: f64, give_up: f64) -> OrderLine {
        OrderLine {
            quantity: amount,
            slope: 0.0,
            maximum: amount,
            give_up,
        }
    }

    /// The line's numbers as Step 01's parameters, anchored at `anchor_price`.
    pub fn params(&self, anchor_price: f64) -> LineParams {
        LineParams {
            anchor_price,
            quantity: self.quantity,
            slope: self.slope,
            maximum: self.maximum,
            give_up: self.give_up,
        }
    }
}

/// The two fractions for one factory. A fraction left out of the JSON is 0.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FactoryOrder {
    /// The share of capacity to use: finite, 0 to 1.
    #[serde(default)]
    pub run: f64,
    /// The share of the full capital bundle to replenish: finite, 0 to 1.
    #[serde(default)]
    pub maintenance: f64,
}

/// Where in a decision a problem is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// The decision as a whole.
    Decision,
    /// The order for the factory with this ID number.
    Factory(u32),
    /// The offer of this good.
    Offer(String),
    /// The bid for this good.
    Bid(String),
}

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Place::Decision => write!(f, "decision"),
            Place::Factory(id) => write!(f, "factory #{id}"),
            Place::Offer(good) => write!(f, "offer '{good}'"),
            Place::Bid(good) => write!(f, "bid '{good}'"),
        }
    }
}

/// One problem found in a decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionError {
    place: Place,
    message: String,
}

impl DecisionError {
    fn new(place: Place, message: impl Into<String>) -> DecisionError {
        DecisionError {
            place,
            message: message.into(),
        }
    }

    /// Where the problem is.
    pub fn place(&self) -> &Place {
        &self.place
    }

    /// The message, without the place.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for DecisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.place, self.message)
    }
}

impl Error for DecisionError {}

/// Every problem found in one refused decision, in document order. Never empty.
pub type DecisionErrors = ErrorList<DecisionError>;

/// The problems with one factory's two fractions.
fn fraction_problems(id: u32, order: &FactoryOrder, problems: &mut Vec<DecisionError>) {
    for (name, value) in [("run", order.run), ("maintenance", order.maintenance)] {
        if !decider::is_fraction(value) {
            problems.push(DecisionError::new(
                Place::Factory(id),
                format!("{name} must be a finite number from 0 to 1 (got {value})"),
            ));
        }
    }
}

/// The anchor used to check a line's shape when the good has no posted price (or there is
/// no view): any positive number does, because the shape checks never read it.
const STAND_IN_ANCHOR: f64 = 1.0;

/// The problems with one line, in Step 01's field order, each at `place`: `Line::new`'s
/// errors with `anchor` as the anchor price, displayed with `LineError`'s `Display`.
fn line_problems(
    place: &Place,
    side: Side,
    line: &OrderLine,
    anchor: f64,
    problems: &mut Vec<DecisionError>,
) {
    if let Err(errors) = Line::new(side, &line.params(anchor)) {
        for error in &errors {
            problems.push(DecisionError::new(place.clone(), error.to_string()));
        }
    }
}

impl Decision {
    /// A decision that leaves every factory alone and trades nothing: the legitimate
    /// "do nothing".
    pub fn empty(business: impl Into<String>) -> Decision {
        Decision {
            business: business.into(),
            factories: BTreeMap::new(),
            offers: BTreeMap::new(),
            bids: BTreeMap::new(),
            trace: None,
        }
    }

    /// Checks the decision against the view it answers and the setting's goods. Reads the
    /// whole decision and reports every problem, each at its place, in document order:
    /// the decision itself, then the factories by ID, then the offers, then the bids by
    /// name. Asking for more than the business holds or can pay is not a problem here.
    pub fn validate(&self, view: &BusinessView, goods: &Goods) -> Result<(), DecisionErrors> {
        let mut problems = Vec::new();
        if self.business != view.business {
            problems.push(DecisionError::new(
                Place::Decision,
                format!(
                    "answers for '{}' against the view of '{}'",
                    self.business, view.business
                ),
            ));
        }
        for (id, order) in &self.factories {
            if !view.factories.contains_key(id) {
                problems.push(DecisionError::new(
                    Place::Factory(*id),
                    "not in this business's view",
                ));
            }
            fraction_problems(*id, order, &mut problems);
        }
        let sections = [
            (
                &self.offers,
                Side::Supply,
                Place::Offer as fn(String) -> Place,
            ),
            (&self.bids, Side::Demand, Place::Bid as fn(String) -> Place),
        ];
        for (table, side, place_of) in sections {
            for (good, line) in table {
                let place = place_of(good.clone());
                let anchor = view
                    .prices
                    .get(good)
                    .map_or(STAND_IN_ANCHOR, |price| price.get());
                line_problems(&place, side, line, anchor, &mut problems);
                if goods.id(good).is_err() {
                    problems.push(DecisionError::new(place, "unknown good"));
                } else if *good == view.currency {
                    problems.push(DecisionError::new(
                        place,
                        "it is this hex's currency, which can't be traded against itself",
                    ));
                } else if !view.prices.contains_key(good) {
                    problems.push(DecisionError::new(
                        place,
                        "no price is posted for it on this hex",
                    ));
                }
            }
        }
        if problems.is_empty() {
            Ok(())
        } else {
            Err(ErrorList::new(problems))
        }
    }

    /// Compact JSON, one line.
    pub fn to_json(&self) -> String {
        decider::to_json(self)
    }

    /// Indented JSON.
    pub fn to_json_pretty(&self) -> String {
        decider::to_json_pretty(self)
    }

    /// Reads a decision, refusing JSON that isn't the format or holds a fraction or a
    /// line that isn't a valid value (the same checks `validate` makes on numbers).
    pub fn from_json(text: &str) -> Result<Decision, FormatError> {
        let decision: Decision = decider::from_json(text)?;
        decider::checked(
            decision
                .number_problems()
                .iter()
                .map(ToString::to_string)
                .collect(),
        )?;
        Ok(decision)
    }

    /// The number problems, in document order: for each factory, a `run` or `maintenance`
    /// that isn't a fraction; for each offer and bid, the line's shape problems (checked
    /// against a stand-in anchor, as there is no view here).
    pub(crate) fn number_problems(&self) -> Vec<DecisionError> {
        let mut problems = Vec::new();
        for (id, order) in &self.factories {
            fraction_problems(*id, order, &mut problems);
        }
        for (good, line) in &self.offers {
            let place = Place::Offer(good.clone());
            line_problems(&place, Side::Supply, line, STAND_IN_ANCHOR, &mut problems);
        }
        for (good, line) in &self.bids {
            let place = Place::Bid(good.clone());
            line_problems(&place, Side::Demand, line, STAND_IN_ANCHOR, &mut problems);
        }
        problems
    }
}

/// Goods `labor`, `grain`, `copper` and `silver`, for the decider's unit tests.
#[cfg(test)]
pub(crate) fn test_goods() -> Goods {
    use crate::goods::{Good, Perishes};
    use crate::id::{NameRegistry, NamedList};

    let mut names = NameRegistry::new();
    let mut items = Vec::new();
    for name in ["labor", "grain", "copper", "silver"] {
        names.register(name).unwrap();
        items.push(Good {
            perishes: Perishes::Never,
            movable: true,
        });
    }
    NamedList::from_parts(names, items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decider::view::test_view;

    fn order(run: f64, maintenance: f64) -> FactoryOrder {
        FactoryOrder { run, maintenance }
    }

    fn flat(amount: f64, give_up: f64) -> OrderLine {
        OrderLine::flat(amount, give_up)
    }

    fn line(quantity: f64, slope: f64, maximum: f64, give_up: f64) -> OrderLine {
        OrderLine {
            quantity,
            slope,
            maximum,
            give_up,
        }
    }

    fn lines(items: &[(&str, OrderLine)]) -> BTreeMap<String, OrderLine> {
        items.iter().map(|(n, l)| (n.to_string(), *l)).collect()
    }

    fn full() -> Decision {
        Decision {
            business: "a".to_string(),
            factories: BTreeMap::from([(0, order(0.5, 0.25))]),
            offers: lines(&[("grain", flat(3.0, 2.0))]),
            bids: lines(&[("labor", flat(2.0, 1.0))]),
            trace: Some("because".to_string()),
        }
    }

    fn messages(errors: &DecisionErrors) -> Vec<String> {
        errors.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn an_empty_decision_is_valid() {
        assert_eq!(
            Decision::empty("a").validate(&test_view(), &test_goods()),
            Ok(())
        );
    }

    #[test]
    fn a_full_decision_is_valid() {
        assert_eq!(full().validate(&test_view(), &test_goods()), Ok(()));
    }

    #[test]
    fn asking_for_too_much_is_valid() {
        let mut d = Decision::empty("a");
        d.offers = lines(&[("grain", flat(1e6, 2.0))]);
        d.bids = lines(&[("labor", flat(1e9, 1.0)), ("grain", flat(5.0, 2.0))]);
        assert_eq!(d.validate(&test_view(), &test_goods()), Ok(()));
    }

    #[test]
    fn lines_that_trade_nothing_and_sloped_lines_are_valid() {
        let mut d = Decision::empty("a");
        d.offers = lines(&[("grain", line(2.0, 0.5, 6.0, 1.0))]);
        d.bids = lines(&[
            ("labor", line(2.0, -0.5, 6.0, 3.0)),
            ("grain", line(0.0, 0.0, 5.0, 2.0)),
        ]);
        assert_eq!(d.validate(&test_view(), &test_goods()), Ok(()));
        d.offers = lines(&[("grain", line(0.0, 0.0, 0.0, 0.0))]);
        d.bids = lines(&[("labor", flat(0.0, 0.0))]);
        assert_eq!(d.validate(&test_view(), &test_goods()), Ok(()));
    }

    #[test]
    fn every_refusal_is_reported_at_its_place() {
        let d = Decision {
            business: "a".to_string(),
            factories: BTreeMap::from([(1, order(f64::NAN, 0.0)), (2, order(1.5, -0.1))]),
            offers: lines(&[
                ("silver", flat(1.0, 1.0)),
                ("copper", flat(1.0, 1.0)),
                ("grain", line(-2.0, -1.0, 4.0, 1.0)),
            ]),
            bids: lines(&[
                ("wheat", flat(1.0, 1.0)),
                ("labor", line(1.0, 1.5, 4.0, -1.0)),
                ("grain", line(5.0, 0.0, 4.0, 2.0)),
            ]),
            trace: None,
        };
        let errors = d.validate(&test_view(), &test_goods()).unwrap_err();
        assert_eq!(
            messages(&errors),
            [
                "factory #1: run must be a finite number from 0 to 1 (got NaN)",
                "factory #2: not in this business's view",
                "factory #2: run must be a finite number from 0 to 1 (got 1.5)",
                "factory #2: maintenance must be a finite number from 0 to 1 (got -0.1)",
                "offer 'copper': no price is posted for it on this hex",
                "offer 'grain': quantity must be a finite number of at least 0 (got -2)",
                "offer 'grain': slope of a supply line must be at least 0 (got -1)",
                "offer 'silver': it is this hex's currency, which can't be traded against itself",
                "bid 'grain': quantity at the anchor price (5) is above the maximum (4)",
                "bid 'labor': slope of a demand line must be at most 0 (got 1.5)",
                "bid 'labor': give-up price must be a finite number of at least 0 (got -1)",
                "bid 'wheat': unknown good",
            ]
        );
        assert_eq!(errors.len(), 12);
    }

    #[test]
    fn non_finite_numbers_are_reported_by_field() {
        let mut d = Decision::empty("a");
        d.offers = lines(&[("grain", line(1.0, f64::NAN, f64::INFINITY, 1.0))]);
        let errors = d.validate(&test_view(), &test_goods()).unwrap_err();
        assert_eq!(
            messages(&errors),
            [
                "offer 'grain': slope must be a finite number (got NaN)",
                "offer 'grain': maximum must be a finite number of at least 0 (got inf)",
            ]
        );
    }

    #[test]
    fn a_line_problem_and_a_good_problem_are_both_reported() {
        let mut d = Decision::empty("a");
        d.offers = lines(&[("wheat", line(1.0, -1.0, 2.0, 1.0))]);
        let errors = d.validate(&test_view(), &test_goods()).unwrap_err();
        assert_eq!(
            messages(&errors),
            [
                "offer 'wheat': slope of a supply line must be at least 0 (got -1)",
                "offer 'wheat': unknown good",
            ]
        );
    }

    #[test]
    fn a_decision_for_another_business_is_refused() {
        let errors = Decision::empty("b")
            .validate(&test_view(), &test_goods())
            .unwrap_err();
        assert_eq!(
            messages(&errors),
            ["decision: answers for 'b' against the view of 'a'"]
        );
    }

    #[test]
    fn a_decision_round_trips_through_json() {
        let mut d = full();
        d.trace = Some("line one\nline two".to_string());
        assert_eq!(Decision::from_json(&d.to_json()).unwrap(), d);
        assert_eq!(Decision::from_json(&d.to_json_pretty()).unwrap(), d);
    }

    #[test]
    fn sloped_lines_with_awkward_numbers_round_trip() {
        let mut d = full();
        d.offers = lines(&[("grain", line(1.0 / 3.0, 5e-324, f64::MAX, 2.0 / 3.0))]);
        d.bids = lines(&[("labor", line(0.1, -1.0 / 7.0, f64::MAX, 1e-300))]);
        assert_eq!(Decision::from_json(&d.to_json()).unwrap(), d);
        assert_eq!(Decision::from_json(&d.to_json_pretty()).unwrap(), d);
    }

    #[test]
    fn an_empty_decision_writes_empty_tables_and_no_trace() {
        assert_eq!(
            Decision::empty("a").to_json(),
            r#"{"business":"a","factories":{},"offers":{},"bids":{}}"#
        );
    }

    #[test]
    fn a_line_is_written_with_its_four_numbers_in_order() {
        let mut d = Decision::empty("a");
        d.offers = lines(&[("grain", flat(8.0, 2.0))]);
        assert!(
            d.to_json().contains(
                r#""offers":{"grain":{"quantity":8.0,"slope":0.0,"maximum":8.0,"give_up":2.0}}"#
            ),
            "{}",
            d.to_json()
        );
    }

    #[test]
    fn a_hand_written_decision_reads_as_expected() {
        let text = r#"{"business": "a", "factories": {"0": {"run": 1}}, "offers": {"grain": {"quantity": 8, "slope": 0, "maximum": 8, "give_up": 0.5}}, "bids": {"labor": {"quantity": 6, "slope": 0, "maximum": 6, "give_up": 1}}, "trace": "sell the grain"}"#;
        let d = Decision::from_json(text).unwrap();
        assert_eq!(d.factories, BTreeMap::from([(0, order(1.0, 0.0))]));
        assert_eq!(d.offers, lines(&[("grain", flat(8.0, 0.5))]));
        assert_eq!(d.bids, lines(&[("labor", flat(6.0, 1.0))]));
        assert_eq!(d.trace.as_deref(), Some("sell the grain"));
        assert_eq!(
            Decision::from_json(r#"{"business": "a"}"#).unwrap(),
            Decision::empty("a")
        );
    }

    #[test]
    fn reading_refuses_malformed_json_with_a_message() {
        match Decision::from_json(r#"{"business": "#) {
            Err(FormatError::Json(m)) => assert!(!m.is_empty()),
            other => panic!("{other:?}"),
        }
        const GRAIN: &str = r#""grain":{"quantity":1,"slope":0,"maximum":1,"give_up":1}"#;
        for (text, expected) in [
            (r#"{"business": 5}"#.to_string(), "invalid type"),
            (
                r#"{"business":"a","offer":{}}"#.to_string(),
                "unknown field",
            ),
            (
                format!(r#"{{"business":"a","offers":{{{GRAIN},{GRAIN}}}}}"#),
                "the key 'grain' is written twice",
            ),
            (
                r#"{"business":"a","factories":{"0":{"run":"all"}}}"#.to_string(),
                "invalid type",
            ),
            (
                r#"{"business":"a","offers":{"grain":{"quantity":1,"slope":0,"maximum":1}}}"#
                    .to_string(),
                "missing field `give_up`",
            ),
            (
                r#"{"business":"a","bids":{"grain":{"quantity":1,"slope":0,"maximum":1,"give_up":1,"anchor":2}}}"#
                    .to_string(),
                "unknown field",
            ),
            (
                r#"{"business":"a","offers":{"grain":{"quantity":"lots","slope":0,"maximum":1,"give_up":1}}}"#
                    .to_string(),
                "invalid type",
            ),
            (
                r#"{"business":"a","offers":{"grain":8}}"#.to_string(),
                "invalid type",
            ),
        ] {
            let error = Decision::from_json(&text).unwrap_err();
            assert!(matches!(error, FormatError::Json(_)), "{text}");
            assert!(error.to_string().contains(expected), "{text}: {error}");
        }
        assert_eq!(
            Decision::from_json(r#"{"business":"a","factories":{"0":{"run":2}}}"#),
            Err(FormatError::Values(vec![
                "factory #0: run must be a finite number from 0 to 1 (got 2)".to_string()
            ]))
        );
        for text in [
            r#"{"business":"a","offers":{"grain":{"quantity":1e999,"slope":0,"maximum":1,"give_up":1}}}"#,
            r#"{"business":"a","offers":{"grain":{"quantity":NaN,"slope":0,"maximum":1,"give_up":1}}}"#,
        ] {
            let error = Decision::from_json(text).unwrap_err();
            assert!(!error.to_string().is_empty(), "{text}");
        }
    }

    #[test]
    fn reading_refuses_bad_line_values_with_a_message_at_the_place() {
        for (text, expected) in [
            (
                r#"{"business":"a","offers":{"grain":{"quantity":-1,"slope":0,"maximum":1,"give_up":1}}}"#,
                "offer 'grain': quantity must be a finite number of at least 0 (got -1)",
            ),
            (
                r#"{"business":"a","bids":{"labor":{"quantity":1,"slope":1,"maximum":1,"give_up":1}}}"#,
                "bid 'labor': slope of a demand line must be at most 0 (got 1)",
            ),
            (
                r#"{"business":"a","offers":{"grain":{"quantity":5,"slope":0,"maximum":4,"give_up":1}}}"#,
                "offer 'grain': quantity at the anchor price (5) is above the maximum (4)",
            ),
        ] {
            assert_eq!(
                Decision::from_json(text),
                Err(FormatError::Values(vec![expected.to_string()])),
                "{text}"
            );
        }
    }

    #[test]
    fn a_flat_line_and_its_params() {
        let l = OrderLine::flat(8.0, 0.5);
        assert_eq!(l, line(8.0, 0.0, 8.0, 0.5));
        assert_eq!(
            l.params(2.0),
            LineParams {
                anchor_price: 2.0,
                quantity: 8.0,
                slope: 0.0,
                maximum: 8.0,
                give_up: 0.5,
            }
        );
    }

    #[test]
    fn error_displays() {
        assert_eq!(Place::Decision.to_string(), "decision");
        assert_eq!(Place::Factory(3).to_string(), "factory #3");
        assert_eq!(
            Place::Offer("grain".to_string()).to_string(),
            "offer 'grain'"
        );
        assert_eq!(Place::Bid("grain".to_string()).to_string(), "bid 'grain'");
        let error = DecisionError::new(Place::Factory(3), "not in this business's view");
        assert_eq!(error.to_string(), "factory #3: not in this business's view");
        assert_eq!(error.place(), &Place::Factory(3));
        assert_eq!(error.message(), "not in this business's view");
        let errors = ErrorList::new(vec![error, DecisionError::new(Place::Decision, "second")]);
        assert_eq!(
            errors.to_string(),
            "factory #3: not in this business's view\ndecision: second"
        );
        assert!(!errors.is_empty());
        assert_eq!((&errors).into_iter().count(), 2);
    }
}
