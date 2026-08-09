//! Owned terminal-input reader thread with a pause/park handshake.
//!
//! The terminal event stream is not admissible as a repeated reactor race —
//! dropping a losing read future can strand its waker — so this module
//! follows the inspected Grok precedent (root SPEC section 20.5): a
//! dedicated thread performs bounded polls and forwards timestamped events
//! through an unbounded channel whose receive side is the K0-admitted
//! [`kittens::source::Mpsc`]. The unbounded channel is deliberate: a
//! synchronous reader thread cannot await capacity.

#[cfg(test)]
use std::cell::RefCell;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use kittens::source::{Mpsc, close};

#[cfg(not(test))]
mod production;
#[cfg(not(test))]
use production::poller_for_reader;

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

type PollOperation = dyn FnMut(Duration) -> io::Result<bool> + Send;
type ReadOperation = dyn FnMut() -> io::Result<crossterm::event::Event> + Send;

/// Function-backed seam between the owned loop and terminal event I/O. The
/// production constructor stores crossterm's functions directly; deterministic
/// tests replace the operations without adding a second implementation whose
/// behavior could drift.
struct EventPoller {
    poll: Box<PollOperation>,
    read: Box<ReadOperation>,
}

impl EventPoller {
    fn poll(&mut self, timeout: Duration) -> io::Result<bool> {
        (self.poll)(timeout)
    }

    fn read(&mut self) -> io::Result<crossterm::event::Event> {
        (self.read)()
    }
}

#[cfg(test)]
thread_local! {
    /// One-shot private override used by no-tty constructor oracles. It is
    /// compiled only for unit tests; the owned reader loop is shared with
    /// production and only the final live-crossterm binding is replaced.
    static POLLER_OVERRIDE: RefCell<Option<EventPoller>> = const { RefCell::new(None) };
}

