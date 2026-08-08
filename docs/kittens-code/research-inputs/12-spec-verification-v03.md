# Verification review of SPEC.md v0.3 (2026-08-08, spec refinement pass 3 input)

## A) Regression check vs input 11: ALL PASS

B1 (Q3 rescope + withdrawal recorded), B2 (one-reactor-per-session, G4/G4b
split), B3 (sealed Capped mechanism + trybuild), M4 (funnel normative, KX
ledger, F-a exclusion), M5 (RLM-as-tool E1 arm), M6 (G7–G10, fork/K3 cut),
M7 (calibrated estimator + G5 bounds), M8 (§6 call model; data-ports+effects
hybrid recorded), m9–m14 and 12a–f all verified addressed; T5 closes the
admission-ledger gap.

## B) Pass-2 additions: 2 major + 10 minor

1. MAJOR — C9 says prompt-pack version recorded in S6 header; S6 lacks the
   field. Fix: add `prompt_pack_version` (+ G2 bump fixture).
2. MAJOR — §14.6 "eight tasks, each existing in every E1 arm" unsatisfiable:
   tasks 2 and 5 require the RLM engine absent from the compaction-only arm.
   Fix: scope tasks per arm / define degraded variants.
3. MINOR — S6 replay refusal has no P8 code. Fix: add `schema_incompatible`.
4. MINOR — verb-level budget hit maps ambiguously to `verb_error` vs
   `budget_exhausted`; causes unenumerated. Fix: enumerate; in-script hits
   bind `verb_error`; `budget_exhausted` reserved for query/turn level.
5. MINOR — EBNF: `ident`/`number`/`character` undefined. (Semantics-table
   examples all parse under the rules as intended — checked.)
6. MINOR — `slice` table row (range only) contradicts the every-verb-reads-a-
   selection preamble. Fix: allow `slice %N` or exempt.
7. MINOR — inline verb errors vs ErrorEvent double-reporting unstated. Fix:
   inline binding + query trace entry, no top-level ErrorEvent.
8. MINOR — §13.1 only overrides reminder templates; C9 promises per-template
   override incl. system + summary prompts. Fix: prompt-pack override table.
9. MINOR — SandboxPolicy has no config key (approval defaults only). Fix:
   add key or state Op-supplied only.
10. MINOR — D2 register staleness ("closes with D10" but D10 closed);
    §8.1 "draft-normative" vs register "closed as KC0 draft" wording.
11. MINOR — Q1 "versioned and STABLE" grammar has no version anchor field.
    Fix: record grammar version alongside finding 1's S6 fix.
12. MINOR — `ask-each` width bound implicit; per-element × width can exceed
    per-verb cap. Fix: total output per-verb-capped; width bounded by
    partition size + verb-count cap.

Checked and clean: --peer candidate status consistent; C9 const data vs T2
fine; every "config data" mention has a §13.1 key; battery↔gate mapping
complete (1→G5/G6, 2→G3/Q3, 3→G4/G4b, 4→G7, 5→G3, 6→E1, 7→G7/M2, 8→G8);
lineage/register/header agree.

Bottom line: no architectural threat; disposition all 12 before freeze.
