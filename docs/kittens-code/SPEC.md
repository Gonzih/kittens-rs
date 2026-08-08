# kittens-code specification

- Spec date: 2026-08-08
- Version: v0.6 — full architectural revision folding the external spec review
  (input 15, Codex gpt-5.6-sol ultra, verdict FREEZE-AFTER-FIXES: 7 blockers,
  9 majors — every finding dispositioned; the corrected topology and call
  model below are the review's, adopted). Prior lineage: v0.1 draft → v0.2
  (input 11) → v0.3 (D3/D10–D14) → v0.4 (input 12) → v0.5 (input 13
  conditions) → this.
  **Not yet frozen, no implementation authorized.** Freeze requires: operator
  review + closure of D2/D4 exact shapes (§15) + one Codex verification pass
  confirming the blocker fixes.
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
- T3. Frontends and external clients link `protocol` (+ transport) only.
  The in-process renderer (I-14) is an ordinary Event consumer. Gate G1b.
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
  verdict }`, `interrupt`, `resume`, `config_patch(SessionConfigPatch)`,
  `shutdown`. Candidate: `fork`, `rewind_to`, swarm mount ops. Every Op gets
  a submission id (u64).
- P2. `Event` (driver→client): turn lifecycle, model deltas, tool call
  proposed/started/output-delta/terminal, approval requests, budget updates,
  compaction started/applied/suppressed, RLM query trace, `ErrorEvent`.
  Events are emitted **only after the underlying records are durably
  acknowledged** (§6 L-D3). Unknown *fields* AND unknown *enum kinds* must
  be tolerated by decoders (F15): every enum is `#[non_exhaustive]`-shaped
  on the wire with an explicit `unknown` catch-all rule.
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
  `internal`. In-script verb cap hits bind `verb_error{cause: budget}`;
  `budget_exhausted` is query/turn level.
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
  stream with no Terminal derives a synthetic `aborted_by_crash` terminal.
  A torn/checksum-failed tail is detected and cleanly ignored (G2 fixture).
  Atomicity is per-record; the *transaction* invariant is "no Terminal
  without its Started on replay", not multi-record atomic writes.
- S4. Codec: framed JSONL (one record per line + checksum field) in
  driver-tokio; postcard framing is the MCU candidate (D7-linked).
- S5. Crash discipline: per-record durable append with a driver-declared
  sync policy; `Persisted { up_to_seq }` CoreInputs report durability
  watermarks to core (§6).
- S6. Log header record: `{ session_id: SessionId, parent:
  Option<SessionId>, schema_epoch: u32, prompt_pack_version: [u16;3],
  verb_grammar_version: [u16;3], codec, created_at:
  Option<driver-claimed-time> }`. Replay refuses a higher `schema_epoch`
  with `schema_incompatible`. One writer per log (create-exclusive/lock);
  multi-process append out of scope for v0.x.
- S7. Store access is **effect-driven** (input 15 F5): `StoreAppend`,
  `StoreReadPage`, `StoreSearchPage` effects with paged, bounded results —
  no synchronous store port (IndexedDB/OPFS and flash are async; JSONL scans
  would block the driver). The RLM engine consumes pages via its
  continuation (§8).

## 6. Core contract: inputs, transitions, actions (input 15 F4/F6/F7)

The core is a synchronous sans-io state machine with an explicit,
re-entrancy-safe boundary:

```
CoreInput  = ClientOp(Op)
           | EffectProgress { id: EffectId, epoch, payload }
           | EffectFinished { id: EffectId, epoch, terminal }
           | Persisted { up_to_seq: u64 }
           | TimerFired { id: EffectId }
CoreAction = Commit(records)                    // append to log
           | StartEffect { id, epoch, spec }    // http/model, sub-model,
                                                // vfs, exec, store page,
                                                // timer arm, embed (l2)
           | CancelEffect { id }
Transition = { actions: bounded owned Vec<CoreAction> }   // no lazy iter
fn handle(&mut self, input: CoreInput) -> Transition
```

