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

**Observation:** a passing locked target link proves compatibility between
the locally packaged public adapter and the registry HAL source identity for
this exact configuration. It does not publish or download `kittens-render`,
exercise the retained hook, provide an executor, observe SPI2 interrupts or
wakes, or prove cancellation, drop, panel, touch, timing, or allocation
behavior on hardware.

**Gap: target execution, interrupt/wake behavior, cancellation/drop on
silicon, panel and touch behavior, timing, and measured resource budgets (no
data exists).** Those remain K2R-1 gates.
