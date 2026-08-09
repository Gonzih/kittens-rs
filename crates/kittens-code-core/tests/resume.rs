//! S2 resume-as-replay coverage: persisted state seeds all monotonic
//! namespaces without re-emitting replayed work.

use std::collections::BTreeMap;

use kittens_code_core::engine::{
    CoreAction, CoreInput, EffectSpec, EffectTerminal, Engine, ModelOutcome, ProposedToolCall,
    ResumeError,
};
use kittens_code_core::record::{LogHeader, Record, RecordKind, RecordPayload};
use kittens_code_protocol::budgets::Budgets;
use kittens_code_protocol::config::{SessionConfig, SessionConfigPatch};
use kittens_code_protocol::event::{Event, ToolOutcome};
use kittens_code_protocol::ids::{EffectId, SessionId, SubmissionId, TurnEpoch};
use kittens_code_protocol::op::{Op, Submission};
use kittens_code_protocol::policy::{ApprovalPolicy, ApprovalVerdict};

fn record(
    seq: u64,
    kind: RecordKind,
    txn: Option<u64>,
    epoch: u64,
    payload: RecordPayload,
) -> Record {
    Record::new(seq, kind, txn.map(EffectId), TurnEpoch(epoch), payload)
        .expect("test record kind matches its payload")
}

fn header() -> Record {
    record(
        0,
        RecordKind::Header,
        None,
        0,
        RecordPayload::Header(LogHeader {
            session_id: SessionId([0x52; 16]),
            parent: None,
            schema_epoch: 1,
            prompt_pack_version: [1, 0, 0],
            verb_grammar_version: [1, 0, 0],
            l3_dialect_version: [1, 0, 0],
            codec: String::from("jsonl"),
            created_at: None,
        }),
    )
}

fn user_submission(id: u64, text: &str) -> Submission {
    Submission {
        id: SubmissionId(id),
        op: Op::UserInput {
            text: String::from(text),
        },
    }
}

fn committed(actions: &[CoreAction]) -> Vec<&Record> {
    actions
        .iter()
        .filter_map(|action| match action {
            CoreAction::Commit(records) => Some(records.iter()),
            _ => None,
        })
        .flatten()
        .collect()
}

fn retain_committed(log: &mut Vec<Record>, actions: &[CoreAction]) {
    log.extend(committed(actions).into_iter().cloned());
}

fn started_model(actions: &[CoreAction]) -> (EffectId, TurnEpoch) {
    actions
        .iter()
        .find_map(|action| match action {
            CoreAction::StartEffect {
                id,
                epoch,
                spec: EffectSpec::ModelCall(_),
            } => Some((*id, *epoch)),
            _ => None,
        })
        .expect("transition starts a model call")
}

fn started_tool(actions: &[CoreAction]) -> (EffectId, TurnEpoch) {
    actions
        .iter()
        .find_map(|action| match action {
            CoreAction::StartEffect {
                id,
                epoch,
                spec: EffectSpec::Tool(_),
            } => Some((*id, *epoch)),
            _ => None,
        })
        .expect("transition starts a tool")
}

fn model_window(actions: &[CoreAction]) -> kittens_code_core::window::WindowLayout {
    actions
        .iter()
        .find_map(|action| match action {
            CoreAction::StartEffect {
                spec: EffectSpec::ModelCall(window),
                ..
            } => Some(window.clone()),
            _ => None,
        })
        .expect("transition carries the next model window")
}

