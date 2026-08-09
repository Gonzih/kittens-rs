//! Writer-protocol oracles: order, flush-per-frame, ack-per-frame,
//! drain-on-close, typed failure then exit.

#![allow(missing_docs)]

use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::time::Instant;

use kittens::source::{ChannelEvent, ReactorSource};
use kittens_tui::{FrameWriter, Presenter, RenderRequest, WriterEvent};

#[derive(Clone, Default)]
struct SharedSink {
    bytes: Arc<Mutex<Vec<u8>>>,
    flushes: Arc<Mutex<usize>>,
    fail_writes: bool,
}

impl Write for SharedSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.fail_writes {
            return Err(io::Error::other("terminal went away"));
        }
        self.bytes.lock().expect("sink lock").extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        *self.flushes.lock().expect("sink lock") += 1;
        Ok(())
    }
}

#[tokio::test]
async fn frames_write_in_order_and_each_is_acknowledged() {
    let sink = SharedSink::default();
    let (writer, handle, mut events) = FrameWriter::spawn(sink.clone());
    let mut presenter = Presenter::new(Duration::ZERO);
    let now = Instant::now();

    let mut committed = Vec::new();
    for payload in [&b"one|"[..], &b"two|"[..], &b"three|"[..]] {
        presenter.request(RenderRequest::Dirty);
        let seq = presenter
            .try_begin(now)
            .expect("gate open")
            .commit(&handle, payload.to_vec())
            .expect("lane open");
        committed.push(seq);
        let event = core::future::poll_fn(|cx| events.poll_next(cx)).await;
        match event {
            ChannelEvent::Item(WriterEvent::Written(acked)) => {
                assert_eq!(acked, seq, "each frame is acknowledged with its sequence");
                assert_eq!(presenter.acknowledge(acked), kittens_tui::Ack::Accepted);
            }
            other => panic!("expected Written, got {other:?}"),
        }
    }

    assert!(
        committed.windows(2).all(|w| w[0] < w[1]),
        "sequences are strictly monotonic"
    );
    assert!(
        committed.windows(2).all(|w| w[0].get() < w[1].get()),
        "raw sequence values preserve the documented monotonic ordering"
    );
    writer.finish(handle).expect("writer joins");
    assert_eq!(
        sink.bytes.lock().expect("sink lock").as_slice(),
        b"one|two|three|",
        "frames land on the sink in submission order"
    );
    assert!(
        *sink.flushes.lock().expect("sink lock") >= 3,
        "every frame is flushed"
    );

    // The writer exited; the event lane closes with a typed event.
    let event = core::future::poll_fn(|cx| events.poll_next(cx)).await;
    assert!(matches!(event, ChannelEvent::Closed));
}

#[tokio::test]
async fn accepted_frames_drain_when_the_lane_closes() {
    let sink = SharedSink::default();
    let (writer, handle, mut events) = FrameWriter::spawn(sink.clone());
    let mut presenter = Presenter::new(Duration::ZERO);
    let now = Instant::now();

    // Submit two frames and immediately close the lane; the writer must
    // still write and acknowledge both before exiting.
    presenter.request(RenderRequest::Dirty);
    let first = presenter
        .try_begin(now)
        .expect("gate open")
        .commit(&handle, b"a".to_vec())
        .expect("lane open");
    // Acknowledge from the known sequence (the writer's event may not have
    // arrived yet; the gate only compares sequences) so the second commit
    // is legal.
    assert_eq!(presenter.acknowledge(first), kittens_tui::Ack::Accepted);
    presenter.request(RenderRequest::Dirty);
    let second = presenter
        .try_begin(now)
        .expect("gate reopened")
        .commit(&handle, b"b".to_vec())
        .expect("lane open");
    assert!(second > first);

    writer.finish(handle).expect("writer drains then joins");
    assert_eq!(
        sink.bytes.lock().expect("sink lock").as_slice(),
        b"ab",
        "accepted frames are never silently discarded"
    );

    // Both acknowledgements were emitted before the typed close.
    let mut written = Vec::new();
    loop {
        match core::future::poll_fn(|cx| events.poll_next(cx)).await {
            ChannelEvent::Item(WriterEvent::Written(seq)) => written.push(seq),
            ChannelEvent::Item(other) => panic!("unexpected event {other:?}"),
            ChannelEvent::Closed => break,
        }
    }
    assert_eq!(written, vec![first, second]);
}

