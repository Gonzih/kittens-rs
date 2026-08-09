//! Codec-independent transcript records and pure startup scanning.
//!
//! The outer store codec is deliberately absent. A decoder validates its
//! framing and then supplies [`crate::record::DecodeOutcome`] values;
//! [`crate::record::scan_records`] checks the semantic record invariants and
//! prepares the durable crash-repair suffix.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use kittens_code_protocol::config::SessionConfigPatch;
use kittens_code_protocol::event::Event;
use kittens_code_protocol::ids::{EffectId, SessionId, TurnEpoch};
use kittens_code_protocol::op::Submission;
use serde::ser::{
    SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};
use serde::{Deserialize, Serialize, Serializer};

/// The checksum algorithm used for KC0 records.
///
/// This is CRC-32/ISO-HDLC (also called IEEE CRC-32): polynomial
/// `0x04c11db7` in normal form (`0xedb88320` reflected), initial value and
/// final XOR `0xffff_ffff`, with reflected input and output.
pub const CHECKSUM_ALGORITHM: &str = "CRC-32/ISO-HDLC";

/// A CRC-32/ISO-HDLC checksum stored on one record.
///
/// This detects accidental corruption; it is not a cryptographic
/// authentication mechanism.
///
/// The covered logical byte sequence is, in order:
///
/// 1. `seq` as eight little-endian bytes;
/// 2. the stable one-byte [`RecordKind`] tag;
/// 3. one byte for `txn` (`0` for absent, `1` for present), followed by the
///    effect id as eight little-endian bytes when present;
/// 4. `epoch` as eight little-endian bytes; and
/// 5. the complete canonical payload representation.
///
/// Header scalars use little-endian integers, option tags, and `u64`
/// little-endian byte lengths before UTF-8 strings. Opaque payloads likewise
/// use a `u64` byte length followed by every payload byte. Rewind targets are
/// `u64` little-endian values and the crash-repair cause is the byte `0`.
/// Published protocol values use the deterministic serde-token encoding in
/// this module: each scalar has a distinct one-byte type tag; strings and byte
/// sequences carry a `u64` length; options carry a variant tag; enum variants
/// carry their `u32` declaration index; compound values carry their declared
/// length (or an absent-length tag) and an end marker. Struct fields are
/// visited in declaration order and maps in their serializer-provided order
/// (the protocol uses ordered maps). The outer codec's punctuation, framing,
/// checksum field, and line terminator are not covered.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Checksum(pub u32);

impl Checksum {
    /// Computes CRC-32/ISO-HDLC over an arbitrary byte slice.
    ///
    /// Record producers should normally use [`Record::new`], which applies
    /// the complete record coverage canon rather than hashing only a payload.
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> Self {
        let mut state = Crc32::new();
        state.write(bytes);
        Self(state.finish())
    }
}

/// The typed payload class carried by a transcript record.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordKind {
    /// The log header.
    Header,
    /// A client operation accepted by the core.
    AcceptedOp,
    /// An emitted wire event.
    EmittedEvent,
    /// A driver effect outcome not otherwise represented by a wire event.
    EffectOutcome,
    /// An accepted, replayable session-configuration patch.
    ConfigPatch,
    /// The first record of a streamed transaction.
    StreamStarted,
    /// One progress record in a streamed transaction.
    StreamProgress,
    /// The sole ordinary terminal of a streamed transaction.
    StreamTerminal,
    /// An append-only rewind/elision marker.
    RewindMarker,
    /// A persisted terminal produced by startup crash repair.
    RepairTerminal,
}

impl RecordKind {
    const fn checksum_tag(self) -> u8 {
        match self {
            Self::Header => 0,
            Self::AcceptedOp => 1,
            Self::EmittedEvent => 2,
            Self::EffectOutcome => 3,
            Self::ConfigPatch => 4,
            Self::StreamStarted => 5,
            Self::StreamProgress => 6,
            Self::StreamTerminal => 7,
            Self::RewindMarker => 8,
            Self::RepairTerminal => 9,
        }
    }
}

