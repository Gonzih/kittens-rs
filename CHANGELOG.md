# Changelog

## Unreleased

- Add the default-off `kittens-render/embedded-graphics` integration: a
  no-alloc, global-coordinate RGB565 target over an exact caller-owned stripe
  byte buffer, with full-panel layout bounds, clipping/translation, and the
  anchor driver's high-byte-first host encoding. Three independent full-frame
  versus real-witness-chain host oracles cover ordinary reconstruction,
  mid-sweep next-epoch scene changes, and post-failure full repaint. The
  feature-off core keeps an empty normal-dependency graph, feature-on remains
  `no_std`, and physical display color/format fidelity remains board-HIL.
- Add `kittens-render` (unpublished K2R-0 host slice): the embedded
  rendering/interaction profile anchored on the Waveshare ESP32-S3 1.8"
  AMOLED V1 — spec-first with five external exit-review rounds adopted in
  full; witness-driven transfer→sweep→demand composition (under sealed
  integrations and cooperative owning-sweep delivery, coverage cannot be
  caller-claimed and is constructed only from matching completed settlements;
  dropped/misapplied settlements and abandonment are published escapes);
  provenance-branded
  demand settlement; generation-latched touch protocol; K2R-0A mechanism
  verdict (SPI2 TransferDone ISR completion in a carrier that is `Unpin`
  exactly when its owned transfer and spare are both `Unpin`); the historical
  verdict and its explicitly non-compile-ready pseudocode delta are retained
  under probes/, while a real exact-HAL adapter remains behind the Xtensa gate.
- Complete the `kittens-render` exit-review Batch 5 evidence: add the
  runnable `host_sweep` lifecycle, trybuild proof-forgery failures with
  compiling escape-surface controls, and a separate downstream `no_std`
  thumbv7em link fixture; make demand-id exhaustion sticky across panic
  unwind and reconcile the conditional-`Unpin`, snapshot, Tick, stage, and
  trace-count contracts with the implementation.
- Complete the `kittens-render` exit-review Batch 6 host-core repair: make
  `StripeTarget::start_flight` the only public flight construction and return
  rejected starts through `StartFlightError`; make `Settled::into_parts`
  return exactly one move-only `StripeSettlement::{Written, Unwritten}` proof;
  enforce one outstanding target per sweep position and poison the epoch when
  its matching cancellation/failure settlement is delivered; document abort's
  stale-write window and the full-repaint plus
  `invalidate()` remedy; use checked, profile-independent epoch and
  `Tick::MAX` horizons; add explicit move-only witness controls; and publish
  the safe shared-backing sent-buffer/spare escape.
- Complete the `kittens-render` exit-review Batch 7 repair: replace the
  callback starter with seal-at-freeze `FlightStarter` and state target/start
  structure only for sealed integrations; reject sweep abort while a target is
  outstanding; carry idle invalidation into the next minted sweep; bound the
  flight-drop-plus-abandon escape with the reviewed adapter's synchronous
  cancel/disarm contract; and complete the constructor-privacy, move-only start
  error, and demand-rejection state evidence.
- Complete the `kittens-render` exit-review Batch 8 repair: require a
  crate-issued, private-constructor, non-`Clone`, lifetime-bound `StartPermit`
  for `FlightStarter::start`; pin direct invocation, the removed raw-closure
  path, permit construction/cloning, and both private `InFlight` construction
  paths with compile-fail controls; re-run all four demand-settlement rejection
  oracles after a successful-write throttle anchor and prove exact future
  eligibility plus successor epoch; and narrow owning-sweep delivery to its
  cooperative contract. Lost or wrong-owner-consumed settlements and
  abandonment recover by dropping old values, `abandon_active` forced repaint,
  and idle `invalidate` before replacement when stale work may overlap.

## 0.1.1 (kittens-tui) — 2026-08-08

- Add `kittens-tui`: the terminal-orchestration profile (K1-TUI slice) —
  spec-first crate with input isolation behind an owned reader thread and
  admitted channel source, a frame writer with per-frame acknowledgement and
  drain-on-close, the concrete presenter render gate (coalescing, one frame
  in flight, stale-ack rejection, throttle deadline, exclusive `Draw`
  permit), an RAII terminal session, the canonical wiring example, and the
  full oracle suite from `crates/kittens-tui/SPEC.md` section 9. Published
  to crates.io as `kittens-tui` 0.1.1.

## 0.1.1 - 2026-08-08

- Add a coordinated animated-sticker logo and yarn-play banner to the project
  README and the rendered `kittens` crates.io page.

## 0.1.0 - 2026-08-08

- Implement the K0 `no_std` reactor/source kernel, Tokio adapters, compile-time
  topology validation, retained expansion controls, mutation suite, Grok-shape
  fixture, embedded-shape fixture, and evidence report.
