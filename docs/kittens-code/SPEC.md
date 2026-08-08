# kittens-code specification

- Spec date: 2026-08-08
- Version: v0.2 — spec refinement pass 1 applied (adversarial review, input 11:
  3 blockers, 5 major, 6 minor — all dispositioned below) plus the operator's
  extend-the-kernel directive and the kernel-fit analysis (input 10).
  **Not yet frozen, no implementation authorized.**
- Controlling evidence slice: section 14 (KC0). Sections 3–13 are normative for
  KC0 only where section 14 imports them; everything else is candidate design
  retained for lineage, mirroring the root SPEC's §37 discipline.
- Research basis: every design decision cites a RESEARCH.md section (R§n) or a
  pinned input (I-nn); items with no source are marked *synthesis-introduced*.
  Open decisions live in section 15.

Label discipline follows RESEARCH.md. **MUST/MUST NOT/SHOULD** are normative
for KC0; "candidate" marks post-KC0 surface that must not be built before its
gate. Rule T5 (admission ledger, imported from root SPEC §37.4 discipline):
any KC0 addition not present in this spec requires a one-sentence recorded
oracle/justification in the decisions register before it is built.

## 1. Scope and non-goals

kittens-code is a coding-agent harness crate family. It exists to prove three
falsifiable bets (R§1) on top of the settled 2026 harness skeleton (R§3.1):
RLM-native context law (R§4), tiered context (R§5), swarm read-mounts (R§6) —
under a hard portability constraint (no_std+alloc core, virtual IO/FS, R§7),
with the operator's standing directive (I-00 §10): rely on existing libraries,
extend the kittens kernel where the harness needs it, target MCU/WASM/bare-metal.

Non-goals for the entire v0.x line:

- N1. TUI rendering. Owned by `kittens-tui` (separate harness). kittens-code
  exposes only the protocol event stream (R§8.1); seam freezes after
  negotiation (D-b).
- N2. Harness self-optimization (R§9 Q10).
- N3. Write-side swarm coordination (task lists, inboxes) (R§6).
- N4. A new async runtime, executor, or rendering protocol.
- N5. Interpreter in the core. Lua lives in the std shim only (R§4.4), post-KC0.

## 2. Terminology

- **Record** — one immutable, typed entry in the transcript log (protocol event
  envelope + monotonic sequence number + logical timestamp).
- **Log** — the append-only record sequence for one session. Never truncated or
  rewritten; rewind and compaction are markers/derived views (R§3.1-3).
- **Window** — the token-bounded message list actually sent to the model (L1).
- **Verb** — one line of the RLM query surface (section 8).
- **Port** — a trait the core depends on for *data access* (Store byte view,
  TokenCount); **Effect** — a typed request the core *emits* for the driver to
  discharge (HTTP call, exec, fs op). See §6 call model.
- **Driver** — the shim-side `kittens::reactor!` loop that feeds the core
  events and discharges its effects.
- **Mount** — read-only attachment of a (possibly foreign) log + indexes into
  the RLM namespace (section 10).
- **Budget** — a typed, enforced limit (bytes, tokens, wall-clock, recursion
  nodes). Budgets are protocol data; enforcement is core law via sealed
  cap-types (P6).

## 3. Crate topology (normative for KC0)

```
crates/kittens-code-protocol    no_std+alloc; serde only
crates/kittens-code-core        no_std+alloc; depends: protocol
crates/kittens-code-tools       no_std+alloc (exec-dependent tools feature-gated)
crates/kittens-code-swarm       candidate (post-KC0; port defined in core now)
crates/kittens-code-std         std shim + driver; tokio + rustls + cap-std
crates/kittens-code             binary; std shim + headless frontend
fixtures/code-no-std            link gate: protocol+core+tools on
                                thumbv7em-none-eabi and wasm32-unknown-unknown
```

Rules:

- T1. `protocol`, `core`, `tools` MUST be `#![no_std]` + `extern crate alloc`,
  compiling in the link-gate fixture from the first commit (R§7).
