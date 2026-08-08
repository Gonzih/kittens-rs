# kittens-code specification

- Spec date: 2026-08-08
- Version: v0.1 draft — first normative pass over [RESEARCH.md](RESEARCH.md) v2;
  refinement passes pending; **not yet frozen, no implementation authorized**
- Controlling evidence slice: section 14 (KC0). Sections 3–13 are normative for
  KC0 only where section 14 imports them; everything else is candidate design
  retained for lineage, mirroring the root SPEC's §37 discipline.
- Research basis: every design decision cites a RESEARCH.md section (R§n) or a
  pinned input (I-nn). Decisions with open evidence are listed in section 15 and
  block freeze, not drafting.

Label discipline follows RESEARCH.md. In this file, **MUST/MUST NOT/SHOULD** are
normative for KC0; "candidate" marks post-KC0 surface that must not be built
before its gate.

## 1. Scope and non-goals

kittens-code is a coding-agent harness crate family. It exists to prove three
falsifiable bets (R§1) on top of the settled 2026 harness skeleton (R§3.1):
RLM-native context law (R§4), tiered context (R§5), swarm read-mounts (R§6) —
under a hard portability constraint (no_std+alloc core, virtual IO/FS, R§7).

Non-goals for the entire v0.x line:

- N1. TUI rendering. Owned by `kittens-tui` (separate harness). kittens-code
  exposes only the protocol event stream (R§8.1). The seam freezes only after
  negotiation with that harness (R§9 Q1).
- N2. Harness self-optimization (R§9 Q10).
- N3. Write-side swarm coordination (task lists, inboxes) — well-occupied
  territory (R§6).
