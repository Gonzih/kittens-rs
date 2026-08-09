//! Terminal-orchestration profile for the Kittens reactor kernel.
//!
//! `kittens-tui` supplies the owned producers, render-gate protocol, and
//! terminal lifecycle that a long-lived TUI reactor needs. It is the
//! K1-TUI slice specified in this crate's `SPEC.md`; the enforcement layer
//! for every guarantee is named there and summarized per item below.
//!
//! What this crate is **not**: a widget, layout, styling, cell-buffer, or
//! diffing framework. A frame payload is opaque bytes produced by the
//! caller; component libraries own rendering above this crate. The kernel's
//! topology checks (arm order, yields, drains) stay declared in
//! [`kittens::reactor!`] in the caller's loop.
//!
//! The five components:
//!
//! - [`TerminalSession`]: raw mode + optional alternate screen, with ordered
//!   best-effort restoration attempts on drop including unwind (RAII).
//! - [`InputReader`]: an owned reader thread with bounded polls and a
//!   pause/park handshake; its reactor edge is an admitted
//!   [`kittens::source::Mpsc`] whose typed `Closed` event means "the reader
//!   thread exited" (structural isolation).
//! - [`FrameWriter`]: an owned writer thread over any sink; frames are
//!   written and flushed in submission order, each acknowledged with its
//!   [`FrameSeq`], failure is a typed terminal event, and accepted frames
//!   are drained before exit (runtime protocol + deterministic tests).
//! - [`Presenter`]: the render gate — request coalescing, at most one frame
//!   in flight, monotonic sequences, stale-acknowledgement rejection, and a
//!   throttle deadline (private runtime state + an exclusive [`Draw`]
//!   permit + deterministic tests).
//! - The canonical wiring lives in `examples/status_line.rs` and in
//!   `SPEC.md` section 6.7 (kernel declarations in the caller's loop).
//!
//! Honest boundary, per root SPEC section 4.12: nothing here can order a
//! raw `write!` performed outside the writer lane, verify that submitted
//! bytes match the state that marked the presenter dirty, or prove that an
//! acknowledged frame was rendered by a physical terminal.

#![forbid(unsafe_code)]

mod input;
mod presenter;
mod terminal;
mod writer;

pub use input::{InputEvent, InputReader, InputSource};
pub use presenter::{Ack, Draw, Presenter, RenderRequest, WriterClosed};
pub use terminal::TerminalSession;
pub use writer::{FrameSeq, FrameWriter, WriterEvent, WriterHandle, WriterSource};
