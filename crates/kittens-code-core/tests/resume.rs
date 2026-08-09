//! S2 resume-as-replay coverage: persisted state seeds all monotonic
//! namespaces without re-emitting replayed work.

use std::collections::BTreeMap;

use kittens_code_core::engine::{
    CoreAction, CoreInput, EffectTerminal, Engine, ModelOutcome, ProposedToolCall, ResumeError,
};
use kittens_code_core::record::{LogHeader, Record, RecordKind, RecordPayload};
use kittens_code_protocol::budgets::Budgets;
use kittens_code_protocol::config::{SessionConfig, SessionConfigPatch};
use kittens_code_protocol::event::Event;
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
