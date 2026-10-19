//! Integration tests for the external-process decider and the project's Python. These need
//! the project's virtual environment (see `decider::python`); without it they fail with
//! its message rather than skip.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::{Duration, Instant};

use common::{load, sample_setting, test_setting};
use policyspace_engine::decider::python::{self, HostError};
use policyspace_engine::decider::{
    ExternalCommand, ExternalDecider, ProcessConfig, ReplyError, StartError,
};
use policyspace_engine::{
    BusinessAnswer, DeciderFailure, DeciderHost, FormatError, Recording, Turn, TurnReport, World,
};

fn python() -> PathBuf {
    python::interpreter().unwrap_or_else(|e| panic!("{e}"))
}

/// `python -c code args...`.
fn script(code: &str, args: &[&str]) -> ExternalCommand {
    ExternalCommand::new(python())
        .arg("-c")
        .arg(code)
        .args(args.iter().copied())
}

fn config(limit_ms: u64) -> ProcessConfig {
    ProcessConfig {
        time_limit: Some(Duration::from_millis(limit_ms)),
        ..ProcessConfig::default()
    }
}

/// The sample world under `host`.
fn sample_world(host: &mut DeciderHost) -> (World, TurnReport) {
    World::new(load(&sample_setting()), host).expect("the host holds `arithmetic`")
}

/// A host holding a script as the sample's `arithmetic` decider.
fn host_with_script(command: ExternalCommand, config: ProcessConfig) -> DeciderHost {
    let decider = ExternalDecider::start(command, config).unwrap_or_else(|e| panic!("{e}"));
    DeciderHost::new().with("arithmetic", decider)
}

fn read_counter(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}

#[test]
fn the_sample_runs_under_the_arithmetic_decider() {
    let recording_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/recordings/sample_turn0.recording.json");
    let recording = Recording::from_json(&fs::read_to_string(recording_path).unwrap()).unwrap();

    let mut host = python::host_for(&load(&sample_setting())).unwrap_or_else(|e| panic!("{e}"));
    let (mut world, report) = sample_world(&mut host);
    for (id, name, _) in world.businesses().iter() {
        let recorded = recording
            .reply
            .decisions
            .iter()
            .find(|d| d.business == name)
            .unwrap();
        assert_eq!(
            world.business_answer(id),
            &BusinessAnswer::Valid(recorded.clone()),
            "{name}"
        );
    }
    assert_eq!(report.decisions.valid, 3);
    assert_eq!(report.decisions.refused + report.decisions.nothing, 0);
    assert!(report.decider_failures.is_empty());

    for _ in 0..5 {
        let report = world.step(&mut host).unwrap();
        assert_eq!(report.decisions.valid, 3, "turn {}", report.turn);
        assert_eq!(report.decisions.refused + report.decisions.nothing, 0);
        assert!(report.decider_failures.is_empty(), "turn {}", report.turn);
        for (id, _, _) in world.businesses().iter() {
            assert!(matches!(
                world.business_answer(id),
                BusinessAnswer::Valid(_)
            ));
        }
    }
}

#[test]
fn a_process_that_exits_before_answering_idles_its_businesses_and_is_restarted() {
    let dir = tempfile::tempdir().unwrap();
    let counter = dir.path().join("count.txt");
    fs::write(&counter, "0").unwrap();
    let path = counter.to_str().unwrap();
    let command = script(
        "import sys,pathlib; p=pathlib.Path(sys.argv[1]); p.write_text(str(int(p.read_text())+1)); \
         sys.stdin.readline(); print('boom', file=sys.stderr); sys.exit(3)",
        &[path],
    );
    let mut host = host_with_script(command, ProcessConfig::default());

    let (mut world, report) = sample_world(&mut host);
    check_exited(&report);
    assert_eq!(read_counter(&counter), "1");

    let report = world.step(&mut host).unwrap();
    check_exited(&report);
    assert_eq!(read_counter(&counter), "2");
    for (id, _, _) in world.businesses().iter() {
        assert_eq!(world.business_answer(id), &BusinessAnswer::Nothing);
    }
}

