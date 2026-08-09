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
| `transfer::OwnedTransfer` + `InFlight` | resources (transport, sent buffer, spare) always return on the driven path; cancel settles at its linearization point and wakes; register-then-recheck completion | trait contract + wake-count oracles + broken-order negative control |
| `Settled::stripe_written` | only a `Completed` settlement can mark coverage — cancelled/failed/never-started stripes are unmarkable | unforgeable witness mint (type construction) |
| `sweep::Sweep<S>` | one immutable snapshot per epoch (shared-ref access), demand-fixed validated plan, in-order full coverage before `SweepWritten` | crate-owned value + consuming witnesses |
| `demand::FrameDemand` | one sweep in flight; provenance-branded settlement rejected without mutation; invalidation discards the affected epoch's settlement; dropped sweeps recoverable | checked state machine + per-table-row oracles |
| `touch` | untorn snapshot reports; wake-dedup without the idle-check TOCTOU; bounded service per activation; no edge for unchanged contacts | atomics protocol + adversarial interleaving oracles + negative control |

## What this crate is not

Not a display driver, widget/layout/scene framework, HAL, or executor. It
does not claim physical presentation (milestones are `StripeWritten` /
`SweepWritten` only), TE synchronization, power/AOD management, or DMA
overlap — each is a named gate in the SPEC. Escape surfaces that compile by
design: raw transport access outside the capability boundary, and any
`TouchReader` implementation's "untorn snapshot" property, which is a
documentation-level contract on the integration (the FT3168 integration
discharges it with a single contiguous register read).

## Deferred, with gates

Xtensa compile probe (espup approval) → board HIL (hardware arrival) →
K2R-1 numbers; kernel-admitted source carrier (root SPEC 37.6 slice) →
real `reactor!` fixture; seam co-sign with `kittens-code`; `write_region`
upstream/fork for stripes; draw-target integration → pixel-equivalence
oracle; `OwnedTransfer` sealing before any freeze.
