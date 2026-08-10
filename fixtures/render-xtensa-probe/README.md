# kittens-render Xtensa compile/link probe

This standalone firmware crate links both profile-owned ESP32-S3 SPI2 paths
against the exact audited `esp-hal` revision
`d48f747ba28accdc51779ba193eba923138e0382`: the sealed blocking SH8601 region
transport, followed by the interrupt-driven owning-completion adapter. Its
empty `[workspace]` table is intentional: the fixture must use the Espressif
`esp` toolchain and a git dependency rather than inherit the root workspace.
The direct dependency and `kittens-render`'s target-only dependency use the
same git URL and revision, so the `SpiDma` resource has one Cargo source
identity.

Build from this directory:

```sh
. "$HOME/export-esp.sh" && cargo +esp build --release --locked --target xtensa-esp32s3-none-elf
```

**Fact:** the linked path is `#![no_std]`/`#![no_main]`, defines no allocator,
uses `#[esp_hal::main]`, and binds real SPI2 plus GDMA channel 0 to the
Waveshare V1 QSPI pins (SIO0–3 GPIO4–7, SCK GPIO11, CS GPIO12) at 40 MHz mode
0. The profile transport first owns exact 16,380-byte RX and TX DMA scratch
buffers, restores a deliberately caller-shortened TX descriptor-chain length
from 1 to 16,380, and links a 368×112 region call over separate static
82,432-byte pixel storage. The retained path necessarily invokes the shared
engine's CASET, PASET, RAMWR, and five RAMWRC HAL calls, including the final
532-byte chunk.

**Fact:** after `BlockingSettled::into_parts`, the fixture verifies the pixel
pointer and sweep settlement, then `SpiDmaBus::split` recovers the exact
`SpiDma`, `DmaRxBuf`, and `DmaTxBuf` identities. The recovered `SpiDma` is the
one moved into the existing asynchronous probe; the blocking path is therefore
part of the entry-point ownership path retained in the linked firmware, not an
unused generic instantiation. A separately named helper accepts arbitrary
same-source `SpiDma` and scratch parts, pinning the documented configuration-
honesty escape.

**Fact:** the asynchronous adapter module forbids unsafe code and implements
the current `kittens_render::transfer::OwnedTransfer` contract:
`poll_done -> Poll<()>`,
register-then-recheck, cancellation linearization plus progress wake, outcome
authority in consuming `recover`, candidate-waker clone before the global
critical section with replaced/unused wakers dropped after exclusion,
`wait()` recovery, and synchronous cancel/wait/disarm cleanup on ordinary
drop. The firmware statically checks
the concrete wrapper and its `InFlight` carrier are `Unpin`, identity-checks
the outer spare, and starts a second transfer with the recovered driver.

**Observation:** a successful link and clean allocator-symbol inspection close
only the exact HAL API, vector-binding, Rust ownership, no-allocation, safe
`SpiDmaBus` construction/split, and no-self-reference questions named by the
profile contract. This fixture manually polls `InFlight`; it does not
establish target-side generated-reactor execution or behavior on silicon.

**Fact (revision-10 run, 2026-08-09):** a fresh locked optimized link produced
a statically linked, unstripped 32-bit little-endian Tensilica Xtensa
executable; `readelf -h` identified `EXEC` and entry point `0x403785e8`. The
final artifact was 208,496 bytes with SHA-256
`648e43a0c03d89d71737d7dd20ff0390d6275b08b4f1f297d15d443af6c68513`;
`xtensa-esp32s3-elf-size -A` reported 116,988 bytes of `.bss`. The complete
demangled symbol table contained the concrete `Sh8601Wire::write`
implementation and contained none of the allocator entry points or Rust
allocation-module symbols listed in the deployment target procedure;
`nm -u -C` was empty.

**Fact:** the post-revision-8 replacement command output and linked artifact
metadata are recorded in `TRACE-MANIFEST.md` and `K2R0A-LOG.md`. The
`xtensa-link` CI job repeats the release link from an uncached target directory
and inspects the resulting Xtensa executable. It requires an empty undefined-
symbol table, retains the concrete wire symbol, and rejects allocator symbols;
it does not substitute `cargo check` for linking.

**Gap: SPI2 interrupt delivery, exact wake counts, completion-before-first-poll
visibility, and cancel/drain behavior remain board-HIL gated (no data exists).**

**Fact:** the separate kernel-admitted completion-source gate is closed with
host + portable-link scope by the real-reactor host oracles and the Thumb/wasm
downstream fixture. This Xtensa probe is an explicit non-control for that row.

**Gap: panel initialization and command acceptance, physical region placement,
RAMWRC interpretation, RGB565 channel/byte fidelity, visible output, tearing,
and timing remain board-HIL gated (no data exists).**