#[cfg(test)]
fn poller_for_reader() -> EventPoller {
    POLLER_OVERRIDE.with(|slot| {
        slot.borrow_mut()
            .take()
            .expect("input tests install one poller before public spawn")
    })
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
        Ok(Self::spawn_with(poller_for_reader(), poll_interval))
    }

    fn spawn_with(mut poller: EventPoller, poll_interval: Duration) -> (Self, InputSource) {
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
        // `spawn_with` is the only constructor and `Drop` is the only path
        // that moves this handle. The `Option` enables that move; an empty live
        // reader is not a constructible handshake state.
        let thread = self
            .thread
            .take()
            .expect("InputReader owns its thread until drop");
        let _ = thread.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Mutex, mpsc as std_mpsc};

    const TEST_TIMEOUT: Duration = Duration::from_secs(2);

    enum PollStep {
        Ready,
        Idle,
        Error,
        GatedReady {
            entered: std_mpsc::SyncSender<()>,
            release: std_mpsc::Receiver<()>,
        },
    }

    enum ReadStep {
        Event(crossterm::event::Event),
        Error,
    }

    struct ScriptState {
        poll_steps: VecDeque<PollStep>,
        read_steps: VecDeque<ReadStep>,
        timeouts: Vec<Duration>,
        reads: usize,
    }

    fn scripted_poller(
        poll_steps: Vec<PollStep>,
        read_steps: Vec<ReadStep>,
    ) -> (EventPoller, Arc<Mutex<ScriptState>>) {
        let state = Arc::new(Mutex::new(ScriptState {
            poll_steps: poll_steps.into(),
            read_steps: read_steps.into(),
            timeouts: Vec::new(),
            reads: 0,
        }));
        let poll_state = Arc::clone(&state);
        let read_state = Arc::clone(&state);

        let poll = move |timeout| {
            let step = {
                let mut state = poll_state.lock().expect("script lock");
                state.timeouts.push(timeout);
                state.poll_steps.pop_front().unwrap_or(PollStep::Idle)
            };
            match step {
                PollStep::Ready => Ok(true),
                PollStep::Idle => {
                    thread::sleep(timeout);
                    Ok(false)
                }
                PollStep::Error => Err(io::Error::other("scripted poll failure")),
                PollStep::GatedReady { entered, release } => {
                    entered.send(()).expect("test still waits for poll entry");
                    release
                        .recv_timeout(TEST_TIMEOUT)
                        .expect("test releases the poller before the deadline");
                    Ok(true)
                }
            }
        };
        let read = move || {
            let step = {
                let mut state = read_state.lock().expect("script lock");
                state.reads += 1;
                state
                    .read_steps
                    .pop_front()
                    .expect("every ready poll has a scripted read")
            };
            match step {
                ReadStep::Event(event) => Ok(event),
                ReadStep::Error => Err(io::Error::other("scripted read failure")),
            }
        };

        (
            EventPoller {
                poll: Box::new(poll),
                read: Box::new(read),
            },
            state,
        )
    }

    fn install_poller(poller: EventPoller) {
        POLLER_OVERRIDE.with(|slot| {
            let mut slot = slot.borrow_mut();
            assert!(slot.is_none(), "test poller override is one-shot");
            *slot = Some(poller);
        });
    }

    fn wait_for_parked_after(
        reader: &InputReader,
        expected: bool,
        release: Option<&std_mpsc::SyncSender<()>>,
    ) {
        let deadline = Instant::now() + TEST_TIMEOUT;
        let mut release = release;
        loop {
            if reader.is_parked() == expected {
                return;
            }
            if let Some(release) = release.take() {
                release
                    .send(())
                    .expect("reader still waits for the gated poll");
            }
            assert!(
                Instant::now() < deadline,
                "reader did not publish parked={expected} before the deadline"
            );
            thread::sleep(Duration::from_millis(1));
        }
    }

    struct ReaderExitProbe {
        reads: Arc<std::sync::atomic::AtomicUsize>,
        exited: std_mpsc::SyncSender<()>,
    }

    impl Drop for ReaderExitProbe {
        fn drop(&mut self) {
            let _ = self.exited.send(());
        }
    }

    fn key(code: char) -> crossterm::event::Event {
        crossterm::event::Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char(code),
            crossterm::event::KeyModifiers::NONE,
        ))
    }

    fn is_closed(event: &kittens::source::ChannelEvent<InputEvent>) -> bool {
        match event {
            kittens::source::ChannelEvent::Item(_) => false,
            kittens::source::ChannelEvent::Closed => true,
        }
    }

    #[tokio::test]
    async fn events_are_forwarded_and_close_is_typed() {
        use kittens::source::{ChannelEvent, ReactorSource};

        let before_spawn = Instant::now();
        let (poller, _state) = scripted_poller(
            vec![PollStep::Ready, PollStep::Ready],
            vec![ReadStep::Event(key('a')), ReadStep::Event(key('b'))],
        );
        let (reader, mut source) = InputReader::spawn_with(poller, Duration::from_millis(1));

        let mut received = Vec::new();
        let mut reader = Some(reader);
        loop {
            let event = core::future::poll_fn(|cx| source.poll_next(cx)).await;
            let closed = is_closed(&event);
            match event {
                ChannelEvent::Item(item) => {
                    assert!(!closed, "an item is not the typed close event");
                    received.push(item);
                    if received.len() == 2 {
                        // Drop signals shutdown and joins; polling once more
                        // must yield the reader's typed close event.
                        drop(reader.take());
                    }
                }
                ChannelEvent::Closed => {
                    assert!(closed, "reader exit is represented by typed close");
                    break;
                }
            }
        }
        assert_eq!(
            received.iter().map(|item| &item.event).collect::<Vec<_>>(),
            vec![&key('a'), &key('b')]
        );
        assert!(received.iter().all(|item| item.at >= before_spawn));
        assert!(
            received.windows(2).all(|pair| pair[0].at <= pair[1].at),
            "events retain reader observation order"
        );
        assert!(source.is_dormant(), "closed input source is dormant");
    }

    #[test]
    fn pause_parks_and_resume_unparks() {
        let (entered_tx, entered_rx) = std_mpsc::sync_channel(1);
        let (release_tx, release_rx) = std_mpsc::sync_channel(1);
        let (poller, _state) = scripted_poller(
            vec![PollStep::GatedReady {
                entered: entered_tx,
                release: release_rx,
            }],
            vec![ReadStep::Event(key('p'))],
        );
        let (reader, _source) = InputReader::spawn_with(poller, Duration::from_millis(1));
        entered_rx
            .recv_timeout(TEST_TIMEOUT)
            .expect("reader enters its bounded poll");
        assert!(
            format!("{reader:?}").contains("paused: false"),
            "Debug is a nonblocking snapshot of the handoff flags"
        );

        reader.pause();
        assert!(
            !reader.is_parked(),
            "a pause request is not a park acknowledgement"
        );
        let requested = format!("{reader:?}");
        assert!(requested.contains("paused: true"));
        assert!(requested.contains("parked: false"));

        wait_for_parked_after(&reader, true, Some(&release_tx));
        let parked = format!("{reader:?}");
        assert!(parked.contains("paused: true"));
        assert!(parked.contains("parked: true"));

        reader.resume();
        wait_for_parked_after(&reader, false, None);
        let flags = Arc::clone(&reader.flags);
        drop(reader);
        assert!(
            format!("{flags:?}").contains("shutdown: true"),
            "Drop publishes shutdown before joining the reader"
        );
    }

    #[tokio::test]
    async fn public_spawn_forwards_poll_error_as_typed_close() {
        use kittens::source::ReactorSource;

        let interval = Duration::from_millis(7);
        let (poller, state) = scripted_poller(vec![PollStep::Error], Vec::new());
        install_poller(poller);
        let (reader, mut source) =
            InputReader::spawn(interval).expect("reader setup is infallible");

        let event = core::future::poll_fn(|cx| source.poll_next(cx)).await;
        assert!(is_closed(&event));
        assert!(source.is_dormant());
        let state = state.lock().expect("script lock");
        assert_eq!(state.timeouts, vec![interval], "poll interval is forwarded");
        assert_eq!(state.reads, 0, "a failed poll never attempts a read");
        drop(state);
        drop(reader);
    }

    #[tokio::test]
    async fn read_error_exits_with_typed_close() {
        use kittens::source::ReactorSource;

        let (poller, state) = scripted_poller(vec![PollStep::Ready], vec![ReadStep::Error]);
        let (reader, mut source) = InputReader::spawn_with(poller, Duration::from_millis(1));

        let event = core::future::poll_fn(|cx| source.poll_next(cx)).await;
        assert!(is_closed(&event));
        assert!(source.is_dormant());
        assert_eq!(state.lock().expect("script lock").reads, 1);
        drop(reader);
    }

    #[test]
    fn dropping_source_stops_the_reader_when_delivery_is_rejected() {
        let (entered_tx, entered_rx) = std_mpsc::sync_channel(1);
        let (release_tx, release_rx) = std_mpsc::sync_channel(1);
        let (exited_tx, exited_rx) = std_mpsc::sync_channel(1);
        let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let probe = ReaderExitProbe {
            reads: Arc::clone(&reads),
            exited: exited_tx,
        };
        let poller = EventPoller {
            poll: Box::new(move |_| {
                entered_tx.send(()).expect("test waits for poll entry");
                release_rx
                    .recv_timeout(TEST_TIMEOUT)
                    .expect("test releases the poller before the deadline");
                Ok(true)
            }),
            read: Box::new(move || {
                probe.reads.fetch_add(1, Ordering::SeqCst);
                Ok(key('x'))
            }),
        };
        let (reader, source) = InputReader::spawn_with(poller, Duration::from_millis(1));

        entered_rx
            .recv_timeout(TEST_TIMEOUT)
            .expect("reader entered its bounded poll");
        drop(source);
        release_tx.send(()).expect("release the scripted event");
        exited_rx
            .recv_timeout(TEST_TIMEOUT)
            .expect("receiver rejection terminates the reader");
        assert_eq!(
            reads.load(Ordering::SeqCst),
            1,
            "the rejected delivery terminates immediately after one read"
        );
        drop(reader);
    }
}
