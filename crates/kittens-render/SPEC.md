# kittens-render profile specification (K2R-0A / K2R-0 slices)

- Status: revision 2, 2026-08-08. Revision 1 was reviewed by the same external reviewer as the research (Codex `gpt-5.6-sol`, ultra effort): 12 blocking, 5 important, 1 minor finding; verdict — *K2R-0A may start only as a non-freezing feasibility experiment; K2R-0 must not start yet.* This revision adopts that verdict and all findings (disposition log in section 12). The controlling consequence: **section 6 is a provisional candidate surface, not a normative API.** K2R-0A selects and demonstrates the real shapes; this spec is then amended before K2R-0 freezes anything.
- Parent contracts: root [`SPEC.md`](../../SPEC.md); [`RESEARCH.md`](RESEARCH.md) revision 2; [`crates/kittens-tui/SPEC.md`](../kittens-tui/SPEC.md) section 10 (generic-gate comparison, unresolved here); the sibling harness contract `docs/kittens-code/SPEC.md` (seam obligations, section 10 below).
- Hardware anchor: **Waveshare ESP32-S3 1.8" AMOLED Touch, V1 — SH8601 display, FT3168 touch, 368×448** (`LCD_TE` GPIO13, `TP_INT` GPIO21, schematic-confirmed).
- Normativity: **MUST/SHOULD** language binds only sections 5, 7, 8, 9, 10, and 11 (the experiment design, protocols-as-rules, and gates). Section 6 is explicitly provisional throughout.

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

- **no `BusIdle` or `FramePresented` facts in these slices** — both transports expose exactly one observable completion boundary; physical presentation and bus-idle milestones wait for hardware evidence (finding 17). The facts are `StripeWritten { epoch, region }` and `SweepWritten { epoch }` only;
- **no lossless touch-transition promise** — this slice's touch semantics are *latest-state-with-coalescing, complete untorn reports*; a bounded transition queue with explicit overflow policy is a separately gated follow-on (finding 11);
- **no damage/partial sweeps** — K2R-0 uses one validated, fixed full-panel sweep plan; damage history is deferred (finding 9).

## 5. What is stable in this revision (normative)

1. **Resource-carrying results.** Success and failure values carry every consumed resource back (`Returned`/`Failed` shapes, whatever their final spelling). Ordinary `drop` of an in-flight completion is a **documented non-returning boundary** — the HAL cancels and drops; nothing comes back through `Future::Output`. Recovery on cancellation therefore REQUIRES an explicit cancel-and-drain transition that is driven to settlement and returns transport, sent buffer, and spare (finding 3).
2. **Sealed capabilities.** The transport capability traits are sealed; only reviewed backend adapters implement them, because ownership alone cannot distinguish a blocking `start_region` from an honest one (finding 8). Raw backend access remains the documented compiling escape surface.
3. **Epoch discipline.** One immutable snapshot per sweep; every transmitted stripe fully reconstructed from it; any failure terminates the current epoch and forces a full repaint. Sweep completion is decided **only** by a consuming progress token over a fixed, validated full-panel plan — never by caller assertion (finding 9).
4. **Honest touch semantics.** Latest-state-with-coalescing: every surfaced report is complete and untorn; intermediate transitions may coalesce; an atomic `produced_generation`/`serviced_generation` state machine with a bounded number of snapshot services per activation and re-latch on generation change, asserted INT, or failure (findings 11, 12). The ISR-side wake-capable producer handle is part of the K2R-0A admission question, not assumed.
5. **Milestone honesty.** `StripeWritten` and `SweepWritten` only (finding 17).
6. **Board anchor facts** of RESEARCH section 2, revision-keyed.

## 6. Provisional candidate surface (NOT normative — K2R-0A input)

Everything in this section is a *candidate* the K2R-0A experiment is free to reject (finding 1). Shown to make the experiment concrete, retained from revision 1 with the review's corrections noted:

