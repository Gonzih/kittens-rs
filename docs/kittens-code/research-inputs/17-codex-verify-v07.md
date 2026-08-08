# External targeted verification of SPEC v0.7 — Codex (gpt-5.6-sol, ultra), 2026-08-08

Third and final Codex spec pass. Verdict: FREEZE-AFTER-FIXES with three
implementation-defining items (append/recovery unification, complete typed
IR + consistent cap/meter rule, ID->layer->gate matrix) — all closed in
SPEC v0.8, ending the external review cycle. Remaining before freeze:
D2/D4 exact shapes + operator review.

## Targeted verification — SPEC v0.7

1. **PARTIAL / BLOCKER — canonical append/failure.** §6 says “`Commit` is the ONLY append path” and defines fatal `PersistFailed` handling ([SPEC §6](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:225)). But imported S7 and §11 still declare `StoreAppend` effects ([S7](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:209), [§11](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:432)). Startup repair also directly “APPENDS” before replay creates the fresh core ([S3](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:193), [P1](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:125)). The sole path and recovery path remain contradictory.

2. **FIXED — whole-batch dispatch.** L-A2b requires capacity for the “ENTIRE Transition,” ordered exactly-once dispatch, and completion before the next `handle`; G11 tests unsplit staging ([L-A2b](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:254), [G11](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:520)).

3. **FIXED with wording nit — bounded continuations.** Q4 adds cursor/fold/join state and terminal discard rules; Q5 adds per-query page/byte/effect ceilings, simultaneous-query session bounds, aggregate retained-memory bounds, and compile-time maxima ([Q4](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:356), [Q5](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:368)). Say explicitly that aggregate memory is “per session.”

4. **NOT FIXED / BLOCKER — self-contained typed verb semantics.** Q2 now claims a “closed enum,” but `By` and `size_or_pattern` are undefined, output variants are not typed, and range/default/escaping semantics remain incomplete ([Q2](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:338)). Appendix A omits several Q5 charges and adds undefined “wall-clock/token meters” ([Appendix A](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:589)). Worse, P8 says verb-cap hits error while Q5 says value caps truncate ([P8](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:162), [Q5](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:376)).

5. **FIXED — T3/G1b topology.** T3 explicitly exempts `kittens-code-cli` as the composition root and applies G1b to non-root consumers ([T3](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:99)).

6. **PARTIAL / MAJOR — ledger/enforcement mapping.** The import ledger is now literal and correctly dispositions L-D2 and T7/Q7 ([§14 ledger](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:467)). The claimed matrix is only ID→gate, not ID→enforcement-layer→gate; T4, T5, and S4 are unmapped, T6 remains unexercised in KC0, and K1–K3 reference undefined G7e ([matrix](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:527)).

7. **R4 FIXED.** `TimerFired` now carries `epoch: TurnEpoch` ([§6](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:220)).

8. **R7 FIXED.** “`resume` is not an Op”; startup repair/replay and epoch/sequence seeding are specified ([P1](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:122)).

9. **R9 PARTIAL.** A persisted repair terminal is intended, but its direct scanner append conflicts with finding 1; “replay … derives a synthetic” also remains stale beside “nothing synthetic exists only in memory” ([S3](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:184)).

10. **R11 PARTIAL, wording-level.** Atomicity capability/fallback, cross-mount refusal, and symlink treatment are defined. “Replace-by-default” still leaves the requested no-replace mode implicit; say “always replaces” or define the option ([K2](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:399)).

11. **R13 PARTIAL; R14 ledger FIXED.** R13 remains blocked by finding 4. R14’s literal enumeration and dispositions themselves are repaired; N3’s enforcement mapping is not.

12. **R15 TEXT FIXED.** P2 now preserves unknown Events as raw bytes, rejects unknown Ops, and requires epoch bumps for unknown state-bearing records; G2c adds compatibility coverage ([P2](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:129), [G2c](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:497)).

13. **R16 PARTIAL.** RESEARCH now has five prior-art families ([RESEARCH §6](/Users/feral/mydev/rust-kittens/docs/kittens-code/RESEARCH.md:481)); E2/E3/E4 preregistration and W4 revocation/watermarks are present ([§14](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:483), [W4](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:426)). Q6 is much improved, but “Rust `regex` crate syntax” is not exact without a crate/version/features and inline-flag policy pin ([Q6](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:379)).

14. **R17 FIXED.** RESEARCH’s header, Q1, and lineage consistently mark the TUI seam resolved/KC1-only ([header](/Users/feral/mydev/rust-kittens/docs/kittens-code/RESEARCH.md:8), [Q1](/Users/feral/mydev/rust-kittens/docs/kittens-code/RESEARCH.md:731), [lineage](/Users/feral/mydev/rust-kittens/docs/kittens-code/RESEARCH.md:809)).

15. **N1 FIXED** by Q4/Q5; **N2 FIXED** by non-authoritative previews, record-id reconciliation, and bounded flush policy ([L-A3](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:259)). Nit: define “bytes-or-millis” as whichever threshold occurs first.

16. **N3 NOT FIXED** per finding 6. **N4 PARTIAL:** its requested text landed, but repair precedes S6’s higher-epoch replay refusal, permitting an old scanner to mutate an incompatible log before refusal ([S6](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:203)). Compatibility validation must precede repair.

17. **N5 NOT FIXED** per finding 4. **N6 FIXED** per finding 5. **N7 FIXED for the reported missing-field/golden defect** via S6, Q6, and G12; the exact regex pin remains a wording/data nit ([G12](/Users/feral/mydev/rust-kittens/docs/kittens-code/SPEC.md:524)).

18. **Sanity sweep:** The new contradictions are confined to append/recovery ordering, repair-before-epoch-refusal, P8-versus-Q5 cap behavior, Appendix-versus-Q5 meters, and undefined G7e. No additional contradiction was found in the changed §4/§5/§6/§8/§14/Appendix-A material.

19. **VERDICT — FREEZE-AFTER-FIXES.** Required before the D2/D4 closure and operator review: unify normal and recovery append paths with failure/order semantics; finish the closed typed input/output IR and exact meter table; and provide a complete ID→named-enforcement-layer→existing-gate matrix. These are implementation-defining, not wording-only.

Read-only; no files modified.