- T2. `core` MUST NOT depend on: std, tokio, any HTTP/TLS crate, any tokenizer,
  any embedding model, wall-clock, or entropy. Data access enters through
  ports; side effects leave as Effects (§6). Note `core` no longer depends on
  `kittens` — the reactor lives in drivers (§6 call model); the kernel
  dependency moves to `kittens-code-std` (and future embassy/wasm drivers).
- T3. Frontends (kittens-tui, ACP clients) MUST link only `protocol`
  (+ transport), never `core` (R§3.1-2).
- T4. New crates require a spec change; sprawl is a reviewed budget (R§1).
- T5. Admission ledger rule (header): unspecced additions need a recorded
  one-sentence oracle in §15 before build.

Dependency policy (operator directive I-00 §10; details I-10): buy over build —
`agent-client-protocol` (ACP, std), `embedded-io`/`-async` (+adapters),
`reqwless`+`embedded-tls` (embassy driver, post-KC0), serde/serde_json(alloc),
`postcard` (candidate codec), `mlua` (post-KC0 escape hatch), `model2vec-rs`
(E3 embedder candidate), `grep` crate (std L3), `regex-automata` (D7 spike).
Build-from-scratch is reserved for the three verified ecosystem gaps: the
no_std `Vfs` port, the verb parser (trivially small by design), the thin store
layer (I-03, I-10).

## 4. Protocol contract

Pure serde data (R§3.1-2, I-01):

- P1. `Op` (client→core): user input, interject, approve/deny {tool, patch},
  interrupt, resume, config-patch, shutdown; candidate: fork, rewind-to,
  swarm mount/unmount.
- P2. `Event` (core→client): turn lifecycle, model deltas, tool call
  proposed/started/output-delta/terminal, approval requests, budget updates,
  compaction events (started/applied/suppressed), RLM query trace, errors.
  Ops expecting responses correlate by submission id (I-01).
- P3. Every tool stream MUST end in exactly one Terminal item (I-07).
- P4. `SandboxPolicy`/`ApprovalPolicy` are protocol data; mechanism is
  shim-side (I-01). Per-tool approval defaults ship in the config schema (D12).
- P5. `WindowLayout` — typed post-compaction recipe `[system, user_info,
  rules_reminder, last_user_query, verbatim_tail, summary, reminders]`;
  tail-atomicity (tool call never split from its result) is enforced by the
  constructor (R§5.1). Segments carry optional semantic region labels (R§5.1);
  no KC0 behavior depends on them.
- P6. `Budgets` + sealed cap-types. Limits: RLM verb output cap (default
  8,192 **bytes** — deliberate deviation from the sources' 8,192 chars
  (I-05/I-06), bytes being the honest no_std unit; input 11 finding 10);
  per-tool-result root budget (B-rule below); RLM recursion node budget
  (depth 1 default, R§4.5); per-query verb-count cap (*synthesis-introduced*,
  input 11 finding 11 — retained as a runaway guard); token budget fields fed
  by `TokenCount`. **Enforcement mechanism (normative, input 11 blocker 3):**
  `Capped<LIMIT>`-style sealed newtypes with private truncating constructors;
  every window-insertion API accepts only cap-typed values; bypass is a
  compile error, verified by trybuild (gate G3).
- P7. Wire evolution: additive-only within v0.x; unknown-field-tolerant
  decode (I-07). Persisted-record versioning is D11 (log header record with
  schema version; replay across versions defined there).
- P8. Error taxonomy (D10, KC0-blocking): `ErrorEvent { class:
  Retryable | Fatal | UserActionable, code, message, correlates: Option<SubmissionId> }`;
  exact code list closes with D10.

## 5. Transcript store contract

- S1. The log is append-only: the `Store` port exposes no delete or rewrite
  method — an API-surface guarantee, gated by G9 (API audit + trybuild).
  Rewind appends a marker record that replay elides (I-07). Fork (candidate,
  post-KC0) copies/COW-references with a parent pointer.
- S2. Records are protocol events — the raw notification stream IS the log
  (I-07). Derived caches MUST be rebuildable from the log alone;
  resume = replay (R§3.1-3).
- S3. `Store` port: append(record) → seq; scan(range, filter) → records; len;
  byte-view for L3 search. Full tool outputs are stored as records even when
  the window shows a truncated view (B-rule) — truncation is reversible
  offload, not loss (R§4.3).
