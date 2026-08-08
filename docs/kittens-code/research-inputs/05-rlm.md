# RLM Research Report — for kittens-code design

## 1. Original RLM (Zhang, Kraska, Khattab — MIT)

**Sources:** Blog Oct 2025: alexzhang13.github.io/blog/2025/rlm/ ; Paper: arXiv:2512.24601 (Dec 2025); Official lib: github.com/alexzhang13/rlm (MIT, pinned HEAD `72d6940142ddfb84ee6be573dc999a37e633e671`, 2026-06-26).

**Mechanism (Fact):**
- Prompt is NOT tokenized into the root LM. Loaded as a Python variable (`context`) in a persistent REPL (Jupyter-style).
- Root LM iterates: write code cell -> observe truncated output -> repeat. Peek/slice/grep/partition `context` programmatically.
- Primitives (official lib): `llm_query` / `llm_query_batched`, `rlm_query` / `rlm_query_batched` (recursive, parallel via `max_concurrent_subcalls`), user `custom_tools`, `context` variable, `history` for multi-turn. Terminate via `FINAL(answer)` or `FINAL_VAR(varname)`.
- Paper only tests recursion depth 1. Sub-calls run in isolated environments; only results return.
- Sandboxes in lib: local exec, ipython, docker, modal, prime, daytona, e2b.

**Numbers (Fact, blog):**
- OOLONG 132k tokens: RLM(GPT-5-mini) ~63 vs GPT-5 ~29 (+34 pts, ~114% relative) at comparable cost; GPT-5-mini alone ~15. At 263k: RLM still +15 pts (~49%) over GPT-5.
- BrowseComp-Plus @ 1,000 docs (~10M tokens): RLM(GPT-5) 100%; RLM without recursion 90%; ReAct+BM25 ~60%; truncated GPT-5 ~20%.
- Cost "scales reasonably"; no hard cost/runtime bounds (seconds to minutes per query).

**Failure modes (Fact):** blocking subcalls, no prefix-cache reuse; runtime variance; unbounded cost tail; counting/aggregation degrade with length; context rot persists in mini root model.

## 2. Prime Intellect

