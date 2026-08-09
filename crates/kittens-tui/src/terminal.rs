//! Terminal lifecycle: raw mode and the alternate screen, with ordered
//! best-effort restoration attempts on drop, including unwind.

use std::cell::RefCell;
use std::fmt;
use std::io::{self, Write};
use std::panic::{RefUnwindSafe, UnwindSafe};

use crossterm::ExecutableCommand;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};

type Output = dyn Write + Send + Sync + UnwindSafe + RefUnwindSafe;
type RawModeOperation = dyn FnMut() -> io::Result<()> + Send + Sync + UnwindSafe + RefUnwindSafe;

/// Private terminal-backend seam. The same methods carry production commands
/// and deterministic no-tty tests, so the seam observes the RAII protocol
/// without becoming its enforcement layer.
struct TerminalBackend {
    output: Box<Output>,
    enable_raw: Box<RawModeOperation>,
    disable_raw: Box<RawModeOperation>,
}

impl TerminalBackend {
    fn production() -> Self {
        Self {
            output: Box::new(io::stdout()),
            enable_raw: Box::new(enable_raw_mode),
            disable_raw: Box::new(disable_raw_mode),
        }
    }

    fn enable_raw_mode(&mut self) -> io::Result<()> {
        (self.enable_raw)()
    }

    fn enter_alternate_screen(&mut self) -> io::Result<()> {
        self.output.execute(EnterAlternateScreen).map(|_| ())
    }

    fn leave_alternate_screen(&mut self) -> io::Result<()> {
        self.output.execute(LeaveAlternateScreen).map(|_| ())
    }

    fn disable_raw_mode(&mut self) -> io::Result<()> {
        (self.disable_raw)()
    }

    fn flush(&mut self) -> io::Result<()> {
        self.output.flush()
    }
}

thread_local! {
    /// One-shot private override used by this module's no-tty lifecycle
    /// oracles. It is compiled in every build so the tested `begin` body is
    /// identical to production; callers cannot reach this private slot.
    static BACKEND_OVERRIDE: RefCell<Option<TerminalBackend>> = const { RefCell::new(None) };
}

fn backend_for_session() -> TerminalBackend {
    // Construct the production backend eagerly even when a unit-test override
    // is present. This keeps the thin production wiring inside the same
    // deterministic coverage boundary without invoking terminal operations.
    let production = TerminalBackend::production();
    BACKEND_OVERRIDE
        .try_with(|slot| slot.borrow_mut().take())
        .ok()
        .flatten()
        .unwrap_or(production)
}

/// An RAII terminal session.
///
/// On drop — including panic unwind — it makes ordered, best-effort attempts
/// to leave the alternate screen (when entered), disable raw mode, and flush.
/// `Drop` has no channel to report restoration failures, so later steps are
/// still attempted after an earlier failure.
///
/// Teardown ordering (SPEC section 6.7): finish the frame writer *before*
/// dropping the session so drained frames land on the live terminal, and
/// drop the session last.
pub struct TerminalSession {
    alternate: bool,
    backend: TerminalBackend,
}

impl fmt::Debug for TerminalSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalSession")
            .field("alternate", &self.alternate)
            .finish_non_exhaustive()
    }
}

impl TerminalSession {
    /// Enables raw mode and optionally enters the alternate screen.
    ///
    /// # Errors
    ///
    /// Propagates terminal backend failures. If alternate-screen entry fails
    /// after raw-mode entry succeeded, a best-effort raw-mode disable is
    /// attempted before the original entry error is returned.
    pub fn begin(alternate_screen: bool) -> io::Result<Self> {
        Self::begin_with(alternate_screen, backend_for_session())
    }

