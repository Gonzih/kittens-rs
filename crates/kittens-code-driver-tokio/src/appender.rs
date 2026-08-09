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
    /// Filesystem failure.
    Io(std::io::Error),
    /// The scan refused the log (incompatible epoch, structural damage).
    Scan(ScanError),
    /// A repair record could not be written durably.
    RepairAppend(std::io::Error),
    /// A fresh log was requested with no valid header record.
    BadFreshHeader,
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
            let line = serde_json::to_string(record).map_err(std::io::Error::other)?;
            file.write_all(line.as_bytes())
                .map_err(OpenError::RepairAppend)?;
            file.write_all(b"\n").map_err(OpenError::RepairAppend)?;
        }
        if !repairs.is_empty() {
            file.sync_data().map_err(OpenError::RepairAppend)?;
        }
        let next_seq = replayable.last().map_or(0, |r| r.seq + 1);
        Ok((
            Self {
                file,
                path: path.to_path_buf(),
                next_seq,
                persisted: next_seq.saturating_sub(1),
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
            let line = serde_json::to_string(record)
                .map_err(|e| (record.seq, std::io::Error::other(e)))?;
            self.file
                .write_all(line.as_bytes())
                .map_err(|e| (record.seq, e))?;
            self.file.write_all(b"\n").map_err(|e| (record.seq, e))?;
            self.next_seq = record
                .seq
                .checked_add(1)
                .ok_or_else(|| (record.seq, std::io::Error::other("sequence exhausted")))?;
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
}
