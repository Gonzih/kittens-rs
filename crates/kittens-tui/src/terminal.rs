//! Terminal lifecycle: raw mode and the alternate screen, restored on drop.

use std::io::{self, Write};

use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

/// An RAII terminal session.
///
/// Restores the terminal on drop — including panic unwind — by leaving the
/// alternate screen (when entered) and disabling raw mode. Restoration is
/// best-effort: `Drop` has no channel to report errors, so they are
/// ignored; this matches the priority that a crashing TUI must not leave
/// the shell unusable.
///
/// Teardown ordering (SPEC section 6.7): finish the frame writer *before*
/// dropping the session so drained frames land on the live terminal, and
/// drop the session last.
#[derive(Debug)]
pub struct TerminalSession {
    alternate: bool,
}

impl TerminalSession {
    /// Enables raw mode and optionally enters the alternate screen.
    ///
    /// # Errors
    ///
    /// Propagates terminal backend failures; on the partial-failure path
    /// (alternate screen fails after raw mode succeeded), raw mode is
    /// disabled before returning.
    pub fn begin(alternate_screen: bool) -> io::Result<Self> {
        enable_raw_mode()?;
        if alternate_screen {
            if let Err(error) = crossterm::execute!(io::stdout(), EnterAlternateScreen) {
                let _ = disable_raw_mode();
                return Err(error);
            }
        }
        Ok(Self {
            alternate: alternate_screen,
        })
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.alternate {
            let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
        }
        let _ = disable_raw_mode();
        let _ = io::stdout().flush();
    }
}
