//! The log-appender: the single owner of the storage write path (SPEC
//! append canon, S3–S7).
//!
//! One appender exists per session. Startup ordering is law: open →
//! validate `schema_epoch` FIRST (an old binary never mutates an
//! incompatible log) → scan → append crash-repair terminals through the
//! same write path → report durability → hand back the replayable records.
//! During the session, `CoreAction::Commit` batches arrive over a channel,
//! are written in strict sequence order as framed JSONL (one
//! checksum-carrying record per line), flushed, and acknowledged with a
//! `Persisted` watermark.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use kittens_code_core::record::{
    DecodeOutcome, Record, ScanError, ScanResult, TailFault, scan_records,
};

/// The persisted-schema epoch this binary supports.
pub const SUPPORTED_SCHEMA_EPOCH: u32 = 0;

/// The codec identifier written into log headers by this driver.
pub const CODEC: &str = "jsonl-v1";

/// Why a log could not be opened.
#[derive(Debug)]
pub enum OpenError {
    /// Another live appender owns the sidecar writer lock.
    WriterLocked,
    /// Filesystem failure.
    Io(std::io::Error),
    /// The scan refused the log (incompatible epoch, structural damage).
    Scan(ScanError),
    /// A repair record could not be written durably.
    RepairAppend(std::io::Error),
    /// A fresh log was requested with no valid header record.
    BadFreshHeader,
    /// The persisted log already used the largest sequence number.
    SequenceExhausted,
}

impl From<std::io::Error> for OpenError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// An open, crash-repaired log with the single write handle.
pub struct Appender {
    file: File,
    path: PathBuf,
    next_seq: u64,
    persisted: u64,
    _writer_lock: WriterLock,
}

struct WriterLock {
    _file: File,
    path: PathBuf,
}

impl WriterLock {
    fn acquire(log_path: &Path) -> Result<Self, OpenError> {
        let mut lock_name = log_path.as_os_str().to_owned();
        lock_name.push(".lock");
        let path = PathBuf::from(lock_name);
        let file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(OpenError::WriterLocked);
            }
            Err(error) => return Err(OpenError::Io(error)),
        };
        Ok(Self { _file: file, path })
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Extracts the schema epoch from a header record, or `None` if it is not a
/// header payload.
fn header_schema_epoch(record: &Record) -> Option<u32> {
    match &record.payload {
        kittens_code_core::record::RecordPayload::Header(h) => Some(h.schema_epoch),
        _ => None,
    }
}

/// Decodes one JSONL line into a scan outcome.
fn decode_line(line: &str) -> DecodeOutcome {
    match serde_json::from_str::<Record>(line) {
        Ok(record) => {
            if record.is_valid() {
                DecodeOutcome::Good(record)
            } else {
                DecodeOutcome::Tail(TailFault::ChecksumMismatch)
            }
        }
        Err(_) => DecodeOutcome::Tail(TailFault::Torn),
    }
}

