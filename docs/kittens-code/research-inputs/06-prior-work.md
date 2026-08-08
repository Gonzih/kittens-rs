# RLM-Harness Prior-Work Mining Report

Sources: prior internal harness experiments (org repos, ~30 total) + Gonzih personal repos (200 scanned, ~15 relevant). Inspected = cloned/fetched source or README. Forks of third-party agents excluded. REDACTION: org name never written; org repos codenamed ORG-1..ORG-7.

## Org repos (codenamed)

**ORG-1 — async agentic runtime crate (Rust, public on crates.io, last commit 2026-03-07)** — the flagship. Fact (inspected source): full agent runtime — steerable AgentLoop (mpsc event/steering channels), multi-provider LLM abstraction, VirtualFs/VirtualExecutor traits for WASM-first portability, hook pipeline (modifying/void/persist), permission gate, cost budget, MCP client, JSONL sessions, hierarchical memory, subagent spawner, and **three pluggable `ContextStrategy` variants on the loop: Classic (compaction), Rlm, SemanticGraph**.
- (a) Context querying: `rlm` module = "Recursive Language Model engine", cites arXiv:2512.24601. In-module principle: "the agent's own conversation history IS the document." Full history serialized to external doc each turn; LLM gets compact window + metadata; accesses past by writing code blocks. **Two REPL languages coexist**: bespoke line DSL (QUERY/QUERY_BATCH/CHUNK BY_LINES|BY_CHARS|BY_REGEX/SLICE/MAP/FILTER/FINAL...) and sandboxed **Lua 5.4** VM (mlua; io/os/debug/require/load stripped; `llm_query()` bridges to async sub-LLM via sync mpsc, single + batched). Engine comment: "Lua (preferred, every LLM knows Lua)" — Observation: DSL came first, Lua superseded it. Defaults: max_iterations 30, max_depth 1 (recursion never really exercised). Also `RecallTool`: agent-invocable tool running the RLM REPL over a `SharedContextDoc` (Arc<RwLock<String>> updated incrementally per turn) — RLM as opt-in tool rather than always-on strategy.
- (b) Compression: `context` module — token-estimate manager, 0.75 compaction threshold, circuit breaker (`compaction_attempted`), strategies: ReversibleOffload / Summarize / StructuredPreserve / Hybrid. Plus `semantic_recursion`: "nothing is ever lost" — context as typed graph, deactivation + `CompactedInto` edges, **6-char-hash symlinks** (`[A3F2B1]: summary`), sentence-level TF-IDF + agglomerative clustering, retrieval = keyword + TF-IDF + graph-neighbor expansion; LLM-facing tools Resolve/Search/ListSymlinks. Pure-Rust, no embedding model.
- (c) Multi-agent: `subagent` module — role-typed cheap-tier subagents (MetadataExtractor, SafetyValidator, Summarizer, General), "dual-model architecture" (expensive main + cheap subs). Context sharing = results fed back as messages, no shared memory.
- (d) `harness` module implementing AutoHarness (DeepMind 2025): learn from tool failures, synthesize shell validators, Thompson sampling over candidate verifiers. Observation: only ActionVerifier implemented; rest stubbed — abandoned mid-build.

**ORG-2 — coding-tools crate (Rust, public, Feb 2026)** — Fact: 7 tools (read/write/edit/bash/grep/find/ls) over VirtualFs/VirtualExecutor; edit has fuzzy fallback (smart quotes, unicode dashes, trailing whitespace) + unified diff output. Clean, small, reusable shape.

**ORG-3 — portable agent tools (Rust, private, Feb 2026)** — Fact: web/git/glob/todo + `rlm_query` tool ("large-context analysis via RLM engine", WASM-capable, provider passed in). RLM was exposed as a plain tool in production composition.

**ORG-4 — agent-architecture research repo (private, Feb 2026)** — Fact (read research/recursive-language-models.md): RLM paper analysis (two data paths, sub-LLM llm_batch(), 8,192-char REPL output cap, tools only for sub-LLMs, answer-dict pattern); "compaction death spiral" taxonomy (summaries-of-summaries distortion); **ranked compaction hierarchy: raw > reversible offload > virtual-file-system > pre-compaction memory flush > anchored iterative summarization > ACON failure-driven > sliding window > token-threshold > model self-management ("context anxiety")**; radical position (Amp): reject compaction, /handoff clean-break to new thread with distilled prompt. Also agent-loop-patterns.md: "each generation simplified the harness because models internalized the patterns — most effective agent loops are now the simplest ones." Also forensic captures of a jailed Claude Code (system prompt, tools.json, request recordings).

**ORG-5 — agent jail + mock-LLM + behavioral capture (Rust/Shell, private, Feb 2026)** — Fact: transparent proxy jail with scripted LLM responses for deterministic harness testing; predecessor = personal repo nexus-clojure-proxy. Observation: genuinely reusable — test harness against a fake deterministic model.

**ORG-6 — old R&D prototypes (TS/JS, private, `_`-prefixed Feb 2026)** — Dead, quarantined; superseded by Rust stack.

**ORG-7 — browser-first rich terminal (Rust, public, Feb 2026)** — GPU widget surface with terminal aesthetics. Not inspected deeply.

## Gonzih personal repos

