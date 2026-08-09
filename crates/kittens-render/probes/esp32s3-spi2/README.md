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
