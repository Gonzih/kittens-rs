# Changelog

## Unreleased

- Add `Engine::resume` replay construction for kittens-code KC0, restoring
  logged configuration and seeding turn, record, effect, and request counters
  above every persisted value without re-emitting replayed work.
- Add the feature-gated Tokio `LiveClient` for Anthropic Messages SSE,
  including typed window lowering, streamed text/tool-use collection,
  provider usage, bounded jittered retries with `Retry-After`, and a
  consecutive-failure circuit breaker. The default driver build remains
  free of the optional HTTP/TLS dependency tree.

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
