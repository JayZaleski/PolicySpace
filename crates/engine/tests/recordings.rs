//! Fixture tests for the committed recordings in `tests/recordings/`: the sample's turn-0
//! recording, the exactness request and the hand-written requests. Set
//! `POLICYSPACE_UPDATE_RECORDINGS` to rewrite the engine's halves (see the folder's README).

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use common::{q, sample_setting, world_of};
use policyspace_engine::decider::{DepositsView, FactoryView, PerishGroup, TechnologyView};
use policyspace_engine::{Answer, BusinessView, Quantity, Recording, Reply, Request, Turn, World};

fn recordings_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recordings")
}

fn updating() -> bool {
    std::env::var_os("POLICYSPACE_UPDATE_RECORDINGS").is_some()
}

const REGENERATE: &str = "regenerate the recordings: set POLICYSPACE_UPDATE_RECORDINGS, run \
`cargo test -p policyspace-engine --test recordings -- --test-threads=1`, then \
`.venv/Scripts/python -m pytest python/tests`, then the cargo test again";

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("{} can't be read ({error}); {REGENERATE}", path.display()))
}

/// The request the engine builds for every business of `world`, in business ID order.
fn sample_request(world: &World) -> Request {
    Request {
        turn: world.turn(),
        views: world
            .businesses()
            .iter()
            .map(|(id, _, _)| world.business_view(id))
            .collect(),
    }
}

#[test]
fn the_sample_recording_matches_the_engine() {
    let (world, _) = world_of(&sample_setting());
    let request = sample_request(&world);
    let path = recordings_dir().join("sample_turn0.recording.json");

    if updating() {
        let reply = fs::read_to_string(&path)
            .ok()
            .and_then(|text| Recording::from_json(&text).ok())
            .map(|recording| recording.reply)
            .unwrap_or(Reply {
                turn: request.turn,
                decisions: vec![],
            });
        let recording = Recording {
            request: request.clone(),
            reply,
        };
        fs::write(&path, recording.to_json_pretty() + "\n").unwrap();
    }

    let text = read(&path);
    let recording = Recording::from_json(&text)
        .unwrap_or_else(|error| panic!("{} doesn't read: {error}", path.display()));
    assert_eq!(
        recording.request, request,
        "the recorded request is stale; {REGENERATE}"
    );
    assert_eq!(
        text,
        recording.to_json_pretty() + "\n",
        "the recording isn't in the engine's canonical form; {REGENERATE}"
    );

    let answers = recording
        .reply
        .validate(&request, &world.rules().goods)
        .unwrap_or_else(|errors| panic!("the recorded reply is refused: {errors}; {REGENERATE}"));
    assert_eq!(answers.len(), request.views.len());
    for (view, answer) in request.views.iter().zip(&answers) {
        match answer {
            Answer::Valid(decision) => assert_eq!(decision.business, view.business),
            other => panic!(
                "no valid decision for {}: {other:?}; {REGENERATE}",
                view.business
            ),
        }
    }
    assert_eq!(recording.reply.decisions.len(), request.views.len());
    for (view, decision) in request.views.iter().zip(&recording.reply.decisions) {
        assert_eq!(
            decision.business, view.business,
            "decisions follow the request's order"
        );
    }
}

fn quantities(items: &[(&str, f64)]) -> BTreeMap<String, Quantity> {
    items.iter().map(|(n, a)| (n.to_string(), q(*a))).collect()
}

/// The request whose JSON holds the numbers Python's writer lays out differently from the
/// engine's, and the extremes of every type.
fn exactness_request() -> Request {
    let never = |amount: f64| PerishGroup {
        amount: q(amount),
        perishes: None,
    };
    let view = BusinessView {
        turn: Turn::MAX,
        business: "exact".to_string(),
        currency: "coin".to_string(),
        cash: q(f64::MAX),
        prices: quantities(&[
            ("a", 1e-5),
            ("b", 1.23e-5),
            ("c", 1e-6),
            ("d", 1e-4),
            ("e", 1e16),
            ("f", 1e15),
            ("g", 1.5e16),
            ("h", 123.0),
            ("i", 1e21),
            ("j", 12345678901234567.0),
            ("k", 0.1),
            ("l", 100.0),
            ("m", 5e-324),
            ("n", 1.0 / 3.0),
            ("o", 123_456_789.123_456_79),
            ("p", 1e-300),
        ]),
        stockpile: BTreeMap::from([
            ("coin".to_string(), vec![never(f64::MAX)]),
            (
                "a".to_string(),
                vec![
                    PerishGroup {
                        amount: q(5e-324),
                        perishes: Some(Turn::MAX),
                    },
                    PerishGroup {
                        amount: q(0.0),
                        perishes: Some(Turn::ZERO),
                    },
                ],
            ),
        ]),
        factories: BTreeMap::from([
            (
                0,
                FactoryView {
                    technology: "t".to_string(),
                    condition: 1.0 / 3.0,
                },
            ),
            (
                u32::MAX,
                FactoryView {
                    technology: "t".to_string(),
                    condition: 0.0,
                },
            ),
        ]),
        technologies: BTreeMap::from([(
            "t".to_string(),
            TechnologyView {
                inputs: quantities(&[("a", 2.5)]),
                output: "b".to_string(),
                capacity: q(1e-5),
                capital: quantities(&[("c", 7.0)]),
                construction: BTreeMap::new(),
                construction_turns: u32::MAX,
                lifetime: 1,
            },
        )]),
        deposits: BTreeMap::from([(
            "z".to_string(),
            DepositsView {
                count: u32::MAX,
                produces: "a".to_string(),
                output: q(0.0),
            },
        )]),
    };
    Request {
        turn: Turn::MAX,
        views: vec![view],
    }
}

#[test]
fn the_exactness_request_is_the_engines() {
    let text = exactness_request().to_json() + "\n";
    let path = recordings_dir().join("exactness.request.json");
    if updating() {
        fs::write(&path, &text).unwrap();
    }
    assert_eq!(
        read(&path),
        text,
        "the exactness request is stale; {REGENERATE}"
    );
}

#[test]
fn hand_written_requests_parse() {
    let mut paths: Vec<PathBuf> = fs::read_dir(recordings_dir())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".request.json"))
        })
        .collect();
    paths.sort();
    if updating() {
        // The exactness request is being rewritten by its own test at the same time.
        paths.retain(|path| {
            path.file_name()
                .is_some_and(|name| name != "exactness.request.json")
        });
        assert_eq!(
            paths.len(),
            6,
            "expected the six hand-written request files"
        );
    } else {
        assert_eq!(
            paths.len(),
            7,
            "expected the exactness request and the six hand-written request files"
        );
    }
    for path in paths {
        let request = Request::from_json(&read(&path))
            .unwrap_or_else(|error| panic!("{} doesn't read: {error}", path.display()));
        assert!(!request.views.is_empty(), "{} has no views", path.display());
        for view in &request.views {
            assert_eq!(view.turn, request.turn, "{} view turn", path.display());
        }
    }
}
