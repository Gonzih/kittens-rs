# kittens-render profile specification (K2R-0A / K2R-0 / K2R-1 staging)

- Status: revision 12, 2026-08-09. The K2R-0A feasibility experiment is now
  **CLOSED WITH HOST + PORTABLE-LINK + EXACT-XTENSA-LINK SCOPE**: the selected
  carrier/completion shape, finite host traces, portable consumer links, and
  exact-HAL Xtensa adapter links all pass. This is a design and compile/link
  result, not target execution or silicon evidence. Revision 12 also selects
  one local **normalized packaged-source + registry-HAL Xtensa
  consumer** gate; that row remains **OPEN** until the clean-package matrix
  below passes. Upload, index availability, and exact-version download for any
  future correctly versioned release remain a separate human-ordered
  publication gate. Revision 11's
  additive, non-freezing async-region
  contract selects one profile-owned, board-branded ESP32-S3/SH8601 adapter
  before implementation. One accepted flight emits the exact window and one
  owning RAMWR payload of at most 16,380 bytes; its evidence row is now
  **CLOSED WITH HOST + EXACT-XTENSA-REACTOR-LINK SCOPE** after the host matrix,
  exact-HAL target Clippy/link, and source/symbol inspections passed. Generic
  `FlightStarter`/`OwnedTransfer` sealing remains reserved for the breaking
  freeze boundary. Revision 10's blocking-region contract selected one sealed,
  profile-owned ESP32-S3/SH8601 adapter and its exact command/failure matrix
  were specified before implementation; the gate is now **CLOSED WITH HOST +
  EXACT-XTENSA-LINK SCOPE** after the protocol suite, exact-HAL link, and
  symbol inspections passed. Every physical-panel claim remains open.
  Revision 9's kernel-carrier
  contract specifies the one no-allocation source shape and closes its
  real-reactor gate with host + portable-link scope; target execution and
  silicon remain open.
  Revision 8's publication-readiness correction recorded that the linked
  Xtensa compile/link feasibility probe is closed with scope; task
  wakers are cloned, dropped, and woken outside the adapter's global critical
  section; publication as an experimental 0.1.x evidence release is explicitly
  not a protocol freeze. Revision 7 added the optional global-coordinate
  RGB565 stripe target and closed the host pixel-equivalence oracle row.
  Earlier revision history remains in section 12.
- Parent contracts: root [`SPEC.md`](../../SPEC.md); [`RESEARCH.md`](RESEARCH.md) revision 5; [`crates/kittens-tui/SPEC.md`](../kittens-tui/SPEC.md) section 10 (generic-gate comparison, unresolved here); the sibling harness contract `docs/kittens-code/SPEC.md` (seam obligations, section 10 below).
- Hardware anchor: **Waveshare ESP32-S3 1.8" AMOLED Touch, V1 — SH8601 display, FT3168 touch, 368×448** (`LCD_TE` GPIO13, `TP_INT` GPIO21, schematic-confirmed).
- Normativity: **MUST/SHOULD** language binds sections 5 through 11. Section 6
  became normative in revision 3; revision 9 specifies the kernel-admitted
  source-carrier shape, revision 10 specifies the blocking-region shape, and
  revision 11 specifies the concrete single-payload async-region shape.
  Revision 12 specifies the normalized packaged-source target-consumer gate
  and repairs the K2R-0A/K2R-0/K2R-1 acceptance staging.
- Slice boundary: **K2R-0 host slice** means the host-model protocol surface
  and oracles may land against amended section 6. K2R-0A's selected design and
  compile/link experiment is now closed with the scope above; that closure does
  not mean full K2R-0 acceptance. The exact Xtensa compile/link feasibility probe is
  **CLOSED WITH SCOPE**: it establishes HAL/API/language/ownership,
  no-allocation, and no-self-reference feasibility, not behavior on silicon.
  The kernel-carrier gate is separately closed with host + portable-link scope.
  The revision-10 blocking `write_region` row is separately closed with host +
  exact-Xtensa-link scope. The revision-11 concrete async adapter row is
  separately closed with host + exact-Xtensa-reactor-link scope. Bilateral seam
  co-sign and async capability sealing gate the K2R-0 freeze. The local
  normalized-package registry-HAL consumer remains open; actual publication is
  human-ordered and orthogonal. Board HIL, silicon interrupt delivery, and
  target-side reactor execution belong to K2R-1 and remain separately named
  gates below.

## 1. One-sentence definition

`kittens-render` is the embedded rendering/interaction profile of Kittens: transport capabilities with resource-carrying results, epoch-snapshotted full-panel sweeps with a consuming progress token, a generation-latched touch protocol with honest latest-state semantics, and frame-demand policy — so a bare-metal application's display and input pipelines are declared reactor topology in the same vocabulary as its backend.

## 2. Problem statement

Unchanged from revision 1 in substance; evidence in RESEARCH sections 2–6. On the anchor board: 329,728-byte frames with a ~16.5 ms QSPI wire floor; a display driver whose framebuffer is private; a HAL completion that is borrowing and `!Unpin` — and, sharpened by this revision's review, *unnameable as a stable associated type without allocation*; touch reads that tear across I²C transactions; stripe buffers that carry no history. Each becomes a type, a rule, or a named gate — in that order of preference, and only after its gate passes.

## 3. Consumers and the merge seam

1. **The sibling harness workstream** (`kittens-code`). Its current contract negotiates a protocol-event frontend seam and owns one reactor per session; it does not yet authorize renderer task loops. The merge is therefore a **bilateral gate** (section 10): one seam section, mirrored in both specs, agreed by both workstreams — this spec does not assume loop ownership, and every fact it emits must be an ordinary typed event a foreign reactor arm can consume.
2. **Application authors** on the anchor board.
3. **Component/engine libraries** above the optional K2R-0 draw-target contract; widgets/layout/scenes are never owned here.

Emittability rule (root 9.4) stands: explicit constructors, stable spellings, no context-dependent sugar — revision 1 violated this by showing typestates with private fields and no constructors; the K2R-0A amendment MUST specify complete construction/transition/teardown APIs for whatever shapes it selects.

## 4. Non-goals

As revision 1 (no widgets; no general display-driver framework beyond sections
6.7–6.8's minimal reviewed SH8601 region transactions; not the generic-gate
resolution; no power/AOD — board-coordinator slice; no DMA overlap — K2R-2
gate; no TE synchronization claim), plus review-sharpened exclusions:

- **no `BusIdle` or `FramePresented` facts in these slices** — both transports expose exactly one observable completion boundary; physical presentation and bus-idle milestones wait for hardware evidence (finding 17). The facts are the private, provenance-carrying `StripeWritten` and `SweepWritten` witnesses only;
- **no lossless touch-transition promise** — this slice's touch semantics are *latest-state-with-coalescing, complete untorn reports*; a bounded transition queue with explicit overflow policy is a separately gated follow-on (finding 11);
- **no damage/partial sweeps** — K2R-0 uses one validated, fixed full-panel sweep plan; damage history is deferred (finding 9).

## 5. What is stable in this revision (normative)

1. **Resource-carrying results.** Every driven asynchronous success or failure
   settles through `Recovered`/`Settled` and returns the transport, sent
   buffer, and spare. The separate synchronous path settles through
   `BlockingSettled` and returns its writer and exact pixel slice. Ordinary
   `drop` of an in-flight completion is a **documented non-returning boundary**
   — the HAL cancels and drops; nothing comes back through `Future::Output`.
   Recovery on cancellation therefore REQUIRES an explicit cancel-and-drain
   transition that is driven to settlement (finding 3).
2. **Capability admission.** `OwnedTransfer` and `FlightStarter` will be
   sealed to reviewed backend adapters at the K2R-0 breaking freeze, because ownership
   alone cannot distinguish an honest region start, acceptance-atomic
   rejection, or drop cancellation from a dishonest implementation (finding
   8; exit-review round-4 finding 1). During the experiment they are
   deliberately open so probes and models can implement them (section 6.2);
   the open state is itself a recorded gate, not a contradiction. The new
   blocking capability has one selected production implementation and no
   experiment-phase downstream implementors, so it is sealed from day one.
   Raw backend access remains the documented compiling escape surface.
3. **Epoch discipline.** One sweep-owned snapshot per sweep, exposed only through `&S`; every transmitted stripe is fully reconstructed from that logically immutable state. Ordinary ownership enforces the owned/shared-reference boundary, but unconstrained `S` can contain interior mutability or handles to shared external state: keeping those stable for the epoch is a documented caller obligation and compiling escape surface. On the cooperative driven path, callers deliver every recovered `StripeSettlement` to its owning `Sweep::settle`; a matching failed or cancelled settlement poisons that sweep, so it can mint no further target or finish and only abort remains. Rust ownership makes settlements unforgeable, move-only, and non-relabelable, but cannot force delivery: dropping a settlement or misapplying it to another sweep consumes the witness and leaves its owner outstanding. Recovery from either escape is to drop the old sweep and any remaining target/flight, call `abandon_active` (which retains demand and forces a full repaint), and call idle `invalidate` before replacement when stale physical work or external invalidation may overlap; its sticky latch makes the next epoch non-clearing so another full repaint remains due. Ordinary flight drop is the related non-returning escape: the reviewed adapter MUST synchronously cancel/disarm on `Drop`; no settlement witness or resources return. Sweep completion itself is still decided **only** by consuming matching, in-order written settlements over a fixed, validated full-panel plan — never by caller assertion — but settlement delivery is a cooperative contract, not a linear-type guarantee (finding 9; exit-review round-3 findings 2–3; exit-review rounds 4–5 finding 3).
4. **Honest touch semantics.** Latest-state-with-coalescing: every surfaced
   report is complete and untorn; intermediate transitions may coalesce; an
   atomic `produced_generation`/`serviced_generation` state machine with a
   bounded number of snapshot services per activation and re-latch on
   generation change, asserted INT, or failure (findings 11, 12). K2R-0A
   selected the host generation/admission shape; a concrete FT3168 ISR producer
   and its silicon delivery are K2R-1 work, not assumed.
5. **Milestone honesty.** `StripeWritten` and `SweepWritten` only (finding 17).
6. **Board anchor facts** of RESEARCH section 2, revision-keyed.
7. **Optional draw integration.** With the default-off `embedded-graphics`
   feature, one target borrows one exact RGB565 stripe byte buffer and is
   admitted only for the owning sweep's outstanding `StripeTarget`. Its
   drawing bounds remain the full panel in global coordinates while writes
   are clipped and translated into that stripe. Constructor validation,
   ordinary borrowing, and deterministic host oracles enforce this boundary;
   physical panel color/order fidelity remains a board-HIL gate.
8. **One admitted blocking-region path.** The blocking operation is
   `StripeTarget::write_region`; it consumes the outstanding target, a mutable
   pixel slice, and a sealed, profile-owned writer. Every ordinary return
   carries the writer and exact slice back with one written or unwritten
   settlement for the owning sweep. The reviewed private SH8601 engine and
   target-only concrete adapter are the admission layer; no public raw-wire
   implementation seam can report a fabricated success. This operation is
   synchronous and serialized: it has no spare buffer, future, cancellation,
   reactor source, timeout, or preemption claim.
