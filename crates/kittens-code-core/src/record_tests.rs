extern crate std;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use kittens_code_protocol::budgets::Budgets;
use kittens_code_protocol::config::{
    CompactionThresholds, QueueBounds, SessionConfigPatch, StationarityThresholds,
};
use kittens_code_protocol::event::Event;
use kittens_code_protocol::ids::{EffectId, SessionId, SubmissionId, TurnEpoch};
use kittens_code_protocol::op::{Op, Submission};
use kittens_code_protocol::policy::{ApprovalPolicy, SandboxPolicy};
use serde::ser::{
    Error as _, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};
use serde::{Serialize, Serializer};

use super::*;

const SUPPORTED_SCHEMA_EPOCH: u32 = 3;

fn header(seq: u64) -> Record {
    Record::new(
        seq,
        RecordKind::Header,
        None,
        TurnEpoch(0),
        RecordPayload::Header(LogHeader {
            session_id: SessionId([0x11; 16]),
            parent: None,
            schema_epoch: SUPPORTED_SCHEMA_EPOCH,
            prompt_pack_version: [1, 2, 3],
            verb_grammar_version: [4, 5, 6],
            l3_dialect_version: [7, 8, 9],
            codec: String::from("jsonl"),
            created_at: Some(String::from("opaque-time")),
        }),
    )
    .expect("header kind and payload agree")
}

fn lifecycle(seq: u64, kind: RecordKind, txn: Option<u64>, epoch: u64) -> Record {
    let payload = match kind {
        RecordKind::StreamStarted => RecordPayload::StreamStarted(vec![1]),
        RecordKind::StreamProgress => RecordPayload::StreamProgress(vec![2]),
        RecordKind::StreamTerminal => RecordPayload::StreamTerminal(vec![3]),
        RecordKind::RepairTerminal => RecordPayload::RepairTerminal {
            cause: RepairTerminalCause::AbortedByCrash,
        },
        _ => panic!("test helper accepts only lifecycle kinds"),
    };
    Record::new(seq, kind, txn.map(EffectId), TurnEpoch(epoch), payload)
        .expect("lifecycle kind and payload agree")
}

fn scan<const N: usize>(outcomes: [DecodeOutcome; N]) -> Result<ScanResult, ScanError> {
    scan_records(Vec::from(outcomes), SUPPORTED_SCHEMA_EPOCH)
}

#[test]
fn every_record_payload_round_trips_with_stable_canonical_checksum() {
    let mut prompts = BTreeMap::new();
    prompts.insert(String::from("system"), String::from("override"));
    let mut approvals = BTreeMap::new();
    approvals.insert(String::from("exec"), ApprovalPolicy::Ask);
    let mut compaction = CompactionThresholds::default();
    compaction.prefire_percent = 60;
    compaction.hard_percent = 80;
    let mut stationarity = StationarityThresholds::default();
    stationarity.identical_calls = 8;
    stationarity.identical_noops = 2;
    let mut queues = QueueBounds::default();
    queues.model_deltas = 1;
    queues.effect_progress = 2;
    queues.interjections = 3;
    queues.max_active_effects = 4;
    let mut patch = SessionConfigPatch::default();
    patch.compaction = Some(compaction);
    patch.stationarity = Some(stationarity);
    patch.budgets = Some(Budgets::default());
    patch.queues = Some(queues);
    patch.prompt_overrides = Some(prompts);
    patch.model_root = Some(String::from("root"));
    patch.model_sub = Some(String::from("sub"));
    patch.approval_defaults = Some(approvals);
    patch.sandbox_default = Some(SandboxPolicy::WorkspaceWrite {
        writable_roots: vec![String::from("workspace")],
    });

    let payloads = vec![
        RecordPayload::Header(LogHeader {
            session_id: SessionId([0x22; 16]),
            parent: Some(SessionId([0x33; 16])),
            schema_epoch: SUPPORTED_SCHEMA_EPOCH,
            prompt_pack_version: [1, 2, 3],
            verb_grammar_version: [4, 5, 6],
            l3_dialect_version: [7, 8, 9],
            codec: String::from("postcard"),
            created_at: None,
        }),
        RecordPayload::AcceptedOp(Submission {
            id: SubmissionId(9),
            op: Op::UserInput {
                text: String::from("hello"),
            },
        }),
        RecordPayload::EmittedEvent(Event::QueryTrace {
            query: EffectId(10),
            line: 2,
            note: String::from("trace"),
        }),
        RecordPayload::EffectOutcome(vec![0, 1, 2]),
        RecordPayload::ConfigPatch(patch),
        RecordPayload::StreamStarted(vec![3]),
        RecordPayload::StreamProgress(vec![4]),
        RecordPayload::StreamTerminal(vec![5]),
        RecordPayload::RewindMarker {
            retain_through_seq: 4,
        },
        RecordPayload::RepairTerminal {
            cause: RepairTerminalCause::AbortedByCrash,
        },
    ];

    for (index, payload) in payloads.into_iter().enumerate() {
        let kind = payload.kind();
        let txn = matches!(
            kind,
            RecordKind::StreamStarted
                | RecordKind::StreamProgress
                | RecordKind::StreamTerminal
                | RecordKind::RepairTerminal
        )
        .then_some(EffectId(77));
        let record = Record::new(index as u64, kind, txn, TurnEpoch(5), payload)
            .expect("every matching payload constructs");
        assert!(record.is_valid());

        let encoded = serde_json::to_vec(&record).expect("record serializes");
        let decoded: Record = serde_json::from_slice(&encoded).expect("record deserializes");
        assert_eq!(decoded, record);
        assert_eq!(decoded.computed_checksum(), record.checksum);
    }
}

