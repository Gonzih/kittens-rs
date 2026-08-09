# kittens-render

Embedded rendering/interaction profile for the [Kittens](../kittens)
reactor kernel, anchored on the Waveshare ESP32-S3 1.8" AMOLED V1 board
(SH8601 display, FT3168 touch, 368×448). The controlling contract is
[`SPEC.md`](SPEC.md) (revision 6: section 6 is the normative K2R-0
surface); [`K2R0A-LOG.md`](K2R0A-LOG.md) is the experiment record and
[`TRACE-MANIFEST.md`](TRACE-MANIFEST.md) maps every required oracle to its
status. Reviews are retained under [`reviews/`](reviews/).

**Stage:** K2R-0 host slice. Not published; board bring-up (K2R-1) awaits
hardware and the Xtensa toolchain gate.

## What each guarantee rests on

| Component | Guarantee | Enforcement layer |
|---|---|---|
| `StripeTarget::start_flight` + `StartPermit` + `FlightStarter` + `OwnedTransfer` + `InFlight` | the only public flight construction invokes the operation-bound starter with that target's exact region and a crate-issued, private-constructor, non-`Clone`, lifetime-bound permit; a reported rejection returns the starter error, spare, and target through `StartFlightError`; resources (transport, sent buffer, spare) return on the driven path; cancel settles at its linearization point and wakes; register-then-recheck completion; `InFlight<X, S>` is `Unpin` exactly for `X: OwnedTransfer + Unpin, S: Unpin` | consuming target + unforgeable dispatch permit + private flight constructor/fields + seal-at-freeze capability contracts + conditional trait bound + compile-fail and runtime oracles; pairing is structural under sealed integrations, while region honesty and acceptance-atomic rejection remain documented obligations during the open-trait experiment |
| `Settled::into_parts` + `StripeSettlement` | every extraction returns exactly one move-only reconciliation witness: only real `Completed` recovery yields `Written(StripeWritten)`, while cancellation/failure yields `Unwritten(StripeUnwritten)` and cannot be relabeled as coverage; delivery to the owning sweep is cooperative | private settlement construction + consuming resource extraction + distinct private-field witness types + forge/rewrite/replay/clone compile-fail controls for witness integrity; documentation + `must_use` for delivery, which Rust cannot force |
| `PanelGeometry` + `SweepPlan` | the canonical plan is tied to admitted anchor geometry; arbitrary geometry is a visibly named escape | private raw plan constructor + admission type + compile-fail/pass controls |
| `sweep::Sweep<S>` | one owned snapshot value per epoch (shared-reference-only access), one outstanding target per plan position, and settlement-gated progression when the caller delivers the matching witness; an accepted written settlement advances once, an accepted unwritten settlement irreversibly poisons, and only healthy full coverage yields `SweepWritten`; `abort` rejects an outstanding target unchanged | crate-owned state machine + `&mut` target issuance + consuming provenance witnesses + poison/rejection oracles; the cooperative cancel/poll/recover/settle path clears outstanding before abort succeeds |
| `demand::FrameDemand` | one machine-active epoch; provenance-branded settlement rejected without mutation; invalidation discards the affected epoch's settlement, including a sticky idle invalidation transferred to the next minted sweep; dropped/outstanding sweeps recoverable; epochs 0 through `u64::MAX` mint once and throttle deadlines never saturate past `Tick::MAX` | checked state machine and sticky checked horizons/latches + per-table-row oracles; `abandon_active` retains demand and forces full repaint, while reviewed adapters synchronously cancel and disarm dropped flights |
| `touch` | wake-dedup without the idle-check TOCTOU; bounded service per activation; no edge for unchanged contacts; reviewed readers must return untorn snapshots | atomics protocol + adversarial interleaving oracles + negative control; reader atomicity is a documentation obligation |

## Runnable lifecycle

Run `cargo run -p kittens-render --example host_sweep` for the canonical host
model over `PanelGeometry::WAVESHARE_18_V1`: first a complete written frame,
then an accepted-flight shutdown through drain → cancelled settlement →
poisoned sweep → abort/`finish_failed`. It prints every ownership and proof
transition and verifies resource recovery without adding a dependency to the
`no_std` library.

## What this crate is not

Not a display driver, widget/layout/scene framework, HAL, or executor. It
does not claim physical presentation (milestones are `StripeWritten` /
`SweepWritten` only), TE synchronization, power/AOD management, or DMA
overlap — each is a named gate in the SPEC. Escape surfaces that compile by
design: raw transport access outside the capability boundary;
an open experiment integration whose `FlightStarter` ignores the supplied
target region, returns an unrelated prestarted transfer, or starts and then
reports rejection (pairing becomes structural only after reviewed integrations
are sealed);
ordinary drop of `Settled`/`StripeSettlement`, or consuming a settlement in a
wrong-owner `Sweep::settle` rejection (the owning sweep remains outstanding);
`FrameDemand::abandon_active`, which cannot revoke retained old sweeps, targets,
or flights;
`PanelGeometry::custom_unvalidated_panel`; interior-mutability or shared
handles inside a sweep snapshot (logical epoch immutability is a caller
obligation); safe shared/interior-mutable backing between the sent buffer and
spare (`spare_mut` proves ownership of the spare value, not disjoint physical
storage); caller-supplied `Tick` truth beyond regression clamping and its
finite checked `Tick::MAX` horizon; and any `TouchReader` implementation's
"untorn snapshot" property, which is a documentation-level contract on the
integration (a future reviewed FT3168 integration must discharge it with a
single contiguous register read). The UI-pass controls publish the dishonest
`FlightStarter`, custom panel, interior-mutable snapshot,
shared-buffer-backing, and prose-only reader boundaries beside the
compile-fail proof suite.

`Sweep::abort` succeeds only from ready or poisoned state and returns the sweep
unchanged while a target is outstanding. On the cooperative path this adds no
liveness cost: `begin_drain` requests cancellation, `poll_complete` settles by
contract, resource recovery returns one move-only settlement, and matching-
owner `Sweep::settle` clears outstanding (poisoning on cancellation) before
abort is retried. A never-accepted target, dropped flight, or lost/misapplied
settlement instead uses the published recovery boundary: drop every old
target/flight/sweep, then call `FrameDemand::abandon_active`, which retains
demand and forces a full repaint; reviewed adapters synchronously cancel and
disarm a dropped flight. If stale physical work or external invalidation may
overlap replacement, call `FrameDemand::invalidate()` while idle after
abandonment and before replacement. Its sticky latch makes that replacement
non-clearing, so another full repaint remains due.

## Deferred, with gates

Xtensa compile probe (espup approval) → board HIL (hardware arrival) →
K2R-1 numbers; kernel-admitted source carrier (root SPEC 37.6 slice) →
real `reactor!` fixture; seam co-sign with `kittens-code`; `write_region`
upstream/fork for stripes; draw-target integration → pixel-equivalence
oracle; `FlightStarter` and `OwnedTransfer` sealing before any freeze.
