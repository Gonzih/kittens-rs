# kittens-code Feasibility Layer — Research Report (2026-08-08)

Lens: crates.io metadata + web sources; versions pinned from crates.io API 2026-08-08. F=Fact, O=Observation, H=Hypothesis.

## 1. no_std building blocks

**HTTP/TLS**
- F: `reqwless` 0.14.0 (2026-01-12) — async no_std HTTP client, alloc-free, built on `embedded-io-async` + `embedded-nal-async`. TLS backends: `embedded-tls` (TLS 1.3 only) or `esp-mbedtls` (1.2+1.3, git-dep only, ESP silicon). Mutually exclusive.
- F: `embedded-tls` 0.19.0 (2026-06-01) — pure-Rust TLS 1.3, no_std. **Cert verification (webpki CertVerifier) is std-only**; no_std mode = PSK or unverified certs. Needs ~16KB record buffer per connection.
- F: `edge-net`/`edge-http` 0.14.0/0.8.0 (2026-06-25, ivmarkov) — alternative no_std async HTTP; smaller adoption (~20K vs reqwless 123K downloads).
- F: `smoltcp` 0.13.1 — no_std TCP/IP stack; `embassy-net` 0.9.1 wraps it async.
- O: reqwless has chunked-transfer body readers (needed for SSE); SSE parsing is trivial line-parsing, doable in core.

**JSON**
- F: serde + serde_json work no_std+alloc (default-features=false, features=["alloc"]); serde_json 1.0.151. Sane choice — LLM payloads unbounded, heapless won't fit.
- F: `serde-json-core` 0.6.0 (stale since 2024-08) — zero-alloc, bounded schemas; wrong for LLM responses except tiny control frames.
- H: streaming JSON of unbounded LLM responses needs incremental parser or SSE-delta accumulation; serde_json wants full slices. Accumulate SSE deltas into rope/string — sidesteps this.

**Async**
- F: Embassy mature: `embassy-executor` 0.10.0 (2026-03-20), 2.6M downloads. Futures against `embedded-io-async` run on Embassy, tokio (adapters), or WASM unchanged.

**Text/rope**
- F: `crop` 0.4.3 — rope, `std` default-off-able → no_std+alloc (edition 2024, rust 1.85).
- F: `ropey` 1.6.1 — std-only. Use crop or String + line index for v1.
- F: `heapless` 0.9.3 (118M downloads) bounded buffers; `postcard` 1.1.3 compact transcript serialization.

## 2. WASM targets