#[test]
fn validity_rejects_kind_and_checksum_corruption() {
    let original = lifecycle(1, RecordKind::StreamProgress, Some(7), 2);
    assert!(original.is_valid());

    let mut wrong_kind = original.clone();
    wrong_kind.kind = RecordKind::StreamTerminal;
    wrong_kind.checksum = wrong_kind.computed_checksum();
    assert!(!wrong_kind.is_valid());

    let mut wrong_checksum = original;
    wrong_checksum.checksum = Checksum(0);
    assert!(!wrong_checksum.is_valid());
}

#[test]
fn displays_describe_build_and_scan_errors() {
    let build = RecordBuildError::KindPayloadMismatch {
        kind: RecordKind::StreamStarted,
        payload_kind: RecordKind::StreamTerminal,
    };
    assert_eq!(
        format!("{build}"),
        "record kind StreamStarted does not match payload kind StreamTerminal"
    );
    assert!(std::error::Error::source(&build).is_none());

    let scan = ScanError::SequenceNotIncreasing {
        previous: 8,
        next: 7,
    };
    assert_eq!(
        format!("{scan}"),
        "transcript scan refused: SequenceNotIncreasing { previous: 8, next: 7 }"
    );
    assert!(std::error::Error::source(&scan).is_none());
}

#[test]
fn scanner_rejects_unusable_headers_and_kind_payload_mismatch() {
    assert_eq!(
        scan([DecodeOutcome::Tail(TailFault::Torn)]),
        Err(ScanError::HeaderUnavailable {
            fault: TailFault::Torn,
        })
    );

    let first = lifecycle(0, RecordKind::StreamStarted, Some(1), 0);
    assert_eq!(
        scan([DecodeOutcome::Good(first)]),
        Err(ScanError::MissingHeader)
    );

    let mut corrupt_header = header(0);
    corrupt_header.checksum = Checksum(0);
    assert_eq!(
        scan([DecodeOutcome::Good(corrupt_header)]),
        Err(ScanError::HeaderUnavailable {
            fault: TailFault::ChecksumMismatch,
        })
    );

    let mut mismatch = Record::new(
        1,
        RecordKind::EffectOutcome,
        None,
        TurnEpoch(1),
        RecordPayload::EffectOutcome(vec![1]),
    )
    .expect("initial record agrees");
    mismatch.kind = RecordKind::AcceptedOp;
    mismatch.checksum = mismatch.computed_checksum();
    assert_eq!(
        scan([
            DecodeOutcome::Good(header(0)),
            DecodeOutcome::Good(mismatch),
        ]),
        Err(ScanError::KindPayloadMismatch {
            seq: 1,
            kind: RecordKind::AcceptedOp,
            payload_kind: RecordKind::EffectOutcome,
        })
    );

    let not_last_checksum = scan([
        DecodeOutcome::Good(header(0)),
        DecodeOutcome::Tail(TailFault::ChecksumMismatch),
        DecodeOutcome::Good(lifecycle(1, RecordKind::StreamStarted, Some(2), 0)),
    ]);
    assert_eq!(
        not_last_checksum,
        Err(ScanError::TailFaultNotLast {
            fault: TailFault::ChecksumMismatch,
        })
    );
}

