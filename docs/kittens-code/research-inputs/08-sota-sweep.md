# Fresh-eyes SOTA gap sweep (2026-08-08, refinement pass 1)

Assignment: hunt for what RESEARCH.md v1 missed; try to falsify its originality/gap
claims. ~14 targeted searches/fetches. Findings numbered by category.

## Category 1 — Harness design developments not in the doc

1. **"Harness engineering" is now a named research field — v1 cites zero of it.**
   "Inside the Scaffold: A Source-Code Taxonomy of Coding Agent Architectures"
   (arXiv:2604.03515, Apr 3 2026) analyzes 13 open-source coding agents at pinned
   commits, file/line grounded — the doc's own methodology, done independently at
   larger scale. Documents **seven distinct context-compaction strategies** and five
   composable control primitives (11/13 agents compose multiple primitives — mildly
   complicates §3's "flat loop is universal" framing). §3 should cite as
   corroboration and mine its compaction taxonomy for §8.3; may contain
   counterexamples to §3 claims derived from only ~7 harnesses.

2. **Harness-effect benchmarks now exist — §8.5 has published methodology to steal.**
   Harness-Bench (arXiv:2605.27922, May 27 2026): 106 sandboxed tasks, 5,194
   trajectories; capability must be attributed to model+harness configuration.
   Companion position paper "Stop Comparing LLM Agents Without Disclosing the
   Harness" (arXiv:2605.23950, May 2026). Terminal-Bench 2.0 ablations:
   **harness alone accounts for 10–20 point swings** (e.g. Gemini 2.5 Pro +17% with
   Terminus 2 over OpenHands) (benchmarkingagents.com/terminal-bench/, 2026).
   §8.5 should adopt Terminal-Bench 2.0 as external eval target alongside the
   mock-LLM jail; 10–20pt is the effect size E1–E5 hunt.

3. **New open-source harnesses since the survey window.** OpenHarness/Ohmo
   (github.com/HKUDS/OpenHarness, v0.1.0 Apr 1 2026, 15.3k stars, Python/MIT —
   43+ tools, skills-as-markdown, subagents; notably NO cross-session context
   sharing). Microsoft Agent Framework 1.0 GA with first-class "Agent Harness" +
   CodeAct component (devblogs.microsoft.com, Apr 2 2026). Neither changes §3's
   skeleton — they confirm it. Field index: github.com/ai-boost/awesome-harness-engineering.

4. **Automatic harness optimization is a new axis:** Meta-Harness/APEX-style
   scaffold co-evolution (harness patched from failure modes, evaluated on
   Terminal-Bench); HarnessBridge (arXiv:2606.12882, Jun 2026). One line in §9 as
   deliberate non-goal or future eval axis.

## Category 2 — RLM follow-ups newer than v1's list

5. **SRLM (arXiv:2603.15653, Mar 7 2026) attacks the RLM line's core assumption:**
   uncertainty-aware self-reflective program search (self-consistency + reasoning
   length + verbalized confidence) beats RLM by up to **22% at equal compute,
   without recursion**; recursion "is not the primary driver of performance."
   Strengthens §4.5's depth-1 cap; suggests cheap addition to §4.4:
   sample-k-programs-and-select on the verb surface. Add to §4.1 envelope.

6. **RL-trained native RLMs exist:** "Reinforcing Recursive Language Models"
   (alphaxiv.org blog, May 13 2026) — Qwen3.5-4B RL-finetuned as native RLM matches
   Claude Sonnet on evidence-selection (0.600 vs 0.607) at 7s vs 60s+ inference;
   child rollouts inherit parent advantages. Relevance: the verb surface kittens-code
   fixes is exactly the action space such a model trains against — spec-level
   argument for keeping the surface stable and small. Also λ-calculus/Y-combinator
   long-context formalization (arXiv:2603.20105, Mar 2026), minor.

## Category 3 — Swarm read-mounts: falsification attempt

7. **Failed to falsify — "zero prior art" survives; three near-misses to cite in §6.**
   (a) Mesh Memory Protocol (arXiv:2604.19540, Apr 21 2026): field-level MESSAGE
   PASSING with lineage tracking — explicitly not raw log reads. (b) Governed
   Shared Memory / MemClaw (arXiv:2606.24535v1, Jun 23 2026): curated multi-tenant
   memory SERVICE, four-level scopes (agent-local/team/tenant/restricted) — agents
   never touch raw transcripts; its scope/governance model is ready-made prior art
   for §6's mount access-control story. (c) Blackboard line (MetaGPT message pool,
   CAMEL share_memory, arXiv:2507.01701): shared WRITTEN store, not each other's
   private logs. Nothing found where one harness mounts another's transcript
   read-only and queries it. **E4 remains original as of 2026-08-08.**

