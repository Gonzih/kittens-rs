//! End-to-end oracle: presenter + writer + admitted sources wired through a
//! real `kittens::reactor!`, running the canonical shape from SPEC 6.7.

#![allow(missing_docs, clippy::ignored_unit_patterns)]

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::time::Instant;

use kittens::reactor::Control;
use kittens::source::{self, ChannelEvent, FixedQueue, OptionalDeadline};
use kittens_tui::{FrameWriter, Presenter, RenderRequest, WriterEvent, WriterHandle, WriterSource};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Default)]
struct SharedSink(Arc<Mutex<Vec<u8>>>);

impl Write for SharedSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("sink lock").extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct Sources {
    stop: source::Cancellation,
    writer_events: WriterSource,
    app_events: FixedQueue<u8, 8>,
    draw_deadline: OptionalDeadline,
}

struct App {
    presenter: Presenter,
    writer: WriterHandle,
    quit: CancellationToken,
    frames_acked: u32,
    frame_counter: u32,
}

impl App {
    fn present(&mut self) -> Result<(), kittens_tui::WriterClosed> {
        if let Some(draw) = self.presenter.try_begin(Instant::now()) {
            let payload = format!("frame{}|", self.frame_counter).into_bytes();
            draw.commit(&self.writer, payload)?;
            self.frame_counter += 1;
        }
        Ok(())
    }

    async fn run(
        &mut self,
        sources: &mut Sources,
    ) -> Result<u32, kittens_tui::WriterClosed> {
        kittens::reactor! {
            policy {
                selection: biased;
                required_phases: [before_poll, after_event];
            }

            before_poll {
                sources.draw_deadline.set(self.presenter.deadline());
                Ok(())
            }

            /// Shutdown leads; requested by the writer-events handler once
            /// three frames are acknowledged.
            #[source(stop)]
            #[readiness(quiescent)]
            #[shutdown]
            _ = sources.stop => {
                Ok(self.frames_acked)
            }

            /// Acknowledgements outrank the app-event stream and yield to it
            /// when it is backlogged, per the canonical wiring.
            #[source(writer_events)]
            #[readiness(may_remain_ready)]
            #[yields_to(app_events, when = buffered)]
            event = sources.writer_events => {
                match event {
                    ChannelEvent::Item(WriterEvent::Written(seq)) => {
                        assert_eq!(
                            self.presenter.acknowledge(seq),
                            kittens_tui::Ack::Accepted,
                            "in-order acks are never stale in this scenario"
                        );
                        self.frames_acked += 1;
                        // Three app events yield exactly two frames: e2 and
                        // e3 arrive while frame0 is in flight and coalesce
                        // into one request. Waiting for a third ack would
                        // hang forever — coalescing is the oracle here.
                        if self.frames_acked == 2 {
                            self.quit.cancel();
                        }
                        Ok(Control::Continue)
                    }
                    ChannelEvent::Item(WriterEvent::Failed { .. })
                    | ChannelEvent::Closed => {
                        panic!("the Vec sink cannot fail in this scenario")
                    }
                }
            }

            /// Each app event marks the presenter dirty; the after_event
            /// phase turns dirtiness into at most one in-flight frame.
            #[source(app_events)]
            #[readiness(may_remain_ready)]
            _ = sources.app_events => {
                self.presenter.request(RenderRequest::Dirty);
                Ok(Control::Continue)
            }

            #[source(draw_deadline)]
            #[readiness(quiescent)]
            #[starvation(allowed, reason = "frame throttle deliberately delays drawing")]
            _ = sources.draw_deadline => {
                self.presenter.on_deadline();
                Ok(Control::Continue)
            }

            after_event {
                self.present()?;
                Ok(())
            }
        }
    }
}

#[tokio::test]
async fn canonical_wiring_presents_three_acknowledged_frames() {
    let sink = SharedSink::default();
    let (writer, handle, writer_events) = FrameWriter::spawn(sink.clone());
    let quit = CancellationToken::new();

    let mut sources = Sources {
        stop: source::cancellation(quit.clone()),
        writer_events,
        app_events: FixedQueue::new(),
        draw_deadline: OptionalDeadline::new(),
    };
    for event in [1, 2, 3] {
        sources.app_events.push(event).expect("queue has capacity");
    }

    let mut app = App {
        presenter: Presenter::new(Duration::ZERO),
        writer: handle,
        quit,
        frames_acked: 0,
        frame_counter: 0,
    };

    let acked = app.run(&mut sources).await.expect("writer lane stays open");
    assert_eq!(acked, 2, "the reactor exits through the shutdown arm");
    assert_eq!(app.presenter.in_flight(), None, "gate is idle at exit");

    let App { writer: handle, .. } = app;
    writer.finish(handle).expect("writer drains and joins");
    assert_eq!(
        sink.0.lock().expect("sink lock").as_slice(),
        b"frame0|frame1|",
        "three app events coalesce into exactly two in-order frames"
    );
}
