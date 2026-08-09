#![forbid(unsafe_code)]
//! Testable JSONL protocol loop for the KC0 headless composition root.
//!
//! The binary owns bootstrap configuration and model selection. This library
//! owns the transport-neutral stdin/stdout behavior: deserialize one
//! `kittens_code_protocol::op::Op` per line, assign a submission id,
//! drive the supplied [`Runner`] to quiescence, and serialize newly published
//! events exactly once.

use std::io;

use kittens_code_core::prompts::PROMPT_PACK_VERSION;
use kittens_code_core::record::{LogHeader, Record, RecordBuildError, RecordKind, RecordPayload};
use kittens_code_driver_tokio::appender::{CODEC, SUPPORTED_SCHEMA_EPOCH};
use kittens_code_driver_tokio::runner::Runner;
use kittens_code_protocol::error::{ErrorCode, ErrorEvent};
use kittens_code_protocol::event::Event;
use kittens_code_protocol::ids::{SessionId, SubmissionId, TurnEpoch};
use kittens_code_protocol::op::{Op, Submission};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

/// Builds the mandatory first record for a fresh KC0 session log.
///
/// The header versions and codec are the versions implemented by the linked
/// core and Tokio driver. Session-id generation remains the composition
/// root's responsibility.
///
/// # Errors
///
/// Returns [`RecordBuildError`] if the header payload and record kind ever
/// cease to agree.
pub fn fresh_header(session_id: SessionId) -> Result<Record, RecordBuildError> {
    Record::new(
        0,
        RecordKind::Header,
        None,
        TurnEpoch(0),
        RecordPayload::Header(LogHeader {
            session_id,
            parent: None,
            schema_epoch: SUPPORTED_SCHEMA_EPOCH,
            prompt_pack_version: PROMPT_PACK_VERSION.0,
            verb_grammar_version: [1, 0, 0],
            l3_dialect_version: [1, 0, 0],
            codec: String::from(CODEC),
            created_at: None,
        }),
    )
}

/// Runs the headless JSONL protocol over an already-opened session runner.
///
/// Each non-empty input line is decoded as an [`Op`], assigned a submission
/// id starting at one, submitted, and driven to quiescence. Only events not
/// written by an earlier drive are serialized, one per output line, and the
/// writer is flushed after every event. A malformed line produces a
/// non-persisted [`ErrorCode::ConfigInvalid`] event and does not stop the
/// stream. Empty lines are ignored. Shutdown and EOF both drain before exit.
///
/// # Errors
///
/// Returns an IO error when input cannot be read, an event cannot be
/// serialized, output cannot be written or flushed, or the submission-id
/// namespace is exhausted.
pub async fn run<R, W>(reader: R, writer: W, runner: &mut Runner) -> io::Result<()>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut reader = reader;
    let mut writer = writer;
    let mut line = String::new();
    let mut next_submission = 1_u64;
    let mut emitted = 0_usize;

    loop {
        line.clear();
        let read = reader.read_line(&mut line).await?;
        if read == 0 {
            drive_and_write(runner, &mut writer, &mut emitted).await?;
            return Ok(());
        }

        let input = line.trim();
        if input.is_empty() {
            continue;
        }
        let op = match serde_json::from_str::<Op>(input) {
            Ok(op) => op,
            Err(error) => {
                let event = Event::Error(ErrorEvent::new(
                    ErrorCode::ConfigInvalid,
                    format!("invalid op JSON: {error}"),
                    None,
                ));
                write_event(&mut writer, &event).await?;
                continue;
            }
        };

        let shutdown = matches!(&op, Op::Shutdown);
        let id = SubmissionId(next_submission);
        next_submission = next_submission.checked_add(1).ok_or_else(|| {
            io::Error::other("submission-id namespace exhausted in protocol loop")
        })?;
        runner.submit(Submission { id, op });
        drive_and_write(runner, &mut writer, &mut emitted).await?;
        if shutdown {
            return Ok(());
        }
    }
}

async fn drive_and_write<W>(
    runner: &mut Runner,
    writer: &mut W,
    emitted: &mut usize,
) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let events = runner.run_to_idle().await;
    for event in &events[*emitted..] {
        write_event(writer, event).await?;
    }
    *emitted = events.len();
    Ok(())
}

async fn write_event<W>(writer: &mut W, event: &Event) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let encoded = serde_json::to_vec(event).map_err(io::Error::other)?;
    writer.write_all(&encoded).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await
}