/// The mandatory first record's payload.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LogHeader {
    /// Identity of the session stored in this log.
    pub session_id: SessionId,
    /// Parent session for a future forked log, when one exists.
    pub parent: Option<SessionId>,
    /// Persisted-schema compatibility epoch.
    pub schema_epoch: u32,
    /// Prompt-pack semantic version as `major.minor.patch` integers.
    pub prompt_pack_version: [u16; 3],
    /// Text-verb grammar version as `major.minor.patch` integers.
    pub verb_grammar_version: [u16; 3],
    /// L3-search dialect version as `major.minor.patch` integers.
    pub l3_dialect_version: [u16; 3],
    /// Driver-declared outer record codec identifier.
    pub codec: String,
    /// Optional driver-claimed creation time, retained as an opaque string.
    pub created_at: Option<String>,
}

/// The only KC0 startup-repair terminal cause.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RepairTerminalCause {
    /// The process stopped after `Started` but before an ordinary terminal.
    AbortedByCrash,
}

/// One typed record payload.
///
/// Published wire/configuration shapes remain typed. The byte vectors are
/// deliberately opaque because the protocol crate does not publish general
/// effect, progress, or terminal payload types in KC0.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "payload_kind", content = "value")]
pub enum RecordPayload {
    /// The log header shape from S6.
    Header(LogHeader),
    /// An accepted client submission.
    AcceptedOp(Submission),
    /// An emitted protocol event.
    EmittedEvent(Event),
    /// Opaque effect-outcome data owned by the core/driver contract.
    EffectOutcome(Vec<u8>),
    /// An accepted session-configuration patch.
    ConfigPatch(SessionConfigPatch),
    /// Opaque metadata for a newly started stream.
    StreamStarted(Vec<u8>),
    /// Opaque bytes for one stream-progress item.
    StreamProgress(Vec<u8>),
    /// Opaque bytes for an ordinary stream terminal.
    StreamTerminal(Vec<u8>),
    /// Elide records strictly after the named sequence in the derived view.
    RewindMarker {
        /// Last sequence retained by the derived view.
        retain_through_seq: u64,
    },
    /// A durable terminal generated during startup repair.
    RepairTerminal {
        /// Why startup repair closed the transaction.
        cause: RepairTerminalCause,
    },
}

impl RecordPayload {
    /// Returns the record kind required for this payload variant.
    #[must_use]
    pub const fn kind(&self) -> RecordKind {
        match self {
            Self::Header(_) => RecordKind::Header,
            Self::AcceptedOp(_) => RecordKind::AcceptedOp,
            Self::EmittedEvent(_) => RecordKind::EmittedEvent,
            Self::EffectOutcome(_) => RecordKind::EffectOutcome,
            Self::ConfigPatch(_) => RecordKind::ConfigPatch,
            Self::StreamStarted(_) => RecordKind::StreamStarted,
            Self::StreamProgress(_) => RecordKind::StreamProgress,
            Self::StreamTerminal(_) => RecordKind::StreamTerminal,
            Self::RewindMarker { .. } => RecordKind::RewindMarker,
            Self::RepairTerminal { .. } => RecordKind::RepairTerminal,
        }
    }

    fn checksum_write(&self, serializer: &mut CanonicalSerializer) {
        match self {
            Self::Header(header) => write_header(serializer, header),
            Self::AcceptedOp(submission) => serialize_canonical(serializer, submission),
            Self::EmittedEvent(event) => serialize_canonical(serializer, event),
            Self::EffectOutcome(bytes)
            | Self::StreamStarted(bytes)
            | Self::StreamProgress(bytes)
            | Self::StreamTerminal(bytes) => serializer.write_sized_bytes(bytes),
            Self::ConfigPatch(patch) => serialize_canonical(serializer, patch),
            Self::RewindMarker { retain_through_seq } => {
                serializer.write_raw(&retain_through_seq.to_le_bytes());
            }
            Self::RepairTerminal {
                cause: RepairTerminalCause::AbortedByCrash,
            } => serializer.write_raw(&[0]),
        }
    }
}

