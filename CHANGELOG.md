# Changelog

## Unreleased

- Complete the kittens-code Q5 executor meters: window `ask-each` subcalls,
  enforce recursion depth and driver-reported ask time/token costs, and bound
  retained continuation payload memory with deterministic byte accounting.
- Connect the kittens-code RLM continuation executor to turn execution through
  the `recall` tool, with store-page and sub-model effects, query tracing,
  suspended-query admission, cancellation/late-terminal handling, and Tokio
  driver support over the real JSONL transcript and existing model client.
- Add the `kittens-code` KC0 headless composition root with a testable JSONL
  stdin/stdout protocol loop, deterministic jail scenarios by default,
  feature-gated live model bootstrap, and resumable session-log opening.
- Add `Engine::resume` replay construction for kittens-code KC0, restoring
  logged configuration and reconstructable conversation/window state while
  seeding turn, record, effect, and request counters above every persisted
  value without re-emitting replayed work.
- Harden kittens-code publication boundaries with checked monotonic allocation,
  exclusive per-log writer locks, exact window tool-call/result lifecycle
  validation, and branded capped tool-result tail entries.
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
