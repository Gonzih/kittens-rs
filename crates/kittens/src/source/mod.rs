//! Persistent sources admitted to [`crate::reactor!`].
//!
//! Source admission establishes one narrow property: when another source wins
//! an arbitration, the stored source retains the operation or event state
//! promised by its adapter. It does not promise rollback, repeat safety, async
//! cleanup, or event delivery after the whole source/reactor is dropped.

use core::task::{Context, Poll};

mod sealed {
    pub trait Sealed {}
    pub trait ReadinessSealed {}
}

/// Sealed readiness markers used by declarations and source adapters.
pub mod readiness {
    /// A source that can produce consecutive ready items without explicit
    /// rearming.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct MayRemainReady;

    /// A source that cannot produce another item until a new external change
    /// or explicit rearming operation.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct Quiescent;
}

impl sealed::ReadinessSealed for readiness::MayRemainReady {}
impl sealed::ReadinessSealed for readiness::Quiescent {}

/// A sealed readiness marker.
pub trait Readiness: sealed::ReadinessSealed {}

impl Readiness for readiness::MayRemainReady {}
impl Readiness for readiness::Quiescent {}

/// A persistent source admitted to generated reactor arbitration.
///
/// Implementations are sealed because this trait represents a reviewed
/// semantic contract, not only a method signature. Every Kittens adapter also
/// documents what happens when the whole adapter is dropped.
#[diagnostic::on_unimplemented(
    message = "source type `{Self}` is not admitted for repeated reactor selection",
    label = "this source does not establish preservation when another source wins",
    note = "use a reviewed retained/latching adapter, or isolate the producer behind an explicitly owned signal/channel; cleanup on drop is not the same contract"
)]
pub trait ReactorSource: sealed::Sealed + Unpin {
    /// The owned event delivered to a handler.
    type Item;
    /// Conservative readiness behavior used by starvation validation.
    type Readiness: Readiness;

    /// Polls the persistent source for its next owned event.
    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Self::Item>;
}

/// Exact-readiness assertion implemented automatically for admitted sources.
#[doc(hidden)]
pub trait HasReadiness<R: Readiness>: ReactorSource {}

impl<S, R> HasReadiness<R> for S
where
    R: Readiness,
    S: ReactorSource<Readiness = R>,
{
}

/// Result of one nonblocking, macro-managed drain probe.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TryNext<T> {
    /// One immediately available item.
    Item(T),
    /// The source remains live but has no immediately available item.
    Empty,
    /// The source is dormant or terminally closed.
    Dormant,
}

/// Capability for allocation-free, nonblocking per-item draining.
#[diagnostic::on_unimplemented(
    message = "source type `{Self}` does not support macro-managed draining",
    note = "remove `#[drain(...)]` or use a stable installed source with a reviewed nonblocking drain operation"
)]
pub trait DrainableSource: ReactorSource {
    /// Attempts to take one immediately available item without registering a
    /// wake or awaiting another event.
    fn try_next(&mut self) -> TryNext<Self::Item>;
}

/// Capability used by a higher source's buffered-yield relationship.
#[diagnostic::on_unimplemented(
    message = "source type `{Self}` cannot be a buffered-yield target",
    note = "move the protected source above the firehose, or use an adapter with a reviewed observational backlog probe"
)]
pub trait BacklogSource: ReactorSource {
    /// Returns whether the adapter already owns an immediately selectable
    /// item. This is an observational hint and may become stale immediately.
    fn has_backlog(&self) -> bool;
}

/// Close policies for channel adapters.
pub mod close {
    /// Silently become dormant when the producer closes.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct Dormant;

    /// Emit one [`super::ChannelEvent::Closed`] event, then become dormant.
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    pub struct Emit;
}

/// An item or the single typed close event from a channel source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChannelEvent<T> {
    /// A producer item.
    Item(T),
    /// The producer closed. This variant is emitted at most once.
    Closed,
}

/// Internal, sealed mapping from a channel close policy to its item type.
#[doc(hidden)]
pub trait ChannelClosePolicy<T>: sealed::Sealed {
    /// Item exposed by the channel adapter.
    type Item;
    /// Whether producer closure is an immediately selectable typed event.
    const EMITS_CLOSE: bool;

    /// Maps a producer item.
    fn map_item(item: T) -> Self::Item;
    /// Produces the one close event, when this policy exposes one.
    fn close_event() -> Option<Self::Item>;
}

impl sealed::Sealed for close::Dormant {}
impl sealed::Sealed for close::Emit {}

impl<T> ChannelClosePolicy<T> for close::Dormant {
    type Item = T;
    const EMITS_CLOSE: bool = false;

    fn map_item(item: T) -> Self::Item {
        item
    }

    fn close_event() -> Option<Self::Item> {
        None
    }
}

impl<T> ChannelClosePolicy<T> for close::Emit {
    type Item = ChannelEvent<T>;
    const EMITS_CLOSE: bool = true;

    fn map_item(item: T) -> Self::Item {
        ChannelEvent::Item(item)
    }

    fn close_event() -> Option<Self::Item> {
        Some(ChannelEvent::Closed)
    }
}

/// Error returned when an already-armed single-slot source is armed again.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlreadyArmed<T> {
    item: T,
}

