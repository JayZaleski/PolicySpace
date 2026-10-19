//! The decision point: once per turn, every business's view goes to the decider its setting
//! entry names and every decision comes back, one batched exchange per decider. The host
//! is a table of deciders by name, built by the caller (the dev UI, the checker, a test),
//! never by the World; the World takes it at creation and at every step and reads from it
//! only during the decision point, so it holds no decider between turns. Every decider
//! receives exactly one request per turn (empty when it controls no business), deciders
//! are taken in name order, and every request is submitted before any reply is collected.
//! A refused decision idles its business; a failed decider idles every business it
//! controls and is reported; the turn never halts.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::time::Duration;

use crate::businesses::{BusinessId, Businesses};
use crate::decider::external::StartError;
use crate::decider::{Answer, Decision, DecisionErrors, FormatError, Reply, ReplyErrors, Request};
use crate::turn::Turn;
use crate::world::World;

/// Answers one turn's request with one reply. Implementations: the external process, the
/// replay decider and the closure decider.
pub trait Decider {
    /// Takes the turn's request. An external process writes it; the others prepare the
    /// reply. A failure here means the decider can't answer this turn.
    fn submit(&mut self, request: &Request) -> Result<(), DeciderFailure>;

    /// Returns the reply to the request last submitted. Called exactly once after every
    /// successful `submit`, never otherwise.
    fn collect(&mut self) -> Result<Reply, DeciderFailure>;

    /// Told that the reply just collected was refused as a whole (wrong turn, unknown or
    /// duplicate business). The default does nothing; the external process stops itself so
    /// that a process one reply behind is started afresh next turn.
    fn reject(&mut self) {}
}

/// Why a decider couldn't answer a turn. Only the process variants carry a stderr tail.
#[derive(Debug, Clone, PartialEq)]
pub enum DeciderFailure {
    /// The process exited, or closed its pipes, before answering.
    Exited {
        /// How it exited.
        status: std::process::ExitStatus,
        /// The tail of its error output.
        stderr: Vec<String>,
    },
    /// No reply within the time limit; the process was killed.
    TimedOut {
        /// The limit that passed.
        limit: Duration,
        /// The tail of its error output.
        stderr: Vec<String>,
    },
    /// The reply line couldn't be read as a reply; the process was killed.
    Unreadable {
        /// Why the line isn't a reply.
        error: FormatError,
        /// The tail of its error output.
        stderr: Vec<String>,
    },
    /// The process had failed earlier and couldn't be started again.
    WontStart {
        /// Why it couldn't start.
        error: StartError,
    },
    /// The reply was refused as a whole (wrong turn, unknown or duplicate business).
    Refused(ReplyErrors),
    /// The replay decider has no recording for this turn.
    NoRecording {
        /// The turn asked for.
        turn: Turn,
    },
    /// The replay decider's recording for this turn holds a different request.
    RequestDiffers {
        /// The turn asked for.
        turn: Turn,
    },
}

impl DeciderFailure {
    /// The stderr tail, empty for the variants that have none. For `WontStart` it is the
    /// `StartError::ExitedAtOnce` tail (empty for `Spawn`).
    pub fn stderr_tail(&self) -> &[String] {
        match self {
            DeciderFailure::Exited { stderr, .. }
            | DeciderFailure::TimedOut { stderr, .. }
            | DeciderFailure::Unreadable { stderr, .. } => stderr,
            DeciderFailure::WontStart { error } => error.stderr_tail(),
            _ => &[],
        }
    }
}

