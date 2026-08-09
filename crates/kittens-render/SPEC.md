# kittens-render profile specification (K2R-0A / K2R-0 slices)

- Status: revision 6, 2026-08-08 (batch 8 incorporates the accepted exit-review round-5 repairs: crate-issued `StartPermit` dispatch, completed flight-construction privacy/regression controls, throttle-anchored rejection evidence, and cooperative owning-sweep reconciliation with its published escapes). Revision 5 incorporated the round-4 starter/abort/invalidation repairs; revision 4 incorporated the round-3 target/settlement/sweep repairs; revision 3 made section 6 normative after the K2R-0A outcome and exit-review round 1. Earlier revision history remains in section 12.
- Parent contracts: root [`SPEC.md`](../../SPEC.md); [`RESEARCH.md`](RESEARCH.md) revision 2; [`crates/kittens-tui/SPEC.md`](../kittens-tui/SPEC.md) section 10 (generic-gate comparison, unresolved here); the sibling harness contract `docs/kittens-code/SPEC.md` (seam obligations, section 10 below).
- Hardware anchor: **Waveshare ESP32-S3 1.8" AMOLED Touch, V1 — SH8601 display, FT3168 touch, 368×448** (`LCD_TE` GPIO13, `TP_INT` GPIO21, schematic-confirmed).
- Normativity: **MUST/SHOULD** language binds sections 5 through 11. Section 6 became normative in revision 3; the kernel-admitted source carrier remains the one explicitly unspecified shape.
- Slice boundary: **K2R-0 host slice** means the host-model protocol surface and oracles may land against amended section 6. It does not mean K2R-0A or full K2R-0 acceptance: the exact Xtensa compile/link probe, kernel admission, seam co-sign, and board HIL remain separately named gates below.

## 1. One-sentence definition

`kittens-render` is the embedded rendering/interaction profile of Kittens: transport capabilities with resource-carrying results, epoch-snapshotted full-panel sweeps with a consuming progress token, a generation-latched touch protocol with honest latest-state semantics, and frame-demand policy — so a bare-metal application's display and input pipelines are declared reactor topology in the same vocabulary as its backend.

## 2. Problem statement

Unchanged from revision 1 in substance; evidence in RESEARCH sections 2–6. On the anchor board: 329,728-byte frames with a ~16.5 ms QSPI wire floor; a display driver whose framebuffer is private; a HAL completion that is borrowing and `!Unpin` — and, sharpened by this revision's review, *unnameable as a stable associated type without allocation*; touch reads that tear across I²C transactions; stripe buffers that carry no history. Each becomes a type, a rule, or a named gate — in that order of preference, and only after its gate passes.

## 3. Consumers and the merge seam

1. **The sibling harness workstream** (`kittens-code`). Its current contract negotiates a protocol-event frontend seam and owns one reactor per session; it does not yet authorize renderer task loops. The merge is therefore a **bilateral gate** (section 10): one seam section, mirrored in both specs, agreed by both workstreams — this spec does not assume loop ownership, and every fact it emits must be an ordinary typed event a foreign reactor arm can consume.
2. **Application authors** on the anchor board.
3. **Component/engine libraries** above the (K2R-0A-selected) draw-surface contract; widgets/layout/scenes are never owned here.

Emittability rule (root 9.4) stands: explicit constructors, stable spellings, no context-dependent sugar — revision 1 violated this by showing typestates with private fields and no constructors; the K2R-0A amendment MUST specify complete construction/transition/teardown APIs for whatever shapes it selects.

## 4. Non-goals

As revision 1 (no widgets; no driver internals; not the generic-gate resolution; no power/AOD — board-coordinator slice; no DMA overlap — K2R-2 gate; no TE synchronization claim), plus review-sharpened exclusions:

- **no `BusIdle` or `FramePresented` facts in these slices** — both transports expose exactly one observable completion boundary; physical presentation and bus-idle milestones wait for hardware evidence (finding 17). The facts are the private, provenance-carrying `StripeWritten` and `SweepWritten` witnesses only;
- **no lossless touch-transition promise** — this slice's touch semantics are *latest-state-with-coalescing, complete untorn reports*; a bounded transition queue with explicit overflow policy is a separately gated follow-on (finding 11);
- **no damage/partial sweeps** — K2R-0 uses one validated, fixed full-panel sweep plan; damage history is deferred (finding 9).

## 5. What is stable in this revision (normative)

