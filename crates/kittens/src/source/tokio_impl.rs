//! Tokio-backed source adapters.

use alloc::boxed::Box;
use core::future::Future;
use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::time::{Instant, Sleep};
use tokio_util::sync::{CancellationToken, WaitForCancellationFutureOwned};

use super::{
    AlreadyArmed, BacklogSource, ChannelClosePolicy, DrainableSource, ReactorSource, TryNext,
    readiness, sealed,
};

/// Either Tokio bounded or unbounded mpsc receiver, normalized behind one
/// statically dispatched adapter type.
#[derive(Debug)]
pub enum MpscReceiver<T> {
    /// A bounded receiver.
    Bounded(mpsc::Receiver<T>),
    /// An unbounded receiver.
    Unbounded(mpsc::UnboundedReceiver<T>),
}

impl<T> From<mpsc::Receiver<T>> for MpscReceiver<T> {
    fn from(receiver: mpsc::Receiver<T>) -> Self {
        Self::Bounded(receiver)
    }
}

impl<T> From<mpsc::UnboundedReceiver<T>> for MpscReceiver<T> {
    fn from(receiver: mpsc::UnboundedReceiver<T>) -> Self {
        Self::Unbounded(receiver)
    }
}

impl<T> MpscReceiver<T> {
    fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
        match self {
            Self::Bounded(receiver) => receiver.poll_recv(cx),
            Self::Unbounded(receiver) => receiver.poll_recv(cx),
        }
    }

    fn try_recv(&mut self) -> Result<T, TryRecvError> {
        match self {
            Self::Bounded(receiver) => receiver.try_recv(),
            Self::Unbounded(receiver) => receiver.try_recv(),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            Self::Bounded(receiver) => receiver.is_empty(),
            Self::Unbounded(receiver) => receiver.is_empty(),
        }
    }

    fn is_closed(&self) -> bool {
        match self {
            Self::Bounded(receiver) => receiver.is_closed(),
            Self::Unbounded(receiver) => receiver.is_closed(),
        }
    }
}

/// A persistent Tokio mpsc source with a static close policy.
///
/// Tokio's `poll_recv` keeps channel state in the receiver, so another branch
/// winning does not lose buffered items. The adapter owns the receiver and on
/// producer closure either becomes dormant or emits one typed close event and
/// then becomes dormant. Tokio mpsc receive polling participates in Tokio's
/// cooperative scheduling budget; synchronous drain probes do not add a
/// Kittens budget wrapper. A budget-induced empty/`Pending` result ends the
/// current Kittens service opportunity.
///
/// Dropping this adapter closes the receive side and drops unread buffered
/// items according to Tokio's receiver contract. No asynchronous cleanup or
/// delivery after drop is promised.
#[derive(Debug)]
pub struct Mpsc<T, C> {
    receiver: Option<MpscReceiver<T>>,
    close: PhantomData<C>,
}

impl<T, C> Unpin for Mpsc<T, C> {}

/// Creates a persistent mpsc source.
pub fn mpsc<T, C>(receiver: impl Into<MpscReceiver<T>>, _close: C) -> Mpsc<T, C>
where
    C: ChannelClosePolicy<T>,
{
    Mpsc {
        receiver: Some(receiver.into()),
        close: PhantomData,
    }
}

impl<T, C> sealed::Sealed for Mpsc<T, C> where C: ChannelClosePolicy<T> {}

impl<T, C> Mpsc<T, C>
where
    C: ChannelClosePolicy<T>,
{
    /// Returns whether the source has transitioned permanently dormant.
    pub const fn is_dormant(&self) -> bool {
        self.receiver.is_none()
    }

    fn close(&mut self) -> Option<C::Item> {
        self.receiver = None;
        C::close_event()
    }
}