9. **One reviewed concrete async region path.** While the generic async traits
   remain deliberately open, revision 11 selects a profile-owned adapter whose
   safe constructor brands the exact Waveshare V1 SPI2/GDMA/pin set and whose
   shared private engine emits the supplied region's CASET/PASET before one
   owning RAMWR payload. This is implementation-specific evidence, not generic
   sealing. The 16,380-byte cap, resource-carrying start error, completion slot,
   cancel/drop cleanup, and target reactor link are named below; multichunk
   async RAMWRC remains deferred.

## 6. Normative K2R-0 surface (amended through the async-region selection)

Revision 8 retains the mechanism selected by the K2R-0A experiment (C
completion in the A′ carrier, `K2R0A-LOG.md`) and exit-review round 1
restructuring, with round-3 batch-6, round-4 batch-7, and round-5 batch-8
repairs to the target/start/settlement/sweep lifecycle and revision 7's
optional draw-target integration. It additionally records the scoped Xtensa
compile/link result and the reviewed waker/critical-section boundary. This
section is **normative** for the K2R-0 host slice, superseding revision 2's
provisional candidates. Revision 9 specifies the kernel-admitted completion
source below. The section-8 real-reactor and portable-link oracles now close
K2R-0A item 3 with host + portable-link scope; revision 12 records the complete
feasibility experiment closed with its combined host/portable/exact-link scope.
Target execution and silicon behavior remain outside that closure. Revision 10 adds the independent blocking
region operation. Revision 11 selects one additive concrete async integration;
it does not freeze or seal the experiment-open generic traits.

### 6.1 Geometry and identity

`Region` — global panel coordinates, never stripe-local. `FrameEpoch` —
scene-snapshot identity, monotonic within one demand machine's exact
2^64-minted-epoch operating horizon, minted only by `FrameDemand`; no
public constructor. Epochs 0 through `u64::MAX` are each minted once. A
further `begin_sweep` attempt panics with one build-profile-independent
exhaustion message before mutating demand state.

### 6.2 Transfer boundary

`OwnedTransfer` (sealed at the K2R-0 freeze; open during the experiment so probes
and models implement it): `poll_done(&mut self, cx) -> Poll<()>` —
**register-then-recheck mandatory** (the check-then-register order has a
lost-wake race; the suite carries a deliberately broken negative control);
reports settlement only. `cancel(&mut self)` — idempotent; classifies and
stores the settlement at its completion-observation **linearization point**
(a racing physical completion after that point is conservatively
`Cancelled`) and MUST wake a registered waker. `recover(self) ->
Recovered<T, B>` — the **sole outcome authority**.

A reviewed interrupt-slot implementation MUST clone the candidate task waker
before entering its global critical section. It MAY compare registrations and
move `Waker` values while excluded, but every replaced, unused, or completed
registration MUST leave the critical section before its `RawWaker` clone,
drop, or wake behavior can run. This prevents executor-lock/global-critical-
section inversion on a multicore target and keeps ISR exclusion bounded to
slot and hardware-state operations.

`StartPermit<'a>` — a crate-issued, non-`Clone` dispatch authority with a
private constructor, lifetime-bound to one `StripeTarget::start_flight` call.
The type is public only so experiment-phase integrations can name the
`FlightStarter` signature; safe external code cannot construct one, and its
lifetime prevents a starter from returning the received permit inside its
non-lifetime-parameterized `Error` for later direct use.

`FlightStarter` (sealed at the K2R-0 freeze on exactly the same schedule as
`OwnedTransfer`; open during the experiment): an operation-bound capability
whose consuming `start(self, Region, StartPermit<'_>) -> Result<Transfer,
Error>` is invoked by the crate, never by caller-supplied callback code. The
permit makes safe direct invocation unavailable even while the trait remains
open; it does not prove that an implementation uses the region honestly.
`Transfer` implements
`OwnedTransfer`. A reviewed implementation MUST start exactly the supplied
region on `Ok`, and `Err` MUST be acceptance-atomic: no transfer was accepted
and no later physical write can result; the error returns every captured
transport/sent-buffer resource. While the trait remains open, safe dishonest
implementations can still ignore the region, return an independently
prestarted transfer, or start and then return `Err`. That experiment-phase
integration-honesty obligation is the documented escape, in the same class as
`TouchReader`'s untorn-snapshot obligation; it becomes structural only under
sealed, reviewed integrations.

`StripeTarget` — non-`Clone`, private-field identity (demand, epoch, region),
minted only by `Sweep::next_target`. Its consuming `start_flight(spare,
starter)` is the **only public construction path** for `InFlight` and invokes
`FlightStarter::start` with exactly this target's region and a fresh
crate-issued `StartPermit`.
On `Ok`, the returned already-started transfer, spare, and same target move
into flight in one operation. On `Err`, no flight exists and a move-only
start error returns `E`, the spare, and the unchanged target; `Err` therefore
means, by the `FlightStarter` integration contract, that no transfer was
accepted and no later physical write can result from the attempt; a start that
may still complete MUST return an `OwnedTransfer` in `Ok`. `E` returns any
transport/sent-buffer resources captured by the starter. The caller may retry
that same target. Because it remains outstanding, terminating instead requires
an eventually accepted flight to drain and settle, or dropping the target and
sweep followed by `FrameDemand::abandon_active`. Pairing is structural under
sealed integrations; integration honesty is an explicit obligation during the
experiment (exit-review round-3 finding 1; exit-review round-4 finding 1).

