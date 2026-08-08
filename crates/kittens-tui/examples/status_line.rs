//! Canonical kittens-tui wiring: a live status line with a periodic tick,
//! keystroke echo, and a clean quit path. This is SPEC section 6.7 as a
//! runnable program; run it in a real terminal and press `q` to quit.

#![allow(clippy::ignored_unit_patterns)]

use std::error::Error;
use std::io;
use std::time::Duration;

use tokio::time::Instant;

use kittens::reactor::Control;
use kittens::source::{self, ChannelEvent, OptionalDeadline};
use kittens_tui::{
    FrameWriter, InputEvent, InputReader, InputSource, Presenter, RenderRequest, TerminalSession,
    WriterEvent, WriterHandle, WriterSource,
};
use tokio_util::sync::CancellationToken;

const TICK: Duration = Duration::from_millis(250);

enum Exit {
    Quit,
}

struct Sources {
    stop: source::Cancellation,
    writer_events: WriterSource,
    input: InputSource,
    draw_deadline: OptionalDeadline,
    tick: OptionalDeadline,
}

struct App {
    presenter: Presenter,
    writer: WriterHandle,
    quit: CancellationToken,
    ticks: u64,
    last_key: Option<char>,
    next_tick: Instant,
}

type AppError = Box<dyn Error + Send + Sync>;

// Opaque bytes: kittens-tui never interprets the payload. A component
// library would produce these from its own cell/diff machinery.
fn render_line(ticks: u64, last_key: Option<char>) -> Vec<u8> {
    format!(
        "\x1b[H\x1b[2Kkittens-tui status | ticks: {ticks} | last key: {} | press q to quit\r\n",
        last_key.map_or("-".to_owned(), |c| c.to_string()),
    )
    .into_bytes()
}

impl App {
    fn present(&mut self) -> Result<(), AppError> {
        // Destructured so the draw's exclusive presenter borrow stays
        // field-precise while the writer handle is borrowed shared.
        let Self {
            presenter,
            writer,
            ticks,
            last_key,
            ..
        } = self;
        if let Some(draw) = presenter.try_begin(Instant::now()) {
            let bytes = render_line(*ticks, *last_key);
            draw.commit(writer, bytes)?;
        }
        Ok(())
    }

    async fn run(&mut self, sources: &mut Sources) -> Result<Exit, AppError> {
        kittens::reactor! {
            policy {
                selection: biased;
                required_phases: [initialize, before_poll, after_event];
            }

            initialize {
                self.presenter.request(RenderRequest::FullRepaint);
                self.present()?;
                Ok(())
            }

            before_poll {
                // Dynamic sources are rearmed at the declared loop-top
                // position, not scattered through handlers.
                sources.draw_deadline.set(self.presenter.deadline());
                sources.tick.set(Some(self.next_tick));
                Ok(())
            }

            /// Shutdown leads. The input handler requests it; the reactor
            /// exits through this arm on the next arbitration.
            #[source(stop)]
            #[readiness(quiescent)]
            #[shutdown]
            _ = sources.stop => {
                Ok(Exit::Quit)
            }

            /// Writer acknowledgements unlock the next frame and outrank
            /// everything below shutdown; they yield to buffered input so a
            /// busy frame lane cannot starve the keyboard.
            #[source(writer_events)]
            #[readiness(may_remain_ready)]
            #[yields_to(input, when = buffered)]
            event = sources.writer_events => {
                match event {
                    ChannelEvent::Item(WriterEvent::Written(seq)) => {
                        self.presenter.acknowledge(seq);
                        Ok(Control::Continue)
                    }
                    ChannelEvent::Item(WriterEvent::Failed { error, .. }) => {
                        Err(AppError::from(error))
                    }
                    ChannelEvent::Closed => Err(AppError::from("frame writer exited")),
                }
            }

            /// Terminal input is protected: every may-remain-ready arm above
            /// it yields to it, so keystrokes stay responsive under load.
            #[source(input)]
            #[readiness(may_remain_ready)]
            event = sources.input => {
                match event {
                    ChannelEvent::Item(InputEvent { event, .. }) => {
                        if let crossterm::event::Event::Key(key) = event {
                            if let crossterm::event::KeyCode::Char(c) = key.code {
                                if c == 'q' {
                                    self.quit.cancel();
                                } else {
                                    self.last_key = Some(c);
                                    self.presenter.request(RenderRequest::Dirty);
                                }
                            }
                        }
                        Ok(Control::Continue)
                    }
                    ChannelEvent::Closed => Err(AppError::from("input reader exited")),
                }
            }

            /// The presenter's throttle deadline; dormant unless before_poll
            /// armed it, disarms before firing, cannot hot-loop.
            #[source(draw_deadline)]
            #[readiness(quiescent)]
            #[starvation(allowed, reason = "frame throttle deliberately delays drawing")]
            _ = sources.draw_deadline => {
                self.presenter.on_deadline();
                Ok(Control::Continue)
            }

            /// Periodic animation tick, deliberately last and best effort.
            #[source(tick)]
            #[readiness(quiescent)]
            #[starvation(allowed, reason = "the tick is periodic best-effort work")]
            #[last]
            fired_at = sources.tick => {
                self.ticks += 1;
                self.next_tick = fired_at + TICK;
                self.presenter.request(RenderRequest::Dirty);
                Ok(Control::Continue)
            }

            after_event {
                self.present()?;
                Ok(())
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), AppError> {
    // Construction order: session first so the terminal is in raw mode
    // before the reader consumes events.
    let session = TerminalSession::begin(true)?;
    let (reader, input) = InputReader::spawn(Duration::from_millis(50))?;
    let (writer, handle, writer_events) = FrameWriter::spawn(io::stdout());
    let quit = CancellationToken::new();

    let mut sources = Sources {
        stop: source::cancellation(quit.clone()),
        writer_events,
        input,
        draw_deadline: OptionalDeadline::new(),
        tick: OptionalDeadline::new(),
    };
    let mut app = App {
        presenter: Presenter::new(Duration::from_millis(33)),
        writer: handle,
        quit,
        ticks: 0,
        last_key: None,
        next_tick: Instant::now() + TICK,
    };

    let result = app.run(&mut sources).await;

    // Teardown order per SPEC 6.7: park/stop the reader, drain and join the
    // writer, and only then restore the terminal so drained frames land on
    // a live screen. The session drop is last.
    drop(reader);
    let App { writer: handle, .. } = app;
    writer
        .finish(handle)
        .map_err(|_| AppError::from("frame writer panicked"))?;
    drop(session);

    result.map(|Exit::Quit| ())
}
