# kittens-render

Embedded rendering/interaction profile for the [Kittens](../kittens)
reactor kernel, anchored on the Waveshare ESP32-S3 1.8" AMOLED V1 board
(SH8601 display, FT3168 touch, 368×448). The controlling contract is
[`SPEC.md`](SPEC.md) (revision 3: section 6 is the normative K2R-0
surface); [`K2R0A-LOG.md`](K2R0A-LOG.md) is the experiment record and
[`TRACE-MANIFEST.md`](TRACE-MANIFEST.md) maps every required oracle to its
status. Reviews are retained under [`reviews/`](reviews/).

**Stage:** K2R-0 host slice. Not published; board bring-up (K2R-1) awaits
hardware and the Xtensa toolchain gate.

## What each guarantee rests on

| Component | Guarantee | Enforcement layer |
|---|---|---|
| `transfer::OwnedTransfer` + `InFlight` | resources (transport, sent buffer, spare) always return on the driven path; cancel settles at its linearization point and wakes; register-then-recheck completion; `InFlight<X, S>` is `Unpin` exactly for `X: OwnedTransfer + Unpin, S: Unpin` | trait contract + conditional trait bound + wake-count oracles + broken-order negative control |
| `Settled::stripe_written` | only a private, real `Completed` settlement can mint coverage, at most once — cancelled/failed/never-started stripes are unmarkable | private construction + consuming unforgeable witness mint + compile-fail suite |
| `PanelGeometry` + `SweepPlan` | the canonical plan is tied to admitted anchor geometry; arbitrary geometry is a visibly named escape | private raw plan constructor + admission type + compile-fail/pass controls |
| `sweep::Sweep<S>` | one owned snapshot value per epoch (shared-reference-only access), demand-fixed validated plan, in-order full coverage before `SweepWritten` | crate-owned value + ordinary borrowing + consuming witnesses |
| `demand::FrameDemand` | one machine-active epoch; provenance-branded settlement rejected without mutation; invalidation discards the affected epoch's settlement; dropped sweeps recoverable | checked state machine + per-table-row oracles; caller drains old physical transfers before replacing an abandoned epoch |
| `touch` | wake-dedup without the idle-check TOCTOU; bounded service per activation; no edge for unchanged contacts; reviewed readers must return untorn snapshots | atomics protocol + adversarial interleaving oracles + negative control; reader atomicity is a documentation obligation |

## Runnable lifecycle

Run `cargo run -p kittens-render --example host_sweep` for the canonical
host-model demand → sweep → per-stripe transfer/proof → written-settlement
cycle over `PanelGeometry::WAVESHARE_18_V1`. It prints every ownership and
proof transition without adding a dependency to the `no_std` library.

## What this crate is not

Not a display driver, widget/layout/scene framework, HAL, or executor. It
does not claim physical presentation (milestones are `StripeWritten` /
`SweepWritten` only), TE synchronization, power/AOD management, or DMA
overlap — each is a named gate in the SPEC. Escape surfaces that compile by
design: raw transport access outside the capability boundary;
`PanelGeometry::custom_unvalidated_panel`; interior-mutability or shared
handles inside a sweep snapshot (logical epoch immutability is a caller
obligation); caller-supplied `Tick` truth beyond regression clamping; and
any `TouchReader` implementation's "untorn snapshot" property, which is a
documentation-level contract on the integration (a future reviewed FT3168
integration must discharge it with a single contiguous register read). The
UI-pass controls publish the custom-panel, interior-mutable-snapshot, and
prose-only-reader boundaries beside the compile-fail proof suite.

## Deferred, with gates

Xtensa compile probe (espup approval) → board HIL (hardware arrival) →
K2R-1 numbers; kernel-admitted source carrier (root SPEC 37.6 slice) →
real `reactor!` fixture; seam co-sign with `kittens-code`; `write_region`
upstream/fork for stripes; draw-target integration → pixel-equivalence
oracle; `OwnedTransfer` sealing before any freeze.