8. **Adjacent family v1 missed: representation-level context sharing.** DroidSpeak
   (arXiv:2411.02820, 2024), KVCOMM (anchor-based cross-agent KV reuse, 70%+ reuse,
   7.8× prefill speedup in 5-agent settings), QKVShare (arXiv:2605.03884, May 2026,
   quantized KV handoff for on-device multi-agent). These share KV CACHES, not logs —
   serving-layer sibling of "reading each other's thoughts." §6 should name this
   family (d) to preempt "prior art!" objections and sharpen what's novel
   (harness-level, model-agnostic, queryable).

## Category 4 — Embeddings on constrained targets (open question 4)

9. **Concrete candidate stack exists; §5.2 + open question 4 substantially closable.**
   Model2Vec/potion static embeddings (github.com/MinishLab/model2vec): distilled
   static token-lookup models, **~30MB best / ~8MB smallest, int8-quantizable to
   25% size with no performance loss**; potion-base-8M model card on HuggingFace.
   **Official Rust implementation model2vec-rs (github.com/MinishLab/model2vec-rs)
   with an explicit `wasm` feature flag and f32/f16/i8 safetensors support**, ~1.7×
   faster than Python. Drop-in `Embedder` impl for std and WASM; for MCU, int8
   static lookup table is flash-mappable in principle (no no_std port yet — that
   part of the gap stands). Also SwiftEmbed (arXiv:2510.24793, Oct 2025).
   The "hash-based only" framing for WASM in §5.2 is too pessimistic; benchmark
   shifts to "potion-8M-int8 vs char-ngram hashing."

## Category 5 — piccolo / no_std Lua (open question 2)

10. **Piccolo not viable as core law in 2026; deferral hardens into v1 rejection.**
    github.com/kyren/piccolo — v0.3.3, pre-1.0, self-described experimental,
    frequent API breakage; resumed after multi-year hiatus; **no no_std feature
    flag exists in its Cargo.toml** (deps gc-arena/hashbrown/ahash are
    no_std-friendly, but no_std unclaimed and untested upstream); stdlib largely
    unimplemented INCLUDING `string` and `table` — fatal for "every LLM knows Lua",
    since string/table calls are what models emit. Sandboxing story (stackless,
    DoS-resilient, fuel/memory metering) genuinely excellent — the reason to
    revisit post-1.0. Open question 2 → answered: mlua std-shim stays the only
    interpreter path.

## Category 6 — Prompt-cache-aware harness design

11. **Fresh empirical result for §4.3/§5.1 compaction scheduling:** "Practical
    Online KV Cache Compaction for LLM Agents" (arXiv:2608.00902, **Aug 2 2026**).
    Findings: **delayed compaction (3–5 turns) consistently beats immediate
    compaction**; simple token eviction competitive with sophisticated attention
    matching under uncertainty; **compaction induces compensatory re-search
    behavior** (agents re-grep to recover weakened context — direct empirical
    support for §4.3's RLM+compaction complementarity thesis). Also MemDecay
    (region-aware KV eviction where the HARNESS annotates prompt segments with
    semantic region labels — a concrete harness↔serving interface contract the
    protocol crate could optionally emit). v1's cache coverage is prompt-level
    only; add a §5.1 paragraph on serving-layer co-design.

## Nothing found (confirms v1 gap claims)

- No shipped no_std/bare-metal agent harness anywhere — §7 claim holds.
- No published {compaction} vs {RLM} vs {both} ablation — E1 unrun in literature
  (SRLM and 2608.00902 circle it; neither isolates the combination).
- No restricted-verb vs full-interpreter transcript-interaction ablation — E2 holds.
- No falsifier for swarm read-mounts — E4 remains the original bet.