impl<T, C> ReactorSource for Mpsc<T, C>
where
    C: ChannelClosePolicy<T>,
{
    type Item = C::Item;
    type Readiness = readiness::MayRemainReady;

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Self::Item> {
        let Some(receiver) = self.receiver.as_mut() else {
            return Poll::Pending;
        };
        match receiver.poll_recv(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(C::map_item(item)),
            // Each sealed policy fixes one outcome statically. `map_or`
            // expresses that choice without a runtime match whose opposite
            // arm is impossible for the monomorphized policy.
            Poll::Ready(None) => self.close().map_or(Poll::Pending, Poll::Ready),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T, C> DrainableSource for Mpsc<T, C>
where
    C: ChannelClosePolicy<T>,
{
    fn try_next(&mut self) -> TryNext<Self::Item> {
        // `try_recv` itself is synchronous and bypasses Tokio's cooperative
        // accounting. Respect an already exhausted task budget without
        // consuming, resetting, or making the operation unconstrained. The
        // next arbitration's normal `poll_recv` registers Tokio's budget wake.
        if !tokio::task::coop::has_budget_remaining() {
            return TryNext::Empty;
        }
        let Some(receiver) = self.receiver.as_mut() else {
            return TryNext::Dormant;
        };
        match receiver.try_recv() {
            Ok(item) => TryNext::Item(C::map_item(item)),
            Err(TryRecvError::Empty) => TryNext::Empty,
            Err(TryRecvError::Disconnected) => self.close().map_or(TryNext::Dormant, TryNext::Item),
        }
    }
}

impl<T, C> BacklogSource for Mpsc<T, C>
where
    C: ChannelClosePolicy<T>,
{
    fn has_backlog(&self) -> bool {
        self.receiver.as_ref().is_some_and(|receiver| {
            !receiver.is_empty() || (C::EMITS_CLOSE && receiver.is_closed())
        })
    }
}

/// Error from arming an [`OptionalMpsc`] that already owns a receiver.
#[derive(Debug)]
pub struct AlreadyArmedReceiver<T> {
    receiver: MpscReceiver<T>,
}

impl<T> AlreadyArmedReceiver<T> {
    /// Returns the receiver that was not installed.
    pub fn into_inner(self) -> MpscReceiver<T> {
        self.receiver
    }
}

/// A dynamically armed Tokio mpsc source.
///
/// Dormant and post-close polling returns `Pending` without self-waking. Arming
/// requires exclusive access between arbitrations; K0 intentionally provides
/// no concurrent control handle. Selection-loss, cooperative-budget, and drop
/// behavior otherwise match [`Mpsc`]. Optional receivers are not drainable in
/// K0 because handlers can replace their installed generation.
#[derive(Debug)]
pub struct OptionalMpsc<T, C> {
    receiver: Option<MpscReceiver<T>>,
    close: PhantomData<C>,
}

impl<T, C> Unpin for OptionalMpsc<T, C> {}

impl<T, C> OptionalMpsc<T, C>
where
    C: ChannelClosePolicy<T>,
{
    /// Creates a dormant optional source.
    pub fn new(_close: C) -> Self {
        Self {
            receiver: None,
            close: PhantomData,
        }
    }

    /// Installs a receiver.
    ///
    /// # Errors
    ///
    /// Returns [`AlreadyArmedReceiver`] containing the uninstalled receiver
    /// when this source already owns a live receiver.
    pub fn arm(
        &mut self,
        receiver: impl Into<MpscReceiver<T>>,
    ) -> Result<(), AlreadyArmedReceiver<T>> {
        let receiver = receiver.into();
        if self.receiver.is_some() {
            Err(AlreadyArmedReceiver { receiver })
        } else {
            self.receiver = Some(receiver);
            Ok(())
        }
    }

    /// Removes and returns the currently installed receiver.
    pub fn disarm(&mut self) -> Option<MpscReceiver<T>> {
        self.receiver.take()
    }

    /// Replaces the current receiver and returns the previous one.
    pub fn replace(&mut self, receiver: impl Into<MpscReceiver<T>>) -> Option<MpscReceiver<T>> {
        self.receiver.replace(receiver.into())
    }

    /// Returns whether no receiver is installed.
    pub const fn is_dormant(&self) -> bool {
        self.receiver.is_none()
    }

    fn close(&mut self) -> Poll<C::Item> {
        self.receiver = None;
        // The sealed policy is a static choice, not adapter runtime state.
        C::close_event().map_or(Poll::Pending, Poll::Ready)
    }
}

impl<T, C> sealed::Sealed for OptionalMpsc<T, C> where C: ChannelClosePolicy<T> {}

impl<T, C> ReactorSource for OptionalMpsc<T, C>
where
    C: ChannelClosePolicy<T>,
{
    type Item = C::Item;
    type Readiness = readiness::MayRemainReady;

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Self::Item> {
        let Some(receiver) = self.receiver.as_mut() else {
            return Poll::Pending;
        };
        match receiver.poll_recv(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(C::map_item(item)),
            Poll::Ready(None) => self.close(),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<T, C> BacklogSource for OptionalMpsc<T, C>
where
    C: ChannelClosePolicy<T>,
{
    fn has_backlog(&self) -> bool {
        self.receiver.as_ref().is_some_and(|receiver| {
            !receiver.is_empty() || (C::EMITS_CLOSE && receiver.is_closed())
        })
    }
}

/// A retained one-shot future.
///
/// The pinned future is stored in adapter-owned heap storage so losing an
/// arbitration never drops or reconstructs it and the outer adapter remains
/// `Unpin`. The allocation is adapter cost, not generated arbitration cost.
/// Cooperative-budget behavior is exactly that of `F`; Kittens adds no budget
/// wrapper. Dropping the adapter synchronously drops an unfinished future, so
/// this type does not claim external rollback or repeat safety.
#[derive(Debug)]
pub struct OneShot<F: Future> {
    future: Option<Pin<Box<F>>>,
}

impl<F: Future> Unpin for OneShot<F> {}

/// Retains a future as a one-shot source.
pub fn one_shot<F: Future>(future: F) -> OneShot<F> {
    OneShot {
        future: Some(Box::pin(future)),
    }
}

impl<F: Future> sealed::Sealed for OneShot<F> {}

impl<F: Future> ReactorSource for OneShot<F> {
    type Item = F::Output;
    type Readiness = readiness::Quiescent;

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Self::Item> {
        let Some(future) = self.future.as_mut() else {
            return Poll::Pending;
        };
        let output = match future.as_mut().poll(cx) {
            Poll::Ready(output) => output,
            Poll::Pending => return Poll::Pending,
        };
        self.future = None;
        Poll::Ready(output)
    }
}

/// A dynamically installed retained one-shot future.
///
/// Cancellation is visible in the method names. Losing selection preserves the
/// pinned future; explicit cancellation or dropping the adapter drops it. The
/// inner future determines cooperative-budget behavior.
#[derive(Debug)]
pub struct OptionalOneShot<F: Future> {
    future: Option<Pin<Box<F>>>,
}

impl<F: Future> Unpin for OptionalOneShot<F> {}

impl<F: Future> OptionalOneShot<F> {
    /// Creates a dormant optional one-shot source.
    pub const fn new() -> Self {
        Self { future: None }
    }

    /// Creates an armed optional one-shot source.
    pub fn from_future(future: F) -> Self {
        Self {
            future: Some(Box::pin(future)),
        }
    }

    /// Arms a dormant source.
    ///
    /// # Errors
    ///
    /// Returns [`AlreadyArmed`] containing the uninstalled future when an
    /// operation is already retained.
    pub fn arm(&mut self, future: F) -> Result<(), AlreadyArmed<F>> {
        if self.future.is_some() {
            Err(AlreadyArmed { item: future })
        } else {
            self.future = Some(Box::pin(future));
            Ok(())
        }
    }

    /// Drops any unfinished future and installs the replacement.
    pub fn cancel_and_replace(&mut self, future: F) -> bool {
        let replaced = self.future.is_some();
        self.future = Some(Box::pin(future));
        replaced
    }

    /// Drops any unfinished future and becomes dormant.
    pub fn cancel_and_disarm(&mut self) -> bool {
        self.future.take().is_some()
    }

    /// Returns whether no future is installed.
    pub const fn is_dormant(&self) -> bool {
        self.future.is_none()
    }
}

impl<F: Future> Default for OptionalOneShot<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Future> sealed::Sealed for OptionalOneShot<F> {}

impl<F: Future> ReactorSource for OptionalOneShot<F> {
    type Item = F::Output;
    type Readiness = readiness::Quiescent;

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Self::Item> {
        let Some(future) = self.future.as_mut() else {
            return Poll::Pending;
        };
        let output = match future.as_mut().poll(cx) {
            Poll::Ready(output) => output,
            Poll::Pending => return Poll::Pending,
        };
        self.future = None;
        Poll::Ready(output)
    }
}

/// A retained cancellation-token waiter.
///
/// The owned waiter is created once and pinned in adapter storage, so a losing
/// arbitration does not surrender fairness position or reconstruct it. The
/// waiter participates in Tokio cooperative scheduling through its underlying
/// notification primitive. After cancellation it disarms before returning
/// `Ready(())`. Dropping the adapter drops the waiter and token clone; it does
/// not cancel unrelated token owners or perform async cleanup.
#[derive(Debug)]
pub struct Cancellation {
    waiter: Option<Pin<Box<WaitForCancellationFutureOwned>>>,
}

impl Unpin for Cancellation {}

/// Creates a retained one-shot cancellation source.
pub fn cancellation(token: CancellationToken) -> Cancellation {
    Cancellation {
        waiter: Some(Box::pin(token.cancelled_owned())),
    }
}

impl sealed::Sealed for Cancellation {}

impl ReactorSource for Cancellation {
    type Item = ();
    type Readiness = readiness::Quiescent;

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Self::Item> {
        let Some(waiter) = self.waiter.as_mut() else {
            return Poll::Pending;
        };
        match waiter.as_mut().poll(cx) {
            Poll::Ready(()) => {
                self.waiter = None;
                Poll::Ready(())
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

/// A dynamic absolute Tokio deadline.
///
/// The absolute instant and pinned `Sleep` are retained across lost
/// arbitrations; firing disarms the source before delivering the instant.
/// Tokio's `Sleep::poll` participates in Tokio's cooperative scheduling
/// budget; Kittens does not make it unconstrained. The `Sleep` is heap-pinned
/// so the outer adapter is `Unpin`; this allocation is adapter cost. Dropping
/// it cancels the timer registration synchronously and no event is delivered
/// afterward.
#[derive(Debug, Default)]
pub struct OptionalDeadline {
    at: Option<Instant>,
    sleep: Option<Pin<Box<Sleep>>>,
}

impl Unpin for OptionalDeadline {}

impl OptionalDeadline {
    /// Creates a dormant deadline.
    pub const fn new() -> Self {
        Self {
            at: None,
            sleep: None,
        }
    }

    /// Sets an absolute deadline, or disarms with `None`.
    pub fn set(&mut self, at: Option<Instant>) {
        let Some(at) = at else {
            self.disarm();
            return;
        };
        self.at = Some(at);
        if let Some(sleep) = self.sleep.as_mut() {
            sleep.as_mut().reset(at);
        } else {
            self.sleep = Some(Box::pin(tokio::time::sleep_until(at)));
        }
    }

    /// Sets an absolute deadline.
    pub fn set_at(&mut self, at: Instant) {
        self.set(Some(at));
    }

    /// Disarms the deadline.
    pub fn disarm(&mut self) {
        self.at = None;
        self.sleep = None;
    }

    /// Returns whether no deadline is armed.
    pub const fn is_dormant(&self) -> bool {
        self.sleep.is_none()
    }

    /// Returns the currently armed absolute instant.
    pub const fn deadline(&self) -> Option<Instant> {
        self.at
    }
}

impl sealed::Sealed for OptionalDeadline {}

impl ReactorSource for OptionalDeadline {
    type Item = Instant;
    type Readiness = readiness::Quiescent;

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Self::Item> {
        let Some(sleep) = self.sleep.as_mut() else {
            return Poll::Pending;
        };
        match sleep.as_mut().poll(cx) {
            Poll::Ready(()) => {
                let at = self.at.take().expect("armed sleep has a deadline");
                self.sleep = None;
                Poll::Ready(at)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
