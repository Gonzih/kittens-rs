# External verification pass on SPEC v0.6 — Codex (gpt-5.6-sol, ultra), 2026-08-08

Regression + new-defect sweep after v0.6 folded input 15. Verdict:
FREEZE-AFTER-FIXES (converging: 8/17 FIXED, 9 PARTIAL with precise fixes;
8 new findings). Required-before-freeze list at the bottom is the v0.7
worklist. Note: input 15 actually contained 8 blockers + 9 majors (its
provenance header previously said 7 — corrected there).

# Verdict: FREEZE-AFTER-FIXES

v0.6 substantially improves the architecture, but is not freeze-ready even subject to the acknowledged D2/D4 closure. Several implementation-defining laws remain contradictory or absent.

## Regression: prior 17 findings

1. **[BLOCKER][FIXED] Tools topology.** §3 says “TOOLS ARE CORE MODULES” and “the separate tools crate is dissolved”; §9 agrees.

2. **[MAJOR][FIXED] Driver topology.** §3 names `kittens-code-driver-tokio`, gives the CLI the requested package/binary names, and says “never a generic `driver-common` until a second driver proves shared code.”

3. **[MAJOR][FIXED] Protocol boundary.** §3 makes protocol “wire types only”; P5 splits SessionConfig from bootstrap configuration; P6/C10 place caps and WindowLayout in core; P9 uses arrays/integers.

4. **[BLOCKER][PARTIAL] Directional call model.** §2 and §6 correctly introduce Op/Event and CoreInput/CoreAction/Transition. But §2 says “every Effect and completion carries the epoch,” while §6 defines `TimerFired { id: EffectId }`. Add `epoch`, or make timer expiry an `EffectFinished`.

5. **[BLOCKER][PARTIAL] Effect-only I/O.** S7 correctly removes synchronous Store and C8 removes TokenCount. However S7/§11 call `StoreAppend` an Effect while §6 separately defines `Commit(records) // append to log`. Choose one append path and define its failure completion.

6. **[BLOCKER][PARTIAL] Re-entrancy/backpressure.** L-A1 fixes recursion; L-A2 adds bounded queues and cancel-aware producer waits. It omits whole-batch capacity handling. Require atomic reservation/staging of an entire Transition, ordered exactly-once dispatch, and completion of dispatch before the next `handle`.

7. **[BLOCKER][PARTIAL] Turn/cancellation ownership.** L-T1/L-T2 supply ownership, terminal-ledger, race, and shutdown laws; G4b covers the requested paths. P1 still imports `resume`, while S2 only says “resume = replay” and L-T2 declares interrupted epochs aborted. Define legal states, idempotence, epoch behavior, and records for `resume`, or remove it.

8. **[BLOCKER][PARTIAL] RLM continuation.** Q4/Q5 add suspension, incremental `ask-each`, and fan-out caps. `QueryCont` lacks page cursor, instruction accumulator/fold state, and join state; Q5 lacks scanned-page/byte/effect limits and an aggregate active-continuation bound. Add these bounds and terminal/discard rules.

9. **[BLOCKER][PARTIAL] Streaming durability.** S2/S3 fix the original whole-stream atomicity defect: streams are “individually framed” and “never buffered whole.” But synthetic `aborted_by_crash` is merely “derived,” conflicting with P2/L-A3’s durable-record-only publication. Persist a repair terminal before publishing it, or declare it internal-only and revise P3.

10. **[MAJOR][FIXED] Runtime caps.** P6 provides branded types, applied limits, truncation metadata, compile-time ceilings, private constructors, and no unchecked Deserialize; G3/G3b cover bypass and malicious decoding.

11. **[MAJOR][PARTIAL] Vfs/Exec contracts.** K2 defines almost every requested field, but “declared rename semantics” declares no semantics. Specify replace/no-replace, atomicity, cross-mount behavior, and symlink treatment.

12. **[MAJOR][FIXED] Web/WASI adapters.** T6 and L-D1/KX4 require disabled Tokio defaults and wake-aware Web/WASI adapters. Input 10 is committed with an explicit correction header and reason.

13. **[MAJOR][PARTIAL] Typed verb IR.** Q2’s `Instr { op, args: [Value] }` remains an untyped argument bag, and Appendix A replaces the semantics table with “Verb semantics as in v0.5 §8.1.” Inline closed per-verb variants, typed inputs/outputs, escaping/ranges, and meter charging.