#[tokio::test]
async fn cancelling_the_event_consumer_does_not_discard_accepted_output() {
    let sink = SharedSink::default();
    let (writer, handle, events) = FrameWriter::spawn(sink.clone());
    let mut presenter = Presenter::new(Duration::ZERO);

    // A reactor may disappear before the owned writer does. Its event lane is
    // then closed, but the accepted-frame drain obligation still controls.
    drop(events);
    presenter.request(RenderRequest::Dirty);
    presenter
        .try_begin(Instant::now())
        .expect("gate open")
        .commit(&handle, b"survives-cancel".to_vec())
        .expect("frame lane remains open");
    writer.finish(handle).expect("writer drains then joins");

    assert_eq!(
        sink.bytes.lock().expect("sink lock").as_slice(),
        b"survives-cancel",
        "event-consumer cancellation cannot discard accepted bytes"
    );
    assert_eq!(
        *sink.flushes.lock().expect("sink lock"),
        1,
        "the unacknowledgeable frame is still flushed"
    );
}

#[tokio::test]
async fn dropping_a_live_writer_owner_detaches_until_the_handle_closes() {
    let sink = SharedSink::default();
    let (writer, handle, mut events) = FrameWriter::spawn(sink.clone());
    let mut presenter = Presenter::new(Duration::ZERO);

    // This is the documented non-canonical teardown path: dropping the owner
    // must not cancel a thread that still has a live submission lane.
    drop(writer);
    presenter.request(RenderRequest::Dirty);
    let seq = presenter
        .try_begin(Instant::now())
        .expect("gate open")
        .commit(&handle, b"after-owner-drop".to_vec())
        .expect("detached writer still accepts frames");
    match core::future::poll_fn(|cx| events.poll_next(cx)).await {
        ChannelEvent::Item(WriterEvent::Written(written)) => assert_eq!(written, seq),
        other => panic!("expected the detached writer acknowledgement, got {other:?}"),
    }

    drop(handle);
    let closed = core::future::poll_fn(|cx| events.poll_next(cx)).await;
    assert!(matches!(closed, ChannelEvent::Closed));
    assert_eq!(
        sink.bytes.lock().expect("sink lock").as_slice(),
        b"after-owner-drop",
        "the live detached writer drains before self-termination"
    );
}

#[tokio::test]
async fn write_failure_is_one_typed_event_then_exit() {
    let sink = SharedSink {
        fail_writes: true,
        ..SharedSink::default()
    };
    let (writer, handle, mut events) = FrameWriter::spawn(sink);
    let mut presenter = Presenter::new(Duration::ZERO);
    presenter.request(RenderRequest::Dirty);
    let seq = presenter
        .try_begin(Instant::now())
        .expect("gate open")
        .commit(&handle, b"doomed".to_vec())
        .expect("the channel accepts the frame before the writer fails");

    match core::future::poll_fn(|cx| events.poll_next(cx)).await {
        ChannelEvent::Item(WriterEvent::Failed { seq: failed, .. }) => {
            assert_eq!(failed, seq, "the failure names the failing frame");
        }
        other => panic!("expected Failed, got {other:?}"),
    }
    let event = core::future::poll_fn(|cx| events.poll_next(cx)).await;
    assert!(
        matches!(event, ChannelEvent::Closed),
        "the writer exits after one typed failure"
    );
    writer.finish(handle).expect("failed writer still joins");
}
