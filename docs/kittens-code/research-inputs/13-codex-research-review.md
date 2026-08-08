# External research review — Codex (gpt-5.6-sol, reasoning effort ultra), 2026-08-08

Independent cross-model-family review of RESEARCH.md v2 + inputs 00-12,
requested per operator loop directive. Read-only run via `codex exec`.
Verdict: YES-WITH-CONDITIONS (findings 43-49). Findings 15-30 are factual
corrections folded into RESEARCH v3; findings 32-39 are blind spots folded
into RESEARCH v3 gaps and SPEC v0.5.


**Bottom line: YES-WITH-CONDITIONS.** The corpus can support a reversible KC0 experimental slice, but it does not yet justify freezing the three bets as established architecture; several factual corrections are required first.

## Confidence audit

Against the twelve rows in the [§9 confidence table](/Users/feral/mydev/rust-kittens/docs/kittens-code/RESEARCH.md:645):

1. **§3.1 convergent skeleton — DISAGREE; corrected: medium-high:** the inner loop and append/replay pattern triangulate well, but typed protocols, reminders, and deferred schemas do not share the claimed five-lineage support.

2. **Depth-capped subagents, one summary — AGREE; medium:** Claude and Grok support the broad pattern, while isolation policy remains contradictory and correctly prevents a higher rating.

3. **Append-only log + replay + RLM-queryable store — DISAGREE; corrected: medium:** Codex and Grok establish append/replay, but they do not establish that the same store is model-queryable, so the row conflates a high-confidence foundation with a medium-confidence extension.

4. **RLM + compaction beat either alone — DISAGREE; corrected: low-medium:** coexistence and circumstantial complementarity exist, but the corpus explicitly finds no controlled three-arm ablation supporting superiority.

5. **Shell verbs are sufficient — DISAGREE; corrected: low-medium:** general tool-surface results do not establish that a line DSL preserves the programmable decomposition responsible for RLM results, and the operator’s own migration favored Lua.

6. **Standing one-line reminder materially helps — DISAGREE; corrected: low:** the only analog is a task-specific environment-tip intervention, not a periodic one-line capability reminder.

7. **Depth-1 default, budgeted deeper — DISAGREE; corrected: medium:** the conservative policy is sensible, but its rationale relies on one small reproduction, misstates the 95× comparator, and is contradicted by current official depth-2/3 evaluations.

8. **Pluggable L2 hints, L3 first — DISAGREE; corrected: medium:** the reversibility and L3-first policy are sound, but neither MEMTIER nor Model2Vec validates transcript retrieval quality, target latency, or MCU feasibility.

9. **Swarm read-mounts improve outcomes — AGREE; unknown:** current near-prior-art invalidates the “zero prior art” rationale, but no outcome experiment establishes either benefit or harm for this exact mount interface.

10. **`no_std+alloc` sans-I/O core feasible — AGREE; medium:** the ingredients exist, while transport, persistence, verified TLS, bounded memory, and KX adapters remain unintegrated.

11. **Kittens reactor is the right host — DISAGREE; corrected: medium:** the loop maps neatly onto K0 on paper, but topology compatibility is not evidence of outcome benefit or complete cancellation ownership.

12. **Crate split — AGREE; low-medium:** it is a defensible boundary hypothesis derived from prior sprawl, but it remains unbuilt and therefore appropriately weak.

## Claim verification

13. **Source precedence:** my pre-2026 knowledge is largely silent—not contradictory—on these 2026 artifacts; the material conflicts below are between the pinned summaries and dated 2026 primary sources, which I have identified rather than silently preferring memory.