/// One immutable transcript-log record.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Record {
    /// Strictly increasing log sequence number.
    pub seq: u64,
    /// Payload/lifecycle class.
    pub kind: RecordKind,
    /// Correlated streamed transaction, when applicable.
    pub txn: Option<EffectId>,
    /// Turn epoch that produced this record.
    pub epoch: TurnEpoch,
    /// Typed record payload.
    pub payload: RecordPayload,
    /// CRC over the declared `seq`-through-`payload` logical bytes.
    pub checksum: Checksum,
}

impl Record {
    /// Constructs a kind-consistent record and computes its checksum.
    ///
    /// This constructor does not impose lifecycle rules; [`scan_records`]
    /// checks transaction ordering across records.
    ///
    /// # Errors
    ///
    /// Returns [`RecordBuildError::KindPayloadMismatch`] when `kind` does not
    /// name the supplied payload variant.
    pub fn new(
        seq: u64,
        kind: RecordKind,
        txn: Option<EffectId>,
        epoch: TurnEpoch,
        payload: RecordPayload,
    ) -> Result<Self, RecordBuildError> {
        let payload_kind = payload.kind();
        if kind != payload_kind {
            return Err(RecordBuildError::KindPayloadMismatch { kind, payload_kind });
        }

        let mut record = Self {
            seq,
            kind,
            txn,
            epoch,
            payload,
            checksum: Checksum::default(),
        };
        record.checksum = record.computed_checksum();
        Ok(record)
    }

    /// Recomputes the checksum using the declared logical byte canon.
    #[must_use]
    pub fn computed_checksum(&self) -> Checksum {
        let mut serializer = CanonicalSerializer::new();
        serializer.write_raw(&self.seq.to_le_bytes());
        serializer.write_raw(&[self.kind.checksum_tag()]);
        match self.txn {
            None => serializer.write_raw(&[0]),
            Some(id) => {
                serializer.write_raw(&[1]);
                serializer.write_raw(&id.0.to_le_bytes());
            }
        }
        serializer.write_raw(&self.epoch.0.to_le_bytes());
        self.payload.checksum_write(&mut serializer);
        Checksum(serializer.finish())
    }

    /// Returns whether kind, payload, and checksum agree.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.kind == self.payload.kind() && self.checksum == self.computed_checksum()
    }
}

/// Why a record could not be constructed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordBuildError {
    /// The explicit discriminator did not agree with the payload variant.
    KindPayloadMismatch {
        /// Explicit record discriminator.
        kind: RecordKind,
        /// Discriminator required by the payload.
        payload_kind: RecordKind,
    },
}

impl fmt::Display for RecordBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KindPayloadMismatch { kind, payload_kind } => write!(
                formatter,
                "record kind {kind:?} does not match payload kind {payload_kind:?}"
            ),
        }
    }
}

impl core::error::Error for RecordBuildError {}

/// A decoder's reason for truncating the valid record prefix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TailFault {
    /// The final frame ended before a complete record was decoded.
    Torn,
    /// The final complete-looking frame failed its checksum.
    ChecksumMismatch,
}

/// One item supplied by a framing/codec decoder to the pure scanner.
#[derive(Clone, Debug, Eq, PartialEq)]
// A startup scan consumes `Good` records immediately. Keeping them inline
// avoids a heap allocation for every record on constrained targets.
#[allow(clippy::large_enum_variant)]
pub enum DecodeOutcome {
    /// A complete record accepted by the decoder, including its checksum.
    ///
    /// The scanner defensively recomputes the checksum and treats a mismatch
    /// as a decoder-contract violation rather than as a tolerable tail.
    Good(Record),
    /// The end of the valid prefix; this marker and the suffix are ignored.
    Tail(TailFault),
}

/// The startup scan's append and replay plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanResult {
    /// Exact records that the sole appender must persist before replay.
    pub repairs: Vec<Record>,
    /// Valid decoded prefix followed by the repair suffix.
    ///
    /// A driver must not replay this sequence until every record in
    /// [`Self::repairs`] has received its durability acknowledgement.
    pub replayable: Vec<Record>,
    /// Tail failure that ended the valid prefix, if one was observed.
    pub ignored_tail: Option<TailFault>,
}

