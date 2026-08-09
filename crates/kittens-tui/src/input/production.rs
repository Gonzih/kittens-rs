//! Live crossterm binding for [`super::InputReader`].
//!
//! This file is the explicit coverage exemption named by SPEC section 9. The
//! process-global terminal event source requires an owned live terminal and
//! cannot be exercised honestly by the deterministic no-tty suite. The shared
//! reader loop remains in `input.rs`; this file is still compiled, linted, and
//! used by the real-terminal example.

use super::EventPoller;

pub(super) fn poller_for_reader() -> EventPoller {
    EventPoller {
        poll: Box::new(crossterm::event::poll),
        read: Box::new(crossterm::event::read),
    }
}
