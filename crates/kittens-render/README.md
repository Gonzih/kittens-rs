# kittens-render

Embedded rendering/interaction profile for the [Kittens](../kittens)
reactor kernel, anchored on the Waveshare ESP32-S3 1.8" AMOLED V1 board
(SH8601 display, FT3168 touch, 368×448). The controlling contract is
[`SPEC.md`](SPEC.md) (revision 4: section 6 is the normative K2R-0
surface); [`K2R0A-LOG.md`](K2R0A-LOG.md) is the experiment record and
[`TRACE-MANIFEST.md`](TRACE-MANIFEST.md) maps every required oracle to its
status. Reviews are retained under [`reviews/`](reviews/).

**Stage:** K2R-0 host slice. Not published; board bring-up (K2R-1) awaits
hardware and the Xtensa toolchain gate.

## What each guarantee rests on

| Component | Guarantee | Enforcement layer |
|---|---|---|
| `StripeTarget::start_flight` + `OwnedTransfer` + `InFlight` | the only public flight construction invokes the starter with that target's exact region; a rejected start returns `E`, spare, and target through `StartFlightError`; resources (transport, sent buffer, spare) return on the driven path; cancel settles at its linearization point and wakes; register-then-recheck completion; `InFlight<X, S>` is `Unpin` exactly for `X: OwnedTransfer + Unpin, S: Unpin` | consuming target + private flight constructor + trait contract + conditional trait bound + structural/start-error/runtime oracles; adapter honesty remains a documented admission obligation |
| `Settled::into_parts` + `StripeSettlement` | every recovered transfer produces exactly one move-only reconciliation witness: only real `Completed` recovery yields `Written(StripeWritten)`, while cancellation/failure yields `Unwritten(StripeUnwritten)` and cannot be relabeled as coverage | private settlement construction + consuming resource extraction + distinct private-field witness types + forge/rewrite/replay/clone compile-fail controls |
| `PanelGeometry` + `SweepPlan` | the canonical plan is tied to admitted anchor geometry; arbitrary geometry is a visibly named escape | private raw plan constructor + admission type + compile-fail/pass controls |
| `sweep::Sweep<S>` | one owned snapshot value per epoch (shared-reference-only access), one outstanding target per plan position, and mandatory settlement; written advances once, unwritten irreversibly poisons, and only healthy full coverage yields `SweepWritten` | crate-owned state machine + `&mut` target issuance + consuming provenance witnesses + poison/rejection oracles; `abort` intentionally remains available during shutdown |
| `demand::FrameDemand` | one machine-active epoch; provenance-branded settlement rejected without mutation; invalidation discards the affected epoch's settlement; dropped sweeps recoverable; epochs 0 through `u64::MAX` mint once and throttle deadlines never saturate past `Tick::MAX` | checked state machine and sticky checked horizons + per-table-row oracles; caller drains old physical transfers before replacing an abandoned epoch |
| `touch` | wake-dedup without the idle-check TOCTOU; bounded service per activation; no edge for unchanged contacts; reviewed readers must return untorn snapshots | atomics protocol + adversarial interleaving oracles + negative control; reader atomicity is a documentation obligation |

## Runnable lifecycle

Run `cargo run -p kittens-render --example host_sweep` for the canonical
host-model demand → sweep → target-driven start → mandatory per-stripe
settlement → written-sweep settlement cycle over
`PanelGeometry::WAVESHARE_18_V1`. It prints every ownership and
proof transition without adding a dependency to the `no_std` library.

## What this crate is not

Not a display driver, widget/layout/scene framework, HAL, or executor. It
does not claim physical presentation (milestones are `StripeWritten` /
`SweepWritten` only), TE synchronization, power/AOD management, or DMA
overlap — each is a named gate in the SPEC. Escape surfaces that compile by
design: raw transport access outside the capability boundary;
an open integration whose starter safely ignores the supplied target region
(structural pairing is not adapter honesty);
`PanelGeometry::custom_unvalidated_panel`; interior-mutability or shared
handles inside a sweep snapshot (logical epoch immutability is a caller
obligation); safe shared/interior-mutable backing between the sent buffer and
spare (`spare_mut` proves ownership of the spare value, not disjoint physical
storage); caller-supplied `Tick` truth beyond regression clamping and its
finite checked `Tick::MAX` horizon; and any `TouchReader` implementation's
"untorn snapshot" property, which is a documentation-level contract on the
integration (a future reviewed FT3168 integration must discharge it with a
single contiguous register read). The UI-pass controls publish the dishonest
starter, custom panel, interior-mutable snapshot, shared-buffer-backing, and
prose-only reader boundaries beside the compile-fail proof suite.

`Sweep::abort` is deliberately always available for shutdown, even with a
target or transfer outstanding. It terminates bookkeeping but cannot revoke
either value: a retained target can still be started, and a live flight can
still write. Drop the target and drain the flight before replacement when
possible. Accepting the abort retains a forced full repaint, and callers invoke
`FrameDemand::invalidate()` if a stale write may overlap a replacement so that
suspect replacement cannot clear the obligation.

## Deferred, with gates

Xtensa compile probe (espup approval) → board HIL (hardware arrival) →
K2R-1 numbers; kernel-admitted source carrier (root SPEC 37.6 slice) →
real `reactor!` fixture; seam co-sign with `kittens-code`; `write_region`
upstream/fork for stripes; draw-target integration → pixel-equivalence
oracle; `OwnedTransfer` sealing before any freeze.