impl fmt::Display for DeciderFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeciderFailure::Exited { status, .. } => {
                write!(f, "the process exited ({status}) before answering")
            }
            DeciderFailure::TimedOut { limit, .. } => write!(
                f,
                "no reply within the time limit of {:.1} s",
                limit.as_secs_f64()
            ),
            DeciderFailure::Unreadable { error, .. } => {
                let text = match error {
                    FormatError::Values(lines) => lines.join("; "),
                    FormatError::Json(message) => message.clone(),
                };
                write!(f, "the reply couldn't be read: {text}")
            }
            DeciderFailure::WontStart { error } => {
                write!(
                    f,
                    "the process couldn't be started again: {}",
                    error.summary()
                )
            }
            DeciderFailure::Refused(errors) => {
                let joined: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
                write!(f, "the reply was refused: {}", joined.join("; "))
            }
            DeciderFailure::NoRecording { turn } => write!(f, "no recording for turn {turn}"),
            DeciderFailure::RequestDiffers { turn } => write!(
                f,
                "the request for turn {turn} differs from the recorded one"
            ),
        }
    }
}

impl Error for DeciderFailure {}

/// One decider's failure in one turn, as the turn report carries it.
#[derive(Debug, Clone, PartialEq)]
pub struct ReportedFailure {
    /// The name of the decider that failed.
    pub decider: String,
    /// What went wrong.
    pub failure: DeciderFailure,
}

impl fmt::Display for ReportedFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "decider '{}': {}", self.decider, self.failure)?;
        let tail = self.failure.stderr_tail();
        if !tail.is_empty() {
            write!(f, "\nits error output ends with:")?;
            for line in tail {
                write!(f, "\n  {line}")?;
            }
        }
        Ok(())
    }
}

impl Error for ReportedFailure {}

/// How many businesses sent a valid decision, were refused, and sent nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DecisionCounts {
    /// Businesses whose decision passed validation.
    pub valid: usize,
    /// Businesses whose decision failed validation.
    pub refused: usize,
    /// Businesses that sent nothing, or whose decider failed.
    pub nothing: usize,
}

/// What one business got at the decision point. Kept in the World until the next one.
#[derive(Debug, Clone, PartialEq)]
pub enum BusinessAnswer {
    /// No decision (none sent, or its decider failed): the business does nothing.
    Nothing,
    /// A decision that passed validation against the business's view.
    Valid(Decision),
    /// A decision that failed validation, kept with its errors; the business does nothing.
    Refused {
        /// The decision as sent.
        decision: Decision,
        /// Every problem found in it.
        errors: DecisionErrors,
    },
}

/// A business names a decider the host doesn't hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownDecider {
    /// The business that names it.
    pub business: String,
    /// The decider name it uses.
    pub decider: String,
    /// The names the host holds, in name order.
    pub known: Vec<String>,
}

impl fmt::Display for UnknownDecider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "business '{}' names decider '{}', which the host doesn't hold ",
            self.business, self.decider
        )?;
        if self.known.is_empty() {
            write!(f, "(it holds no deciders)")
        } else {
            write!(f, "(it holds: {})", self.known.join(", "))
        }
    }
}

impl Error for UnknownDecider {}

/// The deciders by name. `idle()` holds every name and answers nothing for names it has no
/// decider for; `new()` holds only what is added.
pub struct DeciderHost {
    deciders: BTreeMap<String, Box<dyn Decider>>,
    accepts_unknown: bool,
}

impl DeciderHost {
    /// An empty host that accepts only the names added to it.
    pub fn new() -> DeciderHost {
        DeciderHost {
            deciders: BTreeMap::new(),
            accepts_unknown: false,
        }
    }

    /// The idle host: every name is accepted, and a name with no decider gets nothing.
    pub fn idle() -> DeciderHost {
        DeciderHost {
            deciders: BTreeMap::new(),
            accepts_unknown: true,
        }
    }

    /// Adds a decider under `name`.
    ///
    /// # Panics
    /// Panics if the host already holds `name` (`a decider named '{name}' is already in the
    /// host`).
    pub fn add(
        &mut self,
        name: impl Into<String>,
        decider: impl Decider + 'static,
    ) -> &mut DeciderHost {
        let name = name.into();
        assert!(
            !self.deciders.contains_key(&name),
            "a decider named '{name}' is already in the host"
        );
        self.deciders.insert(name, Box::new(decider));
        self
    }

    /// `add` in builder form.
    pub fn with(mut self, name: impl Into<String>, decider: impl Decider + 'static) -> DeciderHost {
        self.add(name, decider);
        self
    }

