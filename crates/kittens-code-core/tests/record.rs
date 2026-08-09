//! G2-class transcript replay and crash-repair evidence.

use std::string::String;
use std::vec;

use kittens_code_core::record::{
    Checksum, DecodeOutcome, LogHeader, Record, RecordBuildError, RecordKind, RecordPayload,
    RepairTerminalCause, ScanError, TailFault, scan_records,
};
use kittens_code_protocol::ids::{EffectId, SessionId, TurnEpoch};

const SUPPORTED_SCHEMA_EPOCH: u32 = 3;

fn record(
    seq: u64,
    kind: RecordKind,
    txn: Option<u64>,
    epoch: u64,
    payload: RecordPayload,
) -> Record {
    Record::new(seq, kind, txn.map(EffectId), TurnEpoch(epoch), payload)
        .expect("test records use matching kinds and payloads")
}

fn header(seq: u64, schema_epoch: u32) -> Record {
    record(
        seq,
        RecordKind::Header,
        None,
        0,
        RecordPayload::Header(LogHeader {
            session_id: SessionId([0x11; 16]),
            parent: None,
            schema_epoch,
            prompt_pack_version: [1, 0, 0],
            verb_grammar_version: [1, 0, 0],
            l3_dialect_version: [1, 0, 0],
            codec: String::from("jsonl"),
            created_at: Some(String::from("driver-time:42")),
        }),
    )
}

fn started(seq: u64, txn: u64, epoch: u64) -> Record {
    record(
        seq,
        RecordKind::StreamStarted,
        Some(txn),
        epoch,
        RecordPayload::StreamStarted(vec![]),
    )
}

fn progress(seq: u64, txn: u64, epoch: u64) -> Record {
    record(
        seq,
        RecordKind::StreamProgress,
        Some(txn),
        epoch,
        RecordPayload::StreamProgress(vec![1, 2, 3]),
    )
}

fn terminal(seq: u64, txn: u64, epoch: u64) -> Record {
    record(
        seq,
        RecordKind::StreamTerminal,
        Some(txn),
        epoch,
        RecordPayload::StreamTerminal(vec![9]),
    )
}

#[test]
fn crc_32_iso_hdlc_matches_the_standard_check_vector() {
    assert_eq!(Checksum::of_bytes(b"123456789"), Checksum(0xcbf4_3926));
}

#[test]
fn checksum_covers_seq_through_payload_and_excludes_the_checksum_field() {
    let original = progress(2, 12, 5);
    let expected = original.computed_checksum();
    assert_eq!(expected, Checksum(0xc0c2_75e4));

    let mut changed = original.clone();
    changed.seq = 3;
    assert_ne!(changed.computed_checksum(), expected);

    let mut changed = original.clone();
    changed.kind = RecordKind::StreamTerminal;
    assert_ne!(changed.computed_checksum(), expected);

    let mut changed = original.clone();
    changed.txn = Some(EffectId(13));
    assert_ne!(changed.computed_checksum(), expected);

    let mut changed = original.clone();
    changed.epoch = TurnEpoch(6);
    assert_ne!(changed.computed_checksum(), expected);

    let mut changed = original.clone();
    changed.payload = RecordPayload::StreamProgress(vec![1, 2, 4]);
    assert_ne!(changed.computed_checksum(), expected);

    let mut changed = original;
    changed.checksum = Checksum(0);
    assert_eq!(changed.computed_checksum(), expected);
}

#[test]
fn higher_header_epoch_is_refused_before_checksum_validation() {
    let mut newer = header(0, SUPPORTED_SCHEMA_EPOCH + 1);
    newer.checksum = Checksum(0);

    let error = scan_records([DecodeOutcome::Good(newer)], SUPPORTED_SCHEMA_EPOCH)
        .expect_err("a newer schema must be refused");

    assert_eq!(
        error,
        ScanError::SchemaIncompatible {
            found: SUPPORTED_SCHEMA_EPOCH + 1,
            supported: SUPPORTED_SCHEMA_EPOCH,
        }
    );
}

