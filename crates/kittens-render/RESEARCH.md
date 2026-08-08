# kittens-render research

- Date: 2026-08-08
- Status: research pass for the embedded rendering/interaction profile; no implementation authorized by this document
- Parent evidence: root [`RESEARCH.md`](../../RESEARCH.md) section 20 (embedded async UI, revision-keyed board facts, firmware anatomy, DMA/selection-loss contracts) and section 20B (coverage model); [`crates/kittens-tui/SPEC.md`](../kittens-tui/SPEC.md) section 10 (the open generic-gate comparison this profile supplies the second arm for)
- Labels: **Fact** / **Observation** / **Hypothesis** / **Recommendation**, as in the root research doc; unresolved questions are marked `**Gap: ...**`

## 1. Charter: the interface becomes a first-class citizen

**Observation:** a harness today is a backend — heavy IO, model streams, tool execution — with an interface bolted on through conventions. The Grok TUI research showed the interface is actually the hardest orchestration in the codebase: input isolation, frame gating, acknowledgement protocols, starvation topology. `kittens-tui` made that law explicit for terminals. This profile does the same for physical displays on bare metal: the rendering pipeline and the input pipeline become declared reactor topology in the same vocabulary as the backend, in one codebase that reads like a backend *and* is a rendering engine.

**Recommendation:** the profile's thesis, falsifiable: *one Kittens vocabulary can express a complete embedded interactive application — display refresh, touch interrupts, sensor IO, backend work — with the same declared-topology coverage the desktop harness gets, on a real board.* The unit of proof is a running app on the named dev board, not a diagram.

## 2. Exact hardware target, revision-keyed

The first dev board is the user's stated hardware: **Waveshare ESP32-S3 1.8inch AMOLED Touch Display, SH8601 display driver, FT3168 capacitive touch, ESP32-S3 LX7 dual-core, 368×448, no battery**.

**Fact (from root RESEARCH 20.1):** SH8601 + FT3168 identifies this as the **V1 revision** of the board. Waveshare discontinued V1 shipments in favor of V2 (CO5300 + CST820) on 2026-05-30; boards in hand and remaining retail stock are V1. Every claim in this profile is revision-keyed; V1 is the primary target because it is the hardware we own, and it is the *better-supported* revision in Rust:

