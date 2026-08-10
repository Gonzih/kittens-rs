# kittens-render normalized-package Xtensa probe

This standalone firmware is the local package-shape/registry-HAL consumer
specified by `crates/kittens-render/SPEC.md` section 9.1. It composes Cargo's
clean, normalized `kittens-render` package source with registry `esp-hal`
1.1.0 and does not join the root workspace. The locally generated 0.1.1
archive reflects the current workspace declaration only; it is not a publish
candidate because crates.io 0.1.1 is already an immutable older artifact. The
relative package path is intentional: CI stages this fixture and the extracted
crate in the same `fixtures/` and `target/package/` layout outside the
checkout.

First generate the normalized source from a clean committed checkout:

```sh
cargo +1.96.0 package -p kittens-render --locked
```

Then, from this directory with the Espressif toolchain exported, run:

```sh
. "$HOME/export-esp.sh"
cargo +esp clippy --release --locked --target xtensa-esp32s3-none-elf \
  --manifest-path ../../target/package/kittens-render-0.1.1/Cargo.toml \
  --no-default-features --features esp32s3-sh8601-async --lib -- -D warnings
cargo +esp clippy --release --locked --target xtensa-esp32s3-none-elf -- -D warnings
cargo +esp build --release --locked --target xtensa-esp32s3-none-elf
```

**Fact (source boundary):** the manifest has an empty `[workspace]`, an exact
path-and-version dependency on the generated package, and a direct exact
registry `esp-hal = "=1.1.0"` dependency. It has no git source, `[patch]`,
direct `kittens`, or direct `critical-section` dependency.

**Fact (source boundary):** the `#![no_std]`/`#![no_main]` entrypoint uses
direct registry-HAL SPI2, DMA_CH0, and GPIO4/5/6/7/11/12 singleton values to
call the typed, `#[inline(never)]` `linked_packaged_registry_parts` constructor
hook. That hook returns the packaged `Waveshare18V1Sh8601Parts`, proving the
cross-crate HAL type identity. The entrypoint black-boxes those parts, two
pixel buffers, and both retained function pointers; it never calls the start
hook.

**Fact (source boundary):** the safe, `#[inline(never)]` retained start hook
sizes the two 368x16 RGB565 buffers and crosses `try_new`, the fixed-panel
sweep plan, `FrameDemand::request`/`begin_sweep`, `Sweep::next_target`,
`Waveshare18V1Sh8601Transport::into_start`, and the target-owned
`StripeTarget::start_flight`. Both resource-owning result paths are recovered
or black-boxed.

**Fact (clean revision-12 run, 2026-08-09):** the complete local matrix passed
from clean implementation commit
`c3e234770ce2de9a277e947f8cf8547700abea28`. Cargo 1.96.0 produced a
206,609-byte package archive with SHA-256
`b0bc8d11e477ca4b5f6421bb49db3ada3b45ea1f555af4e5e412dd93dede4ec4`;
its VCS record names that exact commit and `crates/kittens-render`, with no
dirty marker. The staged `.cargo/config.toml` SHA-256 is
`aa32449e2a38ae9ccac1a7b625a6dff109e3f70fc4c59becab5345b63f27e1e9`.
Package and consumer metadata contain exactly one registry `esp-hal` 1.1.0,
checksum
`6af8fa8216bc126941bd43b5a200a50eab16e43881ccd0dd0b6792f4a82805f0`,
and zero git packages. Both packaged-library and consumer target Clippy
passed.

**Fact (clean link artifact):** the optimized locked ELF is 206,248 bytes with
SHA-256
`5ce57e9e9875f900e1c89987d56dc8fa78a383a041235f175cde4686dd5bdf75`,
entry `0x403785e8`, and 56,680 bytes of `.bss`. Undefined-symbol and allocator-
symbol-filter counts are zero. `nm -a -S -C` retains
`linked_packaged_registry_parts` at `0x20` bytes and
`linked_packaged_registry_start` at `0x16bd` bytes. The local row is therefore
**CLOSED WITH PACKAGED-SOURCE + REGISTRY-HAL XTENSA-LINK SCOPE**. The recurring
CI job repeats the matrix; any individual GitHub run is independent check
evidence and is not part of this local artifact claim.

**Observation:** a passing locked target link proves compatibility between
the locally packaged public adapter and the registry HAL source identity for
this exact configuration. It does not publish or download `kittens-render`,
exercise the retained hook, provide an executor, observe SPI2 interrupts or
wakes, or prove cancellation, drop, panel, touch, timing, or allocation
behavior on hardware. It does not close the K2R-0 protocol freeze; bilateral
seam co-sign and generic capability sealing remain open, and the runtime/HIL
questions below remain K2R-1 work.

**Gap: target execution, interrupt/wake behavior, cancellation/drop on
silicon, panel and touch behavior, timing, and measured resource budgets (no
data exists).** Those remain K2R-1 gates.
