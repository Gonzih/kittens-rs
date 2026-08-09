# esp32s3-spi2 probe (Xtensa-gated)

The HAL-fidelity verdict and compile-ready adapter blueprint live in
[VERDICT.md](VERDICT.md) (external engineering contribution, Codex
gpt-5.6-sol ultra, 2026-08-08): a profile-owned SPI2 TransferDone ISR +
critical-section waker slot implements the OwnedTransfer boundary on
esp-hal v1.1.0 (`d48f747`), stable Rust, no alloc, no unsafe self-reference,
no upstream changes. Building this as a linked firmware for
`xtensa-esp32s3-none-elf` requires the espup toolchain (user approval
pending) and closes the language/API half; a small board HIL closes the
silicon-interrupt half.

## Superseded detail in the retained verdict (exit-review finding 3)

The verdict's blueprint implements `poll_done -> Poll<TransferOutcome>`.
The trait as landed uses `poll_done -> Poll<()>` with recovery as the sole
outcome authority (the verdict's own correction 5, applied after it was
written), and `InFlight::new` now also carries `FrameEpoch` and `Region`
for the settlement witness. The verdict text is retained unedited as the
historical record; the Xtensa probe implements the corrected signatures:

```rust
fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<()>;      // settlement only
fn recover(self) -> Recovered<Self::Transport, Self::Buffer>;   // sole outcome authority
```

Additionally, per finding 2, the blueprint's `cancel()` already stores the
settlement at its completion-observation linearization point — that part is
correct as written and is now also a trait-level contract with an
adversarial oracle (`cancel_then_late_completion_stays_cancelled`).