- S4. Codec: JSONL in std/wasm. MCU codec (postcard) is candidate, blocked on
  D7 (codec × search). KC0 ships JSONL only.
- S5. Crash discipline in shims: atomic append (temp+rename or target-native
  equivalent) (I-07).
- S6. Session identity (D11, KC0-blocking): each log opens with a header
  record {session UUID, parent session (fork lineage), schema version,
  created-at}; one writer per log (single-writer rule; multi-process append
  is out of scope for v0.x and MUST be rejected by shims via lock or
  create-exclusive).

## 6. Turn engine and call model (core law)

**Call model (input 11 finding 8, resolved):** the core is a *synchronous*
sans-io state machine — `handle(Event) → impl Iterator<Item = Effect>` — with
no async, no IO, no clock (R§7; precedent quinn-proto/rustls). The
`kittens::reactor!` invocation lives in each **driver** (std now; embassy/wasm
later): reactor arms translate admitted source items into core Events; handler
bodies call `core.handle(...)` and enqueue returned Effects; owned tasks
discharge Effects and feed results back through admitted sources. L6 below is
therefore normative for *drivers*, and the kernel's scheduling law governs
exactly the layer it was built for.

- L1. Turn = sample → execute tool calls → append results → resample; a model
  response with no tool calls ends the turn (R§3.1-1).
- L2. Interrupt and cancellation, precisely (input 11 blocker 2 + finding 4):
  the driver runs **one reactor per session** (option (b)). The `shutdown`
  arm (terminal, K0 law, unguarded leading prefix) ends the *session*. The
  `interrupt` arm is a separate ordinary non-terminal source placed
  immediately after `shutdown` in the lexical prefix (order frozen with
  `#[before]`), whose handler marks the in-flight turn aborted and emits
  cancellation Effects. **Kernel law guarantees interrupt is *observed* ahead
  of the firehose (G4); cancellation *propagation* into owned tasks is harness
  code, not kernel law, and is gated separately (G4b: in-flight tool abort
  produces a terminal abort record in the log).** Partial streams always
  close with terminal abort records.
- L3. Termination guard: stationarity detection on identical consecutive tool
  calls (single-lineage steal, R§3.2; thresholds config data). No fixed
  iteration cap. Gated in the jail battery (G7).
- L4. Tool execution: approval serial, execution concurrent, results rejoined
  by index; ordering-barrier tools forced into a following batch (I-07).
- L5. Subagent spawning: deferred from KC0 (D8). Protocol reserves the
  shapes; per-spawn isolation policy axis when built (R§3.1-6). Fan-out needs
  either the funnel pattern or a K1-era kernel ask (see L6 ledger).
- L6. Driver law (normative; input 11 finding 4): the **owned-task + funnel
  pattern is the required topology** — model SSE pump task → admitted mpsc
  arm with `#[drain(max)]` + `#[yields_to(input, when = buffered)]`; ALL tool
  completions funnel through ONE admitted mpsc of `(call_id, terminal_item)`
  with rejoin-by-index in application state; interjections on their own arm
  below interrupt; compaction prefire scheduled in `after_event`, its summary
  returning via `OptionalOneShot`; timers via `OptionalDeadline`.
  Kernel-ask ledger (post-KC0, from I-10/input 11): KX1–KX3 embassy adapters
  (channel/signal/deadline) gate MCU runtime; dynamic source sets for
  subagent fan-out and a reviewed SSE/byte-stream adapter are K1-era asks —
  their absence in K0 does NOT trip falsifier F-a, because the funnel pattern
  is the documented K0-conformant story (agent-guide).

## 7. Context engine

- C1. The window is a derived view; compaction never deletes records (R§4.3).
- C2. Escalation order: (1) microcompact — age out stale tool results from
  the window (mechanism adopted from an unverifiable teardown — Observation
  (leak), R§3.2; input 11 finding 14); (2) full summarization into
  `WindowLayout`; (3) mechanical drop-oldest fallback. Circuit breaker after
  deterministic failure (I-06, I-07).