impl Appender {
    /// Opens an existing log or creates a fresh one with `header`.
    ///
    /// Returns the appender plus the replayable record sequence (valid
    /// prefix + persisted repair terminals; SPEC S3 crash repair).
    ///
    /// # Errors
    ///
    /// [`OpenError`] on IO failure, scan refusal (including a higher
    /// `schema_epoch` — checked before any mutation), or a repair append
    /// that cannot be made durable.
    pub fn open(
        path: &Path,
        fresh_header: Option<Record>,
    ) -> Result<(Self, Vec<Record>), OpenError> {
        let writer_lock = WriterLock::acquire(path)?;
        let exists = path.exists();
        if !exists {
            // A fresh log's header is validated exactly like an existing
            // one (review input 19 #16): it must be a well-formed, valid,
            // supported header at seq 0, or the open is refused rather than
            // panicking.
            let header = fresh_header.ok_or(OpenError::BadFreshHeader)?;
            if !header.is_valid()
                || header.seq != 0
                || !matches!(header.kind, kittens_code_core::record::RecordKind::Header)
                || header_schema_epoch(&header).is_none_or(|e| e > SUPPORTED_SCHEMA_EPOCH)
            {
                return Err(OpenError::BadFreshHeader);
            }
            let mut file = OpenOptions::new()
                .create_new(true)
                .append(true)
                .open(path)?;
            let line = serde_json::to_string(&header).map_err(std::io::Error::other)?;
            file.write_all(line.as_bytes())?;
            file.write_all(b"\n")?;
            file.sync_data()?;
            return Ok((
                Self {
                    file,
                    path: path.to_path_buf(),
                    next_seq: 1,
                    persisted: 0,
                    _writer_lock: writer_lock,
                },
                vec![header],
            ));
        }

        // Decode every line, tracking the byte offset at which each valid
        // line ENDS. `scan_records` tolerates only a FINAL fault; a torn or
        // checksum-bad tail must be physically removed so a later repair or
        // append does not land after the bad bytes and turn the tolerable
        // tail into a fatal mid-log fault on the next reopen (review input 20
        // #3 / review-19 #2).
        let reader = BufReader::new(File::open(path)?);
        let mut outcomes = Vec::new();
        let mut byte_offset: u64 = 0;
        let mut valid_prefix_end: u64 = 0;
        for line in reader.lines() {
            let line = line?;
            let line_bytes = line.len() as u64 + 1; // + newline
            let this_start = byte_offset;
            byte_offset += line_bytes;
            if line.trim().is_empty() {
                // Blank lines carry no record but still advance the offset;
                // keep them inside the valid prefix.
                valid_prefix_end = byte_offset;
                continue;
            }
            let outcome = decode_line(&line);
            match &outcome {
                DecodeOutcome::Good(_) => valid_prefix_end = byte_offset,
                DecodeOutcome::Tail(_) => {
                    // The valid prefix ends where this bad line began; the
                    // scanner still validates ordering over the goods.
                    let _ = this_start;
                }
            }
            outcomes.push(outcome);
        }
        let ScanResult {
            repairs,
            replayable,
            ignored_tail,
        } = scan_records(outcomes, SUPPORTED_SCHEMA_EPOCH).map_err(OpenError::Scan)?;

        // If a tail fault was tolerated, truncate the physical file to the
        // last valid record boundary BEFORE any repair/append, so the bad
        // bytes never sit mid-log.
        if ignored_tail.is_some() && valid_prefix_end < byte_offset {
            let truncate = OpenOptions::new().write(true).open(path)?;
            truncate.set_len(valid_prefix_end)?;
            truncate.sync_all()?;
        }

        // Append repair terminals through the same write path before any
        // replay (SPEC: crash repair is persisted, ordering law).
        let mut file = OpenOptions::new().append(true).open(path)?;
        for record in &repairs {
            #[cfg(test)]
            if path.file_name().and_then(|name| name.to_str()) == Some("fail-repair-append.jsonl") {
                return Err(OpenError::RepairAppend(std::io::Error::other(
                    "injected repair append failure",
                )));
            }
            let line = serde_json::to_string(record).map_err(std::io::Error::other)?;
            file.write_all(line.as_bytes())
                .map_err(OpenError::RepairAppend)?;
            file.write_all(b"\n").map_err(OpenError::RepairAppend)?;
        }
        if !repairs.is_empty() {
            file.sync_data().map_err(OpenError::RepairAppend)?;
        }
        let next_seq = replayable.last().map_or(Ok(0), |record| {
            record
                .seq
                .checked_add(1)
                .ok_or(OpenError::SequenceExhausted)
        })?;
        Ok((
            Self {
                file,
                path: path.to_path_buf(),
                next_seq,
                persisted: next_seq.saturating_sub(1),
                _writer_lock: writer_lock,
            },
            replayable,
        ))
    }

