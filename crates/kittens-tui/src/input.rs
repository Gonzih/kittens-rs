//! Owned terminal-input reader thread with a pause/park handshake.
//!
//! The terminal event stream is not admissible as a repeated reactor race —
//! dropping a losing read future can strand its waker — so this module
//! follows the inspected Grok precedent (root SPEC section 20.5): a
//! dedicated thread performs bounded polls and forwards timestamped events
//! through an unbounded channel whose receive side is the K0-admitted
//! [`kittens::source::Mpsc`]. The unbounded channel is deliberate: a
//! synchronous reader thread cannot await capacity.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use kittens::source::{Mpsc, close};

/// One timestamped terminal event.
#[derive(Debug)]
pub struct InputEvent {
    /// When the reader thread observed the event.
    pub at: Instant,
    /// The terminal event.
    pub event: crossterm::event::Event,
}

/// Reactor source type for terminal input.
///
/// The typed `Closed` event means "the reader thread exited" — after
/// shutdown, or after a terminal read error. Treat it as a real event: in a
/// long-lived harness it is usually fatal.
pub type InputSource = Mpsc<InputEvent, close::Emit>;

/// Seam between the reader loop and the terminal backend, so the handshake
/// is deterministically testable without a tty. The crossterm
/// implementation is kept thin.
trait EventPoller: Send {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool>;
    fn read(&mut self) -> io::Result<crossterm::event::Event>;
}

struct CrosstermPoller;

impl EventPoller for CrosstermPoller {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool> {
        crossterm::event::poll(timeout)
    }

    fn read(&mut self) -> io::Result<crossterm::event::Event> {
        crossterm::event::read()
    }
}

#[derive(Default)]
struct Flags {
    paused: AtomicBool,
    parked: AtomicBool,
    shutdown: AtomicBool,
}

/// The owned reader thread.
///
/// Dropping it signals shutdown and joins the thread; the join is bounded
/// by the configured poll interval because every poll is bounded. The input
/// source then yields its typed `Closed` event.
#[derive(Debug)]
pub struct InputReader {
    flags: Arc<Flags>,
    thread: Option<thread::JoinHandle<()>>,
}

impl std::fmt::Debug for Flags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flags")
            .field("paused", &self.paused.load(Ordering::SeqCst))
            .field("parked", &self.parked.load(Ordering::SeqCst))
            .field("shutdown", &self.shutdown.load(Ordering::SeqCst))
            .finish()
    }
}

impl InputReader {
    /// Spawns the owned reader thread over the crossterm backend.
    ///
    /// `poll_interval` bounds how often the thread observes pause and
    /// shutdown; it is an explicit policy value with no default. Smaller
    /// values shorten worst-case shutdown/park latency at the cost of more
    /// wakeups.
    ///
    /// # Errors
    ///
    /// Currently infallible in practice; the `io::Result` is the stable
    /// signature for backends whose setup can fail.
    pub fn spawn(poll_interval: Duration) -> io::Result<(Self, InputSource)> {
        Ok(Self::spawn_with(CrosstermPoller, poll_interval))
    }