#[test]
fn scanner_rejects_duplicate_terminals_and_terminal_epoch_drift() {
    let duplicate = scan([
        DecodeOutcome::Good(header(0)),
        DecodeOutcome::Good(lifecycle(1, RecordKind::StreamStarted, Some(5), 2)),
        DecodeOutcome::Good(lifecycle(2, RecordKind::StreamTerminal, Some(5), 2)),
        DecodeOutcome::Good(lifecycle(3, RecordKind::RepairTerminal, Some(5), 2)),
    ]);
    assert_eq!(
        duplicate,
        Err(ScanError::DuplicateTerminal {
            txn: EffectId(5),
            seq: 3,
        })
    );

    let drift = scan([
        DecodeOutcome::Good(header(0)),
        DecodeOutcome::Good(lifecycle(1, RecordKind::StreamStarted, Some(6), 2)),
        DecodeOutcome::Good(lifecycle(2, RecordKind::StreamTerminal, Some(6), 3)),
    ]);
    assert_eq!(
        drift,
        Err(ScanError::TransactionEpochMismatch {
            txn: EffectId(6),
            started: TurnEpoch(2),
            found: TurnEpoch(3),
            seq: 2,
        })
    );
}

#[test]
fn scanner_accepts_header_only_and_final_tail_without_repairs() {
    let rewind = Record::new(
        1,
        RecordKind::RewindMarker,
        None,
        TurnEpoch(0),
        RecordPayload::RewindMarker {
            retain_through_seq: 0,
        },
    )
    .expect("rewind kind and payload agree");
    let clean = scan([DecodeOutcome::Good(header(0)), DecodeOutcome::Good(rewind)])
        .expect("a complete non-lifecycle record is clean");
    assert!(clean.repairs.is_empty());
    assert_eq!(clean.replayable.len(), 2);
    assert_eq!(clean.ignored_tail, None);

    let tailed = scan([
        DecodeOutcome::Good(header(0)),
        DecodeOutcome::Tail(TailFault::ChecksumMismatch),
    ])
    .expect("a final checksum tail is tolerated");
    assert!(tailed.repairs.is_empty());
    assert_eq!(tailed.ignored_tail, Some(TailFault::ChecksumMismatch));
}

struct AlwaysFails;

impl Serialize for AlwaysFails {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Err(S::Error::custom("forced failure"))
    }
}

