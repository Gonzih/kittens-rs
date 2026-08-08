# kittens-code research report

- Research date: 2026-08-08
- Refinement pass 1: 2026-08-08 (adversarial review + fresh-eyes SOTA sweep applied;
  this is v2 — v1's uncorrected claims are preserved only inside inputs 09's quotes)
- Status: refined synthesis, one coordination gap open (kittens-tui seam); ready to
  seed spec drafting; not a spec, not an implementation
- Scope: `kittens-code`, a coding-agent harness crate family built on the `kittens`
  reactor kernel and the (externally owned) `kittens-tui` rendering abstraction
- Raw inputs: ten pinned files in [`research-inputs/`](research-inputs/) — one
  operator-constraint capture (00), seven vector reports (01–07) produced 2026-08-08
  by parallel research harnesses, and two refinement reports (08 SOTA sweep,
  09 adversarial review of v1). Every claim below traces to one of those files,
  except items explicitly marked *synthesis-introduced*. Read the inputs before
  challenging a claim here; challenge them against their pinned commits before
  challenging the sources.

This report reuses the four labels of the root [`RESEARCH.md`](../../RESEARCH.md):

- **Fact** — supported by inspected code at a pinned commit, a primary document, or a
  published measurement.
- **Observation** — an interpretation of inspected behavior or ecosystem evidence.
  Absence-of-prior-art claims are always at most Observation. Material sourced from
  the 2026-03-31 Claude Code leak is marked **Observation (leak)** and is
  architecture intelligence only — never a copy source.
- **Hypothesis** — something kittens-code must measure rather than assume.
- **Recommendation** — a concrete design decision proposed for kittens-code.

Additionally, **Operator constraint** marks a fixed input from the project operator
(see [`research-inputs/00-operator-constraints.md`](research-inputs/00-operator-constraints.md));
constraints are requirements, not evidence, and the report says so where evidence is
thin under them. **Gap:** marks a question where no data exists; gaps are left
unsettled deliberately.

Pinned primary sources:

| Source | Pin | Input file |
|---|---|---|
| openai/codex | `936f5eb3ee223ab34dcb221fa7c5f9943c8092bd` (2026-08-08) | 01 |
| Claude Code / Agent SDK | public docs + dated teardowns, fetched 2026-08-08 | 02 |
| no_std/WASM crate ecosystem | crates.io API pins, 2026-08-08 | 03 |
| OSS agent landscape | `gh` CLI star/date pins, 2026-08-08 | 04 |
| alexzhang13/rlm | `72d6940142ddfb84ee6be573dc999a37e633e671` | 05 |
| PrimeIntellect-ai/prime-agent | `a18809e00ea30638584d87b3afea7285a9d7296c` (2026-08-07) | 05 |
| Prior harness experiments (org + personal repos) | inspected 2026-08-08 | 06 |
| xai-org/grok-build | `afbc0fb710320c7add294c2106d447ecc3e3af2e` (2026-08-07, v1.0.0) | 07 |
| Harness-engineering literature 2026 | arXiv pins in input 08 | 08 |

Redaction rule (Operator constraint): the operator's former company is referred to
only as "prior internal harness experiments"; its repos carry codenames ORG-1..ORG-7
in input 06. The org name must never enter this document tree. Refinement note:
specific `nexus-*` personal repo names are additionally avoided in this synthesis
(input 06 notes some are stale mirrors of org work — a deanonymization edge); input
06 retains them under the operator-permitted personal-repo allowance.

## 1. Executive summary

**Recommendation:** build kittens-code as a small family of crates around a
`no_std + alloc`, sans-io agent core driven by the kittens reactor, with three
load-bearing original bets, each individually falsifiable:

1. **RLM-native context law.** The transcript is an append-only event log that is
   never destroyed, always queryable by the model through a deliberately tiny
   shell-like verb surface, while the live window is compacted continuously and a
   one-line standing reminder keeps the capability salient. Compaction discards
   *window copies*, never information.
2. **Context tiers as an explicit memory hierarchy.** L1 = compacted live window;
   L2 = pluggable "super good enough" embedding index for fast topic hints;
   L3 = exact search over the full log as ground truth. The harness is given the
   means to orient itself inside its own context the way a CPU orients in its cache
   hierarchy.
3. **Swarm as a mount, not a protocol.** Cross-harness "read each other's thoughts"
   is the same RLM verb surface pointed at another harness's log store, exposed as a
   modular `ContextExchange` seam that plugs in and out for evals. A deliberate
   falsification attempt (input 08 §3) found no prior occupant; the nearest families
   and the standing counter-signal are recorded in section 6.

Everything else is deliberate convergence with the 2026 state of the art. Section 3
separates what is genuinely convergent (three or more independent lineages) from
single-source mechanisms worth stealing — v1 blurred that line; the adversarial
review (input 09) forced the split. Independent corroboration at larger scale now
exists: **Fact (input 08):** "Inside the Scaffold" (arXiv:2604.03515, 2026-04)
taxonomizes 13 open-source coding agents at pinned commits and documents seven
distinct compaction strategies and five composable control primitives — same
skeleton, same method, larger n. **Fact (input 08):** harness choice alone moves
Terminal-Bench 2.0 scores by 10–20 points; Harness-Bench (arXiv:2605.27922) makes
model+harness the attributable unit. The harness is worth engineering; that is now
a measured claim, not a belief.

