# Changelog

## Unreleased

- Select the additive, profile-owned Waveshare V1 async-region adapter contract
  before implementation: exact SPI2/DMA_CH0/GPIO branding, recoverable
  pre-admission parts, a non-swappable idle-command facade, shared CASET/PASET
  truth, and one owning RAMWR payload capped at 16,380 bytes. The required host
  failure/lifecycle matrix and generated-reactor/drop-glue Xtensa symbols remain
  OPEN; generic trait sealing, async RAMWRC, target execution, arbitrary-waker
  allocation behavior, publication, and every physical-panel claim remain
  separate gates.
- Add the sealed, profile-owned blocking SH8601 region path as the sole
  `StripeTarget::write_region` operation. The exact host matrix proves all
  eight CASET/PASET/RAMWR/RAMWRC calls, all eight injected failure boundaries,
  ten ordered preflight rejections, resource identity, conservative sweep
  poisoning, and eight compile-fail plus one explicit-drop compile-pass
  controls. The exact `esp-hal` revision links a real multichunk SPI2/GDMA
  invocation without undefined or allocator symbols, closing this row with
  host + exact-Xtensa-link scope only; published-registry Xtensa consumption,
  async capability sealing, target-side reactor execution, the bilateral seam,
  and every physical-panel/HIL claim remain open.
- Admit the portable, allocation-free `OptionalInlineOneShot` source for
  `Future + Unpin` and drive render `InFlight` completion through a real
  generated reactor. Deterministic tests cover both selection-loss positions,
  same-carrier stripe rearm, graceful drain/settlement, and post-exit disarm;
  the external consumer executes the lifecycle on host and links it for Thumb
  and wasm. The carrier proves retained storage, not arbitrary-future honesty,
  owning-sweep delivery, mutable-handler replacement safety, target execution,
  or silicon behavior; the three formal K0 gates remain open.
- Make `kittens-code` the public website's flagship demo with its complete
  four-crate map, JSONL-to-effect turn flow, transcript/context/RLM/budget
  laws, DISPLAY/ORCHESTRATION/COGNITION layering, deterministic offline quick
  start, published/open evidence ledger, and deep links to the `kc0` source,
  frozen contract, research, architecture note, release record, and inputs.
  The adjacent status boundary states that source is not yet on `main` and
  that full reactor wiring, E1 evaluation, MCU drivers, and stable APIs remain
  open rather than allowing flagship placement to overstate the evidence.
- Promote the standalone ESP32-S3 render probe into CI as a real linked-ELF
  gate: select the exact Xtensa Rust compiler and expected GCC mapping, cache
  the installed toolchain and Cargo downloads (not the target directory), run
  the locked release build, and inspect the output with both `file` and Xtensa
  `readelf`. Repair stale post-revision-8 evidence wording while retaining board
  HIL, silicon wake, kernel admission, and physical display behavior as open
  gates.
- Launch the public Kittens website on GitHub Pages: an evidence-led,
  accessible, dependency-free introduction to the coverage thesis, shipped
  kernel/TUI/render surfaces, lean reactor grammar, honest non-guarantees, and
  longer-term agent/engine/meta-harness vision. The site reuses the canonical
  cat-and-yarn artwork, adds a bespoke social card, records its research and
  controlling W0 contract in-repo, and publishes reproducible output from a
  provenance-marked `gh-pages` branch without analytics or cookies.
- Publish `kittens-render` 0.1.1 to crates.io (2026-08-09) as an
  experimental K2R-0 evidence release, human-ordered and gated on the
  round-7 publication-readiness review: all five blockers fixed, workspace
  coverage gate green (3754/3754 lines, 381/381 functions under the
  declared exclusion contract), dry-run verified, then published from
  main at 21a0b98. Open gates carried honestly in the published docs:
  board HIL, silicon wake delivery, seam co-sign, `write_region`
  transport, kernel-admitted `reactor!` fixture, and capability sealing
  at the 0.2.0 boundary.
- Add the default-off `kittens-render/embedded-graphics` integration: a
  no-alloc, global-coordinate RGB565 target over an exact caller-owned stripe
  byte buffer, with full-panel layout bounds, clipping/translation, and the
  anchor driver's high-byte-first host encoding. Three independent full-frame
  versus real-witness-chain host oracles cover ordinary reconstruction,
  mid-sweep next-epoch scene changes, and post-failure full repaint. The
  feature-off core keeps an empty normal-dependency graph, feature-on remains
  `no_std`, and physical display color/format fidelity remains board-HIL.
- Prepare `kittens-render` as an experimental 0.1.x K2R-0 evidence release:
  the embedded rendering/interaction profile anchored on the Waveshare
  ESP32-S3 1.8" AMOLED V1 — spec-first with seven external exit-review rounds
  adopted in full; witness-driven transfer→sweep→demand composition (under sealed
  integrations and cooperative owning-sweep delivery, coverage cannot be
  caller-claimed and is constructed only from matching completed settlements;
  dropped/misapplied settlements and abandonment are published escapes);
  provenance-branded
  demand settlement; generation-latched touch protocol; K2R-0A mechanism
  verdict (SPI2 TransferDone ISR completion in a carrier that is `Unpin`
  exactly when its owned transfer and spare are both `Unpin`); the historical
  verdict and its explicitly non-compile-ready pseudocode delta are retained
  under `probes/` as a superseded historical record. The real pinned-SHA
  Xtensa firmware fixture closed compile/link feasibility with scope. At this
  initial slice, board HIL and silicon delivery, kernel admission, the
  bilateral seam, `write_region`, and pre-freeze capability sealing remained
  open; the later blocking-row closure is recorded above. Publication of the
  0.1.x evidence release is not the capability freeze.
- Make the workspace coverage gate honest and durable: deterministic TUI
  oracles no longer construct and discard live crossterm bindings merely to
  mark lines covered; the two process-terminal binding files are explicit,
  documented exclusions, while CI enforces 100% lines and functions for every
  included source file and treats compiler-synthesized regions as
  informational.
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