/// A semantic refusal from startup scanning.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScanError {
    /// No usable first header record exists.
    MissingHeader,
    /// The header could not be trusted because its frame was damaged.
    HeaderUnavailable {
        /// Damage reported by the decoder or checksum recheck.
        fault: TailFault,
    },
    /// This binary is older than the persisted schema.
    SchemaIncompatible {
        /// Epoch found in the header.
        found: u32,
        /// Highest epoch this scanner understands.
        supported: u32,
    },
    /// A second header appeared after the mandatory first record.
    DuplicateHeader {
        /// Sequence carrying the duplicate.
        seq: u64,
    },
    /// A decoded record's explicit kind disagreed with its payload.
    KindPayloadMismatch {
        /// Sequence carrying the mismatch.
        seq: u64,
        /// Explicit record discriminator.
        kind: RecordKind,
        /// Discriminator required by the payload.
        payload_kind: RecordKind,
    },
    /// A `Good` decoder outcome did not survive defensive checksum validation.
    ChecksumMismatch {
        /// Sequence carried by the invalid record.
        seq: u64,
    },
    /// A tail marker was followed by another decoder outcome.
    TailFaultNotLast {
        /// Fault incorrectly reported before the iterator ended.
        fault: TailFault,
    },
    /// Sequence numbers failed to increase strictly.
    SequenceNotIncreasing {
        /// Last accepted sequence.
        previous: u64,
        /// Invalid following sequence.
        next: u64,
    },
    /// A lifecycle record omitted its transaction id.
    MissingTransaction {
        /// Sequence carrying the invalid lifecycle record.
        seq: u64,
        /// Lifecycle kind requiring a transaction.
        kind: RecordKind,
    },
    /// The same transaction was started more than once.
    DuplicateStarted {
        /// Reused transaction id.
        txn: EffectId,
        /// Sequence carrying the duplicate start.
        seq: u64,
    },
    /// Progress appeared before a still-open start.
    ProgressWithoutStarted {
        /// Transaction named by the progress record.
        txn: EffectId,
        /// Sequence carrying the progress record.
        seq: u64,
    },
    /// A terminal appeared without any earlier start.
    TerminalWithoutStarted {
        /// Transaction named by the terminal.
        txn: EffectId,
        /// Sequence carrying the terminal.
        seq: u64,
    },
    /// A transaction received more than one terminal.
    DuplicateTerminal {
        /// Already closed transaction.
        txn: EffectId,
        /// Sequence carrying the duplicate terminal.
        seq: u64,
    },
    /// Lifecycle records for one transaction disagreed on turn epoch.
    TransactionEpochMismatch {
        /// Transaction whose epochs differed.
        txn: EffectId,
        /// Epoch on its start record.
        started: TurnEpoch,
        /// Epoch on the later lifecycle record.
        found: TurnEpoch,
        /// Sequence carrying the later epoch.
        seq: u64,
    },
    /// No sequence number remains for a required repair terminal.
    SequenceExhausted,
}

impl fmt::Display for ScanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "transcript scan refused: {self:?}")
    }
}

impl core::error::Error for ScanError {}

