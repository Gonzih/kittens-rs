# kittens-render Xtensa compile/link probe

This standalone firmware crate compiles the profile-owned ESP32-S3 SPI2
completion adapter against the exact audited `esp-hal` revision
`d48f747ba28accdc51779ba193eba923138e0382`. Its empty `[workspace]` table is
intentional: the fixture must use the Espressif `esp` toolchain and a git
dependency rather than inherit the root workspace.

Build from this directory:

```sh
. "$HOME/export-esp.sh" && cargo +esp build --release --target xtensa-esp32s3-none-elf
```

**Fact:** the linked path is `#![no_std]`/`#![no_main]`, defines no allocator,
uses `#[esp_hal::main]`, owns two statically described `DmaTxBuf`s, and binds
real SPI2 plus GDMA channel 0 to the Waveshare V1 QSPI pins (SIO0–3 GPIO4–7,
SCK GPIO11, CS GPIO12). It starts the SH8601 TX-only quad-data RAM-write phase
with command `0x32`, address `0x002c00`, and zero dummy cycles.

**Fact:** the adapter module forbids unsafe code and implements the current
`kittens_render::transfer::OwnedTransfer` contract: `poll_done -> Poll<()>`,
register-then-recheck, cancellation linearization plus progress wake, outcome
authority in consuming `recover`, candidate-waker clone before the global
critical section with replaced/unused wakers dropped after exclusion,
`wait()` recovery, and synchronous cancel/wait/disarm cleanup on ordinary
drop. The firmware statically checks
the concrete wrapper and its `InFlight` carrier are `Unpin`, identity-checks
the outer spare, and starts a second transfer with the recovered driver.

**Observation:** a successful link closes only the HAL API, vector-binding,
Rust ownership, no-allocation, and no-self-reference feasibility question.
It does not establish behavior on silicon.

**Observation:** SPEC revision 8 changed waker registration after the prior
linked artifact. The replacement command output and artifact metadata are
**PENDING REBUILD EVIDENCE** in `TRACE-MANIFEST.md` and `K2R0A-LOG.md`.

**Gap: SPI2 interrupt delivery, exact wake counts, completion-before-first-poll
visibility, and cancel/drain behavior remain board-HIL gated (no data exists).**

**Gap: a kernel-admitted completion source and real `kittens::reactor!`
fixture remain a separate open gate (no data exists).**

**Gap: the blocking `write_region` transport integration remains gated; this
probe compiles the raw SH8601 pixel phase and does not claim a complete display
driver or prepared address-window protocol (no data exists).**