The portability requirement (microcontroller + WASM, virtual IO/FS) has no occupant
in any surveyed or swept source — **Observation (inputs 03, 04, 08):** no coding
agent ships a no_std or WASM core; the 08 sweep re-checked and found none. The
feasibility stack exists (embedded-io-async, reqwless, embedded-tls, serde/alloc),
with one honest hard problem: TLS certificate verification off-std (section 7).

**Rejected up front:** crate sprawl (Codex ~110, Grok ~70 members), a leader
daemon, SQLite as source of truth, a bespoke RLM DSL grammar (tried and superseded
in the operator's own prior work — Observation, input 06), RLM recursion depth >1
by default (measured 95× cost blow-up), tokenizer coupling in the core, a no_std
Lua in the core (piccolo rejected for v1 — input 08 §5: pre-1.0, no no_std claim,
`string`/`table` stdlib unimplemented, which defeats "every LLM knows Lua"; its
fuel-metered sandbox design is the reason to revisit post-1.0), automatic harness
self-optimization (Meta-Harness/HarnessBridge line — declared a non-goal for v1;
the operator's own two abandoned autopoiesis attempts, input 06, support the
deferral), and any TUI rendering responsibility (owned by kittens-tui).

## 2. Method and lineage

Seven research harnesses ran on 2026-08-08 against distinct vectors (six launched
in the first fan-out, plus the operator-constraint capture maintained alongside);
their reports are frozen as inputs 00–07. A synthesis (v1) was then drafted and
immediately subjected to two independent refinement passes: an adversarial
traceability/contradiction review (input 09) and a fresh-eyes SOTA sweep tasked
with falsifying v1's originality claims (input 08). This file is v2, with all 22
review findings and all 11 sweep findings dispositioned. No finding was rejected;
three were softened with recorded reasons (see inline).

Lineage discipline for future harnesses hydrating this work:

- The kernel repo's root `RESEARCH.md` §20B frames Grok Build at commit `393430ee`
  as the desktop north-star fixture; input 07 extends that by one sync (`afbc0fb`,
  v1.0.0) and confirms no loop-architecture change between the pins.
- Detail is not evidence (rule inherited from root report §2): enumerated APIs in
  section 8 are hypotheses until a slice compiles and an eval runs.
- Leak-derived Claude Code material: labeling audited in input 09 finding 22 —
  architecture intel only, no copying recommended anywhere.
- Cross-input contradictions are recorded inline where they exist (sections 3, 4.2,
  4.4, 6); input 09 verified v1 had dropped three and v2 restores them.

## 3. The harness landscape: convergent core vs single-source steals

v1 presented eight "convergent" items; review showed three were single-source or
contradicted. v2 splits them honestly. Where three or more independent production
lineages agree, the pattern is settled prior art for the spec; single-source
mechanisms are adopted (or not) on their own merits and say so.

### 3.1 Genuinely convergent (3+ independent lineages)

**Fact (inputs 01, 02, 04, 05, 07, 08):**

1. **Flat agent loop.** One message list; sample → execute tool calls → append
   results → resample; a response without tool calls ends the turn. Codex
   `run_turn`, Claude Code's `nO` master loop, Grok `process_conversation_turn`,
   pi's minimal kernel (and Thorsten Ball's canonical ~300-line Go example —
   distinct sources, misattributed to each other in v1). Nuance from the larger-n
   taxonomy (input 08): 11/13 agents *compose multiple control primitives* around
   that flat loop (modes, checkpoints, orchestrator tools) — Cline's Plan/Act
   dual-mode and Roo's orchestrator are real counterexamples to "nobody splits
   planning from execution" — but none of the 13 replace the flat sampling loop
   itself. The claim that survives: the *inner* loop is flat everywhere; planning
   structures are layered above it as tools/modes, not woven into it.
2. **Typed protocol boundary.** Codex: `Op`/`EventMsg` queues with submission-id
   correlation and a dependency-light `protocol` crate frontends link instead of
   the engine; Grok: ACP with `impl Agent for AcpGatewaySender` — a plain channel
   sender satisfies the agent trait, so transport swaps freely; OpenCode: HTTP+SSE
   server; Goose: types-only crates split from impl crates. Counter-signal
   recorded: Gemini CLI's core/cli split is soft (in-process, input 04) — the
   boundary is convention there, not law. Approvals ride the same channel as
   ordinary request/response pairs (Codex, Grok).
3. **Append-only log as durable source of truth; resume = replay.** Codex rollout
   JSONL; Grok `updates.jsonl` with replay-tagged re-emission, rewind as
   marker-not-truncation, fork as directory copy; prime-agent session JSONL with
   `/tree` recovery. Caches (Grok `chat_history.jsonl`, FTS index) are derived
   reducers, rebuildable, never authoritative.
4. **Compaction is layered and reversibility-biased.** All of: Codex pre-sampling
   compaction + token budget + `ThreadRolledBack`; Claude Code startup content
   re-injected from disk rather than summarized (Fact, docs); Grok three trigger
   paths with sticky failure suppression and a never-orphan-tool-results tail rule;
   seven-strategy taxonomy across 13 agents (input 08). The reversibility ranking
   (raw > reversible offload > … > summarization) is the operator's prior research
   (input 06, ORG-4), consistent with all of the above.
