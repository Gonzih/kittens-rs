//! **PSEUDOCODE DELTA — NOT COMPILE-READY ADAPTER SOURCE.**
//!
//! This file records only the conceptual differences between `VERDICT.md`'s
//! historical sketch and the landed host contract. It deliberately contains
//! documentation comments rather than a Rust adapter: there are no imports,
//! concrete type, `OwnedTransfer` implementation, complete `cancel` or
//! `recover`, peripheral/DMA setup, or firmware entry point. Treating these
//! comments as an empty Rust crate would exercise no adapter code; installing
//! `espup` does not turn them into the missing source. The exact-HAL
//! compile/link probe remains open and must be supplied as separate real
//! source against the pinned SHA.
//!
//! The interrupt/waker-slot mechanics remain those described in
//! `VERDICT.md`; in particular, cancellation stores its settlement at the
//! completion-observation linearization point.
//!
//! Differences from VERDICT.md's historical blueprint:
//! 1. `poll_done` returns `Poll<()>`; the outcome is stored internally and
//!    reported only by `recover` (correction 5).
//! 2. Public code cannot pair an already-started transfer with a target.
//!    `StripeTarget::start_flight(spare, starter)` calls `starter` with that
//!    target's exact `Region`; `StartFlightError` returns the starter error,
//!    spare, and unchanged target when no transfer was accepted.
//! 3. `Settled::into_parts` returns resources plus exactly one
//!    `StripeSettlement`; `Sweep::settle` advances on `Written` and poisons
//!    irreversibly on `Unwritten` (cancelled/failed).
//! 4. Register-then-recheck ordering is a trait-level contract with an
//!    adversarial oracle; the slot discipline below satisfies it.
//!
//! Conceptual call-site delta (also pseudocode):
//!
//! let flight = target.start_flight(spare, |region| {
//!     adapter.start_region(sent, region) // Result<StartedTransfer, StartError>
//! })?;
//!
//! fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<()> {
//!     if self.settled.is_some() { return Poll::Ready(()); }
//!     let ready = critical_section::with(|cs| {
//!         let mut slot = SPI2_DONE.borrow(cs).borrow_mut();
//!         if slot.event_seen || spi2_done_raw() || self.inner_is_done() {
//!             mask_and_clear_spi2_done(); slot.active = false; true
//!         } else {
//!             slot.waker = Some(cx.waker().clone());          // register
//!             if slot.event_seen || spi2_done_raw() || self.inner_is_done() {
//!                 mask_and_clear_spi2_done(); slot.active = false; true // recheck
//!             } else { false }
//!         }
//!     });
//!     if ready { self.settled = Some(TransferOutcome::Completed); Poll::Ready(()) }
//!     else { Poll::Pending }
//! }
//!
//! A future real adapter's `cancel()` and ISR follow `VERDICT.md`; its
//! complete `recover()` must call the HAL's consuming `wait()` (no busy
//! iterations after settlement), return all resources, and disarm the slot.