#[test]
fn torn_tail_is_ignored_after_the_valid_prefix() {
    let result = scan_records(
        [
            DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
            DecodeOutcome::Good(started(1, 7, 2)),
            DecodeOutcome::Good(progress(2, 7, 2)),
            DecodeOutcome::Tail(TailFault::Torn),
        ],
        SUPPORTED_SCHEMA_EPOCH,
    )
    .expect("a tail fault after a valid header is recoverable");

    assert_eq!(result.ignored_tail, Some(TailFault::Torn));
    assert_eq!(result.repairs.len(), 1);
    assert_eq!(result.replayable.len(), 4);
    assert_eq!(result.replayable[0].seq, 0);
    assert_eq!(result.replayable[2].seq, 2);
    assert_eq!(result.replayable[3], result.repairs[0]);
}

#[test]
fn incomplete_stream_produces_a_checksummed_crash_terminal() {
    let result = scan_records(
        [
            DecodeOutcome::Good(header(10, SUPPORTED_SCHEMA_EPOCH)),
            DecodeOutcome::Good(started(14, 99, 6)),
        ],
        SUPPORTED_SCHEMA_EPOCH,
    )
    .expect("an incomplete stream is repairable");

    assert_eq!(result.repairs.len(), 1);
    let repair = &result.repairs[0];
    assert_eq!(repair.seq, 15);
    assert_eq!(repair.kind, RecordKind::RepairTerminal);
    assert_eq!(repair.txn, Some(EffectId(99)));
    assert_eq!(repair.epoch, TurnEpoch(6));
    assert_eq!(
        repair.payload,
        RecordPayload::RepairTerminal {
            cause: RepairTerminalCause::AbortedByCrash,
        }
    );
    assert!(repair.is_valid());
    assert_eq!(result.replayable.last(), Some(repair));
}

#[test]
fn repair_order_follows_started_encounter_order_not_effect_id() {
    let result = scan_records(
        [
            DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
            DecodeOutcome::Good(started(1, 90, 4)),
            DecodeOutcome::Good(started(2, 2, 5)),
        ],
        SUPPORTED_SCHEMA_EPOCH,
    )
    .expect("both incomplete transactions are repairable");

    assert_eq!(result.repairs.len(), 2);
    assert_eq!(result.repairs[0].txn, Some(EffectId(90)));
    assert_eq!(result.repairs[0].seq, 3);
    assert_eq!(result.repairs[1].txn, Some(EffectId(2)));
    assert_eq!(result.repairs[1].seq, 4);
}

#[test]
fn ordinary_terminal_closes_a_started_transaction_without_repair() {
    let result = scan_records(
        [
            DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
            DecodeOutcome::Good(started(1, 7, 8)),
            DecodeOutcome::Good(progress(2, 7, 8)),
            DecodeOutcome::Good(terminal(3, 7, 8)),
        ],
        SUPPORTED_SCHEMA_EPOCH,
    )
    .expect("a complete stream is replayable");

    assert!(result.repairs.is_empty());
    assert_eq!(result.replayable.len(), 4);
}

#[test]
fn terminal_without_started_is_rejected() {
    let error = scan_records(
        [
            DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
            DecodeOutcome::Good(terminal(1, 41, 3)),
        ],
        SUPPORTED_SCHEMA_EPOCH,
    )
    .expect_err("a terminal may not appear without a start");

    assert_eq!(
        error,
        ScanError::TerminalWithoutStarted {
            txn: EffectId(41),
            seq: 1,
        }
    );
}

#[test]
fn checksum_tail_marker_ends_the_valid_prefix_and_repairs_open_transactions() {
    let result = scan_records(
        [
            DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
            DecodeOutcome::Good(started(1, 12, 5)),
            DecodeOutcome::Tail(TailFault::ChecksumMismatch),
        ],
        SUPPORTED_SCHEMA_EPOCH,
    )
    .expect("checksum corruption is treated as a tail boundary");

    assert_eq!(result.ignored_tail, Some(TailFault::ChecksumMismatch));
    assert_eq!(result.repairs.len(), 1);
    assert_eq!(result.repairs[0].seq, 2);
    assert_eq!(result.replayable.len(), 3);
}

#[test]
fn checksum_invalid_good_record_is_a_decoder_contract_error() {
    let mut corrupt = progress(2, 12, 5);
    corrupt.kind = RecordKind::StreamTerminal;

    let error = scan_records(
        [
            DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
            DecodeOutcome::Good(started(1, 12, 5)),
            DecodeOutcome::Good(corrupt),
        ],
        SUPPORTED_SCHEMA_EPOCH,
    )
    .expect_err("a Good outcome must have a valid checksum");

    assert_eq!(error, ScanError::ChecksumMismatch { seq: 2 });
}