1. **Resource-carrying results.** Every driven success or failure settles through `Recovered`/`Settled` and returns the transport, sent buffer, and spare. Ordinary `drop` of an in-flight completion is a **documented non-returning boundary** — the HAL cancels and drops; nothing comes back through `Future::Output`. Recovery on cancellation therefore REQUIRES an explicit cancel-and-drain transition that is driven to settlement (finding 3).
2. **Sealed capabilities — a pre-freeze obligation.** `OwnedTransfer` and `FlightStarter` will be sealed to reviewed backend adapters before any freeze, because ownership alone cannot distinguish an honest region start, acceptance-atomic rejection, or drop cancellation from a dishonest implementation (finding 8; exit-review round-4 finding 1). During the experiment they are deliberately open so probes and models can implement them (section 6.2); the open state is itself a recorded gate, not a contradiction. Raw backend access remains the documented compiling escape surface.
3. **Epoch discipline.** One sweep-owned snapshot per sweep, exposed only through `&S`; every transmitted stripe is fully reconstructed from that logically immutable state. Ordinary ownership enforces the owned/shared-reference boundary, but unconstrained `S` can contain interior mutability or handles to shared external state: keeping those stable for the epoch is a documented caller obligation and compiling escape surface. On the cooperative driven path, callers deliver every recovered `StripeSettlement` to its owning `Sweep::settle`; a matching failed or cancelled settlement poisons that sweep, so it can mint no further target or finish and only abort remains. Rust ownership makes settlements unforgeable, move-only, and non-relabelable, but cannot force delivery: dropping a settlement or misapplying it to another sweep consumes the witness and leaves its owner outstanding. Recovery from either escape is to drop the old sweep and any remaining target/flight, call `abandon_active` (which retains demand and forces a full repaint), and call idle `invalidate` before replacement when stale physical work or external invalidation may overlap; its sticky latch makes the next epoch non-clearing so another full repaint remains due. Ordinary flight drop is the related non-returning escape: the reviewed adapter MUST synchronously cancel/disarm on `Drop`; no settlement witness or resources return. Sweep completion itself is still decided **only** by consuming matching, in-order written settlements over a fixed, validated full-panel plan — never by caller assertion — but settlement delivery is a cooperative contract, not a linear-type guarantee (finding 9; exit-review round-3 findings 2–3; exit-review rounds 4–5 finding 3).
4. **Honest touch semantics.** Latest-state-with-coalescing: every surfaced report is complete and untorn; intermediate transitions may coalesce; an atomic `produced_generation`/`serviced_generation` state machine with a bounded number of snapshot services per activation and re-latch on generation change, asserted INT, or failure (findings 11, 12). The ISR-side wake-capable producer handle is part of the K2R-0A admission question, not assumed.
5. **Milestone honesty.** `StripeWritten` and `SweepWritten` only (finding 17).
6. **Board anchor facts** of RESEARCH section 2, revision-keyed.

## 6. Normative K2R-0 surface (amended through exit-review round 5)

Revision 5 retains the mechanism selected by the K2R-0A experiment (C
completion in the A′ carrier, `K2R0A-LOG.md`) and exit-review round 1
restructuring, with round-3 batch-6, round-4 batch-7, and round-5 batch-8
repairs to the target/start/settlement/sweep lifecycle. This section is **normative** for the K2R-0 host slice,
superseding revision 2's provisional candidates. The one still-open shape is
the kernel-admitted source carrier (K2R-0A open item 3): how these values
appear as `reactor!` arms awaits the kernel admission slice and is explicitly
not specified here.

### 6.1 Geometry and identity

`Region` — global panel coordinates, never stripe-local. `FrameEpoch` —
scene-snapshot identity, monotonic within one demand machine's exact
2^64-minted-epoch operating horizon, minted only by `FrameDemand`; no
public constructor. Epochs 0 through `u64::MAX` are each minted once. A
further `begin_sweep` attempt panics with one build-profile-independent
exhaustion message before mutating demand state.

### 6.2 Transfer boundary

`OwnedTransfer` (sealed-before-freeze; open during the experiment so probes
and models implement it): `poll_done(&mut self, cx) -> Poll<()>` —
**register-then-recheck mandatory** (the check-then-register order has a
lost-wake race; the suite carries a deliberately broken negative control);
reports settlement only. `cancel(&mut self)` — idempotent; classifies and
stores the settlement at its completion-observation **linearization point**
(a racing physical completion after that point is conservatively
`Cancelled`) and MUST wake a registered waker. `recover(self) ->
Recovered<T, B>` — the **sole outcome authority**.

