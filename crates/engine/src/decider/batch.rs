//! The batch: one turn's views served together, one turn's decisions collected together, and
//! the recording that keeps both.
//!
//! A [`Request`] holds a view per business served and a [`Reply`] holds decisions for any
//! subset of them, in any order. [`Reply::validate`] checks a reply against its request and
//! gives one [`Answer`] per business in the request, so a later Step can resolve a turn from
//! it. A [`Recording`] is a request and its reply saved together.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::decider::decision::{Decision, DecisionErrors};
use crate::decider::view::BusinessView;
use crate::decider::{self, FormatError};
use crate::error_list::ErrorList;
use crate::goods::Goods;
use crate::turn::Turn;

/// One turn's views, served together: the engine's message to the deciders.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// The turn every view is for.
    pub turn: Turn,
    /// One view per business served, in business ID order when the engine builds it.
    pub views: Vec<BusinessView>,
}

/// One turn's decisions, collected together: the deciders' message back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    /// The turn the decisions answer. Must match the request's.
    pub turn: Turn,
    /// Decisions in any order, for any subset of the request's businesses. Absent in JSON
    /// means empty.
    #[serde(default)]
    pub decisions: Vec<Decision>,
}

/// A request and its reply, saved together: the recording the Python tests read and the
/// replay decider answers from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recording {
    /// The views served.
    pub request: Request,
    /// The decisions collected.
    pub reply: Reply,
}

/// Why a reply was refused as a whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplyError {
    /// The reply's turn isn't the request's.
    TurnMismatch {
        /// The request's turn.
        request: Turn,
        /// The reply's turn.
        reply: Turn,
    },
    /// A decision for a business that isn't in the request.
    UnknownBusiness {
        /// The business named.
        name: String,
    },
    /// A second decision for one business.
    DuplicateBusiness {
        /// The business named twice.
        name: String,
    },
}

impl fmt::Display for ReplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplyError::TurnMismatch { request, reply } => write!(
                f,
                "the reply answers turn {reply}, but the request is for turn {request}"
            ),
            ReplyError::UnknownBusiness { name } => {
                write!(f, "a decision for '{name}', which isn't in the request")
            }
            ReplyError::DuplicateBusiness { name } => write!(f, "two decisions for '{name}'"),
        }
    }
}

impl Error for ReplyError {}

/// Every reply-level problem, in order: the turn first, then the decisions in reply order.
/// Never empty.
pub type ReplyErrors = ErrorList<ReplyError>;

/// What one business in the request got from the reply.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer<'a> {
    /// No decision: the business does nothing this turn.
    Nothing,
    /// A decision that passed validation against the business's view.
    Valid(&'a Decision),
    /// A decision that failed validation; the host idles its business for the turn.
    Refused {
        /// The decision that failed, as the reply gave it.
        decision: &'a Decision,
        /// Every problem found in it.
        errors: DecisionErrors,
    },
}

impl Reply {
    /// Checks the reply against the request it answers. Refuses the whole reply, with
    /// every reply-level problem, when its turn isn't the request's, it names a business
    /// not in the request, or it names a business twice. Otherwise returns one `Answer` per
    /// view, in the request's order, each decision validated against its view.
    pub fn validate<'a>(
        &'a self,
        request: &Request,
        goods: &Goods,
    ) -> Result<Vec<Answer<'a>>, ReplyErrors> {
        let mut problems = Vec::new();
        if self.turn != request.turn {
            problems.push(ReplyError::TurnMismatch {
                request: request.turn,
                reply: self.turn,
            });
        }
        let requested: BTreeSet<&str> = request
            .views
            .iter()
            .map(|view| view.business.as_str())
            .collect();
        let mut by_business: BTreeMap<&str, &Decision> = BTreeMap::new();
        for decision in &self.decisions {
            let name = decision.business.as_str();
            if !requested.contains(name) {
                problems.push(ReplyError::UnknownBusiness {
                    name: name.to_string(),
                });
            } else if by_business.insert(name, decision).is_some() {
                problems.push(ReplyError::DuplicateBusiness {
                    name: name.to_string(),
                });
            }
        }
        if !problems.is_empty() {
            return Err(ErrorList::new(problems));
        }
        Ok(request
            .views
            .iter()
            .map(|view| match by_business.get(view.business.as_str()) {
                None => Answer::Nothing,
                Some(&decision) => match decision.validate(view, goods) {
                    Ok(()) => Answer::Valid(decision),
                    Err(errors) => Answer::Refused { decision, errors },
                },
            })
            .collect())
    }

    /// Compact JSON, one line.
    pub fn to_json(&self) -> String {
        decider::to_json(self)
    }

    /// Indented JSON.
    pub fn to_json_pretty(&self) -> String {
        decider::to_json_pretty(self)
    }

    /// Reads a reply; value problems are each decision's, prefixed `"{business}: "`.
    pub fn from_json(text: &str) -> Result<Reply, FormatError> {
        let reply: Reply = decider::from_json(text)?;
        decider::checked(reply.value_problems())?;
        Ok(reply)
    }

    /// The decisions' number problems, each prefixed with its business.
    fn value_problems(&self) -> Vec<String> {
        self.decisions
            .iter()
            .flat_map(|decision| {
                decision
                    .number_problems()
                    .into_iter()
                    .map(|problem| format!("{}: {problem}", decision.business))
            })
            .collect()
    }
}