#[test]
fn tail_marker_must_be_the_last_decode_outcome() {
    let error = scan_records(
        [
            DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
            DecodeOutcome::Tail(TailFault::Torn),
            DecodeOutcome::Good(started(1, 12, 5)),
        ],
        SUPPORTED_SCHEMA_EPOCH,
    )
    .expect_err("a tail marker cannot occur in the middle of decoded outcomes");

    assert_eq!(
        error,
        ScanError::TailFaultNotLast {
            fault: TailFault::Torn,
        }
    );
}

#[test]
fn scanner_rejects_structural_and_lifecycle_failures() {
    assert_eq!(
        scan_records([], SUPPORTED_SCHEMA_EPOCH),
        Err(ScanError::MissingHeader)
    );

    assert_eq!(
        scan_records(
            [
                DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
                DecodeOutcome::Good(header(1, SUPPORTED_SCHEMA_EPOCH)),
            ],
            SUPPORTED_SCHEMA_EPOCH,
        ),
        Err(ScanError::DuplicateHeader { seq: 1 })
    );

    assert_eq!(
        scan_records(
            [
                DecodeOutcome::Good(header(1, SUPPORTED_SCHEMA_EPOCH)),
                DecodeOutcome::Good(started(1, 7, 2)),
            ],
            SUPPORTED_SCHEMA_EPOCH,
        ),
        Err(ScanError::SequenceNotIncreasing {
            previous: 1,
            next: 1,
        })
    );

    let missing_txn = record(
        1,
        RecordKind::StreamStarted,
        None,
        2,
        RecordPayload::StreamStarted(vec![]),
    );
    assert_eq!(
        scan_records(
            [
                DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
                DecodeOutcome::Good(missing_txn),
            ],
            SUPPORTED_SCHEMA_EPOCH,
        ),
        Err(ScanError::MissingTransaction {
            seq: 1,
            kind: RecordKind::StreamStarted,
        })
    );

    assert_eq!(
        scan_records(
            [
                DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
                DecodeOutcome::Good(progress(1, 7, 2)),
            ],
            SUPPORTED_SCHEMA_EPOCH,
        ),
        Err(ScanError::ProgressWithoutStarted {
            txn: EffectId(7),
            seq: 1,
        })
    );

    assert_eq!(
        scan_records(
            [
                DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
                DecodeOutcome::Good(started(1, 7, 2)),
                DecodeOutcome::Good(started(2, 7, 2)),
            ],
            SUPPORTED_SCHEMA_EPOCH,
        ),
        Err(ScanError::DuplicateStarted {
            txn: EffectId(7),
            seq: 2,
        })
    );

    assert_eq!(
        scan_records(
            [
                DecodeOutcome::Good(header(0, SUPPORTED_SCHEMA_EPOCH)),
                DecodeOutcome::Good(started(1, 7, 2)),
                DecodeOutcome::Good(progress(2, 7, 3)),
            ],
            SUPPORTED_SCHEMA_EPOCH,
        ),
        Err(ScanError::TransactionEpochMismatch {
            txn: EffectId(7),
            started: TurnEpoch(2),
            found: TurnEpoch(3),
            seq: 2,
        })
    );

    assert_eq!(
        scan_records(
            [
                DecodeOutcome::Good(header(u64::MAX - 1, SUPPORTED_SCHEMA_EPOCH)),
                DecodeOutcome::Good(started(u64::MAX, 7, 2)),
            ],
            SUPPORTED_SCHEMA_EPOCH,
        ),
        Err(ScanError::SequenceExhausted)
    );
}

#[test]
fn constructor_rejects_a_kind_payload_mismatch() {
    let error = Record::new(
        1,
        RecordKind::StreamProgress,
        Some(EffectId(5)),
        TurnEpoch(1),
        RecordPayload::StreamTerminal(vec![]),
    )
    .expect_err("kind and payload must agree");

    assert_eq!(
        error,
        RecordBuildError::KindPayloadMismatch {
            kind: RecordKind::StreamProgress,
            payload_kind: RecordKind::StreamTerminal,
        }
    );
}