| Layer | V1 status | Source |
|---|---|---|
| display driver | [`sh8601-rs` 0.1.8](https://docs.rs/sh8601-rs) supports the exact V1 display with esp-hal, PSRAM, QSPI, and DMA-capable writes; ships a `ws_18in_amoled` example for this very board | root RESEARCH 20.1 |
| touch | [`ft3x68-rs`](https://docs.rs/ft3x68-rs) — synchronous, `no_std`, FT3168-family, at most two points in a fixed-capacity vector; IRQ scheduling left to the application | root RESEARCH 20.1 |
| MCU HAL | `esp-hal` (bare-metal `no_std`): async GPIO (explicitly cancellation-unsafe — root 20.3), timers, QSPI, DMA with ownership-returning transfers, PSRAM | root RESEARCH 20.1, 20.3 |
| executor | Embassy on ESP32-S3 is proven by two nearby all-Rust watch firmwares; the Kittens kernel runs as an ordinary future on it (K0 architecture B) | root RESEARCH 20.2, 20.8 |

**Fact:** a full RGB565 frame at 368×448 is **329,728 bytes**. ESP32-S3 internal SRAM is ~512 KB (shared with everything); the board has 8 MB octal PSRAM. Waveshare's own C BSP renders in DMA-capable **16-row stripes** (368×16×2 = 11,776 B) rather than requiring a full framebuffer.

**Fact:** the inspected 1.8-board sources expose a touch IRQ line but **no tearing-effect (TE) input** (root RESEARCH 20.2 established this for V2 pin/BSP sources; the V1 schematic must be confirmed). Frame pacing therefore cannot gate on TE on this board; it gates on write-completion plus a cadence deadline.

**Gap: V1 TE availability and the exact V1 touch-IRQ GPIO number must be confirmed from the V1 schematic before the spec freezes pin-level claims (the V2-era docs mix revisions).**

## 3. What the display path actually is

**Fact:** `sh8601-rs` flush is **synchronous blocking** at the application boundary: when the call returns, the application-visible transfer is complete (controller scanout may continue). Its `partial_flush` allocates a temporary `Vec` proportional to the rectangle — the driver is `no_std` but not no-alloc (root RESEARCH 20.4).

**Fact:** esp-hal's owning SPI-DMA transfer API consumes the buffer and peripheral and returns them on completion — ordinary Rust ownership makes a second concurrent transfer or a mutation of the in-flight buffer a compile error. This is a HAL/Rust win the profile composes with, never claims (root RESEARCH 20.4; the embedded-shape K0 fixture already models exactly this ownership-returning completion).

**Observation:** the two completion models available on this board map precisely onto the two arms of the open generic-gate question (kittens-tui SPEC section 10):

| | kittens-tui (terminal) | kittens-render (this board) |
|---|---|---|
| submission | `Draw::commit` → writer thread | draw into owned buffer → flush/transfer |
| in-flight token | `FrameSeq`, acknowledged by writer event | the buffer + display *themselves*, returned by completion |
| gate reopens on | ack at-or-beyond sequence | ownership returning to the handler |
| misuse rejection | runtime stale-ack + exclusive `Draw` borrow | compile-time: you cannot submit what you do not hold |

**Hypothesis:** the ownership-returning form is the *stronger* gate (compile-time), and the profile should make it the canonical shape even in the v0 blocking path — by treating the display+buffer pair as a resource that the render step consumes and the completion event returns, so the blocking and DMA paths share one legal API. Whether one generic capacity-returning protocol should unify this with the TUI presenter remains the separately gated comparison; this profile deliberately builds the second concrete arm first.

## 4. Loop anatomy on this class of hardware (inspected, not imagined)

From the two nearby all-Rust watch firmwares (root RESEARCH 20.2):

- the workable shape is `derive cadence → arbitrate → update state → conditionally render → repeat`, with cadence spanning 30 s (off) to 16–33 ms (interactive/game) — the mode-derived absolute deadline is a first-class source, already proven in the K0 embedded fixture;
- touch and button interrupts must be **latched** sources: esp-hal GPIO waits lose edges when a losing waiter drops, so the raw wait is inadmissible — an owned latch armed from the interrupt path is the reviewed shape (K0's `Latched` + the admission diagnostic exist for exactly this);
- an unbounded await inside the loop (Wi-Fi join, TE wait) starves touch for seconds — handler-interior residual class; the profile's answer is topology plus deterministic latency oracles, not preemption claims;
- one firmware's "swap_and_flush" never swapped — descriptive comments rot; behavioral oracles or ownership are the only currencies accepted here.

## 5. What kittens-render is (candidate boundary)

The embedded rendering/interaction profile, layered per root SPEC 9.4:

1. **Sources (producers + adapters):** latched touch events decoded from FT3168 (IRQ → latch → typed `TouchEvent` with the fixed two-point capacity), mode-derived frame deadline, ownership-returning transfer completion, and pass-throughs for whatever backend sources the app declares. Nothing here changes kernel semantics.
2. **The render gate, ownership form:** a `Surface` (display handle + framebuffer/stripe buffers) that the draw step consumes and completion returns. One frame in flight is not a runtime counter — it is *possession*. Dirty/coalescing/cadence policy sits above it in a presenter-shaped protocol sharing kittens-tui's vocabulary (`request`, `try_begin`, deadline) so a harness developer reads both profiles as one system.
3. **Composition boundary:** `embedded-graphics` `DrawTarget` is the drawing contract; the profile hands the draw closure a target and never owns widgets, layout, or styling — same "opaque payload" stance as kittens-tui, with `DrawTarget` playing the role bytes play there. Component/widget libraries build above.
4. **Not owned:** the display driver (wraps `sh8601-rs`), the HAL, the executor, power management, and Slint-style frameworks (a later integration target, not a dependency).

**Recommendation — phased slices:**

- **K2R-0 (host):** the profile's types and protocols against the existing K0 embedded-shape fixture machinery — surface ownership gate, latched touch source with decoded events, cadence presenter — all host-tested, no hardware claim. This is where the spec's oracles live.
- **K2R-1 (board bring-up):** the same app compiled for ESP32-S3 with Embassy + `sh8601-rs` blocking flush + `ft3x68-rs` polling-on-IRQ-latch; stripe rendering within a predeclared SRAM budget; measure flush time and input-to-frame latency. First light on the user's V1 board.
- **K2R-2 (DMA slice, gated):** esp-hal owning-DMA transfers behind the same `Surface` API; gate: the API survives unchanged and the reactor overlaps drawing into a second stripe while one is in flight.

**Recommendation — memory policy:** stripe rendering by default (two 16-row stripes ≈ 23.5 KB SRAM, alternating), full PSRAM framebuffer as an explicit opt-in policy value, never a silent default (root RESEARCH 20.7: a declaration must not silently select PSRAM/alignment/DMA strategy).

## 6. Naming

`kittens-render` says what it is and parallels `kittens-tui`. Alternatives considered: `kittens-surface` (the gate type is a good name, the crate scope is wider), `kittens-display` (sounds like a driver), `kittens-embedded-ui` (claims widgets we refuse to own). **Recommendation:** keep `kittens-render`; name the ownership gate type `Surface`.

## 7. Falsifiers for this profile

- the ownership-returning `Surface` API cannot represent both blocking flush and owning-DMA without self-referential borrowing or API forks;
- stripe rendering through `DrawTarget` forces per-frame allocation the budget cannot absorb;
- the latched touch source loses taps at realistic IRQ rates (measured on hardware, not assumed);
- input-to-frame latency on K2R-1 exceeds what the same board does under the C BSP by a margin an interactive app cannot absorb;
- the shared presenter vocabulary with kittens-tui turns out to obscure rather than unify (agent trials, same method as the K0 pilot).

**Gap: no measurement exists yet for SH8601 blocking-flush duration per stripe/full frame on this board under `sh8601-rs` (no data — K2R-1 produces it).**

## 8. Source ledger (delta over root RESEARCH section 20)

All board, driver, HAL, Embassy, and firmware sources are pinned in root `RESEARCH.md` sections 20.1–20.5 and its section 21 ledger; this profile adds no new external sources yet. The V1 schematic confirmation (section 2 gap) is the next retrieval task.
