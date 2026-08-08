//! The render gate: coalescing, one frame in flight, acknowledgement,
//! throttle deadline.
//!
//! This is the concrete TUI presenter promoted from the K0 parity oracles,
//! faithful to the inspected Grok protocol. It is deliberately not the
//! deferred generic `SingleFlight<Ticket, Merge>` gate; see `SPEC.md`
//! sections 4 and 10 for that relationship.

use std::error::Error;
use std::fmt;
use std::time::Duration;

use tokio::time::Instant;

use crate::writer::{FrameSeq, WriterHandle};

/// A coalescing render request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderRequest {
    /// Something changed; a redraw is wanted.
    Dirty,
    /// The next committed draw must repaint fully. Sticky until a draw is
    /// committed or reports no output.
    FullRepaint,
}

/// Result of feeding an acknowledgement to the presenter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ack {
    /// The acknowledgement was at or beyond the in-flight frame; the gate is
    /// open again.
    Accepted,
    /// Lower than the in-flight frame, or nothing was in flight. Nothing
    /// changed.
    Stale,
}

/// The frame lane is closed; the payload is returned and the render request
/// remains pending.
pub struct WriterClosed {
    /// The payload that was not submitted.
    pub bytes: Vec<u8>,
}

impl fmt::Debug for WriterClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WriterClosed")
            .field("bytes_len", &self.bytes.len())
            .finish()
    }
}

impl fmt::Display for WriterClosed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "the frame writer lane is closed")
    }
}

impl Error for WriterClosed {}

/// The render gate. Protocol is normative in `SPEC.md` section 6.6.
///
/// What the presenter does not prove: it cannot order writes performed
/// outside its writer lane, cannot verify that submitted bytes correspond
/// to the state that marked it dirty, and an accepted acknowledgement means
/// the sink accepted and flushed the bytes — not that a terminal rendered
/// them.
#[derive(Debug)]
pub struct Presenter {
    dirty: bool,
    force_full: bool,
    in_flight: Option<FrameSeq>,
    last_present: Option<Instant>,
    scheduled: Option<Instant>,
    min_interval: Duration,
    next_seq: u64,
}

impl Presenter {
    /// Creates a gate with an explicit frame throttle. There is no
    /// `Default`: the throttle is policy and stays visible at the
    /// construction site.
    pub const fn new(min_interval: Duration) -> Self {
        Self {
            dirty: false,
            force_full: false,
            in_flight: None,
            last_present: None,
            scheduled: None,
            min_interval,
            next_seq: 0,
        }
    }

    /// Records a render request. Requests coalesce; `FullRepaint` is sticky
    /// until a committed or no-output draw.
    pub fn request(&mut self, request: RenderRequest) {
        self.dirty = true;
        if matches!(request, RenderRequest::FullRepaint) {
            self.force_full = true;
        }
    }

    /// Whether a request is pending.
    pub const fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// The frame currently awaiting acknowledgement, if any.
    pub const fn in_flight(&self) -> Option<FrameSeq> {
        self.in_flight
    }

    /// Begins a draw when one is due.
    ///
    /// Returns `None` when nothing is dirty, a frame is in flight, or the
    /// throttle has not elapsed; the throttled case schedules the instant
    /// exposed by [`Presenter::deadline`]. The pending request is never
    /// cleared here — only [`Draw::commit`] or [`Draw::no_output`] clears
    /// it, so dropping the returned [`Draw`] retains the request.
    pub fn try_begin(&mut self, now: Instant) -> Option<Draw<'_>> {
        if !self.dirty || self.in_flight.is_some() {
            return None;
        }
        if let Some(last) = self.last_present {
            let eligible = last + self.min_interval;
            if now < eligible {
                self.scheduled = Some(eligible);
                return None;
            }
        }
        Some(Draw {
            presenter: self,
            now,
        })
    }

    /// Earliest eligible presentation instant. `Some` only while a pending
    /// request is throttle-blocked and nothing is in flight; arm a
    /// [`kittens::source::OptionalDeadline`] from this in `before_poll`.
    pub fn deadline(&self) -> Option<Instant> {
        if self.dirty && self.in_flight.is_none() {
            self.scheduled
        } else {
            None
        }
    }

    /// Clears a fired schedule; call from the deadline arm's handler.
    pub fn on_deadline(&mut self) {
        self.scheduled = None;
    }

    /// Feeds a writer acknowledgement to the gate.
    ///
    /// `Accepted` exactly when a frame is in flight and `seq` is at or
    /// beyond it; every other acknowledgement — lower, or with nothing in
    /// flight — is `Stale` and changes nothing.
    pub fn acknowledge(&mut self, seq: FrameSeq) -> Ack {
        match self.in_flight {
            Some(target) if seq >= target => {
                self.in_flight = None;
                Ack::Accepted
            }
            _ => Ack::Stale,
        }
    }
}