- `Region` (global panel coordinates), `FrameEpoch` (monotonic, minted by demand policy only).
- `Returned<T, B>` / `Failed<T, B, E>`; `BlockingRegionWrite`/`OwningRegionWrite` split — **with the caveats**: `type Completion` cannot name the safe ESP-HAL adapter on stable Rust without allocation (`impl Trait` in associated types is unstable; boxing violates no-alloc; hand-rolled owner-plus-borrower is the rejected self-reference), so the owning capability's final shape is exactly what K2R-0A must demonstrate — RPITIT, an upstream named transfer state, or another compiled shape (finding 2). `BlockingRegionWrite` does not freeze unless a no-allocation adapter compiles against an exact fork/upstream `write_region` SHA — stock `sh8601-rs` cannot honor it (finding 7).
- Stripe typestates (`PreparedStripe`/`StripeInFlight`) — incomplete as drafted (no constructors, `StartFailed` undefined, region dropped, no spare access or pin projection); the K2R-0A amendment MUST deliver the complete outcome-specific transition API or replace the shape (finding 6).
- `FrameDemand` — revised candidate per finding 10 and 18: an unforgeable **active-sweep token** replaces raw epoch callbacks; one `finish(token, now, outcome)` transition; `begin_sweep(now)` is the sole acknowledgment of elapsed eligibility (`on_eligible` removed); a complete state table (request-during-sweep, stale/duplicate outcomes, initial and clearing rules for the full-repaint obligation) is a K2R-0 deliverable.
- A crate-owned `Sweep<S>` value binding the immutable snapshot, target geometry, and repaint mode, plus a crate-owned monotonic tick representation, replacing revision 1's hand-waved `DrawTarget` and host/target `Instant` split (finding 16).

## 7. K2R-0A: the feasibility experiment (normative design)

A **non-freezing experiment**; its deliverable is a selected-and-demonstrated shape plus an amendment to this spec, or the honest result that no viable shape exists.

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

## 8. K2R-0: protocol suite (design frozen only after the K2R-0A amendment)

K2R-0 MUST NOT begin until this spec is amended with K2R-0A's selected shapes. Its suite is then built as a **named trace matrix** (finding 14) — each trace enumerated, each state transition and transport boundary independently observable — covering at minimum:

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

- **K2R-0A** is done when: the matrix has run against recorded SHAs, one candidate is selected by the ordered rule (or ∅ is recorded), the target compile probe exists in-repo, and this spec is amended with the demonstrated shapes (section 6 re-issued as normative).
- **K2R-0** is done when: the amended trace matrix passes in CI; runtime cancel/drop oracles and negative controls are published; the demand/sweep/touch state tables are complete; the seam fixture passes; the crate builds `no_std` without alloc; clippy/fmt/doc gates clean.
- Only then does K2R-1 (V1 board bring-up) graduate into this document, and the merge proceeds on frozen protocols.

## 12. Review log

Spec review, 2026-08-08: Codex `gpt-5.6-sol`, ultra effort, read-only; interrupted once and resumed in-session; 18 findings (12 blocking, 5 important, 1 minor), full text retained in session transcript and `/tmp/codex-spec-review-2.txt` at review time. Disposition: findings 1–14 and 16–18 adopted as restructured above (1→§6 provisional; 2→§7 compile probe + busy-poll rejection; 3→§5.1 cancel-and-drain; 4→§7 matrix/selection rule/∅; 5→§7 candidate B artifact; 6→§6 typestate caveats + §3 emittability; 7→§6/§9 blocking-capability gate; 8→§5.2 sealed capabilities; 9→§5.3 progress token, damage deferred; 10→§6 sweep token + finish; 11→§5.4 honest semantics; 12→§5.4 generation machine + §7 touch admission; 13→§8 runtime oracle; 14→§8 trace matrix; 16→§6 `Sweep<S>`; 17→§4/§5.5 milestone honesty; 18→§6 `on_eligible` removed). Finding 15 adopted as section 10's bilateral gate; the co-signing edit to `docs/kittens-code/SPEC.md` belongs to the sibling workstream and is recorded here as a pending cross-workstream ask, not performed unilaterally. Verdict accepted in full.
