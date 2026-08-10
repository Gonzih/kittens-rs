# kittens-render

Embedded rendering/interaction profile for the [Kittens](../kittens)
reactor kernel, anchored on the Waveshare ESP32-S3 1.8" AMOLED V1 board
(SH8601 display, FT3168 touch, 368×448). The controlling contract is
[`SPEC.md`](SPEC.md) (revision 12: section 6 is the normative K2R-0
surface and sections 9–11 own package/K2R-1/staging gates);
[`K2R0A-LOG.md`](K2R0A-LOG.md) is the experiment record and
[`TRACE-MANIFEST.md`](TRACE-MANIFEST.md) maps every required oracle to its
status. Reviews are retained under [`reviews/`](reviews/).

**Stage:** experimental 0.1.x evidence release; protocols are not frozen.
K2R-0A is **CLOSED WITH HOST + PORTABLE-LINK + EXACT-XTENSA-LINK SCOPE**:
the selected design, finite host traces, portable links, and exact-HAL target
links pass, without claiming target execution or silicon behavior. Full K2R-0
freeze is gated on exactly the bilateral `kittens-code` seam/foreign fixture
and generic async capability sealing at a breaking API boundary. K2R-1 owns a
real target executor, board coordinator, SPI2/TP_INT delivery, board HIL, and
measurements. Revision 12 separately selects a clean packaged-source +
registry-HAL Xtensa consumer gate, which is still **OPEN**. Upload/index/
download for any future correctly versioned release remains a human-ordered
action. Revision 10's
sealed, profile-owned blocking
`write_region` row is **CLOSED WITH HOST + EXACT-XTENSA-LINK SCOPE**. The
kernel-admitted completion carrier and real `reactor!` integration are
separately **CLOSED WITH HOST + PORTABLE-LINK SCOPE**. Neither closure makes a
silicon claim. Revision 11 selects a board-branded, single-payload concrete
async adapter; its evidence row is **CLOSED WITH HOST + EXACT-XTENSA-REACTOR-
LINK SCOPE** after the host matrix and exact-HAL target link/source/symbol
gates passed. This closure likewise makes no target-runtime or silicon claim.

## What each guarantee rests on

