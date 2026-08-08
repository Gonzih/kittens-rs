# Changelog

## Unreleased

- Add `kittens-tui`: the terminal-orchestration profile (K1-TUI slice) —
  spec-first crate with input isolation behind an owned reader thread and
  admitted channel source, a frame writer with per-frame acknowledgement and
  drain-on-close, the concrete presenter render gate (coalescing, one frame
  in flight, stale-ack rejection, throttle deadline, exclusive `Draw`
  permit), an RAII terminal session, the canonical wiring example, and the
  full oracle suite from `crates/kittens-tui/SPEC.md` section 9.

## 0.1.1 - 2026-08-08

- Add a coordinated animated-sticker logo and yarn-play banner to the project
  README and the rendered `kittens` crates.io page.

## 0.1.0 - 2026-08-08

- Implement the K0 `no_std` reactor/source kernel, Tokio adapters, compile-time
  topology validation, retained expansion controls, mutation suite, Grok-shape
  fixture, embedded-shape fixture, and evidence report.
