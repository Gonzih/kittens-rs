# Frontmatter: the three-layer agent decomposition

- Date: 2026-08-09
- Status: architecture orientation, not a spec. Names the layering and shows
  how the KC0 harness composes with `kittens-render` (the DISPLAY layer) to
  run an agent on a microcontroller. Feeds the kittens-code SPEC's driver
  and portability sections; nothing here is frozen.

## 1. The decomposition

An "agent," as the word is casually used, is three separable layers. Naming
them separately is the whole point — each can live on a different device,
and collapsing any two is a deployment choice, not an architectural law.

```
COGNITION      the model. Token generation. Lives wherever inference runs:
               a datacenter, a laptop, an on-device NPU. Reached over a wire.
                          ▲   window in           final text / tool calls out
                          │
ORCHESTRATION  the HARNESS. The turn loop, tool dispatch, context/RLM law,
               the append-only log, budgets, cancellation. This is
               kittens-code: a no_std+alloc sans-io core (kittens-code-core)
               driven by a per-target driver over the kittens reactor.
                          ▲   user input / touch          frames / events out
                          │
DISPLAY        the FRONTMATTER. What the human sees and touches. On a desktop
               that is kittens-tui (a terminal). On the Waveshare ESP32-S3
               1.8" AMOLED it is kittens-render (SH8601 panel, FT3168 touch,
               368×448) — the stripe/sweep/touch profile on the same kernel.
```

- **FRONTMATTER = DISPLAY.** The presentation-and-input surface. Coined here
  because it is the layer that sits *in front of* the harness for a human,
  and it can be anywhere: a terminal, a browser, or a physical panel wired to
  a microcontroller.
- **FRONTMATTER + HARNESS** collapsed onto one device = what people loosely
  call an **AGENT** when they mean "the thing running on my machine." The
  cognition still lives elsewhere.
- **FRONTMATTER + HARNESS + COGNITION** = the full agent. On a big model this
  spans a device and a datacenter; on a tiny local model it could all be one
  box.

The claim the kittens stack makes concrete: because DISPLAY (kittens-render /
kittens-tui) and ORCHESTRATION (kittens-code) are **both kittens profiles
sharing one reactor kernel and one no_std discipline**, FRONTMATTER + HARNESS
compose on a single microcontroller with no glue runtime — they are arms of
the same `kittens::reactor!` loop, not two systems bolted together.

## 2. Why this composes (and most agent stacks cannot)

kittens-render's own research states the thesis: *"a harness today is a
backend — heavy IO, model streams, tool execution — with an interface bolted
on through conventions... This profile does the same for physical displays on
bare metal: rendering and input pipelines become declared reactor topology in
the same vocabulary as the backend."*

Three facts make the composition real rather than aspirational:

1. **Same kernel, same law.** kittens-render is a domain *profile* of the
   kittens kernel, not a separate library. Its sweep/demand/touch protocols
   are admitted kittens sources and phases with the kernel's shared meaning
   for priority, dormancy, cancellation, and phase ordering (render SPEC §0.1:
   "Profiles MUST share the kernel's semantic rules"). Our harness driver is
   the same shape (kittens-code SPEC L-D1, L6).
