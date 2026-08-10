# kittens-render

Embedded rendering/interaction profile for the [Kittens](../kittens)
reactor kernel, anchored on the Waveshare ESP32-S3 1.8" AMOLED V1 board
(SH8601 display, FT3168 touch, 368×448). The controlling contract is
[`SPEC.md`](SPEC.md) (revision 10: section 6 is the normative K2R-0
surface); [`K2R0A-LOG.md`](K2R0A-LOG.md) is the experiment record and
[`TRACE-MANIFEST.md`](TRACE-MANIFEST.md) maps every required oracle to its
status. Reviews are retained under [`reviews/`](reviews/).

**Stage:** experimental 0.1.x evidence release of the K2R-0 host slice;
protocols are not frozen. The linked Xtensa compile/link feasibility probe is
**CLOSED WITH SCOPE**; post-revision-8 artifact metadata is recorded in the
trace manifest, and CI repeats the linked-ELF gate. Board HIL and silicon
interrupt delivery, K2R-1 measurements, bilateral seam co-sign with
`kittens-code`, blocking `write_region` evidence, and async capability sealing
remain open gates. Revision 10 selects the sealed, profile-owned blocking
adapter contract, but does not close it before its host matrix and exact-HAL
Xtensa link pass. The kernel-admitted completion carrier and real `reactor!`
integration are **CLOSED WITH HOST + PORTABLE-LINK SCOPE**; they make no
silicon claim.

## What each guarantee rests on

| Component | Guarantee | Enforcement layer |
|---|---|---|
| `StripeTarget::start_flight` + `StartPermit` + `FlightStarter` + `OwnedTransfer` + `InFlight` | the only public flight construction invokes the operation-bound starter with that target's exact region and a crate-issued, private-constructor, non-`Clone`, lifetime-bound permit; a reported rejection returns the starter error, spare, and target through `StartFlightError`; resources (transport, sent buffer, spare) return on the driven path; cancel settles at its linearization point and wakes; register-then-recheck completion; `InFlight<X, S>` is `Unpin` exactly for `X: OwnedTransfer + Unpin, S: Unpin` | consuming target + unforgeable dispatch permit + private flight constructor/fields + seal-at-freeze capability contracts + conditional trait bound + compile-fail and runtime oracles; pairing is structural under sealed integrations, while region honesty and acceptance-atomic rejection remain documented obligations during the open-trait experiment |
| `OptionalInlineOneShot<InFlight<...>>` consumer composition | one allocation-free, locally rearmable kernel source retains the accepted flight across lost arbitration and yields its real owned `Settled`; the source is dormant before its handler rearms it, and graceful shutdown borrows the flight for `begin_drain` before settlement | sealed kernel source admission + ordinary ownership + private `InFlight` state/register-then-recheck oracles + real-reactor selection-loss/rearm/drain tests + external Thumb/wasm links; inner-future honesty, cooperative settlement delivery, raw polling/await, mutable future replacement, whole-source drop, and silicon behavior remain named non-guarantees |
| `Settled::into_parts` + `StripeSettlement` | every extraction returns exactly one move-only reconciliation witness: only real `Completed` recovery yields `Written(StripeWritten)`, while cancellation/failure yields `Unwritten(StripeUnwritten)` and cannot be relabeled as coverage; delivery to the owning sweep is cooperative | private settlement construction + consuming resource extraction + distinct private-field witness types + forge/rewrite/replay/clone compile-fail controls for witness integrity; documentation + `must_use` for delivery, which Rust cannot force |
| `PanelGeometry` + `SweepPlan` | the canonical plan is tied to admitted anchor geometry; arbitrary geometry is a visibly named escape | private raw plan constructor + admission type + compile-fail/pass controls |
| `sweep::Sweep<S>` | one owned snapshot value per epoch (shared-reference-only access), one outstanding target per plan position, and settlement-gated progression when the caller delivers the matching witness; an accepted written settlement advances once, an accepted unwritten settlement irreversibly poisons, and only healthy full coverage yields `SweepWritten`; `abort` rejects an outstanding target unchanged | crate-owned state machine + `&mut` target issuance + consuming provenance witnesses + poison/rejection oracles; the cooperative cancel/poll/recover/settle path clears outstanding before abort succeeds |
| `draw_target::Rgb565StripeDrawTarget` (`embedded-graphics` feature) | drawing uses global panel coordinates and full-panel dimensions while clipping and translating into exactly one caller-owned stripe; pixels are row-major RGB565 high byte then low byte | private sweep/target provenance + constructor validation + ordinary mutable borrowing + focused packing/clipping/layout tests + independent full-frame versus real-witness-chain stripe oracles |
| `demand::FrameDemand` | one machine-active epoch; provenance-branded settlement rejected without mutation; invalidation discards the affected epoch's settlement, including a sticky idle invalidation transferred to the next minted sweep; dropped/outstanding sweeps recoverable; epochs 0 through `u64::MAX` mint once and throttle deadlines never saturate past `Tick::MAX` | checked state machine and sticky checked horizons/latches + per-table-row oracles; `abandon_active` retains demand and forces full repaint, while reviewed adapters synchronously cancel and disarm dropped flights |
| `touch` | wake-dedup without the idle-check TOCTOU; bounded service per activation; no edge for unchanged contacts; reviewed readers must return untorn snapshots | atomics protocol + adversarial interleaving oracles + negative control; reader atomicity is a documentation obligation |

