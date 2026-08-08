# Kittens research report

- Initial research date: 2026-08-06
- Preimplementation challenge review: 2026-08-07
- Embedded generality review: 2026-08-07
- Executor-boundary review: 2026-08-07 (section 20A, Tokio cooperative scheduling budget as an expansion-experiment confound)
- Coverage-model and consumer review: 2026-08-07 (section 20B, layered defect elimination, escape surface, meta-harness and engine-author consumers)
- Status: evidence review and reversible implementation recommendation, not an implementation
- North-star codebase: Grok Build at commit [`393430ee4934bc791b0d538f304a21691c517433`](https://github.com/xai-org/grok-build/commit/393430ee4934bc791b0d538f304a21691c517433), committed 2026-08-06
- Second forcing fixture: Waveshare ESP32-S3 AMOLED firmware, revision-keyed and source-pinned in section 20

This report uses four labels deliberately:

- **Fact** — directly supported by a language/runtime contract, primary source, or inspected code.
- **Observation** — an interpretation of inspected behavior or ecosystem evidence.
- **Hypothesis** — something Kittens must measure rather than assume.
- **Recommendation** — the concrete design decision proposed for Kittens.

## 1. Executive summary

**Recommendation:** Kittens should continue to investigate an agent-first compile-time orchestration layer for long-lived Rust async systems. The evidence is strong enough to justify implementing one reversible reactor/source kernel, but not strong enough to freeze the broad v0.1 surface previously proposed. Structured scopes, capability values, cooperative resource shutdown, a generic render gate, and a simulator may belong in an eventual architecture; they are separate hypotheses and must not be bundled into the first implementation merely because they compose conceptually.

The stronger reactor-centered proposition should replace the earlier “safer wrappers around Tokio” framing as the long-term research direction:

> Kittens lets coding agents describe selected legal topology and scheduling constraints of long-lived async harnesses using familiar Rust, and compiles them into an ordinary future plus local compiler feedback. Tokio is the first production integration; lifecycle, authority, state-machine, and protocol features remain separately gated extensions.

**Recommendation:** treat that proposition as a Rust-embedded constraint language with a `no_std`, preferably no-alloc semantic kernel and compiler, then build runtime integrations and domain profiles above it. A future TUI, embedded-UI, or agent-harness package may add reviewed vocabulary and utilities, but it must share the kernel's meanings for source admission, precedence, dormancy, phases, and bounded service. “Language” here means machine-consumed orchestration notation embedded in familiar Rust, not a replacement general-purpose language. This separates reusable orchestration law from domain-specific side-effect protocols without freezing a package split before K0 evidence.

The important qualification is that Kittens validates declared structure, not the order in which the outside world produces events. For branches using its constrained path, it may prove that a declared shutdown source is polled before a firehose, that a selected source belongs to the reviewed repeated-race admission set, that a branch marked last is last, that a macro-generated drain is bounded, that declared relation graphs are acyclic, and that a successfully continuing generated handler passes through an after-event hook. It cannot prove that an adapter's semantic implementation is truthful, discover missing declarations, inspect arbitrary handler loops/awaits, or prove when a model token, keystroke, network packet, or writer acknowledgement will arrive.

The pinned Grok Build event/render loop provides strong empirical support for a narrower claim. Its behavior depends on a 23-arm biased `tokio::select!`, carefully ordered comments, a terminal-input channel introduced specifically for cancellation safety, a gate that prevents ACP streaming from starving input, a bounded ACP batch of 32, many `Option<Instant>`-to-`pending()` dormant sources, a deliberately last voice/STT source, loop-top housekeeping, post-select rendering, and a runtime frame-acknowledgement protocol. Source order, macro-managed drain bounds, buffered-yield gates, dormant-source behavior, and phase placement are plausible Kittens responsibilities. The frame state machine, terminal handoff, task ownership, and teardown order remain application/runtime protocols until a Kittens abstraction proves that it improves their legal API and diagnostics.

The embedded forcing case changes one important boundary. Embassy's own `no_std` selection combinators are ordinary `Future` implementations, and a stable-Rust scratch probe showed that a tiny ordered `core::future::poll_fn` selector can run under both Tokio and `embassy-futures`. The scheduling topology is therefore not inherently Tokio-specific. This does **not** prove that a Grok-scale generated future will pin and borrow cleanly, nor that an ESP-HAL interrupt source can retain a self-borrowing waiter ergonomically. Direct core polling is now the leading implementation candidate and the first falsification target, not a frozen public mechanism.

The working architecture remains a hybrid, but only its first two elements are committed to the initial evidence-producing slice:

- a procedural macro for a small set of global reactor constraints and generated lexical polling in one ordinary future, compared directly against a Tokio-select baseline;
- conservative persistent source adapters and trait admission for capabilities the macro cannot inspect, with semantic truth established by reviewed primitive contracts and tests; Tokio adapters come first and Embassy/HAL adapters remain deferred;
- ordinary ownership and typestate for local invariants when a later feature needs them;
- private runtime state for inherently dynamic facts such as sequence numbers and frame acknowledgements;
- structured scopes, capabilities, bracket-like shutdown, and deterministic tooling only after separate vertical slices demonstrate value.

**Rejected for the kernel and current architecture:** a new async runtime, a general effect type, an indexed computation type, a full session-type DSL, borrowed parallel task spawning based on unsafe lifetime erasure, a custom fair scheduler, and a home-grown deterministic simulator.

**Constraint-revealing verbosity remains a hypothesis with a strict admission test.** A declaration earns its place only when it changes generated behavior, rules out a program, selects a checkable risk policy, improves a diagnostic, or measurably improves agent comprehension. The earlier eight-attribute source contract repeats several facts already fixed by a sealed adapter. In particular, a mandatory `#[cancellation_safe]` annotation says nothing when every accepted reactor source must already satisfy the same trait bound. Lifecycle and close annotations similarly risk becoming marker-equality ceremony when the macro performs no global analysis with them. The first slice therefore starts with fewer declarations and compares them empirically against the fuller form.

### 1.1 Confidence after the preimplementation challenge

| Claim | Confidence | Why |
|---|---|---|
| Tokio should remain the first production integration | high | ecosystem compatibility and Grok both support it; nothing in the embedded evidence justifies shipping an Embassy backend in the first slice |
| reactor topology is fundamentally executor-independent | medium-high | `Future::poll`, Embassy's selectors, and the scratch probe support the boundary; source pinning and large expansion remain unproven |
| generated direct ordered polling should replace `tokio::select!` inside the kernel | medium | the small no-alloc probe passed, but Grok-scale borrowing, wake behavior, code size, spans, and non-`Unpin` adapters have not |
| the reactor/source kernel can remain `no_std` and allocation-free | medium | only the disposable three-source selector and a wasm no-std build passed; proc-macro expansion, real adapters, an ESP target, and self-borrowing sources remain absent |
| a macro can add value for source-order, cycle, last, shutdown, phase, and literal-drain checks | medium-high | the facts are syntactically global and macro prior art is strong; Kittens diagnostics remain untested |
| persistent admitted sources can distinguish reviewed repeated races from arbitrary futures | high for the boundary, medium for the API | Tokio, Grok, and ESP-HAL's explicitly cancellation-unsafe GPIO waits support the need; exact trait, sealing, pinning, and adapter ergonomics remain untested |
| the proposed 23-arm expansion will borrow cleanly and yield local errors | medium | no retained reactor prototype exists |
| priority classes are better than source-specific relations in the first surface | low | Grok supplies exact source-order relationships, not evidence that users need a reusable class DAG immediately |
| all eight base source annotations improve agent outcomes | low | several annotations only restate sealed type facts |
| phase capability values and a generic `SingleFlight` belong in the kernel | low-medium | the runtime protocol is real, but the generalized API and borrow behavior are unproven |
| `scope`, `resource`, `cap`, `flow`, and `sim` must ship with the first reactor | low | they address real problems but do not help falsify the reactor thesis and greatly widen implementation risk |
| Tokio-style `Scope` semantics generalize to Embassy | low | Embassy spawned tasks have static pools and no public join/abort handle; its lifecycle model is materially different |
| semantic verbosity improves agent generation and repair | unknown | plausible, but no controlled Kittens agent experiment has run |
| the core-poll and Tokio-select forms can be compared on a clean behavioral oracle | medium, lowered by section 20A | Tokio's per-task cooperative scheduling budget is consumed differently by the two forms and can return `Pending` with an item available; the confound must be instrumented before divergence is attributed to the mechanism |

## 2. Method, scope, and evidence

The original review covered stable Rust, Tokio, `tokio-util`, structured-concurrency crates, Cats Effect, ZIO, Haskell concurrency/effect libraries, Rust session types, capability systems, deterministic concurrency tools, Rust effect-system experiments, Tower, state-machine libraries, procedural-macro capabilities, and the Grok Build loop at the pinned commit. On 2026-08-07, `git ls-remote` confirmed that the pinned commit was still upstream `HEAD`. The commit remains a snapshot and reproducible oracle; the documents must not silently treat future Grok changes as already researched.

The Grok checkout was a temporary, shallow clone of the public upstream. GitKB indexed the pager and render crates; it identified `app::event_loop::run` as called by `app::run`, `drain_and_process` as called by the reactor, `process_effects` as the bridge to the effect executor, and the render writer as a separate acknowledgement-producing thread. Exact source reads were then made at the pinned commit.

The workspace contains no Rust implementation. Before the user prohibited implementation code, an ephemeral stable-Rust scratch crate checked several local type-system claims; it was subsequently moved to Trash in full. Section 15 records those results as historical evidence. The embedded review explicitly requested small research prototypes, so a second disposable crate tested only the uncertain executor-neutral polling claim outside the workspace and was removed after its results were recorded in section 20. It was not a Kittens implementation. The workspace remained documentation-only.

The embedded pass inspected the current official Waveshare documentation and board repository, `esp-hal`, Embassy, `embedded-hal-async`, display interfaces, `embedded-graphics`, Slint's MCU integration, the maintained `sh8601-rs` driver, and two nearby all-Rust watch firmware repositories. GitKB was used for the real firmware call graph before exact source reads. Hardware facts are revision-keyed; nearby 2.06-inch firmware is never presented as exact 1.8-inch board support.

This challenge pass applies an additional rule: detail is not evidence. A fully enumerated API, diagnostic catalog, or state machine remains a hypothesis until it is supported by a language/runtime contract, inspected production code, a retained Rust experiment, or an agent benchmark. The review therefore distinguishes behavioral oracles worth freezing from public spellings that must remain reversible.

Unless a subsection says otherwise, original ecosystem observations are dated 2026-08-06 and the embedded sources were rechecked on 2026-08-07. Download counts are directional ecosystem signals, not quality measures.

## 3. Current Rust async landscape

### 3.1 What Rust and Tokio already guarantee

**Fact:** Rust's [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) contract specifies polling. It does not specify a universal cancellation protocol. In common Rust async execution, cancellation occurs when the owner stops polling and drops the future.

**Fact:** synchronous destructors run through [`Drop`](https://doc.rust-lang.org/core/ops/trait.Drop.html). Stable Rust does not provide a general async destructor. Async drop remains experimental work tracked in [rust-lang/rust#126482](https://github.com/rust-lang/rust/issues/126482).

**Fact:** Tokio 1.53.1's [`spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html) requires `Send + 'static` for the task future and output. This prevents a normal spawned task from borrowing stack data whose lifetime could end first. A dropped [`JoinHandle`](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html) detaches rather than cancels the task, however, so ownership of the handle is not structural ownership by itself.

**Fact:** [`JoinSet`](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html) owns a set of tasks, aborts them on drop, and documents `join_next` as cancellation-safe. `abort_all` alone does not wait; callers must continue joining or call `shutdown` to know cancellation completed.

**Fact:** Tokio's [`select!`](https://docs.rs/tokio/latest/tokio/macro.select.html) drops losing branch futures. Default selection randomizes the first branch it polls, providing only “some level” of fairness. `biased;` polls top-to-bottom and explicitly makes fairness the programmer's responsibility. Tokio itself gives the high-volume-stream-versus-shutdown example and says shutdown should appear first.

**Fact:** Tokio documents `mpsc`, broadcast, watch receive, listener accept, and basic `read`/`write` operations as cancellation-safe. It documents `read_exact`, `read_to_end`, `read_to_string`, `write_all`, and fairness-queued `Mutex`, `RwLock`, `Semaphore`, and `Notify` waits as not cancellation-safe in a repeated select loop.

**Fact:** [`timeout`](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html) cancels by dropping the inner future and may exceed its nominal deadline if that future does not yield. [`spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) work generally cannot be aborted after it starts.

**Fact:** [`CancellationToken`](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html) provides cooperative cancellation and child tokens; [`TaskTracker`](https://docs.rs/tokio-util/latest/tokio_util/task/struct.TaskTracker.html) can wait until it is closed and empty, but closing it does not prohibit later spawning.

**Fact:** async closures and the `AsyncFn*` traits became stable in Rust 1.85 ([Rust 1.85 announcement](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html)). They improve higher-ranked callback ergonomics but do not make borrowed parallel spawning sound when the owning scope future can itself be forgotten or dropped.

**Fact:** [`Future::poll`](https://doc.rust-lang.org/std/future/trait.Future.html) receives `Pin<&mut Self>` because some future state machines are address-sensitive. The [`std::pin` documentation](https://doc.rust-lang.org/std/pin/) emphasizes that pinning is a library contract and that most `Unpin` values need no special application handling. **Candidate:** source adapters should hide pin projection from reactor handlers. Whether the public outer source must be `Unpin` remains part of the kernel trait/diagnostic experiment.

### 3.2 What remains expressible

Raw Tokio still permits all of the following:

- dropping a `JoinHandle` and unintentionally detaching work;
- placing a cancellation-unsafe future in a repeated `select!`;
- placing an always-ready source above shutdown in a biased select;
- creating priority cycles in comments and gates;
- draining an unbounded channel inside a branch;
- turning a closed optional receiver into a hot loop;
- performing blocking work in a select handler;
- aborting a task without awaiting termination;
- assuming an async finalizer will run after arbitrary future drop;
- using ambient filesystem, process, environment, and network authority;
- mutating runtime booleans into illegal workflow states;
- calling raw `tokio::spawn` outside the lifecycle that should own the task.

**Observation:** Rust eliminates memory unsafety and many lifetime errors; Tokio provides excellent primitives and accurately documents their contracts. Neither makes the global topology of a long-lived reactor a compiler-validated artifact.

**Recommendation:** retain Tokio as the first production integration, but stop treating Tokio selection as the semantic definition of a reactor. Kittens should neither replace Tokio's I/O driver, task scheduler, channel ecosystem, nor task implementation. Its candidate runtime-independent boundary is much smaller: one generated future that polls persistent sources in a validated lexical order. Section 20 makes that boundary and its remaining feasibility risks explicit.

## 4. The Grok Build reactor: empirical north star

### 4.1 Inspected implementation

The primary loop is [`event_loop::run` at lines 731–2848](https://github.com/xai-org/grok-build/blob/393430ee4934bc791b0d538f304a21691c517433/crates/codegen/xai-grok-pager/src/app/event_loop.rs#L731-L2848); its biased select is at [lines 2081–2840](https://github.com/xai-org/grok-build/blob/393430ee4934bc791b0d538f304a21691c517433/crates/codegen/xai-grok-pager/src/app/event_loop.rs#L2081-L2840). The module describes itself as a thin `tokio::select!` loop, but the function is more than 2,000 lines because it owns the orchestration boundary. The async frame writer and acknowledgement state live in [`render/draw.rs` lines 58–356](https://github.com/xai-org/grok-build/blob/393430ee4934bc791b0d538f304a21691c517433/crates/codegen/xai-grok-pager-render/src/render/draw.rs#L58-L356). The repository pins Tokio 1.52.3 at this commit, one patch release behind the 1.53.1 ecosystem version observed during research.

The reactor has one initialization phase and four recognizable repeating phases:

1. **Initialize once:** construct the presenter/application state and request the initial presentation before entering the main loop.
2. **Before poll:** run deferred terminal suspends, lazily start voice, enforce voice/session ownership, synchronize keyboard mode, re-arm dynamic polls, and derive deadlines.
3. **Poll/select:** a biased select chooses one of 23 sources.
4. **Handle:** the selected branch mutates application state and may spawn effects into a `JoinSet`.
5. **After event:** `presenter.present_if_dirty` attempts one coalesced frame.

That shape is strong evidence for making loop-top and post-event control-flow positions explicit. It is not evidence for the previously proposed phase-capability types or their exact macro grammar; those are implementation hypotheses.

### 4.2 Actual biased source order

The order below is semantic because the loop uses `biased;`.

| Order | Source | Current contract encoded by code/comments |
|---:|---|---|
| 1 | leader connection cancellation | terminal, highest priority; prevents a retained sender from hanging the loop |
| 2 | graceful quit notification | above ACP firehose so SIGTERM cannot starve |
| 3 | writer event | acknowledgement or fatal writer failure; unlocks the next frame |
| 4 | ACP/model stream | may remain ready; disabled while terminal input is buffered; drains at most 32 messages and stops early for input |
| 5 | `JoinSet` completion | cancellation-safe task completion |
| 6 | restore progress channel | repeated mpsc source |
| 7 | background update oneshot | dynamically present, consumed once, then dormant |
| 8 | terminal input channel | cancellation-safe mpsc facade over a dedicated crossterm reader thread; drains/coalesces a backlog |
| 9 | resize deadline | optional deadline; debounce |
| 10 | deferred render deadline | optional deadline; frame-rate throttle |
| 11 | terminal-suspend retry | optional deadline; merely opens a loop-top gate |
| 12 | scroll clock | dynamically derived deadline |
| 13 | animation/recovery tick | dynamic deadline; also reconciles lost cancellation/turn completion |
| 14 | billing poll | optional deadline |
| 15 | access-gate poll | optional deadline |
| 16 | subscription watch | optional deadline |
| 17 | roster poll | active only while the dashboard is open |
| 18 | away-recap poll | periodic timer |
| 19 | pager config watcher | repeated watcher |
| 20 | system appearance watcher | optional watcher mapped to `pending()` when absent |
| 21 | leader connection-status watch | optional watch source |
| 22 | reconnect reinitialization oneshot | optional, generation-bound completion |
| 23 | voice/STT channel | optional mpsc, explicitly and deliberately last |

**Fact:** [the current voice/STT arm](https://github.com/xai-org/grok-build/blob/393430ee4934bc791b0d538f304a21691c517433/crates/codegen/xai-grok-pager/src/app/event_loop.rs#L2794-L2839) is still present, and its comments say it must be last because an open microphone can generate interim transcripts at roughly 5–20 Hz and backlog its 128-slot channel. The ordering guarantees only that voice cannot starve sources above it; it does not guarantee that voice itself eventually runs.

**Fact:** [the ACP branch at lines 2112–2150](https://github.com/xai-org/grok-build/blob/393430ee4934bc791b0d538f304a21691c517433/crates/codegen/xai-grok-pager/src/app/event_loop.rs#L2112-L2150) is gated on `input_rx.is_empty()`. Its comment rejects simply moving input above ACP because that would reverse the starvation direction. It drains up to `ACP_DRAIN_BATCH_MAX = 32`, stopping as soon as input arrives.

**Observation:** a continuously backlogged ACP source can still dominate task completions and timers below it. The current design protects shutdown, writer acknowledgements, and terminal input explicitly; other lower sources rely on the ACP channel becoming temporarily non-ready or accept weaker service. Kittens should force this to be declared as an intentional starvation allowance or repaired with a real yield relationship.

The graceful-quit arm reconstructs `quit_notify.notified()` on each select iteration. Tokio lists fairness-queued `Notify::notified` among operations that are not cancellation-safe in the general repeated-select sense because a losing waiter can lose queue position. Grok appears to have a single logical quit waiter and `Notify` retains a permit when no waiter consumes it, so this is not evidence of an observed lost quit; it is evidence that the safety argument depends on usage details outside the branch type. Kittens' Notify adapter should retain an owned waiter inside the persistent source, making the repeated-race contract structural.

### 4.3 Terminal input and cancellation safety

Grok does not poll crossterm's event stream directly. [The terminal-reader comment and spawn sequence](https://github.com/xai-org/grok-build/blob/393430ee4934bc791b0d538f304a21691c517433/crates/codegen/xai-grok-pager/src/app/event_loop.rs#L1449-L1511) record that dropping a losing `next()` future could strand its waker. The implementation instead starts a dedicated reader thread and forwards timestamped events through an unbounded Tokio mpsc receiver, whose `recv()` is documented cancellation-safe.

The thread also supports an explicit pause/park handshake before `$EDITOR` or `$PAGER` takes the tty. Atomic writes are ordered to reject stale acknowledgements; the reader uses bounded crossterm polls so pause and shutdown are observed; the handoff starts only after input is parked and all accepted frames are drained.

After the first input event, [`drain_and_process`](https://github.com/xai-org/grok-build/blob/393430ee4934bc791b0d538f304a21691c517433/crates/codegen/xai-grok-pager/src/app/event_loop.rs#L3064-L3345) drains every immediately available event into a vector before coalescing rapid keys, CSI fragments, and paste. That immediate drain has no numeric bound. The optional non-bracketed-paste extension uses 2 ms detection and 10 ms continuation windows and caps extension iterations at 5,000, although each `drain_immediate` call can still collect the currently buffered remainder. A Kittens design can preserve this only by explicitly accepting an unbounded application-level drain; the v0.1 benchmark instead proposes a bounded input batch and treats the changed latency/coalescing behavior as a migration decision to test.

**Observation:** this is the canonical adapter for a cancellation-unsafe or lifecycle-awkward producer: isolate it in owned background work and expose a cancellation-safe channel source to the reactor.

**Recommendation:** the kernel diagnostic for an unapproved repeated async future should direct the user to isolate the operation behind an approved persistent channel source. The first slice need not invent `source::channel_task(...)` or require a Kittens `Scope`; an ordinary explicitly owned Tokio task or RAII thread in the fixture is sufficient to test the source boundary. A Kittens-owned helper and scope integration earn promotion only if a later lifecycle slice improves on that explicit baseline without hidden detachment.

### 4.4 Dynamic sources and hot-loop prevention

Grok repeatedly maps `Option<Instant>`, optional receivers, optional watchers, and optional oneshots to either an awaited operation or `std::future::pending()`. After voice closes it sets `voice_rx = None`; after the update oneshot completes it sets the receiver to `None`. This prevents a completed/closed source from returning immediately forever.

**Observation:** the pattern is correct but replicated, and a small omission can create a busy loop.

**Recommendation:** at least one optional deadline and one optional channel adapter in the kernel should own dormant/armed/closed state and transition to dormant automatically. That is enough to test the contract and the hot-loop mutation. A complete family of optional watch, oneshot, interval, and close-policy variants should follow demonstrated use rather than be frozen up front. “Closed once, then pending until explicitly replaced” is a library behavior and test oracle, not a per-branch descriptive annotation.

### 4.5 Rendering and writer acknowledgement

Grok's [`Presenter` at lines 322–418](https://github.com/xai-org/grok-build/blob/393430ee4934bc791b0d538f304a21691c517433/crates/codegen/xai-grok-pager/src/app/event_loop.rs#L322-L418) holds five runtime fields: dirty, force-full-repaint, an optional in-flight sequence target, last draw time, and an optional scheduled draw deadline. Requests coalesce. A frame is submitted only when dirty and no target is in flight. The writer reserves a monotonically increasing sequence before enqueueing bytes, sends `Written(sequence)` after flushing, and reports failure as a terminal event. An acknowledgement at or beyond the target opens the gate. Tests cover coalescing, sticky force-repaint, no-output frames, late acknowledgements, and waiting for the last payload emitted by a draw.

The writer thread itself is owned and joined during terminal teardown. [The outer application cleanup](https://github.com/xai-org/grok-build/blob/393430ee4934bc791b0d538f304a21691c517433/crates/codegen/xai-grok-pager/src/app/mod.rs#L973-L990) drops the terminal sender, joins the writer, emits teardown escape sequences, drops an agent-shutdown guard that cancels and joins the in-process agent with a grace period, and then kills remaining child processes. The [`AgentShutdownGuard` at lines 63–103](https://github.com/xai-org/grok-build/blob/393430ee4934bc791b0d538f304a21691c517433/crates/codegen/xai-grok-pager/src/acp/spawn.rs#L63-L103) exists specifically to make cancellation/join happen on normal return, `?`, or panic unwind.

**Observation:** the one-frame-in-flight invariant is dynamic because acknowledgements and sequence values are dynamic. Encoding sequence numbers at the type level would be counterproductive. The established boundary is application-private runtime state so callers cannot set `in_flight` directly. An ownership-bearing submission permit is a later comparison hypothesis, not yet the right generic API. Any candidate must model Grok's synchronous queue/reserve step, its valid “draw produced no writer payload” outcome, and the fact that an arbitrary submission closure's acceptance atomicity cannot itself be proven by Rust's type system.

**Revised recommendation:** the first reactor fixture should keep a small application-owned presenter faithful to Grok's runtime behavior. This tests whether writer acknowledgements, draw deadlines, requests, and `after_event` coexist with the macro without prematurely asserting that a generic gate is better than an ordinary private Rust struct. A later rendering slice may promote a generic single-flight gate only if it prevents an invalid call sequence, keeps borrowing local, and gives clearer failures than the application-owned baseline.

### 4.6 Task and shutdown ownership gaps

Most application effects are spawned into a `JoinSet`, and dropping the loop aborts them. That is a good ownership center. A few paths use raw spawning: the lazy voice pipeline discards its `JoinHandle`, and reconnect reinitialization retains only an `AbortHandle`. These tasks are expected to terminate when their channels close, hit a timeout, or are explicitly aborted during a later reconnect.

**Observation:** those tasks are not as structurally legible as the `JoinSet` effects. The implementation can be correct, but correctness depends on channel closure and handler discipline across files.

**Revised recommendation:** use the Grok-like task-completion source in the reactor fixture, but keep task ownership outside the kernel contract. A structured `Scope` remains a high-value follow-on hypothesis. It should be tested against `JoinSet`/`TaskTracker` in its own vertical slice rather than coupled to the first macro borrow and diagnostic experiment.

### 4.7 Grok invariants by enforcement mechanism

This table classifies the likely enforcement boundary; it is not a commitment that every listed Kittens mechanism belongs in the first slice.

| Invariant | Current Grok mechanism | Recommended Kittens mechanism |
|---|---|---|
| shutdown precedes a firehose | lexical order + comment | proc-macro graph validation |
| voice is last | lexical order + long comment | `#[last]` plus best-effort/starvation declaration |
| ACP yields to buffered input | manual `if input_rx.is_empty()` | checked yield edge + backlog-capable source |
| ACP drain is bounded | constant + loop condition | macro-validated `drain(max = 32)` and `DrainableSource` bound |
| terminal receive survives internal selection loss | architecture comment + mpsc | admitted persistent source contract; reconstruction/repeat claims remain separate |
| optional sources become dormant | `Option` + repeated `pending()` blocks | dormant source adapter |
| every successfully continuing selected event can render once | statement after `select!` | generated after-event phase |
| terminal suspend runs before next poll | loop-top call + comment | generated before-poll phase |
| one frame is in flight | private booleans/sequence state + tests | private runtime state + consuming permit |
| writer acknowledgement gates next frame | runtime sequence comparison | runtime protocol, deterministic tests |
| terminal handoff parks input before drain | function order + atomics | encapsulated resource/protocol API; Loom tests for atomics |
| tasks terminate with loop | mostly `JoinSet`, some conventions | later structured-scope experiment; not a reactor-kernel guarantee |
| ACP event IDs are monotonic/deduplicated | runtime high-water marks | runtime protocol checks; cannot be type-level |
| active source depends on UI/session state | `Option`, booleans, match guards | explicit runtime guard and dynamic adapter; typed state tables deferred |
| external event order | runtime | not statically enforceable |

### 4.8 What a Kittens migration should expose, not hide

The Kittens form may be longer in the declaration section, but it should name only facts consumed by checking or generation: stable source IDs, readiness where starvation analysis needs it, drain bounds, yield/precedence edges, shutdown, and phase placement. Adapter types own lifecycle, close, and selection-loss behavior unless a global check needs separate metadata. Handler bodies should remain ordinary Rust. The leading generated form should remain recognizable as one loop, one small ordered poll future, one event match, bounded drain loops, and hook calls; direct `tokio::select!` remains the fidelity control.

The migration should expose two facts the current comments make easy to miss:

1. strict priority is not fairness;
2. sources below an always-ready source require a yield path or an explicit decision that starvation is acceptable.

## 5. Cats Effect findings

Cats Effect 3.7.0 was current during this review ([project site](https://typelevel.org/cats-effect/), [v3.7.0 release](https://github.com/typelevel/cats-effect/releases/tag/v3.7.0), 2026-03-08).

| Cats concept | Harness value | Rust/Tokio equivalent | Kittens decision |
|---|---|---|---|
| `IO` | explicit delayed computation | `Future<Output = Result<...>>` | do not duplicate |
| fibers | task identity and cancellation | Tokio tasks/handles | own through `Scope` |
| structured scopes/supervisor | prevents fiber leaks | partial with `JoinSet`/trackers | provide a narrow scope |
| `Resource` / bracket | async acquire-use-release | RAII only covers synchronous drop | steal semantics for cooperative async release |
| `uncancelable` / poll regions | protect acquisition/release | no universal Rust equivalent | provide narrow cancellation-deferred regions inside Kittens operations |
| `Ref`, `Deferred`, `Semaphore` | coordination | Tokio locks/channels/semaphore | reuse Tokio; add no branded wrappers without a stricter contract |
| temporal/race | deadline and racing semantics | Tokio time/select | expose constrained reactor/scope operations |
| typed error channel | recoverable errors | `Result<T, E>` | use ordinary `Result` |

Cats Effect's [`MonadCancel`](https://typelevel.org/cats-effect/docs/typeclasses/monadcancel) and [`Resource`](https://typelevel.org/cats-effect/api/3.x/cats/effect/kernel/Resource.html) distinguish cancellation masking and finalization across success, failure, and interruption. Its fiber docs warn that low-level `start` can leak lifecycle and prefer scoped/supervised forms ([concepts](https://typelevel.org/cats-effect/docs/concepts), [`Supervisor`](https://typelevel.org/cats-effect/api/3.x/cats/effect/std/Supervisor.html)).

**Recommendation:** steal bracket/finalizer outcomes, cooperative cancellation backpressure, and supervisor ownership. Do not recreate `IO`, type classes, or syntax.

## 6. ZIO findings

ZIO 2.1.26 was current ([release](https://github.com/zio/zio/releases/tag/v2.1.26), 2026-05-06).

[`ZIO[R, E, A]`](https://zio.dev/reference/core/zio/) expresses required environment, typed failure, and success. [`Scope`](https://zio.dev/reference/resource/scope/) and interruption provide strong resource/fiber semantics; [`ZLayer`](https://zio.dev/reference/contextual/zlayer) builds dependency graphs; schedules compose retry and repeat policies ([schedule guide](https://zio.dev/guides/tutorials/retry-and-repeat-policies-with-schedule/)).

**Observation:** Rust already has a typed error channel (`Result`) and explicit values/constructor injection. Encoding an environment type `R` would spread generic parameters through harness code and obscure which concrete authority value is used at the action site.

**Recommendation:** pass capability/service values explicitly. Adopt bounded, inspectable retry policies as plain structs. Do not introduce a ZIO-like effect environment or layer graph in v0.1.

## 7. Haskell findings

Haskell's [`async` 2.2.6](https://hackage.haskell.org/package/async) makes `withAsync`-style lifetimes form a task tree. [`resourcet` 1.3.0](https://hackage.haskell.org/package/resourcet) generalizes registered release actions. [`io-sim`](https://hackage.haskell.org/package/io-sim) demonstrates the value of a pure concurrency/time simulator with replay and schedule exploration.

[`stm` 2.5.3.1](https://hackage.haskell.org/package/stm) provides composable atomic transactions over `TVar`, queues, and related primitives. That semantic is valuable when an invariant spans multiple shared variables, but Rust/Tokio has no directly equivalent general STM, and recreating it would add a second coordination model. A harness reactor already benefits from single-owner state mutation plus channels; Kittens should reuse locks/channels for the remaining cases and leave database/storage transactions to their native systems. It should not add STM in v0.1.

Parameterized/indexed monads can describe `Computation<Before, After, Output>`; Atkey's foundational treatment is [“Parameterised Notions of Computation”](https://www.cambridge.org/core/journals/journal-of-functional-programming/article/parameterised-notions-of-computation/82CE5F0583C3390BBBD305830255FAA0). GHC's [`LinearTypes`](https://downloads.haskell.org/ghc/9.14.1/docs/users_guide/exts/linear_types.html) remains explicitly experimental in ergonomics.

Current effect-library directions differ:

- [`effectful` 2.6.1.0](https://haskell-effectful.github.io/) emphasizes an efficient extensible effect environment;
- [`Bluefin 0.7.0.1`](https://hackage.haskell.org/package/bluefin) uses value-level handles whose scope limits effect use;
- [`Polysemy 1.9.2.0`](https://github.com/polysemy-research/polysemy), [`fused-effects 1.1.2.7`](https://github.com/fused-effects/fused-effects), and freer approaches trade syntax, interpretation, and performance differently.

**Observation:** Bluefin's explicit value handles transfer well to Rust capability values. Indexed computation syntax does not: Rust ownership plus `Run<Approved>` and consuming methods produces more local diagnostics.

**Recommendation:** steal value-scoped authority and structured lifetime semantics. Express local transitions with ordinary Rust typestate, not an indexed monad.

## 8. Session types and typed protocols in Rust

| Project | Version observed | Maintenance signal | Ergonomic finding |
|---|---:|---|---|
| [Dialectic](https://docs.rs/dialectic/latest/dialectic/) | 0.4.1 | last release 2021 | expressive binary protocols; macro/type machinery and old maintenance |
| [Ferrite](https://docs.rs/ferrite-session/latest/ferrite_session/) | 0.3.0 | last release 2022; [paper](https://arxiv.org/abs/2205.06921) | strong theory, limited docs, macro-heavy |
| [async-session-types](https://docs.rs/async-session-types/latest/async_session_types/) | 0.1.2 | last release 2022 | small, old experiment |
| [`par`](https://docs.rs/par/latest/par/) | 0.3.10 | release 2025 | native-enum direction is attractive, but lifecycle/drop expectations do not match Kittens without adaptation |
| Rumpsteak / [AURA fork](https://docs.rs/rumpsteak-aura/latest/rumpsteak_aura/) | 0.9.1 fork | recent but low adoption; [paper](https://arxiv.org/abs/2112.12693) | sophisticated multiparty checking, high conceptual cost |
| [mpstthree](https://docs.rs/mpstthree/latest/mpstthree/) | 0.1.17 | last release 2024 | multiparty macro/type complexity |
| [Telltale types](https://docs.rs/telltale-types/latest/telltale_types/) | 17.0.0 | active, rapidly versioned in 2026 | ambitious choreography/Lean correspondence; much larger system than Kittens needs |
| [Hibana](https://docs.rs/hibana/latest/hibana/) | 0.9.6 | active in 2026, early adoption | small endpoint surface over affine multiparty projection; promising but specialized |

**Observation:** session types can reject illegal message order, but full global choreography adds generic/macro/diagnostic cost, transport assumptions, and a second lifecycle model. The ecosystem is innovative but fragmented; newer Telltale and Hibana work is too young to make a foundational v0.1 dependency.

**Recommendation:** full session typing is not in Kittens core or v0.1. Use app-owned consuming endpoint typestate for short, high-value binary protocols. Keep an experimental adapter boundary for Hibana/Telltale evaluation. The reactor's event topology and the application protocol are separate: priority order cannot prove remote protocol compliance.

## 9. Capability-security findings

Object-capability design treats authority as possession of an unforgeable reference rather than a global permission check; the classic rationale is described in [“The Structure of Authority”](https://papers.agoric.com/papers/the-structure-of-authority-why-security-is-not-a-separable-concern/abstract/).

[`cap-std` 4.0.2](https://docs.rs/cap-std/latest/cap_std/) demonstrates practical Rust values such as directory handles whose operations are relative to an existing namespace. Ambient constructors require an explicit [`AmbientAuthority`](https://docs.rs/cap-std/latest/cap_std/ambient_authority/struct.AmbientAuthority.html). WASI similarly passes preopened handles; Wasmtime grants no filesystem access by default unless a directory is preopened ([WASI capabilities](https://github.com/WebAssembly/WASI/blob/main/docs/Capabilities.md), [`WasiCtxBuilder`](https://docs.wasmtime.dev/api/wasmtime_wasi/struct.WasiCtxBuilder.html)).

**Recommendation:** Kittens capabilities are concrete values wrapping real roots, transports, policies, and revocation state—not zero-sized “permission present” markers.

Required rules:

- constructors live at a visually obvious bootstrap boundary;
- normal modules cannot mint authority;
- narrowing computes an intersection and cannot expand authority;
- delegation is explicit cloning/borrowing of the narrowed handle;
- one-shot approval is non-`Clone`, target-bound at runtime, and consumed;
- revocation is a runtime shared-state check;
- capability presence is not claimed as a sandbox if code can still call `std`, Tokio, or process APIs directly;
- adversarial plugins require an OS/WASI/process sandbox in addition to Rust API discipline.

## 10. Structured-concurrency findings

[`futures-concurrency` 7.7.1](https://docs.rs/futures-concurrency/latest/futures_concurrency/) offers runtime-agnostic join/race composition that can borrow because it does not detach spawned tasks. [`moro 0.4.0`](https://docs.rs/moro/latest/moro/) explores borrowed async scopes but is experimental. [`async-scoped 0.9.0`](https://docs.rs/async-scoped/latest/async_scoped/) exposes the core difficulty: its completely safe API blocks, while async borrowed-scope paths require safety caveats because forgetting/dropping the scope future can invalidate borrowed tasks.

**Extension hypothesis, not K0:** a future **Tokio-specific** scope may use safe `Send + 'static` spawned children owned by a registry. Borrowed concurrency remains available through ordinary join combinators, not detached spawned tasks. That candidate would:

- rejects accidental detach;
- requests cooperative cancellation on exit;
- waits for a configured grace period;
- hard-aborts survivors;
- drains task completions before reporting shutdown complete;
- surfaces panic, cancellation, cleanup timeout, and task failure distinctly;
- cannot promise async cleanup after the scope future itself is externally dropped and no longer polled.

## 11. Cancellation and resource safety

The earlier lifecycle design used four terms, but the embedded pass requires a fifth distinction at the source boundary:

- **selection-loss preserving source:** another reactor source can win without discarding this source's required durable/armed state.
- **reconstructable waiter:** dropping and recreating this specific waiter preserves its documented progress; this matches Tokio's repeated-select question but is not implied by source retention.
- **cancellation-atomic:** an operation-specific observable commit occurs entirely once or not at all. This requires protocol/storage support and is independent of waiter reconstruction.
- **cancellation-deferred:** a Kittens cancellation request is recorded but observed only after the protected region finishes.
- **cleanup-guaranteed:** synchronous `Drop` runs when the owner is dropped; an async finalizer is guaranteed only while a Kittens-owned future remains driven through cooperative shutdown and within its cleanup budget.

**Fact:** arbitrary external future drop cannot await an async finalizer. Process abort, `abort` without draining, runtime destruction, and non-yielding work further limit guarantees.

**Extension hypothesis, not K0:** any later resource API must state outcomes rather than promise magic. Acquisition and release could run cancellation-deferred; use could remain cancellation-aware; release could be awaited after success, recoverable error, and cooperative cancellation while the owner remains driven. Panic unwind and arbitrary outer drop still provide only synchronous `Drop`, not the async release callback. The exact API needs its own evidence slice.

The Grok terminal teardown is a useful future resource fixture: accepted frames are drained before terminal reset, teardown still runs if the drain fails, and agent cancellation/join has a bounded grace. K0 records that order but does not implement nested resources or infer cleanup order from field drop order.

## 12. Deterministic concurrency testing

| Tool | Version observed | Strength | Limitation for Kittens |
|---|---:|---|---|
| [Loom](https://docs.rs/loom/latest/loom/) | 0.7.2 | exhaustive small-model checking of replacement atomics/synchronization | intrusive types; not a Tokio application simulator |
| [Shuttle](https://docs.rs/shuttle/latest/shuttle/) | 0.9.1 | randomized, PCT, replay, and DFS schedule exploration | replacement runtime/primitives; not transparent Tokio compatibility |
| [MadSim](https://docs.rs/madsim/latest/madsim/) | 0.2.34 | deterministic distributed simulation and Tokio-like replacements | cfg/ecosystem integration cost |
| [Turmoil](https://docs.rs/turmoil/latest/turmoil/) | 0.7.2 | deterministic multi-host network simulation over Tokio-like networking | focused on networked systems, not arbitrary TUI sources |
| [io-sim](https://hackage.haskell.org/package/io-sim) | 1.9.1.0 | conceptual gold standard for time/concurrency/fault simulation | Haskell, not reusable directly |
| [FoundationDB simulation](https://apple.github.io/foundationdb/testing.html) | project practice | demonstrates deterministic time, faults, and reproducible seeds at scale | architecture, not a Rust library |

Tokio's test-util clock supports paused and automatically advanced time ([Tokio testing guide](https://tokio.rs/tokio/topics/testing)).

**Recommendation:** K0 uses private controllable sources and Tokio paused time for its mutations. A public injectable clock/fault/randomness/transport layer and scripted trace driver remain a separately gated extension. Shuttle/MadSim/Turmoil adapters are optional research paths; Kittens should not build a new general simulator.

If a public scripted reactor is later promoted, it should record at least: complete modeled ready-set, selected source, source class, service-window size, handler outcome, hook outcome, task lifecycle event, and virtual timestamp. A production biased poller cannot generally observe the full ready-set after it finds a ready source and must label that field unavailable. Replay should identify a source by stable declared ID, never lexical branch index.

## 13. Existing Rust effect-system findings

[`effect-rs 0.1.0`](https://docs.rs/crate/effect-rs/latest/source/README.md) models `Effect<R, E, A>` but was alpha-level with negligible adoption. Rust [`effectful 0.3.0`](https://docs.rs/effectful/latest/effectful/) similarly introduces effect/context/layer machinery; [`effectful_tokio`](https://docs.rs/effectful_tokio/latest/effectful_tokio/) documents runtime compromises including non-`Send` behavior around Tokio spawning. Other category-oriented crates remain niche.

**Observation:** these systems make required effects explicit, but at the cost of a new computation representation, interpreters, macro/context machinery, generic signatures, and unfamiliar compiler errors. They duplicate `async`, `Result`, function parameters, and established Tokio integration without demonstrating better coding-agent performance.

**Recommendation:** explicitly omit `kittens::effect`. Reactor handler capabilities are enforced by the APIs they call and values they possess, not an effect row. Do not add `#[requires(shell)]` metadata unless the generated code can actually restrict or verify it; decorative effect annotations would be semantic theater.

## 14. Established Rust patterns and macro feasibility

### 14.1 Tower

[`tower::Service` 0.5.3](https://docs.rs/tower/latest/tower/) provides a familiar request/response abstraction with readiness and backpressure. [`ServiceBuilder`](https://docs.rs/tower/latest/tower/struct.ServiceBuilder.html) composes timeout, retry, concurrency limits, and other middleware.

**Recommendation:** accept and expose Tower services at LLM/tool transport boundaries. Do not use `Service` for reactor sources, typestate, scopes, or capabilities. A dropped `call` future still requires a documented cancellation/idempotency contract; retry must be policy- and operation-aware.

### 14.2 Typestate and state machines

The [Embedded Rust Book's typestate/state-machine chapters](https://docs.rust-embedded.org/book/static-guarantees/typestate-programming.html) demonstrate consuming transitions and state-specific methods. [`statig 0.4.1`](https://docs.rs/statig/latest/statig/) shows that an attribute macro can generate straightforward sync or async hierarchical state-machine code.

**Recommendation:** use app-owned `Run<State>` types for short linear workflows and ordinary runtime enums for long-lived reactor state. A process restart restores serialized runtime tags into an enum of validated typed variants; deserializing a phantom marker is not proof.

### 14.3 Procedural macros

The [Rust Reference](https://doc.rust-lang.org/reference/procedural-macros.html) confirms that procedural macros consume and emit token streams, carry spans for error reporting, and can emit `compile_error!`. Stable [`compile_error!`](https://doc.rust-lang.org/stable/core/macro.compile_error.html) supports local custom failures.

There are important limits:

- a proc macro cannot ask rustc which traits an arbitrary expression implements;
- it cannot prove semantic cancellation safety from an async body;
- it cannot inspect runtime readiness or external event order;
- stable [`proc_macro::Diagnostic`](https://doc.rust-lang.org/proc_macro/struct.Diagnostic.html) remains nightly-only, so rich multi-span warning/help diagnostics are constrained;
- proc macros are unhygienic and must emit absolute paths and collision-resistant private names;
- rust-analyzer enables proc-macro expansion by default, but custom token DSLs still have weaker editing/refactoring behavior than ordinary Rust ([rust-analyzer configuration](https://rust-analyzer.github.io/book/configuration)).

**Observation:** these limits favor a hybrid compiler boundary. The proc macro validates declared global metadata. Generated helper calls let rustc validate source trait contracts. Runtime checks own dynamic facts.

### 14.4 Prior art for compile-time scheduling

[RTIC](https://rtic.rs/) demonstrates that a Rust macro can analyze a whole declared task/resource topology, reject invalid priority/resource configurations, and generate low-overhead code. Its real-time/embedded execution model is not reusable for a Tokio harness, but its division between declarative topology analysis and boring generated code is directly relevant.

[`selectme 0.7.2`](https://docs.rs/selectme/latest/selectme/) explores fast/fair select expansion but does not validate a harness-level priority graph, lifecycle, cancellation contracts, or phases. Stream merging is attractive when all sources share one policy, but it erases per-source priority, guard, drain, and lifecycle distinctions central to the Grok case.

## 15. Candidate architectures and experiments

### 15.1 Comparative decision table

The ratings below are research judgments, not measured agent outcomes. They justify which alternative to probe first; the K0 ablation must replace the subjective diagnostic/familiarity ratings with evidence.

| Criterion | A. Rust-native typestate/caps only | B. `Effect<R,E,A>` | C. Rust-native + reactor proc macro | Raw Tokio |
|---|---:|---:|---:|---:|
| familiar handler code | excellent | poor | good | excellent |
| local diagnostic quality | excellent | poor/variable | good if designed deliberately | variable |
| global scheduling validation | none | possible only with more machinery | strong | none |
| cancellation-source checking | local wrappers only | interpreter-dependent | strong for curated adapters | documentation only |
| lifecycle ownership | good with scope | potentially strong | strong with scope | optional |
| generic entropy | low | high | low outside generated code | low |
| generated-code transparency | n/a | often interpreter-based | can be direct ordered `Poll` control flow | direct |
| Tower/ecosystem compatibility | excellent | adapter boundary | excellent | excellent |
| agent prior familiarity | high | low | medium-high | highest |
| Grok topology coverage | insufficient | theoretically broad, practically costly | best practical frontier | comments/tests only |

**Recommendation:** test C first because it is the only candidate that directly addresses global topology while preserving ordinary handlers. This is a falsifiable selection, not a public API freeze. If a Grok-scale function-like macro cannot preserve normal borrowing, local errors, rust-analyzer usability, and boring expansion, narrow or replace the macro surface rather than defend the table.

### 15.2 API surface alternatives for `reactor`

| Surface | Global validation | Diagnostics | Familiarity | Decision |
|---|---|---|---|---|
| builder API | mostly runtime unless types explode | runtime or generic errors | high | reject as primary; useful only for dynamic escape |
| deeply typed priority-list generics | possible | poor and nonlocal | low | reject |
| declarative `macro_rules!` | syntax checks, weak graph algorithms/spans | moderate | high | insufficient |
| attribute macro over arbitrary function body | can parse metadata, but distinguishing phases/branches is fragile | good | highest | keep as future sugar |
| function-like proc macro with Rust expressions/blocks | full declared graph, controlled expansion | potentially best stable spans | recognizable as `select!` | first implementation probe; retain only if the 23-arm gate passes |
| runtime scheduler/interpreter | dynamic graph possible | runtime | medium | explicit escape only |

### 15.3 Historical stable-Rust scratch results

An ephemeral crate was checked with `rustc 1.96.0 (2026-05-25)`, Cargo 1.96.0, Tokio 1.53.1, and tokio-util 0.7.19 before the no-code directive. `cargo check --all-targets` and the test suite passed. The crate was then removed.

| Experiment | Result | Representative diagnostic/result |
|---|---|---|
| consuming typestate across `.await` | passed | `Run<Proposed>::validate(self).await -> Run<Validated>` compiled |
| execute before approval | rejected locally | E0599: no method `execute` on `Run<Proposed>`; note says method exists for `Run<Approved>` |
| non-copyable approval used twice | rejected locally | E0382: use of moved value `approval` |
| filesystem authority used as network authority | rejected locally | E0308: expected `&Network`, found `&ReadWorkspace` |
| runtime narrowing of read root | passed | absolute/parent traversal rejected; child authority resolved beneath root |
| async closure borrowing a scope body | passed | stable `AsyncFnOnce` higher-ranked body compiled |
| spawned child borrowing stack data | rejected | E0597: value does not live long enough; child requires `'static` |
| consuming endpoint next-state | passed | send produced `Orchestrator<Waiting>`; wrong receive on `Ready` produced E0599 |
| dynamic typestate restoration | passed | runtime tag restored to an enum containing typed variants |
| timeout/drop behavior | passed | synchronous `Drop` flag ran; code after the pending await did not |
| cooperative async release | passed | cancellation-aware use returned only after async release completed |

**Fact established by the experiment:** local typestate, authority possession, consumption, and small endpoint transitions produce repairable rustc errors on stable Rust.

**Fact not established:** a production scope's full panic/grace/abort semantics, a reactor proc macro, source-trait diagnostics, macro expansion quality, and Grok-scale borrow checking. The user prohibited further code before those prototypes were created. They are mandatory preimplementation gates in `SPEC.md`, not claims of completed validation.

## 16. Reactor/source design findings

### 16.1 Source contracts should be sealed and conservative

A public safe marker trait lets anyone make a false semantic claim. An `unsafe trait` would incorrectly imply a memory-safety obligation. Therefore the kernel source-admission contract should be sealed and implemented by Kittens for reviewed adapters while extension/certification options remain research.

The source implementation needs whatever static contracts the reactor actually consumes. The earlier design made all of the following mandatory branch declarations:

- lifecycle: repeating, one-shot, or dynamically dormant;
- selection loss: required operation state is retained when another source wins;
- readiness: may remain continuously ready or cannot do so by construction;
- close behavior: not closable, silently dormant on close, or one typed close event then dormant;
- capabilities: drainable and/or backlog-probeable.

Only selection-loss admission and readiness currently feed kernel-wide checks. Whole-source Drop, waiter reconstruction, external repeat safety, lifecycle, and close behavior are valuable adapter documentation and runtime tests, but the macro has no demonstrated global use for repeating them on every arm. Drainability and backlog probing should remain capability traits required only when the corresponding operation is requested.

Arbitrary work can become a source by isolation behind a persistent channel. The exact Kittens task helper and unchecked escape spelling are provisional. Any future escape must not use Rust's `unsafe` keyword merely for logic risk, because breaking the orchestration contract is not automatically undefined behavior.

### 16.2 Macro/type division

The macro can validate identifiers, declared relation/yield graphs, cycles, lexical order, `last`, shutdown precedence, drain literals, hook presence, and incompatible policies visible in its input. K0 keeps reactor-state availability in explicit runtime guards; compile-time state tables are deferred. The macro emits calls whose trait bounds verify that a source is admitted and that drain/yield operations use sources with the required capabilities.

The macro cannot introspect those trait implementations itself. Metadata used for global analysis must therefore be visible in the declaration and checked against the source type by generated Rust. The first slice should require only metadata that the macro actually consumes. Readiness qualifies because starvation analysis needs it. A mandatory cancellation-safe annotation does not qualify when every branch already has the same unconditional source-admission bound—and it would now misleadingly collapse several contracts. Whether explicit quiescent readiness is better than a checked default remains an agent-ergonomics experiment.

### 16.3 Scheduling is a partial order; fairness is separate

Declared source-to-source precedence should form a directed acyclic graph. The written arm order should be required to be a valid linear extension and emitted unchanged into ordered polling. This avoids a macro silently moving branches away from their local comments and handlers. Priority classes are a plausible compression for repeated relations, but Grok does not establish that they belong in the first grammar; start with source IDs and add classes only if the 23-arm fixture or agent edits demonstrate clear value. A relation means local polling precedence when sources are ready together; it is not a claim about event arrival, handler preemption, or executor task priority.

Starvation analysis is conservative:

- a may-remain-ready source above a protected source requires a checked yield edge or higher placement of the protected source; the first slice should test a protected-by-default policy in which an explicit reason is required only to accept starvation;
- shutdown sources may not accept starvation and should form a compiler-enforced leading prefix before all non-shutdown sources;
- `last` is structural, not a fairness guarantee;
- bounded drain limits handler monopolization but does not by itself make lower select branches fair;
- general weighted/round-robin fairness requires stateful arbitration beyond the proposed lexical poller and is deferred.

A dynamic source that is quiescent after firing can still be rearmed by application code on every loop iteration. No source marker can prove that an arbitrary rearm expression eventually changes. The kernel must document this as a runtime liveness boundary and cover immediate-rearm failures with ordinary deterministic tests rather than overstate the static starvation guarantee.

This distinction would make the current Grok ACP-versus-lower-timers assumption visible rather than silently “approved.”

### 16.4 Generated code should be boring

The candidate expansion model has no runtime graph, registry lookup, allocation merely to represent topology, or hidden task spawn. The first slice implements only per-item bounded drain; batch collection waits until a real use case justifies its allocation and capture complexity. Before the embedded pass, the target unconditionally generated `tokio::select!`. The leading candidate now generates a small `core::future::poll_fn` body that polls sources in lexical order and returns one owned event. This is still ordinary Rust control flow, not an executor: it uses the caller executor's `Context` and waker and owns no task queue, timer wheel, I/O driver, or wake thread.

The first implementation must retain a direct Tokio-select expansion as a comparison. The runtime-neutral form graduates only if its Grok-scale borrowing, pinning, wake behavior, expansion size, performance, and diagnostics are no worse in material ways. The target expansion then generates:

1. a borrow-ending selected-item representation, with a private event enum as the leading candidate;
2. one ordinary loop;
3. the before-poll block;
4. one ordered poll closure whose source polls produce enum variants, or the retained Tokio baseline during the comparison;
5. optional bounded drain code;
6. one match containing ordinary handler blocks;
7. the after-event block;
8. explicit continue/stop propagation.

The source graph exists only during macro expansion and in generated metadata for diagnostics/observability. Direct polling does not make arbitrary operations safe: an admitted source must preserve its operation across an internal lost race, and destroying the entire reactor may still cancel that operation according to its adapter contract.

### 16.5 Phase capabilities are useful only for participating APIs

A generated `after_event` block can guarantee that the hook runs; it cannot stop an arbitrary handler from calling an arbitrary writer method. A phase capability could restrict APIs that voluntarily require it, but neither its borrow behavior nor agent value has been demonstrated. The kernel should implement phase placement and execution first, using an application-owned presenter. A phase capability is promoted only if a subsequent mutation shows an invalid call that it prevents with a local diagnostic and without awkward generic spread.

### 16.6 Enforced constraint versus declaration

The challenged design uses this admission table for reactor syntax:

| Candidate declaration | What it changes | Kernel decision |
|---|---|---|
| source ID | graph identity, traces, and diagnostics | keep |
| shutdown | forces a leading terminal branch and terminal result shape | keep |
| source-specific `before` | adds a validated ordering edge and can reject a move/cycle | keep; priority classes remain provisional |
| `last` | rejects any later branch | keep |
| readiness | drives starvation analysis and is checked against the source type | keep one minimal spelling; compare explicit-both versus checked-default forms |
| buffered yield | generates a guard and drain stop, and requires backlog probing | keep |
| bounded drain | generates bounded behavior and requires drainability | keep `each` mode first; batch mode is follow-on |
| required phase | rejects omission and controls generated execution | keep |
| dynamic `when` | changes branch enablement but cannot prove the predicate correct | keep as ordinary operational syntax, with an explicit limitation |
| lifecycle | currently restates adapter behavior without changing topology | remove from mandatory kernel syntax |
| cancellation-safe | currently restates the unconditional approved-source bound | remove from branch syntax |
| close behavior | currently restates adapter behavior without changing topology | remove from mandatory kernel syntax |
| starvation-allowed reason | explicitly weakens the default protection policy | keep only if protection is the default; it is a checked risk acceptance, not a safety proof |

Two scope limits must be stated beside the feature, not buried in non-goals:

- a macro-managed drain can be bounded, but an ordinary handler can still write its own unbounded `try_recv` loop;
- a reactor source can be approved for repeated races, but a handler can still await a cancellation-unsafe operation after selection.

Kittens narrows the supported path. It does not inspect arbitrary handler semantics.

## 17. Lessons specifically relevant to coding agents

1. **Local intent is a plausible advantage over remote prose.** The Grok loop's best comments are precise, but a coding agent moving a branch may not retrieve the comment that establishes a cross-branch invariant. Checked attributes beside a source may keep the contract in the edit window; the agent benchmark must measure whether they actually help.
2. **Semantic verbosity may improve context.** Naming a source `acp_stream`, declaring `may_remain_ready`, `drain(max = 32)`, and `yields_to(input)` gives both the macro and the LLM a compact architectural summary. Lifecycle/cancellation/close restatements do not currently meet the same bar.
3. **Compiler errors should give the most direct causal repair.** A cancellation-unsafe source diagnostic should suggest channel isolation; a cycle should name the conflicting edges; an invalid drain should request a positive literal bound. When several repairs preserve different policies, the diagnostic must state the alternatives rather than prescribe a weakening as canonical.
4. **Avoid equivalent spellings after evidence selects one.** One source declaration form and one optional deadline should reduce API hallucination; future scope/escape spellings are not frozen.
5. **Do not leak macro internals.** Generated helper types need readable names and `#[doc(hidden)]`; diagnostics must use declared source IDs, not tuple positions or nested generic aliases.
6. **Compile-fail examples are core documentation.** Each invariant needs a smallest invalid example, semantic diagnostic anchors, and a constraint-preserving counterpart. Exact prose/numbering freezes only after repair trials.
7. **Expansion snapshots matter.** Agents debugging runtime behavior need a supported way to inspect the ordinary generated polling future and compare it with the direct Tokio oracle.
8. **Runtime checks still need constrained APIs.** Sequence numbers, reconnect generations, and frame acknowledgements remain dynamic. Application-private state is established; consuming permits are useful only if a later comparison demonstrates a clearer legal API.

**Hypothesis:** the added declarations will improve agent repair because the intended topology appears locally and diagnostics name the violated relationship. This must be benchmarked; verbosity that merely repeats types without enabling a check should be deleted.

### 17.1 Semantic verbosity test

An extra declaration earns its place only if all applicable answers are yes:

| Question | Example that passes | Example that fails |
|---|---|---|
| Does rustc/the macro/runtime validator consume it? | `#[drain(max = 32)]` generates a bounded loop and a trait assertion | prose label `#[important]` |
| Does it expose a local architectural decision? | `#[starvation(allowed, reason = "telemetry is best effort")]` | repeating a type name already visible next door |
| Can a contradictory declaration fail? | readiness metadata is checked against the sealed source marker | free-form `#[cancellation_safe]` trusted on any future |
| Does the diagnostic identify a repair? | yield error names dominant and protected sources | an unresolved nested marker tuple |
| Does it avoid spreading generic entropy? | branch attributes disappear into one ordinary polling future | adding scheduler parameters to every handler type |
| Can the agent benchmark measure value? | mutation stops compiling and repair iterations fall | ceremony with no invalid counterpart |

This reframes a historical cost. Human keystrokes matter less when agents generate code, while local architectural evidence and compiler inputs may matter more. Tokens, attention, compile time, and maintenance still cost something. Kittens should test semantic declarations and retain only those that outperform local non-enforced context; aliases, repeated unchecked prose, and annotations that cannot disagree with anything should be removed.

### 17.2 Harness-first programming economics

**Observation:** the relevant programming loop is not source generation followed by a final compiler gate. It is a repeated search loop: an agent proposes Rust, the compiler and macro validator reduce the legal space, tests or simulation expose dynamic failures, and structured diagnostics guide the next repair. This makes the error surface part of the programming interface.

**Recommendation:** Kittens diagnostics should be designed as a causal repair gradient. A useful message names the local declaration, the relationship it violates, the operational consequence (for example starvation or lost-race risk), and one policy-preserving repair. It should not merely expose a generated generic failure, recommend deleting the declaration, or make an arbitrary waiver the easiest path. The benchmark must record whether agents preserve the invariant rather than merely reach a compiling state.

**Observation:** canonicality has a different value for an agent than for a human library author. A human can select among `select!`, task sets, channels, and cancellation primitives from experience; an agent samples from many plausible idioms and may choose one that has not been audited by Kittens. One canonical spelling can reduce hallucinated API combinations and make examples, diagnostics, and expansion snapshots align.

**Hypothesis:** semantic redundancy helps only when it is independently consumed. A declaration that repeats a sealed adapter fact without changing generated code, legal program space, diagnostics, runtime validation, or test identity is ceremony—even if it makes the file appear more rigorous. The annotated-baseline, lean, and maximal conditions in the specification are intended to falsify this hypothesis rather than assume that verbosity is beneficial.

**Recommendation:** retain familiar Rust as the host language and expose intent while hiding mechanism. Attributes and ordinary Rust blocks may state precedence, fairness exceptions, phase placement, or bounded service; generated polling, wake registration, borrow scopes, and bookkeeping should remain boring Rust. This preserves the model's learned Rust priors while moving orchestration topology out of comments and working memory.

### 17.3 Context amnesia and rehydration

**Observation:** code-generation cost and context-reconstruction cost are asymmetric. An agent can emit another declaration cheaply, but after context compaction it may not recover why a branch is last, why a future must be isolated, or which capability authorizes a call. A short human-oriented source file can therefore be a poor agent artifact even when its abstractions are elegant.

**Recommendation:** treat important source artifacts as recoverable memory. The local source, types and method availability, macro declarations, compile-fail examples, reason-bearing waivers, diagnostics, and expansion should form a connected path by which a fresh agent can reconstruct the local operating model. High-level architecture documents remain summaries; repair-critical policy should not live only in them.

**Hypothesis:** semantic redundancy can act like an error-correcting code for architecture. A type-level adapter contract and a nearby checked reactor declaration may serve different consumers—local retrieval versus global validation—and a disagreement can expose an edit that would otherwise silently weaken the program. This is beneficial only when both representations are independently consumed. Repeated labels that cannot disagree with or affect anything remain semantic theater.

**Recommendation:** add an agent rehydration benchmark distinct from generation and repair. Agent A receives the original requirements and establishes a fixture; its context is discarded; Agent B receives only the repository and a risky modification request. Measure recovered invariants, retrieval operations, attempted illegal edits, declarations weakened or deleted, diagnostics used, and hidden-oracle preservation. Compare raw Tokio, inert local metadata, lean Kittens, and maximal Kittens. This directly tests whether source artifacts can re-educate a forgetful agent.

**Observation:** descriptive names and reason-bearing exceptions can improve local recovery, but “longer is better” is not established. Names should carry domain, authority, or lifecycle when that disambiguates a consequential call; token/context cost remains a measured tradeoff. A reason string explains a policy choice but is not a proof and must not be allowed to contain secrets.

## 18. Naming research

As of 2026-08-06, the exact Rust package name `kittens` appeared unclaimed in the [crates.io search](https://crates.io/search?q=kittens), while `kitten` exists. Practical collision risk remains substantial:

- [Typelevel Kittens](https://github.com/typelevel/kittens) is a known Scala/Cats derivation library, particularly close to this research domain;
- [ThunderKittens](https://github.com/HazyResearch/ThunderKittens) is a prominent GPU kernel project;
- [Kitty terminal](https://sw.kovidgoyal.net/kitty/kittens/) uses “kittens” for extensions;
- npm and PyPI searches contain uses of the term and therefore offer no clean cross-registry namespace.

The [USPTO search portal](https://www.uspto.gov/trademarks/search) contains unrelated uses of the word, and the USPTO's own [clearance guidance](https://www.uspto.gov/trademarks/search/federal-trademark-searching) explains that federal searching is only part of a clearance search. This research is not a legal opinion.

**Recommendation:** retain “Kittens” as the project name for now, reserve the crates.io name if desired, and prepare `kittens-orchestrate` or `kittens-reactor` as package-name fallbacks. Do not claim trademark clearance.

## 19. Concrete final recommendation

The first authorized implementation should still be an unpublished, reversible kernel rather than the previously specified broad v0.1. The embedded pass changes the first mechanism to test, not the required restraint. It consists of:

- two packages, `kittens` and `kittens-macros`, because Rust requires a separate proc-macro package; the facade's candidate reactor/source base is `no_std` and no-alloc, while the proc macro runs on the host with `std`;
- one function-like `reactor!` probe whose leading expansion is an ordinary future using explicit lexical source polling, with a direct biased `tokio::select!` expansion retained as the control condition;
- source-specific ordering edges, shutdown-prefix and last checks, required loop phases, one allocation-free bounded per-item drain, one buffered-yield relation, and conservative starvation analysis;
- one conservative persistent-source admission contract plus only the capability traits actually requested by drain/backlog operations;
- the minimum Tokio adapters needed for cancellation, mpsc with type-visible dormant/emit-once close behavior, optional mpsc, optional deadline, and retained one-shot behavior;
- a 23-arm Grok-shape fixture and a small embedded-shape fixture with timer, interrupt-like, dormant, and ownership-returning completion sources; the latter is host-testable and does not claim ESP32 hardware support;
- intentionally broken desktop and embedded mutations, expansion inspection, real rustc/rust-analyzer diagnostics, and immediate coding-agent repair trials;
- test-private scheduling controls and Tokio paused time, not a public simulator or replay schema.

The kernel deliberately omits priority classes, batch collection, mandatory lifecycle/cancellation/close annotations, phase capability values, a generic single-flight gate, `scope`, timeout, `resource`, `cap`, `flow`, public `sim`, stable tracing/serialization, Tower/process backends, and escape APIs. Omission does not reject those ideas. It prevents independent hypotheses from hiding the result of the reactor experiment.

The first slice is successful only if the 23-arm shape borrows naturally, generated code remains recognizable as straightforward `Future::poll` control flow, the Tokio oracle remains behaviorally faithful, the embedded-shape fixture needs no allocation, the targeted illegal mutations fail at the intended layer, and a coding agent can repair the failures from local diagnostics without weakening unrelated constraints. Failure to meet those conditions should reshape or falsify the macro/source architecture before a public surface is frozen.

The most important unknown is now sharper: whether one core-polling expansion can preserve normal Rust borrowing and pinning for both a 23-arm Grok shape and a retained interrupt/transfer source, while producing causal diagnostics. The earliest high-information result is therefore not a completed adapter catalog. It is a side-by-side core-poll versus Tokio-select expansion, the two borrow fixtures, four representative compile-fail mutations, and diagnostic-only agent repair attempts.

If that gate passes, rendering and **Tokio** structured scope should be tested as separate vertical slices against application-owned `Presenter` and `JoinSet` baselines. An Embassy adapter is a later source-integration experiment, not a K0 deliverable. Capabilities, resources, typestate guidance, protocols, simulation, and observability remain architectural research until their own evidence warrants promotion.

## 20. Embedded async UI / reactor generality

### 20.1 Exact hardware target and support status

**Fact:** “Waveshare ESP32-S3-Touch-AMOLED-1.8” is not one controller configuration. Waveshare's current shipment policy says V1 was discontinued and shipments changed to V2 on 2026-05-30. The [current official documentation](https://docs.waveshare.com/ESP32-S3-Touch-AMOLED-1.8) and [pinned first-party repository](https://github.com/waveshareteam/ESP32-S3-Touch-AMOLED-1.8/blob/ba32b5cbca96f0e04b0736d04959b6e832268d3f/README.md#L24-L59) establish:

| Board profile | Display | Touch | Shared relevant hardware |
|---|---|---|---|
| 1.8 V1 | SH8601 over QSPI | FT3168 over I2C | ESP32-S3R8, 368×448 AMOLED, 8 MB PSRAM, 16 MB flash |
| 1.8 V2, current shipment | CO5300 over QSPI | CST820 over I2C | same MCU, resolution, PSRAM, and flash |

The first-party source notes that V2 files retaining `CST816x` identifiers use a compatible API name; they do not change the fitted controller from CST820. The [revision probe](https://github.com/waveshareteam/ESP32-S3-Touch-AMOLED-1.8/blob/ba32b5cbca96f0e04b0736d04959b6e832268d3f/examples/esp-idf/92_qmi8658_imu/components/board_variant/board_variant.c#L74-L119) distinguishes the touch addresses. The [official color-bar example](https://github.com/waveshareteam/ESP32-S3-Touch-AMOLED-1.8/blob/ba32b5cbca96f0e04b0736d04959b6e832268d3f/examples/esp-idf/13_display_colorbar/main/display_colorbar_main.c#L15-L106) applies the V2 panel gap and uses an explicitly DMA-capable 16-row stripe rather than requiring a whole framebuffer. A full RGB565 frame is 329,728 bytes.

**Observation:** the published SKUs distinguish package/battery options, while Waveshare instructs users to identify controller revision from the rear label. Current official prose is not internally clean: the compatibility notice and pinned repository give the V1/V2 mapping, while a later screen-description paragraph still says FT3168 only. The mapping above is triangulated from the compatibility notice, product material, and pinned repository rather than inferred from one stale paragraph.

**Observation:** first-party firmware/support is ESP-IDF C and Arduino C++ rather than Rust. The maintained [`sh8601-rs` 0.1.8 example](https://github.com/theembeddedrustacean/sh8601-rs/blob/4bcddfd529017135f19a5a9a6e79dd6b8ef1b460/examples/ws_18in_amoled.rs#L40-L171) supports the exact V1 display with ESP-HAL, PSRAM, QSPI, and DMA, but it is a sequential draw/flush/delay demo with no touch reactor. Research found no maintained full Rust firmware for the exact V2 CO5300/CST820 1.8 board. That is an absence-of-evidence result, not proof that none exists. Generic CO5300 crates and nearby watch firmware are not board support packages.

**Recommendation:** every fixture and future adapter must name a revision-keyed board profile. “Waveshare 1.8” is too imprecise for a hardware claim. Kittens must not claim current 1.8 V2 support from nearby 2.06-inch evidence.

#### Ecosystem support map

| Layer | Current evidence | Relevance to Kittens |
|---|---|---|
| first-party board support | Waveshare's [ESP-IDF BSP 2.0.3](https://components.espressif.com/components/waveshare/esp32_s3_touch_amoled_1_8/versions/2.0.3/readme) configures CO5300 QSPI with automatic SPI DMA and compatible touch probes | establishes a current C path, not a Rust/no-std adapter; despite the README's V1 compatibility claim, the inspected display path always constructs CO5300, so V1 display operation was not validated here |
| `esp-hal` | bare-metal `no_std`; async GPIO, timers, DMA, PSRAM, SPI, and sleep APIs; current GPIO waits explicitly are not cancellation-safe and are feature-gated `unstable` with no stability guarantee | supplies primitives and ownership, but every async source needs a version-specific contract audit |
| Embassy on ESP | no-alloc executor/futures/time integration is used by real nearby firmware; selector and spawned-task semantics are inspected below | proves ordinary futures can host the reactor, not that Tokio scope semantics transfer |
| `embedded-hal` / `embedded-hal-async` | executor-neutral device traits; async methods specify I/O behavior but generally not future-drop behavior | useful adapter boundary, insufficient source-admission proof by itself |
| display interfaces | [`display-interface`](https://docs.rs/display-interface/latest/display_interface/) includes synchronous and async data/command traits; async send has no general selection-loss contract | an async display call is not automatically a repeated reactor source |
| exact V1 display | `sh8601-rs` 0.1.8, embedded-graphics-compatible, PSRAM/QSPI/DMA example | maintained Rust display path, currently blocking at the application boundary |
| generic V2 controller | [`display-driver-co5300` 0.1.1](https://docs.rs/display-driver-co5300/latest/display_driver_co5300/) exists for CO5300-class panels | not evidence of 1.8 V2 pins, reset, offset, touch, or whole-board support |
| V1 touch family | [`ft3x68-rs`](https://docs.rs/ft3x68-rs/latest/ft3x68_rs/) is a synchronous no-std FT3x68-family driver relevant to FT3168 and returns at most two points in a fixed-capacity vector | existing types can bound V1 touch storage; IRQ scheduling and V2 CST820 board support remain application/integration work |
| nearby V2 touch family | [`cst816s` 1.0.1](https://docs.rs/crate/cst816s/1.0.1/source/README.md) is a no-std blocking CST816S driver whose own status leaves interrupt handling unfinished | Waveshare's CST816x-compatible naming makes it a research lead, not evidence that it correctly supports the fitted CST820 or exact V2 board; no reviewed CST820 adapter was found |
| `embedded-graphics` | no-std synchronous drawing traits and fixed-capacity framebuffer options | Kittens should orchestrate around it, never replace it |
| Slint MCU | application-driven superloop, multiple buffer strategies, allocator required by current MCU setup | Kittens may surround framework events/transfers; it should not duplicate Slint's renderer/event ownership |

PSRAM is optional heap/storage policy, not part of source semantics. ESP-HAL DMA adds chip- and memory-region-specific alignment/descriptor requirements; some are runtime-validated and some are encoded by ownership/alignment wrappers. Kittens must not make a generic `drain` or rendering declaration silently select PSRAM, allocation, alignment, or a DMA strategy.

### 20.2 Real Rust firmware loop anatomy

The closest maintained production-shaped Rust fixture is [`infinition/waveshare-watch-rs` at `15c052b`](https://github.com/infinition/waveshare-watch-rs/tree/15c052ba2389a9a97bf68ae0135da0641a71b4dd). It targets the **2.06-inch** Waveshare watch: the MCU and CO5300/QSPI/PSRAM shape are nearby, but its 410×502 panel, FT3168 touch, flash size, TE pin, and board wiring differ from 1.8 V2.

**Fact:** its [`main` loop](https://github.com/infinition/waveshare-watch-rs/blob/15c052ba2389a9a97bf68ae0135da0641a71b4dd/src/main.rs#L570-L733) computes a runtime cadence, then races three futures in this written order:

1. `Timer::after(tick)`;
2. touch falling edge;
3. button falling edge.

Embassy's `select3` polls in argument order, but this firmware discards which branch won and processes all current state after wake. Source order is therefore wake arbitration, not handler priority. Cadence ranges from 30 seconds while off, through 10-second AOD checks and one-second watch updates, to 16–33 ms interactive/game ticks. The rest of the iteration gates IMU, RTC, battery, touch, radio, UI state, and rendering by runtime state. The shape is recognizably `derive cadence → poll → update state → conditionally render → repeat`, but it is not a clean universal four-hook template: early continues, direct per-app flushes, and long awaits are part of the loop.

**Observation:** priority at the selection boundary does not make handlers responsive. The firmware can await Wi-Fi connection for up to eight seconds in the same task before touch is polled again. Kittens can validate poll topology; it cannot imply handler preemption or bound arbitrary awaits.

The inspected rendering path is also a semantic-theater warning. [`Framebuffer::swap_and_flush`](https://github.com/infinition/waveshare-watch-rs/blob/15c052ba2389a9a97bf68ae0135da0641a71b4dd/src/drivers/framebuffer.rs#L23-L80) says it swaps front and back buffers, but it never calls `swap`. GitKB found zero callers of `Framebuffer::swap` and one `swap_and_flush` call from the Flappy handler. The [QSPI implementation](https://github.com/infinition/waveshare-watch-rs/blob/15c052ba2389a9a97bf68ae0135da0641a71b4dd/src/drivers/qspi_bus.rs#L1-L145) uses `SpiDmaBus<Blocking>` and 8 KB conversion chunks. There is no reactor-visible frame acknowledgement or independently in-flight display submission.

A second nearby firmware, [`QuackHack-McBlindy/ESP32-S3-WATCH-rs` at `9cf0df9`](https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/tree/9cf0df918a2d013084f41946eb1c5fe11f53f4b4), is also for a [2.06-inch CO5300/FT3168 board](https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/blob/9cf0df918a2d013084f41946eb1c5fe11f53f4b4/README.md#L744-L848). It distributes touch, display, voice/audio, and networking across Embassy tasks rather than one giant reactor. Its [display task](https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/blob/9cf0df918a2d013084f41946eb1c5fe11f53f4b4/src/main.rs#L129-L225) has genuinely dormant and active modes, but an [unbounded async TE wait](https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/blob/9cf0df918a2d013084f41946eb1c5fe11f53f4b4/src/gui/mod.rs#L127-L146) can still delay Stop once rendering starts. That TE behavior is nearby-only: both 2.06 fixtures use a dedicated TE input, while the inspected exact 1.8 V2 pin/BSP sources expose touch IRQ but no equivalent TE input. This is evidence that Kittens' useful unit is a long-lived orchestration boundary, not necessarily the whole firmware or a portable display milestone.

**Hypothesis:** one small reactor syntax can describe both Grok's many-source loop and an embedded task's timer/interrupt/dormant topology without pretending that every firmware component belongs in the same loop. The dual fixture, not diagram similarity, must decide this.

### 20.3 Selection-loss contracts across Tokio and embedded HALs

**Fact:** [`embassy_futures::select3`](https://github.com/embassy-rs/embassy/blob/f37b9b6bbf1d4540575d97582da7b4244ca4c202/embassy-futures/src/select.rs#L105-L147) is a `no_std` ordinary future. It polls left to right, returns at the first ready branch, and drops losing futures. A continuously ready early branch can starve later branches inside the task regardless of Embassy executor fairness.

**Fact:** [`embedded_hal_async::digital::Wait`](https://github.com/rust-embedded/embedded-hal/blob/41f29f6bfced1cae0cbe712ba96ee32c075b3125/embedded-hal-async/src/digital.rs#L154-L184) specifies edge/level behavior but no future-drop guarantee. Kittens cannot blanket-admit a trait method merely because it is async.

**Fact:** current [`esp-hal` GPIO waits](https://github.com/esp-rs/esp-hal/blob/e1a042e3fa92839b157f72ef60b8db884156d067/esp-hal/src/gpio/asynch.rs#L11-L19) explicitly are not cancellation-safe. Dropping the waiter stops listening; an edge after drop is ignored by a later wait. The nearby watch loop reconstructs exactly such waits in every `select3`. Its comment that drop removes the interrupt is true cleanup behavior, but it is not event-preservation evidence.

**Fact:** Embassy timer contracts are not uniform either. [`Timer`](https://github.com/embassy-rs/embassy/blob/f37b9b6bbf1d4540575d97582da7b4244ca4c202/embassy-time/src/timer.rs#L124-L140) retains an absolute expiry and deliberately returns `Pending` on its first poll even if already expired, scheduling a wake before a later `Ready`. Rebuilding an expired `Timer::at` every iteration while an earlier source wins can therefore prevent the timer from completing. [`Ticker::next`](https://github.com/embassy-rs/embassy/blob/f37b9b6bbf1d4540575d97582da7b4244ca4c202/embassy-time/src/timer.rs#L354-L356) separately documents cancellation safety. ESP-HAL one-shot timer construction can restart hardware. “Timer” is not one portable source contract; persistent absolute-deadline adapters and primitive-specific tests are required.

**Observation:** the earlier name `RestartSafeSource` collapsed four different properties:

- the reactor retains source state when another source wins;
- dropping an operation performs memory/resource-safe cleanup;
- dropping and reconstructing preserves logical progress/events;
- repeating after cancellation is externally safe or idempotent.

These properties are not synonyms. ESP-HAL's owning DMA transfer can retain a buffer and peripheral safely and clean them up on drop while a partial external transaction is still not logically restartable. Conversely, a Kittens reactor can retain a cancellation-unsafe waiter across **internal** source races without promising that destroying the entire reactor preserves it.

**Hypothesis:** polling a persistent source object directly can retain some otherwise cancellation-unsafe waiters across internal races. This helps only if the source already owns durable/armed state. Ordered polling stops at the first ready source, so a lower lazy waiter may not be polled at all in that arbitration; direct polling cannot preserve an edge for an interrupt that was never armed. K0 therefore admits only eager/latching or otherwise durable adapters. A future topology-conditional adapter that is safe only when it is guaranteed to be polled before every possible winner is deferred because that contract couples local source admission to global order and weakens diagnostics. The unresolved part is retaining a HAL waiter that borrows its peripheral without unsafe self-reference or hostile borrow errors; the disposable probe's ready slots did not test it.

**Recommendation:** use executor-neutral wording such as “selection-loss preserving” in the contract and diagnostics until agent trials choose a public trait name. The source admission trait proves only that reactor code chose a reviewed adapter and requested capabilities its type exposes; primitive evidence, review, and tests establish behavior. A generic ESP-HAL GPIO adapter remains deferred until an implementation can retain or latch the waiter without self-referential borrowing, or can isolate the interrupt behind an owned signal/channel.

### 20.4 Rendering, DMA, and framework boundaries

**Fact:** [`embedded-graphics::DrawTarget`](https://docs.rs/embedded-graphics/latest/embedded_graphics/draw_target/trait.DrawTarget.html) is a synchronous drawing contract and leaves device flushing to drivers. Its fixed framebuffer implementation uses const capacity and a compile-time size check. Kittens should compose with those guarantees, not reproduce pixel geometry or storage types.

**Fact:** [Slint's MCU integration](https://docs.slint.dev/latest/docs/rust/slint/docs/mcu/) requires the application platform to drive the outer timers/input/draw/sleep superloop. Slint owns its animation/dirtiness, UI dispatch, and renderer-side interpretation of the application-selected repaint-buffer mode. The application/HAL still waits for physical swap readiness and rotates buffer references correctly; choosing or maintaining the wrong history remains an application integration bug. Slint supports full-buffer, double-buffer, and line-buffer rendering, its MCU configuration currently requires a global allocator, and its API must not be called directly from interrupt handlers. Kittens may structure the outer platform orchestration, but it must not claim Slint or physical-buffer semantics and must not become a UI framework.

**Fact:** even the exact-V1 [`sh8601-rs::partial_flush`](https://github.com/theembeddedrustacean/sh8601-rs/blob/4bcddfd529017135f19a5a9a6e79dd6b8ef1b460/src/lib.rs#L694-L714) path allocates a temporary `Vec` with capacity proportional to the rectangle's pixel bytes. The driver is `no_std` but not universally no-alloc. “Partial” describes transfer geometry, not a fixed memory bound.

**Fact:** current embedded rendering exposes several incompatible completion models:

- synchronous blocking flush, as in `sh8601-rs` and the inspected 2.06 firmware;
- an owning DMA handle that consumes the buffer/peripheral and returns them on completion;
- front/back buffer swap completion;
- line-buffer credits while another line is in DMA;
- a later transport, TE, or scanout milestone.

In the inspected exact-V1 driver and nearby 2.06 firmware, flush is synchronous at the application boundary, so no application-visible DMA/write operation remains in flight after the blocking call returns; controller scanout/visibility may continue. With an ownership- or borrow-bearing API such as ESP-HAL's owning SPI DMA transfer, ordinary Rust prevents a second transfer through that same owned peripheral or mutation through the transferred buffer; rendering may overlap only into a different buffer/line for which the application still has ownership. Generic display-interface traits and arbitrary drivers do not establish that guarantee. Where the HAL provides it, it is a HAL/Rust win, not a Kittens win. Dirty rectangles, line buffers, buffer swaps, and full frames are renderer/driver storage strategies, not reactor source kinds.

**Observation:** Grok writer acknowledgement and asynchronous DMA completion share only a capacity-returning outline: demand may coalesce while submission capacity is occupied, and a later milestone restores eligibility. They do not share ticket ordering, one universal completion milestone, buffer count, visibility semantics, or failure behavior. “DMA complete,” “bus idle,” “controller RAM updated,” and “scanned out” are materially different.

**Hypothesis:** a later comparison may discover a reusable capacity-returning protocol below both systems, or may show that Grok tickets and ownership-returning display transfers should remain separate protocols. A shared API is justified only if it composes with both while improving misuse diagnostics over application state/HAL types.

**Recommendation:** keep the presenter/transfer protocol application-owned through K0. Do not promote the current generic `SingleFlight`, numeric tickets, or `Frame<Writable/Ready/InFlight>` into Kittens core. A later capacity-returning protocol comparison must name its completion milestone and beat ordinary HAL ownership/application state on misuse rejection, borrowing, and agent repair.

### 20.5 Task lifecycle, physical resources, and low power

**Fact:** Embassy tasks use statically allocated pools, accept `'static` arguments, return `()` or `!`, and [`Spawner::spawn`](https://github.com/embassy-rs/embassy/blob/f37b9b6bbf1d4540575d97582da7b4244ca4c202/embassy-executor/src/spawner.rs#L149-L166) returns no join or abort handle. Completion frees a static pool slot, but there is no public Tokio-like spawned-task join/abort lifecycle. Embassy `join` combines futures inside one task; it is not a spawned task handle.

**Recommendation:** the proposed cancel/grace/abort/drain `Scope` remains Tokio-specific. Do not hide this fracture behind executor generics. A future embedded lifecycle model would need static capacity, cooperative signals, and likely permanent tasks; it is a separate design.

ESP-HAL's safe singleton, exclusive-borrow, owning-DMA, alignment-wrapper, and buffer paths already express the exclusivity they actually model. Other HAL/driver APIs may not. Kittens should not wrap SPI/I2C/DMA ownership or generalize those conditional guarantees. Cross-resource timing—for example, an eight-second network await preventing touch service—belongs to architecture, handler policy, deterministic tests, and measurements.

Low power is likewise a system protocol. ESP-HAL [deep sleep does not return while light sleep resumes synchronously](https://github.com/esp-rs/esp-hal/blob/e1a042e3fa92839b157f72ef60b8db884156d067/esp-hal/src/rtc_cntl/sleep/mod.rs#L102-L186); its RAII wake locks can block automatic light sleep. The [esp-rtos idle path](https://github.com/esp-rs/esp-hal/blob/e1a042e3fa92839b157f72ef60b8db884156d067/esp-rtos/src/sleep.rs#L46-L136) also depends on ready tasks, wake locks, both cores, and the next timer. A source declaration cannot prove sleep entry or current draw. Kittens may help disarm an unnecessary frame source, but sleep eligibility and energy use require runtime/platform integration and hardware measurement.

### 20.6 Semantic classification

| Concept | Actual layer | Revised Kittens boundary |
|---|---|---|
| source ID and relation graph | proc-macro compile-time metadata | runtime-independent |
| `Control` and owned selected event | Rust/core values | runtime-independent, no allocation required |
| persistent source lifecycle and dormancy | executor-independent polling plus adapter state | core contract; implementation is adapter-specific |
| selection-loss preservation | executor-independent semantic obligation | admitted adapter contract, never inferred from `async` |
| lexical precedence and `last` | ordered `Poll` behavior | runtime-independent local arbitration |
| readiness/starvation classification | semantic adapter metadata plus graph analysis | runtime-independent analysis, trusted only for reviewed adapters |
| bounded per-item drain | generated repeated polling/handling | runtime-independent work bound; no collection implied |
| `before_poll` / `after_event` | generated control-flow positions | runtime-independent; bodies remain application-specific |
| timers, channels, cancellation tokens | runtime primitives | Tokio- or Embassy/HAL-specific adapters |
| task spawning and lifecycle scope | executor API | Tokio-specific current hypothesis; no shared scope promised |
| process, network, Tower, terminal | OS/Tokio/application integration | outside core |
| GPIO IRQ, DMA, PSRAM, wake locks | HAL/chip/platform integration | embedded-specific adapters or existing ownership |
| rendering dirtiness, frame visibility, UI state | application/UI framework/runtime protocol | outside reactor kernel |
| typestate and capability ownership | ordinary Rust semantics | runtime-independent but separately gated Kittens extensions |

This table is the smallest coherent common kernel the evidence supports. It is a constrained multi-source future compiler, not a universal runtime abstraction.

### 20.7 `no_std`, bounded memory, and dynamic states

**Recommendation:** `no_std` should shape the architecture now without making Embassy support a v0.1 deliverable. The `kittens` facade should be capable of a `no_std`, no-alloc reactor/source/`Control` base behind features; Tokio enables its integration modules. `kittens-macros` remains a normal host `std` proc-macro package while emitting target-core code. Do not create public `kittens-core`, `kittens-tokio`, and `kittens-embassy` packages before dependency pressure requires them; one facade with clearly named integration modules is easier for agents to infer.

`#[drain(max = N)]` bounds service work, not queue capacity and not memory. K0 processes each item immediately and allocates no batch. A future collected-batch API must require an explicit storage policy—caller buffer, fixed capacity/`heapless`, or alloc-backed—and must not make the same declaration silently allocate on one target. Queue capacity and drain maximum remain independent because a bounded producer can continuously refill a queue.

No allocation is not the same as small memory. An owned event variant or a large source can enlarge the generated future and task stack even when no heap symbol is linked. K0 must therefore measure the future/event-enum layout and the embedded fixture's stack/static footprint against a predeclared budget as well as checking allocator absence.

The embedded modes strengthen runtime dormant sources and state-transition testing, but they do not justify compile-time source-availability tables yet. Proving “frame timer is inactive while Off” would require the macro to own an exhaustive application state machine and every transition; a declaration alone would merely restate intent. Keep runtime guards/arming, add deterministic mode-transition scenarios and power measurements, and revisit state tables only if a real mutation can be rejected without invasive generic state plumbing.

### 20.8 Disposable executor-neutral polling probe

At the embedded review's explicit request, a disposable non-workspace crate tested the narrow feasibility claim on stable `rustc 1.96.0` and Cargo 1.96.0. It defined a `#![no_std]` source trait with a `Pin<&mut Self>` poll method and an ordered three-source selector using `core::future::poll_fn`.

**Fact:** all of these checks passed before the scratch directory was removed:

- `cargo check --lib --no-default-features`;
- no-std compilation for installed `wasm32-unknown-unknown`;
- execution as an ordinary future under Tokio 1.53.1;
- execution with `embassy-futures` 0.1.2;
- strict first/second/third lexical polling;
- a ready lower source retained its item after the earlier source won;
- separate struct-field source borrows ended before the handler mutated application state;
- no allocation, boxing, task spawn, runtime registry, or unsafe code in the selector.

**Not established:** proc-macro expansion, 23 arms, non-`Unpin` or self-borrowing sources, an actual ESP target, an ESP-HAL interrupt adapter, ownership-returning DMA completion, adversarial wake schedules, performance/code size, rust-analyzer behavior, or diagnostic quality. The probe raises confidence in architecture B below; it does not authorize its public trait shape.

The installed `wasm32-unknown-unknown` check is historical feasibility evidence, not a sufficient production `no_std` gate because that target has a distributed standard library available. K0 therefore requires a separate link on a stable bare-metal target plus dependency/symbol inspection; ESP32-S3 target compilation remains a later adapter gate.

The wake obligation remains ordinary `Future` law, not new runtime machinery: every enabled source reached by a poll and returning `Pending` must arrange for the current waker to be scheduled when it may make progress. If no source is ready, the ordered poller must reach all enabled sources. A deliberately dormant source need not self-wake while unchanged, but an external arming handle must update that source's retained state and wake the reactor. Merely waking does not resnapshot an application guard that was false when the arbitration began; external mode changes instead need an enabled event that completes the arbitration. The macro can generate complete polling; adapter tests and primitive contracts must establish race-free registration/check ordering.

### 20.9 Architecture comparison

| Candidate | Strength | Main cost/risk | Decision |
|---|---|---|---|
| A. Honest Tokio-only reactor | smallest implementation delta; exact Grok/Tokio familiarity; public semantics make no portability promise | embedded topology is out of scope and the common core is not tested | retain as K0 control/fallback, not the leading architecture |
| B. Runtime-neutral reactor core with Tokio/Embassy adapters | one ordered-poll semantic model, `no_std`, reviewed persistent adapters can retain state across internal selection losses, ordinary future runs on either executor | pin projection, source borrows, wake correctness, code size, and adapter contracts become Kittens' responsibility | **recommend and falsify first** |
| C. Shared syntax with runtime-specific expansion | uses each ecosystem's native selector | duplicate selection semantics and tests; Tokio/Embassy order, loss, guard, and drain behavior can diverge behind one syntax | reject unless B fails specifically on an unavoidable backend primitive |
| D. Tokio-semantic core behind a nominally portable facade, with a later embedded shim | lowest short-term work while preserving portable-looking names | hides backend-specific loss/guard/order semantics and risks promising portability that an adapter cannot repair | reject; Tokio-first release sequencing is acceptable only if the core semantics are tested independently |

Architecture B does not create a Kittens executor. It owns branch polling inside one future; the enclosing executor still owns tasks, wake queues, time, I/O, interrupts, and sleep. B must fall back to A or narrowly to C if the dual-fixture implementation requires unsafe self-referential storage, boxing/allocation solely for orchestration, unreadable expansion, materially worse code size/latency, or nonlocal borrow errors.

**Hypothesis:** B is the smallest strong architecture, not the proven final implementation. Its survival criterion is operational equivalence and better cross-runtime boundaries under the K0 falsifiers—not architectural elegance.

### 20.10 Embedded mutation benchmark

| Mutation | Correct enforcement layer | Honest expected result |
|---|---|---|
| continuously ready sensor precedes protected touch | Kittens graph/readiness check plus latency simulation | reject missing yield/reorder; runtime rate still measured |
| high-rate frame source remains armed while screen is Off | app runtime state plus deterministic mode test/power measurement | not a compile-time K0 win |
| direct ESP-HAL edge waiter is reconstructed after every lost race | Kittens adapter admission | reject arbitrary future; suggest retained/latching adapter or owned signal isolation |
| retained interrupt source is destroyed during reactor shutdown | adapter cleanup contract and runtime test | allowed only with explicitly documented event-loss boundary |
| optional/closed source self-wakes forever | source runtime invariant and deterministic test | transitions dormant; no busy loop |
| unbounded event drain | Kittens macro | compile-time reject for macro-managed drain |
| bounded queue is mistaken for bounded service | Kittens macro plus simulation | still require explicit drain maximum |
| long network/sensor await runs in input handler | runtime lint/test/measurement or refactor to task | topology cannot preempt it |
| second DMA transfer uses a safe API that consumes or exclusively borrows the peripheral/buffer | existing Rust/HAL ownership | compile failure for that selected API; not generalized to all drivers and not credited to Kittens |
| framebuffer is mutated while a safe DMA API exclusively owns/borrows it | existing Rust/HAL ownership | compile failure for that selected API; not generalized to all drivers and not credited to Kittens |
| generic async display-interface send is placed directly in the repeated race | Kittens adapter admission | reject absent a reviewed selection-loss-preserving adapter; `async` is not evidence |
| display submits twice in an application protocol not encoded by ownership | local app gate/runtime assertion | outside K0; candidate later comparison |
| comment says double-buffered but no swap occurs | behavioral/render test or API that consumes buffer roles | declaration alone has no value |
| DMA complete is treated as visible scanout | precise adapter contract plus TE/hardware test | not statically inferable |
| Stop is below timer but checked before next iteration | topology plus bounded-latency simulation | order alone does not define end-to-end Stop latency |
| dynamic deadline is not rearmed or is reset on every race | source runtime tests | persistent absolute deadline avoids reset; liveness remains dynamic |
| alloc-backed batch appears in no-alloc build | feature/type boundary and compile-fail fixture | core drain stays per-item/no-alloc |
| rendering occurs outside a declared phase through raw HAL access | outside constrained path | compiles unless a participating API requires a permit; K0 makes no global claim |

### 20.11 Comparative conclusion and revised confidence

The embedded case strengthens the **reactor-centered** thesis because interrupt races, dormant sources, biased selection, bounded service, dynamic cadence, phases, and starvation appear without Tokio. It weakens two overbroad conclusions: that the reactor must expand through Tokio, and that Grok's scope/render protocols are generic core abstractions.

It also supports a layered product direction: keep those common laws in one `no_std` semantic kernel, then place terminal, embedded hardware, agent, rendering, and lifecycle utilities in profiles that share the kernel but own their domain protocols. The profile/package split remains provisional; the semantic dependency direction is the stronger decision.

The two north-star fixtures should remain asymmetric:

- Grok is the full semantic fidelity and agent-repair benchmark for many-source desktop orchestration.
- ESP32-S3 UI is a smaller architectural counterexample and no-std/interrupt/ownership pressure fixture. It does not require an Embassy product backend in K0 and must not substitute nearby hardware for exact-board validation.

Readiness to begin the reversible kernel experiment rises modestly because executor-neutral ordered polling is no longer merely elegant speculation. Confidence in an exact public source trait and expansion drops until pinning and HAL waiters are tested. Confidence in a generic early rendering gate and cross-runtime scope drops materially. The subjective planning assessment is **90/100 ready to begin the reversible K0 slice** and **55/100 ready to freeze a public v0.1 API**. These are calibration scores for readiness to run the falsification experiment and freeze an API, not probabilities that architecture B will survive. Readiness to learn is high; readiness to publish remains lower.

## 20A. Cooperative scheduling budget: an unexamined confound in the expansion experiment

This section was added on 2026-08-07 after a review pass found that the entire report used "cooperative" only in the sense of *cooperative cancellation*, and never in the sense of Tokio's *cooperative scheduling budget*. That omission matters specifically because K0's central experiment is a behavioral-equivalence comparison between direct core polling and `tokio::select!`.

**Fact:** Tokio implements a per-task cooperative scheduling budget, exposed publicly at [`tokio::task::coop`](https://docs.rs/tokio/latest/tokio/task/coop/index.html). Its documented rationale is that "a single call to `poll` on a top-level task may potentially do a lot of work before it returns `Poll::Pending`." Tokio resource operations consume budget at their yield points.

**Fact:** [`coop::poll_proceed`](https://docs.rs/tokio/latest/tokio/task/coop/fn.poll_proceed.html) "decrements the task budget and returns `Poll::Pending` if the budget is depleted." The returned `RestoreOnPending` guard "will revert the budget to its former value when dropped unless `RestoreOnPending::made_progress` is called." The public surface also includes `has_budget_remaining`, `consume_budget`, and `unconstrained`, whose documentation warns that opting out "exposes your service to starvation if the unconstrained future never yields otherwise."

**Fact:** the budget is invisible in the per-primitive documentation an adapter author would consult. [`mpsc::Receiver::recv`](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Receiver.html) documents its cancel safety in detail and says nothing about budget exhaustion or yielding. An adapter reviewer reading only the primitive's page will not discover this behavior.

**Observation:** the consequence for Kittens is precise and narrow. A budget-aware Tokio source can return `Poll::Pending` *while an item is available*. Kittens' kernel vocabulary currently has no name for this state. It is not dormancy (the source is armed), not closure, and not selection-loss (nothing won). It is an executor-level backpressure signal that the kernel's readiness metadata cannot express and its starvation analysis does not model.

**Observation:** this directly stresses three K0 claims that would otherwise be measured as architecture defects:

1. **The equivalence oracle.** Section 20.2 of the specification requires that the core-poll form and the Tokio-select control "select the same source and produce the same wake-driven progress" on curated traces. The two forms do not necessarily consume budget identically. A drain window that handles up to 32 items through one adapter, and a `select!` control that re-enters the macro per item, can deplete the budget at different points and therefore diverge on *which* source is selected in a later arbitration — with no bug in either expansion. Attributing that divergence to the core-poll mechanism would falsify architecture B for the wrong reason.
2. **Bounded drain.** `drain(max = 32)` bounds Kittens-managed service work. It does not bound budget consumption, and a budget-exhausted immediate probe returns "empty" indistinguishably from a genuinely drained channel under the current contract. The service window can therefore terminate early for an executor reason the declaration never mentions.
3. **The no-std kernel boundary.** The budget is a Tokio-runtime concept with no Embassy or bare-metal counterpart. It is correctly outside the kernel; it is incorrectly outside the *adapter* contract, where it currently has no disclosure requirement.

**Observation:** this is an argument for adapter-level disclosure, not for kernel machinery. Kittens must not reimplement, defeat, or wrap the budget. `unconstrained` in particular would trade a measurement confound for a starvation hazard in exactly the firehose-versus-shutdown topology the project exists to protect.

**Hypothesis:** budget interaction is observable rather than merely theoretical at Grok scale. Grok's ACP branch drains up to 32 messages from a Tokio channel inside a 23-arm biased select; that is the shape most likely to reach a budget boundary in a single task poll. Whether it does so in practice is a measurement K0 can make cheaply and has not planned.

**Recommendation:** treat the budget as a named confound with three obligations. Each Tokio adapter states whether its polled operation consumes budget. The expansion experiment instruments budget state at arbitration boundaries so a divergence is classified before it is attributed. The equivalence oracle is qualified as holding under equal budget conditions, with unequal-budget divergence recorded as a finding about the executor boundary rather than a defect in either expansion. Kittens must not call `unconstrained` in generated code.

**Gap: does a 23-arm biased select with a 32-item drain actually exhaust a task budget under realistic Grok message rates? (no data exists — no reactor prototype has run)**

## 20B. Coverage model: how race and ordering defects are eliminated

This section was added on 2026-08-07 after a product-direction review posed the governing question directly: can this SDK, for an all-async embedded GUI/TUI harness, capture on the order of 99.9999% of the codebase's race conditions and ordering issues at the code and compilation level? The question is answerable, but only after "capture" is decomposed, because the honest answer is a layered claim, not a static-analysis claim.

**Observation:** exactly three mechanisms make a concurrency defect disappear, and their ceilings differ:

1. **Inexpressibility** — the bug cannot be written. Coverage of its class is ~100%, permanently, at zero diagnostic cost.
2. **Static detection** — the bug compiles into an error. Coverage is near-complete for *declared* topology and zero for undeclared intent; the boundary is measured by the specification's negative controls, which erase declarations and show that the macro cannot infer them back.
3. **Deterministic schedule exploration** — the bug is dynamic but reproducible under scripted schedules and paused time. Coverage is asymptotic and bounded by the scenario corpus; this is the io-sim/FoundationDB lesson from section 12.

Any credible high total-coverage figure is the *product* of these layers, and the dominant contributor is the least glamorous one: inexpressibility.

**Fact:** safe Rust guarantees the absence of data races but explicitly does not prevent general race conditions such as ordering and interleaving bugs ([Rustonomicon, "Data Races and Race Conditions"](https://doc.rust-lang.org/nomicon/races.html)). That documented seam — memory races eliminated, ordering races permitted — is exactly the boundary Kittens exists to move.

**Observation:** the single-reactor architecture is itself the largest coverage mechanism, and it is architectural rather than a check. One owner of application state, handlers that run to completion between arbitrations, and concurrency entering only through declared sources make intra-reactor logical shared-state races structurally inexpressible. Grok's loop already has this shape by convention; Kittens turns the convention into vocabulary with a compiler behind it.

The defect classes of the target system distribute across layers as follows:

| Defect class | Eliminating layer | Coverage character |
|---|---|---|
| memory-level data races | safe Rust ownership, `Send`/`Sync` | inexpressible; complete before Kittens exists |
| intra-reactor shared-state logical races | single-owner reactor architecture | inexpressible by construction within one reactor |
| arbitration ordering: shutdown prefix, precedence, `last`, starvation, unbounded macro drains, missing yields | `reactor!` static validation | complete for declared relations; undeclared intent is the measured boundary |
| selection-loss and cancellation races at sources | sealed admission plus reviewed adapters | complete on the constrained path; adapter truth remains audit plus lost-race tests |
| dormancy and hot-loop defects | adapter-owned dormant state | inexpressible on the supported path |
| interrupt/DMA reuse races | existing HAL/Rust ownership, composed not claimed | complete where the selected API is ownership-bearing |
| dynamic protocol races: stale acknowledgements, double in-flight submission | runtime protocol state plus deterministic tests | runtime-checked; inherently not static |
| handler-interior defects: long awaits, unchecked loops, rearm liveness | deterministic scenarios, latency oracles, watchdogs | residual class; unreachable by static means |
| external event order, hardware timing | none | bounded and measured, never proven |

**Observation:** total coverage is bounded by **escape surface**, not by check strength. Raw `tokio::spawn`, a raw select, an undeclared producer, a handler side channel, or a second executor task with a shared channel web all move behavior out of every layer above the residual ones. The inspected 2.06-inch watch firmware (section 20.2) distributes work across Embassy tasks; races *between* reactors in separate tasks are outside any current guarantee, and Embassy's task lifecycle remains deferred (section 20.5). One reactor per executor task is the high-coverage topology; every task split reintroduces channel discipline between reactors as an application obligation.

**Hypothesis:** with full vocabulary adoption — every producer behind an admitted source, every ordering intent a declared relation, state owned by the reactor, effects leaving through visible mechanisms — the first six classes approach structural completeness, and the residual concentrates in handler interiors, rearm liveness, and external order, which only the deterministic scenario layer and runtime oracles reach. A six-nines figure is therefore admissible only as this per-class, layered, falsifiable claim. Presenting it as static omniscience would be semantic theater at product scale, violating the same admission test the specification applies to individual declarations.

**Gap: no field baseline exists for the distribution of real-world async firmware/TUI race and ordering bugs across these classes (no data — the SPEC section 37.9 mutation corpus is a designed proxy, not a field study).**

**Recommendation:** encode the layered model as SPEC section 2.1 and leave K0 unchanged — it is already the falsification step for the static and admission layers. Re-weight the post-K0 extension queue: the deterministic scenario layer (SPEC section 21) and an escape-surface lint graduate first among extensions, because they bound the residual classes no static mechanism reaches, and the lint's product is a measured escape surface rather than a prohibition.

### 20B.1 Consumer expansion: harnesses that build harnesses, and engine authors

**Observation:** the coverage model implies two consumer tiers beyond the single-harness coding agent:

- **Meta-harnesses.** An agent harness that generates, hosts, and supervises other harnesses consumes Kittens twice: as the reactor substrate of its own loop and as the target vocabulary of the code it emits. For a generator, canonical spellings and machine-readable topology metadata are load-bearing rather than convenient — emission and verification are mechanical, so one spelling per operation and a stable declaration schema matter more than they do for an agent editing one file interactively. The topology descriptor (SPEC 21.4) gains a second consumer: not only replay tooling but the generator that must verify what it emitted.
- **Engine authors.** Next-generation rendering engines and async I/O engines need a declared-topology substrate beneath frame pacing, input pipelines, and device-completion handling. The research boundaries stand unchanged: Kittens is not a renderer, framebuffer, compositor, or I/O stack (sections 20.4, 20.5). The engine owns pixels, buffers, transports, and device protocols and builds them *on* the kernel's orchestration law through profiles; the kernel owns only the law.

**Recommendation:** record the tiers in SPEC section 3 and the profile direction in SPEC section 9.4. This changes no K0 scope. It strengthens the post-K0 case for the machine-readable topology descriptor and for profile APIs designed to be emitted by programs, not only written by agents.

## 21. Source and version ledger

Original entries were accessed 2026-08-06; the Grok and embedded sources were rechecked 2026-08-07.

| Area | Primary source | Version/status observed |
|---|---|---|
| Rust | [stable docs](https://doc.rust-lang.org/stable/), [1.85 async closures](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html) | docs 1.97.1; historical scratch toolchain 1.96.0 |
| Tokio | [crate docs](https://docs.rs/tokio/latest/tokio/), [changelog](https://docs.rs/crate/tokio/1.53.1/source/CHANGELOG.md) | 1.53.1, released 2026-07-20; Grok lockfile 1.52.3 |
| Tokio cooperative budget | [`task::coop`](https://docs.rs/tokio/latest/tokio/task/coop/index.html), [`poll_proceed`](https://docs.rs/tokio/latest/tokio/task/coop/fn.poll_proceed.html) | public API observed 2026-08-07; budget-depletion `Pending` is undocumented on the individual primitive pages |
| tokio-util | [crate docs](https://docs.rs/tokio-util/latest/tokio_util/) | 0.7.19, released 2026-07-21 |
| Tower | [crate docs](https://docs.rs/tower/latest/tower/) | 0.5.3, active |
| Cats Effect | [site](https://typelevel.org/cats-effect/), [releases](https://github.com/typelevel/cats-effect/releases) | 3.7.0, active |
| ZIO | [reference](https://zio.dev/reference/), [releases](https://github.com/zio/zio/releases) | 2.1.26, active |
| Haskell async | [Hackage](https://hackage.haskell.org/package/async) | 2.2.6, active release 2026 |
| Haskell STM | [Hackage](https://hackage.haskell.org/package/stm) | 2.5.3.1, maintained core concurrency library |
| ResourceT | [Hackage](https://hackage.haskell.org/package/resourcet) | 1.3.0 |
| effectful (Haskell) | [project](https://haskell-effectful.github.io/) | 2.6.1.0, active |
| Bluefin | [Hackage](https://hackage.haskell.org/package/bluefin) | 0.7.0.1, active |
| cap-std | [crate docs](https://docs.rs/cap-std/latest/cap_std/) | 4.0.2, released 2026-02-15 |
| futures-concurrency | [crate docs](https://docs.rs/futures-concurrency/latest/futures_concurrency/) | 7.7.1, active |
| Loom | [crate docs](https://docs.rs/loom/latest/loom/) | 0.7.2, mature |
| Shuttle | [crate docs](https://docs.rs/shuttle/latest/shuttle/) | 0.9.1, active |
| MadSim | [crate docs](https://docs.rs/madsim/latest/madsim/) | 0.2.34, active |
| Turmoil | [crate docs](https://docs.rs/turmoil/latest/turmoil/) | 0.7.2, active |
| RTIC | [project book](https://rtic.rs/) | 2.x line, active prior art |
| statig | [crate docs](https://docs.rs/statig/latest/statig/) | 0.4.1, active |
| selectme | [crate docs](https://docs.rs/selectme/latest/selectme/) | 0.7.2, active |
| Rust effect-rs | [crate source](https://docs.rs/crate/effect-rs/latest/source/README.md) | 0.1.0, alpha/very low adoption |
| Rust effectful | [crate docs](https://docs.rs/effectful/latest/effectful/) | 0.3.0, experimental/low adoption |
| Grok Build | [repository](https://github.com/xai-org/grok-build), [pinned commit](https://github.com/xai-org/grok-build/commit/393430ee4934bc791b0d538f304a21691c517433) | public monorepo sync, 2026-08-06 |
| Waveshare 1.8 hardware | [current docs](https://docs.waveshare.com/ESP32-S3-Touch-AMOLED-1.8), [pinned official repository](https://github.com/waveshareteam/ESP32-S3-Touch-AMOLED-1.8/tree/ba32b5cbca96f0e04b0736d04959b6e832268d3f) | V1 discontinued; V2 current from 2026-05-30 |
| Waveshare 1.8 BSP | [component 2.0.3](https://components.espressif.com/components/waveshare/esp32_s3_touch_amoled_1_8/versions/2.0.3/readme), [pinned C source](https://github.com/waveshareteam/Waveshare-ESP32-components/tree/9f4030c6e5cb888ad4cc268bfa7584c93ad53e30/bsp/esp32_s3_touch_amoled_1_8) | current first-party ESP-IDF component; inspected display path is CO5300 |
| Waveshare 1.8 Rust display | [`sh8601-rs`](https://docs.rs/sh8601-rs/latest/sh8601_rs/), [pinned source](https://github.com/theembeddedrustacean/sh8601-rs/tree/4bcddfd529017135f19a5a9a6e79dd6b8ef1b460) | 0.1.8; exact V1 display only, `no_std` with `alloc`, not a full board reactor |
| nearby Rust watch fixture | [`waveshare-watch-rs`](https://github.com/infinition/waveshare-watch-rs/tree/15c052ba2389a9a97bf68ae0135da0641a71b4dd) | 2.06-inch board, Embassy/no-std; not 1.8-compatible evidence |
| second nearby watch fixture | [`ESP32-S3-WATCH-rs`](https://github.com/QuackHack-McBlindy/ESP32-S3-WATCH-rs/tree/9cf0df918a2d013084f41946eb1c5fe11f53f4b4) | 2.06-inch CO5300/FT3168 only; distributed Embassy task fixture |
| Embassy | [pinned repository](https://github.com/embassy-rs/embassy/tree/f37b9b6bbf1d4540575d97582da7b4244ca4c202) | source snapshot 2026-08-07; `embassy-futures` scratch dependency 0.1.2 |
| ESP-HAL | [pinned repository](https://github.com/esp-rs/esp-hal/tree/e1a042e3fa92839b157f72ef60b8db884156d067), [GPIO docs](https://docs.rs/esp-hal/latest/esp_hal/gpio/struct.Input.html) | source snapshot 2026-08-07; async GPIO API explicitly cancellation-unsafe |
| embedded-hal-async | [pinned source](https://github.com/rust-embedded/embedded-hal/tree/41f29f6bfced1cae0cbe712ba96ee32c075b3125/embedded-hal-async) | 1.x trait contracts; no general cancellation guarantee |
| display-interface | [crate docs](https://docs.rs/display-interface/latest/display_interface/) | 0.5.0; sync/async transport traits, no general selection-loss contract |
| FT3x68 touch | [crate docs](https://docs.rs/ft3x68-rs/latest/ft3x68_rs/) | 0.1.2; FT3168/FT3268, synchronous driver, interrupt left to application |
| CST816S-family touch | [crate source/README](https://docs.rs/crate/cst816s/1.0.1/source/README.md) | 1.0.1; no-std blocking WIP, interrupt handling unfinished, no exact CST820/V2 validation |
| heapless | [crate docs](https://docs.rs/heapless/latest/heapless/) | 0.9.3; fixed-capacity storage candidate only, not a K0 dependency |
| embedded-graphics | [crate docs](https://docs.rs/embedded-graphics/latest/embedded_graphics/) | 0.8.x line; no-std synchronous drawing abstraction |
| Slint MCU | [current MCU integration docs](https://docs.slint.dev/latest/docs/rust/slint/docs/mcu/) | 1.17.x documentation observed 2026-08-07 |
