# Deployment target: Xtensa (ESP32-S3)

Build and deployment process for Xtensa firmware in this monorepo. The
anchor hardware is the **Waveshare ESP32-S3 1.8" AMOLED Touch Display, V1
revision** (SH8601 display driver over QSPI on SPI2, FT3168 capacitive touch
on I2C, 368×448, TE on GPIO13, TP_INT on GPIO21) — the board named in
`crates/kittens-render/SPEC.md`. The first firmware in the repo is the
compile/link probe at `fixtures/render-xtensa-probe/`.

Evidence labels per AGENTS.md: **Fact** = executed in this repo or a recorded
contract status; **Observation** = what inspected evidence shows without a
broader guarantee; **Hypothesis** = an expected process not yet executed here;
**Recommendation** = a proposed action rather than evidence. Hardware has not
yet arrived.

**Fact (revision-12 contract status):** K2R-0A is closed with host + portable-
link + exact-Xtensa-link scope. That scoped feasibility result is not the
K2R-0 protocol freeze: bilateral seam co-sign and sealing the generic
`FlightStarter`/`OwnedTransfer` capabilities still gate K2R-0. A real target
executor, the board coordinator, target-runtime observations, and every HIL
claim belong to K2R-1.

**Observation (selected open row):** revision 12 specifies a separate local
clean packaged-source + registry-HAL Xtensa consumer gate. Its matrix has not
yet passed. The exact-git fixture documented below remains the reviewed
source-revision gate and is an explicit non-control for the normalized package
source identity. Upload, index availability, and exact-version download for a
future correctly versioned release remain human-ordered publication work.

## 1. Toolchain (Fact — executed 2026-08-09)

Xtensa is not a rust-lang upstream target; it needs Espressif's fork of
rustc, installed and managed by `espup`:

```sh
cargo install espup --locked   # installed espup v0.17.1
espup install                  # installs the rustup toolchain named `esp`
```

`espup install` writes an environment file to `$HOME/export-esp.sh` and the
toolchain appears as `esp` in `rustup toolchain list`. **Every shell that
builds for Xtensa must source that file first** (it puts the Xtensa gcc
linker and libclang on PATH):

```sh
. "$HOME/export-esp.sh"
```

Without sourcing it the build fails at link time. This step is per-shell,
not per-machine.

## 2. Building (Fact — executed 2026-08-09)

Xtensa firmware crates do **not** join the root workspace (they need the
`esp` toolchain; the root workspace must keep building with stock stable
Rust). Each firmware crate is standalone: an empty `[workspace]` table in its
`Cargo.toml`, plus a `.cargo/config.toml` carrying:

```toml
[unstable]
build-std = ["core"]           # core is built from source for the target

[target.xtensa-esp32s3-none-elf]
rustflags = [
    "-C", "link-arg=-Wl,-Tlinkall.x",   # esp-hal's linker script
    "-C", "link-arg=-nostartfiles",
]
```

Build command, from inside the firmware crate directory:

```sh
. "$HOME/export-esp.sh"
cargo +esp clippy --release --locked --target xtensa-esp32s3-none-elf -- -D warnings
cargo +esp clippy --release --locked --target xtensa-esp32s3-none-elf \
  --manifest-path ../../crates/kittens-render/Cargo.toml \
  --no-default-features --features esp32s3-sh8601-async --lib -- -D warnings
cargo +esp build --release --locked --target xtensa-esp32s3-none-elf
file target/xtensa-esp32s3-none-elf/release/kittens-render-xtensa-probe
xtensa-esp32s3-elf-readelf -h \
  target/xtensa-esp32s3-none-elf/release/kittens-render-xtensa-probe
xtensa-esp32s3-elf-nm -a -S -C \
  target/xtensa-esp32s3-none-elf/release/kittens-render-xtensa-probe
```