**nexus-clojure (Clojure, May 2026, dead)** — "REPL IS the harness" — tools are namespaces, harness rewrites own tools at runtime via eval/alter-var-root/nREPL. Dead-ended within weeks; self-modification thread continued only as AutoHarness in ORG-1. Plausible reason: autopoiesis is a research toy; Rust portability won.

**nexus-reasoning-graph (TS, May 2026, dead)** — Claude Code hooks → local service → SQLite; sliding-window chunker (512 tok/128 stride), local MiniLM embeddings with OpenAI→TF-IDF fallback chain, cosine "influence edges", live D3 force graph. Provenance observability, not compression — TF-IDF-fallback trick and expansion/contraction framing survive as concepts.

**nexus-gravitas (TS, May 2026, dead)** — Datomic-style temporal memory — atomic [entity, attribute, value, tx_id] "gravits", append-only, retraction-as-new-fact, time-travel queries, influence weights, namespaced entity IDs, MCP-only interface, pgvector. Observation: append-only/never-delete instinct = same instinct as RLM's "document only grows."

**nixagent (TS, Mar 2026, dead)** — single-tool agent — sandboxed shell; thesis: "text CLIs beat structured tool calling because Unix is in training data since the 1970s." `sh` template tag + pipeline validator. Observation: shell-flavored answer to the same question Lua answered in ORG-1. Author tried shell (nixagent), bespoke DSL (ORG-1 v1), Lua (ORG-1 v2). Never Python.

**multi-brain (Rust, Jul 2026, most recent pre-kittens)** — daemon + CLI + MCP bus keeping harnesses (claude/codex/open-claw/generic) alive; agents discover, spawn, inspect, message each other; registry lockfile, stream-json piping, injected personality prompts. Context sharing = message passing between processes, nothing shared-memory.

**cc-agent / cc-suite / cc-discord (TS, Jun 2026)** — MCP server spawning Claude Code in cloned repos; create_plan = dependency graph of agent jobs; generate_workflow = NL goal → staged agents; mid-task stdin send_message; per-repo cost tracking. **swarm-agents (JS, Mar 2026)**: task discovery from issues/TODOs/failing tests, isolated clones, self-review-and-merge PRs, per-task budget. Observation: swarm coordination via git/GitHub as shared substrate — no context sharing; isolation was the feature.

**ouroboros (TS, Apr 2026, dead)** — no direct LLM API calls, `claude --print` subprocess as only intelligence; v0.2: Claude as persistent coordinator holding a control-plane MCP (spawn_worker, approve_evolution). Observation: "harness becomes an MCP toolbelt for a persistent model" inversion is a live idea; product framing died.

## Synthesis

**Lessons that survive:**
1. **Context-as-document with model agency** — strongest thread (ORG-1/3/4): history externalized, never destroyed, model queries it programmatically. RecallTool packaging (RLM-as-tool over shared incrementally-updated doc, cheap sub-model) directly liftable.
2. **Compaction hierarchy: reversible offload >> summarization**, circuit breaker on repeated compaction. ORG-4 death-spiral analysis is evidence-backed and current.
3. **Query language is a training-data question**: shell > Lua > bespoke DSL, per the author's own migration. Bespoke DSLs lose — models weren't trained on them. (Paper says Python; author bet Lua for embeddability — defensible via mlua.)
4. **Sandbox discipline for the REPL**: strip io/os/require, cap output/iteration (8K), cap iterations (30), sync-channel bridge to async sub-LLM — proven mlua wiring.
5. **Pluggable ContextStrategy on the loop** (Classic/Rlm/SemanticGraph) so strategies are benchmarkable — good crate API shape.
6. **Deterministic harness testing** via mock-LLM proxy jail + behavioral capture.
7. **Simplest loop wins** — author's own research: harness complexity should shrink as models improve.
8. **Isolation-first swarms**: git clones/branches/PRs as coordination substrate beat any shared-context scheme tried; role-typed cheap subagents as intra-loop version.

**Thinking now obsolete:**
1. Bespoke RLM DSL (QUERY/CHUNK/MAP) — superseded by Lua in the same codebase; don't resurrect.
2. TF-IDF symlink/semantic-graph engine — 2025-era retrieval; modern cheap embeddings or plain RLM-recall dominate. Appears unused beyond the module.
3. Self-modifying/autopoietic harness — twice attempted, twice abandoned half-built.
4. Temporal fact-store memory (gravits) — heavyweight for what append-only JSONL + RLM recall gives free; dead in 3 months.
5. WASM-first everything — VirtualFs/VirtualExecutor taxed every tool with trait indirection. For kittens-code it's a decision, not an inheritance (NOTE: operator has since made it a hard requirement — but design the tax consciously).
6. max_depth 1 — "recursive" LM never actually recursed; treat true recursion as unproven, sub-LLM fan-out (batched) as the proven part.

**Gap: no benchmark results exist comparing the three ContextStrategy variants — the harness to compare them was built, the comparison apparently never run.**

Timeline (Fact): personal TS experiments Mar–Jun 2026 → Rust org stack consolidated Feb–Mar 2026 (dates interleave; some nexus-* repos are stale mirrors of org work) → multi-brain Jul 2026 → kittens-rs (no_std reactor kernel) Aug 2026: compile-checked orchestration replacing runtime-dynamic harnessing.
