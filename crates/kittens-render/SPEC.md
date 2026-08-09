# kittens-render profile specification (K2R-0A / K2R-0 slices)

- Status: revision 3, 2026-08-08 (section 6 amended into the normative K2R-0 surface per the K2R-0A outcome and exit-review round 1; revision 2 recorded the review adoption). Revision 1 was reviewed by the same external reviewer as the research (Codex `gpt-5.6-sol`, ultra effort): 12 blocking, 5 important, 1 minor finding; verdict — *K2R-0A may start only as a non-freezing feasibility experiment; K2R-0 must not start yet.* Revision 2 adopted that verdict and all findings (disposition log in section 12) and made section 6 provisional; the K2R-0A experiment then selected and demonstrated the shapes, and exit-review round 1's restructuring landed — so revision 3 amends section 6 into the normative surface, exactly the sequence the verdict required.
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
2. **Sealed capabilities — a pre-freeze obligation.** The transport capability traits will be sealed to reviewed backend adapters before any freeze, because ownership alone cannot distinguish a blocking `start_region` from an honest one (finding 8). During the experiment they are deliberately open so probes and models can implement them (section 6.2); the open state is itself a recorded gate, not a contradiction. Raw backend access remains the documented compiling escape surface.
3. **Epoch discipline.** One sweep-owned snapshot per sweep, exposed only through `&S`; every transmitted stripe is fully reconstructed from that logically immutable state. Ordinary ownership enforces the owned/shared-reference boundary, but unconstrained `S` can contain interior mutability or handles to shared external state: keeping those stable for the epoch is a documented caller obligation and compiling escape surface. Any failure terminates the current epoch and forces a full repaint. Sweep completion is decided **only** by a consuming progress token over a fixed, validated full-panel plan — never by caller assertion (finding 9).
4. **Honest touch semantics.** Latest-state-with-coalescing: every surfaced report is complete and untorn; intermediate transitions may coalesce; an atomic `produced_generation`/`serviced_generation` state machine with a bounded number of snapshot services per activation and re-latch on generation change, asserted INT, or failure (findings 11, 12). The ISR-side wake-capable producer handle is part of the K2R-0A admission question, not assumed.
5. **Milestone honesty.** `StripeWritten` and `SweepWritten` only (finding 17).
6. **Board anchor facts** of RESEARCH section 2, revision-keyed.

## 6. Normative K2R-0 surface (amended per K2R-0A outcome and exit-review round 1)

Revision 3 amendment: the K2R-0A experiment selected its mechanism (C
completion in the A′ carrier, `K2R0A-LOG.md`) and exit-review round 1
restructured the composition; this section is now **normative** for the
K2R-0 host slice, superseding revision 2's provisional candidates. The one
still-open shape is the kernel-admitted source carrier (K2R-0A open item
3): how these values appear as `reactor!` arms awaits the kernel admission
slice and is explicitly not specified here.

### 6.1 Geometry and identity

`Region` — global panel coordinates, never stripe-local. `FrameEpoch` —
scene-snapshot identity, monotonic within one demand machine's documented
2^64-sweep operating horizon, minted only by `FrameDemand`; no public
constructor.

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

`StripeTarget` — non-`Clone`, private-field identity (demand, epoch, region),
minted only by `Sweep::next_target` and consumed by `InFlight::new`; a
transfer and the identity it claims cannot be paired independently.

`InFlight<X, S>` — `&mut`-polled and **conditionally** `Unpin`: exactly
`X: OwnedTransfer + Unpin` and `S: Unpin`; `X::Transport` and `X::Buffer`
need no `Unpin` bound because they are not stored separately in flight. It
owns the transfer, independently writable spare, and `StripeTarget`;
`begin_drain`/`poll_complete` is the only resource-returning path; ordinary
drop is the documented non-returning boundary (but integrations MUST disarm
their completion slot on drop — no stale registrations).

`Settled<T, B, S>` — private resources, outcome, and single-use target;
safe external code cannot construct one or rewrite its proof-bearing state.
`Settled::stripe_written(&mut self)` is the **only mint** for a
`StripeWritten` witness, consumes the target at most once, and succeeds only
for `Completed` settlements: marking a cancelled, failed, never-started, or
already-witnessed stripe is unrepresentable.

### 6.3 Sweep

`PanelGeometry` — admitted full-panel geometry; the anchor board is
`WAVESHARE_18_V1`. `custom_unvalidated_panel` is the deliberately loud,
compiling escape for hosts and unadmitted hardware.

`SweepPlan` — validated full-panel stripe plan (empty/zero/overflow
rejected) over a `PanelGeometry`, fixed at `FrameDemand` construction;
sweeps cannot substitute another. `Sweep<S>` — crate-owned, minted only by
`begin_sweep`; owns the snapshot (shared-reference access only), the plan,
the repaint-obligation state, and the provenance-branded epoch. Ordinary
borrowing does not freeze interior mutable or externally shared state inside
`S`; callers MUST keep that state logically immutable for the epoch.
`mark_written(StripeWritten)` enforces demand provenance, epoch match, and
plan order; `finish(self)` yields `(SweepWritten, S)` only at full coverage;
`abort(self)` yields `(AbortedSweep, S)` at any point. Coverage is a
construction, never a caller claim. Every K2R-0 plan covers the full panel;
`full_repaint == false` means no outstanding forced-repaint obligation, not
a partial sweep.

### 6.4 Demand

`FrameDemand` — owns the plan and accepts caller-supplied `Tick` values from
a trusted monotonic platform time source. Regressing written-settlement
times are clamped, but arbitrary forward values are outside this type's
validation. Demand provenance uses a thumbv7em-compatible `AtomicU32` with
checked exhaustion, widened to `u64` in witnesses; epoch uniqueness is
bounded by the documented 2^64-sweep lifetime of one demand machine.
`request` (the only kittens-tui-shared vocabulary), `begin_sweep(now,
snapshot)` (sole eligibility acknowledgment; one active demand epoch),
`eligible_at`, `finish_written(SweepWritten, now) ->
Result<WrittenDisposition, ForeignSweep>`, `finish_failed(AbortedSweep,
now)`, `abandon_active` (dropped-sweep recovery), `invalidate` (bool latch;
a mid-sweep invalidation makes that sweep's settlement
`DiscardedByInvalidation`: obligations retained, throttle unchanged).
`abandon_active` is witness-terminal only: before beginning the replacement
sweep, callers MUST drain or settle every transfer from the abandoned one,
because a retained old `Sweep` can still drive physical writes. Foreign or
stale witnesses are rejected **without mutation, in release builds**.
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
- sweep-plan coverage: the consuming progress token is the only path to `SweepWritten`; failure aborts the epoch; full-repaint obligation set and cleared per the state table;
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