- F: `wasm32-unknown-unknown` — no syscalls; all IO via host imports (wasm-bindgen/JS). Cloudflare Workers: `worker` crate 0.8.5 (2026-06-12), 3.8M downloads, wasm-bindgen based.
- F: `wasm32-wasip2` — Tier 2 since Rust 1.82 (2024-11). WASI 0.2: wasi-filesystem, wasi-sockets, wasi-http as typed component-model interfaces; std::fs/std::net mostly work under wasmtime.
- F: WASI 0.3 (native async in components) spec landed early 2026; `wasm32-wasip3` Tier-2 promotion in flight (rust-lang/compiler-team#1001). Watch — aligns with async core.
- O: no_std+alloc core is target-agnostic: compiles to both wasm targets identically. Shim decides: wasm-bindgen fetch for Workers/browser, wasi-http for wasmtime/Spin.
- F: Wasmtime deny-by-default capabilities (dirs/hosts preopened explicitly) — free sandboxing for tool execution.

## 3. Virtual filesystem prior art

- F: `vfs` crate 0.13.0 — std-only trait design (std::io, Box<dyn>). Good API-shape reference, unusable in core.
- F: `cap-std` 4.0.2 (Bytecode Alliance) — capability-based std FS; std-only; underlies WASI hosts. Belongs in std shim.
- F: `embedded-io` 0.7.1 / `embedded-io-async` 0.7.0 — no_std IO trait layer. 98M/5M downloads. `embedded-io-adapters` 0.7.0: bidirectional adapters for std::io, tokio, futures. Maintained by embedded-wg. **Verdict: mature, adopted, correct choice for byte-stream IO.**
- O: embedded-io covers *streams*, not filesystems — no standard no_std VFS trait (littlefs2 0.8.1, embedded-sdmmc 0.9.0 bespoke). kittens-code must define its own small `Vfs` trait (open/read/write/list/stat, &str paths, files as embedded-io-async Read/Write/Seek) + impls: in-memory (core), std/cap-std, WASI, littlefs2. Build-it gap, not pick-it gap.

## 4. Prior art: agents on MCUs / WASM

- F: Microsoft **wassette** (2025-08-06): Rust, wasmtime-based runtime exposing Wasm components as MCP tools; deny-by-default permissions; OCI distribution. Closest precedent for "agent tools in wasm sandbox" — but sandboxes *tools*, host harness native.
- F: **Lunatic** dead-ish (0.14.1, 2023-08). Spin/Fermyon alive. Workers AI + `worker` crate = production Rust-in-wasm calling LLM APIs today.
- O: ESP32 LLM clients mostly C/ESP-IDF (OpenAI realtime embedded SDK, ESP32-S3). Rust-on-ESP32 pieces exist (reqwless + esp-mbedtls examples) but no publicized full *agent loop* on MCU. **Gap: no direct precedent for full LLM agent harness on bare-metal Rust — kittens-code would be first.**
- O: Reported embedded LLM client constraints: TLS handshake+record memory (~16–40KB/conn), long streaming responses vs watchdogs, token buffers forcing PSRAM on ESP32 (transcripts unbounded — need spillable store; RLM-over-abstract-store is right), cert chains rotate (unverified TLS or pinned roots is the no_std reality).

## 5. Architecture split — precedent

- F: sans-io established: quinn-proto (pure QUIC state machine), str0m, boringtun, rustls Connection (buffer-in/out), smoltcp; Firezone sans-IO essay canonical. hyper splits similarly.
- O: Two composable patterns, use both:
  1. **Sans-io core** for agent state machine: `AgentCore::handle_event(Event) -> Vec<Effect>` — no IO, no time, no async; deterministic, replayable, trivially testable. Transcript model + RLM query engine here over abstract `Store` trait.
  2. **Trait-parameterized ports** (hexagonal) for byte world: embedded-io-async Read/Write for streams, custom `Vfs` + `HttpClient` traits.
- H: Crate layout: `kittens-code-core` (no_std+alloc, sans-io, zero platform deps) / `kittens-code-http` (client generic over embedded-io-async) / shims: `-std` (tokio+rustls+cap-std), `-wasm` (fetch / wasi-http), `-embassy` (embassy-net+embedded-tls+littlefs2).

## Recommended stack

core: no_std+alloc, sans-io event/effect state machine; serde+serde_json(alloc); crop(no-default) or String+index; heapless bounded internals; postcard persistence; own ~6-method Vfs trait; embedded-io-async as stream vocabulary.
shims: std=tokio+reqwest/hyper+rustls+cap-std · wasm-workers=worker 0.8.5 fetch · wasm-wasi=wasip2/wasi-http (watch wasip3) · mcu=embassy 0.10+embassy-net+reqwless 0.14+embedded-tls 0.19 (esp-mbedtls on ESP32).

## Risk list — cannot live in the no_std core

1. **TLS cert verification** — webpki verifier std-only; no_std = PSK/pinned/unverified. MCU mitigation: hw TLS offload, esp-mbedtls, or local TLS-terminating proxy. Highest genuine risk.
2. **Wall-clock time & entropy** — injected ports (TLS needs both).
3. **DNS + sockets** — shim-side always.
4. **Memory ceiling** — TLS 16KB + streaming JSON + transcript: budget ~64–128KB RAM min; ESP32-without-PSRAM marginal, RP2040 workable only with spill-to-flash store.
5. **wasm32-unknown-unknown has no ambient anything** — core must never assume even a clock; enforce with CI compiling core for `thumbv7em-none-eabi` + `wasm32-unknown-unknown`.
6. **No direct precedent** for full MCU agent harness — de-risk with early ESP32-S3 (PSRAM) spike hitting real LLM endpoint through reqwless.

Sources: reqwless/embedded-tls (drogue-iot), Wassette announcement (opensource.microsoft.com 2025-08-06), wasip2 Tier 2 (blog.rust-lang.org 2024-11-26), wasip3 (rust-lang/compiler-team#1001), Firezone sans-IO essay, quinn-proto, crates.io API (version pins 2026-08-08).
