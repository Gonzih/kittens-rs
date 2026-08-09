# SUPERSEDED: historical esp32s3-spi2 probe record

> **SUPERSEDED.** This directory retains the pre-implementation engineering
> verdict and pseudocode for audit history. The current compile-ready adapter
> and linked-firmware source live in
> [`fixtures/render-xtensa-probe`](https://github.com/Gonzih/kittens-rs/tree/main/fixtures/render-xtensa-probe).
> Nothing in this directory is current implementation evidence or guidance.

The HAL-fidelity verdict and its historical adapter blueprint live in
[VERDICT.md](VERDICT.md) (external engineering contribution, Codex
gpt-5.6-sol ultra, 2026-08-08): a profile-owned SPI2 TransferDone ISR +
critical-section waker slot implements the OwnedTransfer boundary on
esp-hal v1.1.0 (`d48f747`), stable Rust, no alloc, no unsafe self-reference,
no upstream changes. At the time of this record, turning that verdict into a
real linked firmware for `xtensa-esp32s3-none-elf` still required source and
the espup toolchain. That work subsequently landed in the fixture linked
above, closing the language/API/ownership compile-link question with scope;
board HIL still gates the silicon-interrupt half.

## Pseudocode delta over the retained verdict

The verdict's blueprint implements `poll_done -> Poll<TransferOutcome>`.
The trait as landed uses `poll_done -> Poll<()>` with recovery as the sole
outcome authority (the verdict's own correction 5, applied after it was
written). The landed host API also removed public `InFlight::new`:
`StripeTarget::start_flight` now invokes an operation-bound `FlightStarter`
with the target's exact region and a crate-issued `StartPermit<'_>`, and
`Settled::into_parts` returns exactly one move-only
`StripeSettlement::{Written, Unwritten}`. The cooperative caller path delivers
that witness to its owning `Sweep::settle`; Rust cannot force delivery or
prevent a consuming wrong-owner rejection. The
verdict text is retained unedited as the historical record; the subsequently
landed exact Xtensa probe implements the corrected signatures and lifecycle.
Pairing is structural under integrations reviewed and sealed at freeze; while both
capability traits remain open for the experiment, region honesty and
acceptance-atomic rejection remain explicit integration obligations:

```rust
fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<()>;      // settlement only
fn recover(self) -> Recovered<Self::Transport, Self::Buffer>;   // sole outcome authority
fn start(
    self,
    region: Region,
    permit: StartPermit<'_>,
) -> Result<Self::Transfer, Self::Error>; // FlightStarter
```

Additionally, per finding 2, the blueprint's `cancel()` already stores the
settlement at its completion-observation linearization point — that part is
correct as written and is now also a trait-level contract with an
adversarial oracle (`cancel_then_late_completion_stays_cancelled`).

[adapter-blueprint.rs](adapter-blueprint.rs) is emphatically **not a corrected
adapter source file**. It is a documentation-comment pseudocode delta over
`VERDICT.md`: it has no imports, concrete adapter type,
`OwnedTransfer`/`FlightStarter` implementations, complete `cancel`/`recover`,
hardware setup, or firmware entry point. Compiling it as an empty Rust crate
would exercise no adapter code. The real pinned-SHA fixture linked above is the
only compile/link evidence; this directory remains historical pseudocode.
