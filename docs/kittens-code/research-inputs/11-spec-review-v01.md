# Adversarial review of SPEC.md v0.1 (2026-08-08, spec refinement pass 1)

Method: 27 normative-rule citations traced to RESEARCH.md v2 and inputs
01/05/06/07 (24 hold); K0 grammar checked against agent-guide.md and root SPEC
§37.3. Verdict: the three bets and slice structure survive; blockers 1–3 must
be fixed before freeze.

## Blockers

1. **Q3/G3 contradict L1/K1 — root-protection law false as written.** L1
   appends tool results to the message list; K1 read/grep/exec return bulk
   output to the root — that is the harness loop. Q3/G3's evidence (I-05,
   R§4.2) belongs to an architecture where sub-LMs hold the tools; KC0
   inverts that (Q4: `ask` has no tools). Fix: rescope Q3/G3 to
   RLM-originated data (verb output + ask digests cap-typed) and add the rule
   R§4.2 actually recommends — every tool result surfaced to the root is
   truncated to a typed per-result budget — with its own gate.
2. **L6 interrupt mapping incoherent against K0 grammar.** `#[shutdown]` is
   terminal (handler returns Exit) but L2 requires interrupt to abort the turn
   without ending the session. Reactor lifetime unspecified; decides F-a.
   Fix: choose (a) one reactor per turn (interrupt = shutdown, session loop
   wraps reactors) or (b) one reactor per session (interrupt = ordinary
   leading non-terminal arm; separate `shutdown` terminal arm; state which K0
   guarantee each inherits).
3. **G3 "enforced-by-type" has no described mechanism.** Achievable via sealed
   `Capped` newtype (private truncating constructor; window-insertion API
   accepts only `Capped`) — spec never says so. Fix: specify pattern in
   P6/Q2; gate = trybuild compile-fail on bypass + adversarial fixture.
   (S1 is honest — API-surface claim — but no gate covers it; add one.)

## Kernel fit (F-a plausibility)

4. **Major.** Clean K0 mappings: interrupt/shutdown leading prefix (modulo 2);
   model-delta via owned HTTP task → admitted mpsc with drain + yields_to;
   interjection mpsc; prefire in after_event + optional one-shot; deadline
   timers. **No K0 story:** (a) unbounded concurrent tool calls — no dynamic
   source sets/JoinSet/channel-task helpers (§37.3 excludes); only story is
   one owned task funneling completions into a single mpsc, rejoin-by-index
   as application logic; (b) per-tool cancellation visibility — owned tasks
   carry no Kittens guarantee; L2's "one cancellation lineage" is application
   discipline, not kernel law; G4 tests interrupt latency only, add G4b
   (in-flight tool aborted → terminal abort record in log); (c) subagent
   fan-out needs dynamic sources K0 lacks (post-KC0 kernel ask);
   (d) no HTTP/SSE adapter in K0's sealed set and admission is sealed.
   Fix: make owned-task+funnel NORMATIVE in L6; demote L2's kernel claim;
   record (c)/(d) as K1-era kernel asks so F-a isn't tripped spuriously.

## Internal consistency

5. **Major.** Q7 (recall tool) exists "so E1 can compare always-on vs
   tool-mediated" but E1 arms omit the tool-mediated arm. Add the arm or
   defer Q7.
6. **Major.** Built surface with no gate: K1/K2 tools (incl. exec +
   apply_patch parser), C7 escaping, L3 stationarity guard, fork, M2
   retry/circuit breaker. (Converse class — gates requiring unbuilt
   surface — empty.) Fix: tools-conformance gate + C7 adversarial fixture,
   or cut (see 13).
7. **Major.** C8/D-a adopts a recorded reject: bytes/4 (I-07 reject; R§9 Q7).
   KC0 compaction triggers — hence G5/E1, the headline evidence — run on the
   rejected estimator; mis-triggered compaction contaminates E1 arms. Fix:
   promote D-a to KC0-blocking or record estimator error bounds in G5.
8. **Major.** Sans-io form dropped: R§7 recommends pure
   `handle_event(Event) → Effects`; §11 has core depending on port traits
   (DI). Different architectures, different no_std async implications; spec
   never chooses. Fix: state the call model in §6/§11.
9. Minor. §14.4 "C1–C8 complete" overstates (C8 placeholder per D-a).

## Traceability residue

10. Minor. Units drift: P6/Q2 "8192 bytes" vs all sources "8192 chars" —
    bytes is the better no_std choice; mark deliberate deviation.
11. Minor. Q2 "per-query verb-count cap" uncited — mark synthesis-introduced
    or drop.

## Completeness (ranked)

12. (a) Major: error taxonomy (retryable/fatal, codes, Op correlation) —
    D2 can't close without it. (b) Major: persisted record-schema versioning +
    session identity (G2 replay across versions undefined; no session-id/
    naming; multi-process append unaddressed). (c) Major: config schema +
    precedence (thresholds are "config data" with no schema). (d) Major:
    prompt content ownership (system prompt, reminder text — D9 covers only
    the summary prompt). (e) Medium: mock-LLM jail interface + E1 task
    battery undefined (G5's evidential value unmeasurable). (f) Medium:
    approval-policy defaults per tool.

## Slice honesty

13. Minor. Cut without weakening G1–G6: fork (§14.3), K3 trait+memory impl,
    mlua(feature) in §3 KC0 topology (E2 is post-KC0). Real-endpoint drive is
    ungated but is the slice's purpose — keep, add smoke gate. Port
    definitions for Embedder/Similar/ContextExchange — cheap, keep.
14. Minor. C2 microcompact lost its Observation(leak) provenance marker.

## Empty classes (genuinely checked)

- Gates requiring surface KC0 doesn't build: none.
- Decisions register vs body contradictions: none (D6↔M1, D7↔S4/Q5, D8↔L5,
  D-a↔C8 agree).
- Redaction: clean. Leak-code copying: none (14 is labeling, not usage).
- Root-§37 mirroring: honest; one gap — no analog of §37.4's admission
  ledger; consider importing the one-sentence-missing-oracle rule.