impl<T> AlreadyArmed<T> {
    /// Returns the item that was not installed.
    pub fn into_inner(self) -> T {
        self.item
    }
}

/// A locally armed, allocation-free single-event latch.
///
/// `Latched` is useful for no-std orchestration and host-modeled interrupt
/// fixtures. An event is installed before arbitration through exclusive access,
/// so it survives both being polled pending before another source wins and an
/// earlier source winning before this source is polled. Dormant polling returns
/// `Pending` without waking itself. There is no concurrent arming handle; code
/// that arms from another execution context must provide a reviewed wake-aware
/// adapter instead.
///
/// Dropping the adapter drops any unhandled item synchronously.
#[derive(Debug, Default)]
pub struct Latched<T> {
    item: Option<T>,
}

impl<T> Unpin for Latched<T> {}

impl<T> Latched<T> {
    /// Creates a dormant latch.
    pub const fn new() -> Self {
        Self { item: None }
    }

    /// Installs an event.
    ///
    /// # Errors
    ///
    /// Returns [`AlreadyArmed`] containing `item` when an unhandled event is
    /// already latched.
    pub fn arm(&mut self, item: T) -> Result<(), AlreadyArmed<T>> {
        if self.item.is_some() {
            Err(AlreadyArmed { item })
        } else {
            self.item = Some(item);
            Ok(())
        }
    }

    /// Removes and returns a latched event without handling it.
    pub fn disarm(&mut self) -> Option<T> {
        self.item.take()
    }

    /// Reports whether no event is latched.
    pub const fn is_dormant(&self) -> bool {
        self.item.is_none()
    }
}

impl<T> sealed::Sealed for Latched<T> {}

impl<T> ReactorSource for Latched<T> {
    type Item = T;
    type Readiness = readiness::Quiescent;

    fn poll_next(&mut self, _cx: &mut Context<'_>) -> Poll<Self::Item> {
        match self.item.take() {
            Some(item) => Poll::Ready(item),
            None => Poll::Pending,
        }
    }
}

impl<T> BacklogSource for Latched<T> {
    fn has_backlog(&self) -> bool {
        self.item.is_some()
    }
}

/// Error returned when a fixed-capacity source is full.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Full<T> {
    item: T,
}

impl<T> Full<T> {
    /// Returns the item that did not fit.
    pub fn into_inner(self) -> T {
        self.item
    }
}

/// A locally filled, fixed-capacity, allocation-free source.
///
/// The source never wakes itself. Producers outside the reactor task require a
/// different, wake-aware adapter. Dropping it synchronously drops buffered
/// items.
#[derive(Debug)]
pub struct FixedQueue<T, const N: usize> {
    slots: [Option<T>; N],
    head: usize,
    len: usize,
}

impl<T, const N: usize> Unpin for FixedQueue<T, N> {}

impl<T, const N: usize> FixedQueue<T, N> {
    /// Creates an empty source.
    pub fn new() -> Self {
        Self {
            slots: core::array::from_fn(|_| None),
            head: 0,
            len: 0,
        }
    }

    /// Adds one item.
    ///
    /// # Errors
    ///
    /// Returns [`Full`] containing `item` when the fixed capacity is
    /// exhausted (including a zero-capacity source).
    pub fn push(&mut self, item: T) -> Result<(), Full<T>> {
        if self.len == N {
            return Err(Full { item });
        }
        if N == 0 {
            return Err(Full { item });
        }
        let tail = (self.head + self.len) % N;
        self.slots[tail] = Some(item);
        self.len += 1;
        Ok(())
    }

    /// Returns the number of buffered items.
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns whether no items are buffered.
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn pop(&mut self) -> Option<T> {
        if self.len == 0 || N == 0 {
            return None;
        }
        let item = self.slots[self.head].take();
        self.head = (self.head + 1) % N;
        self.len -= 1;
        item
    }
}

impl<T, const N: usize> Default for FixedQueue<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> sealed::Sealed for FixedQueue<T, N> {}

impl<T, const N: usize> ReactorSource for FixedQueue<T, N> {
    type Item = T;
    type Readiness = readiness::MayRemainReady;

    fn poll_next(&mut self, _cx: &mut Context<'_>) -> Poll<Self::Item> {
        match self.pop() {
            Some(item) => Poll::Ready(item),
            None => Poll::Pending,
        }
    }
}

impl<T, const N: usize> DrainableSource for FixedQueue<T, N> {
    fn try_next(&mut self) -> TryNext<Self::Item> {
        match self.pop() {
            Some(item) => TryNext::Item(item),
            None => TryNext::Empty,
        }
    }
}

impl<T, const N: usize> BacklogSource for FixedQueue<T, N> {
    fn has_backlog(&self) -> bool {
        !self.is_empty()
    }
}

#[cfg(all(feature = "tokio", not(target_os = "none")))]
mod tokio_impl;

#[cfg(all(feature = "tokio", not(target_os = "none")))]
pub use tokio_impl::{
    AlreadyArmedReceiver, Cancellation, Mpsc, MpscReceiver, OneShot, OptionalDeadline,
    OptionalMpsc, OptionalOneShot, cancellation, mpsc, one_shot,
};