    /// True for the idle host.
    pub fn accepts_unknown(&self) -> bool {
        self.accepts_unknown
    }

    /// True if `name` may be used by a business: it was added, or the host accepts every
    /// name.
    pub fn holds(&self, name: &str) -> bool {
        self.accepts_unknown || self.deciders.contains_key(name)
    }

    /// The names added, in name order.
    pub fn names(&self) -> impl Iterator<Item = &str> + '_ {
        self.deciders.keys().map(String::as_str)
    }

    /// The first business (in ID order) whose decider the host doesn't hold, if any.
    pub(crate) fn check(&self, businesses: &Businesses) -> Result<(), UnknownDecider> {
        for (_, name, business) in businesses.iter() {
            if !self.holds(&business.decider) {
                return Err(UnknownDecider {
                    business: name.to_string(),
                    decider: business.decider.clone(),
                    known: self.names().map(str::to_string).collect(),
                });
            }
        }
        Ok(())
    }

    /// The decision point for `world`'s current turn: submits every decider's request,
    /// then collects every reply and validates it.
    pub(crate) fn decision_point(&mut self, world: &World) -> DecisionPoint {
        struct Exchange {
            name: String,
            ids: Vec<BusinessId>,
            request: Request,
            submitted: Result<(), DeciderFailure>,
        }

        let businesses = world.businesses();
        let goods = &world.rules().goods;
        let mut answers = vec![BusinessAnswer::Nothing; businesses.len()];
        let mut failures = Vec::new();

        let mut exchanges = Vec::with_capacity(self.deciders.len());
        for (name, decider) in self.deciders.iter_mut() {
            let ids: Vec<BusinessId> = businesses
                .iter()
                .filter(|(_, _, business)| business.decider == *name)
                .map(|(id, _, _)| id)
                .collect();
            let request = Request {
                turn: world.turn(),
                views: ids.iter().map(|id| world.business_view(*id)).collect(),
            };
            let submitted = decider.submit(&request);
            exchanges.push(Exchange {
                name: name.clone(),
                ids,
                request,
                submitted,
            });
        }

        for exchange in exchanges {
            let Exchange {
                name,
                ids,
                request,
                submitted,
            } = exchange;
            let collected = match submitted {
                Ok(()) => self
                    .deciders
                    .get_mut(&name)
                    .expect("the exchange names a decider the host holds")
                    .collect(),
                Err(failure) => Err(failure),
            };
            let reply = match collected {
                Ok(reply) => reply,
                Err(failure) => {
                    failures.push(ReportedFailure {
                        decider: name,
                        failure,
                    });
                    continue;
                }
            };
            match reply.validate(&request, goods) {
                Err(errors) => {
                    if let Some(decider) = self.deciders.get_mut(&name) {
                        decider.reject();
                    }
                    failures.push(ReportedFailure {
                        decider: name,
                        failure: DeciderFailure::Refused(errors),
                    });
                }
                Ok(validated) => {
                    for ((id, view), answer) in ids.iter().zip(&request.views).zip(validated) {
                        answers[id.index()] = match answer {
                            Answer::Nothing => BusinessAnswer::Nothing,
                            Answer::Valid(decision) => BusinessAnswer::Valid(decision.clone()),
                            Answer::Refused(errors) => BusinessAnswer::Refused {
                                decision: reply
                                    .decisions
                                    .iter()
                                    .find(|decision| decision.business == view.business)
                                    .expect("a refused answer has a decision")
                                    .clone(),
                                errors,
                            },
                        };
                    }
                }
            }
        }
        DecisionPoint { answers, failures }
    }
}

impl Default for DeciderHost {
    fn default() -> DeciderHost {
        DeciderHost::new()
    }
}

impl fmt::Debug for DeciderHost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeciderHost")
            .field("names", &self.names().collect::<Vec<_>>())
            .field("accepts_unknown", &self.accepts_unknown)
            .finish()
    }
}