| Component | Guarantee | Enforcement layer |
|---|---|---|
| `StripeTarget::start_flight` + `StartPermit` + `FlightStarter` + `OwnedTransfer` + `InFlight` | the only public flight construction invokes the operation-bound starter with that target's exact region and a crate-issued, private-constructor, non-`Clone`, lifetime-bound permit; a reported rejection returns the starter error, spare, and target through `StartFlightError`; resources (transport, sent buffer, spare) return on the driven path; cancel settles at its linearization point and wakes; register-then-recheck completion; `InFlight<X, S>` is `Unpin` exactly for `X: OwnedTransfer + Unpin, S: Unpin` | consuming target + unforgeable dispatch permit + private flight constructor/fields + seal-at-K2R-0-breaking-freeze capability contracts + conditional trait bound + compile-fail and runtime oracles; pairing is structural under sealed integrations, while region honesty and acceptance-atomic rejection remain documented obligations during the current 0.1.x open-trait interval |
| revision-11 concrete async adapter | the profile-owned Waveshare V1 starter accepts one exact region of at most 16,380 logical bytes, shares the blocking engine's CASET/PASET truth, then owns one RAMWR DMA transfer through the existing `InFlight`/`Settled` path | exact peripheral singleton types + private branded transport/start/transfer state + shared private engine + ordinary resource ownership + reviewed concrete-Waker interrupt slot + exact host failure/lifecycle matrix + three target type failures/one Parts pass + direct target Clippy + generated-reactor/drop-glue exact-HAL link/source/symbol gates. External trait implementations, async RAMWRC, synchronous CASET/PASET blocking inside start, arbitrary executor-waker allocation behavior, target execution, and silicon behavior remain explicit non-guarantees |
| revision-12 packaged-source compatibility gate | the clean normalized package and a standalone consumer must resolve one registry `esp-hal =1.1.0` type identity and compile the public board constructor/async-start path for Xtensa | **OPEN:** exact embedded VCS HEAD/path/clean provenance + Cargo-normalized manifest metadata + fixed-path standalone consumer + direct packaged-library/consumer target Clippy under the fixture's Xtensa `build-std` config + locked link and undefined/allocator/retained-symbol inspections. Host normalization, copied source, `[patch]`, the exact-git fixture, and publication are explicit non-controls |
| `StripeTarget::write_region` + `BlockingWritePermit` + `BlockingRegionWrite` + `BlockingSettled` | the sole blocking spelling consumes the outstanding target, exact mutable pixel slice, and admitted writer; only the profile-owned ESP32-S3/SH8601 adapter may implement the sealed capability; every ordinary return carries the same writer and slice plus one owning-sweep written or failed settlement | sealed trait admission + private permit/engine/result/witness state + ordinary ownership + the exact eight-call host trace, eight injected failure boundaries, ten preflight cases, eight compile-fail controls, one explicit-drop compile-pass control, and the exact-HAL no-allocator Xtensa link/symbol gate; raw HAL access, unbranded bus configuration, result drop, handler blocking, and every physical-panel property remain named non-guarantees |
| `OptionalInlineOneShot<InFlight<...>>` consumer composition | one allocation-free, locally rearmable kernel source retains the accepted flight across lost arbitration and yields its real owned `Settled`; the source is dormant before its handler rearms it, and graceful shutdown borrows the flight for `begin_drain` before settlement | sealed kernel source admission + ordinary ownership + private `InFlight` state/register-then-recheck oracles + real-reactor selection-loss/rearm/drain tests + external Thumb/wasm links; inner-future honesty, cooperative settlement delivery, raw polling/await, mutable future replacement, whole-source drop, and silicon behavior remain named non-guarantees |
| `Settled::into_parts` + `StripeSettlement` | every extraction returns exactly one move-only reconciliation witness: only real `Completed` recovery yields `Written(StripeWritten)`, while cancellation/failure yields `Unwritten(StripeUnwritten)` and cannot be relabeled as coverage; delivery to the owning sweep is cooperative | private settlement construction + consuming resource extraction + distinct private-field witness types + forge/rewrite/replay/clone compile-fail controls for witness integrity; documentation + `must_use` for delivery, which Rust cannot force |
| `PanelGeometry` + `SweepPlan` | the canonical plan is tied to admitted anchor geometry; arbitrary geometry is a visibly named escape | private raw plan constructor + admission type + compile-fail/pass controls |
| `sweep::Sweep<S>` | one owned snapshot value per epoch (shared-reference-only access), one outstanding target per plan position, and settlement-gated progression when the caller delivers the matching witness; an accepted written settlement advances once, an accepted unwritten settlement irreversibly poisons, and only healthy full coverage yields `SweepWritten`; `abort` rejects an outstanding target unchanged | crate-owned state machine + `&mut` target issuance + consuming provenance witnesses + poison/rejection oracles; the cooperative cancel/poll/recover/settle path clears outstanding before abort succeeds |
| `draw_target::Rgb565StripeDrawTarget` (`embedded-graphics` feature) | drawing uses global panel coordinates and full-panel dimensions while clipping and translating into exactly one caller-owned stripe; pixels are row-major RGB565 high byte then low byte | private sweep/target provenance + constructor validation + ordinary mutable borrowing + focused packing/clipping/layout tests + independent full-frame versus real-witness-chain stripe oracles |
| `demand::FrameDemand` | one machine-active epoch; provenance-branded settlement rejected without mutation; invalidation discards the affected epoch's settlement, including a sticky idle invalidation transferred to the next minted sweep; dropped/outstanding sweeps recoverable; epochs 0 through `u64::MAX` mint once and throttle deadlines never saturate past `Tick::MAX` | checked state machine and sticky checked horizons/latches + per-table-row oracles; `abandon_active` retains demand and forces full repaint, while reviewed adapters synchronously cancel and disarm dropped flights |
| `touch` | host generation/latch admission provides wake-dedup without the idle-check TOCTOU, bounded service per activation, and no edge for unchanged contacts; reviewed readers must return untorn snapshots | atomics protocol + adversarial host interleaving oracles + negative control; concrete TP_INT delivery and an FT3168 contiguous transaction are K2R-1, while reader atomicity remains a documentation obligation until that integration exists |

