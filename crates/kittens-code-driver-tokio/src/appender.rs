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
use std::path::Path;

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
}

impl From<std::io::Error> for OpenError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// An open, crash-repaired log with the single write handle.
pub struct Appender {
    file: File,
    next_seq: u64,
    persisted: u64,
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
    ///
    /// # Panics
    ///
    /// If `path` does not exist and `fresh_header` is `None`.
    pub fn open(
        path: &Path,
        fresh_header: Option<Record>,
    ) -> Result<(Self, Vec<Record>), OpenError> {
        let exists = path.exists();
        if !exists {
            let header = fresh_header.expect("fresh log requires a header record");
            let mut file = OpenOptions::new()
                .create_new(true)
                .append(true)
                .open(path)?;
            let line = serde_json::to_string(&header).map_err(std::io::Error::other)?;
            file.write_all(line.as_bytes())?;
            file.write_all(b"\n")?;
            file.sync_data()?;
            let next_seq = header.seq + 1;
            return Ok((
                Self {
                    file,
                    next_seq,
                    persisted: header.seq,
                },
                vec![header],
            ));
        }

        // Decode every line; only the FINAL fault is a tolerable tail —
        // scan_records enforces that (a mid-log fault is structural).
        let reader = BufReader::new(File::open(path)?);
        let mut outcomes = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            outcomes.push(decode_line(&line));
        }
        let ScanResult {
            repairs,
            replayable,
            ignored_tail: _,
        } = scan_records(outcomes, SUPPORTED_SCHEMA_EPOCH).map_err(OpenError::Scan)?;

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
            debug_assert_eq!(record.seq, self.next_seq, "appender requires strict order");
            let line = serde_json::to_string(record)
                .map_err(|e| (record.seq, std::io::Error::other(e)))?;
            self.file
                .write_all(line.as_bytes())
                .map_err(|e| (record.seq, e))?;
            self.file.write_all(b"\n").map_err(|e| (record.seq, e))?;
            self.next_seq = record.seq + 1;
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
}
