# External spec review — Codex (gpt-5.6-sol, reasoning effort ultra), 2026-08-08

Independent cross-model-family review of SPEC.md v0.5, crate-structure focus
per operator loop directive. Read-only `codex exec` run; inspected real
kernel source (source/mod.rs, Cargo.toml) alongside the spec. Verdict:
FREEZE-AFTER-FIXES — 7 blockers, 9 majors. All folded into SPEC v0.6.

## Verdict

**FREEZE-AFTER-FIXES.** KC0 is not buildable from v0.5 without architectural decisions being made by the implementer. The evidence-slice idea is sound, but the effect boundary, crate graph, durability model, and controlling import ledger must be corrected first.

## Corrected topology

Arrow means “depends on”:

```text
KC0
kittens-code-core          → kittens-code-protocol
  └─ built-in tools are core modules

kittens-code-driver-tokio → core + protocol + kittens[macros,tokio]
kittens-code-cli          → driver-tokio + protocol
fixtures/code-no-std      → core + protocol

Post-KC0
kittens-code-driver-web    → core + protocol + kittens[macros,web-adapters]
kittens-code-driver-wasi-p2→ core + protocol + kittens[macros,wasi-adapters]
kittens-code-driver-embassy→ core + protocol + kittens[macros,embassy]
kittens-code-swarm         → core + protocol, only after D16/E4
```

Use package `kittens-code-cli` with binary name `kittens-code`. Do not create a runtime-neutral driver crate until a second driver proves reusable code beyond the existing core transition API.

## Findings

1. [BLOCKER] The tools crate has no specified acyclic edge. Core owns tool orchestration, while tools consume the core’s `Vfs`/`Exec` effect vocabulary ([§3](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:66), [K1–K2](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:349), [§11](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:383)). That implies either `core ↔ tools` or an unstated driver-side engine. Merge KC0 tools into `core`; they have one consumer and share its budgets, effect IDs, and continuations.

2. [MAJOR] Rename `kittens-code-std` to `kittens-code-driver-tokio`; keep the reactor there. Concrete source adapters, task ownership, clocks, and pumps are runtime-specific ([§6](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:183)). A generic driver crate now would be a one-implementation abstraction. Future Web, WASI-p2, and Embassy drivers should be siblings with conformance tests for the same logical topology.

3. [MAJOR] “Protocol: serde only” is sustainable only after removing non-wire concerns. Move `WindowLayout`, prompt state, and invariant-bearing capped values into core. Split `SessionConfig`—budgets, thresholds, prompt overrides, symbolic model-profile IDs—from target bootstrap configuration—endpoint/auth/TLS/store path/preopens/flash partition. Only `SessionConfig` may be patched and logged ([P5–P6](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:117), [configuration](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:416)). Represent IDs, versions, checksums, and timestamps with protocol-owned arrays/integers rather than requiring `uuid`, `semver`, or checksum dependencies.

4. [BLOCKER] `Event` has two opposite directions. P2 defines it as core→client, while `handle(Event)` consumes driver→core input; effect completions have no type, and §11 has no protocol-event emission effect ([P1–P2](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:107), [call model](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:185)). Define:

   ```rust
   CoreInput::{ClientOp, EffectProgress, EffectFinished, Persisted, TimerFired}
   CoreAction::{Commit, StartEffect, CancelEffect}
   Transition { actions: bounded owned collection }
   ```

   Every operation needs `EffectId` and `TurnEpoch`. The driver publishes protocol events only after the corresponding `Commit` receives durable acknowledgement.

5. [BLOCKER] The port/effect split is arbitrary and breaks sans-I/O. `Store` mutates and scans storage synchronously, yet the core promises no I/O ([S3](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:157), [§11](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:385)). Browser IndexedDB/OPFS and common MCU flash APIs are asynchronous; JSONL scans would block Tokio. Make Store append/read/search-page, embedding, peer access, Vfs, Exec, HTTP, and timers correlated Effects. Retain a synchronous port only when specified as deterministic, bounded, nonblocking, and memory-only. C8 needs no `TokenCount` port: provider usage is input data and the calibrated byte-ratio estimator is core logic.

