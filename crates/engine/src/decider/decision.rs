//! The decision: a business's answer for one turn, and its validation against the view it
//! answers.
//!
//! A decision names its business and carries, per factory, a run and a maintenance fraction
//! (a factory left out is left alone), offers and bids per good, and an optional free-text
//! trace the engine stores and never acts on. Validation refuses nonsense and reports every
//! problem at its place, in a fixed order, as the setting loader does; asking for more than
//! the business holds or can pay is a valid request (Step 08 trims it).

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::decider::view::BusinessView;
use crate::decider::{self, FormatError};
use crate::error_list::ErrorList;
use crate::goods::Goods;

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
    /// Amounts to sell, by good name. Absent in JSON means empty.
    #[serde(default, deserialize_with = "crate::decider::no_duplicate_keys")]
    pub offers: BTreeMap<String, f64>,
    /// Amounts to buy, by good name. Absent in JSON means empty.
    #[serde(default, deserialize_with = "crate::decider::no_duplicate_keys")]
    pub bids: BTreeMap<String, f64>,
    /// The decider's account of its choice, free text, any number of lines. Stored for the
    /// dev UI, never acted on. Absent in JSON when None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
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

/// The problem with one amount, if any.
fn amount_problem(place: &Place, amount: f64, problems: &mut Vec<DecisionError>) {
    if !decider::is_amount(amount) {
        problems.push(DecisionError::new(
            place.clone(),
            format!("amount must be a finite number of at least 0 (got {amount})"),
        ));
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
            (&self.offers, Place::Offer as fn(String) -> Place),
            (&self.bids, Place::Bid as fn(String) -> Place),
        ];
        for (table, place_of) in sections {
            for (good, amount) in table {
                let place = place_of(good.clone());
                amount_problem(&place, *amount, &mut problems);
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

    /// Reads a decision, refusing JSON that isn't the format or holds a fraction or an
    /// amount that isn't a valid value (the same checks `validate` makes on numbers).
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
    /// that isn't a fraction; for each offer and bid, an amount that isn't a valid quantity.
    pub(crate) fn number_problems(&self) -> Vec<DecisionError> {
        let mut problems = Vec::new();
        for (id, order) in &self.factories {
            fraction_problems(*id, order, &mut problems);
        }
        for (good, amount) in &self.offers {
            amount_problem(&Place::Offer(good.clone()), *amount, &mut problems);
        }
        for (good, amount) in &self.bids {
            amount_problem(&Place::Bid(good.clone()), *amount, &mut problems);
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

    fn table(items: &[(&str, f64)]) -> BTreeMap<String, f64> {
        items.iter().map(|(n, a)| (n.to_string(), *a)).collect()
    }

    fn full() -> Decision {
        Decision {
            business: "a".to_string(),
            factories: BTreeMap::from([(0, order(0.5, 0.25))]),
            offers: table(&[("grain", 3.0)]),
            bids: table(&[("labor", 2.0)]),
            trace: Some("because".to_string()),
        }
    }

    fn lines(errors: &DecisionErrors) -> Vec<String> {
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
        d.offers = table(&[("grain", 1e6)]);
        d.bids = table(&[("labor", 1e9), ("grain", 5.0)]);
        assert_eq!(d.validate(&test_view(), &test_goods()), Ok(()));
    }

    #[test]
    fn every_refusal_is_reported_at_its_place() {
        let d = Decision {
            business: "a".to_string(),
            factories: BTreeMap::from([(1, order(f64::NAN, 0.0)), (2, order(1.5, -0.1))]),
            offers: table(&[("silver", 1.0), ("copper", 1.0), ("grain", -2.0)]),
            bids: table(&[("wheat", 1.0), ("labor", f64::INFINITY)]),
            trace: None,
        };
        let errors = d.validate(&test_view(), &test_goods()).unwrap_err();
        assert_eq!(
            lines(&errors),
            [
                "factory #1: run must be a finite number from 0 to 1 (got NaN)",
                "factory #2: not in this business's view",
                "factory #2: run must be a finite number from 0 to 1 (got 1.5)",
                "factory #2: maintenance must be a finite number from 0 to 1 (got -0.1)",
                "offer 'copper': no price is posted for it on this hex",
                "offer 'grain': amount must be a finite number of at least 0 (got -2)",
                "offer 'silver': it is this hex's currency, which can't be traded against itself",
                "bid 'labor': amount must be a finite number of at least 0 (got inf)",
                "bid 'wheat': unknown good",
            ]
        );
        assert_eq!(errors.len(), 9);
    }

    #[test]
    fn a_decision_for_another_business_is_refused() {
        let errors = Decision::empty("b")
            .validate(&test_view(), &test_goods())
            .unwrap_err();
        assert_eq!(
            lines(&errors),
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
    fn an_empty_decision_writes_empty_tables_and_no_trace() {
        assert_eq!(
            Decision::empty("a").to_json(),
            r#"{"business":"a","factories":{},"offers":{},"bids":{}}"#
        );
    }

    #[test]
    fn a_hand_written_decision_reads_as_expected() {
        let text = r#"{"business": "a", "factories": {"0": {"run": 1}}, "offers": {"grain": 8}, "bids": {"labor": 6}, "trace": "sell the grain"}"#;
        let d = Decision::from_json(text).unwrap();
        assert_eq!(d.factories, BTreeMap::from([(0, order(1.0, 0.0))]));
        assert_eq!(d.offers, table(&[("grain", 8.0)]));
        assert_eq!(d.bids, table(&[("labor", 6.0)]));
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
        for (text, expected) in [
            (r#"{"business": 5}"#, "invalid type"),
            (r#"{"business":"a","offer":{}}"#, "unknown field"),
            (
                r#"{"business":"a","offers":{"grain":1,"grain":2}}"#,
                "the key 'grain' is written twice",
            ),
            (
                r#"{"business":"a","factories":{"0":{"run":"all"}}}"#,
                "invalid type",
            ),
        ] {
            let error = Decision::from_json(text).unwrap_err();
            assert!(matches!(error, FormatError::Json(_)), "{text}");
            assert!(error.to_string().contains(expected), "{text}: {error}");
        }
        assert_eq!(
            Decision::from_json(r#"{"business":"a","offers":{"grain":-1}}"#),
            Err(FormatError::Values(vec![
                "offer 'grain': amount must be a finite number of at least 0 (got -1)".to_string()
            ]))
        );
        assert_eq!(
            Decision::from_json(r#"{"business":"a","factories":{"0":{"run":2}}}"#),
            Err(FormatError::Values(vec![
                "factory #0: run must be a finite number from 0 to 1 (got 2)".to_string()
            ]))
        );
        for text in [
            r#"{"business":"a","offers":{"grain":1e999}}"#,
            r#"{"business":"a","offers":{"grain":NaN}}"#,
        ] {
            let error = Decision::from_json(text).unwrap_err();
            assert!(!error.to_string().is_empty(), "{text}");
        }
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