/// An exclusive draw permit.
///
/// Exactly one `Draw` can exist at a time: it holds the presenter's `&mut`
/// borrow, so a second simultaneous draw is a compile error (E0499), not a
/// runtime state.
#[must_use = "a Draw that is neither committed nor reported as no_output retains the pending request without presenting"]
pub struct Draw<'p> {
    presenter: &'p mut Presenter,
    now: Instant,
}

impl Draw<'_> {
    /// Whether this draw must repaint fully.
    pub fn full_repaint(&self) -> bool {
        self.presenter.force_full
    }

    /// Reserves the next sequence, submits the payload to the writer lane,
    /// records the frame in flight, clears the request, and advances the
    /// throttle from the instant captured at [`Presenter::try_begin`].
    ///
    /// # Errors
    ///
    /// [`WriterClosed`] returns the payload and leaves the request pending;
    /// no presenter state advances.
    pub fn commit(self, writer: &WriterHandle, bytes: Vec<u8>) -> Result<FrameSeq, WriterClosed> {
        let seq = FrameSeq(self.presenter.next_seq);
        match writer.submit(seq, bytes) {
            Ok(()) => {
                self.presenter.next_seq += 1;
                self.presenter.in_flight = Some(seq);
                self.presenter.dirty = false;
                self.presenter.force_full = false;
                self.presenter.scheduled = None;
                self.presenter.last_present = Some(self.now);
                Ok(seq)
            }
            Err(bytes) => Err(WriterClosed { bytes }),
        }
    }

    /// The legitimate no-payload outcome: the presentation is satisfied, the
    /// request clears, and the throttle advances — without creating an
    /// in-flight frame or an acknowledgement target.
    pub fn no_output(self) {
        self.presenter.dirty = false;
        self.presenter.force_full = false;
        self.presenter.scheduled = None;
        self.presenter.last_present = Some(self.now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::writer::FrameWriter;

    fn now() -> Instant {
        Instant::now()
    }

    #[test]
    fn requests_coalesce_and_full_repaint_is_sticky() {
        let mut p = Presenter::new(Duration::ZERO);
        p.request(RenderRequest::Dirty);
        p.request(RenderRequest::FullRepaint);
        p.request(RenderRequest::Dirty);
        assert!(p.is_dirty());

        let (writer, handle, _events) = FrameWriter::spawn(Vec::new());
        let draw = p.try_begin(now()).expect("draw is due");
        assert!(draw.full_repaint(), "FullRepaint is sticky until drawn");
        let seq = draw.commit(&handle, b"frame".to_vec()).expect("lane open");
        assert!(!p.is_dirty());
        assert_eq!(p.in_flight(), Some(seq));
        writer.finish(handle).expect("writer joins");
    }

    #[test]
    fn one_frame_in_flight_until_acknowledged() {
        let mut p = Presenter::new(Duration::ZERO);
        let (writer, handle, _events) = FrameWriter::spawn(Vec::new());
        p.request(RenderRequest::Dirty);
        let seq = p
            .try_begin(now())
            .expect("first draw")
            .commit(&handle, b"a".to_vec())
            .expect("lane open");

        p.request(RenderRequest::Dirty);
        assert!(
            p.try_begin(now()).is_none(),
            "gate is closed while a frame is in flight"
        );

        assert_eq!(p.acknowledge(seq), Ack::Accepted);
        assert!(p.try_begin(now()).is_some(), "acknowledgement reopens");
        writer.finish(handle).expect("writer joins");
    }

    #[test]
    fn stale_and_absent_acknowledgements_do_not_unlock() {
        let mut p = Presenter::new(Duration::ZERO);
        assert_eq!(
            p.acknowledge(FrameSeq(0)),
            Ack::Stale,
            "no in-flight frame means every ack is stale"
        );

        let (writer, handle, _events) = FrameWriter::spawn(Vec::new());
        p.request(RenderRequest::Dirty);
        let first = p
            .try_begin(now())
            .expect("draw")
            .commit(&handle, b"a".to_vec())
            .expect("lane open");
        assert_eq!(p.acknowledge(first), Ack::Accepted);

        p.request(RenderRequest::Dirty);
        let second = p
            .try_begin(now())
            .expect("draw")
            .commit(&handle, b"b".to_vec())
            .expect("lane open");
        assert!(second > first, "sequences are strictly monotonic");
        assert_eq!(
            p.acknowledge(first),
            Ack::Stale,
            "an earlier sequence cannot unlock a newer frame"
        );
        assert_eq!(p.in_flight(), Some(second));
        writer.finish(handle).expect("writer joins");
    }

    #[test]
    fn no_output_clears_request_without_ack_target() {
        let mut p = Presenter::new(Duration::ZERO);
        p.request(RenderRequest::FullRepaint);
        p.try_begin(now()).expect("draw").no_output();
        assert!(!p.is_dirty());
        assert_eq!(p.in_flight(), None, "no acknowledgement target invented");

        // The sticky full-repaint was satisfied by the no-output draw.
        p.request(RenderRequest::Dirty);
        let (writer, handle, _events) = FrameWriter::spawn(Vec::new());
        let draw = p.try_begin(now()).expect("draw");
        assert!(!draw.full_repaint());
        draw.no_output();
        writer.finish(handle).expect("writer joins");
    }

    #[test]
    fn dropped_draw_retains_the_request() {
        let mut p = Presenter::new(Duration::ZERO);
        p.request(RenderRequest::Dirty);
        {
            let _draw = p.try_begin(now()).expect("draw");
            // Dropped without commit or no_output.
        }
        assert!(p.is_dirty(), "an uncommitted draw does not lose the request");
        assert_eq!(p.in_flight(), None);
    }

    #[test]
    fn throttle_schedules_a_deadline_and_survives_delayed_ack() {
        let interval = Duration::from_millis(50);
        let mut p = Presenter::new(interval);
        let (writer, handle, _events) = FrameWriter::spawn(Vec::new());

        let t0 = now();
        p.request(RenderRequest::Dirty);
        let seq = p
            .try_begin(t0)
            .expect("first draw is unthrottled")
            .commit(&handle, b"a".to_vec())
            .expect("lane open");

        // A new request while in flight: no deadline yet, the gate blocks on
        // the acknowledgement, not the throttle.
        p.request(RenderRequest::Dirty);
        assert_eq!(p.deadline(), None, "in-flight frame masks the throttle");

        // Acknowledge, then attempt to draw again inside the interval.
        assert_eq!(p.acknowledge(seq), Ack::Accepted);
        let t1 = t0 + Duration::from_millis(10);
        assert!(p.try_begin(t1).is_none(), "inside the throttle window");
        assert_eq!(
            p.deadline(),
            Some(t0 + interval),
            "the deadline is the earliest eligible instant"
        );

        // The deadline fires; the draw becomes due at the eligible instant.
        p.on_deadline();
        let t2 = t0 + interval;
        assert!(p.try_begin(t2).is_some(), "eligible exactly at the deadline");
        writer.finish(handle).expect("writer joins");
    }

    #[test]
    fn commit_on_closed_lane_returns_bytes_and_retains_request() {
        struct FailingSink;
        impl std::io::Write for FailingSink {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("sink rejects every write"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        // The writer emits one typed Failed event and exits, which closes
        // the frame lane while the handle is still alive.
        let (_writer, handle, _events) = FrameWriter::spawn(FailingSink);
        let mut first = Presenter::new(Duration::ZERO);
        first.request(RenderRequest::Dirty);
        first
            .try_begin(now())
            .expect("draw")
            .commit(&handle, b"doomed".to_vec())
            .expect("the channel accepts the frame before the writer fails");

        // Wait until the exited writer thread has dropped the receiver.
        while handle.submit(FrameSeq(u64::MAX), Vec::new()).is_ok() {
            std::thread::sleep(Duration::from_millis(1));
        }

        let mut p = Presenter::new(Duration::ZERO);
        p.request(RenderRequest::Dirty);
        let error = p
            .try_begin(now())
            .expect("draw")
            .commit(&handle, b"payload".to_vec())
            .expect_err("lane is closed after the writer failure");
        assert_eq!(error.bytes, b"payload");
        assert!(p.is_dirty(), "request survives a failed submission");
        assert_eq!(p.in_flight(), None);
        assert!(
            p.try_begin(now()).is_some(),
            "the retained request is immediately drawable again"
        );
    }
}