5. **Mutable state travels as reminders attached to user messages, over a
   cache-stable prompt prefix.** Claude Code system-reminder blocks (Fact, docs);
   Grok reminder rebuilds post-compaction plus TodoGate/date/abort nudges and
   memory injected once, guarded against re-injection specifically to preserve the
   KV-cache prefix (Fact, code). Precision forced by review: the Claude prompt is
   not "static" — **Observation (leak):** ~7 static cached sections + ~13 dynamic
   sections with a deliberate cache-break boundary. The convergent law is *a
   deliberately managed static-prefix/dynamic-suffix cache boundary*, not a frozen
   prompt.
6. **Subagents: depth-capped spawning, one summary back.** Claude Code depth 3;
   Grok task tool depth-capped with large results returned by file reference;
   (Codex ships `multi_agents_v2` but input 01 records no depth data — not cited
   for this half). **Isolation is NOT convergent — it is a policy axis.**
   Contradiction recorded: Claude subagents default to fresh context with an
   explicit full-inheritance `fork` variant (Fact, docs), while Grok children
   *share* the parent's hunk tracker/fs/terminal/env and fork summarized context
   (Fact, code). The spec must model isolation as a per-spawn policy, not a law.
7. **Small tool kernel, heavy schemas deferred.** pi ships four tools and no MCP
   in core (85k stars); ORG-2 shipped seven over virtual traits; Claude and Codex
   defer MCP tool schemas behind on-demand search; OpenHarness (input 08) ships
   43+ tools and thereby demonstrates the opposite pole exists — the kernel-vs-rim
   split is a choice, and the minimal pole has the strongest star-per-complexity
   signal. Operator constraint (REDUCE) decides the pole; evidence says both live.

### 3.2 Single-source mechanisms worth stealing (adopted as Recommendations, not consensus)

- **Stationarity guard instead of iteration caps** — Grok only (Observation:
  single lineage; no analog found in Codex/Claude inputs). Detect runs of
  identical tool calls (Grok: 16 generic / 4 no-op) rather than fixing
  max-iterations. Adopted for §8.2 because it fails toward permissiveness on
  productive loops and toward termination on genuine doom loops.
- **Prefire compaction** — Grok only: background summarization starts ~10 points
  before the 85% trigger, cache-keyed by conversation fingerprint. Adopted (§5.1).
  Now reinforced by independent evidence: **Fact (input 08):** delayed compaction
  (3–5 turns) consistently beats immediate compaction (arXiv:2608.00902,
  2026-08-02) — scheduling compaction off the critical path matters.
- **Microcompact before full summary** — **Observation (leak):** Claude's six-tier
  escalation ages out tool results >60min old before any narrative summary.
  Adopted as a mechanism on its own merits; unverifiable against source.
- **Prompt-injection escaping of untrusted config content** — Grok only (Fact:
  HTML-escapes the leading `<` of AGENTS.md-style content). Adopted; v1's claim
  that Claude does the same was unsourced and is withdrawn.
- **Grammar-constrained patch tool** — Codex only (apply_patch as a Lark-grammar
  freeform tool + standalone streaming parser crate). Adopted for §8.1 tools.
- **Shadow-snapshot checkpointing decoupled from user git** — Cline shadow repo /
  OpenCode snapshot / sketch.dev containers (input 04 shortlist #5; dropped by v1,
  restored per review finding 8). Adopted: a copy-on-write layer over the `Vfs`
  port gives undo/branch on hosts with no git and no OS, and is the natural
  checkpoint story for MCU/WASM targets.

**Observation:** the operator's prior-work research reached the simplicity endpoint
independently: "each generation simplified the harness because models internalized
the patterns" (input 06, ORG-4). Convergence from five independent lineages on the
*inner loop*, now corroborated at n=13 (input 08), is the strongest signal in this
report.

**Recommendation:** adopt §3.1 as baseline skeleton without re-derivation; adopt
§3.2 items as named, individually reversible decisions. Novelty budget goes to
sections 4–7.

## 4. RLM-native context law

### 4.1 The mechanism and its measured envelope

**Fact (input 05):** the RLM line (MIT blog Oct 2025 → arXiv:2512.24601) stores the
prompt/history as a *variable in an environment* rather than tokens in the window;
the root model iterates code → truncated output, peeking/grepping/partitioning and
delegating spans to sub-LM calls; only the final answer surfaces. Measured:
OOLONG 132k, RLM(GPT-5-mini) ≈63 vs GPT-5 ≈29; BrowseComp-Plus at ~10M tokens,
RLM(GPT-5) 100% vs ReAct+BM25 ~60%.

**Fact (input 05):** prime-agent (pin `a18809e`) is the production-shaped proof:
persistent IPython kernel as the *only* tool; sub-agents are full instances spawned
via `await rlm(...)` with async handles; history is append-only JSONL; compaction
runs as an async "garbage collector" agent while the kernel keeps working;
CRUD-editable harness state (`/refine`). ARC-AGI-3 with Opus 5: 95.5% Best@1
(human expert 95.4%).

**Costs and limits (restored per review finding 7) — Fact (input 05):** wall-clock
is significantly worse than baseline everywhere Prime Intellect measured it;
subcalls block and defeat prefix caching in the original line; cost tails are
unbounded; counting/aggregation queries degrade with length; RLM scaffolding
*hurts* skill tasks (PI math-python regression); and all strong results to date
are prompting-only — PI's stated thesis is that real gains need RL training of the
context-management behavior. **Fact (input 08):** SRLM (arXiv:2603.15653, 2026-03)
beats RLM by up to 22% at equal compute *without recursion* via
sample-k-programs-and-select with uncertainty scoring — recursion is not the
performance driver; program *search* over the environment is. **Fact (input 08):**
RL-trained native RLMs now exist (Qwen3.5-4B matching a frontier model on
evidence-selection at ~9× lower latency), which is a spec-level argument for
keeping the verb surface small and *stable*: it is the action space a future
trained model would target.