/// What the decision point produced: one answer per business by index, and the failures.
pub(crate) struct DecisionPoint {
    /// One answer per business, by business index.
    pub(crate) answers: Vec<BusinessAnswer>,
    /// Every decider that failed, in decider name order.
    pub(crate) failures: Vec<ReportedFailure>,
}

/// A decider built from a Rust function, for tests.
pub struct ClosureDecider {
    f: Box<dyn FnMut(&Request) -> Reply>,
    pending: Option<Reply>,
}

impl ClosureDecider {
    /// A decider that answers each request with `f(request)`.
    pub fn new(f: impl FnMut(&Request) -> Reply + 'static) -> ClosureDecider {
        ClosureDecider {
            f: Box::new(f),
            pending: None,
        }
    }
}

impl Decider for ClosureDecider {
    fn submit(&mut self, request: &Request) -> Result<(), DeciderFailure> {
        self.pending = Some((self.f)(request));
        Ok(())
    }

    fn collect(&mut self) -> Result<Reply, DeciderFailure> {
        Ok(self.pending.take().expect("collect follows submit"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Bundle;
    use crate::businesses::{Business, BusinessKind};
    use crate::decider::decision::test_goods;
    use crate::decider::view::test_view;
    use crate::id::NameRegistry;
    use crate::map::{HexId, HexKind};

    fn empty_reply(request: &Request) -> Reply {
        Reply {
            turn: request.turn,
            decisions: vec![],
        }
    }

    fn closure() -> ClosureDecider {
        ClosureDecider::new(empty_reply)
    }

    /// Three businesses: `a` and `c` name `rule`, `b` names `other`.
    fn businesses() -> Businesses {
        let mut hexes: NameRegistry<HexKind> = NameRegistry::new();
        let hex: HexId = hexes.register("h0").unwrap();
        let mut names: NameRegistry<BusinessKind> = NameRegistry::new();
        for name in ["a", "b", "c"] {
            names.register(name).unwrap();
        }
        let business = |decider: &str| Business {
            hex,
            decider: decider.to_string(),
            starting_inventory: Bundle::new(Vec::new()),
        };
        Businesses::from_parts(
            names,
            vec![business("rule"), business("other"), business("rule")],
            1,
        )
    }

    #[test]
    fn the_idle_host_holds_every_name_and_the_empty_host_none() {
        assert!(DeciderHost::idle().holds("x"));
        assert!(DeciderHost::idle().accepts_unknown());
        assert!(!DeciderHost::new().holds("x"));
        assert!(!DeciderHost::new().accepts_unknown());
        let mut host = DeciderHost::new();
        host.add("y", closure()).add("x", closure());
        assert!(host.holds("x") && host.holds("y"));
        assert!(!host.holds("z"));
        assert_eq!(host.names().collect::<Vec<_>>(), ["x", "y"]);
        let mut idle = DeciderHost::idle();
        idle.add("x", closure());
        assert!(idle.holds("x") && idle.holds("anything"));
        assert_eq!(idle.names().collect::<Vec<_>>(), ["x"]);
        assert!(format!("{host:?}").contains("accepts_unknown: false"));
    }

    #[test]
    #[should_panic(expected = "already in the host")]
    fn adding_a_name_twice_panics() {
        let _ = DeciderHost::new().with("x", closure()).with("x", closure());
    }

    #[test]
    fn check_names_the_first_business_with_an_unknown_decider() {
        let businesses = businesses();
        let host = DeciderHost::new().with("rule", closure());
        let error = host.check(&businesses).unwrap_err();
        assert_eq!(
            error,
            UnknownDecider {
                business: "b".to_string(),
                decider: "other".to_string(),
                known: vec!["rule".to_string()],
            }
        );
        assert_eq!(
            error.to_string(),
            "business 'b' names decider 'other', which the host doesn't hold (it holds: rule)"
        );
        assert_eq!(DeciderHost::idle().check(&businesses), Ok(()));
        let empty = DeciderHost::new().check(&businesses).unwrap_err();
        assert_eq!(empty.business, "a");
        assert!(empty.to_string().ends_with("(it holds no deciders)"));
        let both = DeciderHost::new()
            .with("rule", closure())
            .with("other", closure());
        assert_eq!(both.check(&businesses), Ok(()));
        let two = UnknownDecider {
            business: "b".to_string(),
            decider: "z".to_string(),
            known: vec!["a".to_string(), "b".to_string()],
        };
        assert!(two.to_string().ends_with("(it holds: a, b)"));
    }

    #[test]
    fn failure_and_report_displays() {
        let turn = Turn::new(7);
        let request = Request {
            turn: Turn::new(0),
            views: vec![test_view()],
        };
        let wrong = Reply {
            turn: Turn::new(1),
            decisions: vec![Decision::empty("x")],
        };
        let refused = DeciderFailure::Refused(wrong.validate(&request, &test_goods()).unwrap_err());
        assert_eq!(
            refused.to_string(),
            "the reply was refused: the reply answers turn 1, but the request is for turn 0; \
             a decision for 'x', which isn't in the request"
        );
        let cases = [
            (
                DeciderFailure::TimedOut {
                    limit: Duration::from_millis(2500),
                    stderr: vec![],
                },
                "no reply within the time limit of 2.5 s",
            ),
            (
                DeciderFailure::Unreadable {
                    error: FormatError::Json("bad".to_string()),
                    stderr: vec![],
                },
                "the reply couldn't be read: bad",
            ),
            (
                DeciderFailure::Unreadable {
                    error: FormatError::Values(vec!["a".to_string(), "b".to_string()]),
                    stderr: vec![],
                },
                "the reply couldn't be read: a; b",
            ),
            (
                DeciderFailure::WontStart {
                    error: StartError::Spawn {
                        command: "py".to_string(),
                        error: "gone".to_string(),
                    },
                },
                "the process couldn't be started again: 'py' couldn't be started: gone",
            ),
            (
                DeciderFailure::NoRecording { turn },
                "no recording for turn 7",
            ),
            (
                DeciderFailure::RequestDiffers { turn },
                "the request for turn 7 differs from the recorded one",
            ),
        ];
        for (failure, text) in cases {
            assert_eq!(failure.to_string(), text);
            assert!(failure.stderr_tail().is_empty());
        }
        let at_once = DeciderFailure::WontStart {
            error: StartError::ExitedAtOnce {
                command: "py".to_string(),
                status: std::process::ExitStatus::default(),
                stderr: vec!["one".to_string(), "two".to_string()],
            },
        };
        let text = at_once.to_string();
        assert!(!text.contains('\n'), "{text}");
        assert!(text.starts_with("the process couldn't be started again: 'py' exited ("));
        assert!(text.ends_with(") as soon as it started"));
        assert_eq!(at_once.stderr_tail(), ["one", "two"]);
        let reported = ReportedFailure {
            decider: "rule".to_string(),
            failure: DeciderFailure::TimedOut {
                limit: Duration::from_secs(5),
                stderr: vec!["one".to_string(), "two".to_string()],
            },
        };
        assert_eq!(
            reported.to_string(),
            "decider 'rule': no reply within the time limit of 5.0 s\n\
             its error output ends with:\n  one\n  two"
        );
        assert_eq!(
            ReportedFailure {
                decider: "rule".to_string(),
                failure: DeciderFailure::NoRecording { turn },
            }
            .to_string(),
            "decider 'rule': no recording for turn 7"
        );
        assert_eq!(
            DecisionCounts::default(),
            DecisionCounts {
                valid: 0,
                refused: 0,
                nothing: 0
            }
        );
    }

    #[test]
    fn a_closure_decider_answers_its_request() {
        let mut decider = ClosureDecider::new(|r| Reply {
            turn: r.turn,
            decisions: vec![],
        });
        let request = Request {
            turn: Turn::new(4),
            views: vec![],
        };
        decider.submit(&request).unwrap();
        assert_eq!(
            decider.collect().unwrap(),
            Reply {
                turn: Turn::new(4),
                decisions: vec![]
            }
        );
    }
}