6. [BLOCKER] The transition API has no re-entrancy or backpressure law. Return an owned bounded batch, not a lazy iterator borrowing core state. Completions—including the synchronous jail—must queue through an admitted source and never call `handle` recursively. Require bounded SSE/progress/effect queues, maximum active effects, cancel-aware producer waits, and whole-batch capacity handling. `#[drain]` and `#[yields_to]` govern service order, not capacity; the real backlog probe is expressly observational and immediately stale ([source trait](/Users/feral/mydev/rust-kittens/crates/kittens/src/source/mod.rs:90)).

7. [BLOCKER] Turn and cancellation ownership is unspecified. Core must own `SessionState`, `TurnState { phase, epoch, pending IDs, call order }`, RLM continuations, and the exactly-once terminal ledger. Drivers own only task/socket/process handles keyed by `EffectId`. Define first-terminal-wins, duplicate/late completion behavior, interrupt versus normal-terminal races, resume semantics, and shutdown draining/joining. Expand G4b beyond one tool case to model, tool, sub-model, timer, and shutdown paths ([L2](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:197)).

8. [BLOCKER] `ask` cannot work without a suspended interpreter model. Define a core-owned continuation containing query ID, program counter, typed `%N` results, pending node IDs, and remaining budgets. `ask` emits a sub-model Effect and resumes only on its terminal input. `ask-each` schedules incrementally and rejoins by partition index. Add separate limits for recursion depth, total subcalls, parallel subcalls, partitions, and selected bytes; partition size and verb count do not bound fan-out as claimed ([Q4](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:285), [`ask-each`](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:339)).

9. [BLOCKER] Streaming and durability laws contradict one another. Every model/tool delta is a record, but S7 requires a tool call and its later result to be one atomic append unit ([P2](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:110), [S2/S7](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:154)). Do not buffer an entire streamed result. Specify individually framed records—sequence, exact checksum coverage, transaction/effect ID—forming `Started → Progress* → exactly one Terminal`. Replay of an incomplete stream must append or derive `aborted_by_crash`. Also define the log payload as accepted Ops plus emitted Events/effect outcomes; “notification stream is the log” conflicts with logged config Ops.

10. [MAJOR] P6’s const-shaped cap conflicts with runtime-patchable budgets and serde replay ([P6](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:122), [Config](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:418)). Use core-private, kind-branded values such as `Capped<VerbOutput>` carrying the applied runtime limit and truncation metadata, under a compile-time hard ceiling. Do not derive unchecked `Deserialize`. Trybuild should prove category/bypass safety; property tests should prove runtime limits, aggregate meters, and malicious decode handling.

11. [MAJOR] `Vfs` and `Exec` are names, not implementable contracts. Specify normalized relative paths, `..`/absolute/symlink policy, bounded range reads, paged directories, revision-aware atomic writes, and rename semantics. `Exec` needs argv rather than a shell string, cwd, bounded env/stdin, sequenced stdout/stderr, exit status, deadline, and cancellation. A startup `SessionCapabilities` must control advertised tool schemas; Web/MCU must not advertise Exec merely because its data variant compiles ([K1–K2](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:351)).

12. [MAJOR] The current WASM adapter story is structurally wrong. `ReactorSource` is sealed, while `Latched`/`FixedQueue` cannot be armed by callbacks and do not self-wake ([trait](/Users/feral/mydev/rust-kittens/crates/kittens/src/source/mod.rs:37), [local sources](/Users/feral/mydev/rust-kittens/crates/kittens/src/source/mod.rs:177)). Single-threaded WASM does not solve borrowing or wakeup. Ledger reviewed Web Promise/channel/timer and WASI Pollable adapters in Kittens. Non-Tokio drivers must explicitly disable Kittens’ default Tokio feature ([manifest](/Users/feral/mydev/rust-kittens/crates/kittens/Cargo.toml:14)).

13. [MAJOR] The verb surface needs one typed IR. The EBNF does not enforce arity, result-reference types, duplicate flags, escaping, forward references, range semantics, or newline exclusion, and external `ask` means execution is not “total by construction” ([grammar](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:300)). Shell text, typed function calls, and future Lua should lower to the same closed IR. Mark grammar as versioned-experimental until E2 runs, not STABLE now. Define a versioned exact L3 pattern dialect before calling L3 a frozen baseline.