Recorded first-run result for `fixtures/render-xtensa-probe` (2026-08-09):
`Finished release [optimized] in 26.73s`, artifact
`target/xtensa-esp32s3-none-elf/release/kittens-render-xtensa-probe`,
204,700 bytes, `file` reports
`ELF 32-bit LSB executable, Tensilica Xtensa, version 1 (SYSV), statically
linked, not stripped`. A successful build **must** produce this linked ELF —
`cargo check` is not the gate (linking is where missing vectors, linker
scripts, and start files fail).

A later post-revision-8 freshness rebuild after source changes produced the
204,292-byte, SHA-256
`4fff6dcd8284fd35891731caa6fea574f0a70a6601e348048d1db54b8bca4f49`
revision-9 artifact recorded in `crates/kittens-render/K2R0A-LOG.md`. That is a
separate chronology point from the 204,700-byte first run above.

Recorded revision-10 blocking-region result (2026-08-09): a fresh locked
optimized link produced a 208,496-byte artifact with SHA-256
`648e43a0c03d89d71737d7dd20ff0390d6275b08b4f1f297d15d443af6c68513`.
`file` identified the statically linked Xtensa executable, `readelf -h`
reported `EXEC` with entry point `0x403785e8`, and
`xtensa-esp32s3-elf-size -A` reported 116,988 bytes of `.bss`. `nm -u -C` was
empty. The complete demangled symbol table retained the concrete
`Esp32s3Sh8601BlockingTransport` wire implementation and contained none of the
allocator symbols listed below. The linked entry path deliberately shortens the
TX descriptor-chain length to 1 before constructor admission restores it to
16,380.

**Fact (revision-11 run, 2026-08-09):** the standalone source removes its local
async adapter and enables the profile-owned `esp32s3-sh8601-async`
implementation.
Its entrypoint preserves the executable revision-10 blocking path and retains,
without calling, two function pointers: one generated-reactor path that rearms
two completed 368×16 flights and drains a third, and one armed-source drop path
that exercises drop-plus-`abandon_active` glue. A dedicated shim performs
exactly one opaque `Waker::noop` poll; it is not an executor. The fresh locked
optimized ELF is 214,352 bytes with SHA-256
`30cd240176d206d6483e04fd0f2384ced2b101491ff6e516ec635a4bbd98664a`,
entry `0x403785e8`, and 115,492 bytes of `.bss`. Its undefined-symbol and
allocator scans are empty. Nonzero `nm -S -C` text symbols retain the reactor
hook (`0x168`), one-poll shim (`0xaf6`), and armed-drop hook (`0x137`). These
values supersede neither the revision-9 nor revision-10 chronology above.

For every no-allocator gate, inspect the complete demangled `nm` output and
require no allocator entry points or Rust allocation-module symbols. The
revision-10 blocking-region evidence checks at least `__rust_alloc`,
`__rust_dealloc`, `__rust_realloc`, `__rust_alloc_zeroed`, `malloc`, `calloc`,
`realloc`, `free`, `__rdl_*`, `__rg_*`, top-level `alloc`/`esp_alloc` code, and
`GlobalAlloc` entry points; a successful link alone is not allocation
evidence.

Dependency pinning: the repository-source probe pins the audited HAL revision
`esp-hal = { git = "https://github.com/esp-rs/esp-hal", rev = "d48f747ba28accdc51779ba193eba923138e0382", default-features = false, features = ["esp32s3", "rt", "unstable"] }`
(that rev is the v1.1.0 release audited in
`crates/kittens-render/probes/esp32s3-spi2/VERDICT.md`). esp-hal's async/DMA
API surface is behind its `unstable` cargo feature — that is HAL API
stability, not Rust nightly.

### 2.1 Pending clean packaged-source + registry-HAL gate

**Observation:** earlier host package inspection showed that Cargo's generated
manifest retains registry `esp-hal =1.1.0` while dropping the repository-only
git location. That observation does not exercise registry type identity or
target linking and therefore does not pass revision 12's selected gate.

The pending local gate in `crates/kittens-render/SPEC.md` section 9.1 will:

1. run `cargo +1.96.0 package -p kittens-render --locked` from a clean,
   committed checkout without `--allow-dirty`, then require the archive's
   `.cargo_vcs_info.json` to identify exact `HEAD`, the
   `crates/kittens-render` path, and no truthy dirty marker;