impl Request {
    /// Compact JSON, one line.
    pub fn to_json(&self) -> String {
        decider::to_json(self)
    }

    /// Indented JSON.
    pub fn to_json_pretty(&self) -> String {
        decider::to_json_pretty(self)
    }

    /// Reads a request; value problems are each view's, prefixed `"{business}: "`.
    pub fn from_json(text: &str) -> Result<Request, FormatError> {
        let request: Request = decider::from_json(text)?;
        decider::checked(request.value_problems())?;
        Ok(request)
    }

    /// The views' value problems, each prefixed with its business.
    fn value_problems(&self) -> Vec<String> {
        self.views
            .iter()
            .flat_map(|view| {
                view.problems()
                    .into_iter()
                    .map(|problem| format!("{}: {problem}", view.business))
            })
            .collect()
    }
}

impl Recording {
    /// Compact JSON, one line.
    pub fn to_json(&self) -> String {
        decider::to_json(self)
    }

    /// Indented JSON.
    pub fn to_json_pretty(&self) -> String {
        decider::to_json_pretty(self)
    }

    /// Reads a recording; value problems are the request's then the reply's.
    pub fn from_json(text: &str) -> Result<Recording, FormatError> {
        let recording: Recording = decider::from_json(text)?;
        let mut problems = recording.request.value_problems();
        problems.extend(recording.reply.value_problems());
        decider::checked(problems)?;
        Ok(recording)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decider::decision::{FactoryOrder, test_goods};
    use crate::decider::view::test_view;

    fn view_of(name: &str) -> BusinessView {
        let mut view = test_view();
        view.business = name.to_string();
        view
    }

    fn request() -> Request {
        Request {
            turn: Turn::new(4),
            views: vec![view_of("a"), view_of("b"), view_of("c")],
        }
    }

    fn reply(turn: u64, names: &[&str]) -> Reply {
        Reply {
            turn: Turn::new(turn),
            decisions: names.iter().map(|n| Decision::empty(*n)).collect(),
        }
    }

    fn reply_errors(reply: &Reply) -> Vec<ReplyError> {
        reply
            .validate(&request(), &test_goods())
            .unwrap_err()
            .iter()
            .cloned()
            .collect()
    }

    #[test]
    fn a_subset_in_any_order_is_accepted() {
        let r = reply(4, &["c", "a"]);
        let q = request();
        let g = test_goods();
        let answers = r.validate(&q, &g).unwrap();
        assert_eq!(
            answers,
            [
                Answer::Valid(&r.decisions[1]),
                Answer::Nothing,
                Answer::Valid(&r.decisions[0]),
            ]
        );
    }

    #[test]
    fn an_empty_reply_means_everyone_does_nothing() {
        let r = reply(4, &[]);
        let answers = r.validate(&request(), &test_goods()).unwrap();
        assert_eq!(answers, [Answer::Nothing, Answer::Nothing, Answer::Nothing]);
    }

    #[test]
    fn a_wrong_turn_is_refused() {
        let errors = reply_errors(&reply(5, &["a"]));
        assert_eq!(
            errors,
            [ReplyError::TurnMismatch {
                request: Turn::new(4),
                reply: Turn::new(5)
            }]
        );
        assert_eq!(
            errors[0].to_string(),
            "the reply answers turn 5, but the request is for turn 4"
        );
    }

    #[test]
    fn an_unknown_business_is_refused() {
        let errors = reply_errors(&reply(4, &["z"]));
        assert_eq!(
            errors,
            [ReplyError::UnknownBusiness {
                name: "z".to_string()
            }]
        );
        assert_eq!(
            errors[0].to_string(),
            "a decision for 'z', which isn't in the request"
        );
    }

    #[test]
    fn a_duplicate_business_is_refused() {
        let errors = reply_errors(&reply(4, &["a", "a"]));
        assert_eq!(
            errors,
            [ReplyError::DuplicateBusiness {
                name: "a".to_string()
            }]
        );
        assert_eq!(errors[0].to_string(), "two decisions for 'a'");
    }

    #[test]
    fn every_reply_problem_is_reported() {
        let r = reply(5, &["a", "z", "a"]);
        let q = request();
        let g = test_goods();
        let errors = r.validate(&q, &g).unwrap_err();
        assert_eq!(
            errors.iter().cloned().collect::<Vec<_>>(),
            [
                ReplyError::TurnMismatch {
                    request: Turn::new(4),
                    reply: Turn::new(5)
                },
                ReplyError::UnknownBusiness {
                    name: "z".to_string()
                },
                ReplyError::DuplicateBusiness {
                    name: "a".to_string()
                },
            ]
        );
        assert_eq!(errors.len(), 3);
        assert_eq!(
            errors.to_string(),
            "the reply answers turn 5, but the request is for turn 4\n\
             a decision for 'z', which isn't in the request\n\
             two decisions for 'a'"
        );
    }

    #[test]
    fn an_invalid_decision_is_refused_without_refusing_the_reply() {
        let mut bad = Decision::empty("a");
        bad.factories.insert(
            9,
            FactoryOrder {
                run: 0.0,
                maintenance: 0.0,
            },
        );
        let r = Reply {
            turn: Turn::new(4),
            decisions: vec![bad, Decision::empty("c")],
        };
        let q = request();
        let g = test_goods();
        let answers = r.validate(&q, &g).unwrap();
        assert_eq!(answers.len(), 3);
        match &answers[0] {
            Answer::Refused { decision, errors } => {
                assert_eq!(decision.business, "a");
                assert_eq!(
                    errors.iter().map(ToString::to_string).collect::<Vec<_>>(),
                    ["factory #9: not in this business's view"]
                );
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(answers[1], Answer::Nothing);
        assert_eq!(answers[2], Answer::Valid(&r.decisions[1]));
    }

    #[test]
    fn request_reply_and_recording_round_trip() {
        let mut q = request();
        q.turn = Turn::MAX;
        let mut r = reply(4, &["a", "c"]);
        r.turn = Turn::MAX;
        r.decisions[0].offers.insert("grain".to_string(), 1.5);
        r.decisions[0].trace = Some("two\nlines".to_string());
        let recording = Recording {
            request: q.clone(),
            reply: r.clone(),
        };
        assert_eq!(Request::from_json(&q.to_json()).unwrap(), q);
        assert_eq!(Request::from_json(&q.to_json_pretty()).unwrap(), q);
        assert_eq!(Reply::from_json(&r.to_json()).unwrap(), r);
        assert_eq!(Reply::from_json(&r.to_json_pretty()).unwrap(), r);
        assert_eq!(
            Recording::from_json(&recording.to_json()).unwrap(),
            recording
        );
        assert_eq!(
            Recording::from_json(&recording.to_json_pretty()).unwrap(),
            recording
        );
    }

    #[test]
    fn reading_a_reply_with_no_decisions_and_prefixing_problems() {
        assert_eq!(
            Reply::from_json(r#"{"turn": 4}"#).unwrap(),
            Reply {
                turn: Turn::new(4),
                decisions: Vec::new()
            }
        );
        assert_eq!(
            Reply::from_json(r#"{"turn":4,"decisions":[{"business":"a","offers":{"grain":-1}}]}"#),
            Err(FormatError::Values(vec![
                "a: offer 'grain': amount must be a finite number of at least 0 (got -1)"
                    .to_string()
            ]))
        );
    }

    #[test]
    fn value_problems_in_a_request_and_a_recording_are_prefixed() {
        let mut q = request();
        q.views[1].factories.get_mut(&0).unwrap().condition = 2.0;
        let expected = "b: factory #0: condition must be a finite number from 0 to 1 (got 2)";
        assert_eq!(
            Request::from_json(&q.to_json()),
            Err(FormatError::Values(vec![expected.to_string()]))
        );
        let mut r = reply(4, &["a"]);
        r.decisions[0].bids.insert("labor".to_string(), -3.0);
        let recording = Recording {
            request: q,
            reply: r,
        };
        assert_eq!(
            Recording::from_json(&recording.to_json()),
            Err(FormatError::Values(vec![
                expected.to_string(),
                "a: bid 'labor': amount must be a finite number of at least 0 (got -3)".to_string()
            ]))
        );
    }
}
