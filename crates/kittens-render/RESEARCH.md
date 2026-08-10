# kittens-render research

- Date: 2026-08-08
- Revision 2, same day: incorporates the full 14-finding external review (Codex `gpt-5.6-sol`, ultra effort, read-only repository access). Five findings were blocking; one exposed a factual error in revision 1. Corrections are recorded explicitly per house rules — drift is a first-class defect, never silently patched.
- Revision 3, 2026-08-09: records the exact-source blocking-region audit that
  informed SPEC revision 10. The normative authorization and evidence gates
  live in the SPEC; this document remains research, not an implementation
  contract.
- Revision 4, 2026-08-09: records the pinned-HAL async-buffer/configuration
  audit that informed SPEC revision 11's additive single-payload adapter.
- Revision 5, 2026-08-09: reconciles the completed K2R-0A feasibility matrix
  with K2R-0 freeze and K2R-1 target ownership, and records the normalized-
  package/registry-HAL compatibility gap selected by SPEC revision 12.
- Revision 6, 2026-08-09: records the clean committed package-consumer result
  that closes revision 12's local compatibility row with packaged-source +
  registry-HAL Xtensa-link scope.
- Status: research record for the embedded rendering/interaction profile.
  K2R-0A is closed with host + portable-link + exact-Xtensa-link scope. SPEC
  revision 10's blocking row and revision 11's concrete async-region row are
  closed with their named host + target-link scopes. K2R-0 freeze remains
  gated on the bilateral seam and generic capability sealing. K2R-1 owns
  target execution, the board coordinator, silicon behavior, and measurements.
  Clean packaged-source registry-HAL compatibility is separately **CLOSED WITH
  PACKAGED-SOURCE + REGISTRY-HAL XTENSA-LINK SCOPE**; publication remains
  human-ordered.
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

**Superseded fact (finding 2, resolved by SPEC revisions 9 and 11):** DMA
completion could not then be a reactor source. `ReactorSource` was sealed and
`Unpin` with `&mut self` polling; `Latched` was locally armed only, with no
concurrent arming handle and no ISR wake. The HAL's completion future borrows
the owned transfer and is generally `!Unpin`; reconstructing it per poll would
drop its listener.

**Observation:** the selected resolution does not reconstruct that HAL future
or unseal `ReactorSource`. The kernel now admits the sealed, inline,
rearmable `OptionalInlineOneShot<F>` for `F: Future + Unpin`; render's
conditionally-`Unpin` owning `InFlight` retains one reviewed SPI2 completion
slot future across polls. The real-reactor host/portable row and the branded
adapter's host + exact-Xtensa-reactor-link row are closed with their named
scopes. This resolves the K2R-0A feasibility question. A real target executor,
its waker behavior, and silicon interrupt truth remain K2R-1 gates.

**Fact (finding 9):** the FT3168 path has the same shape: a one-bit latch cleared after a multi-transaction I²C read loses or fabricates input under IRQ interleavings. **Recommendation:** ISR-side wake-aware *generation* latch; task-side single contiguous register snapshot; parse count/event/ID from that snapshot; drain while INT is asserted; restore pending state on I²C failure; and the source declares itself as *latest-state-with-coalescing* or *lossless-transitions* — one latch cannot promise both. K2R-0A selected and tested the host generation/latch protocol; concrete TP_INT wake delivery and the contiguous physical transaction are K2R-1 integration work.

**Fact (finding 10):** shared-I²C arbitration, expander register shadowing, reset epochs, and serialized panel commands (brightness/AOD/sleep interleaved with CASET/PASET/RAMWR) need one owner. **Recommendation:** a board coordinator owns them, or Off/AOD semantics are removed from the initial profile. Initial profile: **removed**; the real coordinator is K2R-1 work with the SH8601 command surface as evidence.

## 6. Transport decision that gates everything

**Fact (finding 3):** "stock `sh8601-rs` + 16-row stripes" is not implementable — the streaming interface is private. The real options:

| Option | Cost | Verdict |
|---|---|---|
| stock `sh8601-rs`, full PSRAM framebuffer | 329,728 B PSRAM + bandwidth math of section 4; alloc in `partial_flush` | viable for first light (K2R-1 baseline); measured, not assumed |
| upstream/fork a `write_region(region, pixels)` transport | driver work + review; enables stripes and the owning-DMA path | historical gate resolved by SPEC revision 10's profile-owned transaction; upstream disposition remains separate |

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