/// Scans a decoded prefix, prepares crash terminals, and returns replay data.
///
/// The first outcome must be a header. Its `schema_epoch` comparison occurs
/// before checksum revalidation or any lifecycle processing, so an older
/// binary never prepares repairs for a newer schema. A final tail marker after
/// the header ends the accepted prefix without error; a marker followed by
/// another outcome is a structural refusal. Incomplete transactions are
/// repaired in the encounter order of their `StreamStarted` records, with
/// contiguous sequence numbers after the last accepted record and the start
/// record's epoch and transaction id.
///
/// # Errors
///
/// Returns [`ScanError`] when the header is absent or incompatible, record
/// ordering is invalid, a kind/payload pair disagrees, a transaction lifecycle
/// is malformed, or a needed repair sequence would overflow `u64`.
pub fn scan_records<I>(outcomes: I, supported_schema_epoch: u32) -> Result<ScanResult, ScanError>
where
    I: IntoIterator<Item = DecodeOutcome>,
{
    let mut outcomes = outcomes.into_iter();
    let first = match outcomes.next() {
        Some(DecodeOutcome::Good(record)) => record,
        Some(DecodeOutcome::Tail(fault)) => {
            return Err(ScanError::HeaderUnavailable { fault });
        }
        None => return Err(ScanError::MissingHeader),
    };

    let schema_epoch = match (&first.kind, &first.payload) {
        (RecordKind::Header, RecordPayload::Header(header)) => header.schema_epoch,
        _ => return Err(ScanError::MissingHeader),
    };
    if schema_epoch > supported_schema_epoch {
        return Err(ScanError::SchemaIncompatible {
            found: schema_epoch,
            supported: supported_schema_epoch,
        });
    }
    if first.checksum != first.computed_checksum() {
        return Err(ScanError::HeaderUnavailable {
            fault: TailFault::ChecksumMismatch,
        });
    }

    let mut last_seq = first.seq;
    let mut replayable = Vec::new();
    replayable.push(first);
    let mut ignored_tail = None;
    let mut open = BTreeMap::<EffectId, TurnEpoch>::new();
    let mut started = BTreeSet::<EffectId>::new();
    let mut start_order = Vec::<EffectId>::new();

    while let Some(outcome) = outcomes.next() {
        let record = match outcome {
            DecodeOutcome::Good(record) => record,
            DecodeOutcome::Tail(fault) => {
                if outcomes.next().is_some() {
                    return Err(ScanError::TailFaultNotLast { fault });
                }
                ignored_tail = Some(fault);
                break;
            }
        };

        if record.checksum != record.computed_checksum() {
            return Err(ScanError::ChecksumMismatch { seq: record.seq });
        }
        let payload_kind = record.payload.kind();
        if record.kind != payload_kind {
            return Err(ScanError::KindPayloadMismatch {
                seq: record.seq,
                kind: record.kind,
                payload_kind,
            });
        }
        if record.seq <= last_seq {
            return Err(ScanError::SequenceNotIncreasing {
                previous: last_seq,
                next: record.seq,
            });
        }
        last_seq = record.seq;

        if record.kind == RecordKind::Header {
            return Err(ScanError::DuplicateHeader { seq: record.seq });
        }
        apply_lifecycle(&record, &mut open, &mut started, &mut start_order)?;
        replayable.push(record);
    }

    let mut repairs = Vec::new();
    for txn in start_order {
        let Some(epoch) = open.remove(&txn) else {
            continue;
        };
        last_seq = last_seq
            .checked_add(1)
            .ok_or(ScanError::SequenceExhausted)?;
        let repair = Record::new(
            last_seq,
            RecordKind::RepairTerminal,
            Some(txn),
            epoch,
            RecordPayload::RepairTerminal {
                cause: RepairTerminalCause::AbortedByCrash,
            },
        )
        .map_err(|_| ScanError::KindPayloadMismatch {
            seq: last_seq,
            kind: RecordKind::RepairTerminal,
            payload_kind: RecordKind::RepairTerminal,
        })?;
        replayable.push(repair.clone());
        repairs.push(repair);
    }

    Ok(ScanResult {
        repairs,
        replayable,
        ignored_tail,
    })
}

fn apply_lifecycle(
    record: &Record,
    open: &mut BTreeMap<EffectId, TurnEpoch>,
    started: &mut BTreeSet<EffectId>,
    start_order: &mut Vec<EffectId>,
) -> Result<(), ScanError> {
    match record.kind {
        RecordKind::StreamStarted => {
            let txn = required_txn(record)?;
            if !started.insert(txn) {
                return Err(ScanError::DuplicateStarted {
                    txn,
                    seq: record.seq,
                });
            }
            open.insert(txn, record.epoch);
            start_order.push(txn);
        }
        RecordKind::StreamProgress => {
            let txn = required_txn(record)?;
            let Some(started_epoch) = open.get(&txn).copied() else {
                return Err(ScanError::ProgressWithoutStarted {
                    txn,
                    seq: record.seq,
                });
            };
            check_epoch(record, txn, started_epoch)?;
        }
        RecordKind::StreamTerminal | RecordKind::RepairTerminal => {
            let txn = required_txn(record)?;
            let Some(started_epoch) = open.get(&txn).copied() else {
                return Err(if started.contains(&txn) {
                    ScanError::DuplicateTerminal {
                        txn,
                        seq: record.seq,
                    }
                } else {
                    ScanError::TerminalWithoutStarted {
                        txn,
                        seq: record.seq,
                    }
                });
            };
            check_epoch(record, txn, started_epoch)?;
            open.remove(&txn);
        }
        RecordKind::Header
        | RecordKind::AcceptedOp
        | RecordKind::EmittedEvent
        | RecordKind::EffectOutcome
        | RecordKind::ConfigPatch
        | RecordKind::RewindMarker => {}
    }
    Ok(())
}

