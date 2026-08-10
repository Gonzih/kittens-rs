# kittens-render research

- Date: 2026-08-08
- Revision 2, same day: incorporates the full 14-finding external review (Codex `gpt-5.6-sol`, ultra effort, read-only repository access). Five findings were blocking; one exposed a factual error in revision 1. Corrections are recorded explicitly per house rules — drift is a first-class defect, never silently patched.
- Revision 3, 2026-08-09: records the exact-source blocking-region audit that
  informed SPEC revision 10. The normative authorization and evidence gates
  live in the SPEC; this document remains research, not an implementation
  contract.
- Status: research record for the embedded rendering/interaction profile.
  SPEC revision 10 selects the blocking-region design, but its implementation
  evidence remains gated until the exact host and target matrix runs.
- Parent evidence: root [`RESEARCH.md`](../../RESEARCH.md) sections 20/20B; [`crates/kittens-tui/SPEC.md`](../kittens-tui/SPEC.md) section 10; [`crates/kittens/src/source/mod.rs`](../kittens/src/source/mod.rs) (the sealed kernel source contract, which section 5 shows is itself a constraint here)
- Labels: **Fact** / **Observation** / **Hypothesis** / **Recommendation**; unresolved questions are `**Gap: ...**`

## 1. Charter: the interface becomes a first-class citizen

**Observation:** a harness today is a backend — heavy IO, model streams, tool execution — with an interface bolted on through conventions. The Grok research showed the interface is the hardest orchestration in the codebase; `kittens-tui` made that law explicit for terminals. This profile does the same for physical displays on bare metal: rendering and input pipelines become declared reactor topology in the same vocabulary as the backend.

**Recommendation (unchanged):** the falsifiable thesis: *one Kittens vocabulary can express a complete embedded interactive application — display refresh, touch interrupts, sensor IO, backend work — with declared-topology coverage, on a real board.* The unit of proof is a running app on the named dev board.

## 2. Exact hardware target, revision-keyed and schematic-corrected

First dev board: **Waveshare ESP32-S3 1.8" AMOLED Touch, SH8601 display, FT3168 touch, 368×448** — the **V1 revision** (root RESEARCH 20.1; V2 shipped CO5300/CST820 from 2026-05-30). V1 is the revision in hand and the better-supported one in Rust.

**Fact (revision-1 error, corrected by review finding 4):** revision 1 claimed this board has no tearing-effect input, over-generalizing the root research's V2 pin/BSP note. The [V1 schematic](https://files.waveshare.com/wiki/ESP32-S3-Touch-AMOLED-1.8/ESP32-S3-Touch-AMOLED-1.8.pdf) routes **`LCD_TE` to GPIO13** and **`TP_INT` to GPIO21**, and driver initialization enables tearing-effect output. TE availability does not prove tear-free rendering, but an architecture derived from "TE unavailable" was invalid.

**Fact (review finding 4):** at 40 MHz quad-SPI, a full 368×448 RGB565 frame (329,728 B) has a theoretical wire floor of ~**16.5 ms** before commands, copies, or rendering — a full-frame write spans most of a 60 Hz period, so TE phase matters for tear behavior.

**Recommendation:** cadence eligibility (when a frame is *wanted*) and TE synchronization (when a write is *safe*) are distinct facts and stay distinct in any API. A measured TE experiment joins the gate list: edge behavior by panel mode, safe write phase, tearing outcome, behavior while asleep.