**Fact (inputs 05, 06):** the depth-2 reproduction (arXiv:2603.02615) found
recursion depth 2 *degrades* accuracy and explodes cost ~95× (3.6s → 344.5s);
depth 1 helps complex reasoning; overthinking hurts simple retrieval. minRLM beat
the official harness (72.7% vs 69.7%) with *fewer* primitives at 3.6× fewer tokens.

**Fact (input 06):** the operator's flagship prior crate (ORG-1) implemented the
whole family in Rust in Feb 2026: three pluggable `ContextStrategy` variants
(Classic/Rlm/SemanticGraph), an RLM engine whose module doc states "the agent's own
conversation history IS the document," a bespoke line DSL and a sandboxed Lua 5.4
VM coexisting (**Observation, input 06:** the DSL came first and Lua superseded
it — "every LLM knows Lua"), max_depth 1, output caps, and a `RecallTool` packaging
RLM as an opt-in tool over an incrementally updated shared doc. **Gap: the
three-strategy benchmark was built but never run — no comparison data exists.**

### 4.2 The two hard rules that actually keep the window small

Two rules recur: (1) raw bulk output never enters the root window — tools beyond
the environment are callable only by sub-LMs, the root sees digested results;
(2) environment output is hard-capped per turn (8,192 chars).

Attribution contradiction recorded (review finding 6): input 05 lists both rules as
Prime Intellect *deltas from* the MIT version (whose root keeps `custom_tools`);
input 06's ORG-4 analysis attributes them to the paper itself. The pin-level truth
is unresolved here; it does not matter for the decision.

**Recommendation:** both rules are kernel law in kittens-code, enforced by the core
(typed output budget on every RLM verb and every tool result surfaced to the root),
not by prompt convention.

### 4.3 RLM and compaction are complements — the evidence

**Operator constraint:** RLM does not replace compression; both run together, with
standing one-line reminders of the RLM capability.

**Fact (input 05):** MEMTIER (arXiv:2605.03675) measured that **62% of
context-compaction events produce a behavioral break** with flat memory, and
tool-execution success decays 14pp over 72h — the strongest published argument
that destructive compaction alone is architecturally insufficient.
**Fact (input 05):** ACE documents "context collapse" from iterative summarization;
the operator's ORG-4 research independently ranked compaction strategies and
documented the "compaction death spiral" with a circuit breaker as mitigation.
**Fact (input 08, new):** the 2026-08-02 online-compaction study observed
*compensatory re-search behavior* — agents re-grep to recover weakened context
after compaction — which is the complementarity thesis caught on camera: when the
window is thinned, agents reach for exact search if and only if a search surface
exists.
**Fact (input 05):** the closest measured analog to the standing-reminder principle
is Prime Intellect's DeepDive result: RLM under-performs without environment tips
and roughly doubles with them.

**Observation:** prime-agent already *is* the hybrid (threshold compaction +
always-queryable JSONL + async GC), but no controlled ablation isolates the
combination — the 08 sweep re-searched and confirmed none exists.
**Gap: no published study compares {compaction alone} vs {RLM alone} vs {both} —
eval E1 (§8.5) would be the first.**

**Recommendation:** the transcript store is append-only from day one and is
simultaneously (a) the L3 query target, (b) the crash-recovery/replay log, and
(c) the swarm-readable surface. Live-window compaction follows §3 mechanics
(prefire scheduling off the critical path, microcompact first, tool-call/result
atomicity, sticky failure suppression, re-inject-from-store rather than
re-summarize, circuit breaker) and *never* deletes log records. One standing
reminder line advertises the query verbs and the log's existence every turn; its
exact wording is an eval variable, not a constant.

### 4.4 The query surface: shell-shaped verbs, not a language

**Operator constraint:** the RLM language must be simple and familiar — closer to
shell/grep than a programming language; Python is too much, Lua acceptable,
simpler is better; REDUCE.

**Fact (input 05):** capability is surface-invariant within ~3pp across
Read/Grep/Bash vs bash-only vs code-only agent surfaces (arXiv:2607.10569) — the
surface is a *cost and sandboxability* knob, not a capability knob. The
contradictory pair v1 dropped is restored: SWE-agent's ACI ablation claims +10.7pp
from IDE-style primitives while mini-swe-agent exceeds 74% on SWE-bench Verified
with bash only; input 05's resolution — strong recent models don't need rich
surfaces for *capability*; surfaces price differently per (task-regime, model) —
is adopted here. CodeAct's +20% for code-actions applies to *general tool
composition*, not context interaction; minRLM shows primitive economy beating
primitive richness for exactly our use case.