fn required_txn(record: &Record) -> Result<EffectId, ScanError> {
    record.txn.ok_or(ScanError::MissingTransaction {
        seq: record.seq,
        kind: record.kind,
    })
}

fn check_epoch(record: &Record, txn: EffectId, started: TurnEpoch) -> Result<(), ScanError> {
    if record.epoch == started {
        Ok(())
    } else {
        Err(ScanError::TransactionEpochMismatch {
            txn,
            started,
            found: record.epoch,
            seq: record.seq,
        })
    }
}

fn write_header(serializer: &mut CanonicalSerializer, header: &LogHeader) {
    serializer.write_raw(&header.session_id.0);
    match header.parent {
        None => serializer.write_raw(&[0]),
        Some(parent) => {
            serializer.write_raw(&[1]);
            serializer.write_raw(&parent.0);
        }
    }
    serializer.write_raw(&header.schema_epoch.to_le_bytes());
    for version in [
        header.prompt_pack_version,
        header.verb_grammar_version,
        header.l3_dialect_version,
    ] {
        for component in version {
            serializer.write_raw(&component.to_le_bytes());
        }
    }
    serializer.write_sized_bytes(header.codec.as_bytes());
    match &header.created_at {
        None => serializer.write_raw(&[0]),
        Some(created_at) => {
            serializer.write_raw(&[1]);
            serializer.write_sized_bytes(created_at.as_bytes());
        }
    }
}

fn serialize_canonical<T>(serializer: &mut CanonicalSerializer, value: &T)
where
    T: Serialize + ?Sized,
{
    if value.serialize(serializer).is_err() {
        unreachable!("the canonical serializer is infallible");
    }
}

struct Crc32(u32);

impl Crc32 {
    const fn new() -> Self {
        Self(u32::MAX)
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 ^= u32::from(byte);
            for _ in 0..8 {
                self.0 = if self.0 & 1 == 0 {
                    self.0 >> 1
                } else {
                    (self.0 >> 1) ^ 0xedb8_8320
                };
            }
        }
    }

    const fn finish(self) -> u32 {
        !self.0
    }
}

struct CanonicalSerializer {
    crc: Crc32,
}

impl CanonicalSerializer {
    const fn new() -> Self {
        Self { crc: Crc32::new() }
    }

    fn write_raw(&mut self, bytes: &[u8]) {
        self.crc.write(bytes);
    }

    fn write_len(&mut self, len: usize) {
        let len = u64::try_from(len).unwrap_or(u64::MAX);
        self.write_raw(&len.to_le_bytes());
    }

    fn write_optional_len(&mut self, len: Option<usize>) {
        match len {
            None => self.write_raw(&[0]),
            Some(len) => {
                self.write_raw(&[1]);
                self.write_len(len);
            }
        }
    }

    fn write_sized_bytes(&mut self, bytes: &[u8]) {
        self.write_len(bytes.len());
        self.write_raw(bytes);
    }

    const fn finish(self) -> u32 {
        self.crc.finish()
    }
}

#[derive(Debug)]
struct CanonicalError;

impl fmt::Display for CanonicalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("canonical serialization failed")
    }
}

impl core::error::Error for CanonicalError {}

impl serde::ser::Error for CanonicalError {
    fn custom<T>(_message: T) -> Self
    where
        T: fmt::Display,
    {
        Self
    }
}

struct CanonicalCompound<'a> {
    serializer: &'a mut CanonicalSerializer,
}

impl CanonicalCompound<'_> {
    fn finish(self) {
        self.serializer.write_raw(&[0xff]);
    }
}