- C3. Scheduling: prefire — background summarization below the hard trigger
  (defaults 75%/85%, config data), keyed by conversation fingerprint; delayed
  rather than eager application (R§3.2, I-08 §6).
- C4. Startup/config content re-injected from source after compaction, never
  summarized (R§3.1-4).
- C5. Reminders travel as blocks attached to user-role messages; static
  prompt prefix with one deliberate static/dynamic cache boundary (R§3.1-5).
- C6. RLM standing reminder every turn; wording is config data (E1 variable).
- C7. Untrusted config-file content is escaped at the injection boundary
  (R§3.2, I-07). Gated by adversarial fixture (G8).
- C8. Token accounting (D-a, resolved for KC0 — input 11 finding 7): the
  driving number is hybrid — provider-reported usage for history (exact) +
  a tail estimator **self-calibrated each turn against the provider's
  reported usage** (running bytes-per-token ratio, not a fixed /4). G5
  reports the estimator's observed error bounds next to E1 results so
  compaction-trigger noise is quantified, not assumed away.

## 8. RLM engine (core law)

- Q1. Surface: fixed verb set, one verb per line, Unix-flavored (R§4.4):
  `grep`, `slice`, `head`, `tail`, `count`, `partition`, `ask`, `ask-each`,
  `final`, `final-var`. Namespace targets: own log (default), mounted peers
  (candidate), never working files (those are tools). Grammar (D3) is
  versioned and STABLE — a future RL action space (R§4.4, I-08 §2).
- Q2. Budgets enforced via P6 cap-types: per-verb output cap; verb-count cap
  (*synthesis-introduced*); recursion node budget (depth 1 default);
  token + wall-clock meters on every `ask`/`ask-each` node surfaced as
  protocol events (R§4.1 cost-tail differentiator).
- Q3. Root-protection law (rescoped per input 11 blocker 1): **all
  RLM-originated data entering the root window is cap-typed** (verb output,
  `ask` digests), and **every tool result surfaced to the root window is
  truncated to a typed per-result budget** (head/tail excerpt + log-offset
  pointer; full output lives in the store as a record, S3) — truncation is
  reversible offload: the model recovers the remainder via verbs (R§4.2,
  R§4.3). The v0.1 claim that bulk output "never enters" the root described
  the sub-LM-holds-tools architecture (I-05) and is withdrawn for KC0's
  root-holds-tools shape.
- Q4. `ask` runs against the sub-model tier; selection + question, no tool
  access in KC0. `ask-each` = bounded fan-out over a partition. `sample-k`
  on `ask` is an E2 arm, feature-gated (R§4.4).
- Q5. L3 search (`grep`) core-mandatory over the Store byte view; no_std
  pattern dialect is D7; KC0 uses the std matcher behind a core trait.
- Q6. L2 (`Embedder`/`Similar` ports) defined in KC0, implemented post-KC0
  (E3). L2 answers carry log-offset provenance (R§5.2).
- Q7. RLM also exposed as an opt-in `recall` tool (I-06 RecallTool) — and E1
  gains the corresponding arm (input 11 finding 5): always-on vs
  tool-mediated access are compared, not just presence/absence.

## 9. Tools kernel

- K1. KC0 set: `read`, `write`, `edit` (fuzzy fallback + unified diff,
  I-06), `grep` (workspace), `exec` (feature-gated, absent on MCU),
  `apply_patch` (grammar-constrained + streaming parser, I-01). All results
  flow through the Q3 truncation law. Conformance gated by G7.
- K2. File tools speak `Vfs` effects; process tools speak `Exec` effects.
- K3. COW checkpoint layer over Vfs: **cut from KC0** (input 11 finding 13);
  candidate for KC1 with the std impl (R§3.2 remains the evidence).
- K4. MCP, web, skills: candidate, post-KC0, outside kernel crates (R§3.1-7).

## 10. Swarm (candidate — port now, crate post-KC0)

- W1. `ContextExchange` port defined in core, unimplemented in KC0: enumerate
  peers; mount/unmount read-only stores; resolve peer offsets to records.
- W2. Scopes deny-by-default, own/team/all (governed-memory precedent, R§6);
  no write path exists in the port.
