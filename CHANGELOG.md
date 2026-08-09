# Changelog

## Unreleased

- Add `kittens-render` (unpublished K2R-0 host slice): the embedded
  rendering/interaction profile anchored on the Waveshare ESP32-S3 1.8"
  AMOLED V1 — spec-first with two external review rounds adopted in full;
  witness-driven transfer→sweep→demand composition (coverage cannot be
  claimed, only constructed from Completed settlements); provenance-branded
  demand settlement; generation-latched touch protocol; K2R-0A mechanism
  verdict (SPI2 TransferDone ISR completion in an Unpin carrier) with the
  compile-ready blueprint retained under probes/.

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