`StartPermit<'a>` — a crate-issued, non-`Clone` dispatch authority with a
private constructor, lifetime-bound to one `StripeTarget::start_flight` call.
The type is public only so experiment-phase integrations can name the
`FlightStarter` signature; safe external code cannot construct one, and its
lifetime prevents a starter from returning the received permit inside its
non-lifetime-parameterized `Error` for later direct use.

`FlightStarter` (sealed-before-freeze on exactly the same schedule as
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

## 7. K2R-0A: the feasibility experiment (normative design)

A **non-freezing experiment**; its deliverable is a selected-and-demonstrated shape plus an amendment to this spec, or the honest result that no viable shape exists. A host-model selection may authorize the K2R-0 host slice and its section 6 amendment, but K2R-0A itself does not pass until the exact target criteria and open items are discharged.

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

**Touch admission** is decided in the same experiment (finding 12): the ISR-side wake-capable generation handle is a kernel-admission question of the same kind, answered by the same matrix.

## 8. K2R-0: protocol suite (host slice amended; full acceptance gated)

The K2R-0 host suite MUST NOT begin until this spec is amended with K2R-0A's host-model-selected shapes. Full K2R-0 acceptance additionally requires K2R-0A's exact target gate and every acceptance item in section 11. The host suite is built as a **named trace matrix** (finding 14) — each trace enumerated, each state transition and transport boundary independently observable — covering at minimum:

- both selection-loss positions for completion; completion before first poll; completion during waker registration;
- cancel-and-drain on every in-flight state; injected failure at every command/chunk boundary of an enumerated reference trace;
- sweep-plan coverage: target-consuming start through `FlightStarter::start` with a crate-issued `StartPermit` is the only public flight construction; one target is outstanding per plan position; the cooperative driven path delivers every recovered transfer settlement to its owning `Sweep::settle`; matching written settlements are the only path to `SweepWritten`; matching failed/cancelled settlements poison and force abort; abort rejects outstanding work; dropped or wrong-owner settlements and abandonment are published escapes with drop-plus-`abandon_active` full-repaint recovery and idle-`invalidate` protection when stale work may overlap; full-repaint and sticky-invalidation obligations are set and cleared per the state table;
- demand-policy state table: request-during-sweep, stale/duplicate `finish`, slow-sweep throttling under paused time;
- touch generation machine: IRQ before registration/during read/after flag sample; INT still asserted; I²C failure restoring pending state; bounded services per activation; startup with INT asserted; generation wrap;
- Outcome-B-specific (if selected): receiver closure, task shutdown, resource-return backpressure;
- an external-consumer canonical reactor fixture (the seam gate, section 10) and a target compile/link fixture against the chosen HAL SHA;
- runtime (not compile-fail) oracles for drop/cancel transitions — the revision-1 "dropped permit compile-fail" was impossible as written and is removed (finding 13).

Negative controls published beside them, as always.

## 9. Board anchor obligations

As revision 1 (TE measured behavior, `write_region` upstream/fork decision, per-backend peak memory/bandwidth budgets with zero-allocation-after-init), with finding 7's sharpening: the `write_region` decision is a **K2R-0A-adjacent gate** — the blocking capability freezes only with a compiled no-alloc adapter against an exact SHA.

## 10. The bilateral seam (merge with the harness workstream)

This spec proposes and the sibling `kittens-code` spec must co-sign (finding 15): a single seam section, mirrored verbatim in both documents, defining — construction of render sources by the harness's reactor owner; the typed facts (`StripeWritten`, `SweepWritten`, touch reports) as ordinary arm events; ordering/starvation declarations recommended for them; task ownership for Outcome B if selected; and teardown order. Acceptance of either spec's slice is gated on an external-consumer fixture: a canonical reactor owned by *harness-style* code that consumes this profile end to end. Until co-signed, neither spec claims the merge.

## 11. Slice acceptance

- **K2R-0A** is done when: the matrix has run against recorded SHAs, one candidate is selected by the ordered rule (or ∅ is recorded), the exact target compile/link probe passes, and this spec is amended with the demonstrated shapes (section 6 re-issued as normative). The current host-model selection is not this full acceptance.
- **K2R-0** is done when: K2R-0A is done; the amended trace matrix passes in CI; runtime cancel/drop oracles and negative controls are published; the demand/sweep/touch state tables are complete; the seam fixture passes; the crate builds and links through an external `no_std` consumer without alloc; clippy/fmt/doc gates clean.
- Only then does K2R-1 (V1 board bring-up) graduate into this document, and the merge proceeds on frozen protocols.

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
unsealable callback and shares `OwnedTransfer`'s seal-at-freeze gate; pairing
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