#[test]
fn canonical_serializer_exercises_every_serde_shape_and_error_path() {
    let mut serializer = CanonicalSerializer::new();
    Serializer::serialize_bool(&mut serializer, false).expect("bool");
    Serializer::serialize_bool(&mut serializer, true).expect("bool");
    Serializer::serialize_i8(&mut serializer, -1).expect("i8");
    Serializer::serialize_i16(&mut serializer, -2).expect("i16");
    Serializer::serialize_i32(&mut serializer, -3).expect("i32");
    Serializer::serialize_i64(&mut serializer, -4).expect("i64");
    Serializer::serialize_i128(&mut serializer, -5).expect("i128");
    Serializer::serialize_u8(&mut serializer, 1).expect("u8");
    Serializer::serialize_u16(&mut serializer, 2).expect("u16");
    Serializer::serialize_u32(&mut serializer, 3).expect("u32");
    Serializer::serialize_u64(&mut serializer, 4).expect("u64");
    Serializer::serialize_u128(&mut serializer, 5).expect("u128");
    Serializer::serialize_f32(&mut serializer, 1.25).expect("f32");
    Serializer::serialize_f64(&mut serializer, 2.5).expect("f64");
    Serializer::serialize_char(&mut serializer, 'λ').expect("char");
    Serializer::serialize_str(&mut serializer, "text").expect("str");
    Serializer::serialize_bytes(&mut serializer, &[1, 2]).expect("bytes");
    Serializer::serialize_none(&mut serializer).expect("none");
    Serializer::serialize_some(&mut serializer, &7_u8).expect("some");
    Serializer::serialize_unit(&mut serializer).expect("unit");
    Serializer::serialize_unit_struct(&mut serializer, "Unit").expect("unit struct");
    Serializer::serialize_unit_variant(&mut serializer, "Enum", 1, "Unit").expect("unit variant");
    Serializer::serialize_newtype_struct(&mut serializer, "Newtype", &8_u8)
        .expect("newtype struct");
    Serializer::serialize_newtype_variant(&mut serializer, "Enum", 2, "Newtype", &9_u8)
        .expect("newtype variant");

    let mut seq = Serializer::serialize_seq(&mut serializer, None).expect("seq");
    SerializeSeq::serialize_element(&mut seq, &10_u8).expect("seq element");
    SerializeSeq::end(seq).expect("seq end");

    let mut tuple = Serializer::serialize_tuple(&mut serializer, 1).expect("tuple");
    SerializeTuple::serialize_element(&mut tuple, &11_u8).expect("tuple element");
    SerializeTuple::end(tuple).expect("tuple end");

    let mut tuple_struct =
        Serializer::serialize_tuple_struct(&mut serializer, "Tuple", 1).expect("tuple struct");
    SerializeTupleStruct::serialize_field(&mut tuple_struct, &12_u8).expect("tuple struct field");
    SerializeTupleStruct::end(tuple_struct).expect("tuple struct end");

    let mut tuple_variant =
        Serializer::serialize_tuple_variant(&mut serializer, "Enum", 3, "Tuple", 1)
            .expect("tuple variant");
    SerializeTupleVariant::serialize_field(&mut tuple_variant, &13_u8)
        .expect("tuple variant field");
    SerializeTupleVariant::end(tuple_variant).expect("tuple variant end");

    let mut map = Serializer::serialize_map(&mut serializer, Some(1)).expect("map");
    SerializeMap::serialize_key(&mut map, "key").expect("map key");
    SerializeMap::serialize_value(&mut map, &14_u8).expect("map value");
    SerializeMap::end(map).expect("map end");

    let mut strukt = Serializer::serialize_struct(&mut serializer, "Struct", 1).expect("struct");
    SerializeStruct::serialize_field(&mut strukt, "field", &15_u8).expect("struct field");
    SerializeStruct::end(strukt).expect("struct end");

    let mut struct_variant =
        Serializer::serialize_struct_variant(&mut serializer, "Enum", 4, "Struct", 1)
            .expect("struct variant");
    SerializeStructVariant::serialize_field(&mut struct_variant, "field", &16_u8)
        .expect("struct variant field");
    SerializeStructVariant::end(struct_variant).expect("struct variant end");

    assert_ne!(serializer.finish(), 0);
    let error = CanonicalError;
    assert_eq!(format!("{error}"), "canonical serialization failed");
    let custom = <CanonicalError as serde::ser::Error>::custom("ignored");
    assert!(std::error::Error::source(&custom).is_none());

    let panic = std::panic::catch_unwind(|| {
        let mut serializer = CanonicalSerializer::new();
        serialize_canonical(&mut serializer, &AlwaysFails);
    });
    assert!(panic.is_err());

    // `usize` cannot exceed `u64` on any supported Rust target, so the
    // defensive `u64::MAX` fallback in `write_len` is unreachable.
    const { assert!(usize::BITS <= u64::BITS) };
}