## Cargo features

The default feature set is empty: the core remains dependency-free,
`no_std`, no-alloc, and usable without a graphics framework. The optional
`embedded-graphics` feature adds embedded-graphics' `no_std` API and exports
the canonical integration as
`kittens_render::draw_target::{Rgb565StripeDrawTarget, StripeDrawTargetError}`.
The default-off `esp32s3-sh8601-blocking` feature is target-only and exports
the exact-HAL adapter as
`kittens_render::esp32s3_sh8601::Esp32s3Sh8601BlockingTransport` on Xtensa.
The additive `esp32s3-sh8601-async` feature depends on that target support and
exports the board-branded single-payload
`Waveshare18V1Sh8601Parts`, `Waveshare18V1Sh8601Transport`,
`Waveshare18V1Sh8601Start`, `Waveshare18V1Sh8601StartError`,
`Waveshare18V1Sh8601Transfer`, and borrowed
`Waveshare18V1Sh8601IdleCommands` roles under
`kittens_render::esp32s3_sh8601_async` on Xtensa.
Repository builds pin `esp-hal` revision
`d48f747ba28accdc51779ba193eba923138e0382`; the feature requires the
Espressif toolchain, while feature-off portable builds retain the Rust 1.85
floor and empty normal dependency graph. Cargo packages normalize the target
dependency to registry `esp-hal =1.1.0`; revision 12's separate package gate
must exercise that normalized source rather than infer compatibility from the
repository git build.

The target binds a sweep's current outstanding `StripeTarget` to an exact
caller-owned `&mut [u8]` of `region.width * region.height * 2` bytes. It keeps
full-panel global `Dimensions`, clips all writes outside that stripe, and
stores RGB565 high byte then low byte. It does not clear or reconstruct stale
scratch storage: render the background and complete ordered scene from
`Sweep::snapshot()` for every stripe before consuming the same target through
exactly one transport operation: asynchronous `start_flight` or blocking
`write_region`.

After constructor admission, the exact byte length and stripe clipping jointly
prove that every accepted pixel's local coordinates, row-major byte offset, and
two-byte slice are in range. The draw loop uses that invariant directly rather
than retaining unreachable fallback branches. Checked length arithmetic is
tested independently at both `usize` exhaustion edges; the maximum public
`u16` geometry returns `BufferSizeOverflow` on targets where its byte count is
not representable and the exact `WrongBufferLength` count otherwise.

**Fact:** revision 10's host suite records the reference 368×112 transaction
as exactly CASET, PASET, RAMWR, and five RAMWRC calls; injects failure at each
of the eight boundaries; and exercises ten ordered preflight failures plus the
first-row, 1×1, exact-right/bottom, and nonzero-origin positive controls. The
eight new compile-fail fixtures reject external capability implementation,
permit construction/cloning/lifetime escape/direct dispatch, result forgery,
and result-to-settlement laundering. The adjacent compile-pass control
publishes ordinary opaque-result drop as an escape.

**Fact:** the exact-HAL fixture source deliberately shortens its TX descriptor
chain to one byte before construction. Its linked, unexecuted entry path
contains the admission restore to 16,380, the same multichunk engine call over
SPI2/GDMA_CH0, and checks for the returned bus, RX/TX scratch, pixel pointer,
and owning-sweep settlement. The fresh optimized ELF is
208,496 bytes with SHA-256
`648e43a0c03d89d71737d7dd20ff0390d6275b08b4f1f297d15d443af6c68513`,
entry point `0x403785e8`, and 116,988 bytes of `.bss`; undefined symbols are
empty, allocator-symbol matches are zero, and the concrete wire symbol is
retained. CI repeats the locked link and both symbol assertions.

