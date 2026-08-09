//! Owned frame-writer thread with an acknowledgement lane.

use std::io::{self, Write};
use std::sync::mpsc as std_mpsc;
use std::thread;

use kittens::source::{Mpsc, close};

/// Monotonic frame identity issued by the presenter and echoed by the writer.
///
/// One presenter and one writer form one sequence domain; feeding a foreign
/// sequence into a presenter is outside the protocol guarantee.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct FrameSeq(pub(crate) u64);

impl FrameSeq {
    /// Returns the raw sequence number.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// A completion or failure event from the writer thread.
///
/// The reactor-facing lane is [`WriterSource`]; its typed `Closed` event
/// means the writer thread exited — after a [`WriterEvent::Failed`] or after
/// draining accepted frames when the frame lane closed.
#[derive(Debug)]
pub enum WriterEvent {
    /// The frame with this sequence was written and flushed.
    Written(FrameSeq),
    /// A write or flush failed; the writer exits after emitting this once.
    Failed {
        /// Sequence of the frame whose write failed.
        seq: FrameSeq,
        /// The underlying I/O failure.
        error: io::Error,
    },
}

/// Reactor source type for writer events.
pub type WriterSource = Mpsc<WriterEvent, close::Emit>;

/// The non-`Clone` frame submission lane.
///
/// One lane per writer keeps "who can submit" visible in ownership. Frames
/// are submitted only through [`crate::Draw::commit`], which is the single
/// legal path from a render request to bytes on the sink.
#[derive(Debug)]
pub struct WriterHandle {
    sender: std_mpsc::Sender<(FrameSeq, Vec<u8>)>,
}

impl WriterHandle {
    pub(crate) fn submit(&self, seq: FrameSeq, bytes: Vec<u8>) -> Result<(), Vec<u8>> {
        self.sender
            .send((seq, bytes))
            .map_err(|returned| (returned.0).1)
    }
}

/// The owned writer thread.
///
/// Dropping `FrameWriter` without [`FrameWriter::finish`] while the
/// [`WriterHandle`] is alive leaves the thread running; it drains and exits
/// on its own once the handle drops. `finish` is the canonical teardown
/// spelling and joins the thread after the drain. Dropping a writer whose
/// thread already finished reaps it.
#[derive(Debug)]
pub struct FrameWriter {
    thread: Option<thread::JoinHandle<()>>,
}

impl FrameWriter {
    /// Spawns the owned writer thread over any sink.
    ///
    /// Production callers pass a stdout handle; tests pass a `Vec`-backed
    /// sink. The writer protocol is normative in `SPEC.md` section 6.4:
    /// in-order write+flush, one acknowledgement per frame, one typed
    /// failure then exit, accepted frames drained before exit.
    ///
    /// # Panics
    ///
    /// Panics only if the operating system refuses to spawn a thread,
    /// which is outside normal operating limits.
    pub fn spawn<W: Write + Send + 'static>(mut sink: W) -> (Self, WriterHandle, WriterSource) {
        let (frame_tx, frame_rx) = std_mpsc::channel::<(FrameSeq, Vec<u8>)>();
        let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<WriterEvent>();

        let thread = thread::Builder::new()
            .name("kittens-tui-writer".to_owned())
            .spawn(move || {
                // Iterating the receiver drains frames accepted before the
                // lane closed: recv keeps succeeding until the channel is
                // both closed and empty.
                for (seq, bytes) in frame_rx {
                    let outcome = sink.write_all(&bytes).and_then(|()| sink.flush());
                    match outcome {
                        Ok(()) => {
                            if event_tx.send(WriterEvent::Written(seq)).is_err() {
                                // Event lane consumer is gone; keep draining
                                // frames so accepted work is still written.
                            }
                        }
                        Err(error) => {
                            let _ = event_tx.send(WriterEvent::Failed { seq, error });
                            return;
                        }
                    }
                }
            })
            .expect("spawning the writer thread cannot fail under normal OS limits");

        (
            Self {
                thread: Some(thread),
            },
            WriterHandle { sender: frame_tx },
            kittens::source::mpsc(event_rx, close::Emit),
        )
    }

    /// Closes the frame lane and joins the writer after it drains accepted
    /// frames. The supported teardown path; run it before dropping the
    /// enclosing [`crate::TerminalSession`] so drained output lands on a
    /// live terminal.
    ///
    /// # Errors
    ///
    /// Propagates the writer thread's panic payload, as `std::thread::join`
    /// does. The writer itself reports I/O failure through
    /// [`WriterEvent::Failed`], not through this result.
    ///
    /// # Panics
    ///
    /// Panics if the private thread-handle invariant is violated. No public
    /// constructor can create a `FrameWriter` without its thread handle.
    pub fn finish(mut self, handle: WriterHandle) -> thread::Result<()> {
        drop(handle);
        // `spawn` is the only constructor, and `finish` consumes `self` before
        // `Drop` can take the handle. `Option` exists solely to move the handle
        // out during one of those two teardown paths; an empty live writer is
        // not a constructible protocol state.
        let thread = self
            .thread
            .take()
            .expect("FrameWriter owns its thread until finish or drop");
        thread.join()
    }
}

impl Drop for FrameWriter {
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            if thread.is_finished() {
                let _ = thread.join();
            }
            // A live thread self-terminates once the WriterHandle drops:
            // the frame lane closes, accepted frames drain, and the loop
            // exits. Documented as the non-canonical path; use `finish`.
        }
    }
}