- W3. E4 gate: swarm crate ships only with the eval running isolation-only
  (null hypothesis, I-06) vs +read-mounts (R§6).

## 11. Ports and effects

Data-access ports (core pulls): `Store` (S3), `TokenCount` (C8),
`Embedder`/`Similar` (Q6, E3-gated), `ContextExchange` (W1).
Effects (core emits, drivers discharge): `Http` (SSE-capable model call),
`Vfs` ops, `Exec` (absent on MCU), timer arm/disarm, sub-model call (`ask`).
`Clock`/`Entropy` values are injected on Events that need them — never
ambient (R§7).

KC0 impls: Store = in-memory (core tests) + JSONL file (std); Vfs = in-memory
+ cap-std; Http = reqwest+rustls SSE; Exec = tokio process; TokenCount =
hybrid per C8. Port-shape rule: effect-level, few and coarse (R§7 tax
warning; E5 measures the residual).

## 12. Model client (std driver, KC0)

- M1. One wire dialect: Anthropic-style messages, streaming SSE (D6).
- M2. Retry: bounded exponential ladder, jitter, cancel-aware sleeps,
  Retry-After honored; semantic retries (compact-and-resubmit) above
  transport (I-07); plus a simple failure-count circuit breaker (I-07 records
  its absence in Grok as a reject). Gated in the jail battery (G7).
- M3. Two model tiers configured: root and sub (`ask`) (I-06).

## 13. Frontend seams (KC0: headless only)

- F1. Headless driver (stdin/stdout protocol stream + JSONL event dump).
- F2. ACP adapter: candidate, std shim (R§8.1).
- F3. kittens-tui wiring: blocked on D-b negotiation; offered contract is the
  protocol event stream; no privileged path (I-07 lesson).

## 14. KC0 — the controlling evidence slice

Scope (adjusted per input 11 findings 5, 9, 13):

1. `protocol` + `core` + `tools` compiling under the link gate (T1).
2. Turn engine per §6 call model: sync core + std driver expressed in
   `kittens::reactor!` (L6 funnel topology), driving a real Anthropic-dialect
   endpoint and the mock-LLM jail.
3. Append-only JSONL store with resume-as-replay (S1–S6; fork excluded).
4. Context engine C1–C8 present (C8 = calibrated hybrid per D-a); RLM engine
   Q1–Q5 + Q7 complete; L2/swarm ports defined, unimplemented.
5. Tools K1–K2 complete (K3 cut).
6. Eval rig: mock-LLM jail (D14 defines its interface: scripted
   request-fingerprint→response sequences + behavioral capture, I-06 ORG-5)
   + E1 executed end-to-end with arms {compaction-only, RLM-always-on,
   RLM-as-tool, both} × {reminder on/off} on the D14 task battery.
   Terminal-Bench 2.0 is a KC1 gate (I-08).

Gates (all MUST pass before any post-KC0 surface is built):

- G1. Link gate green on thumbv7em-none-eabi + wasm32-unknown-unknown (CI).
- G2. Replay determinism: resume-from-log reproduces identical window state
  (property test), including across a schema-version bump fixture (D11).
- G3. Budget law by construction: trybuild compile-fail on any window
  insertion bypassing cap-types (P6 mechanism) + adversarial runtime fixture
  attempting oversized verb output and oversized tool results (Q3).
- G4. Interrupt observation: interrupt-during-firehose serviced within one
  service window (kernel-level, K0 machinery reused).
- G4b. Cancellation propagation: in-flight tool aborted on interrupt; log
  closes with terminal abort record (harness-level).
- G5. E1 report with effect directions, confidence, and the C8 estimator's
  observed error bounds. A null result is a pass; an unrun eval is the only
  failure.
- G6. Compaction atomicity: no window ever splits a tool call from its
  result (fuzzed over synthetic histories).
- G7. Tools + loop-guard conformance: apply_patch parser fuzz; edit
  fuzzy-fallback vectors; stationarity-guard and retry/circuit-breaker jail
  scenarios.
- G8. Injection-escape adversarial fixture for C7 (hostile AGENTS.md-class
  content cannot break the reminder frame).
- G9. Store immutability API audit: no delete/rewrite surface exists
  (trybuild + review checklist item).