14. **[BLOCKER][PARTIAL] Import ledger.** §14 now includes the formerly omitted P/S/C laws and D2/D4 are correctly freeze-blocking. But it claims “no more ranges” while using `T1–T7; P1–P9; …`, silently omits L-D2, and both imports T7 and excludes “Q7/T7 features on.” Enumerate every ID literally and disposition L-D2.

15. **[MAJOR][PARTIAL] Gates.** G1b, G2, G7b–d, and L-D3 add the requested structural, crash, stream, barrier, parser, and seeding evidence. P2’s unknown-kind rule has no lossless representation or compatibility fixture; add opaque-tag/payload preservation and decode/replay/re-encode tests.

16. **[MAJOR][PARTIAL] Prior six conditions.** C1: numeric/TLS/“nearly free” corrections are sound, but RESEARCH §6 still says “prior art falls into four families, none … transcript read-mounting” immediately before admitting OpenClaw. C2: removable features are fixed, but Q6’s “full regex” is not an exact baseline. C3: only E1 is preregistered; required metrics, two model families, E3, and E4 are absent. C4: D16/W4 omits access revocation and stable read watermarks. C5’s coordinator arm and C6’s MCU deferral are fixed.

17. **[MAJOR][PARTIAL] TUI freeze status.** SPEC F3/D-b and tracked input 14 consistently make it KC1-only. RESEARCH still says “one coordination gap open” and marks the interface unknown/freeze-blocking. Update its header, open question 1, and lineage.

## Target-matrix impossibility check

- **[PARTIAL] Synchronous durable Store:** reads/search are effects, but append has the conflicting `StoreAppend`/`Commit` spellings.
- **[FIXED] Callback-driven Web sources:** L-D1 explicitly rejects local adapters and requires KX4.
- **[FIXED] Host-independent Web runtime:** `wasm32-unknown-unknown` is only a core link gate; the Web driver names concrete host facilities.
- **[FIXED] UUIDv7 without wall time:** P9 uses entropy-generated `[u8;16]`.
- **[FIXED] Buffered atomic streams:** S3 uses per-record framing and atomicity.

## New-defect sweep

1. **[BLOCKER] Priority (a): pending state remains unbounded.** Per-query selected bytes do not bound pages scanned, empty search pages, total page effects, simultaneous suspended queries, or aggregate retained continuation memory. Add hard per-query and per-session ceilings under compile-time maxima.

2. **[MAJOR] Priority (b): durable streaming latency is stated but not accepted or bounded.** L-A3 withholds every delta until `Persisted`; S5 leaves sync policy driver-declared. Either record this UX tradeoff and bound flush latency/bytes, or define explicitly non-authoritative preview deltas with reconciliation.

3. **[MAJOR] Priority (c): ledger↔gate closure fails.** G4’s service-window law has no imported Kittens law; T6’s G1b check is vacuous without a non-Tokio KC0 driver. Conversely P2/P7, L-A1–A3, S7/Q4, Q6, K2/K3, and F4 lack named evidence. Add an ID→enforcement-layer→gate matrix.

4. **[MAJOR] Priority (d): unknown kinds do not compose with replay as written.** Lossy catch-all decoding cannot preserve checksum-covered bytes, and unknown lifecycle records can cause false crash-abort derivation. Preserve client Events opaquely; reject unknown Ops; require epoch bumps for unknown state-bearing kinds. Clarify G2 as fresh-run byte equality versus replay state equivalence.

5. **[BLOCKER] Priority (e): Appendix A is neither self-contained nor consistent with Q5.** Its v0.5 reference reimports the rejected `partition --size`/verb-count fan-out rationale. Restore a current semantics table mapping all seven Q5 meters. Also resolve P6/G3 “truncate with metadata” versus P8 “verb cap hits bind `verb_error`”—recommended split: value caps truncate; aggregate/query meters error.

6. **[BLOCKER] T3 contradicts the KC0 topology.** §3 says the CLI links driver-tokio; F1 calls it a frontend; T3/G1b require frontends to link protocol only. Exempt the composition-root binary and apply T3 to presentation/client modules.

7. **[MAJOR] Q6 claims an unrecorded version.** Q6 says `l3_dialect_version` “records it,” but S6/D11 omit that field. Add it to the header, pin driver-tokio’s exact dialect, and add search replay goldens.

8. **[MINOR] Review count is wrong.** Input 15 contains eight blockers plus nine majors, not “7 blockers, 9 majors.”

Required before freeze: canonical append/failure protocol; whole-batch dispatch law; bounded paged continuations; self-contained typed verb semantics; repaired T3/G1b topology; complete ledger/enforcement mapping; then D2/D4 closure and operator review.

Read-only review; no files modified.