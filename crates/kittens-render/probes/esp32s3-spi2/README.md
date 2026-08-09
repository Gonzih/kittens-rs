# esp32s3-spi2 probe record (Xtensa-gated; no compile-ready adapter yet)

The HAL-fidelity verdict and its historical adapter blueprint live in
[VERDICT.md](VERDICT.md) (external engineering contribution, Codex
gpt-5.6-sol ultra, 2026-08-08): a profile-owned SPI2 TransferDone ISR +
critical-section waker slot implements the OwnedTransfer boundary on
esp-hal v1.1.0 (`d48f747`), stable Rust, no alloc, no unsafe self-reference,
no upstream changes. Turning that verdict into a real linked firmware for
`xtensa-esp32s3-none-elf` still requires both source that does not yet exist in
this probe directory and the espup toolchain (user approval pending); that
would close the language/API half, while a small board HIL closes the
silicon-interrupt half.

## Pseudocode delta over the retained verdict

The verdict's blueprint implements `poll_done -> Poll<TransferOutcome>`.
The trait as landed uses `poll_done -> Poll<()>` with recovery as the sole
outcome authority (the verdict's own correction 5, applied after it was
written). The landed host API also removed public `InFlight::new`:
`StripeTarget::start_flight` now invokes the starter with the target's exact
region, and `Settled::into_parts` returns the mandatory
`StripeSettlement::{Written, Unwritten}` consumed by `Sweep::settle`. The
verdict text is retained unedited as the historical record; any future exact
Xtensa probe must implement the corrected signatures and lifecycle:

```rust
fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<()>;      // settlement only
fn recover(self) -> Recovered<Self::Transport, Self::Buffer>;   // sole outcome authority
```

Additionally, per finding 2, the blueprint's `cancel()` already stores the
settlement at its completion-observation linearization point — that part is
correct as written and is now also a trait-level contract with an
adversarial oracle (`cancel_then_late_completion_stays_cancelled`).

[adapter-blueprint.rs](adapter-blueprint.rs) is emphatically **not a corrected
adapter source file**. It is a documentation-comment pseudocode delta over
`VERDICT.md`: it has no imports, concrete adapter type,
`OwnedTransfer` implementation, complete `cancel`/`recover`, hardware setup,
or firmware entry point. Compiling it as an empty Rust crate would exercise no
adapter code; installing the Xtensa toolchain does not turn it into the missing
source. A real, compile-ready adapter and linked firmware against the pinned
esp-hal SHA remain an open Xtensa gate.
