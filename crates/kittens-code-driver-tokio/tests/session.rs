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
async fn recall_pages_over_the_real_log_and_completes_the_turn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("session.jsonl");
    let (appender, _) = Appender::open(&log, Some(header_record(4))).expect("open");
    let engine = Engine::new(SessionConfig::default(), appender.next_seq());
    let model = Arc::new(JailClient::new(vec![
        JailStep {
            text: String::new(),
            tool_calls: vec![(
                String::from("recall"),
                serde_json::json!({
                    "script": "grep \"needle\"\nfinal %1"
                })
                .to_string(),
            )],
            usage: None,
            fail: None,
        },
        JailStep {
            text: String::from("found it in the transcript"),
            tool_calls: vec![],
            usage: None,
            fail: None,
        },
    ]));
    let mut runner = Runner::new(engine, appender, model.clone(), dir.path().to_path_buf());
    runner.submit(user(1, "put this needle in the real log"));
    let events = runner.run_to_idle().await;

    assert!(events.iter().any(|event| matches!(
        event,
        Event::ToolTerminal {
            outcome: ToolOutcome::Succeeded,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        Event::TurnEnded {
            reason: TurnEnd::Completed,
            ..
        }
    )));
    assert_eq!(model.captured().len(), 2, "recall completion resampled");
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

#[tokio::test]
async fn reopened_session_resumes_without_id_collision() {
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("session.jsonl");

    // First drive: one completed turn writes records to the log.
    {
        let model = Arc::new(JailClient::new(vec![JailStep {
            text: String::from("first"),
            tool_calls: vec![],
            usage: None,
            fail: None,
        }]));
        let mut runner = Runner::open(
            &log,
            Some(header_record(1)),
            SessionConfig::default(),
            model,
            dir.path().to_path_buf(),
        )
        .expect("open fresh");
        runner.submit(user(1, "hello"));
        runner.run_to_idle().await;
    }

    // Reopen via Runner::open: the engine resumes from the log. A new turn
    // must produce records whose sequences continue past the persisted max.
    let max_seq_before = {
        let text = std::fs::read_to_string(&log).unwrap();
        text.lines().count() as u64 - 1 // records are 0-indexed by line
    };
    let model = Arc::new(JailClient::new(vec![JailStep {
        text: String::from("second"),
        tool_calls: vec![],
        usage: None,
        fail: None,
    }]));
    let mut runner = Runner::open(
        &log,
        None,
        SessionConfig::default(),
        model,
        dir.path().to_path_buf(),
    )
    .expect("reopen resumes");
    runner.submit(user(2, "again"));
    let events = runner.run_to_idle().await;
    assert!(events.iter().any(|e| matches!(
        e,
        Event::TurnEnded {
            reason: TurnEnd::Completed,
            ..
        }
    )));
    // The log grew: the resumed turn appended new records past the prior max.
    let after = std::fs::read_to_string(&log).unwrap().lines().count() as u64;
    assert!(
        after > max_seq_before,
        "resumed session appended new records"
    );
}

#[tokio::test]
async fn torn_tail_line_is_tolerated_and_reopen_continues() {
    // A crash mid-write leaves a truncated final JSON line. Reopen must
    // ignore that torn tail, keep the valid prefix, and continue appending
    // (review input 19 #24).
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("session.jsonl");
    let header = header_record(4);
    let mut text = String::new();
    text.push_str(&serde_json::to_string(&header).unwrap());
    text.push('\n');
    // A half-written record: valid JSON prefix, no closing brace, no newline.
    text.push_str("{\"seq\":1,\"kind\":\"emitted_event\",\"txn\":null,");
    std::fs::write(&log, text).expect("seed log");

    let (mut appender, replay) = Appender::open(&log, None).expect("reopen past torn tail");
    // Only the header survives the prefix; the torn line is dropped.
    assert_eq!(replay.len(), 1);
    assert_eq!(appender.next_seq(), 1);
    // Appending continues cleanly from the recovered sequence.
    let rec = Record::new(
        1,
        RecordKind::EmittedEvent,
        None,
        TurnEpoch(0),
        RecordPayload::EmittedEvent(kittens_code_protocol::event::Event::ShuttingDown),
    )
    .expect("record");
    assert!(appender.append(&[rec]).is_ok());
}

#[tokio::test]
async fn checksum_corrupt_tail_line_is_tolerated() {
    // A last line whose checksum no longer matches its payload (bit-rot on
    // the final unsynced write) is treated as a tolerable tail, not a fatal
    // mid-log fault (review input 19 #24).
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("session.jsonl");
    let header = header_record(5);
    // A well-formed record whose checksum we then corrupt.
    let rec = Record::new(
        1,
        RecordKind::EmittedEvent,
        None,
        TurnEpoch(0),
        RecordPayload::EmittedEvent(kittens_code_protocol::event::Event::ShuttingDown),
    )
    .expect("record");
    // Corrupt the checksum by flipping one bit of its stored value.
    let mut json: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&rec).unwrap()).unwrap();
    let stored = json["checksum"].as_u64().unwrap();
    json["checksum"] = serde_json::json!(stored ^ 1);
    let mut text = String::new();
    text.push_str(&serde_json::to_string(&header).unwrap());
    text.push('\n');
    text.push_str(&serde_json::to_string(&json).unwrap());
    text.push('\n');
    std::fs::write(&log, text).expect("seed log");

    let (appender, replay) = Appender::open(&log, None).expect("reopen past corrupt tail");
    assert_eq!(replay.len(), 1, "corrupt tail dropped, header retained");
    assert_eq!(appender.next_seq(), 1);
}

#[tokio::test]
async fn out_of_order_append_is_refused_in_release() {
    // The strict-order contract is enforced in release, not just debug
    // (review input 19 #16/#24): an append at the wrong sequence errors
    // before corrupting the log.
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("session.jsonl");
    let (mut appender, _) = Appender::open(&log, Some(header_record(6))).expect("open");
    // Header consumed seq 0; next expected is 1. Try to append seq 5.
    let wrong = Record::new(
        5,
        RecordKind::EmittedEvent,
        None,
        TurnEpoch(0),
        RecordPayload::EmittedEvent(kittens_code_protocol::event::Event::ShuttingDown),
    )
    .expect("record");
    let err = appender.append(&[wrong]).expect_err("out-of-order refused");
    assert_eq!(err.0, 5, "the failing sequence is reported");
}
