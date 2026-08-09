//! End-to-end tests for the in-memory headless protocol transport.

use std::sync::Arc;

use kittens_code_cli::{fresh_header, run};
use kittens_code_driver_tokio::model::{JailClient, JailStep};
use kittens_code_driver_tokio::runner::Runner;
use kittens_code_protocol::config::SessionConfig;
use kittens_code_protocol::error::ErrorCode;
use kittens_code_protocol::event::{Event, TurnEnd};
use kittens_code_protocol::ids::SessionId;
use kittens_code_protocol::op::Op;
use tokio::io::BufReader;

fn runner(steps: Vec<JailStep>) -> (tempfile::TempDir, Runner) {
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("session.jsonl");
    let runner = Runner::open(
        &log,
        Some(fresh_header(SessionId([0x43; 16])).expect("header")),
        SessionConfig::default(),
        Arc::new(JailClient::new(steps)),
        dir.path().to_path_buf(),
    )
    .expect("runner");
    (dir, runner)
}

async fn drive(input: &str, runner: &mut Runner) -> Vec<Event> {
    let reader = BufReader::new(input.as_bytes());
    let mut output = Vec::new();
    run(reader, &mut output, runner)
        .await
        .expect("protocol run");
    String::from_utf8(output)
        .expect("utf8 output")
        .lines()
        .map(|line| serde_json::from_str(line).expect("event JSON"))
        .collect()
}

#[tokio::test]
async fn user_input_emits_one_complete_turn() {
    let (_dir, mut runner) = runner(vec![JailStep {
        text: String::from("hello back"),
        tool_calls: vec![],
        usage: Some((5, 20)),
        fail: None,
    }]);
    let input = format!(
        "{}\n",
        serde_json::to_string(&Op::UserInput {
            text: String::from("hello")
        })
        .expect("op JSON")
    );

    let events = drive(&input, &mut runner).await;

    assert!(matches!(events.first(), Some(Event::TurnStarted { .. })));
    assert!(matches!(
        events.last(),
        Some(Event::TurnEnded {
            reason: TurnEnd::Completed,
            ..
        })
    ));
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::TurnStarted { .. }))
            .count(),
        1,
        "events are emitted once rather than replayed after the EOF drain"
    );
}

#[tokio::test]
async fn shutdown_drains_and_stops_before_later_input() {
    let (_dir, mut runner) = runner(Vec::new());
    let input = "{\"op\":\"shutdown\"}\n{\"op\":\"user_input\",\"text\":\"too late\"}\n";

    let events = drive(input, &mut runner).await;

    assert_eq!(events, vec![Event::ShuttingDown]);
}

#[tokio::test]
async fn malformed_json_reports_an_error_and_continues() {
    let (_dir, mut runner) = runner(vec![JailStep {
        text: String::from("still running"),
        tool_calls: vec![],
        usage: None,
        fail: None,
    }]);
    let input = "not-json\n{\"op\":\"user_input\",\"text\":\"hello\"}\n";

    let events = drive(input, &mut runner).await;

    assert!(matches!(
        events.first(),
        Some(Event::Error(error)) if error.code == ErrorCode::ConfigInvalid
    ));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::TurnEnded { .. }))
    );
}
