# kittens-render Xtensa compile/link probe

This standalone firmware crate links the sealed blocking SH8601 region writer
and the branded single-payload async adapter against audited `esp-hal` revision
`d48f747ba28accdc51779ba193eba923138e0382`. Its empty `[workspace]` table is
intentional: the fixture needs the Espressif `esp` toolchain and git-pinned HAL
without joining the stock-Rust root workspace. The direct HAL dependency and
`kittens-render`'s target-only dependency therefore resolve to one Cargo source
identity.

Under the revision-12 acceptance map, this is the repository's exact-git
source-revision fixture. Its evidence contributes to the K2R-0A matrix, which
is closed with host + portable-link + exact-Xtensa-link scope, but the fixture
does not by itself close that matrix or the K2R-0 protocol freeze. Bilateral
seam co-sign and sealing `FlightStarter`/`OwnedTransfer` still gate K2R-0. A
real target executor, board coordinator, runtime observations, and board HIL
belong to K2R-1.

Build from this directory:

```sh
. "$HOME/export-esp.sh"
cargo +esp clippy --release --locked --target xtensa-esp32s3-none-elf -- -D warnings
cargo +esp clippy --release --locked --target xtensa-esp32s3-none-elf \
  --manifest-path ../../crates/kittens-render/Cargo.toml \
  --no-default-features --features esp32s3-sh8601-async --lib -- -D warnings
cargo +esp build --release --locked --target xtensa-esp32s3-none-elf
```

**Fact (source boundary):** the firmware is `#![no_std]`/`#![no_main]`, defines
no allocator, uses `#[esp_hal::main]`, and keeps the revision-10 blocking path
on the executable entry path. That path binds SPI2 plus GDMA channel 0 to the
Waveshare V1 QSPI roles (SIO0–3 GPIO4–7, SCK GPIO11, CS GPIO12) at 40 MHz mode
0. It gives the sealed blocking transport symmetric 16,380-byte command
scratch, restores a deliberately shortened TX descriptor chain, and retains a
368×112 write over separate 82,432-byte pixel storage: CASET, PASET, RAMWR,
four full RAMWRC chunks, and a final 532-byte RAMWRC chunk.

**Fact (source boundary):** the superseded fixture-local RAMWR-only adapter is
gone. The standalone manifest enables `kittens-render`'s
`esp32s3-sh8601-async` feature, depends on `kittens` only for its no-default
`macros` feature, and has no direct `critical-section` dependency. The profile
now owns the exact SPI2 completion slot, HAL mapping, branded constructor,
window preamble, start-error recovery, cancel/recover, and drop behavior.

**Fact (source boundary):** `linked_async_reactor_paths` constructs a real
branded 368×16 flight and a generated `kittens::reactor!` over
`OptionalInlineOneShot<InFlight<...>>`. Its linked handler paths recover and
settle two completed stripes, rearm the same carrier after each, then borrow
the accepted third flight through `future_mut` for drain. The final handler
keeps distinct `Completed` and `Cancelled` reconciliation branches and aborts
the ready or poisoned sweep. A separate `#[inline(never)]` shim pins and
black-boxes the generated future and performs exactly one `Waker::noop` poll;
there is no fixture executor or manual polling loop.

**Fact (source boundary):** `linked_async_drop_path` constructs one accepted
real flight, arms the complete source owner, explicitly drops that owner,
drops the outstanding sweep, then calls `FrameDemand::abandon_active`. The
entry point coerces both outer hooks to function pointers and black-boxes the
pointers without calling either. The hooks therefore retain target
monomorphizations without claiming observed settlement, cancellation, or drop.

Three target-only compile-fail bins pin configuration admission at the public
`Waveshare18V1Sh8601Parts::new` boundary:

- `compile-fail-spi3` substitutes SPI3 for SPI2;
- `compile-fail-dma-ch1` substitutes DMA_CH1 for DMA_CH0; and
- `compile-fail-swapped-sio` swaps the GPIO4/GPIO5 SIO roles.

CI requires each command to fail with E0308 in its own fixture and checks the
expected and actual singleton names. These are negative controls for the
profile-owned brand, not claims about raw HAL calls, which still compile
outside the adapter. The `compile-pass-parts` control separately constructs,
extracts, and reconstructs the exact SPI2/DMA_CH0/GPIO4–7/11/12 tuple and
type-checks both public `try_new` result arms.

**Fact (revision-11 run, 2026-08-09):** the direct profile-library target
Clippy, standalone fixture target Clippy, and optimized locked link passed. The
ELF is 214,352 bytes with SHA-256
`30cd240176d206d6483e04fd0f2384ced2b101491ff6e516ec635a4bbd98664a`,
entry `0x403785e8`, and 115,492 bytes of `.bss`. The undefined-symbol table and
allocator scan are empty. `nm -S -C` retains nonzero text symbols for
`linked_async_reactor_paths` (`0x168`), `poll_generated_reactor_once`
(`0xaf6`), and `linked_async_drop_path` (`0x137`), plus the blocking wire
symbol. The three branded target failures reach their intended E0308
diagnostics and the Parts roundtrip control passes.

**Fact (revision-10 run, 2026-08-09):** the previous blocking-region fixture
produced a statically linked, unstripped 32-bit little-endian Tensilica Xtensa
executable. `readelf -h` identified `EXEC` and entry point `0x403785e8`; the
artifact was 208,496 bytes with SHA-256
`648e43a0c03d89d71737d7dd20ff0390d6275b08b4f1f297d15d443af6c68513`,
and `.bss` was 116,988 bytes. Its undefined-symbol table was empty, the
blocking wire implementation was retained, and the documented allocator scan
was clean. That chronology remains evidence only for revision 10.

**Observation:** the revision-11 link closes the named async adapter row only
for exact HAL API, vector binding, Rust ownership, branded type rejection,
retained generated code, and allocator-symbol questions. Because the async
hooks are not called, it is explicitly not evidence for executor scheduling,
SPI2 interrupt delivery, wake counts, completion/cancellation races, drop at
runtime, panel
commands, or pixels on silicon. The allocator scan constrains this exact
noop-waker binary only; an arbitrary executor's `RawWaker` callbacks may
allocate or perform other unchecked work.

**Observation (selected open row):** this fixture resolves the exact git HAL
revision and is an explicit non-control for revision 12's clean normalized
packaged-source + registry-HAL Xtensa consumer gate. That separate local gate
must compose Cargo's extracted package with one registry `esp-hal =1.1.0`
identity, cross the packaged board constructor and `start_flight`, then pass
target Clippy, optimized link, and ELF/symbol inspections. It remains open;
this fixture's green history cannot substitute for it. Even a future pass
would not authorize publication. Upload, index availability, and exact-version
download for a correctly versioned release are human-ordered only.

**Gap: real target-executor polling, board-coordinator serialization, physical
panel initialization and command acceptance, SPI2 interrupt delivery, exact
wake counts, async cancellation/drop behavior on silicon, region placement,
RGB565 fidelity, visible output, tearing, and timing (no data exists).** These
are K2R-1 runtime and board-HIL gates.
