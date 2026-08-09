# Changelog

## Unreleased

## kittens-code family 0.0.1 — 2026-08-09

First crates.io release of the `kittens-code` KC0 coding-agent harness,
published in strict dependency order at an independent per-crate `0.0.1`:
`kittens-code-protocol` → `kittens-code-core` → `kittens-code-driver-tokio`
→ `kittens-code-cli`. `0.0.1` is a deliberate experimental / expect-churn
signal matching the SPEC's evidence-release framing; the SPEC is FROZEN and
authorizes publication. Release-review correctness blockers were closed and
independently re-verified before publish: the appender torn-tail durability
fix (physical truncate + `sync_all` to the last valid record boundary before
any repair/append, so a second reopen never faults) and the lifecycle-ledger
ownership guard (`commit_stream_terminal` fires only for an owned,
current-epoch, not-yet-finished effect id; every other completion routes to
`commit_dropped_completion`, so no orphan `StreamTerminal` is ever persisted).
Cross-target link gate green on `thumbv7em-none-eabi` and
`wasm32-unknown-unknown`; workspace tests, `clippy -D warnings`, `fmt`, and
the `rustdoc -D warnings` publish gate all pass. This release contains the
items previously listed under Unreleased:

- Enforce kittens-code RLM output and aggregate-budget boundaries with
  branded ask digests, per-line/final verb caps, compiled meter ceilings,
  durable `BudgetUpdate` events, and harden Tokio write/edit tools with
  same-directory atomic replacement plus repeated symlink containment checks.
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
- Drive `kittens-code-driver-tokio` module coverage to ~97% (appender, model,
  runner, tools, and the feature-gated live client) via a deterministic
  `cfg(test)` fault seam; remaining uncovered code is defensive or
  external-only (the live HTTP socket = G10 external smoke).

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