fn replay_log() -> Vec<Record> {
    let mut budgets = Budgets::default();
    budgets.tool_result_bytes = 321;
    let mut approvals = BTreeMap::new();
    approvals.insert(String::from("shell"), ApprovalPolicy::Ask);
    let mut patch = SessionConfigPatch::default();
    patch.budgets = Some(budgets);
    patch.approval_defaults = Some(approvals);

    vec![
        header(),
        record(
            1,
            RecordKind::AcceptedOp,
            None,
            3,
            RecordPayload::AcceptedOp(user_submission(100, "old turn")),
        ),
        record(
            2,
            RecordKind::AcceptedOp,
            None,
            4,
            RecordPayload::AcceptedOp(Submission {
                id: SubmissionId(700),
                op: Op::Approve {
                    request: SubmissionId(750),
                    verdict: ApprovalVerdict::Approve,
                },
            }),
        ),
        record(
            3,
            RecordKind::ConfigPatch,
            None,
            4,
            RecordPayload::ConfigPatch(patch),
        ),
        record(
            4,
            RecordKind::EmittedEvent,
            None,
            8,
            RecordPayload::EmittedEvent(Event::ApprovalRequested {
                request: SubmissionId(800),
                call: EffectId(600),
                description: String::from("old approval"),
            }),
        ),
        record(
            5,
            RecordKind::EffectOutcome,
            Some(900),
            12,
            RecordPayload::EffectOutcome(vec![1, 2, 3]),
        ),
    ]
}

#[test]
fn replay_restores_config_and_seeds_every_monotonic_namespace() {
    let records = replay_log();

    let mut engine = Engine::resume(SessionConfig::default(), &records).expect("valid log resumes");
    assert_eq!(engine.config().budgets.tool_result_bytes, 321);
    assert_eq!(engine.persisted(), 5, "the replay slice is already durable");

    let transition = engine.handle(CoreInput::ClientOp(user_submission(901, "new turn")));
    let records_after_resume = committed(&transition.actions);
    assert_eq!(
        records_after_resume.first().map(|record| record.seq),
        Some(6)
    );
    assert!(records_after_resume.iter().all(|record| record.seq > 5));

    let (model_id, epoch) = transition
        .actions
        .iter()
        .find_map(|action| match action {
            CoreAction::StartEffect { id, epoch, .. } => Some((*id, *epoch)),
            _ => None,
        })
        .expect("new user input starts a model effect");
    assert_eq!(epoch, TurnEpoch(13));
    assert!(epoch > TurnEpoch(12));
    assert_eq!(model_id, EffectId(901));
    assert!(transition.actions.iter().any(|action| matches!(
        action,
        CoreAction::Publish(Event::TurnStarted {
            epoch: TurnEpoch(13),
            ..
        })
    )));

    let tool_transition = engine.handle(CoreInput::EffectFinished {
        id: model_id,
        epoch,
        terminal: EffectTerminal::Model(ModelOutcome {
            text: String::new(),
            tool_calls: vec![ProposedToolCall {
                name: String::from("shell"),
                args_json: String::from("{}"),
            }],
            usage: None,
        }),
    });
    assert!(tool_transition.actions.iter().any(|action| matches!(
        action,
        CoreAction::Publish(Event::ApprovalRequested {
            request: SubmissionId(801),
            call: EffectId(902),
            ..
        })
    )));
}

#[test]
fn header_only_resume_behaves_like_a_fresh_engine() {
    let config = SessionConfig::default();
    let mut fresh = Engine::new(config.clone(), 1);
    let mut resumed = Engine::resume(config, &[header()]).expect("header-only log resumes");
    let submission = user_submission(1, "hello");

    assert_eq!(
        resumed.handle(CoreInput::ClientOp(submission.clone())),
        fresh.handle(CoreInput::ClientOp(submission))
    );
    assert_eq!(resumed.epoch(), TurnEpoch(1));
}

