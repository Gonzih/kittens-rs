//! Corrected ESP32-S3 SPI2 adapter blueprint — the verdict's design updated
//! to the landed trait signatures (poll_done -> Poll<()>, recovery as sole
//! outcome authority, StripeTarget-bound InFlight construction).
//!
//! NOT COMPILED in this workspace: building it requires the Xtensa
//! toolchain (`espup`) and the pinned esp-hal SHA recorded in VERDICT.md.
//! It exists so the corrected shape is concrete before that gate opens;
//! the interrupt/waker-slot mechanics are unchanged from VERDICT.md, whose
//! cancel() already stores its settlement at the completion-observation
//! linearization point.
//!
//! Differences from VERDICT.md's historical blueprint:
//! 1. `poll_done` returns `Poll<()>`; the outcome is stored internally and
//!    reported only by `recover` (correction 5).
//! 2. The adapter is constructed into `InFlight::new(transfer, spare,
//!    target)` where the `StripeTarget` comes from `Sweep::next_target()` —
//!    identity cannot be claimed independently (round-2 finding 4).
//! 3. Register-then-recheck ordering is a trait-level contract with an
//!    adversarial oracle; the slot discipline below satisfies it.
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
//! cancel() and the ISR are as in VERDICT.md; recover() calls the HAL's
//! consuming wait() (no busy iterations after settlement) and disarms.
