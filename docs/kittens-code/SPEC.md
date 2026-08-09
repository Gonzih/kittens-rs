# kittens-code specification

- Spec date: 2026-08-08
- Version: v0.8 — v0.7's targeted verification (input 17: 2 blockers,
  2 majors, rest fixed/nits) closed here: one log-appender unifies session
  and recovery appends with epoch-validation-before-repair; the IR is
  complete (typed outputs, By enum, range/escaping semantics); the value-
  cap-truncates vs meter-errors rule is now stated identically in P8, Q5,
  and Appendix A; the ledger matrix carries named enforcement layers with
  T4/T5/S4 dispositioned and G7e defined. Prior: v0.7 — v0.6 folded the
  external spec review (input 15, 8 blockers
  + 9 majors, corrected topology and call model adopted); the verification
  pass on v0.6 (input 16, FREEZE-AFTER-FIXES, converging: 8/17 FIXED) drove
  this version: canonical Commit-only append path with PersistFailed,
  whole-batch dispatch law, bounded paged continuations, self-contained
  typed verb semantics + IR variants, preview/authoritative event split,
  persisted crash-repair terminals, resume demoted from Op to startup mode,
  rename semantics, unknown-kind replay rules, literal import ledger with
  ledger→gate matrix, l3 dialect pinned + recorded. Prior lineage: v0.1 →
  v0.5 in git history of this file.
  **FROZEN 2026-08-09 by operator directive.** Implementation is authorized;
  the KC0 contract is stable. D2/D4 are closed to the implemented, tested
  types (§15) — the shipped `Op`/`Event`/`CoreInput`/`CoreAction`/
  `WindowLayout` shapes ARE the frozen contract. Freeze does not claim the
  implementation is complete or bug-free: the final release review (input 20)
  found correctness blockers (#3 appender torn-tail truncation, #4 lifecycle
  ledger validation ordering, #6 resume state reconstruction) that gate
  crates.io publication, plus deferred KC0 scope (#7 evidence gates, #8–#13).
  Freezing pins the interface so those fixes proceed against a stable contract
  rather than a moving one — "we can always improve, but stop re-speccing."
- Controlling evidence slice: section 14 (KC0) with its **exhaustive import
  ledger** (input 15 F14 — no more ranges). Everything not imported there is
  candidate design retained for lineage, per the root SPEC §37 discipline.
- Research basis: RESEARCH.md v3. Rules cite R§n / I-nn; unsourced items are
  marked *synthesis-introduced*.

**MUST/MUST NOT/SHOULD** are normative for KC0. T5 admission rule: any KC0
addition not in this spec needs a one-sentence recorded oracle in §15 first.

## 1. Scope and non-goals

kittens-code is a coding-agent harness crate family proving three falsifiable
bets (R§1) on the settled 2026 skeleton (R§3.1) under a hard portability
constraint (no_std+alloc core, effects at the boundary, R§7), with the
operator directives: rely on existing libraries, extend the kittens kernel
where needed, target MCU/WASM/bare-metal, clean structure over everything
(I-00 §10, loop directive).

Non-goals (v0.x): N1 TUI rendering (kittens-tui owns it; seam resolved by
conformance, I-14); N2 harness self-optimization; N3 write-side swarm
coordination; N4 new runtime/executor/rendering protocol; N5 interpreter in
core (Lua = post-KC0 driver-side escape hatch).

## 2. Terminology

- **Record** — one immutable framed entry in the transcript log: sequence
  number, kind, transaction/effect correlation, payload, checksum (§5).
- **Log** — the append-only record sequence for one session; rewind and
  compaction are markers/derived views, never rewrites.
- **Window** — the token-bounded message list sent to the model (L1).
- **Op / Event** — client→driver and driver→client *wire* messages
  (protocol crate). Events are derived from committed records only.
- **CoreInput / CoreAction / Transition** — the core's *internal* contract
  with drivers (§6): inputs in, bounded action batches out.
- **Effect** — a correlated, cancellable unit of driver-side work
  (`EffectId`), started/cancelled by CoreActions, reporting back as
  CoreInputs.
- **Driver** — the target-specific crate hosting the `kittens::reactor!`
  loop, owning tasks/sockets/processes keyed by `EffectId`.
- **Budget / Cap** — protocol-declared limits enforced in core via branded
  cap-types (§4 P6).
- **TurnEpoch** — monotonically increasing turn identity; every Effect and
  completion carries the epoch it belongs to.

## 3. Crate topology (normative for KC0; input 15 corrected topology adopted)

```
KC0 workspace members
  crates/kittens-code-protocol     wire types only; serde; no uuid/semver/
                                   checksum deps (ids/versions/digests are
                                   plain arrays/integers)
  crates/kittens-code-core         no_std+alloc; sans-io engine; built-in
                                   TOOLS ARE CORE MODULES (input 15 F1 —
                                   the separate tools crate is dissolved);
                                   WindowLayout + cap types live HERE, not
                                   in protocol (F3)
  crates/kittens-code-driver-tokio std driver: kittens[macros,tokio] reactor,
                                   effect dischargers (fs/exec/http/store),
                                   clocks, entropy, pumps (F2 rename)
  crates/kittens-code-cli          package kittens-code-cli, binary name
                                   `kittens-code`; links driver-tokio +
                                   protocol
  fixtures/code-no-std             link gate: protocol + core on
                                   thumbv7em-none-eabi + wasm32-unknown-unknown

Post-KC0 siblings (candidate; each a separate crate, never a generic
"driver-common" until a second driver proves shared code — F2):
  kittens-code-driver-web          named-host wasm (Fetch/Web Streams/
                                   IndexedDB-or-OPFS/WebCrypto); no Exec
  kittens-code-driver-wasi-p2      wasi:http, preopens, WASI clocks/random
  kittens-code-driver-embassy      reqwless/embedded-tls, flash store
  kittens-code-swarm               only after D16 + E4 design
```

Rules:

- T1. `protocol` and `core` MUST be `#![no_std]` + alloc and compile in the
  link-gate fixture from the first commit. Gate G1.
- T2. `core` MUST NOT depend on std, any runtime, HTTP/TLS, tokenizers,
  embedding models, wall-clock, or entropy. Anything IO-shaped is an Effect;
  the only synchronous seams are deterministic, bounded, memory-only (§11).
  Enforced by G1b (cargo-metadata feature-tree check), not convention.
- T3. Presentation/client modules and external frontends link `protocol`
  (+ transport) only; the in-process renderer (I-14) is an ordinary Event
  consumer. **The composition-root binary (kittens-code-cli) is exempt** —
  it links driver-tokio to wire the system together (input 16 N6); the rule
  binds everything that *consumes* the harness, not the root that assembles
  it. Gate G1b checks the non-root members.
- T4. New crates require a spec change (sprawl budget).
- T5. Admission ledger rule (header).
- T6. Non-Tokio drivers MUST depend on `kittens` with
  `default-features = false` (the kernel's `tokio` feature is default-on —
  input 15 F12).
- T7. L2 and swarm surface in core are **removable cargo features** (`l2`,
  `swarm-port`), off by default in KC0 builds (input 15 F16 condition 2).

Dependency policy (buy-over-build, I-00 §10, I-10): serde (+postcard
candidate), embedded-io-async vocabulary at driver seams, reqwless/
embedded-tls (embassy driver), mlua (post-KC0, driver-tokio feature),
model2vec-rs (E3 candidate), grep/regex-automata (D7). Build-from-scratch
stays limited to: the Vfs/Exec effect contracts (§9), the verb IR (§8), the
framed store codec (§5).

## 4. Protocol contract (wire only — slimmed per input 15 F3)

- P1. `Op` (client→driver): `user_input`, `interject`, `approve { id,
  verdict }`, `interrupt`, `config_patch(SessionConfigPatch)`, `shutdown`.
  Candidate: `fork`, `rewind_to`, swarm mount ops. Every Op gets a
  submission id (u64). **`resume` is not an Op** (input 16 R7): resume is a
  startup mode of the driver — open log, run crash-repair (S3), replay into
  a fresh core, seed epoch/seq counters from the log maxima, continue. No
  wire message exists for it.
- P2. `Event` (driver→client): turn lifecycle, model deltas (authoritative,
  plus optional `preview: true` pre-durability copies — L-A3), tool call
  proposed/started/output-delta/terminal, approval requests, budget updates,
  compaction started/applied/suppressed, RLM query trace, `ErrorEvent`.
  Authoritative Events are emitted only after the underlying records are
  durably acknowledged (L-A3). Unknown-kind rules (input 16 N4, replacing
  the v0.6 catch-all): **clients** preserve unknown Event kinds opaquely
  (raw bytes retained, never re-encoded lossily); **drivers reject unknown
  Ops** with `ErrorEvent { code: config_invalid }`-class refusal; unknown
  *state-bearing record kinds* in a log require a `schema_epoch` bump —
  replay never guesses their lifecycle (prevents false `aborted_by_crash`).
  Gate G2c: decode → replay → re-encode fixtures across an epoch bump.
- P3. Tool-call streams on the wire are `Started → OutputDelta* → exactly
  one Terminal`, property-tested (G7b).
- P4. `SandboxPolicy` / `ApprovalPolicy` are protocol data; mechanism is
  driver-side. Per-tool approval defaults + default SandboxPolicy live in
  SessionConfig (§13).
- P5. Config split (F3): **`SessionConfig`** — budgets, thresholds, prompt
  overrides, symbolic model-profile ids, approval/sandbox defaults —
  patchable via `config_patch`, every accepted patch is a logged record.
  **Bootstrap config** — endpoints, auth, TLS roots, store paths, preopens,
  flash partitions — is driver-only, never enters protocol or log.
- P6. Budgets are protocol *data* (numbers). Enforcement is core law via
  **kind-branded cap-types in core** (input 15 F10): `Capped<VerbOutput>`,
  `Capped<ToolResult>`, `Capped<AskDigest>` carry the applied runtime limit
  + truncation metadata, constructed only through core's truncating
  constructors under a compile-time hard ceiling; no unchecked `Deserialize`
  impl exists for them. Window-insertion APIs accept only cap-typed values.
  Gates G3 (trybuild bypass = compile fail) + G3b (property tests: runtime
  limits, aggregate meters, malicious decode).
- P7. Wire evolution: additive-only; unknown-tolerant decode (P2). Persisted
  compatibility uses an explicit **`schema_epoch: u32`** in the log header —
  not semver majors, which are all 0 during v0.x (F15).
- P8. Error taxonomy (D10): `ErrorEvent { class: Retryable | Fatal |
  UserActionable, code, message, correlates }`; codes: `model_transport`,
  `model_overloaded`, `model_context_length`, `model_auth`, `tool_denied`,
  `tool_failed`, `tool_timeout`, `budget_exhausted{budget_kind}`,
  `verb_error{verb, cause ∈ {bad_ref,bad_range,bad_flag,parse,budget}}`,
  `schema_incompatible`, `store_io`, `config_invalid`, `cancelled`,
  `internal`. Cap/meter rule (aligned with Q5, input 17 finding 4): value
  caps (verb output, tool result, ask digest) TRUNCATE and never raise an
  error; in-script *aggregate meter* hits (pages, bytes, subcalls,
  partitions) bind `verb_error{cause: budget}`; query- and turn-level
  budgets raise `budget_exhausted`.
- P9. Identity types (F3): `SessionId([u8;16])` — driver-generated from the
  entropy source, no UUID/wall-time dependency (MCUs have no trusted clock);
  `EffectId(u64)`, `TurnEpoch(u64)`, digests as `[u8;32]`.

## 5. Transcript store contract (records + durability; F9/F14 rewrite)

- S1. Append-only law: no delete/rewrite API exists on the store effect
  surface; rewind appends an elision marker; fork (candidate) copies with a
  parent pointer. Gate G9 (API audit + trybuild).
- S2. **Log payload = accepted Ops + emitted Events/effect outcomes** (F9
  correction of "the notification stream is the log"): config patches,
  approvals, and effect terminals are first-class records, so replay
  reconstructs config and approval state too. Derived caches are rebuildable;
  resume = replay.
- S3. Record framing (D15, F9): every record carries `seq: u64`,
  `kind`, `txn: Option<EffectId>`, `epoch: TurnEpoch`, payload, and a
  checksum whose coverage is exactly `seq..payload` (declared, tested).
  Streamed work is **individually framed** — `Started → Progress* → exactly
  one Terminal` sharing one `txn` — never buffered whole (F9). Replay of a
  stream with no Terminal gets a real `aborted_by_crash` terminal record
  APPENDED during startup repair (ordering in §6 append canon).
  A torn/checksum-failed tail is detected and cleanly ignored (G2 fixture).
  Atomicity is per-record; the *transaction* invariant is "no Terminal
  without its Started on replay", not multi-record atomic writes.
  **Crash repair is persisted (input 16 R9):** on open, the scanner APPENDS
  a real `aborted_by_crash` Terminal record for every incomplete
  transaction, THEN replays — so the durable-record-only publication rule
  (L-A3) holds for repair terminals too; nothing synthetic exists only in
  memory.
- S4. Codec: framed JSONL (one record per line + checksum field) in
  driver-tokio; postcard framing is the MCU candidate (D7-linked).
- S5. Crash discipline: per-record durable append with a driver-declared
  sync policy; `Persisted { up_to_seq }` CoreInputs report durability
  watermarks to core (§6).
- S6. Log header record: `{ session_id: SessionId, parent:
  Option<SessionId>, schema_epoch: u32, prompt_pack_version: [u16;3],
  verb_grammar_version: [u16;3], l3_dialect_version: [u16;3], codec,
  created_at: Option<driver-claimed-time> }`. Replay refuses a higher
  `schema_epoch` with `schema_incompatible`. One writer per log
  (create-exclusive/lock); multi-process append out of scope for v0.x.
- S7. Store *reads* are **effect-driven** (input 15 F5, unified per input 17
  finding 1): `StoreReadPage` and `StoreSearchPage` effects with paged,
  bounded results — no synchronous store port (IndexedDB/OPFS and flash are
  async; JSONL scans would block the driver). There is NO `StoreAppend`
  effect: all appends go through the single log-appender component behind
  `CoreAction::Commit` (§6 append canon). The RLM engine consumes pages via
  its continuation (§8).

## 6. Core contract: inputs, transitions, actions (input 15 F4/F6/F7)

The core is a synchronous sans-io state machine with an explicit,
re-entrancy-safe boundary:

```
CoreInput  = ClientOp(Op)
           | EffectProgress { id: EffectId, epoch: TurnEpoch, payload }
           | EffectFinished { id: EffectId, epoch: TurnEpoch, terminal }
           | Persisted { up_to_seq: u64 }
           | PersistFailed { at_seq: u64, error }
           | TimerFired { id: EffectId, epoch: TurnEpoch }
CoreAction = Commit(records)                    // THE append path (sole)
           | StartEffect { id, epoch, spec }    // http/model, sub-model,
                                                // vfs, exec, store READ/
                                                // SEARCH page, timer arm,
                                                // embed (l2)
           | CancelEffect { id }
Transition = { actions: bounded owned Vec<CoreAction> }   // no lazy iter
fn handle(&mut self, input: CoreInput) -> Transition
```

Append canon (input 16 R5/R9; unified with recovery per input 17 finding 1):
one **log-appender** component per driver owns the storage write path.
During a session, `Commit` is the only way records reach it; the appender
writes in strict `seq` order; success advances the `Persisted` watermark;
failure arrives as `PersistFailed`, which core treats as Fatal `store_io`
(drain and stop — a harness that cannot persist must not keep acting).
Startup ordering (also fixes input 17 finding 16): open log →
**validate `schema_epoch` FIRST** (refusal happens before any mutation; an
old binary never repairs an incompatible log) → scan → append
`aborted_by_crash` repair terminals **through the same appender** →
`Persisted` confirmation → replay into the fresh core. Appender failure at
startup = refuse to open. Every CoreInput that references an effect carries
its `TurnEpoch` (R4 — including `TimerFired`).

- L-A1. `handle` is never re-entrant: effect completions (including the
  synchronous test jail's) enter as queued CoreInputs through admitted
  reactor sources, never as recursive calls (F6).
- L-A2. Backpressure law: `Transition.actions` is bounded; the driver
  maintains bounded queues for SSE deltas, effect progress, and interjections
  (sizes = SessionConfig data); a full queue applies producer-side,
  cancel-aware waiting in the owning task — never inside `handle`. Max
  concurrent effects is a config bound. Kernel `#[drain]`/`#[yields_to]`
  govern service order; capacity is these queues (F6).
- L-A2b. Whole-batch dispatch law (input 16 R6): the driver reserves/stages
  capacity for an ENTIRE Transition before dispatching any of it, dispatches
  actions in order exactly once, and completes dispatch before the next
  `handle` call. A Transition is never partially applied across a `handle`
  boundary. Gate G11.
- L-A3. Event publication: the driver derives protocol Events from `Commit`
  records and publishes them only after the covering `Persisted` watermark
  (durable-ack rule, F4). The jail driver acks synchronously.
  **Streaming latency (input 16 N2):** model output deltas MAY additionally
  be published immediately as explicitly non-authoritative `preview: true`
  Events; authoritative Events follow the watermark and reference the same
  record ids, so clients reconcile by id. Drivers MUST bound flush latency
  with a bytes-or-millis sync policy — flush when EITHER threshold is
  reached first (declared in bootstrap config); the recorded tradeoff:
  durability beats display for authority, preview restores UX.
- L-T1. Turn law: sample → tool effects → resample; a model terminal with no
  tool calls ends the turn. Core owns `SessionState` and `TurnState { phase,
  epoch, pending_effects, call_order }`, RLM continuations, and the
  **exactly-once terminal ledger** per effect: first terminal wins; late or
  duplicate completions (stale epoch or already-terminal id) are dropped
  with a trace record (F7).
- L-T2. Interrupt: `interrupt` bumps nothing but marks the current epoch
  aborted; core emits `CancelEffect` for every pending id and a terminal
  abort record per open stream; a new turn = new `TurnEpoch`. `shutdown`
  additionally drains: core stops starting effects, driver joins/aborts
  handles, final records commit, reactor's terminal arm exits. Races
  (completion vs cancel) resolve by the terminal ledger. Gate G4b covers
  model, tool, sub-model, timer, and shutdown paths (F7).
- L-T3. Stationarity guard on identical consecutive tool calls (thresholds
  config data); no fixed iteration cap. Gate G7.
- L-T4. Tool execution: approval serial, execution concurrent, rejoin by
  call order; ordering-barrier tools (plan-exit class) start only after the
  prior batch's terminals. Gate G7c scenario.
- L-T5. Subagent spawning deferred (D8); protocol reserves shapes.
- L-D1. Driver law (normative): the `kittens::reactor!` loop lives in the
  driver; owned-task + funnel topology — interrupt/shutdown prefix arms,
  model-delta pump → admitted mpsc with `#[drain]` + `#[yields_to]`, ONE
  effect-completion funnel mpsc, interjection arm, timer deadline arm.
  Kernel-ask ledger (post-KC0): KX1–KX3 embassy adapters; **KX4 (corrected,
  input 15 F12): Web Promise/channel/timer wake-aware adapters and WASI
  Pollable adapters are required kernel work for the web/wasi drivers** —
  local `Latched`/`FixedQueue` cannot be armed from host callbacks; dynamic
  source sets + SSE adapter remain K1-era asks. None of these absences trips
  F-a (funnel pattern is the K0-conformant story).
- L-D2. A driver conformance suite (shared logical-topology tests, run per
  driver) is the growth path for future drivers — not a premature common
  crate (F2).
- L-D3. Jail determinism: the jail driver seeds clock, entropy, session ids,
  and retry jitter from the scenario file so "same scenario + same config ⇒
  byte-identical log" is actually achievable (F15). Gate G2.

## 7. Context engine (core)

- C1. Window is a derived view; compaction never deletes records.
- C2. Escalation: microcompact (age out stale tool results — Observation
  (leak) provenance, R§3.2) → full summarization into `WindowLayout` →
  mechanical drop-oldest; circuit breaker on deterministic failure.
- C3. Prefire scheduling (75%/85% config defaults) keyed by conversation
  fingerprint; delayed application; SelfCompact-style model-invoked
  compaction is a candidate E1 arm.
- C4. Startup/config content re-injected from source after compaction.
- C5. Reminders as blocks on user-role messages; managed static-prefix cache
  boundary.
- C6. RLM standing reminder each turn; wording = SessionConfig data.
- C7. Untrusted config-file content escaped at the injection boundary;
  peer-mounted content shares this boundary and carries taint (W2). Gate G8.
- C8. Token accounting is **core logic on input data** (input 15 F5 —
  TokenCount port deleted): provider-reported usage arrives in model effect
  terminals; the tail estimator is a self-calibrating byte ratio in core;
  G5 reports observed error bounds.
- C9. Prompt-pack: versioned data in core (system prompt, reminder
  templates, summary prompt), per-template overridable via SessionConfig;
  version recorded in S6.
- C10. `WindowLayout` is a **core type** (moved from protocol, F3): the
  typed post-compaction recipe with constructor-enforced tail atomicity
  (never splits a call from its terminal) and optional region labels. D4
  closes with its exact fields at freeze.

## 8. RLM engine (core; continuation model per input 15 F8/F13)

- Q1. Verb surface: `grep`, `slice`, `head`, `tail`, `count`, `partition`,
  `ask`, `ask-each`, `final` — one verb per line, `%N` result refs.
  **Status: versioned-experimental until E2 runs** (F13 downgrade from
  STABLE); version recorded in S6. The stability *goal* (future RL action
  space) stands, the freeze happens on E2 evidence.
- Q2. **Typed IR (F13; completed per input 17 finding 4):** every surface
  (verb text now; typed function calls and Lua later) lowers to one closed
  IR: `Query = [Instr]`, each `Instr` a closed enum variant with typed
  inputs AND a declared output type
  `Out = Records | Chunks | Count | Digest | DigestList | Answer`:
  - `Grep { pattern: Str, sel: Sel, ctx: u16, kind: Option<EventKind> } → Records`
  - `Slice { sel: Sel } → Records`
  - `Head { sel: Sel, n: u32 } → Records` / `Tail { … } → Records`
  - `Count { pattern: Option<Str>, sel: Sel } → Count`
  - `Partition { sel: Sel, by: By, size: Option<u32>, pattern: Option<Str> } → Chunks`
    where `By = Turns | Bytes | Regex`; `size` required for Turns/Bytes,
    `pattern` required for Regex (validated at lowering)
  - `Ask { sel: Sel, question: Str, sample_k: Option<u8> } → Digest`
  - `AskEach { chunks: Ref<Chunks>, question: Str } → DigestList`
  - `Final { value: Str | Ref<any> } → Answer`
  `Sel = Ref(%N) | Range | Whole` (Whole is the default when omitted).
  Range semantics: `unit:a..b` is inclusive `a`, exclusive `b`; units:
  `turn` = user-turn index, `seq` = record sequence, `byte` = offset in the
  store byte view. String escaping: `\"` and `\\` only; raw newlines are
  illegal inside strings (Appendix A grammar). Ref typing: a `Ref` must
  name an earlier line, and `AskEach.chunks` must reference a `Chunks`
  output. All validated at lowering (G7d). The EBNF is the text surface
  only; the IR is the semantic contract; E2 compares surfaces above it.
- Q3. Root-protection law: all RLM-originated data entering the root window
  is cap-typed; every tool result surfaced to the root is truncated to
  `Capped<ToolResult>` (head/tail excerpt + log-offset pointer; full output
  is in the log — reversible offload). Gate G3.
- Q4. Execution is a **core-owned continuation** (F8; state completed per
  input 16 R8): `QueryCont { query_id, pc, results: typed %N slots,
  page_cursor: Option<PageCursor>, fold: Option<FoldState>  // grep/count/
  partition accumulators, join: Option<JoinState>  // ask-each partition
  rejoin, pending: [EffectId], budgets: MeterSet }`. Verbs that need data
  emit paged store-read/search effects (S7); `ask` emits a sub-model effect
  and suspends; `ask-each` schedules incrementally (bounded parallel window)
  and rejoins by partition index. Nothing blocks inside `handle`.
  Terminal/discard rules: a query reaching `final`, erroring at query level,
  or being cancelled emits CancelEffect for its pending ids and drops its
  continuation with a trace record; turn interrupt discards all query
  continuations of that epoch the same way.
  **D2 effect identity/shape (freeze candidate):** the `recall` tool call's
  `EffectId` is the query id. Each continuation suspension gets a fresh child
  `EffectId` in the same `TurnEpoch`: `StoreReadPage { sel, cursor }` completes
  as `Pages(Page)`, and `SubModel { requests: [AskRequest] }` completes as
  `Ask([AskResult])`. KC0 emits one request per `SubModel` child effect so
  `ask-each` can rejoin out of order, `AwaitingMore` waits on already-started
  children, and every child has an independent exactly-once terminal. The
  vector payloads remain additive batch seams for a future driver optimization.
- Q5. Budget set (F8 + input 16 N1 — pending state actually bounded):
  per-verb output cap; verb-count cap per query (*synthesis-introduced*);
  recursion depth (default 1, economics rationale R§4.5); total subcalls
  per query; parallel subcalls; partition count; selected-bytes ceiling;
  **scanned-pages and scanned-bytes ceilings per query; total page-effects
  per query; per-`ask`-node wall-clock and token meters (the R§4.5 recursion
  budget, named here so Appendix A's charges are all Q5-defined); max
  simultaneous suspended queries per session; aggregate retained
  continuation memory ceiling per session** — all runtime limits under
  compile-time hard maxima (P6 pattern). All meters surface as protocol
  events. Meter charging split (input 16 N5): **value caps truncate** with
  metadata (`Capped<…>`) and never error; **aggregate/query meters error**
  (`verb_error{cause: budget}` in-script; `budget_exhausted` at query/turn
  level). P8's disambiguation rule says exactly this (updated with it).
- Q6. L3 search: `grep` over store search-page effects; the exact pattern
  dialect is **versioned and recorded in the S6 header** (`l3_dialect_version`
  — input 16 N7). KC0 pins dialect 1.0.0 = `regex` crate v1.x with default
  features, Unicode on, case-sensitive, inline flags (`(?i)` etc.)
  REJECTED at query validation, no backreferences/lookaround (inexpressible
  in the crate). no_std dialect closes with D7. Gate G12: search replay
  goldens over a fixed corpus.
- Q7. L2 ports (`Embedder`/`Similar`) behind the `l2` feature (T7), defined,
  unimplemented; index contract carries model fingerprint, dims, metric,
  quantization, chunker version, source hash, watermark, rebuild policy,
  stale-hint marking.
- Q8. `recall` tool packaging for the E1 tool-mediated arm. Its canonical
  function-tool argument is the JSON object `{ "script": <verb-text string> }`.
  Core lowers `script` before starting the continuation; malformed JSON or any
  lowered error binding resolves the call as a failed tool result containing
  `verb_error{verb,cause}`. A successfully lowered query runs under the session
  Q5 budgets and its `final` answer resolves the ordinary tool-result slot, so
  Q3 capping/offload and L-T1 resampling are reused without new wire types.
- Q9. Inline verb errors bind to `%N` + query trace record; no top-level
  ErrorEvent (P8 codes reused).

## 9. Tools (core modules — crate dissolved per input 15 F1)

- K1. KC0 set: `read`, `write`, `edit` (fuzzy fallback + unified diff),
  `grep` (workspace), `exec` (feature-gated), `apply_patch`
  (grammar-constrained, streaming parser). All results flow through Q3
  truncation. Gate G7.
- K2. Implementable contracts (input 15 F11): **Vfs effects** operate on
  normalized relative paths (no `..`, no absolute, symlink policy = refuse
  by default), bounded range reads, paged directory listings, atomic
  revision-aware writes (expected-generation compare). Rename semantics
  (input 16 R11): rename ALWAYS replaces within one mount — no no-replace
  mode in v0.x, callers wanting create-new use expected-generation writes;
  atomicity is a
  per-driver declared capability (atomic where the backend supports it,
  copy+delete fallback declared, never silent); cross-mount rename is
  refused; renaming through or onto a symlink is refused. **Exec effects**
  take argv (never a shell string), cwd, bounded env + stdin, deliver
  sequenced stdout/stderr progress records, exit status terminal, deadline,
  cancellation. Gate G7e (contract conformance tests per driver).
- K3. `SessionCapabilities` (F11): drivers declare at startup which effect
  families exist (exec: no on web/MCU); tool schemas are advertised to the
  model ONLY for capable families — a data variant compiling is not a
  capability.
- K4. COW checkpointing over Vfs: KC1 candidate. MCP/web/skills: post-KC0,
  outside core.

## 10. Swarm (candidate; unchanged gates, one added arm)

- W1. `ContextExchange` behind `swarm-port` feature: enumerate/mount/unmount
  read-only peer stores; resolve peer offsets. Novelty = uniform mount only
  (R§6 v3).
- W2. Deny-by-default scopes; peer content tainted (no tool authority).
- W3. E4 arms (F16 condition 5 complete): {cost-matched isolation,
  structured typed handoff, **centralized coordinator**, raw read-mount,
  filtered/snapshot mount} × task topologies.
- W4. D16 (retention/redaction/taint + correction records + provenance +
  **access revocation + stable read watermarks** (input 16 R16 c4) +
  enforcement oracle) blocks the swarm crate.

## 11. Effects and seams summary

Effects (driver-discharged, `EffectId`-correlated, cancellable): model call
(SSE), sub-model call, StoreReadPage/SearchPage (appends go through Commit,
not an effect — §6), Vfs ops, Exec,
timer arm/disarm, embed (l2). Synchronous core seams: none that touch IO —
window assembly, token estimation, verb lowering, and budget enforcement are
plain core logic (F5). Clock readings and entropy arrive as fields on
CoreInputs that need them, never ambient.

## 12. Model client (driver-tokio, KC0)

- M1. One wire dialect: Anthropic-style messages, streaming SSE (D6).
  The KC0 Tokio driver posts to `<endpoint-base>/v1/messages` with
  `x-api-key`, `anthropic-version: 2023-06-01`, JSON content type, and
  `stream: true`; the bootstrap profile supplies the exact model id and
  maximum output-token count, so neither is frozen into the library. The
  top-level `system` value concatenates the layout's non-empty `system`,
  `rules_reminder`, and reminder blocks. `user_info`, `summary`, and
  `last_user_query` form the leading user turn. Verbatim-tail messages use
  the core's canonical `[user] ` / `[assistant] ` prefixes (an untagged
  `TailItem::Message` is assistant text); tool call/result pairs lower to
  Anthropic `tool_use` / `tool_result` content blocks using a deterministic
  id derived from `EffectId`, with adjacent equal roles coalesced.
  `message_start` supplies input-token usage; text deltas and partial tool
  JSON are accumulated by content-block index until `message_stop`.
  Unknown SSE event and delta types are ignored (additive wire evolution),
  while malformed JSON, a dropped stream, or a missing `message_stop` is a
  transport failure. KC0 has no tool-schema registry: the request therefore
  omits `tools`, but the response parser still preserves any `tool_use`
  blocks an Anthropic-dialect endpoint emits. Outbound tool declaration is
  deferred until a registry has a controlling contract and gate.
- M2. Retry ladder + jitter + Retry-After + semantic retries + failure-count
  circuit breaker; cancel-aware sleeps. Gate G7 scenario. The driver uses a
  configurable bounded exponential policy over `(attempt, failure class,
  elapsed, Retry-After, jitter)`: transport failures, HTTP 429, HTTP 5xx,
  provider overload events, and stream drops retry; authentication,
  context-length/request-too-large failures, and other HTTP 4xx responses
  do not. `Retry-After` is a lower bound on the selected delay. Dropping the
  model future cancels the Tokio sleep and in-flight request. The breaker
  counts terminally failed model calls (not individual retry attempts),
  resets after success, opens at the configured consecutive-failure count,
  fails fast with the last terminal error class during a bounded cooldown,
  and permits a new call after cooldown.
- M3. Two model tiers (root, sub) as symbolic profile ids in SessionConfig,
  resolved to endpoints by bootstrap config (P5 split).

## 13. Frontends and configuration

- F1. KC0 headless: stdin/stdout protocol stream + JSONL event dump
  (`kittens-code-cli`). Each non-empty stdin line is one serialized `Op`;
  the composition root assigns monotonically increasing `SubmissionId`s,
  drives the runner to quiescence after each accepted op, and writes each
  newly published `Event` exactly once as one stdout line, flushing each
  line. Malformed input produces a non-persisted protocol error event and
  does not stop later lines. `Shutdown` is submitted and drained before the
  loop exits; EOF performs one final drain. Bootstrap precedence is CLI args
  over environment over documented defaults for the session log, workspace
  root, and model backend. The default backend is the deterministic
  `JailClient` loaded from a JSON scenario file and never opens the network;
  the `live` cargo feature admits explicit `LiveClient` selection with the
  provider API key and model id supplied only as bootstrap environment.
- F2. ACP adapter: candidate (driver-tokio). ACP *wire protocol* version
  (currently 1) is distinct from schema/package artifact versions.
- F3. kittens-tui seam: **resolved in shape by conformance** (I-14) — one
  shared driver reactor hosting harness + kittens-tui source families
  (wirings share Grok-fixture ancestry); renderer = unprivileged in-process
  Event consumer linking protocol + kittens-tui only. KC1 pins kittens-tui
  as its first external consumer and reports name friction before their API
  freeze. D-b is NOT a freeze prerequisite (input 15 F17 resolved).
- F4. SessionConfig (P5): TOML in driver; precedence defaults < file <
  config_patch (patches logged). Keys: compaction thresholds, stationarity,
  budgets (incl. Q5 set), prompt-pack overrides, model profile ids, approval
  defaults, default SandboxPolicy, queue bounds (L-A2). Unknown keys warn.
  Bootstrap config is separate and never logged.

## 14. KC0 — controlling evidence slice

**Import ledger (literal enumeration, input 16 R14):** T1, T2, T3, T4, T5,
T6, T7 (T7 = the features exist and are OFF in KC0 builds); P1, P2, P3, P4,
P5, P6, P7, P8, P9; S1, S2, S3, S4, S5, S6, S7; L-A1, L-A2, L-A2b, L-A3,
L-T1, L-T2, L-T3, L-T4, L-D1, L-D3; C1, C2, C3, C4, C5, C6, C7, C8, C9,
C10; Q1, Q2, Q3, Q4, Q5, Q6 (driver-tokio dialect 1.0.0 only), Q8, Q9; K1,
K2, K3; M1, M2, M3; F1, F4. **Not imported (dispositioned):** L-T5/D8
(subagents, post-KC0); L-D2 (driver conformance suite — activates with the
second driver, KC1); Q7 implementations (`l2` off); W1–W4 (`swarm-port`
off; crate gated on D16); K4; F2; F3 (KC1 pins kittens-tui); fork;
postcard codec (D7); all post-KC0 drivers.

Scope: (1) protocol+core under the link gate; (2) driver-tokio reactor
driving a real Anthropic-dialect endpoint and the seeded jail; (3) framed
JSONL store, resume-as-replay, crash-tail + incomplete-stream tolerance;
(4) context engine complete; RLM continuation engine complete at depth-1
defaults; (5) core tool modules with Vfs/Exec effect contracts and
SessionCapabilities; (6) eval rig: seeded jail + E1 with arms
{compaction-only, RLM-always-on, RLM-as-tool, both} × {reminder on/off}
on the D14 battery (eight tasks; tasks 2/5 RLM-arms-only with degraded
variants), **with preregistered falsifiers, thresholds, and cost/time/token
budgets recorded in the battery manifest before first implementation
results** (F16 condition 3); Terminal-Bench 2.0, E2, E3, and E4 are KC1+
gates whose manifests — same metric set, same ≥2-model-family rule, per-arm
falsifiers and budgets — are preregistered at KC0 close (input 16 R16).

Gates:

- G1. Link gate (thumbv7em + wasm32-unknown-unknown), CI.
- G1b. Structure gate: cargo-metadata check — core's dependency and feature
  tree matches T2/T6/T7; frontends link protocol only (T3).
- G2. Replay determinism, two distinct claims (input 16 N4): (a) fresh-run
  byte equality on the seeded jail (L-D3); (b) replay *state equivalence*
  when re-opening an existing log. Fixtures: schema-epoch bump refusal,
  torn tail, checksum corruption, incomplete stream → persisted
  `aborted_by_crash` repair record, config-patch precedence replay (F4).
  G2c: unknown-kind decode/replay/re-encode across an epoch bump (P2).
- G3. Budget law: trybuild cap-type bypass fails; adversarial oversized
  verb/tool/ask outputs truncate with metadata. G3b property tests
  (runtime limits, aggregate meters, malicious decode).
- G4. Interrupt observation within one service window (kernel-level).
- G4b. Cancellation matrix: in-flight model / tool / sub-model / timer /
  shutdown paths each produce correct terminal ledger state and abort
  records; duplicate/late completions dropped with trace (L-T1/T2).
- G5. E1 report per preregistered manifest incl. C8 estimator error bounds.
  Null results pass; unrun evals fail.
- G6. Compaction atomicity fuzz (window never splits call from terminal).
- G7. Tools + loop conformance: apply_patch parser fuzz + golden/rejection
  suites, edit fuzzy vectors, stationarity scenario, retry/breaker scenario.
  G7b: wire stream property tests (P3). G7c: ordering-barrier scenario
  (L-T4). G7d: verb text→IR golden + rejection + property tests (Q2).
- G8. Injection-escape adversarial fixture (C7).
- G9. Store immutability API audit (S1).
- G10. Real-endpoint smoke session, replayable offline afterward.
- G11. Boundary-law tests (L-A1/A2/A2b/A3): re-entrancy rejection,
  queue-full producer wait, whole-batch staging (a Transition is never
  split across `handle` calls), preview/authoritative reconciliation by
  record id.
- G7e. Vfs/Exec contract conformance tests, run per driver (K2).
- G12. L3 search goldens: fixed corpus, pinned dialect 1.0.0, byte-stable
  results (Q6).

Ledger→enforcement-layer→gate matrix (input 17 finding 6 — complete; layers:
**TY** type system, **API** API-surface absence, **CORE** core runtime
check, **DRV** driver protocol, **CI** structural CI check, **FIX** test
fixture/property, **REV** review-process law with no runtime gate):

| IDs | Layer | Gate |
|---|---|---|
| T1 | CI | G1 |
| T2, T3, T7 | CI | G1b |
| T4, T5 | REV | none — process law, enforced at spec/PR review by design |
| T6 | CI | G1b variant, activates with the first non-Tokio driver (KC1 conformance suite; recorded as deferred, not vacuous) |
| P1, P8, P9 | DRV/FIX | G2, G4b |
| P2, P7 | DRV/FIX | G2c |
| P3 | FIX | G7b |
| P4, P5 | DRV/FIX | G2 config-precedence fixture |
| P6 | TY/FIX | G3, G3b |
| S1 | API | G9 |
| S2, S5, S6 | DRV/FIX | G2 |
| S3 | DRV/FIX | G2 crash/torn/checksum fixtures |
| S4 | DRV/FIX | G2 (JSONL framing is the codec the fixtures exercise) |
| S7 | API/CI | G1b dependency audit (no sync store call sites) + G11 |
| L-A1, L-A2, L-A2b, L-A3 | DRV/FIX | G11 |
| L-T1, L-T2 | CORE/FIX | G4b |
| L-T3, L-T4 | CORE/FIX | G7, G7c |
| L-D1 | DRV + kernel law (KTR checks) | G4 |
| L-D3 | DRV/FIX | G2 |
| C1–C7, C9, C10 | CORE/FIX | G5, G6, G8 |
| C8 | CORE | G5 (error bounds) |
| Q1, Q2 | CORE/FIX | G7d |
| Q3 | TY | G3 |
| Q4, Q5 | CORE/FIX | G11 meters + G5 |
| Q6 | DRV/FIX | G12 |
| Q8, Q9 | CORE/FIX | G5, G7d |
| K1 | CORE/FIX | G7 |
| K2, K3 | DRV/FIX | G7e |
| M1, M3 | DRV | G10 |
| M2 | DRV/FIX | G7 retry scenario |
| F1, F4 | DRV/FIX | G2 fixtures |

Falsifiers: F-a (funnel/topology inexpressible in reactor macro without raw
selection escape — ledgered kernel absences excluded); F-b (>5% hot-path
overhead vs std-native control — E5, KC1); F-c (cap-type law unmaintainable
without interpreter-grade parsing).

## 15. Decisions register

| ID | Decision | Status | Blocking |
|---|---|---|---|
| D1 | Topology §3 (tools dissolved into core; driver-tokio/cli naming) | set for KC0 | freeze after KC0 |
| D2 | Exact Op/Event/CoreInput/CoreAction field shapes | **CLOSED 2026-08-09**: the implemented, tested types in `kittens-code-protocol` (op.rs/event.rs) and `kittens-code-core` (engine.rs) are the frozen contract; additive-only within v0.x (P7) | — |
| D3 | Verb grammar + IR (§8, appendix A) | closed as KC0 draft; grammar versioned-experimental | E2 |
| D4 | WindowLayout exact fields (core type) | **CLOSED 2026-08-09**: the implemented `WindowLayout` in `kittens-code-core` (window.rs) is the frozen shape; the #11/#14 constructor hardening is a bugfix within the frozen field set, not a shape change | — |
| D5 | Isolation policy enum | deferred with D8 | post-KC0 |
| D6 | Wire dialects (Anthropic-style only) | closed for KC0 | KC1 |
| D7 | no_std search dialect × postcard codec (+ `l3_dialect_version`) | open, spike | MCU claims |
| D8 | Subagent tool | deferred | post-KC0 |
| D-a | Token accounting = core logic on provider usage + calibrated ratio (C8; port deleted) | resolved | — |
| D-b | kittens-tui seam | **resolved in shape (I-14)**; KC1 pins as first consumer | KC1 only |
| D-c | ESP32-S3 full-system spike (endpoint, DNS, verified TLS via rustpki, streaming, persistence, RAM/power) | scheduled | MCU claims |
| D10 | Error taxonomy (P8) | closed as KC0 draft | verification |
| D11 | Header shape (S6: schema_epoch, [u8;16] ids, version triples) | closed as KC0 draft | verification |
| D12 | SessionConfig/bootstrap split + keys (P5, F4) | closed as KC0 draft | verification |
| D13 | Prompt-pack in core (C9) | closed as KC0 draft | verification |
| D14 | Jail interface + seeded determinism + E1 battery + preregistration | closed as KC0 draft | verification |
| D15 | Record framing/durability (S3–S5) | closed as KC0 draft | verification |
| D16 | Retention/redaction/taint + correction records + provenance oracle | open | swarm crate |
| KX1–3 | embassy channel/signal/deadline kernel adapters | ledgered | MCU runtime |
| KX4 | Web Promise/channel/timer + WASI Pollable wake-aware kernel adapters (corrected from "none needed") | ledgered | web/wasi drivers |
| KX-K1 | dynamic source sets + SSE kernel adapter | ledgered | post-KC0 |

## Appendix A — verb text surface (lowering target: §8 Q2 IR)

```ebnf
script    = line , { line } ;
line      = verb , { arg } , newline ;
verb      = "grep" | "slice" | "head" | "tail" | "count" | "partition"
          | "ask" | "ask-each" | "final" ;
arg       = flag | value ;
flag      = "--" , ident , [ "=" , value ] ;
value     = ref | range | number | string | ident ;
ref       = "%" , digit , { digit } ;
range     = unit , ":" , number , ".." , number ;
unit      = "turn" | "seq" | "byte" ;
string    = '"' , { character - '"' - newline | '\"' } , '"' ;
ident     = letter , { letter | digit | "-" | "_" } ;
number    = digit , { digit } ;
letter    = "a".."z" | "A".."Z" ;  digit = "0".."9" ;
character = ? any UTF-8 scalar value ? ;
```

Semantics table (self-contained, input 16 N5; every verb reads a `Sel` =
`%N` ref, range, or whole target; every line's capped output binds `%N`;
meters charged per Q5 — value caps truncate, aggregate meters error):

| Verb | Typed form (Q2 IR) | Output | Meters charged |
|---|---|---|---|
| `grep` | `Grep{pattern, sel, ctx, kind}` | matching records + context; binds hit-selection | scanned-pages/bytes, page-effects, verb output cap |
| `slice` | `Slice{sel}` | records in selection | scanned-pages/bytes, verb output cap |
| `head`/`tail` | `Head/Tail{sel, n}` | first/last n records | scanned-pages/bytes, verb output cap |
| `count` | `Count{pattern?, sel}` | count (engine-side aggregation) | scanned-pages/bytes, page-effects |
| `partition` | `Partition{sel, by, size?, pattern?}` | chunk-list selection (partition-count meter bounds it) | scanned-pages/bytes, partition count |
| `ask` | `Ask{sel, question, sample_k?}` | sub-model digest (`Capped<AskDigest>`) | selected-bytes, total/parallel subcalls, recursion depth, per-ask wall-clock/token meters (Q5) |
| `ask-each` | `AskEach{chunks, question}` | per-chunk digests, rejoined by index; concatenation is verb-output-capped | selected-bytes, total/parallel subcalls, partition count |
| `final` | `Final{value}` | terminates query; literal or `%N` is the answer | — |

Inline errors bind to `%N` + query trace record, no top-level ErrorEvent
(Q9). Arity/type/duplicate/forward-ref validation at lowering (G7d).

## 16. Lineage

- 2026-08-08: v0.1→v0.5 (see prior entries in git history of this file).
- 2026-08-08: v0.6 — external spec review (input 15) folded in full:
  corrected topology (tools into core; driver-tokio/cli; post-KC0 driver
  siblings), CoreInput/CoreAction/Transition boundary with durable-ack event
  publication, effect-driven store (sync ports eliminated; TokenCount
  deleted), re-entrancy/backpressure law, core-owned turn/cancellation state
  with exactly-once terminal ledger, RLM continuation model with the full
  budget set, framed streaming records reconciling S2/S7, branded runtime
  cap-types, implementable Vfs/Exec contracts + SessionCapabilities,
  KX4 corrected (web/wasi kernel adapters required; input 10 annotated),
  typed verb IR + versioned-experimental grammar, exhaustive KC0 import
  ledger, gates G1b/G2-fixtures/G3b/G7b-d added, jail seeding, schema_epoch,
  E4 coordinator arm, eval preregistration, L2/swarm as removable features,
  D-b unified as resolved (I-14). RESEARCH v3 stale text corrected same
  commit (95×, 16KB, prior-art phrasing, "nearly free").
- 2026-08-08: v0.7 — input 16's required-before-freeze list applied in full.
  RESEARCH updated in the same commit: §6 five-family list, resolved
  kittens-tui seam.
- 2026-08-08: v0.8 — input 17 (targeted verification of v0.7) closed:
  append/recovery unification behind one appender with
  epoch-validation-before-repair; complete typed IR; cap/meter rule made
  consistent across P8/Q5/Appendix A; full ID→layer→gate matrix; wording
  nits (per-session aggregate, always-replace rename, either-threshold
  flush, exact regex pin with inline-flag rejection).
- 2026-08-08: v0.8 implementation clarification — Q4/Q8 freeze candidates
  now fix `recall`'s JSON argument, query/child effect identity, singleton
  sub-model child effects, and existing tool-result/cap/ledger reuse. This
  closes review input 19 finding #6 without changing protocol wire shapes.
- Next: close D2/D4 exact shapes; operator review; freeze KC0 sections.
  The Codex review cycle is concluded at input 17 — remaining items are
  drafting (D2/D4) and human judgment, not review findings. Implementation
  remains unauthorized until freeze.