14. [BLOCKER] The controlling KC0 import ledger omits v0.5’s own laws. Section 14 imports S1–S6 and C1–C8, excluding KC0-blocking S7 and closed C9; P1–P8 are not explicitly imported either ([normativity boundary](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:15), [KC0 scope](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:431)). Replace ranges with an exhaustive ID ledger. D2 leaves exact wire shapes open and D4 leaves `WindowLayout` types unresolved ([register](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:519)); both must close before freeze, not at first-code review.

15. [MAJOR] Gate coverage is incomplete. Add cargo-metadata/feature-tree checks for T2/T3; terminal-path property tests for P3; torn-frame/incomplete-stream fixtures for S5/S7; ordering-barrier scenarios for L4; and parser golden/rejection/property tests for Q1. S7 says G2 gains a crash-tail fixture, but G2 omits it ([S7](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:165), [G2](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:477)). Seed jail clock, entropy, UUID/session IDs, and retry jitter before demanding byte-identical logs. Replace “higher semver major” with an explicit schema compatibility epoch—every v0.x version otherwise has major zero—and define unknown enum-kind handling, not merely unknown fields.

16. [MAJOR] The claim that all six prior-review conditions were folded is false:

   - Condition 1: partial; RESEARCH still repeats stale 95×, 16 KB TLS, no-prior-art, and “nearly free” language ([RESEARCH](/Users/feral/mydev/rust-kittens/docs/kittens-code/RESEARCH.md:93), [later correction](/Users/feral/mydev/rust-kittens/docs/kittens-code/RESEARCH.md:570)).
   - Condition 2: not met; D2 is open, and unimplemented L2/swarm ports are frozen into core rather than removable.
   - Condition 3: partial; metrics and two model families exist, but E1–E4 falsifiers, thresholds, and cost/time/token budgets do not. E2 has no actual gate, and E5 is referenced only by F-b.
   - Condition 4: safely deferred by D16, but correction records, provenance, and an enforcement oracle are missing.
   - Condition 5: not met; E4 omits the required centralized-coordinator arm ([W3](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:372)).
   - Condition 6: met; KC0 promises only no-std compilation plus a std runtime, with MCU runtime claims behind D-c.

17. [MAJOR] Freeze-status drift remains. The header makes D-b a freeze prerequisite, the register labels it “frontend only,” and the current untracked [input 14](/Users/feral/mydev/rust-kittens/docs/kittens-code/research-inputs/14-kittens-tui-seam.md:48) says it is resolved and KC1-only. Pin and incorporate input 14 or remove it; one D-b status must govern freeze.

## Target walk after correction

| Target | Runtime layering and platform ownership | Structural status |
|---|---|---|
| native std/Tokio | protocol + core + `driver-tokio`; JSON/framed store, cap-std Vfs, Tokio process, reqwest/rustls, monotonic/wall clocks and CSPRNG all in driver | Viable once Store is effect-driven |
| `wasm32-unknown-unknown` | protocol/core compile generically; concrete `driver-web` uses Fetch/Web Streams, host TLS, IndexedDB/OPFS, Performance/Date, WebCrypto; no Exec | Generic “unknown WASM runtime” is impossible; name a host and add wake-aware Kittens adapters |
| `wasm32-wasip2` | protocol/core + `driver-wasi-p2`; `wasi:http` host TLS, preopened FS, WASI clocks/random; Exec only through a declared host extension | Viable after async Store and WASI Pollable adapters |
| Embassy/MCU | protocol/core + `driver-embassy`; reqwless/embedded-tls, flash/littlefs Store/Vfs, embassy monotonic timer, board entropy and optional RTC; no Exec | Allocator-equipped MCU only; D7 and D-c must cover bounded heap, TLS, flash, cancellation, and power |

Architecturally impossible as written are synchronous durable Store access, callback-driven Web sources using the current local adapters, a host-independent `wasm32-unknown-unknown` runtime, mandatory UUIDv7 without trusted wall time, and buffering atomic streamed tool transactions. Concrete driver/adaptor implementations after those corrections are merely missing work.

No files were modified. GitKB queries were attempted, but its SQLite index could not create required temporary storage under the read-only sandbox; the actual source traits and macro guide were inspected directly.