#[test]
fn a_process_whose_reply_is_refused_is_restarted() {
    let dir = tempfile::tempdir().unwrap();
    let counter = dir.path().join("count.txt");
    fs::write(&counter, "0").unwrap();
    let path = counter.to_str().unwrap();
    // After each request, two reply lines for that turn: the second one is left in the pipe,
    // so the next turn's exchange reads a stale reply.
    let command = script(
        "import sys,json,pathlib; p=pathlib.Path(sys.argv[1]); p.write_text(str(int(p.read_text())+1))\n\
         for line in sys.stdin:\n\
         \tturn=json.loads(line)['turn']\n\
         \tr=json.dumps({'turn': turn, 'decisions': []})\n\
         \tprint(r, flush=True)\n\
         \tprint(r, flush=True)\n",
        &[path],
    );
    let mut host = host_with_script(command, ProcessConfig::default());

    let (mut world, report) = sample_world(&mut host);
    assert!(report.decider_failures.is_empty());
    assert_eq!(report.decisions.nothing, 3);

    let report = world.step(&mut host).unwrap();
    assert_eq!(report.decider_failures.len(), 1);
    match &report.decider_failures[0].failure {
        DeciderFailure::Refused(errors) => {
            assert!(
                errors.iter().any(|e| matches!(
                    e,
                    ReplyError::TurnMismatch { request, reply }
                        if *request == Turn::new(1) && *reply == Turn::new(0)
                )),
                "{errors:?}"
            );
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(report.decisions.nothing, 3);

    let report = world.step(&mut host).unwrap();
    assert!(report.decider_failures.is_empty(), "{report:?}");
    assert_eq!(report.decisions.nothing, 3);
    assert_eq!(read_counter(&counter), "2");
}

fn check_exited(report: &TurnReport) {
    assert_eq!(report.decider_failures.len(), 1);
    assert_eq!(report.decider_failures[0].decider, "arithmetic");
    match &report.decider_failures[0].failure {
        DeciderFailure::Exited { status, stderr } => {
            assert_eq!(status.code(), Some(3));
            assert_eq!(stderr, &["boom".to_string()]);
        }
        other => panic!("{other:?}"),
    }
    assert!(
        report.decider_failures[0]
            .failure
            .to_string()
            .contains("the process exited")
    );
    assert_eq!(report.decisions.nothing, 3);
    assert_eq!(report.decisions.valid + report.decisions.refused, 0);
}

#[test]
fn a_process_that_answers_late_times_out() {
    let command = script("import sys,time; sys.stdin.readline(); time.sleep(30)", &[]);
    let mut host = host_with_script(command, config(300));
    let started = Instant::now();
    let (mut world, report) = sample_world(&mut host);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "took {:?}",
        started.elapsed()
    );
    assert_eq!(report.decider_failures.len(), 1);
    assert!(matches!(
        &report.decider_failures[0].failure,
        DeciderFailure::TimedOut { limit, .. } if *limit == Duration::from_millis(300)
    ));

    let report = world.step(&mut host).unwrap();
    assert!(matches!(
        &report.decider_failures[0].failure,
        DeciderFailure::TimedOut { .. }
    ));
}

#[test]
fn a_reply_that_isnt_a_reply_is_a_failure() {
    let command = script(
        "import sys; sys.stdin.readline(); print('not json', flush=True); sys.stdin.readline()",
        &[],
    );
    let mut host = host_with_script(command, ProcessConfig::default());
    let (_, report) = sample_world(&mut host);
    assert_eq!(report.decider_failures.len(), 1);
    let failure = &report.decider_failures[0].failure;
    assert!(matches!(
        failure,
        DeciderFailure::Unreadable {
            error: FormatError::Json(_),
            ..
        }
    ));
    assert!(
        failure
            .to_string()
            .starts_with("the reply couldn't be read: ")
    );
}

#[test]
fn a_script_that_exits_at_once_fails_host_construction() {
    let error = ExternalDecider::start(
        script("import sys; sys.exit(2)", &[]),
        ProcessConfig::default(),
    )
    .err()
    .expect("it exits at once");
    match error {
        StartError::ExitedAtOnce { status, .. } => assert_eq!(status.code(), Some(2)),
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_unknown_decider_name_is_a_host_error() {
    let setting = load(&test_setting("production_valid"));
    let error = python::host_for(&setting).expect_err("`rule` is unknown");
    match &error {
        HostError::Start {
            decider,
            error: StartError::ExitedAtOnce { status, stderr, .. },
        } => {
            assert_eq!(decider, "rule");
            assert_eq!(status.code(), Some(2));
            assert!(
                stderr.iter().any(|l| l.contains("unknown decider 'rule'")),
                "{stderr:?}"
            );
        }
        other => panic!("{other:?}"),
    }
    let text = error.to_string();
    assert!(text.contains("decider 'rule'"), "{text}");
    assert!(text.contains("unknown decider 'rule'"), "{text}");
}

#[test]
fn dropping_a_host_ends_its_processes() {
    let dir = tempfile::tempdir().unwrap();
    let beat = dir.path().join("beat.txt");
    let path = beat.to_str().unwrap();
    let command = script(
        // Appends a dot every 50 ms (appending never leaves the file briefly empty, as
        // rewriting it would).
        "import sys,time\n\
         while True:\n open(sys.argv[1], 'a').write('.'); time.sleep(0.05)",
        &[path],
    );
    let host = host_with_script(command, ProcessConfig::default());

    sleep(Duration::from_millis(300));
    let first = fs::read_to_string(&beat).unwrap();
    sleep(Duration::from_millis(200));
    let second = fs::read_to_string(&beat).unwrap();
    assert_ne!(first, second, "the heartbeat should be running");

    drop(host);
    sleep(Duration::from_millis(300));
    let before = fs::read_to_string(&beat).unwrap();
    sleep(Duration::from_millis(300));
    let after = fs::read_to_string(&beat).unwrap();
    assert_eq!(before, after, "the heartbeat should have stopped");
}
