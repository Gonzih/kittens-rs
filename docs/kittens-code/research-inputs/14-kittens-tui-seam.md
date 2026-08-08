# kittens-tui seam analysis — D-b resolution input (2026-08-08)

Trigger: the kittens-tui harness published `crates/kittens-tui/SPEC.md`
(K1-TUI slice, controlling contract) and kittens-tui 0.1.1 to crates.io the
same day. This input reads that contract and resolves kittens-code's D-b
("kittens-tui seam blocked on negotiation") by conformance: kittens-code
consumes the published K1-TUI surface as-is; nothing needs negotiating away.

## What kittens-tui actually is (Fact, their SPEC)

Terminal orchestration, not rendering: `TerminalSession` (RAII raw-mode/alt-
screen), `InputReader` (owned reader thread → admitted `Mpsc<InputEvent,
close::Emit>` source), `FrameWriter` (owned writer thread, in-order
write+flush, ack-per-frame, drain-on-close, typed failure), `Presenter`
(render gate: coalescing, one-frame-in-flight, monotonic `FrameSeq`,
stale-ack rejection, throttle deadline via `OptionalDeadline`, exclusive
`Draw` permit). Frame payloads are opaque bytes; widgets/layout/diffing are
explicitly out of scope ("component-library territory"). Depends on kittens
(tokio feature) + tokio + crossterm — std-only, driver-side by construction.

## Composition model for kittens-code (Recommendation)

One driver reactor, not two processes. The kittens-code std driver's
`kittens::reactor!` hosts BOTH source families:

- harness sources (SPEC §6 L6): interrupt/shutdown prefix, model-delta funnel
  mpsc with drain + yields_to, tool-completion funnel, interjection arm,
  prefire one-shot, retry deadline;
- kittens-tui sources (their §6.7): `writer_events` above the render tier
  with `#[yields_to(terminal_input, when = buffered)]`, protected
  `terminal_input`, `draw_deadline` armed in `before_poll`, present in
  `initialize`/`after_event`.

The two canonical wirings are derived from the same pinned Grok fixture and
compose without conflict: shutdown prefix leads (both), writer acks outrank
the model firehose AND the render tier, terminal input is protected from
both firehoses, `after_event` carries both present-and-prefire work (phases
run ordinary app code; no contention). The kernel's 23-arm grok_shape test
already exercises this combined scale.

Layering (T3 preserved): the rendering layer — protocol Events → history
cells/markdown → frame bytes for `Draw::commit` — lives in the `kittens-code`
binary initially (a future component crate at most). It links
`kittens-code-protocol` + `kittens-tui` only, never `kittens-code-core`.
There is no privileged path: the renderer consumes the same Event stream any
ACP client gets, in-process via the channel pair.

## Concrete D-b resolution for SPEC F3/D-b

- F3 rewrite: seam = the protocol Event stream consumed in-process by a
  renderer layered on kittens-tui's Presenter/FrameWriter/InputReader inside
  the shared driver reactor; canonical wiring = union of kittens-code L6 and
  kittens-tui §6.7 (compatible by construction, same fixture ancestry).
- D-b status: resolved in shape by conformance to the published K1-TUI
  contract. Residual coordination item, explicitly small: kittens-tui's API
  names freeze "after the crate's first external consumer" (their §6) —
  kittens-code is positioned to be that first consumer, so KC1 (not KC0,
  which is headless) should pin kittens-tui and report any name friction
  back to that harness before their freeze.
- Non-goals confirmed both sides: kittens-code owns no rendering; kittens-tui
  owns no harness/protocol semantics; neither redefines kernel law.

## One watch item

kittens-tui is std/tokio/crossterm-bound (their §11) — correct for the std
target; MCU/WASM frontends (if ever) are separate profile crates on their
side or ours, and nothing in the D-b seam depends on kittens-tui becoming
portable. kittens-code's portability law (T1/T2) is untouched.