- N4. A new async runtime, executor, or rendering protocol (inherited from the
  kittens kernel's own non-goals).
- N5. Interpreter in the core. Lua lives in the std shim only (R§4.4); piccolo
  rejected for v1 (I-08 §5).

## 2. Terminology

- **Record** — one immutable, typed entry in the transcript log (protocol event
  envelope + monotonic sequence number + logical timestamp).
- **Log** — the append-only sequence of records for one session. Never truncated,
  never rewritten; rewind and compaction are markers/derived views (R§3.1-3).
- **Window** — the token-bounded message list actually sent to the model (L1).
- **Verb** — one line of the RLM query surface (section 8).
- **Port** — a trait the core depends on; implemented by shims (section 11).
- **Shim** — a target-specific crate discharging effects (std, wasm, embassy).
- **Mount** — a read-only attachment of a (possibly foreign) log + indexes into
  the RLM namespace (section 10).
- **Budget** — a typed, enforced limit (bytes, tokens, wall-clock, recursion
  nodes). Budgets are data in the protocol crate, enforcement is core law.

## 3. Crate topology (normative for KC0)

Exactly as R§8.1, reproduced as the buildable contract:

```
crates/kittens-code-protocol    no_std+alloc; serde only
crates/kittens-code-core        no_std+alloc; depends: protocol, kittens
crates/kittens-code-tools       no_std+alloc (exec-dependent tools feature-gated)
crates/kittens-code-swarm       candidate (post-KC0; port defined in core now)
crates/kittens-code-std         std shim; tokio + rustls + cap-std + mlua(feature)
crates/kittens-code             binary; std shim + frontend wiring
fixtures/code-no-std            link gate: protocol+core+tools on
                                thumbv7em-none-eabi and wasm32-unknown-unknown
```

Rules:

- T1. `protocol`, `core`, `tools` MUST be `#![no_std]` with `extern crate alloc`,
  and MUST compile in the link-gate fixture from the first commit (R§7).
- T2. `core` MUST NOT depend on: std, tokio, any HTTP/TLS crate, any tokenizer,
  any embedding model, wall-clock, or entropy. All such capabilities enter
  through ports (section 11).
- T3. Frontends (including kittens-tui and any ACP client) MUST link only
  `protocol` (+ a transport), never `core` (R§3.1-2). In-process embedding uses
  the channel-pair form of section 4 — still protocol-typed.
- T4. New crates require a spec change; the sprawl budget is a reviewed resource
  (R§1 rejections).

## 4. Protocol contract

The protocol crate defines, as pure serde data (R§3.1-2, I-01):

- P1. `Op` (client→core): user input, interject, approve/deny {tool, patch},
  interrupt, resume, fork, rewind-to(marker), config-patch, swarm mount/unmount
  (candidate), shutdown.
- P2. `Event` (core→client): turn lifecycle, model deltas (text/reasoning),
  tool call proposed/started/output-delta/terminal, approval requests, budget
  updates, compaction events (started/applied/suppressed), RLM query trace
  events, error events. Every `Op` that expects a response correlates by
  submission id (I-01).
- P3. Every tool stream MUST end in exactly one Terminal item (I-07 steal).
- P4. `SandboxPolicy` and `ApprovalPolicy` are protocol data, mechanism lives in
  shims (I-01 steal 6).
- P5. `WindowLayout` — the typed post-compaction recipe `[system, user_info,
  rules_reminder, last_user_query, verbatim_tail, summary, reminders]` with the
  tail-atomicity invariant (a tool call and its result are never split) encoded
  in the type's constructor, not convention (R§5.1). Segments carry optional
  semantic region labels for serving-layer co-design (R§5.1); no KC0 behavior
  depends on them.
- P6. `Budgets` — per-turn output caps, RLM verb output cap (default 8192 bytes,
  R§4.2), RLM recursion node budget (default: depth 1, R§4.5), token budget
  fields fed by the `TokenCount` port.
- P7. Wire evolution: additive-only within v0.x; unknown-field-tolerant decode
  (Grok's `_x.ai` extension lesson, I-07).

## 5. Transcript store contract

- S1. The log is append-only. No API exists to delete or rewrite a record.
  Rewind appends a marker record that replay elides (I-07). Fork copies (or
  COW-references) the log with a parent pointer.
- S2. Records are protocol events — the raw notification stream IS the log
  (I-07). Derived caches (window state, search indexes) MUST be rebuildable
  from the log alone; resume = replay (R§3.1-3).
- S3. The `Store` port (section 11) exposes: append(record) → seq;
  scan(range, filter) → records; len; and a byte-view sufficient for L3 search.
- S4. Codec: JSONL in std/wasm shims. MCU codec (postcard) is candidate,
  blocked on decision D7 (codec × search, R§5.3): KC0 ships JSONL only.
- S5. Crash discipline in shims: atomic append (temp+rename or equivalent
  target-native guarantee) (I-07).

## 6. Turn engine (core law)

- L1. Turn = sample → execute tool calls → append results → resample; a
  model response with no tool calls ends the turn (R§3.1-1).
- L2. One cancellation lineage threads every phase; user interrupt is a
  shutdown-class source under the kittens reactor's unguarded leading prefix
  (R§8.4). Interrupt aborts the turn without corrupting the log (partial
  streams get terminal abort records).
- L3. Termination guard: stationarity detection on identical consecutive tool
  calls (single-lineage steal, R§3.2; thresholds are config data, Grok defaults
  16/4). No fixed iteration cap.
- L4. Tool execution: approval serial, execution concurrent, results rejoined
  by index; ordering-barrier tools (plan-exit class) forced into a following
  batch (I-07).
- L5. Subagent spawning: **deferred from KC0** (D8, R§9 Q8). The protocol
  reserves the op/event shapes; core implements none of it in KC0. When built,
  isolation is a per-spawn policy axis, not a constant (R§3.1-6).
- L6. Reactor hosting: the core's event sources map onto `kittens::reactor!`
  arms — shutdown/interrupt in the leading prefix; model-delta stream as
  may-remain-ready with drain bound and yields_to on interactive input; tool
  completions admitted via reviewed adapters; compaction prefire scheduled in
  after_event (R§8.4). KC0 MUST express the loop through the kernel macro, not
  a hand-rolled select — kittens-code is the kernel's second forcing fixture.

## 7. Context engine

- C1. The window is a derived view; compaction never deletes records (R§4.3).
- C2. Escalation order: (1) microcompact — age out stale tool results from the
  window (R§3.2); (2) full summarization into `WindowLayout`; (3) mechanical
  drop-oldest fallback. A circuit breaker suppresses repeated compaction after
  deterministic failure (I-06, I-07).
- C3. Scheduling: prefire — background summarization starts at a threshold
  below the hard trigger (defaults 75%/85%, config data), keyed by conversation
  fingerprint; delayed rather than eager application (R§3.2, I-08 §6).
- C4. Startup/config content is re-injected from source after compaction, never
  summarized (R§3.1-4).
- C5. Reminders: mutable state (todo/plan, rules, capability notices) travels
  as reminder blocks attached to user-role messages; the static prompt prefix is
  cache-stable with one deliberate static/dynamic boundary (R§3.1-5).
- C6. The RLM standing reminder (one line naming the verbs and the log) is
  injected every turn; its wording is config data — an E1 eval variable, not a
  constant (R§4.3).
- C7. Untrusted config-file content (AGENTS.md-class) is escaped at the
  injection boundary (R§3.2, I-07).
- C8. Token accounting: `TokenCount` port; the driving number is hybrid —
  provider-reported usage for history + estimator for the in-flight tail
  (I-07; D-a resolves the estimator choice; bytes/4 is the KC0 placeholder
  behind the port, explicitly disposable).

## 8. RLM engine (core law)

- Q1. Surface: fixed verb set, one verb per line, Unix-flavored (R§4.4):
  `grep`, `slice`, `head`, `tail`, `count`, `partition`, `ask`, `ask-each`,
  `final`, `final-var`. Verb namespace targets: own log (default), mounted
  peers (`--peer`, candidate with swarm crate), working files (via tools, not
  verbs). Grammar is versioned and STABLE — it is a future RL action space
  (R§4.4, I-08 §2); breaking changes require a major version.
- Q2. Budgets (enforced, not advisory): per-verb output cap (default 8192
  bytes, R§4.2); per-query verb-count cap; recursion node budget with depth 1
  default (R§4.5); token + wall-clock meters on every `ask`/`ask-each` node,
  surfaced as protocol events (the cost-tail differentiator, R§4.1).
- Q3. Root protection: raw bulk output (tool results, `ask` sub-model inputs)
  never enters the root window; the root sees capped verb output and digested
  `ask` results only (R§4.2).
- Q4. `ask` runs against the sub-model tier (cheap model), receives the
  selection + question, no tool access in KC0. `ask-each` is bounded fan-out
  over a partition. `sample-k` selection on `ask` is an eval arm (E2), feature-
  gated, not default (R§4.4).
- Q5. L3 search (`grep`) is core-mandatory over the `Store` byte view. Pattern
  dialect on no_std targets is decision D7; KC0 (std-only behavior, no_std
  link-only) uses full regex in the shim-provided matcher behind a core trait.
- Q6. L2 (`Embedder`/`Similar` ports) is defined in KC0, implemented post-KC0
  (E3 gates it). L2 answers always carry log-offset provenance (R§5.2).
- Q7. The RLM engine is also exposed as an opt-in tool (`recall`) so E1 can
  compare always-on vs tool-mediated access (I-06 RecallTool precedent).

## 9. Tools kernel

- K1. KC0 set: `read`, `write`, `edit` (fuzzy fallback + unified diff output,
  I-06 ORG-2), `grep` (workspace), `exec` (feature-gated, absent on MCU),
  `apply_patch` (grammar-constrained format + streaming parser, I-01).
- K2. All file tools speak `Vfs`; all process tools speak `Exec` (section 11).
- K3. Checkpointing: COW snapshot layer over `Vfs`, decoupled from user git
  (R§3.2); KC0 ships the trait + in-memory impl; std impl candidate.
- K4. MCP, web, skills: candidate, post-KC0, outside the kernel crates (R§3.1-7).

## 10. Swarm (candidate — port now, crate post-KC0)

- W1. `ContextExchange` port in core (defined, unimplemented in KC0): enumerate
  peers; mount/unmount read-only peer stores; resolve peer offsets to records.
- W2. Scopes: deny-by-default; own/team/all levels (governed-memory precedent,
  R§6). Peer mounts are read-only by construction — no write path exists in the
  port.
- W3. E4 gate: the swarm crate ships only with the eval that runs
  isolation-only (null hypothesis, I-06 lesson 8) vs +read-mounts (R§6).

## 11. Ports (core traits; all object-safe or generic per measured cost, E5)

| Port | KC0 impls | Notes |
|---|---|---|
| `Store` | in-memory (core tests), JSONL file (std) | S3 |
| `Vfs` | in-memory (core), cap-std (std) | ~6 methods; paths &str; files as embedded-io-async streams (I-03) |
| `Http` | reqwest+rustls SSE (std) | SSE-capable; core sees byte streams + typed deltas |
| `Exec` | tokio process (std) | absent on MCU; feature-gated in tools |
| `Clock`, `Entropy` | std | injected, never ambient (R§7) |
| `TokenCount` | bytes/4 placeholder (std) | C8, D-a |
| `Embedder`, `Similar` | none in KC0 | E3-gated; model2vec-rs is the std/wasm candidate (R§5.2) |
| `ContextExchange` | none in KC0 | W1 |

Port-shape rule: ports sit at the effect level (few, coarse); no per-syscall
wrappers (R§7 tax warning; E5 measures the residual cost).

## 12. Model client (std shim, KC0)

- M1. One wire dialect in KC0: Anthropic-style messages (decision D6 resolved
  for KC0 scope; second dialect is candidate). Streaming SSE; deltas mapped to
  protocol events.
- M2. Retry: bounded exponential ladder with jitter and cancel-aware sleeps;
  Retry-After honored; semantic retries (compact-and-resubmit) above transport
  (I-07). A simple failure-count circuit breaker is added (Grok's absence of one
  is a recorded reject, I-07).
- M3. Two model tiers configured: root and sub (`ask`) — dual-model precedent
  (I-06).

## 13. Frontend seams (KC0: headless only)

- F1. KC0 ships a headless driver (stdin/stdout protocol stream + JSONL event
  dump) sufficient for evals — no TUI dependency.
- F2. ACP adapter over the protocol stream: candidate, std shim (R§8.1).
- F3. kittens-tui wiring: blocked on Q1 negotiation; the protocol event stream
  is the offered contract; no privileged path will be added (I-07 lesson).

## 14. KC0 — the controlling evidence slice

KC0 proves, with the smallest honest surface, that the three-bet architecture
stands up on a real host while remaining no_std-linkable. Scope:

1. `protocol` + `core` + `tools` crates compiling under the link gate (T1) —
   thumbv7em-none-eabi + wasm32-unknown-unknown, from first commit.
2. Turn engine hosted by `kittens::reactor!` (L6) on the std shim, driving a
   real Anthropic-dialect endpoint (M1) and the mock-LLM jail.
3. Append-only JSONL store with resume-as-replay and fork (S1–S5).
4. Context engine C1–C8 complete; RLM engine Q1–Q5 + Q7 complete; L2/swarm
   ports defined but unimplemented (Q6, W1).
5. Tools K1–K2 complete; K3 trait + memory impl.
6. Deterministic eval rig: mock-LLM jail (scripted responses, behavioral
   capture, I-06 ORG-5) + E1 executed end-to-end (compaction-only vs RLM-only
   vs both, ± reminder) on a small task battery. External yardstick
   (Terminal-Bench 2.0) is a KC1 gate, not KC0.

KC0 gates (all MUST pass before any post-KC0 surface is built):

- G1. Link gate green on both bare targets (CI).
- G2. Replay determinism: resume-from-log reproduces identical window state
  (property test over recorded sessions).
- G3. Budget law: no execution path can deliver more than the verb cap into
  the root window (enforced-by-type test + adversarial fixture).
- G4. Reactor law: interrupt-during-firehose is serviced within one service
  window (kernel-level test, K0 machinery reused).
- G5. E1 produces a signed report with effect directions and confidence — the
  first data anywhere on {compaction}×{RLM} (R§4.3 Gap). A null result is a
  pass (it is evidence); an unrun eval is the only failure.
- G6. Compaction atomicity: no window ever splits a tool call from its result
  (fuzzed).

KC0 falsifiers (any one forces architecture revision, not patching):

- F-a. The reactor-hosted loop cannot express the streaming/tool topology
  without escaping the macro (kills L6 → re-evaluate kernel fit, R§8.4).
- F-b. The no_std core forces >2 port indirections on the hot sampling path
  with measured overhead >5% vs a std-native control (E5 threshold; kills T2's
  current shape, not the goal).
- F-c. G3 cannot be enforced without interpreter-grade parsing (kills the
  verb-surface bet's simplicity claim; escalates E2 priority).

## 15. Decisions register

| ID | Decision | Status | Blocking |
|---|---|---|---|
| D1 | Crate topology §3 | set for KC0 | freeze after KC0 |
| D2 | Op/Event wire shapes | draft P1–P7 | spec refinement |
| D3 | Verb grammar (exact syntax/EBNF) | open — next spec pass | KC0 build |
| D4 | WindowLayout field types | draft P5 | spec refinement |
| D5 | Isolation policy enum (subagents) | deferred with L5/D8 | post-KC0 |
| D6 | Wire dialects | KC0 = Anthropic-style only | revisit KC1 |
| D7 | no_std search dialect × store codec | open, needs spike (R§9 Q3) | MCU claims only |
| D8 | Subagent tool in v1 | deferred from KC0 (R§9 Q8) | post-KC0 |
| D9 | Compaction summary-prompt ownership | open (R§9 Q9): candidate = prompt as protocol data, call via std shim | KC0 build |
| D-a | TokenCount estimator | placeholder bytes/4 behind port (C8) | KC1 |
| D-b | kittens-tui seam | blocked on external negotiation (Q1) | frontend only |
| D-c | ESP32-S3 spike | scheduled pre-MCU-claims (R§7) | MCU claims only |

## 16. Lineage

- 2026-08-08: SPEC v0.1 drafted from RESEARCH.md v2 immediately after research
  commit `1586eba`. Normativity model copied from root SPEC §37 discipline:
  a small controlling slice (KC0) + candidate surface retained for lineage.
  Known-incomplete: D3 grammar, D2/D4 exact types, adversarial spec review not
  yet applied. Next: spec refinement pass (adversarial review agent), then
  operator/other-harness review, then freeze of KC0 sections only.
