//! Integration tests for the decider host and the decision point, through the engine's
//! public API only. No Python: the deciders are closures and replayed recordings.

mod common;

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

use common::{load, sample_setting, test_setting};
use policyspace_engine::decider::{
    ClosureDecider, DecisionCounts, FactoryOrder, ReplayDecider, ReplyError,
};
use policyspace_engine::{
    BusinessAnswer, DeciderFailure, DeciderHost, Decision, Recording, Reply, ReportedFailure,
    Request, StepError, Turn, UnknownDecider, World,
};

fn empty_reply(request: &Request) -> Reply {
    Reply {
        turn: request.turn,
        decisions: Vec::new(),
    }
}

fn closure(f: impl FnMut(&Request) -> Reply + 'static) -> ClosureDecider {
    ClosureDecider::new(f)
}

/// A decision for `name` that runs the given factories at (run, maintenance).
fn decision_for(name: &str, factories: &[(u32, (f64, f64))]) -> Decision {
    let mut decision = Decision::empty(name);
    for (id, (run, maintenance)) in factories {
        decision.factories.insert(
            *id,
            FactoryOrder {
                run: *run,
                maintenance: *maintenance,
            },
        );
    }
    decision
}

/// Every business's answer, in business ID order.
fn answers_of(world: &World) -> Vec<BusinessAnswer> {
    world
        .businesses()
        .iter()
        .map(|(id, _, _)| world.business_answer(id).clone())
        .collect()
}

/// A host for `businesses_valid` whose two deciders answer with the given functions.
fn host_of(
    rule: impl FnMut(&Request) -> Reply + 'static,
    other_rule: impl FnMut(&Request) -> Reply + 'static,
) -> DeciderHost {
    DeciderHost::new()
        .with("rule", closure(rule))
        .with("other_rule", closure(other_rule))
}

fn names(request: &Request) -> Vec<String> {
    request.views.iter().map(|v| v.business.clone()).collect()
}

fn valid_empty(name: &str) -> BusinessAnswer {
    BusinessAnswer::Valid(Decision::empty(name))
}

fn recording_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recordings/sample_turn0.recording.json")
}

fn sample_recording() -> Recording {
    let text = fs::read_to_string(recording_path()).expect("the sample recording is committed");
    Recording::from_json(&text).expect("the sample recording reads")
}

#[test]
fn an_unknown_decider_name_fails_world_creation() {
    let setting = load(&test_setting("businesses_valid"));
    let mut host = DeciderHost::new().with("rule", closure(empty_reply));
    let error = World::new(setting, &mut host).expect_err("refused");
    assert_eq!(
        error,
        UnknownDecider {
            business: "idle".to_string(),
            decider: "other_rule".to_string(),
            known: vec!["rule".to_string()],
        }
    );

    let setting = load(&test_setting("businesses_valid"));
    let (world, report) = World::new(setting, &mut DeciderHost::idle()).unwrap();
    assert_eq!(answers_of(&world), vec![BusinessAnswer::Nothing; 3]);
    assert_eq!(
        report.decisions,
        DecisionCounts {
            nothing: 3,
            ..DecisionCounts::default()
        }
    );
    assert!(report.decider_failures.is_empty());
}

#[test]
fn a_rebuilt_host_missing_a_name_refuses_the_step() {
    let setting = load(&test_setting("businesses_valid"));
    let mut full = host_of(empty_reply, empty_reply);
    let (mut world, _) = World::new(setting, &mut full).unwrap();
    let before = answers_of(&world);

    let mut partial = DeciderHost::new().with("rule", closure(empty_reply));
    let error = world.step(&mut partial).unwrap_err();
    assert!(matches!(error, StepError::UnknownDecider(ref e) if e.decider == "other_rule"));
    assert!(error.to_string().contains("other_rule"));
    assert_eq!(world.turn(), Turn::ZERO);
    assert_eq!(answers_of(&world), before);

    world.step(&mut full).unwrap();
    assert_eq!(world.turn(), Turn::new(1));
}