**Fact (revision 4 pinned-HAL audit):** `DmaTxBuf` exposes logical-length reset
only from the beginning of its backing buffer. Its in-progress `BufView` has no
public safe offset/range constructor and no safe operation that later rejoins
separately borrowed descriptor/buffer slices into the original owned value.
Implementing an offset DMA view locally would require the unsafe
`DmaTxBuffer` trait, forbidden by this crate. Repeatedly shifting unsent bytes
to the front would mutate the resource contract and add quadratic copying.
The intended 368×16 RGB565 stripe is 11,776 bytes and fits beneath the existing
16,380-byte transaction constant, so arbitrary async multichunk support is not
required to test the selected stripe architecture.

**Fact (revision 4 configuration audit):** esp-hal's public `SpiDma` erases the
concrete SPI instance, while the reviewed completion slot reads and masks SPI2
registers directly. A safe public adapter constructor accepting arbitrary
`SpiDma` could therefore be handed SPI3 and would make the interrupt contract
dishonest. Exact `SPI2`, `DMA_CH0`, and GPIO singleton types remain available at
construction and can brand the profile-owned transport before erasure.

**Recommendation (adopted by SPEC revision 11):** add one board-branded
profile adapter under the existing experiment-open async traits. Share the
blocking engine's geometry and CASET/PASET encoding, preflight one logical
pixel buffer of at most 16,380 bytes, then start exactly one owning RAMWR DMA
transfer. Retain the command scratch inside the transfer so recovery rebuilds
the same branded transport. Defer async RAMWRC, overlap, and destructive-copy
semantics to a measured later slice; retain arbitrary external trait
implementation as the explicit non-sealed control. Because exact construction
otherwise owns SPI2 before the still-external panel initializer can run, expose
one visibly exceptional idle-command closure whose private-field borrowed
facade exposes command writes but cannot move, replace, or reconfigure the
underlying bus; do not mislabel those raw commands as part of the stripe proof
or as the K2R-1 coordinator.

**Fact (revision 5 package-source audit):** Cargo's multiple-locations form
uses the exact git HAL while developing in the repository but normalizes the
packaged target dependency to registry `esp-hal =1.1.0`. The historical
package evidence was produced from a dirty tree and verified only the host
package target. It did not compile an external Xtensa consumer against the
normalized source identity.

**Recommendation (adopted by SPEC revision 12):** retain the exact-git fixture
as the source-revision control and add an independent clean packaged-source
consumer. Extract the candidate archive outside the checkout, compose its
public constructor with direct registry singleton types, and run direct
packaged-library plus consumer target Clippy, optimized link, metadata-source,
undefined-symbol, allocator-symbol, and retained-hook gates. This is locally
executable package compatibility, not crates.io publication or target runtime.

**Fact (revision 6 clean package-consumer result):** from clean
implementation commit `c3e234770ce2de9a277e947f8cf8547700abea28`, locked
Cargo 1.96 packaging produced a 206,609-byte archive with SHA-256
`b0bc8d11e477ca4b5f6421bb49db3ada3b45ea1f555af4e5e412dd93dede4ec4`.
Its VCS record names that exact commit and `crates/kittens-render`, with no
dirty marker. The extracted package and standalone consumer resolve exactly
one registry `esp-hal` 1.1.0, checksum
`6af8fa8216bc126941bd43b5a200a50eab16e43881ccd0dd0b6792f4a82805f0`,
and zero git packages; the staged Xtensa configuration hashes to
`aa32449e2a38ae9ccac1a7b625a6dff109e3f70fc4c59becab5345b63f27e1e9`.
Direct packaged-library and consumer target Clippy pass. The locked optimized
ELF is 206,248 bytes with SHA-256
`5ce57e9e9875f900e1c89987d56dc8fa78a383a041235f175cde4686dd5bdf75`,
entry `0x403785e8`, and 56,680 bytes of `.bss`; undefined and allocator scans
are empty, while the exact parts and async-start hooks remain as nonzero text
symbols of sizes `0x20` and `0x16bd`.

**Observation (revision 6 scope and review):** this closes only **PACKAGED-
SOURCE + REGISTRY-HAL XTENSA-LINK SCOPE**. The async-start hook is retained but
uncalled, so publication, target execution, interrupt/cancel/drop behavior,
arbitrary-waker allocation, and silicon truth remain outside the evidence.
Claude Code 2.1.224 `claude-opus-4-8` at maximum effort reported **SOUND, zero
P0–P2 defects**; the retained report is
`reviews/2026-08-09-packaged-registry-implementation-precommit-claude.md`.
The already published 0.1.1 is immutable older source, and any future correctly
versioned release remains human-ordered.