    fn spawn_with<P: EventPoller + 'static>(
        mut poller: P,
        poll_interval: Duration,
    ) -> (Self, InputSource) {
        let flags = Arc::new(Flags::default());
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<InputEvent>();
        let thread_flags = Arc::clone(&flags);

        let thread = thread::Builder::new()
            .name("kittens-tui-input".to_owned())
            .spawn(move || {
                loop {
                    if thread_flags.shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                    if thread_flags.paused.load(Ordering::SeqCst) {
                        // Park acknowledgement: the caller may take the tty
                        // once it observes `is_parked()`. The thread keeps
                        // observing shutdown at the poll cadence.
                        thread_flags.parked.store(true, Ordering::SeqCst);
                        thread::sleep(poll_interval);
                        continue;
                    }
                    thread_flags.parked.store(false, Ordering::SeqCst);
                    match poller.poll(poll_interval) {
                        Ok(true) => match poller.read() {
                            Ok(event) => {
                                let delivered = tx
                                    .send(InputEvent {
                                        at: Instant::now(),
                                        event,
                                    })
                                    .is_ok();
                                if !delivered {
                                    // Receiver gone: the reactor no longer
                                    // wants input.
                                    break;
                                }
                            }
                            Err(_) => break,
                        },
                        Ok(false) => {}
                        Err(_) => break,
                    }
                }
            })
            .expect("spawning the input reader thread cannot fail under normal OS limits");

        (
            Self {
                flags,
                thread: Some(thread),
            },
            kittens::source::mpsc(rx, close::Emit),
        )
    }

    /// Requests that the reader stop consuming terminal events.
    ///
    /// The thread acknowledges by parking; wait for [`InputReader::is_parked`]
    /// before handing the tty to another consumer. The full drained
    /// editor/pager handoff choreography is deferred (SPEC section 12); this
    /// primitive is its building block.
    pub fn pause(&self) {
        self.flags.paused.store(true, Ordering::SeqCst);
    }

    /// Whether the reader has acknowledged the pause and parked.
    pub fn is_parked(&self) -> bool {
        self.flags.parked.load(Ordering::SeqCst)
    }

    /// Resumes event consumption.
    pub fn resume(&self) {
        self.flags.paused.store(false, Ordering::SeqCst);
    }
}

impl Drop for InputReader {
    fn drop(&mut self) {
        self.flags.shutdown.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Scripted poller: yields each event once, then reports no events.
    struct ScriptedPoller {
        events: Mutex<Vec<crossterm::event::Event>>,
    }

    impl EventPoller for ScriptedPoller {
        fn poll(&mut self, _timeout: Duration) -> io::Result<bool> {
            Ok(!self.events.lock().expect("test lock").is_empty())
        }

        fn read(&mut self) -> io::Result<crossterm::event::Event> {
            Ok(self.events.lock().expect("test lock").remove(0))
        }
    }

    fn key(code: char) -> crossterm::event::Event {
        crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(code),
            crossterm::event::KeyModifiers::NONE,
        ))
    }

    #[tokio::test]
    async fn events_are_forwarded_and_close_is_typed() {
        use kittens::source::{ChannelEvent, ReactorSource};

        let poller = ScriptedPoller {
            events: Mutex::new(vec![key('a'), key('b')]),
        };
        let (reader, mut source) = InputReader::spawn_with(poller, Duration::from_millis(1));

        let mut received = Vec::new();
        while received.len() < 2 {
            let event = core::future::poll_fn(|cx| source.poll_next(cx)).await;
            if let ChannelEvent::Item(item) = event {
                if let crossterm::event::Event::Key(k) = item.event {
                    received.push(k.code);
                }
            }
        }
        assert_eq!(
            received,
            vec![
                crossterm::event::KeyCode::Char('a'),
                crossterm::event::KeyCode::Char('b')
            ]
        );

        // Drop signals shutdown and joins; the source then yields Closed.
        drop(reader);
        let event = core::future::poll_fn(|cx| source.poll_next(cx)).await;
        assert!(matches!(event, ChannelEvent::Closed));
        assert!(source.is_dormant(), "closed input source is dormant");
    }

    #[test]
    fn pause_parks_and_resume_unparks() {
        let poller = ScriptedPoller {
            events: Mutex::new(Vec::new()),
        };
        let (reader, _source) = InputReader::spawn_with(poller, Duration::from_millis(1));

        reader.pause();
        while !reader.is_parked() {
            thread::sleep(Duration::from_millis(1));
        }

        reader.resume();
        while reader.is_parked() {
            thread::sleep(Duration::from_millis(1));
        }
        // Drop joins within a bounded number of poll intervals.
    }
}