2. inspect Cargo's generated manifest and metadata for exact registry
   `esp-hal =1.1.0` with the required target features and no git source;
3. stage the extracted package and the standalone
   `fixtures/render-packaged-xtensa-probe` consumer outside the checkout,
   requiring exactly one registry-sourced `esp-hal` identity and no `[patch]`;
4. pass direct packaged-library and consumer target Clippy, an optimized
   locked Xtensa link, ELF/undefined/allocator inspection, and a nonzero
   retained hook that crosses the packaged board constructor and
   `start_flight` spelling.

This row remains **OPEN** until that complete clean matrix runs. A dirty
archive, copied source, host-only package verification, or the exact-git
fixture is a non-control. Passing it will establish only packaged-source +
registry-HAL Xtensa-link compatibility; it will not execute the hook, prove a
target executor or interrupt/wake behavior, provide board/HIL evidence, or
authorize publication. The current crates.io 0.1.1 artifact is immutable older
source and cannot stand in for this tree. Any future upload must be correctly
versioned and explicitly human-ordered.

Firmware shape: `#![no_std]`, `#![no_main]`, `#[esp_hal::main]`, an
explicit `#[panic_handler]`, no allocator, `panic = "abort"`, fat LTO. The
application stays under `#![forbid(unsafe_code)]`; the target-only profile
adapter is the reviewed HAL boundary.

## 3. Deploying to the board (Hypothesis — hardware in transit, not yet executed)

The ESP32-S3 exposes a native USB-Serial/JTAG device on the board's USB-C
port; flashing uses `espflash` (v3+):

```sh
cargo install espflash --locked
. "$HOME/export-esp.sh"
espflash flash --chip esp32s3 --monitor \
    target/xtensa-esp32s3-none-elf/release/kittens-render-xtensa-probe
```

`espflash` converts the ELF to an image, writes the bootloader/partition
table, and `--monitor` attaches the serial console after reset. Expected
mechanics to verify at bring-up:

- If the device does not enumerate for flashing, hold **BOOT** while
  tapping **RESET** to force download mode, then release BOOT.
- Board flash/PSRAM sizes (Waveshare wiki lists 16 MB flash, 8 MB octal
  PSRAM for this module) — confirm with `espflash board-info` before
  passing `--flash-size`; PSRAM stays unused until a spec section admits it.
- Runtime logging channel (esp-println over USB-Serial/JTAG vs UART) is an
  open choice for the K2R-1 slice; the compile probe has no logging and
  parks in a spin loop by design.

**Gap: physical flash + boot on the V1 board (no data exists — hardware
not yet arrived).** Record first-flash evidence here when it happens.

## 4. What a green Xtensa build does and does not prove

Revision 12 records the complete K2R-0A feasibility matrix as closed with host
+ portable-link + exact-Xtensa-link scope. The repository fixture contributes
the exact-source link evidence to that matrix; it does not by itself close the
matrix and it does not close K2R-0. The K2R-0 freeze still requires the
bilateral seam and generic capability seals. Target execution, the real board
coordinator, and the physical observations excluded below are K2R-1 work.

For the blocking region row, the host matrix plus exact-HAL link and clean
symbol inspection close only the declared host + exact-Xtensa-link scope: the
API shape exists, ownership works, the complete call path is retained, no
allocator is linked, and vectors bind. Compilation and linking **cannot** prove
silicon interrupt delivery, wake counts, panel command acceptance, physical
placement/color, RAMWRC behavior, TE timing, or visual output — those are
board-HIL gates listed in
`crates/kittens-render/TRACE-MANIFEST.md` and the K2R-1 checklist in
`crates/kittens-render/SPEC.md`. Do not claim them from a build log.