## 7. What kittens-render is (boundary, post-review)

Revision-2 boundary, amended by revisions 3–4: sources (generation-latched touch
with decoded events, cadence deadline, TE edge where measurement justifies it,
completion delivery per the K2R-0A outcome); explicit blocking and async
transport capabilities; frame-demand policy above them sharing only `request`;
and `embedded-graphics` global-coordinate targets as the composition boundary.
The profile now owns the one minimal private SH8601 region transaction selected
by SPEC revision 10 and the board-branded single-payload async composition
selected by SPEC revision 11, superseding the earlier blanket exclusion of
display-driver internals. It still does not own a complete driver, observed
panel initialization, widgets/layout, HAL, executor, power/AOD, or the real
board coordinator (all target ownership is staged in K2R-1), or Slint.

## 8. Naming

`kittens-render` stands. The gate is no longer one `Surface`. Revision 3
supersedes the provisional blocking `Returned`/`Failed` nouns with the exact
SPEC surface: target-owned `write_region`, opaque `BlockingSettled`, and one
written/unwritten `StripeSettlement`; the async path retains its separate
`InFlight`/`Settled` vocabulary.

## 9. Slice plan and measured gates (replacing revision 1's plan per finding 11)

1. **K2R-0A — CLOSED with host + portable-link + exact-Xtensa-link scope:** the pinned-source/completion feasibility spike selected the inline carrier and profile-owned adapter shapes. Its target artifacts are unexecuted link evidence, not runtime/HIL.
2. **K2R-0 — host protocol evidence passed; freeze GATED on exactly two items:** co-sign the bilateral `kittens-code` seam and pass its foreign fixture; seal `FlightStarter`/`OwnedTransfer` at an authorized breaking API boundary. Publication is not a freeze prerequisite.
3. **K2R-1 — target runtime and V1 board baseline:** integrate a real executor and minimal board coordinator, TP_INT/FT3168 transaction, panel initialization, and the stock full-framebuffer comparison; record exact memory (static, stacks, heap high-water), allocation count after init, SPI2/touch wake and cancel/drop behavior, TE edge behavior by panel mode, flush latency, and touch latency *during* flush.
4. **K2R-2 — DMA overlap, conditionally:** only if it improves total frame time or p99 input latency under a fixed workload versus K2R-1, with failure injection at every command/chunk boundary.

The clean packaged-source registry-HAL link is an orthogonal release-readiness
row between repository development and any human-ordered publication. It is
**CLOSED WITH PACKAGED-SOURCE + REGISTRY-HAL XTENSA-LINK SCOPE**, but is not a
K2R acceptance condition and does not authorize publication. Board DMA overlap
is not specified until its gate has numbers.

## 10. Review log

External review, 2026-08-08: Codex `gpt-5.6-sol`, ultra reasoning effort, read-only repository access, 14 numbered findings (5 blocking, 8 important, 1 minor), full text retained in the session transcript. Disposition: findings 1–11 adopted as written above; finding 12 adopted (vocabulary split); finding 13's type signatures adopted as the leading candidate pending the K2R-0A prototype; finding 14 (worktree missing `AGENTS.md`/`kittens-tui` SPEC at the reviewed commit) resolved by rebasing this branch onto main once PR #2 merges, before any SPEC graduation. Historical verdict accepted: **not ready to graduate** under the then-current section-9 gates. Subsequent evidence and SPEC revision 12 supersede that prospective status; current acceptance is the explicit SPEC section-11 map.

**Gap: V1 TE measured behavior (edge/mode/safe-phase/tearing) — no data until K2R-1.**
**Gap: upstream disposition for the local `write_region` transaction — no
maintainer contact yet; this does not block the profile-owned revision-10
gate.**
**Gap: SH8601 blocking-flush duration per full frame under `sh8601-rs` on this board — no data until K2R-1.**
**Observation: clean packaged-source registry-HAL Xtensa composition — CLOSED
WITH PACKAGED-SOURCE + REGISTRY-HAL XTENSA-LINK SCOPE by the revision-12 local
gate. crates.io publication remains separate and human-ordered.**