impl<'a> Serializer for &'a mut CanonicalSerializer {
    type Ok = ();
    type Error = CanonicalError;
    type SerializeSeq = CanonicalCompound<'a>;
    type SerializeTuple = CanonicalCompound<'a>;
    type SerializeTupleStruct = CanonicalCompound<'a>;
    type SerializeTupleVariant = CanonicalCompound<'a>;
    type SerializeMap = CanonicalCompound<'a>;
    type SerializeStruct = CanonicalCompound<'a>;
    type SerializeStructVariant = CanonicalCompound<'a>;

    fn serialize_bool(self, value: bool) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[u8::from(value)]);
        Ok(())
    }

    fn serialize_i8(self, value: i8) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x02, value.to_le_bytes()[0]]);
        Ok(())
    }

    fn serialize_i16(self, value: i16) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x03]);
        self.write_raw(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_i32(self, value: i32) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x04]);
        self.write_raw(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_i64(self, value: i64) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x05]);
        self.write_raw(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_i128(self, value: i128) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x06]);
        self.write_raw(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_u8(self, value: u8) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x07, value]);
        Ok(())
    }

    fn serialize_u16(self, value: u16) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x08]);
        self.write_raw(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_u32(self, value: u32) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x09]);
        self.write_raw(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_u64(self, value: u64) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x0a]);
        self.write_raw(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_u128(self, value: u128) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x0b]);
        self.write_raw(&value.to_le_bytes());
        Ok(())
    }

    fn serialize_f32(self, value: f32) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x0c]);
        self.write_raw(&value.to_bits().to_le_bytes());
        Ok(())
    }

    fn serialize_f64(self, value: f64) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x0d]);
        self.write_raw(&value.to_bits().to_le_bytes());
        Ok(())
    }

    fn serialize_char(self, value: char) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x0e]);
        self.write_raw(&u32::from(value).to_le_bytes());
        Ok(())
    }

    fn serialize_str(self, value: &str) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x0f]);
        self.write_sized_bytes(value.as_bytes());
        Ok(())
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x10]);
        self.write_sized_bytes(value);
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x11]);
        Ok(())
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.write_raw(&[0x12]);
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x13]);
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x14]);
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.write_raw(&[0x15]);
        self.write_raw(&variant_index.to_le_bytes());
        Ok(())
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.write_raw(&[0x16]);
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: Serialize + ?Sized,
    {
        self.write_raw(&[0x17]);
        self.write_raw(&variant_index.to_le_bytes());
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        self.write_raw(&[0x18]);
        self.write_optional_len(len);
        Ok(CanonicalCompound { serializer: self })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.write_raw(&[0x19]);
        self.write_len(len);
        Ok(CanonicalCompound { serializer: self })
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.write_raw(&[0x1a]);
        self.write_len(len);
        Ok(CanonicalCompound { serializer: self })
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.write_raw(&[0x1b]);
        self.write_raw(&variant_index.to_le_bytes());
        self.write_len(len);
        Ok(CanonicalCompound { serializer: self })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.write_raw(&[0x1c]);
        self.write_optional_len(len);
        Ok(CanonicalCompound { serializer: self })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.write_raw(&[0x1d]);
        self.write_len(len);
        Ok(CanonicalCompound { serializer: self })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.write_raw(&[0x1e]);
        self.write_raw(&variant_index.to_le_bytes());
        self.write_len(len);
        Ok(CanonicalCompound { serializer: self })
    }
}

impl SerializeSeq for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish();
        Ok(())
    }
}

impl SerializeTuple for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalError;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish();
        Ok(())
    }
}

impl SerializeTupleStruct for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish();
        Ok(())
    }
}

impl SerializeTupleVariant for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalError;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish();
        Ok(())
    }
}

impl SerializeMap for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalError;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        key.serialize(&mut *self.serializer)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish();
        Ok(())
    }
}

impl SerializeStruct for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalError;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish();
        Ok(())
    }
}

impl SerializeStructVariant for CanonicalCompound<'_> {
    type Ok = ();
    type Error = CanonicalError;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(&mut *self.serializer)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.finish();
        Ok(())
    }
}
