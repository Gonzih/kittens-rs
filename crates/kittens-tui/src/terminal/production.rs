//! Live crossterm binding for [`super::TerminalSession`].
//!
//! This file is the explicit coverage exemption named by SPEC section 9. The
//! process-global stdout/raw-mode binding requires an owned live terminal and
//! cannot be exercised honestly by the deterministic no-tty suite. The shared
//! lifecycle protocol remains in `terminal.rs`; this file is still compiled,
//! linted, and used by the real-terminal example.

use std::io;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

use super::TerminalBackend;

pub(super) fn backend_for_session() -> TerminalBackend {
    TerminalBackend {
        output: Box::new(io::stdout()),
        enable_raw: Box::new(enable_raw_mode),
        disable_raw: Box::new(disable_raw_mode),
    }
}