#[test]
fn each_decider_receives_only_its_businesses_in_id_order_and_an_empty_request_otherwise() {
    type Seen = Rc<RefCell<Vec<Request>>>;
    fn recorder(seen: &Seen) -> impl FnMut(&Request) -> Reply + 'static {
        let seen = Rc::clone(seen);
        move |request| {
            seen.borrow_mut().push(request.clone());
            empty_reply(request)
        }
    }
    let (rule, other, spare): (Seen, Seen, Seen) = Default::default();
    let mut host = DeciderHost::new()
        .with("rule", closure(recorder(&rule)))
        .with("other_rule", closure(recorder(&other)))
        .with("spare", closure(recorder(&spare)));
    let (mut world, report) =
        World::new(load(&test_setting("businesses_valid")), &mut host).unwrap();

    assert_eq!(rule.borrow().len(), 1);
    assert_eq!(rule.borrow()[0].turn, Turn::ZERO);
    assert_eq!(names(&rule.borrow()[0]), ["farm_a", "farm_b"]);
    assert_eq!(names(&other.borrow()[0]), ["idle"]);
    assert_eq!(spare.borrow().len(), 1);
    assert!(spare.borrow()[0].views.is_empty());
    assert_eq!(
        report.decisions,
        DecisionCounts {
            nothing: 3,
            ..DecisionCounts::default()
        }
    );
    assert!(report.decider_failures.is_empty());

    world.step(&mut host).unwrap();
    for seen in [&rule, &other, &spare] {
        assert_eq!(seen.borrow().len(), 2);
        assert_eq!(seen.borrow()[1].turn, Turn::new(1));
    }
}