2. **Both are no_std and dependency-lean.** kittens-render is a **zero-runtime-
   dependency** no_std crate. kittens-code-core + protocol are proven to link
   on `thumbv7em-none-eabi` and `wasm32-unknown-unknown` (gate G1, review
   input 19 #21). Neither drags in std, so both fit the same bare-metal image.
3. **Sans-IO core, effects at the edge.** kittens-code-core is a pure
   `handle(CoreInput) -> Transition` state machine with no IO, clock, or
   entropy. kittens-render, likewise, is proof-carrying state machines
   (`Sweep`, `FrameDemand`, `StripeTarget`) whose IO — the actual SH8601
   transfer, the FT3168 register read — is an admission obligation discharged
   by a board integration. The device-specific parts of *both* layers live in
   the same place: the embassy/MCU driver.

## 3. What each kittens-render primitive gives the harness

kittens-render (K2R-0 host slice today; board bring-up K2R-1 pending hardware)
exposes five modules, each a proof-carrying protocol:

| render module | what it is | harness use |
|---|---|---|
| `demand::FrameDemand` | one machine-active render epoch with throttle deadlines | the render-gate the harness arms when transcript state changes (analogous to kittens-tui's `Presenter`) |
| `sweep::Sweep` | one owned frame snapshot per epoch, one outstanding target per plan position, mandatory settlement, `SweepWritten` proof | drives a frame region-by-region; the harness hands it bytes, gets a written-proof back |
| `transfer::StripeTarget` / `OwnedTransfer` / `InFlight` | the only public flight construction; resources return on the driven path; cancel settles and wakes | the physical SH8601 write lane; back-pressure stalls only this, like the kittens-tui writer thread |
| `geometry::PanelGeometry` | admitted panel geometry (`WAVESHARE_18_V1`, 368×448); arbitrary geometry is a named escape | the harness renders against a validated panel, not a magic-number rect |
| `touch` | wake-dedup FT3168 touch without idle-check TOCTOU, bounded service per activation, untorn-snapshot reader contract | the input source: a touch becomes a `user_input`/interject Op, exactly as a keystroke does on the terminal frontend |

Milestones are `StripeWritten` / `SweepWritten` only — kittens-render proves
bytes reached the panel's write path, not that photons emitted. TE sync,
power/AOD, DMA overlap are named later gates. The harness treats a
`SweepWritten` like kittens-tui treats a frame acknowledgement.

## 4. How the harness runs on the ESP32-S3 (the target picture)

The composed image is one `kittens::reactor!` loop in a
`kittens-code-driver-embassy` crate (the MCU driver sibling named in
kittens-code SPEC §3, currently a candidate). Its source families are the
UNION of the harness's and kittens-render's — which compose because both were
derived from the same Grok-fixture reactor discipline:

```
kittens::reactor! {                       // in kittens-code-driver-embassy
  // --- shutdown / interrupt prefix (harness) ---
  #[shutdown] _ = power_or_fault  => ...
  interrupt   _ = cancel_signal   => ...   // KX2 embassy Signal adapter

  // --- DISPLAY: kittens-render arms (frontmatter) ---
  writer_ack  = stripe_done       => sweep.settle(...)   // SH8601 transfer done
  touch_evt   = ft3168_touch      => submit user_input Op // FT3168, via `touch`
  draw_deadline = frame_demand.deadline() ...            // render gate

  // --- ORCHESTRATION: harness arms ---
  model_delta = model_stream_rx   => drain, yields_to input   // KX1 embassy channel
  effect_done = tool_completions  => core.handle(EffectFinished)
  before_poll { arm frame_demand from transcript dirty-state }
  after_event { core.handle(...); if compaction due -> prefire }
}
```

Layer-by-layer on the board:

- **DISPLAY (on-device):** kittens-render, no_std, driving the SH8601 panel
  and reading FT3168 touch. The `StripeTarget`/`touch` IO obligations are
  discharged by the board integration (esp-hal SPI/QSPI + GPIO) in the embassy
  driver. This is what Codex is building toward in the render worktree.
- **ORCHESTRATION (on-device):** kittens-code-core + protocol + tools,
  no_std+alloc, proven to link on thumbv7em/wasm today. The transcript log is
  an append-only store over a **flash-backed `Store`** (littlefs/embedded
  codec — the postcard codec candidate, kittens-code SPEC S4/D7); the RLM
  query engine runs over it. Ports needed from the MCU driver: `Http`
  (reqwless + embedded-tls, ~32KB/conn), `Clock`+`Entropy` (embassy-time +
  board RNG), `Vfs` (littlefs2), no `Exec` (no shell on an MCU — that tool
  family is simply not advertised, SessionCapabilities K3).
- **COGNITION (off-device):** the model, reached over TLS via the `Http`
  port. On the ESP32-S3 the realistic path is a remote endpoint (the live
  Anthropic client's logic, minus reqwest, re-homed on reqwless). Local
  on-device inference is a separate, much later question.

The kernel adapters this needs — KX1 embassy channel, KX2 signal, KX3
deadline — are already ledgered in kittens-code SPEC (KX-K1) and are the
MCU-runtime gate; kittens-render supplies the DISPLAY arms; the harness
supplies the ORCHESTRATION arms. Nobody writes a second event loop.

## 5. What this orientation fixes for kittens-code

- The `kittens-code-driver-embassy` crate is not just "the harness on an MCU"
  — it is the **composition root for FRONTMATTER + HARNESS on one reactor**,
  linking kittens-render for DISPLAY and kittens-code-core for ORCHESTRATION.
- The DISPLAY seam (SPEC F3, previously "kittens-tui, negotiated") generalizes:
  the harness offers the same protocol Event stream to *any* frontmatter
  profile — kittens-tui on a terminal, kittens-render on a panel. A frontend
  is an Event consumer that produces input Ops; whether it draws with ANSI or
  drives SH8601 stripes is the profile's business. No privileged path either
  way (the kittens-tui lesson, input 07).
- The `Store`/`Vfs`/`Http` ports the harness already defines are exactly the
  device-specific seam; kittens-render's transfer/touch IO obligations are the
  same kind of seam on the DISPLAY side. Both are discharged once, in the
  embassy driver.

## 6. Open, honest

- kittens-render is at K2R-0 (host slice); real board bring-up (K2R-1) awaits
  hardware and the Xtensa/esp toolchain. The composition above is proven at
  the *type/link* level (both no_std, both kittens profiles) but not yet on
  silicon.
- kittens-code's MCU runtime is gated on the KX embassy adapters (unbuilt),
  the flash `Store` codec (D7), and the ESP32-S3 TLS spike (D-c). Today the
  proven claim is: the harness core *links* for the target; running it on the
  board is the next milestone after the terminal (std) slice closes.
- COGNITION on-device is out of scope. FRONTMATTER + HARNESS on the board with
  remote COGNITION is the near-term shape.
