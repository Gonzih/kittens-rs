# kittens-render profile specification (K2R-0A / K2R-0 slices)

- Status: controlling contract for the first two kittens-render slices only — **K2R-0A** (completion-delivery feasibility gate) and **K2R-0** (adversarial host protocol suite). Board bring-up (K2R-1) and DMA overlap (K2R-2) are explicitly **not specified here**; they are research-gated in [`RESEARCH.md`](RESEARCH.md) section 9 and graduate into this document only with measured evidence, per the external review verdict recorded in RESEARCH section 10.
- Parent contracts: root [`SPEC.md`](../../SPEC.md) (kernel semantics; section 9.4 profile rules; section 2.1 coverage thesis), [`RESEARCH.md`](RESEARCH.md) revision 2 (this profile's evidence, including the adopted 14-finding external review), [`crates/kittens-tui/SPEC.md`](../kittens-tui/SPEC.md) section 10 (the open generic-gate comparison; this profile is its second arm and deliberately does not resolve it).
- Hardware anchor: **Waveshare ESP32-S3 1.8" AMOLED Touch, V1 revision — SH8601 display driver, FT3168 capacitive touch, ESP32-S3 LX7 dual-core, 368×448** — schematic facts revision-keyed in RESEARCH section 2 (`LCD_TE` = GPIO13, `TP_INT` = GPIO21). Everything in this spec is written so that this exact board is the first place the stack runs.
- The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, and **MAY** are normative within this crate's K2R-0A/K2R-0 boundary only; the root spec controls on conflict.

## 1. One-sentence definition

`kittens-render` is the embedded rendering/interaction profile of Kittens: transport capabilities with resource-carrying results (`write_region` blocking, `start_region` owning-async), typestate stripe overlap, epoch-snapshotted frame reconstruction, a generation-latched touch protocol, and frame-demand policy — so that a bare-metal application's display and input pipelines are declared reactor topology in the same vocabulary as its backend, and one frame in flight is *possession*, not a runtime counter.

## 2. Problem statement (condensed; evidence in RESEARCH sections 2–6)

On the anchor board: a full RGB565 frame is 329,728 bytes with a ~16.5 ms wire floor at 40 MHz QSPI, so frame pacing, TE phase, and write granularity are correctness concerns, not tuning; the maintained display driver's framebuffer is private, so region streaming requires an explicit transport capability; the HAL's owning-DMA completion future borrows the transfer and is generally `!Unpin`, which the sealed `Unpin` kernel source contract cannot admit today; the touch controller's multi-transaction I²C reads tear under interrupt interleavings; and two stripe buffers are scratch, not spatial history, so partial redraw without epoch discipline paints stale pixels. Each of these is a defect class this profile turns into a type, a protocol rule, or a named gate.

## 3. Consumers and the merge plan

Three consumers, in order of immediacy:

1. **The sibling harness effort.** A parallel workstream is building the Kittens-based harness design (`kittens-code`); the two merge into one stack: *harness backend + this renderer on one reactor*. This spec therefore treats its API as a merge surface: every type is constructible from explicit values, every protocol fact is an ordinary event a reactor arm can consume, and nothing assumes it owns the loop — the harness declares the topology; this profile supplies sources, transports, and protocols. Per root section 9.4, profile APIs MUST be emittable by programs: stable spellings, no context-dependent sugar.
2. **Application authors** building an interactive app on the anchor board.
3. **Component/engine libraries** above: widgets, layout, scenes, styling are all built *on* the `DrawTarget`-facing contracts here and are never owned here.

## 4. Non-goals

`kittens-render` is not, and this slice especially is not:

- a widget, layout, scene-graph, styling, or asset framework;
- a display driver — it defines transport *capabilities* that a driver integration implements; `sh8601-rs` internals stay upstream;
- a resolution of the generic render-gate question (tui SPEC section 10): only `request` is shared vocabulary with `kittens-tui`; the protocols are deliberately not unified (RESEARCH finding 12);
- a power/AOD/brightness manager — panel command serialization and the shared-I²C/expander coordinator are a separate, later slice (RESEARCH finding 10); Off/AOD semantics are **out** of this profile until then;
- a DMA-overlap implementation — K2R-2 has a conditional gate (measured frame-time or p99 input-latency win) and is not authorized by this document;
- a claim that TE synchronization is handled — TE facts are recorded, the TE experiment is a K2R-1 gate, and cadence eligibility is kept strictly distinct from TE write-phase safety.

## 5. Architecture and enforcement layers

| Component (this slice) | Provides | Guarantee | Enforcement layer |
|---|---|---|---|
| transport capabilities (§6.2) | `BlockingRegionWrite`, `OwningRegionWrite` | resources (transport + buffer) always come back, on success *and* failure; the two start/completion protocols are never conflated | ownership + resource-carrying result types |
| stripe typestates (§6.3) | `PreparedStripe` → `StripeInFlight` | the in-flight transfer owns the completion and sent buffer; the spare buffer stays independently writable; a second submission of held resources is unwritable | typestate + move semantics |
| frame epochs (§6.4) | `FrameEpoch`, sweep rules | every transmitted stripe is reconstructed from one immutable scene snapshot; any failure or discontinuity forces full repaint | protocol rules + K2R-0 pixel-equivalence oracles |
| frame demand (§6.5) | `FrameDemand` | request coalescing, one sweep in flight, throttle eligibility (`eligible_at`), cadence kept separate | private runtime state + deterministic tests |
| touch protocol (§6.6) | generation latch + snapshot contract | no lost or fabricated input across IRQ/read interleavings; declared coalescing semantics | protocol rules + K2R-0 adversarial interleaving oracles |
| completion delivery (§7) | the K2R-0A decision | a `!Unpin` completion can wake the reactor without dropping its listener | **gate, not yet a guarantee** — K2R-0A decides the mechanism |

The crate core is `#![no_std]`, no-alloc, `#![forbid(unsafe_code)]`; host tests use std. Nothing here redefines kernel semantics.

## 6. Public API (normative in shape for this slice)

Names freeze after the K2R-0 suite passes and the Codex spec review disposition is recorded; shapes below are the contract.

### 6.1 Geometry and identity

```rust
/// A rectangular panel region in global panel coordinates (never
/// stripe-local; RESEARCH finding 6).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Region { pub x: u16, pub y: u16, pub width: u16, pub height: u16 }

/// Identity of one immutable scene snapshot. Monotonic; minted by
/// `FrameDemand::begin_sweep`, never by transports.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct FrameEpoch(u64);
```

### 6.2 Transport capabilities

```rust
/// Successful completion: everything consumed comes back.
pub struct Returned<T, B> { pub transport: T, pub buffer: B }

/// Failure: everything consumed still comes back, plus the error. Panel
/// GRAM/cursor state after a failure is uncertain by contract — §6.4 rule 5
/// forces a full repaint; the value carries no "partial success" claim.
pub struct Failed<T, B, E> { pub transport: T, pub buffer: B, pub error: E }

/// Synchronous region write: when `write_region` returns, the
/// application-visible transfer is complete (controller scanout may
/// continue). This is the sh8601-class boundary.
pub trait BlockingRegionWrite<B>: Sized {
    type Error;
    fn write_region(self, region: Region, pixels: B)
        -> Result<Returned<Self, B>, Failed<Self, B, Self::Error>>;
}

/// Owning asynchronous region write: `start_region` consumes transport and
/// buffer into a completion future. The completion MAY be `!Unpin`; how it
/// is polled from a reactor is exactly the K2R-0A question (§7).
pub trait OwningRegionWrite<B>: Sized {
    type Error;
    type Completion: Future<Output = Result<Returned<Self, B>, Failed<Self, B, Self::Error>>>;
    fn start_region(self, region: Region, pixels: B)
        -> Result<Self::Completion, Failed<Self, B, Self::Error>>;
}
```

Normative: implementations MUST NOT fake one capability with the other — a `start_region` that blocks to completion before returning is a contract violation, not an adapter choice (RESEARCH finding 1). An integration exposes whichever capabilities its underlying API honestly has.

### 6.3 Stripe overlap typestates

```rust
/// Ready to start one stripe: owns the transport, the filled buffer for
/// `region`, the spare buffer, and the epoch being swept.
pub struct PreparedStripe<T, B> {
    transport: T, ready: B, spare: B, frame: FrameEpoch, region: Region,
}

/// One stripe in flight: owns the completion (pin before polling, per the
/// K2R-0A outcome) and — independently — the spare buffer the renderer may
/// fill for the next region of the same epoch.
pub struct StripeInFlight<C, B> {
    completion: C, spare: B, frame: FrameEpoch,
}

impl<T, B> PreparedStripe<T, B>
where T: OwningRegionWrite<B> {
    pub fn start(self)
        -> Result<StripeInFlight<T::Completion, B>, StartFailed<T, B, T::Error>>;
}
```

Normative: the renderer can hold and fill `spare` while the transfer is in flight — the ownership topology is explicit, never an indivisible "surface" (RESEARCH finding 8). `StartFailed` carries transport and both buffers back.

### 6.4 Frame epochs and sweep rules (normative protocol)

1. A sweep renders exactly one `FrameEpoch`: an immutable scene snapshot frozen at `begin_sweep`. State changes during a sweep target the *next* epoch; a transmitted frame is never a mix of epochs.
2. Every transmitted stripe is **fully reconstructed** from background plus the complete ordered scene of its epoch. Stripe buffers are scratch; they carry no spatial or temporal history (RESEARCH finding 5).
3. Stripe targets use global panel coordinates; a stripe target MUST NOT change layout semantics by reporting a stripe-local bounding box (RESEARCH finding 6).
4. Three protocol facts are distinct and MUST never be conflated: `StripeWritten { epoch, region }`, `BusIdle`, and `FramePresented { epoch }` (emitted only when every stripe of the epoch has been written).
5. Any `Failed`, transport reset, or epoch discontinuity invalidates damage history: the next sweep MUST be a full repaint. This is a protocol rule verified by K2R-0 oracles, not a field on a type.

### 6.5 Frame demand

```rust
pub struct FrameDemand { /* private: dirty, sweeping: Option<FrameEpoch>,
                            last_present, scheduled, min_interval, next_epoch */ }

impl FrameDemand {
    /// Explicit throttle policy; no Default.
    pub const fn new(min_interval: Duration) -> Self;
    /// Coalescing demand — the only vocabulary shared with kittens-tui.
    pub fn request(&mut self);
    /// Mints the epoch when demand is due: `None` while clean, while a sweep
    /// is unfinished, or while throttled (which schedules `eligible_at`).
    pub fn begin_sweep(&mut self, now: Instant) -> Option<FrameEpoch>;
    /// Earliest eligible sweep instant; feeds an `OptionalDeadline`.
    /// Deliberately NOT named `deadline` (RESEARCH finding 12) — periodic
    /// cadence demand is the application's own `cadence_deadline` source,
    /// distinct from throttle eligibility, distinct from TE phase.
    pub fn eligible_at(&self) -> Option<Instant>;
    pub fn on_eligible(&mut self);
    /// Sweep outcome: `presented` clears demand for that epoch;
    /// `failed` retains demand and records the full-repaint obligation.
    pub fn sweep_presented(&mut self, epoch: FrameEpoch);
    pub fn sweep_failed(&mut self, epoch: FrameEpoch);
    pub fn full_repaint_required(&self) -> bool;
}
```

`Instant` is `tokio::time::Instant` on host builds and the platform monotonic instant on target builds, behind one type alias; K2R-0 tests drive it with paused time.

### 6.6 Touch protocol (host-modeled contract; hardware integration is K2R-1)

Normative protocol, from RESEARCH finding 9:

1. The ISR path does exactly one thing: bumps a wake-capable **generation counter** latch. It never performs I²C.
2. Task context reads one contiguous register snapshot per service, parses count/event-type/touch-ID/coordinates from that single snapshot, and re-services while the INT line remains asserted.
3. An I²C failure restores pending state — a generation is never consumed by a failed read.
4. The source declares its semantics as **latest-state-with-coalescing** (this profile's choice): consumers receive the newest complete report; intermediate reports may coalesce; *transitions* (down/up edges per touch ID) are reconstructed from report deltas and MUST NOT be silently dropped — an edge observed in a snapshot survives coalescing.
5. The reactor-facing type is a kernel-admitted source; its concrete carrier follows the K2R-0A outcome (pinned source or channel boundary). Reports use a fixed two-point capacity matching the FT3168.

## 7. K2R-0A: the completion-delivery feasibility gate

**The question:** the kernel's `ReactorSource` is sealed, `Unpin`, `&mut self`-polled; the HAL completion is a borrowing, generally `!Unpin` future whose drop cancels its listener. These are incompatible today. K2R-0A decides the mechanism, against exact pinned HAL SHAs recorded at spike start:

- **Outcome A — kernel pin admission:** the kernel adds a reviewed pinned-source path (`poll_next(self: Pin<&mut Self>, ...)`), the comparison root SPEC 37.6 explicitly reserved. Requires a root-spec amendment and kernel-side fixtures; this profile then stores `StripeInFlight` in a pinned slot.
- **Outcome B — named executor boundary:** completion is driven by an explicitly named Embassy task that resolves the future and delivers `Returned`/`Failed` through an admitted channel source. The task is a visible, spec'd component — never hidden inside a profile type.

Pass criteria for the spike (whichever outcome): completion wake-up demonstrably reaches the reactor (no lost wake across a losing arbitration); all resources are recovered on success, failure, and cancellation; no hidden task (outcome A) or exactly one named task (outcome B); zero allocation after init. Fail criteria: unsafe self-reference, double-drive of the completion, or wake loss under the adversarial schedules of §8. **The outcome is recorded in this spec and in the root graduation map before K2R-0 freezes its carrier types.**

## 8. K2R-0: required oracles

All host, deterministic, paused-time where time matters:

1. **Completion delivery** (per the K2R-0A outcome): completion resolves while another source wins the arbitration — the wake is not lost; cancellation mid-flight returns transport and buffer; dropped-permit and double-start are unwritable (compile-fail fixtures).
2. **Resource recovery:** every `Failed` path hands back transport and buffer; a model transport with injected failure at every command/chunk boundary never leaks either.
3. **Epoch/pixel equivalence:** a reference full-frame render and a stripe-swept render of the same epoch produce identical pixels, including across mid-sweep state changes (which must land in the next epoch) and after an injected failure (full repaint).
4. **Demand policy:** coalescing; one sweep in flight; `eligible_at` scheduling under paused time; `sweep_failed` retains demand and sets the full-repaint obligation.
5. **Touch interleavings:** adversarial generation-latch schedules — IRQ before registration, during read, after flag sample, INT still asserted after read, I²C failure mid-service — with the oracle that no down/up edge is lost and no torn report is ever surfaced.
6. **Negative controls**, published beside the oracles: raw writes to a transport outside the capabilities compile; nothing bounds a renderer's scene-replay cost (measured, not prevented); TE-phase tearing is not prevented by anything in this slice.

## 9. Board anchor obligations carried by this spec

Revision-keyed V1 facts this spec depends on and their status: `LCD_TE` GPIO13 and `TP_INT` GPIO21 (schematic-confirmed); TE measured behavior (**Gap** — K2R-1); `write_region` transport for SH8601 (**Gap** — upstream/fork decision gated before K2R-1 stripes; stock full-framebuffer path is the K2R-1 baseline); per-backend peak memory/bandwidth budgets including DMA reserves, descriptors, and stacks (**Gap** — published at K2R-1 with a zero-allocation-after-init requirement).

## 10. Error model

Concrete error types per transport integration; `Failed<T, B, E>` is the carrier, never a bare error — losing the resources is the failure mode this profile exists to prevent. No crate-wide error enum. Panic policy follows the kernel: no `catch_unwind`, drop order documented per type.

## 11. Slice acceptance

K2R-0A/K2R-0 are done when: the K2R-0A outcome is recorded here and in the root graduation map; all §8 oracles pass in CI on the workspace toolchains; compile-fail fixtures cover the unwritable states (double-start, dropped permit, buffer reuse while in flight); negative controls are published; clippy/fmt/doc gates are clean; and the crate builds `no_std` without alloc. Only then does K2R-1 (V1 board bring-up) graduate into this document with its measured gates — and the merge with the sibling harness workstream happens on top of these frozen protocols, not before.