    /// Appends a committed batch in order; returns the durability
    /// watermark after flush.
    ///
    /// # Errors
    ///
    /// The IO error and the failing sequence, for `PersistFailed`.
    pub fn append(&mut self, records: &[Record]) -> Result<u64, (u64, std::io::Error)> {
        for record in records {
            // Strict monotonic order is a hard contract, enforced in release
            // too (review input 19 #16): an out-of-order record is refused
            // before it can corrupt the sequence, not merely asserted.
            if record.seq != self.next_seq {
                return Err((
                    record.seq,
                    std::io::Error::new(std::io::ErrorKind::InvalidInput, "append out of sequence"),
                ));
            }
            let next_seq = record.seq.checked_add(1).ok_or_else(|| {
                (
                    record.seq,
                    std::io::Error::other("sequence namespace exhausted"),
                )
            })?;
            let line = serde_json::to_string(record)
                .map_err(|e| (record.seq, std::io::Error::other(e)))?;
            self.file
                .write_all(line.as_bytes())
                .map_err(|e| (record.seq, e))?;
            self.file.write_all(b"\n").map_err(|e| (record.seq, e))?;
            self.next_seq = next_seq;
        }
        if let Some(last) = records.last() {
            self.file.sync_data().map_err(|e| (last.seq, e))?;
            self.persisted = last.seq;
        }
        Ok(self.persisted)
    }

    /// The durability watermark.
    #[must_use]
    pub fn persisted(&self) -> u64 {
        self.persisted
    }