| Layer | V1 status | Notes after review |
|---|---|---|
| display driver | [`sh8601-rs` 0.1.8](https://docs.rs/sh8601-rs) — exact V1 display, esp-hal, QSPI | **but**: its `DrawTarget` writes a private full-screen framebuffer, `flush` sends the full window, `partial_flush` assumes that framebuffer and allocates; the streaming interface a stripe path needs is private (finding 3) |
| touch | [`ft3x68-rs`](https://docs.rs/ft3x68-rs) — sync, `no_std`, ≤2 points | multi-transaction I²C reads can tear; event/ID bits are discarded; IRQ handling explicitly left to the application (finding 9) |
| MCU HAL | `esp-hal`: owning DMA transfers, async GPIO (cancellation-unsafe), timers, PSRAM | `SpiDmaTransfer::wait_for_done(&mut self)` yields a borrowing, generally `!Unpin` future (finding 2) |
| board control | touch shares the I²C domain with a TCA9554 expander; reset writes whole expander registers | shared-bus arbitration and panel-command serialization need one owner (finding 10) |

## 3. The transport boundary: two capabilities, not one `Surface`

**Superseded (revision-1 hypothesis, rejected by findings 1 and 8):** revision 1 proposed a single ownership-returning `Surface` spanning blocking flush and owning-DMA. The review showed the two APIs do not share a boundary — a façade equating them would let "start" block to completion, making the completion ceremonial — and an indivisible surface *contradicts* two-buffer overlap: if the transfer owns everything, the renderer has no spare buffer to fill.

**Superseded recommendation (revision 2; blocking half replaced by revision 3
and SPEC revision 10):** two explicit capabilities with resource-carrying
results, and typestates for overlap:

```rust
pub trait BlockingRegionWrite<B>: Sized {
    type Error;
    fn write_region(self, region: Region, pixels: B)
        -> Result<Returned<Self, B>, Failed<Self, B, Self::Error>>;
}

pub trait OwningRegionWrite<B>: Sized {
    type Error;
    type Completion: Future<Output = Result<Returned<Self, B>, Failed<Self, B, Self::Error>>>; // may be !Unpin
    fn start_region(self, region: Region, pixels: B)
        -> Result<Self::Completion, Failed<Self, B, Self::Error>>;
}

pub struct PreparedStripe<T, B> { transport: T, ready: B, spare: B, frame: FrameEpoch, region: Region }
pub struct StripeInFlight<C, B> { completion: C /* pin before polling */, spare: B, frame: FrameEpoch }
```

`Returned` and every failure variant carry the transport and the sent buffer back; the in-flight state owns the completion and the *spare* buffer independently. Three distinct facts are emitted, never conflated: `StripeWritten`, `BusIdle`, `FramePresented`. Frame-demand policy (`request`/coalescing) is shared *above* both capabilities; their start/completion protocols are not claimed interchangeable.

**Recommendation (finding 12, vocabulary):** share only `request` with kittens-tui. Use `eligible_at` for presenter throttling, `cadence_deadline`/`next_frame_at` for periodic demand, `write_region`/`start_region` for transport. No shared trait until the separately authorized generic-gate comparison proves identical semantics.

## 4. Stripe rendering is a renderer contract, not a buffer trick

**Fact (findings 5–7):** two alternating stripe buffers are scratch, not spatial history — partial redraw over them repaints stale pixels; a state change mid-sweep produces a visibly mixed frame; `DrawTarget` clipping rejects pixels only after primitives are generated, so replaying a scene 28× can rasterize 28×; and the honest memory budget includes DMA RX/TX reserves (~32.8 KB in the example transport), descriptors, stacks, and driver state — not just 23.5 KB of stripes. A PSRAM full framebuffer costs ~19.8 MB/s of bandwidth at 30 fps (write+read) before blending or contention.

**Recommendation:** the stripe path requires: an immutable **`FrameEpoch`** snapshot frozen for the whole sweep; every transmitted stripe fully reconstructed from background plus the complete ordered scene; damage history invalidated to full repaint on any reset, partial-transfer failure, or epoch discontinuity; a **global-coordinate** stripe target; and either a bounded display list with spatial culling or measured scene-replay cost. Per-backend peak memory and bandwidth budgets are published separately, with a zero-allocation-after-init requirement.

## 5. The kernel is a constraint here, and that is a finding about the kernel

**Fact (finding 2):** DMA completion cannot currently be a reactor source. `ReactorSource` is sealed and `Unpin` with `&mut self` polling; `Latched` is locally armed only, with no concurrent arming handle and no ISR wake. The HAL's completion future borrows the owned transfer and is generally `!Unpin`. Reconstructing that future per poll is invalid (dropping it removes the completion listener).

**Recommendation:** this graduates from profile problem to **kernel feasibility gate (K2R-0A)**: either the kernel admits pinned no-std sources (`poll_next(self: Pin<&mut Self>, ...)` — the pin-boundary comparison root SPEC 37.6 explicitly reserved), or the profile explicitly admits a named Embassy task/channel boundary for completion delivery. Neither is hidden behind an abstraction; the K0 report's provisional pin/`Unpin` row anticipated exactly this pressure.

**Fact (finding 9):** the FT3168 path has the same shape: a one-bit latch cleared after a multi-transaction I²C read loses or fabricates input under IRQ interleavings. **Recommendation:** ISR-side wake-aware *generation* latch; task-side single contiguous register snapshot; parse count/event/ID from that snapshot; drain while INT is asserted; restore pending state on I²C failure; and the source declares itself as *latest-state-with-coalescing* or *lossless-transitions* — one latch cannot promise both.

**Fact (finding 10):** shared-I²C arbitration, expander register shadowing, reset epochs, and serialized panel commands (brightness/AOD/sleep interleaved with CASET/PASET/RAMWR) need one owner. **Recommendation:** a board coordinator owns them, or Off/AOD semantics are removed from the initial profile. Initial profile: **removed**; the coordinator is its own later slice with the SH8601 command surface as evidence.

## 6. Transport decision that gates everything

**Fact (finding 3):** "stock `sh8601-rs` + 16-row stripes" is not implementable — the streaming interface is private. The real options:

| Option | Cost | Verdict |
|---|---|---|
| stock `sh8601-rs`, full PSRAM framebuffer | 329,728 B PSRAM + bandwidth math of section 4; alloc in `partial_flush` | viable for first light (K2R-1 baseline); measured, not assumed |
| upstream/fork a `write_region(region, pixels)` transport | driver work + review; enables stripes and the owning-DMA path | required for the stripe/DMA architecture; **gate before the SPEC freezes** |

**Observation (revision 3 exact-source audit):** `sh8601-rs` 0.1.8 resolves to
commit `4bcddfd529017135f19a5a9a6e79dd6b8ef1b460`. Its `set_window` rejects the
valid first-row endpoint `y_end == 0`, does not bound both inclusive ends to
the panel, and its public `partial_flush` allocates and copies through the
driver's private framebuffer. The stock type exposes neither the private
interface nor an ownership-returning handoff, so it cannot accept Kittens'
external stripe buffer or compose with the pinned HAL transport without a
fork.

**Recommendation (adopted by SPEC revision 10):** keep stock full-framebuffer
bring-up only as a later first-light baseline. For the architectural region
gate, own the minimal CASET/PASET/RAMWR/RAMWRC transaction in this profile,
derived from the exact audited commit, and compile its one sealed production
adapter against `esp-hal` revision
`d48f747ba28accdc51779ba193eba923138e0382`. Keep the wire seam private; a
public generic interface could do nothing and report success, so sealing a
wrapper around it would merely move the integration-honesty escape. The stock
driver is protocol provenance, not a compiled dependency.

## 7. What kittens-render is (boundary, post-review)

Revision-2 boundary, amended by revision 3: sources (generation-latched touch
with decoded events, cadence deadline, TE edge where measurement justifies it,
completion delivery per the K2R-0A outcome); explicit blocking and async
transport capabilities; frame-demand policy above them sharing only `request`;
and `embedded-graphics` global-coordinate targets as the composition boundary.
The profile now owns the one minimal private SH8601 region transaction selected
by SPEC revision 10, superseding the earlier blanket exclusion of display-
driver internals. It still does not own a complete driver, panel initialization,
widgets/layout, HAL, executor, power/AOD (deferred to the board coordinator
slice), or Slint.

## 8. Naming

`kittens-render` stands. The gate is no longer one `Surface`. Revision 3
supersedes the provisional blocking `Returned`/`Failed` nouns with the exact
SPEC surface: target-owned `write_region`, opaque `BlockingSettled`, and one
written/unwritten `StripeSettlement`; the async path retains its separate
`InFlight`/`Settled` vocabulary.

## 9. Slice plan and measured gates (replacing revision 1's plan per finding 11)

1. **K2R-0A — pinned-source/completion feasibility spike** against exact pinned HAL SHAs: prove completion wake-up and full resource recovery with no hidden task or allocation; outcome decides kernel pin-admission versus Embassy-boundary delivery.
2. **K2R-0 — adversarial host protocol suite:** lost-wake interleavings, busy requests, dropped permits, failure/cancellation recovery carrying resources, absolute deadlines, and full-frame versus stripe pixel-equivalence oracles (FrameEpoch reconstruction correctness).
3. **K2R-1 — V1 board baseline:** stock full-framebuffer path; record exact memory (static, stacks, heap high-water), allocation count after init, TE edge behavior by panel mode, flush latency, and touch latency *during* flush.
4. **K2R-2 — DMA overlap, conditionally:** only if it improves total frame time or p99 input latency under a fixed workload versus K2R-1, with failure injection at every command/chunk boundary.

Per the review verdict: only the K2R-0A/K2R-0 contract graduates into the first SPEC; board DMA overlap is not specified until its gate has numbers.

## 10. Review log

External review, 2026-08-08: Codex `gpt-5.6-sol`, ultra reasoning effort, read-only repository access, 14 numbered findings (5 blocking, 8 important, 1 minor), full text retained in the session transcript. Disposition: findings 1–11 adopted as written above; finding 12 adopted (vocabulary split); finding 13's type signatures adopted as the leading candidate pending the K2R-0A prototype; finding 14 (worktree missing `AGENTS.md`/`kittens-tui` SPEC at the reviewed commit) resolved by rebasing this branch onto main once PR #2 merges, before any SPEC graduation. Verdict accepted: **not ready to graduate**; the section 9 gates control.

**Gap: V1 TE measured behavior (edge/mode/safe-phase/tearing) — no data until K2R-1.**
**Gap: upstream disposition for the local `write_region` transaction — no
maintainer contact yet; this does not block the profile-owned revision-10
gate.**
**Gap: SH8601 blocking-flush duration per full frame under `sh8601-rs` on this board — no data until K2R-1.**