**Observation (was overstated as Fact-grounded in v1):** a small verb surface is
*sufficient* for transcript interaction — supported by RLM transcripts mostly
emitting grep/slice/partition patterns, minRLM's win, and the surface-invariance
result, but the direct ablation does not exist (Gap → eval E2).
**Observation (input 06):** the operator's own migration ran bespoke DSL → Lua,
with shell tried separately ("text CLIs beat structured tool calling because Unix
is in training data since the 1970s"); the bespoke grammar lost because models
were never trained on it.

**Recommendation:** the core RLM surface is a fixed verb set, one verb per line,
Unix-flavored: `grep` (regex over log/store with context lines), `slice`
(byte/line/turn ranges), `head`/`tail`, `count`, `partition` (by turns/bytes/regex),
`ask` (sub-LM query over a selection; batched form `ask-each`), `final` /
`final-var`. Every verb carries the §4.2 output budget. Two additions motivated by
input 08: keep the surface *stable* across versions (it is a future RL action
space), and admit a cheap `sample-k` selection mode on `ask` (SRLM's
search-not-recursion finding) as an eval arm, not a default.
A real interpreter (sandboxed Lua via mlua, the ORG-1 wiring: io/os/require
stripped, sync-channel bridge to async sub-LM calls, iteration cap) ships as a
**std-shim feature only** — escape hatch and eval axis, not core law. piccolo
(no_std Lua, *synthesis-introduced*, verified in input 08) is rejected for v1 and
revisited post-1.0.

### 4.5 Recursion policy

**Recommendation:** RLM recursion depth 1 by default; `ask`/`ask-each` fan-out is
the proven mechanism (paper, prime-agent, and ORG-1 all effectively ran depth ≤1);
deeper recursion is admitted only as an explicitly budgeted resource (token +
wall-clock meters per recursion node). Precision forced by review finding 11: this
is *RLM recursion* depth — subagent *spawn* depth is a different budget (Claude
ships spawn depth 3) and is decided in the spec's subagent section, not here.
Budget metering is the visible differentiator a Rust implementation can own: the
RLM line's acknowledged open wound is unbounded cost tails (§4.1).

## 5. Context tiers: L1 / L2 / L3

**Operator constraint:** think CPU cache levels; quick embedding lookup for topics,
slower search when needed; give the harness context to orient itself in its own
context.

### 5.1 L1 — the live window

§3 mechanics apply (prefire scheduling, microcompact, reminder channel, managed
cache boundary). One structure worth stealing verbatim — **Fact (input 07):**
Grok's post-compaction window layout is a fixed, ordered recipe: `[system,
user_info, rules reminder, last user query, verbatim tail since last real user
turn, summary, reminders]`, with the tail split guaranteed never to orphan a tool
result. **Recommendation:** specify the post-compaction layout as a typed,
testable structure in the protocol crate, not prompt convention.

**Serving-layer co-design (new, input 08):** compaction scheduling interacts with
the serving stack's KV cache, not just the prompt: delayed compaction beats
immediate (arXiv:2608.00902), and MemDecay's harness-annotated semantic region
labels sketch a concrete harness↔serving contract. **Recommendation:** the
protocol crate's window-layout type should be *emittable with region labels* so a
capable serving layer can exploit them; no v1 behavior depends on it.

### 5.2 L2 — the embedding hint layer

**Operator constraint (mid-flight addition):** embeddings need only be "super good
enough"; the embedding system is pluggable per execution target.

**Fact (input 05):** MemGPT/Letta archival memory and MEMTIER's semantic tier
support an embedding tier; the counter-signal is the RLM paper itself —
programmatic grep/partition beat ReAct+BM25 retrieval 100% vs ~60% on
BrowseComp-Plus. **Fact (input 05):** normalize signals before mixing lexical and
dense scores (MEMTIER). **Fact (inputs 06, 07):** cheap-and-pluggable precedents:
a prior personal experiment ran local MiniLM with an OpenAI→TF-IDF *fallback
chain*; Grok's experimental memory uses sqlite-vec + hybrid search + MMR.

**Fact (input 08, closes v1 open question 4 for std/WASM):** Model2Vec/potion
static embeddings are the concrete "super good enough" candidate class: distilled
static token-lookup models, ~8–30MB, int8-quantizable to 25% size with no
performance loss, with an **official Rust implementation (model2vec-rs) carrying a
`wasm` feature flag**. v1's hash-based-only framing for WASM was too pessimistic.
The MCU story remains open: no no_std port exists; an int8 lookup table is
flash-mappable in principle. The benchmark question sharpens to
"potion-8M-int8 vs char-n-gram hashing" (eval E3).

**Observation:** evidence ranks L3-exact above L2-approximate for correctness;
L2's value is latency and topic-level orientation, exactly as the operator framed
it. **Recommendation:** define an `Embedder` port in the core (embed: text →
fixed-dim vector; plus a `Similar` index port), target-selected implementations:
std = model2vec-class local model (or API when allowed); WASM = model2vec-rs wasm
build or host-provided; MCU = hash-based or disabled. L2 answers are always
*hints* carrying provenance (log offsets) so the model can verify via L3 before
trusting. Ship order: **L3 first, L2 second** — measured latency decides when L2
earns its place.

### 5.3 L3 — ground truth search

**Recommendation:** exact search over the append-only log: regex/substring +
turn/time/type filters, implemented over the abstract store so it runs on a
memory-mapped file (std), a WASI file (wasm), or a flash-backed store (MCU).
**Observation (honest contradiction):** the `regex` crate's full engine is
std-bound; `regex-automata`'s DFA layer supports no_std — the core may need a
reduced pattern dialect off-std. Spec decision D7. Interacting decision recorded
per review finding 12b: the store *codec* (JSONL in shims vs postcard on MCU)
changes what "grep the log" means off-std — D7 and the codec decision must be
resolved together.

## 6. Swarm: cross-harness context reads as a mount

**Operator constraint:** a harness should cheaply look up other harnesses' context —
that unlocks agents reading each other's thoughts; must be modular, plug-in/out
for evals.

**Fact (inputs 02, 04, 06, 08):** prior art falls into four families, none of which
is transcript read-mounting:
(a) **message passing** — Claude agent-teams inbox + shared task list, the
operator's harness-bus daemon (agents discover/spawn/message via MCP), A2A
protocol, Mesh Memory Protocol (field-level messages with lineage, explicitly not
raw log reads);
(b) **shared written artifacts** — Amp's URL-shareable threads, blackboard-line
systems (MetaGPT message pool, CAMEL shared memory), swarm coordination via
git/GitHub as substrate;
(c) **governed memory services** — MemClaw/Governed Shared Memory (curated
multi-tenant store with four-level scopes; agents never touch raw transcripts) —
its scope model is ready-made prior art for mount access control;
(d) **representation-level sharing** — DroidSpeak/KVCOMM/QKVShare share KV caches
across agents at the serving layer (70%+ reuse, 7.8× prefill speedups) — a
serving-layer sibling of "reading thoughts," model-locked and not queryable.

**Fact (input 08):** a deliberate falsification attempt found no system where one
harness mounts another's transcript read-only and queries it. The bet is original
as of 2026-08-08.

**Counter-signal (restored per review finding 9) — Observation (input 06):** the
operator's own strongest prior swarm result is that *isolation-first* coordination
(git clones/branches/PRs as the only shared substrate) "beat any shared-context
scheme tried"; "isolation was the feature." The read-mount bet must beat that
baseline, not a strawman.

**Observation:** kittens-code's §4 design makes read-mounting nearly free: every
harness already maintains an append-only, replayable, RLM-queryable log. Reading
another agent's thoughts is *mounting its store read-only* under a namespace and
pointing the same verbs at it: `grep --peer builder-2 "panic in reactor"`. No new
protocol, no new query language, no copy; a peer's L2 index can mount alongside as
hints. What is genuinely novel vs families (a)–(d): harness-level, model-agnostic,
query-shaped access to another agent's *raw* history.

**Recommendation:** a `ContextExchange` port in its own crate
(`kittens-code-swarm`): enumerate peers, mount/unmount peer stores (read-only),
resolve peer log offsets to typed records; scope levels borrowed from the
governed-memory family (own/team/all, deny-by-default). Transport is a shim detail
(same filesystem, socket, relay); the core sees only mounted stores. The crate is
optional at the workspace level — eval E4 runs {isolation-only baseline} vs
{+read-mounts}, with the operator's prior isolation result as the null hypothesis.
Write-side coordination (task lists, inboxes) is out of scope for v1 — that
family is well-occupied.
**Hypothesis:** read-mounts materially improve multi-harness task outcomes over
isolation-first coordination. No data exists anywhere; this is the project's most
original falsifiable claim.

## 7. Portability: no_std core, virtual IO, virtual FS

**Operator constraint:** must run on microcontrollers and WASM; virtual IO and
virtual filesystem independent of std.

**Fact (input 03):** the stack exists and is pinned: `embedded-io`/`-async` 0.7
(98M downloads, bidirectional std/tokio adapters) as the byte-stream vocabulary;
`reqwless` 0.14 + `embedded-tls` 0.19 (TLS 1.3, ~16KB/conn) for no_std HTTP;
serde/serde_json with `alloc`; `crop` rope (no_std-capable); `postcard` for
compact persistence; Embassy 0.10 as the no_std executor world the kittens kernel
already targets. WASM: wasm32-unknown-unknown (host-import IO, Cloudflare `worker`
0.8.5) and wasm32-wasip2 (typed wasi-http/fs; wasip3 async promotion in flight).
**Fact:** there is no standard no_std VFS trait — vfs and cap-std are std-only;
kittens-code must define its own ~6-method `Vfs` port (in-memory, std/cap-std,
WASI, littlefs2 impls). **Fact:** WASI's preopen/deny-by-default capability model
doubles as the sandbox model (input 04 reached the same conclusion from the
security side). The shadow-snapshot checkpoint layer (§3.2) composes with the same
port as copy-on-write.

**Fact (input 06, cautionary):** the operator's prior stack already carried
VirtualFs/VirtualExecutor traits everywhere, and the prior-work report flags the
trait-indirection tax on every tool as a real cost paid without the MCU ever
materializing. The constraint stands; the tax must be paid consciously: ports at
the *effect* level (few, coarse), not wrappers around every std call. Eval E5
makes the tax visible instead of assumed.

**Fact (input 03):** honest risk list for the core: TLS certificate verification is
std-only in embedded-tls (no_std reality = pinned roots/PSK/unverified or
hardware/proxy termination); wall-clock and entropy must be injected ports;
DNS/sockets are always shim-side; realistic RAM floor ~64–128KB — ESP32-S3-with-
PSRAM class comfortable, RP2040 workable only with a spill-to-flash store.
**Observation (inputs 03, 04, 08):** no full agent harness has shipped on
bare-metal Rust or a WASM core; the 08 sweep re-verified. kittens-code would be
first. De-risk step inherited from input 03 (restored per review finding 18): an
early ESP32-S3+PSRAM spike driving a real LLM endpoint through reqwless, before
the spec freezes MCU claims.

**Recommendation:** sans-io discipline, two layers deep:

1. `kittens-code-core` is a pure state machine — `handle_event(Event) → Effects` —
   no IO, no clock, no entropy, no async runtime; the kittens reactor *hosts* it,
   mapping sources (user ops, model deltas, tool completions, timers, swarm
   notifications) onto reactor arms with the shutdown-prefix/starvation law the
   kernel already enforces. Precedent: quinn-proto, rustls Connection, smoltcp.
   kittens-code thereby becomes the kernel's second forcing fixture — the
   agent-harness profile the root SPEC already names as a consumer.
2. Effects are discharged by shims through the ports: `Http` (SSE-capable), `Vfs`,
   `Exec` (absent on MCU), `Clock+Entropy`, `Store` (append/scan/read of the log),
   `Embedder`/`Similar` (§5.2), `TokenCount` (§9 D-new-a). CI compiles core for
   `thumbv7em-none-eabi` and `wasm32-unknown-unknown` from day one (extend the
   kernel repo's existing gate), because a single stray `std::time::Instant` in a
   dependency is how this dies silently.

## 8. Recommended shape (all Hypothesis until a slice compiles)

### 8.1 Crate hierarchy

Small on purpose — Codex (~110) and Grok (~70) both pay a sprawl tax; Goose's
types/impl split and pi's minimal kernel are the models.

```
crates/
  kittens-code-protocol   no_std+alloc. Op/Event enums, items, approval+sandbox
                          policies as data, post-compaction layout type (with
                          optional region labels, §5.1), budgets. serde only.
                          The only contract frontends see.
  kittens-code-core       no_std+alloc. Sans-io turn engine on kittens reactor;
                          transcript log model; compaction engine; RLM verb
                          parser+executor; ports (Store, Vfs, Http, Exec, Clock,
                          Entropy, Embedder, Similar, TokenCount); budget meters.
                          Modules, not crates.
  kittens-code-tools      no_std+alloc where possible. Minimal kernel set:
                          read/write/edit(fuzzy-fallback)/exec/grep over ports;
                          grammar-constrained patch application (Codex shape,
                          streaming parser); COW checkpoint layer over Vfs.
                          Everything else lives outside the kernel.
  kittens-code-swarm      Optional. ContextExchange port + mounts + scopes (§6).
  kittens-code-std        Host shim: tokio, reqwest/rustls SSE, cap-std Vfs,
                          real exec + sandbox policies, ACP adapter, JSONL store,
                          Lua escape hatch (mlua), model2vec embedder, real
                          tokenizer-backed TokenCount.
  kittens-code            Binary: wires core + std shim + kittens-tui frontend.