For the closed revision-11 async row, a green source/type/link gate can
additionally show that the branded constructor rejects SPI3, DMA_CH1, and swapped
GPIO4/GPIO5 roles; the real profile transfer fits the admitted inline carrier;
generated rearm/drain and armed-drop code survive optimized linking; the exact
HAL source identity remains pinned; and the noop-waker binary has no matched
allocator symbols. The retained outer hooks are never called. Consequently a
green build is **not** evidence for a target executor, IRQ delivery, wake
counts, completion-versus-cancellation behavior, runtime drop, arbitrary
executor-waker allocation, or physical panel behavior.

## 5. CI link gate (Fact — configured 2026-08-09)

The `xtensa-link` GitHub Actions job runs the section-2 repository-source
release build for
`fixtures/render-xtensa-probe` on every push and pull request. Its enforcement
layer is the CI workflow plus explicit artifact inspection: the job runs
`cargo +esp build --release --locked --target xtensa-esp32s3-none-elf`, then
requires `file` and the Xtensa `readelf` to identify the result as a linked
Tensilica Xtensa executable. Revision 10 additionally requires `nm -u` to be
empty, the concrete blocking wire symbol to remain present, and the section-2
allocator-symbol scan to have zero matches; `cargo check` is not a substitute
for this link and symbol evidence.

Revision 11 adds fail-closed source checks for the profile feature, macro-only
kernel dependency, removed fixture adapter, one-poll shim, and uncalled outer
function-pointer spelling. Three target-only bins must fail with E0308 for
SPI3, DMA_CH1, and swapped SIO0/SIO1 singleton arguments. The optimized ELF is
paired with a positive constructor/extractor target check, then inspected with
`nm -S -C`: the generated-reactor hook, its single-poll shim, and armed-drop
hook must each have a nonzero text symbol, while the undefined and allocator
scans remain clean. The parent-owned revision-11 run passed all three intended
E0308 controls, the Parts compile-pass control, direct exact-target profile
Clippy, fixture Clippy/link, source identity, undefined/allocator scans, and
all three nonzero-symbol assertions with the artifact recorded above.

The job pins `esp-rs/xtensa-toolchain` v1.7.0 by commit SHA, selects Xtensa Rust
1.95.0.0, limits the installed GCC targets to ESP32-S3, and asserts the full
observed GCC mapping (`15.2.0_20250920`) before building. The action itself
downloads the current espup release; espup is **not** version-pinned by the
action. An espup mapping epoch in the cache key plus the exact compiler/GCC
assertions make any changed mapping fail loudly, but do not turn that upstream
download into a reproducible pin. The cache strategy has two deliberately
separate parts:

- cache the named `esp` toolchain under an espup-mapping/compiler/GCC key, while
  still running espup on a cache hit so its version/path checks reuse the
  matching Rust, GCC, and LLVM components and regenerate the environment
  export;
- cache Cargo registry and git downloads under the fixture lockfile key, but
  **do not cache the fixture target directory**, so every job links the current
  source rather than accepting a previously linked ELF.

The build shell copies the action's export to `$HOME/export-esp.sh` and sources
that canonical path before invoking Cargo. A green job closes only the repeatable
CI compile/link gate described in section 4; it does not close any board-HIL or
silicon-behavior gate.

This existing job resolves the exact git HAL revision and is therefore an
explicit non-control for section 2.1's normalized packaged-source + registry-
HAL row. Revision 12 selects a separate recurring clean-package job, but that
job and its evidence remain pending at the status recorded here.

## 6. Troubleshooting (Fact — each observed in this repo)

- `error: unexpected argument '--sandbox'` from `codex exec resume`:
  global flags go **before** the subcommand
  (`codex exec --sandbox workspace-write resume <id> "…"`).
- Sandboxed agents cannot fetch the esp-hal git dependency
  (`failed to create directory ~/.cargo/git/db/…`, `Could not resolve
  host`): run the build from a normal terminal; the sandbox denies network
  and `~/.cargo` writes.
- Link failures mentioning `linkall.x` or `crt0`: the shell did not source
  `$HOME/export-esp.sh`, or `.cargo/config.toml` rustflags are missing.
- Mixed-toolchain artifacts (E0514): never build the same target dir with
  both stock and `esp` toolchains; standalone firmware crates keep their
  own `target/`.
