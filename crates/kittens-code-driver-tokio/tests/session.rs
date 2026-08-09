//! End-to-end driver tests (SPEC G2/G10-adjacent at KC0 scope): a
//! jail-scripted session drives the real appender, engine, and filesystem
//! tools, and a second open replays the same log.

use std::sync::Arc;

use kittens_code_core::engine::Engine;
use kittens_code_core::prompts::PROMPT_PACK_VERSION;
use kittens_code_core::record::{LogHeader, Record, RecordKind, RecordPayload};
use kittens_code_driver_tokio::appender::{Appender, CODEC};
use kittens_code_driver_tokio::model::{JailClient, JailStep};
use kittens_code_driver_tokio::runner::Runner;
use kittens_code_protocol::config::SessionConfig;
use kittens_code_protocol::event::{Event, ToolOutcome, TurnEnd};
use kittens_code_protocol::ids::{SessionId, SubmissionId, TurnEpoch, VersionTriple};
use kittens_code_protocol::op::{Op, Submission};

fn header_record(session: u8) -> Record {
    Record::new(
        0,
        RecordKind::Header,
        None,
        TurnEpoch(0),
        RecordPayload::Header(LogHeader {
            session_id: SessionId([session; 16]),
            parent: None,
            schema_epoch: 0,
            prompt_pack_version: PROMPT_PACK_VERSION.0,
            verb_grammar_version: [1, 0, 0],
            l3_dialect_version: [1, 0, 0],
            codec: String::from(CODEC),
            created_at: None,
        }),
    )
    .expect("header record")
}

fn user(id: u64, text: &str) -> Submission {
    Submission {
        id: SubmissionId(id),
        op: Op::UserInput {
            text: String::from(text),
        },
    }
}

#[tokio::test]
async fn message_only_session_persists_and_replays() {
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("session.jsonl");

    // First drive: one message-only turn.
    {
        let (appender, replay) = Appender::open(&log, Some(header_record(1))).expect("open fresh");
        assert_eq!(replay.len(), 1, "fresh log yields just the header");
        let engine = Engine::new(SessionConfig::default(), appender.next_seq());
        let model = Arc::new(JailClient::new(vec![JailStep {
            text: String::from("hello back"),
            tool_calls: vec![],
            usage: Some((10, 40)),
            fail: None,
        }]));
        let mut runner = Runner::new(engine, appender, model, dir.path().to_path_buf());
        runner.submit(user(1, "hello"));
        let events = runner.run_to_idle().await;
        assert!(events.iter().any(|e| matches!(
            e,
            Event::TurnEnded {
                reason: TurnEnd::Completed,
                ..
            }
        )));
    }

    // Reopen: the log replays without repair (clean shutdown).
    let (appender2, replay) = Appender::open(&log, None).expect("reopen");
    assert!(
        replay.len() > 1,
        "replay carries the committed turn, not just the header"
    );
    // The reopened appender continues the sequence.
    assert_eq!(appender2.next_seq(), replay.last().unwrap().seq + 1);
}

#[tokio::test]
async fn tool_round_trip_reads_a_real_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("greeting.txt"), "from disk").expect("seed file");
    let log = dir.path().join("session.jsonl");

    let (appender, _) = Appender::open(&log, Some(header_record(2))).expect("open");
    let engine = Engine::new(SessionConfig::default(), appender.next_seq());
    // Turn 1: propose a read tool call. Turn 2 (resample): finish.
    let model = Arc::new(JailClient::new(vec![
        JailStep {
            text: String::new(),
            tool_calls: vec![(
                String::from("read"),
                String::from("{\"path\":\"greeting.txt\"}"),
            )],
            usage: None,
            fail: None,
        },
        JailStep {
            text: String::from("the file said: from disk"),
            tool_calls: vec![],
            usage: None,
            fail: None,
        },
    ]));
    let mut runner = Runner::new(engine, appender, model, dir.path().to_path_buf());
    runner.submit(user(1, "read the greeting"));
    let events = runner.run_to_idle().await;

    assert!(events.iter().any(|e| matches!(
        e,
        Event::ToolTerminal {
            outcome: ToolOutcome::Succeeded,
            ..
        }
    )));
    assert!(events.iter().any(|e| matches!(
        e,
        Event::TurnEnded {
            reason: TurnEnd::Completed,
            ..
        }
    )));
}

#[tokio::test]
async fn crash_repair_closes_an_open_stream_on_reopen() {
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("session.jsonl");

    // Hand-write a header plus an orphan StreamStarted (a crash mid-stream).
    let header = header_record(3);
    let started = Record::new(
        1,
        RecordKind::StreamStarted,
        Some(kittens_code_protocol::ids::EffectId(7)),
        TurnEpoch(1),
        RecordPayload::StreamStarted(Vec::new()),
    )
    .expect("started record");
    let mut text = String::new();
    text.push_str(&serde_json::to_string(&header).unwrap());
    text.push('\n');
    text.push_str(&serde_json::to_string(&started).unwrap());
    text.push('\n');
    std::fs::write(&log, text).expect("seed log");

    // Reopen: the scanner must append a repair terminal for effect 7.
    let (_appender, replay) = Appender::open(&log, None).expect("reopen with repair");
    assert!(
        replay
            .iter()
            .any(|r| matches!(r.kind, RecordKind::RepairTerminal)),
        "an aborted_by_crash repair terminal was persisted and replayed"
    );
}

#[tokio::test]
async fn schema_epoch_from_the_future_is_refused_before_mutation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("session.jsonl");
    // Header with an unsupported epoch.
    let future = Record::new(
        0,
        RecordKind::Header,
        None,
        TurnEpoch(0),
        RecordPayload::Header(LogHeader {
            session_id: SessionId([9; 16]),
            parent: None,
            schema_epoch: 999,
            prompt_pack_version: VersionTriple::default().0,
            verb_grammar_version: [1, 0, 0],
            l3_dialect_version: [1, 0, 0],
            codec: String::from(CODEC),
            created_at: None,
        }),
    )
    .expect("future header");
    let mut text = serde_json::to_string(&future).unwrap();
    text.push('\n');
    std::fs::write(&log, &text).expect("seed");
    let before = std::fs::read_to_string(&log).unwrap();

    let result = Appender::open(&log, None);
    assert!(result.is_err(), "future epoch is refused");
    // The refusal happened before any write.
    assert_eq!(std::fs::read_to_string(&log).unwrap(), before);
}