`InFlight<X, S>` — `&mut`-polled and **conditionally** `Unpin`: exactly
`X: OwnedTransfer + Unpin` and `S: Unpin`; `X::Transport` and `X::Buffer`
need no `Unpin` bound because they are not stored separately in flight. It
owns the transfer, spare, and `StripeTarget`; `begin_drain`/`poll_complete`
is the only resource-returning path; ordinary drop is the documented
non-returning boundary (integrations MUST synchronously cancel the physical
operation and disarm their completion slot on drop — no stale write or
registration survives the adapter's `Drop` return). “Spare is independently writable” describes
the owned Rust value only: unconstrained sent-buffer and spare types may share
safe interior-mutable backing storage, so disjoint physical storage remains a
reviewed-integration/caller obligation and a published compiling escape.

For kernel composition, `InFlight<X, S>` MUST implement
`Future<Output = Settled<X::Transport, X::Buffer, S>>` exactly when
`X: OwnedTransfer + Unpin` and `S: Unpin`; `poll` delegates to the existing
`poll_complete` operation and introduces no second settlement path. A reactor
owner stores that future in
`kittens::source::OptionalInlineOneShot<InFlight<X, S>>`, the sole no-alloc
inline one-shot admitted by root section 37.6.1. The source starts dormant,
is armed only with the `Ok(InFlight)` returned by
`StripeTarget::start_flight`; a `StartFlightError` recovers resources without
arming it. The source yields the real `Settled`, becomes dormant before its
handler runs, and is rearmed only after the handler has recovered resources
and delivered the settlement to its owning `Sweep`. Graceful shutdown uses
`if let Some(flight) = source.future_mut() { flight.begin_drain(); }`; `None`
means there is no flight to drain. Arbitration continues until an armed flight
settles.

Consumer-side composition preserves this profile crate's empty feature-off
normal dependency graph.

Enforcement layers: the kernel's sealed carrier and ordinary ownership retain
the same inline future across selection loss and return one owned output;
`InFlight`'s private state plus deterministic register-then-recheck tests
establish level-visible completion and wake behavior; the owning-sweep
delivery remains the existing cooperative documentation boundary. The generic
carrier does not certify an arbitrary future's producer semantics, and direct
`.await`/manual `poll_complete` remain compiling raw bypasses. The mutable
future borrow used for `begin_drain` also permits ordinary
`mem::replace`; raw handler-side replacement can discard the cooperative
resource-return path and is a published compiling escape, not something the
carrier claims to prevent. Dropping an armed source drops the flight, invoking
the reviewed synchronous cancel/disarm contract but returning no resources.
Arming is local-only and schedules no wake; rearm occurs inside a handler/phase
whose continuation starts the next arbitration. The section-8 controls
independently pin an inert inner future and raw mutable replacement that still
compile, plus a readiness-declaration mismatch that does not.

`Settled<T, B, S>` — private resources, outcome, and target; safe external
code cannot construct one or rewrite its proof-bearing state.
`Settled::into_parts(self)` is the only resource extraction path and returns
the transport, sent buffer, spare, and exactly one move-only
`StripeSettlement`. That settlement is either `Written(StripeWritten)` for a
real `Completed` recovery or `Unwritten(StripeUnwritten)` carrying the real
`Cancelled`/`Failed` outcome. Both inner witnesses have private
demand/epoch/region fields and are non-`Clone`; neither can be forged,
rewritten, duplicated, or minted twice. Delivery to the owning `Sweep::settle`
is nevertheless cooperative: safe code may drop the settlement or consume it
in a rejected call on another sweep. Either escape leaves the owner
outstanding and requires drop-plus-`abandon_active` full-repaint recovery, plus
idle `invalidate` before replacement when stale physical work or external
invalidation may overlap. A never-started transfer cannot produce `Settled` at
all (exit-review round-3 finding 2; exit-review round-5 finding 3).

### 6.3 Sweep

`PanelGeometry` — admitted full-panel geometry; the anchor board is
`WAVESHARE_18_V1`. `custom_unvalidated_panel` is the deliberately loud,
compiling escape for hosts and unadmitted hardware.

`SweepPlan` — validated full-panel stripe plan (empty/zero/overflow
rejected) over a `PanelGeometry`, fixed at `FrameDemand` construction;
sweeps cannot substitute another. `Sweep<S>` — crate-owned, minted only by
`begin_sweep`; owns the snapshot (shared-reference access only), the plan,
the repaint-obligation state, the provenance-branded epoch, and one private
position state (`Ready`/`Outstanding`/`Poisoned`). Ordinary borrowing does
not freeze interior mutable or externally shared state inside `S`; callers
MUST keep that state logically immutable for the epoch.

`next_target(&mut self)` mints at most one target for the current plan
position. While that target is outstanding it returns `None`; only accepting
its matching `StripeSettlement` clears the outstanding state. `settle`
rejects a foreign demand, epoch, region, non-outstanding target, or any
settlement after poison without changing observable state. A matching
`Written` settlement clears outstanding and advances coverage exactly once;
a matching `Unwritten` settlement clears outstanding and irreversibly
poisons the sweep. A poisoned sweep returns no target and `finish` always
returns it unchanged. `finish(self)` yields `(SweepWritten, S)` only at full,
healthy, fully-settled coverage. `abort(self)` yields `(AbortedSweep, S)` only
from `Ready` or `Poisoned`; while a target is `Outstanding` it returns
`Err(self)` unchanged.

Settlement-gated abort adds no liveness cost to an accepted flight:
`begin_drain` requests cancellation, `poll_complete` settles by the
`OwnedTransfer` contract, `Settled::into_parts` returns its exactly-one
settlement, and cooperative delivery to `Sweep::settle` either advances or
poisons before `abort` succeeds. It closes the old path that authorized a
replacement while a live target/flight remained independently usable, but it
does not supply linear ownership of the settlement's destination. A
never-accepted outstanding target, a dropped/misapplied settlement, or an
ordinary dropped flight must instead use the explicit drop-plus-
`abandon_active` recovery boundary: drop all old values, abandon to retain
demand and force a full repaint, and use idle `invalidate` before replacement
when stale physical work or external invalidation may overlap. Under sealed
integrations **and cooperative owning-sweep delivery**, coverage is a
construction, never a caller claim; the documented integration-honesty,
settlement-delivery, and drop/abandon escapes remain. Every K2R-0 plan covers
the full panel; `full_repaint == false` means no outstanding forced-repaint
obligation, not a partial sweep (exit-review round-3 finding 3; exit-review
rounds 4–5 finding 3).

### 6.4 Demand

`FrameDemand` — owns the plan and accepts caller-supplied `Tick` values from
a trusted monotonic platform time source. Regressing written-settlement
times are clamped, but arbitrary forward values are outside this type's
validation. Eligibility uses checked addition: when `last_written +
min_interval` exceeds `Tick::MAX`, `begin_sweep` panics before mutating state
rather than silently shortening the interval. Callers replace the demand
machine only after settling/abandoning its active epoch when their platform
time base reaches that documented horizon. Demand provenance uses a
thumbv7em-compatible `AtomicU32` with checked exhaustion, widened to `u64` in
witnesses; epoch uniqueness lasts exactly 2^64 successful sweeps as specified
in section 6.1.
`request` (the only kittens-tui-shared vocabulary), `begin_sweep(now,
snapshot)` (sole eligibility acknowledgment; one active demand epoch),
`eligible_at`, `finish_written(SweepWritten, now) ->
Result<WrittenDisposition, ForeignSweep>`, `finish_failed(AbortedSweep,
now)`, `abandon_active` (dropped/outstanding-sweep recovery, including lost or
misapplied settlement), `invalidate` (a private
clear/pending/active latch). A mid-sweep invalidation makes that sweep's settlement
`DiscardedByInvalidation`: obligations retained, throttle unchanged. An idle
invalidation sticks until the next successful `begin_sweep`, which transfers
it into that minted epoch's discard state; throttled/rejected/panicking begin
attempts cannot clear it. Thus an invalidate between abort/abandon and
replacement cannot be lost, and the suspect replacement cannot clear the
full-repaint obligation.
`abandon_active` is witness-terminal only: before beginning the replacement
sweep, callers MUST drop the old sweep, every unstarted old target, and every
started flight. It retains demand and forces a full repaint. When stale
physical work or external invalidation may overlap, callers MUST then invoke
idle `invalidate` before minting the replacement; the replacement becomes
non-clearing and leaves another full repaint due.
The reviewed adapter synchronously cancels/disarms a dropped flight, bounding
that explicit escape; a retained old `Sweep` can still mint another old target
and safe Rust cannot force a dishonest caller to drop rather than drive it.
Foreign or stale
witnesses are rejected **without mutation, in release builds**.
Milestone vocabulary is written-only: `finish_written`, `last_written`,
`SweepWritten` — nothing claims physical presentation.

### 6.5 Touch

Per findings 10–13 (implementation authored by the reviewing engineer):
the ISR-side producer bumps the generation **then** wakes on a separate
pending-latch `swap` — wake dedup without the idle-check TOCTOU; the
service side clears with clear-then-recheck/re-latch; a persistent retry
latch stays authoritative across INT-only activations, read failure,
budget exhaustion, and counter wrap; per-activation budget is
`NonZeroU8`; semantics are latest-state-with-coalescing over complete
untorn snapshots, with edges reconstructed between surfaced reports and
**no edge for an unchanged contact**.

### 6.6 Optional embedded-graphics stripe target

The crate's default feature set remains empty. Enabling the
`embedded-graphics` feature adds `Rgb565StripeDrawTarget<'a>` and the
`embedded-graphics` dependency; disabling it leaves the core dependency-free,
`no_std`, and no-alloc.

`Rgb565StripeDrawTarget::new(&Sweep<S>, &StripeTarget, &'a mut [u8])` is the
single construction path. It accepts only the supplied sweep's currently
outstanding target, deriving both the stripe region and the full panel from
private sweep state; a foreign, stale, or non-outstanding target is rejected.
The byte slice length MUST equal `stripe.width * stripe.height * 2`, checked
without overflow. The target writes row-major RGB565 using the anchor driver's
host format (raw RGB565 high byte, then low byte), translates global panel
coordinates into stripe-local byte offsets, and clips every pixel outside the
one stripe. Drawing is infallible after construction and allocates nothing.
The target retains no spatial history and does not clear or reconstruct stale
scratch storage automatically: for every stripe, callers MUST paint the
background and complete ordered scene from that sweep's snapshot.

Its `Dimensions::bounding_box` MUST be the owning sweep's complete panel region
in global coordinates, including a nonzero custom-panel origin. It MUST NOT
report stripe-local bounds: scene code that centers or lays out from target
dimensions must produce the same geometry on every stripe as it does against a
full-frame target.

Enforcement layers: private sweep/target provenance plus constructor admission
for pairing and exact length; ordinary Rust borrowing for exclusive buffer
access; deterministic draw-target tests for packing, translation, clipping,
and global bounds; and the section-8 real-witness-chain pixel oracles for epoch
reconstruction. This host integration does **not** prove the target adapter's
physical byte/channel interpretation or delivery, panel `MADCTL` color order,
physical color fidelity, TE behavior, or scene-replay cost. Those remain exact-
adapter, board-HIL, and measurement gates.

### 6.7 Blocking SH8601 region transport

The sole canonical blocking operation is
`StripeTarget::write_region(pixels, writer)`. It consumes the one outstanding
target, one exact `&mut [u8]`, and a `BlockingRegionWrite` implementation. The
trait is sealed from its first release and its dispatch method requires a
private-constructor, non-`Clone`, lifetime-bound `BlockingWritePermit`, so safe
external code can neither implement the capability nor invoke its raw dispatch
without a target. The wire trait and SH8601 transaction engine are private.
The only admitted production implementation in this revision is
`Esp32s3Sh8601BlockingTransport<'d>`, available only under the explicit
default-off `esp32s3-sh8601-blocking` feature on `target_arch = "xtensa"`.
This supersedes RESEARCH section 3's provisional open
`BlockingRegionWrite<B>` spelling. Structural sealing and the dispatch permit
serve different checks: the seal rejects unreviewed success reporters, while
the permit prevents even an admitted writer from being dispatched outside the
consuming target operation. The async traits remain open by the explicit non-
freezing experiment contract; test models and the dishonest-starter negative
control deliberately exercise that openness until a separately authorized
breaking freeze seals the reviewed integration set.

The complete public shape (under `kittens_render::blocking`, except for the
target module named below) is:

```rust
pub struct BlockingWritePermit<'a> {
    _key: &'a mut (),
}

pub trait BlockingRegionWrite: private::Sealed + Sized {
    type Error;

    fn write_region_admitted(
        self,
        region: Region,
        pixels: &[u8],
        permit: BlockingWritePermit<'_>,
    ) -> (Self, Result<(), Self::Error>);
}

#[must_use = "recover the writer, pixels, result, and owning-sweep settlement"]
pub struct BlockingSettled<T, P, E> {
    writer: T,
    pixels: P,
    result: Result<(), E>,
    target: StripeTarget,
}

impl StripeTarget {
    pub fn write_region<'pixels, W>(
        self,
        pixels: &'pixels mut [u8],
        writer: W,
    ) -> BlockingSettled<W, &'pixels mut [u8], W::Error>
    where
        W: BlockingRegionWrite;
}

impl<T, P, E> BlockingSettled<T, P, E> {
    pub fn region(&self) -> Region;
    pub fn outcome(&self) -> TransferOutcome;
    pub fn into_parts(self) -> (T, P, Result<(), E>, StripeSettlement);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sh8601Axis {
    X,
    Y,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sh8601PixelCommand {
    RamWriteStart,
    RamWriteContinue,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sh8601WriteStage {
    ColumnAddress,
    PageAddress,
    Pixel {
        command: Sh8601PixelCommand,
        chunk: usize,
        offset: usize,
        len: usize,
    },
}

#[derive(Debug)]
pub enum Sh8601RegionWriteError<E> {
    EmptyWidth,
    EmptyHeight,
    CoordinateOverflow { axis: Sh8601Axis },
    OutOfBounds { region: Region },
    WrongByteLength { expected: u32, actual: usize },
    Io { stage: Sh8601WriteStage, source: E },
}
```

The constant `SH8601_DMA_CHUNK_BYTES: usize = 16_380` is exported from this
module for target scratch sizing. Under the target feature,
`kittens_render::esp32s3_sh8601` exports:

```rust
pub struct Esp32s3Sh8601BlockingTransport<'d> {
    bus: SpiDmaBus<'d, Blocking>,
}

impl<'d> Esp32s3Sh8601BlockingTransport<'d> {
    pub fn try_new(
        spi: SpiDma<'d, Blocking>,
        rx: DmaRxBuf,
        tx: DmaTxBuf,
    ) -> Result<Self, (SpiDma<'d, Blocking>, DmaRxBuf, DmaTxBuf)>;

    pub fn into_parts(self) -> (SpiDma<'d, Blocking>, DmaRxBuf, DmaTxBuf);
}

impl<'d> BlockingRegionWrite for Esp32s3Sh8601BlockingTransport<'d> {
    type Error = Sh8601RegionWriteError<esp_hal::spi::Error>;
    // write_region_admitted body implements the private engine; omitted here.
}
```

No other root re-export is added: these two module paths are canonical.

`write_region_admitted` is the visibly exceptional implementation hook, not a
second consumer spelling; the unconstructible permit prevents safe direct
dispatch. `BlockingSettled` is `must_use`, privately retains the entire
consumed `StripeTarget` as the sole source of demand/epoch/region provenance,
and exposes only the target `region` and `TransferOutcome` classification by
shared reference. The blocking path can produce only `Completed` or `Failed`;
`Cancelled` is unreachable because this call has no cancellation transition.

The target operation always returns a private-field
`BlockingSettled<T, P, E>`. Its consuming `into_parts` returns the same writer,
the exact mutable pixel slice, the operation result (`Ok(())` or the concrete
error), and exactly one `StripeSettlement`. A complete adapter return yields
`Written`; every reported error, including preflight rejection, yields
`Unwritten(Failed)` and never `Cancelled`. The caller delivers that settlement
to the owning sweep;
a failure therefore conservatively poisons the sweep and requires abort/full
repaint rather than introducing a second retry protocol. The result has no
constructor or conversion into any other coverage witness. Panic, abort, and
nontermination are non-returning escapes and recover neither resources nor a
settlement.

Safe code may also drop `BlockingSettled` without extracting its settlement.
That drops the writer, releases the mutable slice borrow without returning the
slice value, and leaves the owning sweep outstanding. This is a documented
compiling escape, not prevented by `must_use`: recover by dropping the old
target-owning sweep, calling `FrameDemand::abandon_active`, and retaining the
underlying pixel storage for a full repaint; use idle `invalidate` before the
replacement when stale physical state may overlap. A compile-pass control that
accepts and explicitly drops an otherwise opaque `BlockingSettled<T, P, E>`
pins this boundary without adding a construction loophole.

The concrete transport consumes `SpiDma<Blocking>`, `DmaRxBuf`, and
`DmaTxBuf` through
`Esp32s3Sh8601BlockingTransport::try_new(spi, rx, tx)`. Rejection returns the
exact `(SpiDma, DmaRxBuf, DmaTxBuf)` tuple and occurs unless both DMA scratch
buffers have at least 16,380 bytes. Requiring the full RX reserve is a profile
admission/memory-budget policy copied from the audited upstream board adapter,
not a claim that TX-only HAL calls consume RX payload storage; target evidence
must pass before a later revision may lower it. After those rejection checks,
admission resets the TX descriptor length to exactly 16,380 before
`with_buffers`: the pinned HAL checks the backing capacity but does not relink
a caller-shortened descriptor chain inside `half_duplex_write`. Rejection
therefore returns untouched parts, while every accepted capacity-valid TX
scratch has descriptors for the maximum operation payload. The target fixture
enters admission with logical TX length one and checks for 16,380 after split.
`into_parts` waits for the
blocking bus to become idle through `SpiDmaBus::split` and returns the same
tuple. The blocking call copies each command or pixel chunk into the HAL-owned
TX scratch, waits at the HAL boundary, and allocates nothing. It is not a
zero-copy operation. No caller-defined interface, callback, or generic backend
can sit beneath the admitted type.

Before the first bus call, the shared private engine validates in this exact
precedence: `EmptyWidth`; `EmptyHeight`; checked `u16` X-exclusive-end
overflow; checked `u16` Y-exclusive-end overflow; `OutOfBounds` for any start
outside the fixed half-open `0..368 × 0..448` anchor panel or any exclusive end
with `x + width > 368` or `y + height > 448` (equality is the valid right/bottom
boundary); then `WrongByteLength`. Once bounds pass, the RGB565 byte count is at most
329,728 and is computed exactly in `u32`; comparison promotes both that value
and the supplied slice length to `u64`, so no host- or target-width overflow
variant is needed. `WrongByteLength` reports the exact `u32` expectation and
`usize` actual length. Every validation error precedes all bus I/O. The engine
then emits exactly:

1. `CASET` (`0x2A`): opcode `0x02`, 24-bit address `0x002A00`, single-line
   command/address/data, zero dummy cycles, and inclusive big-endian X
   endpoints.
2. `PASET` (`0x2B`): the same envelope at address `0x002B00`, with inclusive
   big-endian Y endpoints.
3. The first nonempty pixel chunk: opcode `0x32`, address `0x002C00`
   (`RAMWR`), single-line command/address, quad data, zero dummy cycles.
4. Every remaining chunk: the same pixel envelope at address `0x003C00`
   (`RAMWRC`). Chunks are at most 16,380 bytes; every non-final chunk is
   exactly that size. This compatibility constant is copied from the audited
   upstream board adapter and equals four times the hardware's 4,095-byte
   maximum TX-descriptor payload. The pinned HAL's 4,092-byte default chunking
   uses five descriptors for this reserve. It remains below that HAL's
   32,736-byte SPI DMA transfer ceiling, and its even value never splits one
   RGB565 pixel.

`Sh8601RegionWriteError<E>` distinguishes the exact preflight cases above and
an I/O error whose `Sh8601WriteStage` identifies `ColumnAddress`,
`PageAddress`, or, for a pixel stage, the command kind, zero-based chunk index,
byte offset, and byte length.
Validation errors perform zero I/O. An I/O error stops before every later
command but is not acceptance-atomic: earlier commands/chunks may have changed
the panel, and its GRAM cursor and region contents are conservatively
undefined until a full repaint. `Ok` means only that every blocking HAL call in
the reviewed sequence returned success. It does not mean `BusIdle` beyond the
HAL method's own fence, `FramePresented`, panel command acceptance, correct
physical placement or color, or TE-safe presentation.

The implementation is derived from and reviewed against `sh8601-rs` 0.1.8
commit `4bcddfd529017135f19a5a9a6e79dd6b8ef1b460`; the stock driver is not a
compiled dependency and its allocating `partial_flush` is not used. Repository
and fixture builds compile the concrete adapter against `esp-hal` git revision
`d48f747ba28accdc51779ba193eba923138e0382`; the publishable manifest also
names the matching `=1.1.0` registry version in the same dependency. Cargo's
[documented multiple-locations rule](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations)
uses the git source locally, checks its package version against that
requirement, and retains the registry version in normalized package output. The
exact git source and linked ELF remain the repository-development gate. The
fallback declaration is exercised separately by revision 12's clean packaged-
source consumer gate; the declaration alone is not evidence. The optional target feature
requires the Espressif Rust toolchain (the pinned HAL declares Rust 1.88 or
newer); the feature-off portable crate retains the workspace Rust 1.85 floor
and empty normal dependency graph.

The manifest spelling is exact:

```toml
[features]
esp32s3-sh8601-blocking = ["dep:esp-hal"]

[target.'cfg(target_arch = "xtensa")'.dependencies.esp-hal]
version = "=1.1.0"
git = "https://github.com/esp-rs/esp-hal"
rev = "d48f747ba28accdc51779ba193eba923138e0382"
optional = true
default-features = false
features = ["esp32s3", "unstable"]
```

The standalone repository fixture enables this profile feature and depends on
the identical git URL/revision (with `rt` additionally enabled for firmware),
so its `SpiDma` resource has the same Cargo source identity as the public
constructor expects. Cargo's normalized package manifest instead exposes
registry `esp-hal =1.1.0` types, and a consumer of that package must use the
same registry identity. Revision 12 separates two checks that were previously
conflated: a locally executable clean packaged-source + registry-HAL Xtensa
consumer gate, and an exact-version crates.io download smoke after a future
correctly versioned, human-authorized release. The first requires no upload or
version change and is specified in section 9.1. The second remains human-
ordered under repository publication policy. The already published 0.1.1
artifact is older immutable source and is not evidence for the current tree.
No publication is part of this slice.

Enforcement layers: sealed trait admission plus the profile-owned concrete
adapter exclude external success reporters; private target/result/witness
state and ordinary ownership bind success or failure to the consumed target
and returned resources; the single private engine is shared by the host wire
recorder and real HAL adapter; deterministic traces establish validation,
encoding, chunking, stop-on-error, and resource identity; and a no-allocator
Xtensa link plus symbol inspection establish the target allocation boundary.
The crate-wide `forbid(unsafe_code)` remains in force: the production adapter
uses only esp-hal's safe `SpiDmaBus` construction, blocking-write, and split
surface and introduces no unsafe block in `kittens-render` or the fixture.
Raw direct HAL access still compiles and remains outside the capability.
Calling this synchronous operation inside a reactor handler can block every
other arm. Panel initialization and serialization with reset, sleep/AOD,
brightness, and other commands remain a board-coordinator obligation.

`try_new` accepts esp-hal's unbranded `SpiDma<Blocking>`; sealing does not prove
which SPI peripheral, GDMA channel, QSPI pins, mode, or frequency the caller
configured, nor that the panel is initialized or command access serialized.
`Ok` is therefore scoped to the reviewed transaction over the supplied bus.
The exact fixture binds SPI2/GDMA_CH0/GPIO4–7/11/12 at 40 MHz, while a target
compile-pass function accepting arbitrary same-source `SpiDma` and scratch
buffers pins the configuration-honesty escape. Physical-board truth remains
HIL; no board-construction token is claimed in this slice.

The revision-9 Xtensa feasibility artifact exercised only the distinct owning
`SpiDma::half_duplex_write` surface with small TX-only buffers and provided no
partial evidence for this blocking path. Revision 10 adds a separate retained,
unexecuted entry path through `SpiDmaBus`, the symmetric fixed scratch policy,
and `split`; its host + exact-Xtensa-link evidence closes only the scoped
blocking-region row recorded in sections 9 and 11.

### 6.8 Profile-owned single-payload async region adapter

Revision 11 adds one reviewed concrete implementation beneath the existing
experiment-open `FlightStarter` / `OwnedTransfer` surface. It does **not** add a
parallel capability trait, seal either existing trait, or claim that arbitrary
external implementations are honest. The later 0.2.0 freeze remains the sole
gate that may make generic pairing structural at a breaking-version boundary.

The default-off, Xtensa-only `esp32s3-sh8601-async` feature depends on
`esp32s3-sh8601-blocking`, activates the pinned HAL's `rt` support required by
the concrete interrupt handler, adds a direct optional `critical-section =
"1"` dependency for the reviewed slot, and exports, from
`kittens_render::esp32s3_sh8601_async`, exactly these profile-owned roles:

```toml
[features]
esp32s3-sh8601-async = [
    "esp32s3-sh8601-blocking",
    "esp-hal/rt",
    "dep:critical-section",
]

[target.'cfg(target_arch = "xtensa")'.dependencies.critical-section]
version = "1"
optional = true
```

The standalone target fixture removes its former direct `critical-section`
dependency, enables `esp32s3-sh8601-async` on `kittens-render`, and adds the
kernel only as the no-default-feature macro consumer:

```toml
kittens = { path = "../../crates/kittens", default-features = false, features = ["macros"] }
kittens-render = { path = "../../crates/kittens-render", default-features = false, features = ["esp32s3-sh8601-async"] }
```

The public construction and recovery spellings are exact (the implementation
may carry the narrow `clippy::too_many_arguments` and
`clippy::type_complexity` allowances earned by the one exact board-resource
constructor and its inverse tuple):

```rust,ignore
use esp_hal::{
    Blocking,
    dma::{DmaRxBuf, DmaTxBuf},
    peripherals::{DMA_CH0, GPIO4, GPIO5, GPIO6, GPIO7, GPIO11, GPIO12, SPI2},
    spi::{
        Error,
        master::{Address, Command, DataMode},
    },
};

impl<'d> Waveshare18V1Sh8601Parts<'d> {
    pub fn new(
        spi2: SPI2<'d>,
        dma_ch0: DMA_CH0<'d>,
        sio0_gpio4: GPIO4<'d>,
        sio1_gpio5: GPIO5<'d>,
        sio2_gpio6: GPIO6<'d>,
        sio3_gpio7: GPIO7<'d>,
        sck_gpio11: GPIO11<'d>,
        cs_gpio12: GPIO12<'d>,
        rx_scratch: DmaRxBuf,
        tx_scratch: DmaTxBuf,
    ) -> Self;

    pub fn into_parts(self) -> (
        SPI2<'d>, DMA_CH0<'d>,
        GPIO4<'d>, GPIO5<'d>, GPIO6<'d>, GPIO7<'d>,
        GPIO11<'d>, GPIO12<'d>,
        DmaRxBuf, DmaTxBuf,
    );
}

impl<'d> Waveshare18V1Sh8601Transport<'d> {
    pub fn try_new(
        parts: Waveshare18V1Sh8601Parts<'d>,
    ) -> Result<Self, Waveshare18V1Sh8601Parts<'d>>;

    pub fn with_idle_commands<R>(
        &mut self,
        f: impl FnOnce(&mut Waveshare18V1Sh8601IdleCommands<'_, 'd>) -> R,
    ) -> R;

    pub fn into_start(
        self,
        pixels: DmaTxBuf,
    ) -> Waveshare18V1Sh8601Start<'d>;
}

impl<'bus, 'd> Waveshare18V1Sh8601IdleCommands<'bus, 'd> {
    pub fn half_duplex_write(
        &mut self,
        data_mode: DataMode,
        command: Command,
        address: Address,
        dummy_cycles: u8,
        data: &[u8],
    ) -> Result<(), Error>;
}

impl<'d> Waveshare18V1Sh8601StartError<'d> {
    pub fn failure(&self) -> &Sh8601AsyncStartFailure<Error>;
    pub fn into_parts(self) -> (
        Sh8601AsyncStartFailure<Error>,
        Waveshare18V1Sh8601Transport<'d>,
        DmaTxBuf,
    );
}

impl<'d> FlightStarter for Waveshare18V1Sh8601Start<'d> {
    type Transfer = Waveshare18V1Sh8601Transfer<'d>;
    type Error = Waveshare18V1Sh8601StartError<'d>;
    // `start` has the one canonical trait spelling from section 6.2.
}

impl<'d> OwnedTransfer for Waveshare18V1Sh8601Transfer<'d> {
    type Transport = Waveshare18V1Sh8601Transport<'d>;
    type Buffer = DmaTxBuf;
    // `poll_done`, `cancel`, and `recover` retain section 6.2's spellings.
}
```

The concrete transfer and
`InFlight<Waveshare18V1Sh8601Transfer<'d>, DmaTxBuf>` MUST be `Unpin`.

- `Waveshare18V1Sh8601Parts<'d>` is a private-field, validation-free
  construction bundle. Its sole `new(...)` constructor consumes the exact
  esp-hal singleton types and pins them to one bus role: `SPI2`, `DMA_CH0`,
  SIO0=GPIO4, SIO1=GPIO5, SIO2=GPIO6, SIO3=GPIO7, SCK=GPIO11, and CS=GPIO12,
  followed by one `DmaRxBuf` and one `DmaTxBuf` command-scratch reserve.
  `into_parts` returns that exact tuple in constructor order, so a rejected
  caller can replace either scratch buffer or repurpose every singleton rather
  than retrying a guaranteed rejection or dropping resources;
- `Waveshare18V1Sh8601Transport<'d>::try_new(parts) -> Result<Self,
  Waveshare18V1Sh8601Parts<'d>>` rejects unless both
  scratch capacities are at least 16,380 bytes. After both rejection checks it
  normalizes TX descriptor length to 16,380, configures the fixed Waveshare V1
  QSPI binding internally as SPI mode 0 at 40 MHz, and owns the resulting
  `SpiDmaBus<Blocking>`. External code cannot construct this branded transport
  from an erased `SpiDma`. Scratch rejection occurs before `Spi::new`. The
  pinned constant configuration is reviewed as infallible; if the consumed
  HAL constructor nevertheless returns `ConfigError`, the adapter panics as an
  internal-invariant/non-returning boundary because esp-hal cannot return the
  consumed SPI2 singleton. That path returns no resources and is not described
  as ordinary construction rejection;
- `Waveshare18V1Sh8601IdleCommands<'bus, 'd>` is a public, private-field,
  borrowed facade with no public constructor.
  `Waveshare18V1Sh8601Transport::with_idle_commands` is the
  visibly exceptional temporary board-command escape for panel initialization and
  other commands outside proof-bearing stripe operations. The private-field
  borrowed facade exposes only `half_duplex_write(data_mode, command,
  address, dummy_cycles, data) -> Result<(), esp_hal::spi::Error>` over the
  owned bus while no transfer exists. It does not expose `&mut SpiDmaBus`, a
  configuration method, or any operation that can move or replace the bus;
  safe code therefore cannot swap an erased SPI3 bus into the branded
  transport. The closure's commands, termination, serialization, and panel-
  state truth are unchecked, and a synchronous command may block its caller;
  start reinstalls the reviewed SPI2 interrupt handler. Panic remains a non-
  returning boundary. A real board coordinator, its serialization protocol,
  and observed initialization belong to K2R-1; this closure is not that
  coordinator;
- `Waveshare18V1Sh8601Transport::into_start(pixels)` is the sole constructor
  for `Waveshare18V1Sh8601Start<'d>`. The start value owns both the branded
  transport and one caller-filled `DmaTxBuf` pixel buffer and implements the
  still-open `FlightStarter` trait;
- `Waveshare18V1Sh8601Transfer<'d>` implements `OwnedTransfer`, owns the HAL
  `SpiDmaTransfer`, both command-scratch buffers, and the interrupt-slot
  registration, and recovers the same branded transport plus the sent pixel
  buffer; and
- `Sh8601AsyncStartFailure<E>` is exactly
  `Region(Sh8601RegionWriteError<E>)`,
  `AsyncPayloadTooLarge { bytes: u32, max: usize }`, or
  `RamWriteStart { source: E }`;
- `Waveshare18V1Sh8601StartError<'d>` has private fields. `failure()` exposes
  `&Sh8601AsyncStartFailure<esp_hal::spi::Error>`; consuming `into_parts`
  returns `(failure, transport, pixels)` with the same branded transport and
  pixel buffer. `StartFlightError::into_parts`
  separately returns the outer spare and unchanged outstanding target.

An admitted idle transport intentionally has no `into_parts`: the pinned HAL
builder does not recover the consumed GPIO singleton tokens, and exposing only
an erased `SpiDma` would weaken the board brand without restoring that tuple.
The transport is therefore static board ownership. Ordinary transport drop
loses safe access to its SPI/DMA/pin/scratch resources and makes no peripheral-
reset or panel-state claim; it is an explicit non-returning teardown boundary.
Driven `OwnedTransfer::recover` is not that boundary and always rebuilds the
same branded transport. Raw HAL construction remains the separately named
escape for applications that choose unbranded ownership from the outset.

The exact peripheral singleton types and private transport constructor are the
configuration-admission layer for the profile-owned adapter: its SPI2 ISR may
therefore read and mask only SPI2 registers without accepting an erased SPI3
driver through safe public code. Raw HAL construction remains a compiling path
outside this concrete adapter. Panel initialization, reset/power state, command
serialization with other actors, and physical pin wiring remain board-owner or
HIL obligations; branded Rust singletons do not prove the board is correctly
wired or initialized.

Before any bus I/O or interrupt-slot arming, the private shared SH8601 engine
checks async start in this exact precedence:

1. `EmptyWidth`;
2. `EmptyHeight`;
3. checked `u16` X-exclusive-end overflow;
4. checked `u16` Y-exclusive-end overflow;
5. `OutOfBounds` against the fixed half-open `0..368 × 0..448` panel;
6. `AsyncPayloadTooLarge { bytes, max: 16_380 }`; then
7. `WrongByteLength { expected, actual }`, where `actual` is the pixel
   `DmaTxBuf`'s logical descriptor length, not its backing capacity.

The geometry and inclusive big-endian window encoding are the same private
functions used by revision 10; blocking and async paths MUST NOT carry separate
copies of CASET/PASET truth. A valid async start then performs exactly three
ordered boundaries:

1. blocking CASET with the revision-10 command envelope and inclusive X ends;
2. blocking PASET with the revision-10 command envelope and inclusive Y ends;
3. split the command bus to recover `SpiDma`, arm the reviewed SPI2 completion
   slot on that driver, and call the owning HAL half-duplex start once with
   opcode `0x32`, address `0x002C00`
   (RAMWR), single-line command/address, quad data, zero dummy cycles, and the
   exact logical pixel-buffer length.

The single-payload cap is deliberate. A 368×16 RGB565 stripe is 11,776 bytes
and fits; the adapter does not implement async RAMWRC, buffer offset views,
in-place compaction, or arbitrary-region DMA chaining. The pinned HAL exposes
no safe offset view that can later rejoin a `DmaTxBuf`, while this crate forbids
unsafe code. Destructive copying would add a different ownership/performance
contract. Multichunk async writes and overlap remain a later measured slice;
the blocking path remains the only reviewed RAMWRC implementation.

Every preflight failure performs zero I/O and returns all start resources. A
CASET or PASET error stops before every later boundary and returns the branded
transport and pixels. A successful earlier window command may have changed the
panel's window registers, but no RAMWR was accepted; retry is safe because
every admitted attempt reissues both complete windows before pixels. A RAMWR
start error is acceptance-atomic under the pinned HAL source: the SPI operation
has not started, the slot is synchronously disarmed, and the returned driver,
scratch, and pixels rebuild the branded transport. Any path that may still
write pixels MUST return `Ok(Waveshare18V1Sh8601Transfer)`, never an error.

On accepted start, `InFlight` owns the transfer and caller-supplied spare.
Completion uses the existing register-then-recheck SPI2 slot: candidate wakers
are cloned before global exclusion, and every unused/replaced/completed waker
leaves exclusion before drop or wake. `Completed` is chosen only after the
RAMWR transfer-done level is observed. `cancel` is idempotent, selects
`Completed` if completion was already visible and otherwise selects
`Cancelled` at the false-completion observation before synchronous HAL abort,
then wakes the registered task. `recover` waits/fences the HAL transfer,
disarms the listener and slot, rebuilds the branded transport from the exact
command scratch, and is the sole outcome authority. `Drop` performs the same
synchronous cancel/wait/disarm cleanup but deliberately returns no resources.
No stale slot or physical write may survive adapter drop.

After recovery, the caller delivers the one `Settled` witness to the owning
sweep and may reuse the returned transport and the two recovered pixel buffers
for the next stripe. The canonical target fixture defines a named
`#[inline(never)]` `linked_async_reactor_paths` function. Its generated
`reactor!` contains code paths for settling two 16-row stripes through the same
`OptionalInlineOneShot<InFlight<...>>`, rearming after each handler recovery,
then requesting drain of a third accepted flight through `future_mut`. The
third handler accepts the completion-versus-cancellation race: `Cancelled`
poisons the owning sweep, while `Completed` advances it; either settlement
clears the outstanding position and the code path then aborts the poisoned or
ready sweep. A separate named `#[inline(never)]` poll shim takes the pinned
generated future through `core::hint::black_box` and performs exactly one poll
with `Waker::noop`; there is no spin loop or fixture executor. Those handler
branches are linked code paths, not observed settlements.

A second named `#[inline(never)]` `linked_async_drop_path` function constructs
one accepted real flight, arms the real carrier, explicitly drops the complete
`Sources` owner, drops the outstanding sweep, and calls
`FrameDemand::abandon_active`. It retains the concrete transfer/carrier drop
glue and recovery spelling without claiming that the uncalled hook ran. The
firmware entry point passes both outer function pointers through
`core::hint::black_box` but never calls either. CI uses `nm -S -C` to require
both nonzero concrete symbols. This deliberately retains the generated
reactor, adapter, and drop monomorphizations in the locked ELF without
pretending that the target path executed; it is explicitly **not** a target
executor, wake-scheduling, IRQ, drop-runtime, or silicon-runtime observation.

The protocol/start decisions and the completion-slot decisions each live in a
target-neutral, crate-private core that the Xtensa shell actually calls. Host
traces invoke those same cores; a separate target-only shell maps their actions
to esp-hal and SPI2 registers. Tests that substitute modeled levels/transport
remain labeled model-boundary evidence and do not certify the HAL mapping.

Enforcement layers are: exact peripheral singleton types plus private fields
for concrete configuration admission; the shared crate-private protocol engine
plus deterministic traces for geometry/window/ordering truth; private adapter
state plus wake/cancel/drop traces for lifecycle truth; ordinary Rust ownership
for transport/scratch/sent/spare recovery; and the sealed kernel carrier for
reactor retention. Negative controls remain mandatory: an arbitrary external
`FlightStarter`/`OwnedTransfer` still compiles and can lie; direct `.await` or
manual polling compiles; raw HAL access compiles; the synchronous CASET/PASET
preamble in `FlightStarter::start` can block every reactor arm when start is
called from a handler because kernel priorities do not preempt handler
interiors; same-type sent and spare may share safe interior-mutable backing;
whole-flight/source drop returns no resources; and a host trace or linked ELF
proves no panel or silicon behavior. The allocator-symbol-free fixture uses
only its linked `Waker::noop`; arbitrary executor `RawWaker` clone/drop/wake
callbacks may allocate or perform other unchecked work, so the ELF does not
prove a universal post-init allocation bound for future executors.

## 7. K2R-0A: the feasibility experiment (normative design; closed with scope)

K2R-0A was a **non-freezing experiment** whose deliverable was a selected-and-
demonstrated shape plus an amendment to this spec, or the honest result that no
viable shape existed. Revision 12 records it **CLOSED WITH HOST + PORTABLE-LINK
+ EXACT-XTENSA-LINK SCOPE**: mechanism C in the A-prime carrier, the finite
host wake/cancel/resource traces, the real-reactor portable consumer, and the
exact target compile/link criteria all pass. It selected and demonstrated the
section-6 shapes without freezing the open capabilities. Target executor
scheduling, silicon interrupt/wake/cancel/drop behavior, and physical display
or touch truth were never pass criteria for this feasibility experiment; they
are K2R-1 runtime/HIL work.

**Candidate matrix (exhaustive per finding 4):**

| Candidate | Mechanism |
|---|---|
| A — kernel pin admission | `poll_next(self: Pin<&mut Self>, ...)` reviewed kernel path (root 37.6 reserved comparison) |
| A′ — outer-`Unpin` adapter, caller-pinned inner storage | admitted `Unpin` adapter over storage the caller has pinned (root 37.6's other arm) |
| B — named executor-task boundary | caller-owned `run` future + fixed-capacity, close-semantic channel endpoints + a sealed no-std wake-capable channel adapter (which is itself a kernel admission change — finding 5) |
| C — custom interrupt-backed transfer state | hand-built completion state driven by the transfer-done interrupt, no compiler-generated future |
| ∅ — no viable shape | recorded as a falsifier outcome; the profile redesigns rather than forces |

**Selection rule (ordered):** first candidate that passes all criteria wins; if multiple pass, prefer the one with the smallest kernel change (A′ before A before C before B); ties broken by generated-code size then by diagnostic quality, measured, not asserted.

**Pass criteria, decidable (finding 4):** against exact pinned SHAs recorded at spike start, over the named finite trace set of section 8: (1) an exact, safe, no-alloc **target compile probe** of the completion shape (finding 2); (2) completion wake reaches the reactor in both selection-loss positions (polled-then-lost and unpolled-below-winner); (3) the explicit cancel-and-drain transition returns transport, sent buffer, and spare on every trace; (4) busy-poll/self-waking completions are rejected by inspection of the wake trace (finding 2); (5) zero allocation after init; (6) no unsafe self-reference; (7) for B: task ownership (spawn/stop/join), per-display-vs-per-transfer identity, capacity, and close semantics are all specified in the artifact.

Revision 8 records criterion 1 **CLOSED WITH SCOPE** through
`fixtures/render-xtensa-probe` against the pinned `esp-hal` revision. The
linked result closes only compile/link feasibility. It is not an oracle for
the executor waker, silicon interrupt delivery or wake counts, physical
transfer behavior, the kernel-admitted `reactor!` path, or the blocking
`write_region` transaction; those remain the named runtime/integration gates.

Revision 9 selects the remaining kernel carrier shape:
`OptionalInlineOneShot<InFlight<...>>`, with the future stored inline under the
already-demonstrated outer-`Unpin` bounds. The section-8 real-`reactor!` traces
and external no-std consumer links on both required portable targets have now
run, closing item 3 with host + portable-link scope. The recorded revision-9
manual Xtensa probe and revision-11 uncalled generated-reactor hooks do not
extend that result to target-side reactor execution or silicon.

**Touch admission** was decided in the same experiment (finding 12): the ISR-
side wake-capable generation handle is a kernel-admission question of the same
kind, answered by the same matrix. A concrete FT3168 transaction and its
silicon delivery remain K2R-1 work.

## 8. K2R-0: protocol suite (host slice amended; full acceptance gated)

The K2R-0 host suite did not begin until this spec was amended with K2R-0A's
host-model-selected shapes. Revision 12 records that feasibility prerequisite
closed with scope. Full K2R-0 acceptance still requires every freeze item in
section 11. The host suite is built as a **named trace matrix** (finding 14) —
each trace enumerated, each state transition and transport boundary
independently observable — covering at minimum:

- both selection-loss positions for completion; completion before first poll; completion during waker registration;
- a real `kittens::reactor!` fixture whose source is
  `OptionalInlineOneShot<InFlight<...>>`: one trace polls completion pending
  before another source wins, one lets an earlier source win before completion
  is polled, both recover transport/sent/spare and deliver the real settlement
  to the owning sweep, and the same carrier is rearmed for the next stripe;
  shutdown additionally requests `begin_drain` through the `Some` arm of
  `future_mut` (with dormant `None` a no-op) and proves settlement before stop,
  while a separate drop trace proves synchronous disarm with intentionally
  non-returned resources; an inert inner future compiles as the carrier-honesty
  negative control, raw `future_mut` replacement compiles as the handler-side
  mutation escape, while declaring the carrier `may_remain_ready` fails the
  sealed readiness check;
- cancel-and-drain on every in-flight state;
- the blocking-region reference trace uses
  `Region { x: 0, y: 0, width: 368, height: 112 }` and exactly 82,432 RGB565
  bytes. Its eight ordered calls are `CASET [00 00 01 6f]`,
  `PASET [00 00 00 6f]`, `RAMWR` at offset 0 for 16,380 bytes, four
  `RAMWRC` chunks at offsets 16,380, 32,760, 49,140, and 65,520 for 16,380
  bytes each, then one `RAMWRC` at offset 81,900 for 532 bytes. Independent
  failure injection at every boundary proves the exact prior prefix, reported
  stage and, for pixel stages, command/index/offset/length, absence of every
  later call, writer and pixel pointer identity, an unwritten settlement, and
  owning-sweep poison/abort.
  The success trace proves the exact resources, written settlement, and
  owning-sweep advance;
- blocking preflight traces exercise the exact section-6.7 precedence: zero
  width; zero height; X overflow; Y overflow; out-of-bounds start/extent; and
  short/long buffers, all with zero I/O and exact error payloads. Positive
  boundary controls cover the first row (`y_end == 0`), origin `1×1`, exact
  bottom/right endpoint, and a nonzero origin's big-endian coordinate encoding.
  A compile failure rejects
  an external `BlockingRegionWrite` implementation and separate failures reject
  permit/result/witness construction; raw HAL calls remain the compiling
  bypass. Compile-pass controls accept and drop an opaque `BlockingSettled`,
  accept arbitrary same-source target bus parts for `try_new`, and cite
  `constraint_erasure_boundaries.rs` for the kernel's existing proof that
  handler interiors remain unchecked (including synchronous blocking work).
  The Xtensa fixture invokes this same engine on real SPI2/GDMA/pins,
  uses fixed 16,380-byte RX/TX scratch with no allocator, recovers the exact
  resources, proves that admission restores a deliberately shortened TX
  descriptor length, links at the pinned HAL revision, and is inspected for
  allocator symbols. Host traces and a linked ELF remain non-controls for panel
  interpretation, physical delivery, and visible output;
- the concrete async-region reference trace uses
  `Region { x: 0, y: 0, width: 368, height: 16 }` and exactly 11,776 logical
  RGB565 bytes. It records literal CASET `[00 00 01 6f]`, PASET
  `[00 00 00 0f]`, and one RAMWR envelope in order. Independent failure at
  each of those three boundaries proves the exact prior prefix, classification,
  absence of every later call, and transport/command-scratch/pixel identity.
  Preflight cases cover the shared geometry precedence, a valid in-panel
  region above 16,380 bytes, short and long logical descriptor lengths, and
  zero I/O. Positive cases include the first row, exact 16-row reference,
  nonzero origin, and the largest even payload at or below the cap. The shared
  target-neutral construction core independently rejects short RX with valid
  TX and short TX with valid RX before configuration, accepts capacities
  exactly 16,380, and requests TX descriptor normalization only after both
  checks. Resource-model traces extract the rejection bundle and prove every
  token's identity. Separate Xtensa compile-fail controls reject SPI3,
  DMA_CH1, and swapped GPIO4/GPIO5 role arguments at the branded `Parts::new`
  boundary; the compile-pass constructor/extractor control uses the exact
  SPI2/DMA_CH0/GPIO4–7/11/12 order and both rejection branches. A distinct
  compile-pass control keeps a dishonest external `FlightStarter` and raw HAL
  construction available, proving that this row reviews only the named
  profile-owned adapter rather than silently claiming generic sealing;
- host lifecycle traces run the same target-neutral start and completion-slot
  cores called by the profile-owned Xtensa shell
  through register-then-recheck in both completion positions, waker
  replacement outside exclusion, completion-versus-cancel linearization,
  start rejection, ordinary drop/disarm, exact recovery, owning-sweep
  written/cancelled settlement, and sequential transport/buffer reuse. The
  exact-Xtensa fixture replaces its RAMWR-only fixture-local adapter with the
  branded profile implementation. Its named `#[inline(never)]` link-only
  reactor function contains a generated `reactor!` over the real
  `OptionalInlineOneShot<InFlight<...>>`; handler code paths cover two written
  settlements with same-carrier rearm and a third-flight drain with honest
  `Completed`/`Cancelled` branching. A separate `#[inline(never)]` poll shim
  black-boxes the pinned future and polls it exactly once with `Waker::noop`;
  there is no manual executor loop and the branches are linked, not observed.
  A second retained outer hook explicitly drops an armed real `Sources`, its
  old sweep, and calls `abandon_active`, retaining concrete target drop glue.
  The entrypoint black-boxes but does not call either outer function pointer.
  Target Clippy, locked optimized link, HAL-source identity, empty undefined
  symbols, allocator-symbol absence for this exact noop-waker binary, and
  nonzero concrete reactor/drop symbols from `nm -S -C` are recurring CI
  gates. The firmware paths are retained but not executed by CI, so this is
  explicitly not executor scheduling, IRQ delivery, wake-count, drop-runtime,
  arbitrary-RawWaker allocation, or panel evidence;
- sweep-plan coverage: target-consuming start through `FlightStarter::start` with a crate-issued `StartPermit` is the only public flight construction; one target is outstanding per plan position; the cooperative driven path delivers every recovered transfer settlement to its owning `Sweep::settle`; matching written settlements are the only path to `SweepWritten`; matching failed/cancelled settlements poison and force abort; abort rejects outstanding work; dropped or wrong-owner settlements and abandonment are published escapes with drop-plus-`abandon_active` full-repaint recovery and idle-`invalidate` protection when stale work may overlap; full-repaint and sticky-invalidation obligations are set and cleared per the state table;
- full-frame versus stripe-swept RGB565 pixel equivalence through the real
  target/start/transfer/recover/settle witness chain: ordinary reconstruction,
  a live scene-state change during a sweep that is deferred to the next
  `FrameEpoch` snapshot, and a failed/partially written sweep followed by a
  forced full-repaint sweep;
- demand-policy state table: request-during-sweep, stale/duplicate `finish`, slow-sweep throttling under paused time;
- touch generation machine: IRQ before registration/during read/after flag sample; INT still asserted; I²C failure restoring pending state; bounded services per activation; startup with INT asserted; generation wrap;
- Outcome-B-specific (if selected): receiver closure, task shutdown, resource-return backpressure;
- an external-consumer canonical reactor fixture (the seam gate, section 10) and a target compile/link fixture against the chosen HAL SHA;
- runtime (not compile-fail) oracles for drop/cancel transitions — the revision-1 "dropped permit compile-fail" was impossible as written and is removed (finding 13).

Negative controls published beside them, as always.

## 9. Board anchor, package readiness, and K2R-1 obligations

Revision 10 resolves the `write_region` upstream/fork decision: Kittens owns a
minimal reviewed SH8601 region transaction whose protocol provenance is exact
upstream commit `4bcddfd529017135f19a5a9a6e79dd6b8ef1b460`, while the stock driver crate
is deliberately not compiled. The actual adapter dependency is `esp-hal` at
exact git revision `d48f747ba28accdc51779ba193eba923138e0382`. This explicitly
replaces the earlier ambiguous requirement to compile against both a display-
driver SHA and a HAL SHA; provenance-only review is not mislabeled as a Cargo
dependency.

The blocking capability's **host + exact-Xtensa-link** row closes only after
all section-8 traces and controls pass against the single shared engine, the
real target adapter is invoked in the no-allocator firmware, the locked
optimized ELF links, and allocator-symbol inspection is clean. That scope does
not require hardware and does not freeze the still-open async capabilities.

The revision-11 concrete async row may close independently with **host +
exact-Xtensa-reactor-link scope** only after its section-8 traces run against
the shared engine and concrete-shaped lifecycle, the branded SPI2 adapter is
the one used by the target fixture, a generated reactor retains/rearms/drains
its real `InFlight`, and the locked no-allocator ELF plus source/symbol gates
pass. This reviews one additive implementation under the open traits. It does
not seal them, authorize a 0.2.0 version change, prove a target executor, or
close the K2R-0 freeze.

That matrix now passes for the named adapter: the host protocol, failure,
scratch-admission, completion-slot, cancellation, recovery, reuse, and drop
oracles execute; the three exact-singleton target failures and one Parts
roundtrip control reach their intended diagnostics; and direct target-library
Clippy plus the locked optimized fixture link, exact HAL source, empty
undefined table, allocator-symbol absence, and retained reactor/poll/drop
symbols pass. The scoped row is therefore closed. None of those observations
execute the retained target hooks or broaden the exclusions above.

### 9.1 Orthogonal local packaged-source + registry-HAL Xtensa consumer

Revision 12 adds one local target gate for the current workspace source as
Cargo places it in a crate archive. This is a package-shape/registry-HAL
compatibility row, not a K2R protocol-stage prerequisite or a claim that the
current declared version may be republished. Version 0.1.1 is already an
immutable crates.io artifact from older source; any future release must be
human-authorized and correctly versioned. The local gate MUST run from a clean
committed checkout:

1. The workspace-pinned Cargo 1.96.0 runs
   `cargo +1.96.0 package -p kittens-render --locked` and produces the current
   locally generated `kittens-render-0.1.1.crate` without `--allow-dirty`.
   This filename reflects the current workspace declaration only and is not a
   publish candidate. The archive's
   `.cargo_vcs_info.json` MUST report the exact current `git rev-parse HEAD`
   and no truthy dirty marker; Cargo versions that omit `git.dirty` for a clean
   tree satisfy that rule. Its `path_in_vcs` MUST be exactly
   `crates/kittens-render`. Neither the archive nor its extracted directory is
   committed.
2. The generated manifest MUST contain target dependency
   `esp-hal = "=1.1.0"` with `esp32s3` and `unstable`, and MUST contain no git
   URL or revision for that dependency. CI MUST inspect the generated manifest
   structurally through Cargo metadata rather than depending on TOML rendering.
   This checks Cargo's normalized package output, not `Cargo.toml.orig` or a
   hand-copied manifest.
3. The standalone `fixtures/render-packaged-xtensa-probe` crate MUST have an
   empty `[workspace]`, depend on the extracted normalized package at exact
   relative path `../../target/package/kittens-render-0.1.1` with exact
   declared version `=0.1.1`, and depend
   directly on registry `esp-hal = "=1.1.0"` with no git source and no
   `[patch]`. It has no direct `kittens` or `critical-section` dependency. It
   enables `esp32s3-sh8601-async`, which includes the blocking adapter, and its
   locked metadata MUST contain exactly one `esp-hal` 1.1.0
   package whose source is the crates.io registry and zero git-sourced
   `esp-hal` packages. CI MUST extract the archive and copy the fixture into a
   matching `$RUNNER_TEMP` directory layout outside the checkout; it MUST NOT
   edit the fixture or generated manifest to make them compose. Its committed
   `.cargo/config.toml` MUST retain the exact-git fixture's Xtensa target,
   `build-std = ["core"]`, linker script, and `-nostartfiles` configuration.
4. The consumer MUST pass direct registry-HAL `SPI2`, `DMA_CH0`, and exact GPIO
   singleton values to the packaged
   `Waveshare18V1Sh8601Parts::new`. A named `#[inline(never)]` retained hook
   MUST cross `try_new`, construct the packaged async starter, and reach the
   target-owned `start_flight` spelling. This makes registry type identity and
   target adapter code generation real rather than inferring them from a
   manifest diff. The entrypoint black-boxes but does not call the hook.
5. From the staged fixture working directory, so its committed
   `.cargo/config.toml` governs `build-std`, exact-target Clippy MUST run once
   directly against the extracted packaged library via `--manifest-path` with
   `esp32s3-sh8601-async`, and once against the standalone consumer. A locked
   optimized consumer Xtensa link MUST pass. The ELF MUST be
   a statically linked Tensilica executable, have an empty undefined table,
   contain zero matches under the established allocator-symbol filter, and
   retain the named hook as a nonzero text symbol. A separate recurring CI job
   repeats the clean package, provenance, normalized metadata, both Clippy
   gates, link, and inspection sequence without replacing the repository's
   exact-git fixture job.

Passing this row closes only **PACKAGED-SOURCE + REGISTRY-HAL XTENSA-LINK
SCOPE**. It proves that the clean candidate archive's normalized target
manifest and public board constructor compose with the registry HAL source.
The repository exact-git fixture remains the reviewed source-revision gate;
the registry package is version/checksum selected rather than git-SHA
selected. Host package verification alone, the exact-git fixture, a dirty
archive, a copied source tree, or any `[patch]` is an explicit non-control.
The retained hook is not target execution, interrupt, wake, cancellation/drop,
or panel evidence. The row does not upload `kittens-render`, wait for an index,
download an exact published `kittens-render` version, authorize publication or
a version bump, or prove arbitrary executor-waker allocation behavior. Those
boundaries remain separately named below.

K2R-1 owns the first real target-runtime and board evidence. Its gate requires:
a named no-allocation-after-init target executor that repeatedly polls the
generated reactor with its real `Waker`; a minimal board coordinator that
serializes panel initialization/commands against stripe starts; observed SPI2
interrupt registration, both completion positions, wake counts, cancellation,
recovery, and drop; a concrete TP_INT wake producer plus one contiguous FT3168
snapshot transaction and failure re-latch; panel command acceptance, physical
region/color output, TE behavior, touch reports, latency, and measured static,
stack, heap, and bandwidth budgets. The executor's `RawWaker` allocation and
critical-section behavior MUST be measured/reviewed rather than inferred from
the noop-waker link fixture. Uncalled hooks, host models, and linked symbols are
explicit non-controls for every item in this paragraph.

TE measured behavior, panel initialization/command acceptance, physical region
placement, RAMWRC interpretation, RGB565 channel/byte fidelity, visible
output, tearing, latency, and per-backend measured peak memory/bandwidth remain
K2R-1 board-HIL obligations. No host byte trace or linked ELF discharges them.

## 10. The bilateral seam (merge with the harness workstream) — gates the K2R-0 freeze, not K2R-0A closure

This spec proposes and the sibling `kittens-code` spec must co-sign (finding 15): a single seam section, mirrored verbatim in both documents, defining — construction of render sources by the harness's reactor owner; the typed facts (`StripeWritten`, `SweepWritten`, touch reports) as ordinary arm events; ordering/starvation declarations recommended for them; task ownership for Outcome B if selected; and teardown order. The K2R-0 protocol freeze of both workstreams is gated on an external-consumer fixture: a canonical reactor owned by *harness-style* code that consumes this profile end to end. This does not reopen the scoped K2R-0A feasibility closure. Until co-signed, neither spec claims the merge. The render-owned carrier fixture from section 8 is necessary kernel-admission evidence but is not that foreign harness-style consumer and does not close the bilateral seam.

## 11. Slice acceptance

- **K2R-0A — CLOSED WITH HOST + PORTABLE-LINK + EXACT-XTENSA-LINK
  SCOPE**: the matrix ran against recorded SHAs; mechanism C in the A-prime
  carrier was selected; the exact target compile/link probe and kernel-carrier
  reactor/portable-link oracles pass; and section 6 contains the demonstrated
  normative shapes. This closes feasibility, not target execution or HIL.
- **Blocking region — CLOSED WITH HOST + EXACT-XTENSA-LINK SCOPE**: section
  6.7's sealed adapter, every host oracle/control, and the linked/symbol matrix
  pass. This does not close the seam, async generic seals, or K2R-1.
- **Single-payload async adapter — CLOSED WITH HOST + EXACT-XTENSA-REACTOR-
  LINK SCOPE**: section 6.8's named implementation and complete host/target
  link matrix pass. Generic sealing, async RAMWRC, and K2R-1 remain open.
- **Normalized packaged-source consumer — OPEN, CONTRACT SELECTED**: section
  9.1's clean package, registry-HAL consumer, and exact target matrix have not
  yet run. This orthogonal local package row is not a K2R freeze prerequisite.
- **K2R-0 — GATED ON EXACTLY TWO FREEZE ITEMS**: the bilateral seam must be
  co-signed and its foreign fixture pass; and `FlightStarter`/`OwnedTransfer`
  must be sealed at an explicitly authorized breaking API boundary. A version
  bump or publication is not itself required to implement/review the seal, but
  no release may mis-version the breaking change. All other K2R-0 host,
  portable, cancel/drop, state-table, external-`no_std`, clippy/fmt/doc, and
  negative-control evidence is closed.
- **K2R-1 — GATED, NO TARGET-RUNTIME/HIL DATA**: after the K2R-0 protocol
  freeze, it owns target executor scheduling, board coordinator/init, SPI2 and
  TP_INT delivery, wake/cancel/drop observations, TE, physical pixels/touch,
  latency, and measured memory/bandwidth. Link-only hooks cannot satisfy it.
- **Publication — HUMAN-ORDERED, NOT AUTHORIZED BY THIS SPEC**: it is
  orthogonal to K2R stages. After a future correctly versioned authorized
  release, an exact-version Xtensa download smoke may validate that registry
  index artifact but cannot substitute for K2R-1 HIL.

## 12. Review log

Spec review, 2026-08-08: Codex `gpt-5.6-sol`, ultra effort, read-only; interrupted once and resumed in-session; 18 findings (12 blocking, 5 important, 1 minor), full text retained in session transcript and `/tmp/codex-spec-review-2.txt` at review time. Disposition: findings 1–14 and 16–18 adopted as restructured above (1→§6 provisional; 2→§7 compile probe + busy-poll rejection; 3→§5.1 cancel-and-drain; 4→§7 matrix/selection rule/∅; 5→§7 candidate B artifact; 6→§6 typestate caveats + §3 emittability; 7→§6/§9 blocking-capability gate; 8→§5.2 sealed capabilities; 9→§5.3 progress token, damage deferred; 10→§6 sweep token + finish; 11→§5.4 honest semantics; 12→§5.4 generation machine + §7 touch admission; 13→§8 runtime oracle; 14→§8 trace matrix; 16→§6 `Sweep<S>`; 17→§4/§5.5 milestone honesty; 18→§6 `on_eligible` removed). Finding 15 adopted as section 10's bilateral gate; the co-signing edit to `docs/kittens-code/SPEC.md` belongs to the sibling workstream and is recorded here as a pending cross-workstream ask, not performed unilaterally. Verdict accepted in full.

Exit review round 3, 2026-08-08: six findings and three advisories, all
accepted; full text is retained at
`reviews/2026-08-08-exit-review-3-codex.md`, and the agreed batch-6 shapes are
recorded in `K2R0A-LOG.md`. Revision 4 incorporates their controlling
contract changes in sections 5, 6, and 8 before the batch-6 implementation:
structural target/start coupling; exact-one written-or-unwritten settlement extraction;
poison-on-failure; single-outstanding issuance; honest pseudocode/evidence
labels; checked epoch and eligibility horizons; move-only witness controls;
and the shared-backing-store spare escape.

Exit review round 4, 2026-08-08: findings 1 and 3 remained blocking and
finding 5 remained partial; full text is retained in
`reviews/2026-08-08-exit-review-4-codex.md`, and the agreed batch-7 resolution
is recorded in `K2R0A-LOG.md`. Revision 5 incorporates the controlling
contract changes before implementation: `FlightStarter` replaces the
unsealable callback and shares `OwnedTransfer`'s K2R-0 breaking-freeze gate; pairing
is claimed only under sealed integrations, with experiment-phase honesty
published as an escape; abort rejects an outstanding target until settlement;
idle invalidation sticks to the next minted epoch; and the remaining
drop/abandon boundary is bounded by the reviewed adapter's synchronous
cancel/disarm `Drop` contract rather than mislabeled as static prevention.

Exit review round 5, 2026-08-08: all five must-fixes were accepted; full text
is retained at `reviews/2026-08-08-exit-review-5-codex.md`, and the agreed
batch-8 resolution is recorded in `K2R0A-LOG.md`. Revision 6 incorporates the
controlling changes before implementation: a private-constructor, non-`Clone`,
lifetime-bound `StartPermit` makes direct starter dispatch unavailable to safe
external code; raw-closure and both `InFlight` construction paths are pinned by
compile-fail controls; demand rejection evidence begins with a real successful
write and proves exact future eligibility plus successor epoch; and the chosen
narrow-and-publish resolution states that owning-sweep settlement delivery is
cooperative. Lost or misapplied settlements and abandonment are explicit
escapes recovered by dropping old values, `abandon_active` full repaint, and
idle `invalidate` when stale physical work or invalidation may overlap.

Revision 7, 2026-08-09: the draw-target integration contract was written
before its implementation. The default-off `embedded-graphics` feature adds a
no-alloc, global-coordinate RGB565 stripe target whose only constructor binds
the owning `Sweep`, its outstanding `StripeTarget`, and an exact caller byte
buffer. The target reports full-panel bounds, clips into the stripe, and packs
the pinned anchor driver's host RGB565 byte order. Section 8 now names the
three real-witness-chain pixel-equivalence cases required to close the
manifest row. Physical color/format fidelity remains explicitly behind board
HIL.

Revision 8, 2026-08-09: the publication-readiness corrections record the
Xtensa target compile/link feasibility row as **CLOSED WITH SCOPE**, while
leaving board HIL and silicon interrupt delivery, the kernel-admitted
`reactor!` fixture, the bilateral `kittens-code` seam, blocking `write_region`,
and pre-freeze capability sealing open. The concrete adapter contract now
requires task-waker clone/drop/wake behavior outside the global critical
section. An experimental 0.1.x evidence publication is explicitly not the
freeze that triggers sealing; any later sealing must use an appropriate
breaking-version boundary.

Revision 9, 2026-08-09: the previously unspecified kernel source carrier is
now contracted before implementation. The selected shape is one sealed,
rearmable, inline `OptionalInlineOneShot<F>` in the kernel with `F: Future +
Unpin`, carrying the existing conditionally-`Unpin` `InFlight` future. The
required evidence is a real reactor over both selection-loss positions,
same-carrier rearm, graceful drain, drop behavior, and external thumb/wasm
links. Arbitrary-future honesty, direct polling/await, cooperative settlement
delivery, mutable-handler replacement, silicon behavior, `write_region`,
capability sealing, and the bilateral seam remain explicit non-guarantees or
gates. Implementation review later corrected the draft's overstatement that
`future_mut` could not remove work: `&mut F` necessarily permits
`mem::replace`, so that path is now a named compiling escape.

Revision-9 implementation review, 2026-08-09: Claude Code
`claude-opus-4-8` at maximum effort reported **SOUND, 0 P0–P2 issues** after
statically tracing the carrier, generated reactor, every new oracle/control,
dependency boundary, and scope claim. Its three P3 notes were verification
hygiene (discharged separately), optional selection-loss/rearm symmetry (not a
contract gap), and cosmetic README wrapping (adopted). The retained review is
`reviews/2026-08-09-carrier-precommit-claude.md`.

Revision 10, 2026-08-09: the blocking-region design is selected before
implementation. An audited stock-driver defect rejects the valid first-row
window (`y_end == 0`), while its public partial-flush path allocates and cannot
borrow the profile's external stripe bytes. The selected replacement is a
minimal private SH8601 transaction derived from the exact upstream commit,
not a false claim that the stock crate is compiled. A sealed-from-day-one,
profile-owned ESP32-S3 transport is the only production implementation; a
private permit and target-owned operation make it the single proof-bearing
spelling. The exact command/chunk/failure matrix, resource return, conservative
failure settlement, fixed scratch sizes, no-allocation target link, source
pin, and HIL non-guarantees are now normative. Implementation evidence was
gated until the matrix and exact-HAL ELF passed; those gates subsequently
closed only the host + exact-Xtensa-link row recorded in sections 9 and 11.

Revision-10 implementation drift, 2026-08-09: source-level review of the
pinned HAL found that `SpiDmaBus::half_duplex_write` checks TX backing capacity
but does not relink a caller-shortened descriptor chain. The capacity-only
admission rule was retained and made executable by requiring the concrete
constructor to restore the exact 16,380-byte TX descriptor length after all
rejection checks. The target fixture deliberately enters with length one and
checks the recovered normalized length; no silicon behavior is inferred.

Revision-10 implementation review, 2026-08-09: Claude Code
`claude-opus-4-8` at maximum effort initially rejected an allocator-symbol CI
regex that missed realistic `esp_alloc`, `__rdl`/`__rg`, and `GlobalAlloc`
entry points. After the checker was widened and tested against both the real
ELF and synthetic positive/negative symbols, its follow-up verdict was
**SOUND, zero unresolved P0–P2 findings**. Target Clippy and the review's
descriptor/runtime-wording hygiene findings were also adopted. The retained
review is
`reviews/2026-08-09-write-region-implementation-precommit-claude.md`.

Revision 11, 2026-08-09: the next additive async-integration shape is selected
before implementation. The fixture-local adapter was wake/ownership-feasible
but region-dishonest: it ignored X/Y and emitted only RAMWR. The selected
profile-owned replacement brands the exact Waveshare V1 SPI2/DMA_CH0/pin
construction, shares revision 10's private geometry/window truth, and admits
one exact region of at most 16,380 bytes through CASET, PASET, and one owning
RAMWR transfer. Safe multichunk buffer offset/rejoin is unavailable in the
pinned HAL under crate-wide `forbid(unsafe_code)`, so async RAMWRC is deferred
rather than implemented through destructive copying or semantic pretense. The
required evidence adds exact start-boundary/resource traces and a generated-
reactor Xtensa link. Generic trait sealing, target execution, publication, and
every silicon property remain separate gates.

Revision-11 spec review, 2026-08-09: Claude Code `claude-opus-4-8` at maximum
effort reported **SOUND, zero P0–P2 findings** after checking the complete
contract/diff against the pinned HAL types and methods, exact construction and
rejection resource graph, idle-facade branding, cancel race, generated-reactor
and drop-symbol scope, dependency boundary, and non-guarantees. Its changelog
P3 was adopted; its two implementation watch items remain pinned above. The
retained review is
`reviews/2026-08-09-async-region-spec-precommit-claude.md`.

Revision-11 implementation review, 2026-08-09: Claude Code 2.1.224
`claude-opus-4-8` at maximum effort reported **SOUND, zero P0–P2 defects**
after tracing the exact pinned-HAL API, protocol/preflight ordering,
acceptance-atomic start, concrete-Waker register/recheck/cancel/drop races,
resource recovery, singleton controls, generated-reactor/drop hooks, and CI
enforcement. The host suite and exact target link/symbol/source matrix close
only the named adapter's host + exact-Xtensa-reactor-link row; generic sealing,
async RAMWRC, target execution, arbitrary-executor waker behavior, publication,
the bilateral seam, and silicon remain open. The retained review is
`reviews/2026-08-09-async-region-implementation-precommit-claude.md`.

Revision 12, 2026-08-09: acceptance staging is repaired after the revision-11
evidence closed every literal K2R-0A feasibility criterion. K2R-0A is now
closed with host + portable-link + exact-Xtensa-link scope; the bilateral seam
and generic capability sealing gate the K2R-0 protocol freeze; target executor,
board coordinator, silicon, and measurements belong to K2R-1; publication is
orthogonal and human-ordered. The revision also splits the previously
conflated registry fallback into a locally executable clean packaged-source +
registry-HAL Xtensa consumer gate and a separate future-release exact-version
download smoke. The former is selected before implementation and remains open
until section 9.1's matrix passes; the already published 0.1.1 artifact remains
historical and cannot stand in for current source.

Revision-12 preimplementation review, 2026-08-09: Claude Code 2.1.224
`claude-opus-4-8` at maximum effort reported **SOUND, zero P0–P2 defects**
after checking the acceptance map, immutable-0.1.1 distinction, clean embedded
VCS provenance, normalized registry-HAL graph, temporary staging topology,
fixture-controlled Xtensa `build-std`, both target Clippy gates, retained-hook
scope, and every publication/runtime/HIL non-guarantee. The retained review is
`reviews/2026-08-09-packaged-registry-spec-precommit-claude.md`.