    fn begin_with(alternate_screen: bool, mut backend: TerminalBackend) -> io::Result<Self> {
        backend.enable_raw_mode()?;
        if alternate_screen {
            if let Err(error) = backend.enter_alternate_screen() {
                let _ = backend.disable_raw_mode();
                return Err(error);
            }
        }
        Ok(Self {
            alternate: alternate_screen,
            backend,
        })
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        if self.alternate {
            let _ = self.backend.leave_alternate_screen();
        }
        let _ = self.backend.disable_raw_mode();
        let _ = self.backend.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum Operation {
        EnableRaw,
        DisableRaw,
        Write(Vec<u8>),
        Flush,
    }

    struct RecordingOutput {
        operations: Arc<Mutex<Vec<Operation>>>,
        write_index: usize,
        flush_index: usize,
        fail_write: Option<usize>,
        fail_flush: Option<usize>,
    }

    impl Write for RecordingOutput {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let index = self.write_index;
            self.write_index += 1;
            self.operations
                .lock()
                .expect("operation log lock")
                .push(Operation::Write(bytes.to_vec()));
            if self.fail_write == Some(index) {
                Err(io::Error::other("scripted terminal write failure"))
            } else {
                Ok(bytes.len())
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            let index = self.flush_index;
            self.flush_index += 1;
            self.operations
                .lock()
                .expect("operation log lock")
                .push(Operation::Flush);
            if self.fail_flush == Some(index) {
                Err(io::Error::other("scripted terminal flush failure"))
            } else {
                Ok(())
            }
        }
    }

    fn fake_backend(
        enable_fails: bool,
        disable_fails: bool,
        fail_write: Option<usize>,
        fail_flush: Option<usize>,
    ) -> (TerminalBackend, Arc<Mutex<Vec<Operation>>>) {
        let operations = Arc::new(Mutex::new(Vec::new()));
        let enable_operations = Arc::clone(&operations);
        let disable_operations = Arc::clone(&operations);
        let output_operations = Arc::clone(&operations);

        let enable_raw = move || {
            enable_operations
                .lock()
                .expect("operation log lock")
                .push(Operation::EnableRaw);
            if enable_fails {
                Err(io::Error::other("scripted raw-mode entry failure"))
            } else {
                Ok(())
            }
        };
        let disable_raw = move || {
            disable_operations
                .lock()
                .expect("operation log lock")
                .push(Operation::DisableRaw);
            if disable_fails {
                Err(io::Error::other("scripted raw-mode restore failure"))
            } else {
                Ok(())
            }
        };

        (
            TerminalBackend {
                output: Box::new(RecordingOutput {
                    operations: output_operations,
                    write_index: 0,
                    flush_index: 0,
                    fail_write,
                    fail_flush,
                }),
                enable_raw: Box::new(enable_raw),
                disable_raw: Box::new(disable_raw),
            },
            operations,
        )
    }

    fn install_backend(backend: TerminalBackend) {
        BACKEND_OVERRIDE.with(|slot| {
            let previous = slot.borrow_mut().replace(backend);
            assert!(previous.is_none(), "test backend override is one-shot");
        });
    }

    fn recorded(operations: &Arc<Mutex<Vec<Operation>>>) -> Vec<Operation> {
        operations.lock().expect("operation log lock").clone()
    }

    #[test]
    fn raw_mode_entry_failure_does_not_attempt_false_restoration() {
        let (backend, operations) = fake_backend(true, false, None, None);
        install_backend(backend);

        let error = TerminalSession::begin(false).expect_err("raw-mode entry must fail");
        assert_eq!(error.to_string(), "scripted raw-mode entry failure");
        assert_eq!(recorded(&operations), vec![Operation::EnableRaw]);
    }

    #[test]
    fn alternate_screen_entry_failure_attempts_raw_mode_rollback() {
        let (backend, operations) = fake_backend(false, true, Some(0), None);
        install_backend(backend);

        let error = TerminalSession::begin(true).expect_err("alternate entry must fail");
        assert_eq!(error.to_string(), "scripted terminal write failure");
        assert_eq!(
            recorded(&operations),
            vec![
                Operation::EnableRaw,
                Operation::Write(b"\x1b[?1049h".to_vec()),
                Operation::DisableRaw,
            ]
        );
    }

    #[test]
    fn raw_only_drop_disables_then_flushes_without_leaving_alternate_screen() {
        fn assert_auto_traits<
            T: Send + Sync + Unpin + std::panic::UnwindSafe + std::panic::RefUnwindSafe,
        >() {
        }
        assert_auto_traits::<TerminalSession>();

        let (backend, operations) = fake_backend(false, false, None, None);
        install_backend(backend);
        let session = TerminalSession::begin(false).expect("raw-mode session begins");
        assert_eq!(
            format!("{session:?}"),
            "TerminalSession { alternate: false, .. }"
        );
        drop(session);

        assert_eq!(
            recorded(&operations),
            vec![
                Operation::EnableRaw,
                Operation::DisableRaw,
                Operation::Flush,
            ]
        );
    }

    #[test]
    fn alternate_screen_drop_attempts_restoration_in_protocol_order() {
        let (backend, operations) = fake_backend(false, false, None, None);
        install_backend(backend);
        let session = TerminalSession::begin(true).expect("alternate session begins");
        drop(session);

        assert_eq!(
            recorded(&operations),
            vec![
                Operation::EnableRaw,
                Operation::Write(b"\x1b[?1049h".to_vec()),
                Operation::Flush,
                Operation::Write(b"\x1b[?1049l".to_vec()),
                Operation::Flush,
                Operation::DisableRaw,
                Operation::Flush,
            ]
        );
    }

    #[test]
    fn panic_unwind_attempts_every_restore_step_after_errors() {
        let (backend, operations) = fake_backend(false, true, Some(1), Some(1));
        install_backend(backend);

        let panic = std::panic::catch_unwind(|| {
            let _session = TerminalSession::begin(true).expect("alternate session begins");
            panic!("scripted application panic");
        });
        assert!(
            panic.is_err(),
            "the application panic survives restoration attempts"
        );
        assert_eq!(
            recorded(&operations),
            vec![
                Operation::EnableRaw,
                Operation::Write(b"\x1b[?1049h".to_vec()),
                Operation::Flush,
                Operation::Write(b"\x1b[?1049l".to_vec()),
                Operation::DisableRaw,
                Operation::Flush,
            ]
        );
    }
}