## Cargo features

The default feature set is empty: the core remains dependency-free,
`no_std`, no-alloc, and usable without a graphics framework. The optional
`embedded-graphics` feature adds embedded-graphics' `no_std` API and exports
the canonical integration as
`kittens_render::draw_target::{Rgb565StripeDrawTarget, StripeDrawTargetError}`.

The target binds a sweep's current outstanding `StripeTarget` to an exact
caller-owned `&mut [u8]` of `region.width * region.height * 2` bytes. It keeps
full-panel global `Dimensions`, clips all writes outside that stripe, and
stores RGB565 high byte then low byte. It does not clear or reconstruct stale
scratch storage: render the background and complete ordered scene from
`Sweep::snapshot()` for every stripe before consuming the same target through
`start_flight`.

After constructor admission, the exact byte length and stripe clipping jointly
prove that every accepted pixel's local coordinates, row-major byte offset, and
two-byte slice are in range. The draw loop uses that invariant directly rather
than retaining unreachable fallback branches. Checked length arithmetic is
tested independently at both `usize` exhaustion edges; the maximum public
`u16` geometry returns `BufferSizeOverflow` on targets where its byte count is
not representable and the exact `WrongBufferLength` count otherwise.

## Runnable lifecycle

Run `cargo run -p kittens-render --example host_sweep` for the canonical host
model over `PanelGeometry::WAVESHARE_18_V1`: first a complete written frame,
then an accepted-flight shutdown through drain → cancelled settlement →
poisoned sweep → abort/`finish_failed`. It prints every ownership and proof
transition and verifies resource recovery without adding a dependency to the
`no_std` library.

## What this crate is not

Not a complete display driver, widget/layout/scene framework, HAL, or executor.
Revision 10 owns only the minimal section-6.7 SH8601 region transaction; panel
initialization, reset/power/brightness, command coordination, and physical
truth remain outside it. The crate does not claim physical presentation
(milestones are `StripeWritten` /
`SweepWritten` only), TE synchronization, power/AOD management, or DMA
overlap — each is a named gate in the SPEC. The optional stripe target's host
oracles establish byte-exact model reconstruction, not the selected adapter's
`write_region`, DMA/wire delivery, physical RGB channel/byte interpretation,
or panel color fidelity; those remain exact-adapter and board-HIL questions.
Escape surfaces that compile by design: raw transport access outside the
capability boundary;
construction of the blocking transport from an unbranded same-source HAL bus,
whose peripheral/pins/mode/frequency and panel initialization remain caller
obligations;
direct `.await`/manual `poll_complete` instead of reactor admission; wrapping
an inert or lossy future in the generic kernel carrier; replacing the armed
future through `future_mut`'s ordinary `&mut F`; dropping an armed carrier,
which synchronously drops its reviewed flight but returns no resources;
an open experiment integration whose `FlightStarter` ignores the supplied
target region, returns an unrelated prestarted transfer, or starts and then
reports rejection (pairing becomes structural only after reviewed integrations
are sealed);
ordinary drop of `Settled`/`StripeSettlement`, or consuming a settlement in a
wrong-owner `Sweep::settle` rejection (the owning sweep remains outstanding);
dropping the opaque `BlockingSettled` before extraction, which loses its
writer/returned slice value and settlement while leaving the owning sweep
outstanding (drop that sweep, call `abandon_active`, and retain the underlying
pixel storage for full repaint);
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

The pinned Xtensa compile/link feasibility probe is **CLOSED WITH SCOPE**: it
proves the HAL/API/language/ownership/no-allocation/no-self-reference shape,
not behavior on silicon. The kernel-admitted inline completion carrier and
real-reactor gate are separately **CLOSED WITH HOST + PORTABLE-LINK SCOPE**:
deterministic host tests exercise both selection-loss positions and the
downstream generated-reactor fixture links on Thumb and wasm, but the Xtensa
firmware still serves only its scoped compile/link role. Remaining gates are
board HIL (hardware in transit), including silicon interrupt delivery and
physical RGB565/channel/byte fidelity, before K2R-1 measurements; bilateral
seam co-sign with `kittens-code`; implementation evidence for revision 10's
selected sealed, profile-owned blocking `write_region`; and
`FlightStarter`/`OwnedTransfer` sealing before any freeze. Publishing this
experimental 0.1.x evidence release is not that freeze.