- G10. Real-endpoint smoke: one recorded end-to-end session against the live
  dialect (M1), replayable offline afterward (S2).

KC0 falsifiers (any one forces architecture revision, not patching):

- F-a. The funnel pattern proves inadequate in the driver — e.g.
  rejoin-by-index breaks under interleaving, or the reactor macro cannot
  express the L6 topology without escaping to raw selection (kills §6 call
  model's kernel-hosting claim; re-evaluate kernel fit, R§8.4). The known K0
  absences (dynamic sources, SSE adapter) are ledgered in L6 and do NOT trip
  this falsifier.
- F-b. The no_std core forces measured >5% hot-path overhead vs a std-native
  control (E5 threshold; kills T2's current shape, not the goal).
- F-c. G3's cap-type mechanism cannot be maintained without interpreter-grade
  parsing of verb output (kills the verb-surface simplicity claim; escalates
  E2 priority).

## 15. Decisions register

| ID | Decision | Status | Blocking |
|---|---|---|---|
| D1 | Crate topology §3 (core no longer links kittens; drivers do) | set for KC0 | freeze after KC0 |
| D2 | Op/Event wire shapes | draft P1–P8; closes with D10 | spec refinement |
| D3 | Verb grammar (exact EBNF) | open — next spec pass | KC0 build |
| D4 | WindowLayout field types | draft P5 | spec refinement |
| D5 | Isolation policy enum | deferred with D8 | post-KC0 |
| D6 | Wire dialects | KC0 = Anthropic-style only | revisit KC1 |
| D7 | no_std search dialect × store codec | open, spike | MCU claims only |
| D8 | Subagent tool | deferred from KC0 | post-KC0 |
| D9 | *(merged into D13)* | — | — |
| D-a | TokenCount | **resolved for KC0**: provider-exact history + self-calibrated tail ratio; error bounds reported in G5 (C8) | — |
| D-b | kittens-tui seam | blocked on external negotiation | frontend only |
| D-c | ESP32-S3 spike | scheduled pre-MCU-claims (R§7) | MCU claims only |
| D10 | Error taxonomy (P8 codes/classes) | open, KC0-blocking | KC0 build |
| D11 | Persisted schema versioning + session identity (S6, G2 fixture) | drafted S6; close exact header shape | KC0 build |
| D12 | Config schema + precedence (defaults < file < config-patch Op) + per-tool approval defaults | open, KC0-blocking | KC0 build |
| D13 | Prompt content ownership (system prompt, reminder templates, compaction summary prompt) — candidate: versioned prompt-pack as core data, config-overridable | open, KC0-blocking | KC0 build |
| D14 | Mock-jail interface + E1 task battery definition | open, KC0-blocking (G5 depends) | KC0 build |
| KX1–KX3 | kittens embassy adapters (channel/signal/deadline) | ledgered kernel asks (I-10) | MCU runtime only |
| KX-K1 | dynamic source sets + SSE adapter kernel asks | ledgered, K1-era | post-KC0 |

## 16. Lineage

- 2026-08-08: SPEC v0.1 drafted from RESEARCH.md v2 (commit `a244218`).
- 2026-08-08: spec refinement pass 1 → v0.2. Inputs: adversarial spec review
  (input 11 — blockers: Q3/G3 root-protection contradiction resolved by
  rescoping to cap-typed RLM data + tool-result truncation-as-reversible-
  offload; L2/L6 reactor-lifetime ambiguity resolved as one-reactor-per-
  session with interrupt as leading non-terminal arm; G3 given a real sealed
  cap-type mechanism), kernel-fit analysis (input 10 — funnel pattern
  normative, KX ledger), operator directive (I-00 §10 — dependency policy in
  §3, kernel extension explicitly in scope). Cut from KC0: fork, K3, mlua.
  Added: G4b, G7–G10 gates; D10–D14 decisions; §6 call model (sync core +
  effect queue; reactor lives in drivers).
- Next: close D3 (verb EBNF) + D10–D14 drafts in spec refinement pass 2, then
  operator/other-harness review, then freeze of KC0 sections only.
  Implementation remains unauthorized until freeze.