- L-A1. `handle` is never re-entrant: effect completions (including the
  synchronous test jail's) enter as queued CoreInputs through admitted
  reactor sources, never as recursive calls (F6).
- L-A2. Backpressure law: `Transition.actions` is bounded; the driver
  maintains bounded queues for SSE deltas, effect progress, and interjections
  (sizes = SessionConfig data); a full queue applies producer-side,
  cancel-aware waiting in the owning task — never inside `handle`. Max
  concurrent effects is a config bound. Kernel `#[drain]`/`#[yields_to]`
  govern service order; capacity is these queues (F6).
- L-A3. Event publication: the driver derives protocol Events from `Commit`
  records and publishes them only after the covering `Persisted` watermark
  (durable-ack rule, F4). The jail driver acks synchronously.
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
- Q2. **Typed IR (F13):** every surface (verb text now; typed function
  calls and Lua later) lowers to one closed IR — `Query = [Instr]`,
  `Instr { op, args: [Value], out: RefSlot }`, `Value = Ref(%N) | Range |
  Str | Num | Flag` — with arity, ref-type, duplicate-flag, and
  forward-reference validation at lowering. The EBNF (appendix A) is the
  text surface's grammar; the IR is the semantic contract; E2 compares
  surfaces above the same IR.
- Q3. Root-protection law: all RLM-originated data entering the root window
  is cap-typed; every tool result surfaced to the root is truncated to
  `Capped<ToolResult>` (head/tail excerpt + log-offset pointer; full output
  is in the log — reversible offload). Gate G3.
- Q4. Execution is a **core-owned continuation** (F8): `QueryCont { query_id,
  pc, results: typed %N slots, pending: [EffectId], budgets }`. Verbs that
  need data emit paged store effects (S7); `ask` emits a sub-model effect
  and the continuation suspends until its terminal CoreInput; `ask-each`
  schedules incrementally (bounded parallel window) and rejoins by partition
  index. Nothing blocks inside `handle`.
- Q5. Budget set (F8 — fan-out actually bounded): per-verb output cap;
  verb-count cap per query (*synthesis-introduced*); recursion depth
  (default 1, economics rationale R§4.5); **total subcalls per query;
  parallel subcalls; partition count; selected-bytes ceiling**. All meters
  surface as protocol events.
- Q6. L3 search: `grep` over store search-page effects; the exact pattern
  dialect is **versioned** (`l3_dialect_version`, part of D7) — KC0 pins the
  driver-tokio dialect (full regex) and records it; no_std dialect closes
  with D7.
- Q7. L2 ports (`Embedder`/`Similar`) behind the `l2` feature (T7), defined,
  unimplemented; index contract carries model fingerprint, dims, metric,
  quantization, chunker version, source hash, watermark, rebuild policy,
  stale-hint marking.
- Q8. `recall` tool packaging for the E1 tool-mediated arm.
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
  revision-aware writes (expected-generation compare), declared rename
  semantics. **Exec effects** take argv (never a shell string), cwd, bounded
  env + stdin, deliver sequenced stdout/stderr progress records, exit
  status terminal, deadline, cancellation.
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
  enforcement oracle) blocks the swarm crate.

## 11. Effects and seams summary

Effects (driver-discharged, `EffectId`-correlated, cancellable): model call
(SSE), sub-model call, StoreAppend/ReadPage/SearchPage, Vfs ops, Exec,
timer arm/disarm, embed (l2). Synchronous core seams: none that touch IO —
window assembly, token estimation, verb lowering, and budget enforcement are
plain core logic (F5). Clock readings and entropy arrive as fields on
CoreInputs that need them, never ambient.

## 12. Model client (driver-tokio, KC0)

- M1. One wire dialect: Anthropic-style messages, streaming SSE (D6).
- M2. Retry ladder + jitter + Retry-After + semantic retries + failure-count
  circuit breaker; cancel-aware sleeps. Gate G7 scenario.
- M3. Two model tiers (root, sub) as symbolic profile ids in SessionConfig,
  resolved to endpoints by bootstrap config (P5 split).

## 13. Frontends and configuration

- F1. KC0 headless: stdin/stdout protocol stream + JSONL event dump
  (kittens-code-cli).
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

**Import ledger (exhaustive, input 15 F14):** T1–T7; P1–P9; S1–S7; L-A1–A3,
L-T1–T4, L-D1, L-D3; C1–C10; Q1–Q6 (Q6 driver-tokio dialect only), Q8, Q9;
K1–K3; M1–M3; F1, F4. Explicitly NOT imported: L-T5/D8 (subagents), Q7/T7
features on (l2, swarm-port stay off), W1–W4, K4, F2, F3 (KC1), fork,
postcard codec, all post-KC0 drivers.

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
results** (F16 condition 3); Terminal-Bench 2.0 and E2 are KC1 gates with
their manifests preregistered at KC0 close.

Gates:

- G1. Link gate (thumbv7em + wasm32-unknown-unknown), CI.
- G1b. Structure gate: cargo-metadata check — core's dependency and feature
  tree matches T2/T6/T7; frontends link protocol only (T3).
- G2. Replay determinism on the seeded jail (byte-identical log; L-D3);
  fixtures: schema-epoch bump refusal, torn tail, checksum corruption,
  incomplete stream → `aborted_by_crash`.
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

Falsifiers: F-a (funnel/topology inexpressible in reactor macro without raw
selection escape — ledgered kernel absences excluded); F-b (>5% hot-path
overhead vs std-native control — E5, KC1); F-c (cap-type law unmaintainable
without interpreter-grade parsing).

## 15. Decisions register

| ID | Decision | Status | Blocking |
|---|---|---|---|
| D1 | Topology §3 (tools dissolved into core; driver-tokio/cli naming) | set for KC0 | freeze after KC0 |
| D2 | Exact Op/Event/CoreInput/CoreAction field shapes | open — **must close at freeze** (F14), draft shapes in §4/§6 | freeze |
| D3 | Verb grammar + IR (§8, appendix A) | closed as KC0 draft; grammar versioned-experimental | E2 |
| D4 | WindowLayout exact fields (core type) | open — **must close at freeze** (F14) | freeze |
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

Verb semantics as in v0.5 §8.1 (selection model, `%N` binding, per-verb caps,
`slice` accepts `%N` or range, `ask-each` bounded by Q5's limit set, `final`
takes literal or `%N`, inline errors per Q9) — now defined over the IR;
arity/type/duplicate/forward-ref validation happens at lowering (G7d).

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
- Next: Codex verification pass on v0.6 blocker fixes; close D2/D4 exact
  shapes; operator review; freeze KC0 sections. Implementation remains
  unauthorized until freeze.