fixtures/
  code-no-std             Link gate: protocol+core+tools on thumbv7em/wasm32.
```

Boundaries not owned here (Operator constraint): `kittens-tui` (separate harness)
owns rendering; kittens-code speaks to it only through the protocol event stream —
the Grok lesson (input 07) is that the TUI must be an ordinary client with no
privileged path. ACP compatibility lives in the std shim — **Fact (input 04):**
25+ agents ship ACP including Gemini CLI/OpenCode/Goose/Cline; **Observation:**
it is the de facto client boundary — giving Zed/JetBrains/Neovim for free without
contaminating the core.

### 8.2 Protocol and loop

Codex Op/Event with submission-id correlation as the wire shape; Grok's
channel-sender-satisfies-the-trait pattern as the in-process shape; turn law =
sample→tools→resample; stationarity guard (§3.2, single-lineage steal) instead of
iteration caps; one cancellation lineage threaded end-to-end (the kittens
shutdown-prefix law models this); approvals as data round-trips; parallel tool
execution with serial approval (Grok's split); subagent spawning as a tool with
per-spawn isolation policy (§3.1 item 6) — whether v1 ships it at all is spec
decision D-new-c.

### 8.3 Context engine

Store: append-only record log (JSONL codec in shims; postcard option for MCU —
codec×search interaction is D7-linked), records = protocol events (Grok model:
raw notifications are the log). Window: prefire scheduling + microcompact +
circuit breaker + typed post-compaction layout. RLM: §4 verbs, output budgets,
depth-1, standing reminder. Tiers: L3 exact search core-mandatory; L2 Embedder
port optional per target.

### 8.4 What kittens (the kernel) buys here

**Observation:** the agent loop is precisely the shape the kernel was built to
check: shutdown (user interrupt) must precede the model-delta firehose; tool
completions may-remain-ready and need drain bounds; interjections are a
lower-priority source with yields_to on the streaming source; compaction prefire
is an after_event-phase job. The K0 report's open question — "does the reactor
law help a real harness?" — gets its first production answer inside this crate.

### 8.5 Eval axes (the refinement engine)

**Fact (input 08):** harness effects are 10–20 Terminal-Bench points; Harness-Bench
provides the disclosure/attribution methodology. The eval harness therefore runs
two rigs: the deterministic mock-LLM jail from the operator's prior work (scripted
model responses, behavioral capture — input 06, ORG-5) for mechanism-level
regression, and Terminal-Bench 2.0 as the external yardstick.

- **E1 context law:** compaction-only vs RLM-only vs both (± standing reminder) —
  the never-run ORG-1 comparison and the literature's missing ablation, finally run.
- **E2 query surface:** verb set vs verb set + Lua escape hatch vs `ask sample-k`
  (cost and outcome per task regime).
- **E3 tiers:** L3-only vs L3+L2 (latency, verification rate, wrong-hint damage);
  potion-8M-int8 vs char-n-gram hashing as the L2 candidates.
- **E4 swarm:** multi-harness task, isolation-only baseline (the operator's prior
  best) vs +read-mounts.
- **E5 portability tax:** std-native shortcut vs ports-everywhere core on the same
  tasks (compile-time and runtime overhead made visible, not assumed).

## 9. Confidence and open questions

| Claim | Confidence | Why |
|---|---|---|
| §3.1 convergent skeleton (items 1–5, 7) | high | 5+ lineages, corroborated at n=13 (input 08); counter-signals now recorded inline (Plan/Act modes, Gemini soft split) |
| §3.1 item 6 (depth-capped subagents, one summary) | medium | convergent for those halves; isolation is a contradicted policy axis |
| Append-only log + resume-as-replay + RLM-queryable store | high | Codex/Grok/prime-agent all ship it; MEMTIER quantifies the alternative's cost |
| RLM + compaction together beat either alone | medium | prime-agent hybrid + MEMTIER + compensatory re-search (2608.00902) are circumstantial; direct ablation still unrun — E1 |
| Shell-verb surface sufficient for context interaction | medium | surface-invariance + minRLM + operator migration, against the recorded ACI/mini-swe contradictory pair; ablation missing — E2 |
| Standing one-line reminder materially helps | medium | PI env-tips ~2× on DeepDive is the only analog; transfer unproven |
| RLM recursion depth-1 cap, budgeted deeper | medium-high | one reproduction study (2 models) shows 95× depth-2 regression; no shipped RLM runs depth>1; SRLM shows recursion isn't the driver |
| L2 embeddings as pluggable hint layer, L3 first | medium-high | grep-beats-BM25 + operator constraint align; model2vec-rs closes the WASM path; latency case unmeasured — E3 |
| Swarm read-mounts improve multi-harness outcomes | unknown | zero prior art (falsification attempted, 4 near-miss families recorded); operator's own isolation-first result is the null hypothesis — E4 |
| no_std+alloc sans-io core is feasible | medium | every ingredient pinned and real; no full-system precedent; TLS verification genuinely hard off-std; ESP32-S3 spike is the de-risk gate |
| kittens reactor is the right host for the loop | medium-high | loop shape matches kernel law 1:1 on paper; K0 has never hosted a real harness |
| Crate split (§8.1) | low-medium | reasoned from others' sprawl pain; unvalidated until slices build |

Open questions carried into the spec (spec-blocking marked ✋):

1. ✋ **kittens-tui interface** — owned elsewhere; the protocol event stream must be
   negotiated with that harness before the frontend seam freezes. **Gap: interface
   unknown.**
2. ~~no_std Lua (piccolo)~~ — **closed (input 08):** rejected for v1; revisit
   post-1.0 for its fuel-metered sandbox.
3. ✋ **Reduced regex dialect off-std (D7) × store codec** — regex-automata subset
   vs hand-rolled matcher, jointly with JSONL-vs-postcard record encoding; needs a
   spike, resolved together.
4. **Embedding floor** — narrowed (input 08) to a two-candidate benchmark:
   potion-8M-int8 vs char-n-gram hashing (E3); MCU tier still open (no no_std
   model2vec port).
5. **TLS story on MCU** — pinned roots vs proxy termination; decide per forcing
   fixture (ESP32-S3 spike), do not generalize early.
6. ✋ **Model-client wire dialects** — Grok maintains three; v1 scope decision
   (Anthropic-style first vs two dialects).
7. ✋ **Token-estimation seam** — bytes/4 rejected (input 07) and tokenizer-in-core
   rejected (§1); compaction triggers need *some* counter: `TokenCount` port with
   provider-usage feedback (Grok's hybrid) is the candidate; decide in spec.
8. ✋ **Subagent/task tool in v1?** — §3.1 adopts the pattern; no §8.1 crate owns
   it yet; ship-or-defer is a scope decision.
9. **Compaction summarization ownership** — who holds the summary prompt and model
   call: protocol data (portable, evolvable) vs shim (simple); decide in spec.
10. **Harness self-optimization** (Meta-Harness/HarnessBridge line, input 08) —
    declared non-goal for v1; recorded so future harnesses don't re-litigate
    silently.

## 10. Lineage log

- 2026-08-08: operator directive — start kittens-code research; vectors: Grok
  Build / Claude Code / Codex / OSS pieces; RLM (prime-agent as pure-RLM
  reference); prior internal + personal repos (redacted); lessons: simple RLM
  language, RLM+compression both, reminders, cache-tier context, modular swarm
  reads, no_std/WASM virtual IO. Mid-flight addition: pluggable good-enough
  embeddings.
- 2026-08-08: seven-vector parallel fan-out; inputs 00–07 frozen; synthesis v1.
- 2026-08-08: refinement pass 1 — adversarial review (input 09: 3 blockers,
  9 major, 8 minor, redaction clean, no leak-code usage) and fresh-eyes SOTA sweep
  (input 08: harness-engineering literature, SRLM, swarm falsification attempt
  failed → bet original, model2vec closes WASM embedding path, piccolo rejected,
  delayed-compaction evidence). All findings dispositioned; this file is v2.
- Next: spec drafting (`docs/kittens-code/SPEC.md`), inheriting this file's labels
  and section numbers; spec-blocking questions are §9 items marked ✋; the
  kittens-tui seam (Q1) is negotiated with the owning harness, not decided
  unilaterally here.
