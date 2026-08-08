# Kernel-fit analysis: kittens-code needs vs the implemented K0 surface

Date: 2026-08-08. Method: direct read of `crates/kittens/src/lib.rs`,
`src/source/mod.rs`, `src/source/tokio_impl.rs`, and the `grok_shape.rs` /
`embedded_shape.rs` integration tests at workspace HEAD (post-`4200d2e`).
Companion to the SPEC's L6/F-a and the operator's 2026-08-08 directive:
prefer existing libraries; extend/expand kittens itself where the harness
needs it — code is cheap; targets are MCU / WASM / bare-metal.

## What K0 provides today (Fact, inspected)

- Sealed `ReactorSource` (poll_next, `Unpin`, readiness marker), with
  capability traits `DrainableSource` (allocation-free `try_next`) and
  `BacklogSource` (backlog probe for `yields_to`). Admission is sealed —
  new adapters mean extending the kittens crate, which is the intended path
  (reviewed adapters only).
- Local, allocation-free, no_std sources: `Latched<T>` (single-slot,
  quiescent, backlog-capable) and `FixedQueue<T, N>` (ring, may-remain-ready,
  drainable, backlog-capable). Both are same-task only: "no concurrent arming
  handle" — they never self-wake; arming from another execution context is
  explicitly out of contract.
- Tokio adapters (target-gated off `target_os = "none"`): `Mpsc`/
  `OptionalMpsc` (close policies `Dormant`/`Emit`), `Cancellation`
  (CancellationToken), `OneShot`/`OptionalOneShot` (retained futures),
  `OptionalDeadline` (absolute optional instant), `AlreadyArmedReceiver`.
- The 23-arm `grok_shape.rs` fixture proves the macro handles a Grok-scale
  arm count with mixed Latched/FixedQueue/OptionalOneShot/OptionalMpsc
  sources; `embedded_shape.rs` + the no-std fixture prove bare-metal linking.

## Mapping the kittens-code loop onto K0 (per SPEC §6 L6)

| Harness source | K0 story today | Verdict |
|---|---|---|
| User interrupt / shutdown | `Latched<()>` (local) or `Cancellation` (tokio) in the unguarded leading prefix | clean |
| Model delta stream (SSE) | std: owned pump task feeding an admitted `Mpsc` (the documented K0 pattern for cancellation-awkward producers); deltas drain with `#[drain(max)]`, `yields_to` protects user input | clean on std |
| Tool completions (concurrent, dynamic count) | ONE admitted mpsc of `(call_id, TerminalItem)` — tool tasks push into a single funnel; no dynamic arm set needed. Dynamic source sets are NOT required | clean; resolves the F-a "dynamic fan-out" worry |
| Interjections / follower ops | second mpsc arm below interrupt, above firehose | clean |
| Compaction prefire completion | `OptionalOneShot` (retained future) | clean on std |
| Timers (retry backoff, debounce) | `OptionalDeadline` | tokio-only today |
| Swarm notifications (candidate) | mpsc funnel, same as tools | clean |

**Conclusion (Observation):** on the std host, KC0 needs zero kernel changes.
The funnel pattern (owned tasks + admitted mpsc) covers streaming, tool
fan-out, and subagents without dynamic arms. Falsifier F-a's plausibility
risk on std is low.

## The gap is entirely on the bare-metal/WASM side (Fact by absence)

K0's root RESEARCH already records "Embassy/HAL adapters remain deferred."
kittens-code's portability requirement is the forcing function to build them.
Local `Latched`/`FixedQueue` suffice only when producers run on the same
task (true for single-threaded WASM; false for embassy tasks and ISRs).

## KX — proposed kittens kernel extensions (extend, don't fork; new `embassy` feature)

- KX1 `embassy-sync` channel adapter (`Channel`/`PriorityChannel` receiver →
  ReactorSource + Drainable + Backlog), mirroring the tokio `Mpsc` shape and
  close policies. Wake-aware: embassy channels store wakers, satisfying the
  preservation contract the sealed trait demands.
- KX2 `embassy-sync` `Signal`/`Watch` adapter — the no_std `Cancellation`/
  watch analog for interrupt + config sources.
- KX3 `embassy-time` deadline adapter — `OptionalDeadline` twin over
  `embassy_time::Timer`/`Instant`.
- KX4 (WASM) none required initially: wasm32-unknown-unknown is single-
  threaded, so local `Latched`/`FixedQueue` armed from host callbacks on the
  same task are in contract; revisit only for wasm threads/wasip3 async.
- KX5 (later, MCU model stream) reqwless response body → funnel via an
  embassy task + KX1 channel; no new source kind needed — same pump pattern
  as tokio, which keeps the kernel's "no hidden spawn" law intact.

All KX items are additive reviewed adapters behind a feature gate — exactly
the growth path the kernel spec reserved. None touch macro grammar or
arbitration law. KX1–KX3 are prerequisites for MCU *runtime* claims but not
for KC0 (which is std-runtime + bare-metal link-gate only).

## Dependency policy inputs (operator directive 2026-08-08: buy over build)

Adopted candidates already evidence-backed elsewhere: `agent-client-protocol`
(ACP adapter, std shim), `embedded-io`/`embedded-io-async` (+adapters),
`reqwless`+`embedded-tls` (embassy shim), serde/serde_json(alloc), `postcard`,
`crop` (rope, if/when needed), `mlua` (std Lua escape hatch), `model2vec-rs`
(std/wasm embedder), `grep` crate (std L3 search), `regex-automata` (no_std
search spike, D7), `schemars` (tool schemas — verify no_std story; hand-write
core schemas if std-bound). Build-from-scratch is reserved for: the Vfs port
(no no_std VFS trait exists — verified gap, input 03), the verb parser
(trivial by design), and the store (thin by design over the codec).