#[test]
fn resumed_window_matches_uninterrupted_reconstructable_state() {
    let config = SessionConfig::default();
    let mut uninterrupted = Engine::new(config.clone(), 1);
    let mut log = vec![header()];

    // Turn one exercises the full reconstructable call/result path.
    let transition = uninterrupted.handle(CoreInput::ClientOp(user_submission(1, "read it")));
    retain_committed(&mut log, &transition.actions);
    let (model, epoch) = started_model(&transition.actions);
    let transition = uninterrupted.handle(CoreInput::EffectFinished {
        id: model,
        epoch,
        terminal: EffectTerminal::Model(ModelOutcome {
            // Final assistant text is currently lossy unless authoritative
            // ModelDelta records exist, so this equivalence slice keeps it
            // empty and proves every state-bearing record that does exist.
            text: String::new(),
            tool_calls: vec![ProposedToolCall {
                name: String::from("read"),
                args_json: String::from("{\"path\":\"note.txt\"}"),
            }],
            usage: None,
        }),
    });
    retain_committed(&mut log, &transition.actions);
    let (tool, tool_epoch) = started_tool(&transition.actions);
    let transition = uninterrupted.handle(CoreInput::EffectFinished {
        id: tool,
        epoch: tool_epoch,
        terminal: EffectTerminal::Tool {
            outcome: ToolOutcome::Succeeded,
            output: String::from("persisted tool output"),
        },
    });
    retain_committed(&mut log, &transition.actions);
    let (model, epoch) = started_model(&transition.actions);
    let transition = uninterrupted.handle(CoreInput::EffectFinished {
        id: model,
        epoch,
        terminal: EffectTerminal::Model(ModelOutcome {
            text: String::new(),
            tool_calls: vec![],
            usage: None,
        }),
    });
    retain_committed(&mut log, &transition.actions);

    // Turn two proves that replay distinguishes an idle user input (the
    // last-query region) from a mid-turn interjection (the tail region).
    let transition = uninterrupted.handle(CoreInput::ClientOp(user_submission(2, "second turn")));
    retain_committed(&mut log, &transition.actions);
    let (model, epoch) = started_model(&transition.actions);
    let transition = uninterrupted.handle(CoreInput::ClientOp(Submission {
        id: SubmissionId(3),
        op: Op::Interject {
            text: String::from("include this too"),
        },
    }));
    retain_committed(&mut log, &transition.actions);
    let transition = uninterrupted.handle(CoreInput::EffectFinished {
        id: model,
        epoch,
        terminal: EffectTerminal::Model(ModelOutcome {
            text: String::new(),
            tool_calls: vec![],
            usage: None,
        }),
    });
    retain_committed(&mut log, &transition.actions);

    let mut resumed = Engine::resume(config, &log).expect("committed log resumes");
    let next = user_submission(4, "same next question");
    let uninterrupted_actions = uninterrupted
        .handle(CoreInput::ClientOp(next.clone()))
        .actions;
    let resumed_actions = resumed.handle(CoreInput::ClientOp(next)).actions;

    assert_eq!(
        model_window(&resumed_actions),
        model_window(&uninterrupted_actions),
        "resume must reconstruct the next model's complete window recipe"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn replay_folds_authoritative_deltas_and_applies_compaction_boundary() {
    let records = vec![
        header(),
        record(
            1,
            RecordKind::AcceptedOp,
            None,
            1,
            RecordPayload::AcceptedOp(user_submission(1, "old query")),
        ),
        record(
            2,
            RecordKind::EmittedEvent,
            None,
            1,
            RecordPayload::EmittedEvent(Event::TurnStarted {
                epoch: TurnEpoch(1),
                correlates: Some(SubmissionId(1)),
            }),
        ),
        record(
            3,
            RecordKind::EmittedEvent,
            None,
            1,
            RecordPayload::EmittedEvent(Event::ModelDelta {
                epoch: TurnEpoch(1),
                preview: false,
                record_seq: 3,
                text: String::from("old assistant"),
            }),
        ),
        record(
            4,
            RecordKind::EmittedEvent,
            None,
            1,
            RecordPayload::EmittedEvent(Event::TurnEnded {
                epoch: TurnEpoch(1),
                reason: kittens_code_protocol::event::TurnEnd::Completed,
            }),
        ),
        record(
            5,
            RecordKind::EmittedEvent,
            None,
            1,
            RecordPayload::EmittedEvent(Event::CompactionApplied {
                epoch: TurnEpoch(1),
            }),
        ),
        record(
            6,
            RecordKind::AcceptedOp,
            None,
            2,
            RecordPayload::AcceptedOp(user_submission(2, "recent query")),
        ),
        record(
            7,
            RecordKind::EmittedEvent,
            None,
            2,
            RecordPayload::EmittedEvent(Event::TurnStarted {
                epoch: TurnEpoch(2),
                correlates: Some(SubmissionId(2)),
            }),
        ),
        record(
            8,
            RecordKind::EmittedEvent,
            None,
            2,
            RecordPayload::EmittedEvent(Event::ModelDelta {
                epoch: TurnEpoch(2),
                preview: true,
                record_seq: 8,
                text: String::from("duplicate preview"),
            }),
        ),
        record(
            9,
            RecordKind::EmittedEvent,
            None,
            2,
            RecordPayload::EmittedEvent(Event::ModelDelta {
                epoch: TurnEpoch(2),
                preview: false,
                record_seq: 9,
                text: String::from("recent "),
            }),
        ),
        record(
            10,
            RecordKind::EmittedEvent,
            None,
            2,
            RecordPayload::EmittedEvent(Event::ModelDelta {
                epoch: TurnEpoch(2),
                preview: false,
                record_seq: 10,
                text: String::from("assistant"),
            }),
        ),
        record(
            11,
            RecordKind::EmittedEvent,
            None,
            2,
            RecordPayload::EmittedEvent(Event::TurnEnded {
                epoch: TurnEpoch(2),
                reason: kittens_code_protocol::event::TurnEnd::Completed,
            }),
        ),
    ];

    let mut resumed =
        Engine::resume(SessionConfig::default(), &records).expect("delta log resumes");
    let actions = resumed
        .handle(CoreInput::ClientOp(user_submission(3, "next")))
        .actions;
    let window = model_window(&actions);
    assert_eq!(window.last_user_query, "next");
    assert_eq!(window.summary, "");
    assert_eq!(
        window.verbatim_tail,
        vec![kittens_code_core::window::TailItem::Message(String::from(
            "[assistant] recent assistant"
        ))],
        "pre-compaction and preview text must not leak into the rebuilt tail"
    );
}

#[test]
fn resume_reports_exhausted_monotonic_namespaces() {
    let cases = [
        (
            record(
                u64::MAX,
                RecordKind::EffectOutcome,
                None,
                0,
                RecordPayload::EffectOutcome(vec![]),
            ),
            ResumeError::SequenceExhausted,
        ),
        (
            record(
                1,
                RecordKind::EffectOutcome,
                Some(u64::MAX),
                0,
                RecordPayload::EffectOutcome(vec![]),
            ),
            ResumeError::EffectIdExhausted,
        ),
        (
            record(
                1,
                RecordKind::AcceptedOp,
                None,
                0,
                RecordPayload::AcceptedOp(user_submission(u64::MAX, "last request")),
            ),
            ResumeError::SubmissionIdExhausted,
        ),
        (
            record(
                1,
                RecordKind::EffectOutcome,
                None,
                u64::MAX,
                RecordPayload::EffectOutcome(vec![]),
            ),
            ResumeError::TurnEpochExhausted,
        ),
    ];

    for (near_max, expected) in cases {
        assert_eq!(
            Engine::resume(SessionConfig::default(), &[header(), near_max]).err(),
            Some(expected)
        );
    }
}

#[test]
fn resume_requires_a_header_first() {
    assert!(matches!(
        Engine::resume(SessionConfig::default(), &[]),
        Err(ResumeError::MissingHeader)
    ));
    let not_header = record(
        0,
        RecordKind::EffectOutcome,
        Some(1),
        0,
        RecordPayload::EffectOutcome(vec![]),
    );
    assert!(matches!(
        Engine::resume(SessionConfig::default(), &[not_header]),
        Err(ResumeError::MissingHeader)
    ));
}
