# Adversarial review of RESEARCH.md v1 (2026-08-08, refinement pass 1)

Reviewer read all eight inputs, the synthesis, root RESEARCH.md §1–2, agent-guide.
Verdict: the three original bets survive; what breaks is §3's "everyone agrees"
framing (three of eight "convergent" items single-source or contradicted) and the
claim that all contradictions are recorded (at least three dropped).

## Blockers

1. §3 item 6 — "subagents isolation-first" labeled Fact is contradicted by input 07
   (Grok children SHARE parent hunk tracker/fs/terminal/env; forked context
   summarized) and input 02 (fork inherits full parent conversation). Isolation is
   Claude-default-only. Fix: depth-cap + single-summary convergent; isolation = a
   per-harness policy axis; record contradiction inline.
2. §3 item 8 — stationarity guard sourced to Grok alone, sits in a multi-input Fact
   block requiring 3+ harnesses; Codex/Claude show no such guard. §8.2 adopts it as
   "turn law." Fix: relabel Observation (single lineage); keep as Recommendation.
3. §4.4 + confidence row 4 — input 05's recorded contradictory pair (SWE-agent ACI
   +10.7pp for IDE-style primitives vs mini-swe-agent >74% bash-only) dropped;
   "Fact-grounded as sufficient" overstated next to an admitted Gap. Fix: record
   pair + input 05's resolution; demote to Observation.

## Major

4. §3 items 4–5, §5.1 — Claude microcompact + cache-boundary details are
   Observation(leak, unverified) in input 02, upgraded to Fact; "prefire" is
   Grok-only, presented as consensus. Fix: relabel; scope prefire to Grok.
5. §3 item 5 — "system prompt stays static" vs leak finding (~7 static + ~13
   dynamic sections); "both escape untrusted config" unsourced for Claude. Fix:
   restate as "static cached prefix + deliberate cache boundary"; scope escaping to
   Grok.
6. §4.2 — the two hard rules attributed to "prime-agent AND the RLM paper": input
   05 lists them as PI deltas FROM the MIT version; input 06 (ORG-4) attributes to
   the paper. Cross-input contradiction silently resolved. Fix: record it;
   Recommendation survives either way.
7. §4 omissions — dropped RLM cost bounds from input 05: PI thesis "real gains need
   RL training" (prompting-only results); wall-clock significantly worse
   everywhere; blocking subcalls/no prefix-cache reuse; counting/aggregation
   degradation. Fix: add costs/limits paragraph to §4.1.
8. §8 omission — input 04 shortlist #5 (shadow-snapshot checkpointing decoupled
   from user git, COW over Vfs) vanished. Fix: adopt or reject explicitly.
9. §6 — input 06 lesson 8 ("isolation-first swarms beat any shared-context scheme
   tried") never recorded as the standing counter-signal to the read-mount bet.
   Fix: add inline; isolation baseline = E4 control arm.
10. Confidence row 1 — "no counterexample found" false: Cline Plan/Act dual-mode +
    Roo orchestrator vs "no planner/executor split"; Gemini CLI soft in-process
    split vs "engine never linked." Fix: cite counter-signals; split ratings.
11. Confidence row 6 — "nobody ships depth>1" conflates subagent spawn depth
    (Claude ships 3) with RLM recursion depth; evidence = one reproduction study.
    Fix: reword to RLM depth; medium-high.
12. §9 missing spec blockers: (a) token-estimation seam (bytes/4 rejected,
    tokenizer-in-core rejected — what counts?); (b) store codec × L3 (regex over
    postcard-encoded MCU records unresolved, interacts D7); (c) whether v1 ships a
    subagent/task tool at all — no §8.1 owner; (d) who owns the compaction
    summarization prompt/model call (protocol data vs shim). Fix: add all four.

## Minor

13. §3 item 1 — "pi's ~300-line kernel" misattributes Thorsten Ball's Go example.
14. DSL→Lua "superseded" is Observation in input 06, cited as Fact twice.
15. "Fact: no coding agent targets no_std/WASM" — inputs hedge ("no mainstream",
    Gap/Observation); absence claims from surveys are Observation.
16. piccolo appears in no input — mark as synthesis-introduced.
17. Header count wobble: files = 8 (seven vector reports + operator capture).
18. §7 — "not bare RP2040" distorts input 03 ("workable only with spill-to-flash");
    dropped de-risk step: early ESP32-S3+PSRAM spike against a real endpoint.
    Q4 partially answerable now (TF-IDF fallback precedent sets floor class).
19. §8.1 — ACP "de facto standard" is the survey's judgment (Observation); 25+
    adopters is the Fact.
20. §3 item 6 — Codex depth-cap not evidenced in input 01; don't cite Codex for it.

## Empty classes (genuinely checked)

21. Redaction: clean (grep zero hits; ORG-1..7 codenames only). One weak vector,
    not a violation: synthesis names a specific `nexus-*` personal repo while input
    06 notes some nexus-* repos are stale mirrors of org work — permitted by the
    constraint, but a deanonymization edge worth an operator ping.
22. Leak-code copying: none found; usage is architecture intel only; residual issue
    is labeling (finding 4), not usage.