**RLM blog (Jan 2026, Sebastian Müller):** primeintellect.ai/blog/rlm — RLM in `verifiers` framework (github.com/PrimeIntellect-ai/verifiers), `RLMEnv`, training via prime-rl.
- Key deltas from MIT version (Fact): tools beyond Python REPL usable ONLY by sub-LMs (root never sees raw tool output); output via env variables; iterative answer "diffusion" through editable `answer` variable; REPL output capped 8192 chars/turn; explicitly NEVER summarizes context ("Bitter Lesson" stance — delegate, don't compress).
- Results (Fact): Oolong — plain LLM hits zero beyond short contexts, RLM holds to ~1.5M chars; verbatim-copy — RLM "strictly dominates" except UUIDs; math-python — RLM WORSE than baseline (scaffold alone doesn't help learned skills); DeepDive — RLM underperforms without env tips, ~2x with tips (GLM 4.6). Wall-clock significantly worse everywhere. Thesis: real gains need RL training of context-management behavior.

**prime-agent** (harness): primeintellect.ai/blog/prime-agent ; github.com/PrimeIntellect-ai/prime-agent (MIT, 8.4k stars, pinned HEAD `a18809e00ea30638584d87b3afea7285a9d7296c`, 2026-08-07).
- Design (Fact): persistent IPython kernel is the ONLY tool. "Prompt-as-a-variable" + programmatic tool/sub-agent calling. Sub-agents = full prime-agent instances with own session dir/kernel/history, spawned via `await rlm("sub-task")`; async fan-out returns handles, results via `agent_message.send()`. History = append-only JSONL, recoverable via `/tree`. Compaction on threshold or `compact.run()`, with async "garbage collector" agent compacting while kernel keeps running. Continual Harness: H=(prompt, sub-agents, skills, memory) as CRUD-editable state; `/refine` reads own trajectory, applies smallest CRUD edit. Skills = importable Python packages.
- Evals (Fact, their numbers): ARC-AGI-3 with Opus 5: 95.5% RHAE Best@1 (human expert 95.4%), 99.97% Best@3. GLM-5.2 long-context: OOLONG 0.700, OOLONG-Pairs 0.874, LongBenchPro 0.777, EmulatorBench 0.208. Observation: no model trained for this harness yet — prompting-only results.

## 3. Follow-up / ecosystem since Oct 2025

- **Reproduction/critique** (Fact): "Think, But Don't Overthink: Reproducing RLMs" arXiv:2603.02615 (Mar 2026), DeepSeek v3.2 + Kimi K2. Depth-1 helps complex reasoning; depth-2 DEGRADES accuracy and explodes cost (3.6s -> 344.5s, ~95x). Overthinking hurts simple retrieval. Implication: cap depth at 1 by default.
- **minRLM** (Fact): avilum.github.io/minrlm + github.com/avilum/minrlm (pinned `eac44fe8c0847680d73864e1698f15b3aec0089a`). GPT-5-mini: 72.7% vs 69.7% official RLM vs 69.5% vanilla, at 3.6x fewer tokens — leaner primitive set beat the official harness (Observation: primitive economy > primitive richness).
- Other repls: fullstackwebdev/rlm_repl, grishahq/recursive-llm.
- "Context folding" line (Fact): AgentFold arXiv:2510.24699, Context-Folding arXiv:2510.11967 — hierarchical fold/collapse; PI calls RLM "the simplest, most flexible" of these.
- Memory-hierarchy lineage: MemGPT (arXiv:2310.08560, OS paging -> Letta); ACE / Agentic Context Engineering (contexts as evolving playbooks; "brevity bias" and "context collapse" as failure modes of iterative summarization).

## 4. RLM vs compaction — evidence for the layered (L1/L2/L3) design

- **Against naive compaction (Fact):** MEMTIER, arXiv:2605.03675 (v3 May 2026): tool-execution success degrades 14pp over 72h with flat-file memory; **62% of context-compaction events produce a measurable behavioral break**; flat-text retrieval confuses entity relations with co-occurrence. Tripartite architecture (episodic JSONL + semantic tier + five-signal weighted retrieval) lifts LongMemEval-S 0.050 -> 0.382 with a 7B model. Also: normalize signals before mixing lexical + dense (unnormalized BM25 dominates).
- **Against summarization-only (Fact):** ACE's "context collapse"; PI's information-loss argument; MemGPT explicit paging works but model must remember to page.
- **For combining (Observation):** prime-agent already IS the hybrid: threshold compaction of live window + append-only JSONL always queryable + async GC. **Gap: no controlled study of the exact L1/L2/L3 combination (no data exists).**
- **Periodic-reminder idea:** no direct prior art; nearest evidence PI DeepDive: RLM underperforms *without env tips*, ~2x *with* tips — cheap standing hints have outsized measured effect (Fact for their setting; Hypothesis it transfers).
- **Embedding L2 tier:** MemGPT/Letta archival + MEMTIER semantic tier support it; counter-signal: RLM paper found ReAct+BM25 clearly worse than programmatic grep/partition on BrowseComp-Plus (60% vs 100%). Hypothesis: embeddings as fast *hint* layer, grep as ground truth, is the defensible split.

## 5. Minimal query language vs full Python

- **CodeAct** (arXiv:2402.01030, Fact): unified code-action space beats JSON/DSL tool calls by up to +20% absolute, up to 30% fewer actions. Argument: control flow + composition.
- **Regime x agent ablation** (arXiv:2607.10569, Fact): baseline (Read/Grep/Glob/Edit/Write/Bash) vs bash_only vs code_only: pass rates within 3pp across ALL cells — tool surface affects *cost*, not capability. code_only cost: Artifact/Claude -24.6% (p=7e-14), SWE-bench/Codex -19.9% (p=2e-9), SWE-bench/Claude +14.4% (NS). Edit friction is the cost driver when everything must be a script.
- **Contradictory pair (Fact):** SWE-agent ACI ablation claims +10.7pp from IDE-style primitives; mini-swe-agent >74% SWE-bench Verified with bash only. Resolution (Observation): strong recent models don't need rich surfaces for capability; surfaces are an efficiency knob per (task-regime, model).
- **Gap: no published ablation of a restricted grep-DSL vs full Python specifically for *context/transcript* interaction (no data exists).** Closest: minRLM winning with fewer primitives; RLM root models mostly emit grep/slice/partition patterns anyway (Observation from blog transcripts).

## 6. Implications for an RLM-native Rust coding agent

1. Core loop is cheap: context-as-variable + iterate(code -> truncated output) + llm_query/rlm_query(batched) + FINAL/FINAL_VAR. That is the whole contract; rest is environment choice.
2. **Depth cap = 1 by default** (depth-2 is a 95x time bomb). Make depth a budgeted resource.
3. **Adopt prime-agent's two hard rules:** raw tool output never enters root context (sub-LM-only tools); REPL output hard-capped per turn (8192 chars). These two mechanisms actually keep the root window small.
4. **Transcript = append-only JSONL from day one.** Simultaneously the L3 tier, crash-recovery, and RLM query target. Compaction only ever discards *window* copies, never information — dodges MEMTIER's 62%-behavioral-break finding.
5. **Layered lookup:** L1 live window (compacted), L2 optional embedding index as hint layer, L3 ripgrep over JSONL as ground truth. Rust: ripgrep-as-a-library (`grep` crate) makes L3 near-free — that weakens the case for L2 — ship L3 first, add L2 only if measured latency demands.
6. **Standing capability reminder:** one-line env tip each turn — cheapest intervention with strongest measured analog (PI DeepDive ~2x).
7. **Query language:** capability is surface-invariant (within 3pp) — choose surface for cost and sandboxability. Small verb set (peek/slice/grep/partition/count + llm_query) defensible and easy to sandbox in Rust; keep escape hatch to a real interpreter (embedded Lua or jailed IPython) for regimes where code_only wins. Native Edit/Write primitives stay for file modification (edit-friction finding).
8. **Cost control is the open wound:** original RLM has no cost/runtime bounds. Rust harness can differentiate with hard budgets: token/wall-clock meters per recursion node, prefix-cache-aware subcall scheduling, async fan-out (handle+message pattern).
9. **Don't expect scaffold-only wins on skill tasks** (PI math-python regression). RLM buys long-context capability, not reasoning; position it as the memory subsystem, not the intelligence.

Key URLs: arXiv 2512.24601, 2603.02615, 2605.03675, 2607.10569, 2402.01030, 2510.24699, 2510.11967; alexzhang13.github.io/blog/2025/rlm/; primeintellect.ai/blog/rlm; primeintellect.ai/blog/prime-agent; github.com/PrimeIntellect-ai/prime-agent @ a18809e; github.com/alexzhang13/rlm @ 72d6940; github.com/avilum/minrlm @ eac44fe.