#[test]
fn a_refused_decision_idles_only_its_business() {
    let mut host = host_of(
        |r| Reply {
            turn: r.turn,
            decisions: vec![
                decision_for("farm_a", &[(9, (0.0, 0.0))]),
                Decision::empty("farm_b"),
            ],
        },
        |r| Reply {
            turn: r.turn,
            decisions: vec![Decision::empty("idle")],
        },
    );
    let (world, report) = World::new(load(&test_setting("businesses_valid")), &mut host).unwrap();
    let answers = answers_of(&world);
    match &answers[0] {
        BusinessAnswer::Refused { decision, errors } => {
            assert_eq!(*decision, decision_for("farm_a", &[(9, (0.0, 0.0))]));
            let lines: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            assert_eq!(lines, ["factory #9: not in this business's view"]);
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(answers[1], valid_empty("farm_b"));
    assert_eq!(answers[2], valid_empty("idle"));
    assert_eq!(
        report.decisions,
        DecisionCounts {
            valid: 2,
            refused: 1,
            nothing: 0
        }
    );
    assert!(report.decider_failures.is_empty());
}

/// Builds `businesses_valid` where `rule` answers with `rule_reply` and `other_rule`
/// answers properly; returns the world and turn 0's report.
fn refused_as_a_whole(
    rule_reply: impl Fn(&Request) -> Reply + 'static,
) -> (World, policyspace_engine::TurnReport) {
    let mut host = host_of(rule_reply, |r| Reply {
        turn: r.turn,
        decisions: vec![Decision::empty("idle")],
    });
    World::new(load(&test_setting("businesses_valid")), &mut host).unwrap()
}

#[test]
fn a_reply_refused_as_a_whole_idles_every_business_of_that_decider() {
    type Make = fn(&Request) -> Reply;
    let cases: [(Make, ReplyError); 3] = [
        (
            |r| Reply {
                turn: r.turn.next().unwrap(),
                decisions: vec![],
            },
            ReplyError::TurnMismatch {
                request: Turn::new(0),
                reply: Turn::new(1),
            },
        ),
        (
            |r| Reply {
                turn: r.turn,
                decisions: vec![Decision::empty("nobody")],
            },
            ReplyError::UnknownBusiness {
                name: "nobody".to_string(),
            },
        ),
        (
            |r| Reply {
                turn: r.turn,
                decisions: vec![Decision::empty("farm_a"), Decision::empty("farm_a")],
            },
            ReplyError::DuplicateBusiness {
                name: "farm_a".to_string(),
            },
        ),
    ];
    for (make, expected) in cases {
        let (world, report) = refused_as_a_whole(make);
        let answers = answers_of(&world);
        assert_eq!(answers[0], BusinessAnswer::Nothing, "{expected}");
        assert_eq!(answers[1], BusinessAnswer::Nothing, "{expected}");
        assert_eq!(answers[2], valid_empty("idle"), "{expected}");
        assert_eq!(report.decider_failures.len(), 1, "{expected}");
        let ReportedFailure { decider, failure } = &report.decider_failures[0];
        assert_eq!(decider, "rule");
        match failure {
            DeciderFailure::Refused(errors) => {
                assert_eq!(errors.iter().cloned().collect::<Vec<_>>(), [expected]);
            }
            other => panic!("{other:?}"),
        }
        assert!(
            report.decider_failures[0]
                .to_string()
                .starts_with("decider 'rule': the reply was refused: ")
        );
        assert_eq!(
            report.decisions,
            DecisionCounts {
                valid: 1,
                refused: 0,
                nothing: 2
            }
        );
    }
}

#[test]
fn a_failed_closure_decider_is_asked_again_next_turn() {
    let count = Rc::new(RefCell::new(0usize));
    let counter = Rc::clone(&count);
    let mut host = host_of(
        move |r| {
            *counter.borrow_mut() += 1;
            Reply {
                turn: r.turn.next().unwrap(),
                decisions: vec![],
            }
        },
        empty_reply,
    );
    let (mut world, first) =
        World::new(load(&test_setting("businesses_valid")), &mut host).unwrap();
    assert_eq!(first.decider_failures.len(), 1);
    assert_eq!(*count.borrow(), 1);
    let second = world.step(&mut host).unwrap();
    assert_eq!(*count.borrow(), 2);
    assert_eq!(second.decider_failures.len(), 1);
    assert_eq!(second.decider_failures[0].decider, "rule");
}

#[test]
fn an_empty_reply_means_every_business_does_nothing() {
    let mut host = host_of(empty_reply, empty_reply);
    let (world, report) = World::new(load(&test_setting("businesses_valid")), &mut host).unwrap();
    assert_eq!(answers_of(&world), vec![BusinessAnswer::Nothing; 3]);
    assert_eq!(
        report.decisions,
        DecisionCounts {
            nothing: 3,
            ..DecisionCounts::default()
        }
    );
    assert!(report.decider_failures.is_empty());
}

#[test]
fn the_sample_replays_its_recording_at_turn_0() {
    let recording = sample_recording();
    let replay = ReplayDecider::from_files([recording_path()]).unwrap();
    let mut host = DeciderHost::new().with("arithmetic", replay);
    let (mut world, report) = World::new(load(&sample_setting()), &mut host).unwrap();

    for (id, name, _) in world.businesses().iter() {
        let recorded = recording
            .reply
            .decisions
            .iter()
            .find(|d| d.business == name)
            .unwrap_or_else(|| panic!("the recording has a decision for {name}"));
        assert_eq!(
            world.business_answer(id),
            &BusinessAnswer::Valid(recorded.clone()),
            "{name}"
        );
    }
    assert_eq!(
        report.decisions,
        DecisionCounts {
            valid: 3,
            ..DecisionCounts::default()
        }
    );
    assert!(report.decider_failures.is_empty());

    let report = world.step(&mut host).unwrap();
    assert_eq!(answers_of(&world), vec![BusinessAnswer::Nothing; 3]);
    assert_eq!(
        report.decider_failures,
        [ReportedFailure {
            decider: "arithmetic".to_string(),
            failure: DeciderFailure::NoRecording { turn: Turn::new(1) },
        }]
    );
    assert_eq!(
        report.decisions,
        DecisionCounts {
            nothing: 3,
            ..DecisionCounts::default()
        }
    );
}

#[test]
fn a_replay_of_a_different_request_fails() {
    let mut recording = sample_recording();
    let view = recording
        .request
        .views
        .iter_mut()
        .find(|v| v.business == "center_farm")
        .unwrap();
    view.cash = common::q(view.cash.get() * 2.0);
    let replay = ReplayDecider::new([recording]).unwrap();
    let mut host = DeciderHost::new().with("arithmetic", replay);
    let (world, report) = World::new(load(&sample_setting()), &mut host).unwrap();
    assert_eq!(answers_of(&world), vec![BusinessAnswer::Nothing; 3]);
    assert_eq!(
        report.decider_failures,
        [ReportedFailure {
            decider: "arithmetic".to_string(),
            failure: DeciderFailure::RequestDiffers { turn: Turn::ZERO },
        }]
    );
}

#[test]
fn answers_are_replaced_every_turn() {
    let mut host = host_of(
        |r| Reply {
            turn: r.turn,
            decisions: if r.turn.get() % 2 == 0 {
                vec![Decision::empty("farm_a")]
            } else {
                vec![]
            },
        },
        empty_reply,
    );
    let (mut world, _) = World::new(load(&test_setting("businesses_valid")), &mut host).unwrap();
    let farm_a = world.businesses().id("farm_a").unwrap();
    assert_eq!(world.business_answer(farm_a), &valid_empty("farm_a"));
    world.step(&mut host).unwrap();
    assert_eq!(world.business_answer(farm_a), &BusinessAnswer::Nothing);
    world.step(&mut host).unwrap();
    assert_eq!(world.business_answer(farm_a), &valid_empty("farm_a"));
}