    /// The next sequence the engine must use.
    #[must_use]
    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    /// Filesystem location of this appender's transcript. Kept crate-local
    /// so the runner can discharge store-read effects without creating a
    /// second storage owner.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// Replaces the append handle with a read-only handle so runner tests can
    /// exercise the real persistence-failure path without changing the
    /// production appender API.
    #[cfg(test)]
    pub(crate) fn inject_write_failure(&mut self) {
        self.file = File::open(&self.path).expect("test log remains readable");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kittens_code_core::prompts::PROMPT_PACK_VERSION;
    use kittens_code_core::record::{Checksum, LogHeader, RecordKind, RecordPayload};
    use kittens_code_protocol::event::Event;
    use kittens_code_protocol::ids::{EffectId, SessionId, TurnEpoch};

    fn header(session: u8) -> Record {
        Record::new(
            0,
            RecordKind::Header,
            None,
            TurnEpoch(0),
            RecordPayload::Header(LogHeader {
                session_id: SessionId([session; 16]),
                parent: None,
                schema_epoch: SUPPORTED_SCHEMA_EPOCH,
                prompt_pack_version: PROMPT_PACK_VERSION.0,
                verb_grammar_version: [1, 0, 0],
                l3_dialect_version: [1, 0, 0],
                codec: String::from(CODEC),
                created_at: None,
            }),
        )
        .expect("header")
    }

    fn event_record(seq: u64) -> Record {
        Record::new(
            seq,
            RecordKind::EmittedEvent,
            None,
            TurnEpoch(0),
            RecordPayload::EmittedEvent(Event::ShuttingDown),
        )
        .expect("event record")
    }

    fn write_records(path: &Path, records: &[Record]) {
        let text = records
            .iter()
            .map(|record| serde_json::to_string(record).expect("serialize"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        std::fs::write(path, text).expect("seed log");
    }

    #[test]
    fn open_error_from_io_preserves_the_error() {
        let error = OpenError::from(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        assert!(matches!(
            error,
            OpenError::Io(inner) if inner.kind() == std::io::ErrorKind::PermissionDenied
        ));
    }

    #[test]
    fn writer_lock_io_and_scan_errors_are_distinct() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing_parent = dir.path().join("missing/session.jsonl");
        assert!(matches!(
            Appender::open(&missing_parent, Some(header(1))),
            Err(OpenError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound
        ));

        let corrupt = dir.path().join("mid-log-corruption.jsonl");
        let head = serde_json::to_string(&header(2)).expect("serialize header");
        let tail = serde_json::to_string(&event_record(1)).expect("serialize event");
        std::fs::write(&corrupt, format!("{head}\n{{torn\n{tail}\n")).expect("seed corrupt log");
        assert!(matches!(
            Appender::open(&corrupt, None),
            Err(OpenError::Scan(_))
        ));
    }

    #[test]
    fn fresh_header_rejections_are_table_driven_and_release_the_lock() {
        let mut bad_checksum = header(10);
        bad_checksum.checksum = Checksum(bad_checksum.checksum.0 ^ 1);

        let mut wrong_sequence = header(11);
        wrong_sequence.seq = 1;
        wrong_sequence.checksum = wrong_sequence.computed_checksum();

        let mut wrong_kind = header(12);
        wrong_kind.kind = RecordKind::EmittedEvent;
        wrong_kind.checksum = wrong_kind.computed_checksum();

        let mut non_header_payload = header(13);
        non_header_payload.payload = RecordPayload::EmittedEvent(Event::ShuttingDown);
        non_header_payload.checksum = non_header_payload.computed_checksum();

        let mut future_epoch = header(14);
        if let RecordPayload::Header(header) = &mut future_epoch.payload {
            header.schema_epoch = SUPPORTED_SCHEMA_EPOCH + 1;
        }
        future_epoch.checksum = future_epoch.computed_checksum();

        let dir = tempfile::tempdir().expect("tempdir");
        let cases = [
            ("missing", None),
            ("checksum", Some(bad_checksum)),
            ("sequence", Some(wrong_sequence)),
            ("kind", Some(wrong_kind)),
            ("payload", Some(non_header_payload)),
            ("epoch", Some(future_epoch)),
        ];
        for (name, candidate) in cases {
            let path = dir.path().join(format!("{name}.jsonl"));
            assert!(matches!(
                Appender::open(&path, candidate),
                Err(OpenError::BadFreshHeader)
            ));
            assert!(!path.with_extension("jsonl.lock").exists());
        }
    }

    #[test]
    fn line_decoder_and_header_epoch_cover_every_outcome() {
        let head = header(20);
        assert_eq!(header_schema_epoch(&head), Some(SUPPORTED_SCHEMA_EPOCH));
        assert_eq!(header_schema_epoch(&event_record(1)), None);

        let good = serde_json::to_string(&head).expect("serialize");
        assert!(matches!(decode_line(&good), DecodeOutcome::Good(_)));

        let mut corrupt = head;
        corrupt.checksum = Checksum(corrupt.checksum.0 ^ 1);
        assert!(matches!(
            decode_line(&serde_json::to_string(&corrupt).expect("serialize")),
            DecodeOutcome::Tail(TailFault::ChecksumMismatch)
        ));
        assert!(matches!(
            decode_line("{not-json"),
            DecodeOutcome::Tail(TailFault::Torn)
        ));
    }

    #[test]
    fn blank_lines_stay_in_the_valid_prefix() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("blank-lines.jsonl");
        let head = serde_json::to_string(&header(21)).expect("serialize");
        std::fs::write(&path, format!("{head}\n\n   \n")).expect("seed log");
        let (appender, replay) = Appender::open(&path, None).expect("blank lines are ignored");
        assert_eq!(replay.len(), 1);
        assert_eq!(appender.persisted(), 0);
    }

    #[test]
    fn repair_append_failure_is_classified_and_releases_the_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("fail-repair-append.jsonl");
        let started = Record::new(
            1,
            RecordKind::StreamStarted,
            Some(EffectId(7)),
            TurnEpoch(1),
            RecordPayload::StreamStarted(Vec::new()),
        )
        .expect("stream start");
        write_records(&path, &[header(22), started]);

        assert!(matches!(
            Appender::open(&path, None),
            Err(OpenError::RepairAppend(error)) if error.to_string().contains("injected")
        ));
        assert!(!path.with_extension("jsonl.lock").exists());
    }

    #[test]
    fn append_empty_io_failure_and_sequence_exhaustion_are_reported() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("append.jsonl");
        let (mut appender, _) = Appender::open(&path, Some(header(23))).expect("open");
        assert_eq!(appender.append(&[]).expect("empty append"), 0);
        assert_eq!(appender.persisted(), 0);

        appender.inject_write_failure();
        let error = appender
            .append(&[event_record(1)])
            .expect_err("read-only handle");
        assert_eq!(error.0, 1);
        assert_ne!(error.1.kind(), std::io::ErrorKind::InvalidInput);

        let exhausted_path = dir.path().join("append-exhausted.jsonl");
        let lock = WriterLock::acquire(&exhausted_path).expect("lock");
        let file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&exhausted_path)
            .expect("file");
        let mut exhausted = Appender {
            file,
            path: exhausted_path,
            next_seq: u64::MAX,
            persisted: u64::MAX - 1,
            _writer_lock: lock,
        };
        let error = exhausted
            .append(&[event_record(u64::MAX)])
            .expect_err("namespace exhausted");
        assert_eq!(error.0, u64::MAX);
        assert!(error.1.to_string().contains("namespace exhausted"));
    }
}
