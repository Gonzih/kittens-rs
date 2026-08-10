//! **SUPERSEDED HISTORICAL PSEUDOCODE — NOT IMPLEMENTATION EVIDENCE.**
//!
//! The current compile-ready adapter is
//! `crates/kittens-render/src/esp32s3_sh8601_async.rs`, and
//! `fixtures/render-xtensa-probe/src/main.rs` links it. This retained fragment
//! predates SPEC revision 8's waker correction and MUST NOT be copied: the
//! current implementation clones the candidate waker before entering the
//! global critical section and moves every replaced/unused waker out for drop.
//!
//! This file records only the conceptual differences between `VERDICT.md`'s
//! historical sketch and the landed host contract. It deliberately contains
//! documentation comments rather than a Rust adapter: there are no imports,
//! concrete type, `OwnedTransfer` implementation, complete `cancel` or
//! `recover`, peripheral/DMA setup, or firmware entry point. Treating these
//! comments as an empty Rust crate would exercise no adapter code. At the time
//! of this record the exact-HAL compile/link probe remained open; it was later
//! closed with scope by the separate real fixture against the pinned SHA.
//!
//! The interrupt/waker-slot mechanics remain those described in
//! `VERDICT.md`; in particular, cancellation stores its settlement at the
//! completion-observation linearization point.
//!
//! Differences from VERDICT.md's historical blueprint:
//! 1. `poll_done` returns `Poll<()>`; the outcome is stored internally and
//!    reported only by `recover` (correction 5).
//! 2. `StripeTarget::start_flight(spare, starter)` calls the operation-bound
//!    `FlightStarter::start` implementation with that target's exact `Region`
//!    and a crate-issued `StartPermit<'_>`;
//!    `StartFlightError` returns the starter error, spare, and unchanged target
//!    when the integration reports that no transfer was accepted. This is
//!    structural only after `FlightStarter` is sealed to reviewed adapters.
//!    During the experiment, a dishonest safe implementation can still return
//!    a prestarted transfer for another region or start and then return `Err`.
//! 3. `Settled::into_parts` returns resources plus exactly one move-only
//!    `StripeSettlement`; cooperative delivery to the matching owner advances
//!    on `Written` and poisons irreversibly on `Unwritten` (cancelled/failed).
//!    Rust cannot force delivery or prevent a wrong-owner consuming rejection.
//! 4. Register-then-recheck ordering is a trait-level contract with an
//!    adversarial oracle; the slot discipline below satisfies it.
//!
//! Conceptual call-site delta (also pseudocode):
//!
//! struct Esp32s3RegionStart { adapter: Adapter, sent: SentBuffer }
//!
//! impl FlightStarter for Esp32s3RegionStart {
//!     type Transfer = StartedTransfer;
//!     type Error = StartError;
//!
//!     fn start(
//!         self,
//!         region: Region,
//!         _permit: StartPermit<'_>,
//!     ) -> Result<Self::Transfer, Self::Error> {
//!         self.adapter.start_region(self.sent, region)
//!     }
//! }
//!
//! let starter = Esp32s3RegionStart { adapter, sent };
//! let flight = target.start_flight(spare, starter)?;
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
//! The superseding real adapter's `cancel()` and ISR follow the reviewed
//! lifecycle; its complete `recover()` calls the HAL's consuming `wait()` (no
//! busy iterations after settlement), returns all resources, and disarms the
//! slot.