**Fact:** revision 11's host suite executes the literal three-call 368×16
CASET/PASET/RAMWR trace, all three injected failures, ordered async preflight,
scratch admission, concrete-Waker register/recheck/cancel/drop/reuse, shaped
resource recovery, and owning-sweep outcomes. Three independent target bins
reject SPI3, DMA_CH1, and swapped SIO0/SIO1; the exact Parts roundtrip passes.
Direct profile and fixture target Clippy pass. The fresh locked optimized ELF
is 214,352 bytes with SHA-256
`30cd240176d206d6483e04fd0f2384ced2b101491ff6e516ec635a4bbd98664a`,
entry `0x403785e8`, and 115,492 bytes of `.bss`; undefined and allocator-symbol
scans are empty. Nonzero text symbols retain the generated-reactor hook
(`0x168`), one-poll shim (`0xaf6`), and armed-source drop hook (`0x137`). The
hooks are uncalled link evidence, not target execution evidence.

**Observation:** historical host package verification succeeds and Cargo's
generated registry manifest retains `esp-hal =1.1.0` while dropping the
repository-only git location, as its multiple-locations rule specifies. That
dirty-worktree host run verifies only normalization. Revision 12's open local
gate requires a clean current-HEAD archive and an external Xtensa consumer of
the unpacked package; no publication is required. A future exact-version
crates.io download smoke remains separately human-ordered.

## Runnable lifecycle

Run `cargo run -p kittens-render --example host_sweep` for the canonical host
model over `PanelGeometry::WAVESHARE_18_V1`: first a complete written frame,
then an accepted-flight shutdown through drain → cancelled settlement →
poisoned sweep → abort/`finish_failed`. It prints every ownership and proof
transition and verifies resource recovery without adding a dependency to the
`no_std` library.

## What this crate is not

Not a complete display driver, widget/layout/scene framework, HAL, or executor.
Revision 10 owns only the minimal section-6.7 SH8601 region transaction, and
revision 11 owns only section 6.8's single-payload async composition; panel
initialization, reset/power/brightness, command coordination, and physical
truth remain outside it. The crate does not claim physical presentation
(milestones are `StripeWritten` /
`SweepWritten` only), TE synchronization, power/AOD management, or DMA
overlap — each is a named gate in the SPEC. The optional stripe target's host
oracles establish byte-exact model reconstruction. Revision 10's separate host
wire trace and exact-HAL link establish the admitted transaction and compile/
link shape, not physical DMA delivery, panel command acceptance, RGB channel/
byte interpretation, or color fidelity; those remain board-HIL questions.
Escape surfaces that compile by design: raw transport access outside the
capability boundary;
construction of the blocking transport from an unbranded same-source HAL bus,
whose peripheral/pins/mode/frequency and panel initialization remain caller
obligations;
revision 11's temporary `with_idle_commands` board-command escape, whose private-
field borrowed facade prevents moving, replacing, or reconfiguring the branded
bus but whose arbitrary commands, termination, serialization, blocking, and
panel-state truth remain unchecked and which is not the K2R-1 coordinator;
ordinary drop of an idle revision-11 branded transport, which intentionally
returns no SPI/DMA/pin/scratch resources and proves no hardware reset;
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

- **K2R-0A is closed with host + portable-link + exact-Xtensa-link scope.**
  The kernel carrier, blocking adapter, and concrete async adapter retain their
  separately recorded scoped closures. None proves target execution or
  silicon behavior.
- **K2R-0 freeze is gated on exactly two items:** bilateral seam co-sign plus a
  foreign harness fixture, and sealing `FlightStarter`/`OwnedTransfer` at an
  explicitly authorized breaking API boundary. Publication is not a protocol-
  freeze prerequisite.
- **K2R-1 is gated with no target-runtime/HIL data:** real executor polling,
  minimal board coordination, SPI2 and TP_INT delivery, FT3168 contiguous-read
  integration, physical panel/touch/TE behavior, and latency/memory/bandwidth
  measurements remain open. Uncalled linked hooks are non-controls.
- **Package compatibility is independently open:** the clean packaged-source
  + registry-HAL Xtensa consumer matrix must pass. The locally generated 0.1.1
  archive reflects the workspace declaration but is not eligible for
  republishing; crates.io 0.1.1 is older immutable source. A correctly
  versioned future-release registry download smoke is a separate human-ordered
  action, not a K2R acceptance condition and not authorized by this work.
- Async RAMWRC/multichunk operation remains deferred to a later measured
  contract.