14. **RLM mechanism — VERIFIED:** storing the prompt as an external variable, iterating through a REPL, using truncated observations, and permitting programmatic subcalls accurately reflects the [current RLM paper](https://arxiv.org/html/2512.24601).

15. **RLM benchmark figures — OUTDATED/OVERSTATED:** [§4.1](/Users/feral/mydev/rust-kittens/docs/kittens-code/RESEARCH.md:236) repeats preliminary 20-query blog results—100% versus roughly 60% and approximately 63 versus 29—whereas the May 2026 paper reports 150-query BrowseComp+ results of 91.3 depth-1, 88.0 depth-0, 51.0 CodeAct+BM25, and OOLONG 56 versus 44; retain the old figures only if explicitly labeled “preliminary n=20.” ([preliminary blog](https://alexzhang13.github.io/blog/2025/rlm/), [paper v3](https://arxiv.org/html/2512.24601))

16. **“95× depth-2 cost regression” — MISLABELED:** 344.5/3.6 is indeed 95.7×, but it is **latency versus the non-RLM base**, not cost or depth-2 versus depth-1; the incremental depth-1-to-depth-2 factor is 344.5/89.3 ≈ 3.86×, from one run of 20 samples per condition without significance testing. ([reproduction](https://arxiv.org/html/2603.02615))

17. **Depth greater than one — OUTDATED:** the confidence rationale says no RLM runs exist above depth one, but the current official paper evaluates depths 0–3, with depth 2/3 matching or improving depth 1 on several tasks; this conflicts with the smaller reproduction and makes the evidence mixed rather than one-way. ([RLM Table 1](https://arxiv.org/html/2512.24601))

18. **MEMTIER numbers — NUMERICALLY VERIFIED, EVIDENCE OVERSTATED:** 14 percentage points, 62%, and 0.050→0.382 appear in the paper, but the first two derive from community-issue analysis plus the authors’ AgentRun-72 diagnostic, while the 0.382 result is a separate LongMemEval-S experiment; calling 62% the strongest causal evidence against compaction is unwarranted. ([MEMTIER v3](https://arxiv.org/html/2605.03675v3))

19. **MEMTIER as embedding-tier evidence — WRONG:** its semantic tier consists of LLM-extracted facts followed chiefly by BM25, its default dense-score weight is zero, and dense retrieval appears only as a 100-question baseline with +0.030 accuracy at 3.7× latency; it supports structured tiering, not Model2Vec specifically.

20. **“Normalize signals before mixing” — OVERSTATED AS A RESULT:** MEMTIER recommends normalization because raw BM25 dominates, but five normalization variants produced identical 0.320 accuracy in its probe and removing several auxiliary signals actually improved accuracy by 0.012–0.014.

21. **Compensatory re-search — PARTLY VERIFIED, CAUSAL CLAIM OVERSTATED:** the online-compaction study observes longer trajectories and suggests Qwen may compensate through extra searches, but it does not test a no-search control or prove the synthesis’s “if and only if a search surface exists” hybrid thesis. ([study](https://arxiv.org/html/2608.00902))

22. **Inside the Scaffold counts — VERIFIED WITH SCOPE LIMIT:** 13 agents, seven compaction strategies, five control primitives, and 11/13 composing multiple primitives are supported, but the paper explicitly reports divergence in compaction and state management, so it cannot validate the entire detailed §3 skeleton at high confidence. ([paper](https://arxiv.org/abs/2604.03515))

23. **Harness evidence — COUNTS VERIFIED, CAUSALITY OVERSTATED:** Harness-Bench has 106 tasks and 5,194 trajectories, but explicitly evaluates whole configurations rather than decomposing individual mechanisms; Terminal-Bench’s cited Gemini example is 32.6 versus 16.4, or 16.2 points, so “10–20” is a plausible selected configuration swing—not an expected E1–E5 effect size. ([Harness-Bench](https://arxiv.org/html/2605.27922), [leaderboard](https://www.tbench.ai/leaderboard/terminal-bench/2.0))

24. **Model2Vec — IMPLEMENTATION VERIFIED, FITNESS UNVERIFIED:** the official Rust implementation has WASM/in-memory loading, but `potion-base-8M` denotes roughly eight million parameters while its f32 safetensors file is about 30 MB; size/quantization claims remain vendor evidence, with no coding-transcript retrieval benchmark. ([Rust implementation](https://github.com/MinishLab/model2vec-rs), [model files](https://huggingface.co/minishlab/potion-base-8M/tree/main))

25. **Crate versions — VERIFIED AS PUBLISHED PINS:** `reqwless` 0.14, `embedded-io` 0.7, `embedded-tls` 0.19, Embassy executor 0.10, and Workers Rust 0.8.5 exist as stated, although download counts are volatile popularity metadata rather than architectural evidence. ([reqwless](https://docs.rs/reqwless/latest/reqwless/), [embedded-io](https://docs.rs/embedded-io/latest/embedded_io/), [Embassy](https://docs.rs/crate/embassy-executor/0.10.0), [Workers Rust](https://docs.rs/crate/worker/latest))

26. **Off-`std` certificate verification — WRONG/OUTDATED:** [§7](/Users/feral/mydev/rust-kittens/docs/kittens-code/RESEARCH.md:527) is correct that the `webpki` verifier is `std`-only, but `embedded-tls` 0.19 also exposes a `rustpki` certificate-chain verifier whose feature does not require `std`; endpoint compatibility, roots, hostname/time handling, and algorithms still require a hardware spike. ([features](https://docs.rs/crate/embedded-tls/latest/source/Cargo.toml.orig), [`rustpki` module](https://docs.rs/embedded-tls/latest/embedded_tls/pki/index.html))

27. **“~16 KB/connection” — UNDERSTATED:** the crate’s example allocates separate 16,384-byte read and write record buffers, so a full-size configuration starts near 32 KB before connection state, HTTP buffers, JSON, transcript state, and executor stacks. ([embedded-tls example](https://docs.rs/embedded-tls/latest/embedded_tls/index.html))

28. **ACP versioning — SEMANTICALLY CONFUSED:** the input’s schema/package `v1.20.0` is not the ACP wire version; the official repository says artifact versions are independent and the current stable negotiated protocol version is `1`, so SPEC language should distinguish these. ([ACP versioning](https://github.com/agentclientprotocol/agent-client-protocol))

29. **“Zero prior art” for cross-agent transcript reads — FALSE AS OF THE REVIEW DATE:** OpenClaw exposes scoped `sessions_history`, exact transcript search, and configurable cross-agent QMD search over another agent’s session transcripts; this is not the identical filesystem-mount API, but it is functional prior art for agent-controlled, access-scoped peer-history retrieval, so novelty must be narrowed to the uniform read-only mount/query abstraction. ([cross-agent search](https://docs.openclaw.ai/multi-agent), [session tools](https://docs.openclaw.ai/session-tool), [session search](https://docs.openclaw.ai/concepts/session-search))

30. **Append/replay/store conjunction — INTERNALLY INCONSISTENT:** [row 3](/Users/feral/mydev/rust-kittens/docs/kittens-code/RESEARCH.md:649) says Codex, Grok, and prime-agent “all ship it,” but the pinned Codex and Grok evidence supports persistence/replay, not a shared RLM-queryable store.

31. **Kittens fit — STRUCTURALLY CONSISTENT BUT UNPROVEN:** the mapping in [input 10](/Users/feral/mydev/rust-kittens/docs/kittens-code/research-inputs/10-kernel-fit.md:30) respects K0’s explicit polling and borrow boundaries, but K0 validates reactor topology rather than cancellation of owned model/tool operations or harness outcome quality.

## Blind spots

32. **Durable-log engineering is missing:** research framing should cover framed/checksummed records, sequence IDs, atomic tool-call/result transactions, crash-truncated tails, schema upcasters, snapshots, correction records, index consistency, replay of external effects, and flash-wear behavior; JSONL is a codec, not a durability protocol.

33. **Retention and security law is missing:** “never destroyed” cannot govern secrets, poisoned records, user deletion, retention limits, revocation, or finite flash; specify tombstone/redaction overlays or crypto-shredding, and treat retrieved peer/tool text as tainted data using lessons from [AgentDojo](https://arxiv.org/abs/2406.13352) and capability-tracked control/data separation such as [CaMeL](https://arxiv.org/abs/2503.18813).

34. **Retrieval design is underfactored:** [LongMemEval](https://arxiv.org/abs/2410.10813) separates indexing, retrieval, and reading and highlights chunk granularity, key expansion, temporal queries, updates, and abstention; L1/L2/L3 alone hides these spec-relevant dimensions.

35. **The embedding port is underspecified:** every index needs model fingerprint, dimensions, metric/normalization, quantization, chunk/key-generation version, source-record hash, indexed high-watermark, rebuild policy, and stale-index behavior.

36. **Compaction scheduling research is incomplete:** [SelfCompact](https://arxiv.org/abs/2606.23525) reports model-invoked compaction outperforming fixed thresholds at lower cost, while the August KV-cache paper favors delayed future-query signals; both could change a prefire-only policy.

37. **Multi-agent task topology is missing:** a current 260-configuration study reports outcomes from +80.8% on decomposable work to −70% on sequential planning and greater error propagation without centralized verification, so E4 must stratify parallel, sequential, tool-heavy, and adversarial tasks. ([scaling-agent-systems study](https://arxiv.org/abs/2512.08296))

38. **MCU feasibility needs full-system evidence:** measure a real authenticated endpoint, DNS, verified TLS, SSE/stream parsing, transcript persistence, flash endurance, cancellation, peak RAM, and power on the target board before normative MCU claims freeze.

39. **The query-surface comparison is too narrow:** test structured function calls and a typed AST alongside shell text and Lua, including quoting, escaping, regex dialects, parser recovery, small-model compliance, and action-space stability.

## The three bets

40. **Bet A—RLM context law with shell verbs:** evidence for externalized context and programmable subcalls is honestly represented, but evidence for the shell surface is not; the strongest argument against it is that RLM’s causal advantage may be arbitrary program composition rather than retrieval verbs, and the chief failure is silent retrieval non-initiation after compaction followed by expensive `ask-each` fan-out.

41. **Bet B—L1/L2/L3 with pluggable embeddings:** the synthesis is commendably explicit that L2 is optional and unmeasured, but overstates MEMTIER and BrowseComp as direct support; the strongest argument against it is that lexical L3 may dominate for one local transcript while embeddings add footprint and stale-index complexity, and the chief failure is a plausible but entity-confused or stale hint anchoring the agent before exact verification.

42. **Bet C—peer transcript read-mounts:** outcome uncertainty and the isolation-first null are represented honestly, but originality and “nearly free” are not; the strongest argument against it is that typed handoffs are beneficial information bottlenecks, and the chief failure is transitive prompt injection or correlated hallucination causing a reader with greater capabilities to act on unsafe peer text.

## Verdict and freeze conditions

43. **VERDICT — YES-WITH-CONDITIONS:** the research is sufficient to freeze KC0 only as a reversible evidence-producing slice, not as validation of shell sufficiency, embedding value, swarm benefit, or complete MCU feasibility.

44. **Condition 1:** correct findings 15–30 and split compound confidence rows before treating [RESEARCH.md](/Users/feral/mydev/rust-kittens/docs/kittens-code/RESEARCH.md:1) as the normative evidence ledger.

45. **Condition 2:** freeze only the versioned event envelope, replay semantics, exact L3 query baseline, hard budgets, and evaluation hooks; keep L2, Lua, and swarm behind removable features.

46. **Condition 3:** preregister E1–E4 falsifiers and budgets—quality, Recall@k, abstention, verification rate, tokens, dollars, p50/p95 latency, subcalls, and at least two model families—before implementation results are visible.

47. **Condition 4:** define retention, redaction, correction, access revocation, stable read watermarks, provenance, and taint/capability handling before peer mounts share the common store.

48. **Condition 5:** expand E4 to cost-matched isolation, structured handoff, centralized coordinator, raw mount, and filtered/snapshot mount arms across different task topologies.

49. **Condition 6:** complete the real ESP32-S3/no-`std` endpoint and memory spike before freezing transport/TLS/RAM claims; otherwise KC0 should promise only a compiling sans-I/O core and std runtime.

No files were modified.