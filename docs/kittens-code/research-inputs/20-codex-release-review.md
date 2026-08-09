# Final release-readiness + spec-fidelity review — Codex (gpt-5.6-sol, high), 2026-08-09

Read-only review of branch kc0 for merge + crates.io publish. Verdict:
NOT-READY. 7 blockers, 6 majors. #1 (spec authorization) resolved by
operator freeze directive. #3 (appender torn-tail) independently CONFIRMED by
the author. #4/#6 are author-introduced regressions from the lifecycle/resume
commits. This is the publish-gate worklist: fix the confirmed correctness
blockers (#3, #4, #6) before any crates.io publish; the remaining
partial/major items (#7 gates, #8-#13) are pre-existing deferred KC0 scope.

Release-readiness summary:

- All four manifests have `description`, SPDX `license`, and `repository`; workspace inheritance resolves correctly.
- All are publish-enabled. Only the CLI has an auto-detected README; none has keywords/categories. These are release-quality issues, not Cargo hard blockers. [Cargo guidance](https://doc.crates.io/crates-io.html)
- `0.1.1` does not collide with `kittens` or `kittens-tui`: crates.io versions are scoped by package name. Live registry name availability still requires checking.
- Internal dependencies have both paths and exact `0.1.1` registry versions. Required publication order is protocol → core → driver → CLI, waiting for index visibility between steps. `reqwest`/rustls are acceptably optional behind `live`.
- The worktree is clean. I did not execute packaging, builds, or tests in the read-only sandbox. Dependent dry-runs cannot resolve successfully until their preceding crates exist in the registry.

1. **Blocker — `docs/kittens-code/SPEC.md:21-23,667-669`.**  
   **Problem (Fact):** The controlling contract explicitly says “Not yet frozen, no implementation authorized”; D2 and D4 remain freeze blockers. This prohibits publication and, under the repository’s spec-first law, also blocks merging the implementation as KC0.  
   **Fix:** Complete operator review, close D2/D4, record the final verification, and change the header to authorize implementation/publication. Until then, add `publish = false` to all four manifests to prevent accidental release.

2. **Minor — `crates/kittens-code-{protocol,core,driver-tokio}/Cargo.toml:1-8`; `crates/kittens-code-cli/Cargo.toml:1-8`.**  
   **Problem (Fact):** Protocol, core, and driver have no README; all four lack keywords/categories and explicit `readme` metadata. The CLI README is auto-detected.  
   **Fix:** Add a README per crate, enforcement-layer table, relevant categories/keywords, and explicit `readme` paths before publication.

3. **Blocker — `crates/kittens-code-driver-tokio/src/appender.rs:125-155`.**  
   **Problem (Fact):** Review-19 #2 is not fixed. Scanning discards a torn/checksum-bad final frame logically but never truncates the physical file to the valid-prefix byte boundary. Repairs or later records are appended after/onto the bad bytes; the next reopen can fail with a mid-log fault. Existing tests stop before that decisive second reopen.  
   **Fix:** Track the valid byte boundary, truncate/synchronize to it before repairs or normal appends, and test torn and checksum-bad tails through append plus a second reopen.

4. **Blocker — `crates/kittens-code-core/src/engine.rs:600-615,630-663,779-797`.**  
   **Problem (Fact):** Lifecycle/ledger work is only partial. Denied calls emit `ToolTerminal` without `ToolStarted`. More seriously, any unknown current-epoch completion commits `StreamTerminal` before ownership and terminal-kind validation; this can persist a terminal with no start and make the next replay fail.  
   **Fix:** Validate effect ownership, epoch, pending state, and expected terminal kind before closing a stream. Route unknown/wrong-kind completions solely through traced-drop records, and make denied wire lifecycles satisfy `Started → Terminal`.

5. **Blocker — `crates/kittens-code-driver-tokio/src/runner.rs:47-58,130-156,169-215`.**  
   **Problem (Fact):** Review-19 #8 and the imported driver laws remain violated. The funnel is unbounded; configured queue/effect limits are unused; `Persisted` is fed to `handle` before the original action batch finishes dispatching; the two-pass algorithm reorders actions; `CancelEffect` is ignored; and no `kittens::reactor!` loop exists. Interrupt/shutdown therefore wait for uncancelled tasks rather than observing cancellation within the required window.  
   **Fix:** Implement the specified reactor topology, bounded admitted sources, whole-batch capacity reservation and ordered dispatch, deferred durability acknowledgements, active-effect admission, and owned-handle cancellation.

6. **Blocker — `crates/kittens-code-core/src/engine.rs:389-448`; `crates/kittens-code-core/tests/resume.rs:126-181`.**  
   **Problem (Fact):** Review-19 #5 is only partially fixed. `Runner::open` calls `Engine::resume`, but replay restores only configuration and counter maxima. It does not reconstruct conversation tail, last query, summary, token/compaction state, or other derived session state. A reopened session consequently sends the next model call without prior conversation context; the tests assert counters/configuration, not G2 state equivalence.  
   **Fix:** Replay accepted ops, authoritative events, outcomes, compaction state, and terminal state into a fresh engine, then compare the next transition/window against uninterrupted execution.

7. **Blocker — `docs/kittens-code/SPEC.md:568-580,584-616`; `.github/workflows/ci.yml:19-25`.**  
   **Problem (Fact):** Multiple mandatory KC0 deliverables/gates are absent: complete context-engine integration, core-owned Vfs/Exec contracts and `SessionCapabilities`, E1 rig/report, G1b structure audit, G2/G2c determinism and unknown-kind fixtures, G4/G4b matrix, G5, G6 fuzz, G7 apply-patch/ordering suites, G8, G10 endpoint evidence, G11, and G12. The spec explicitly says unrun E1 fails. CI principally supplies ordinary tests plus the G1 link builds.  
   **Fix:** Implement the imported KC0 scope and add every named oracle/evidence artifact before merge.

8. **Major — `crates/kittens-code-core/src/rlm/exec.rs:146-299,826-850`; `crates/kittens-code-core/src/engine.rs:886-1088`.**  
   **Problem (Fact):** Recall now genuinely drives `Executor` and child page/sub-model effects, but Q5 is not correct. Aggregate budgets lack compile-time ceilings/validated admission; verb record/chunk outputs never apply `verb_output_bytes`; continuation memory is per executor rather than aggregate per session; meter advances do not emit `BudgetUpdate`; and ask digests remain plain strings rather than branded capped values.  
   **Fix:** Validate/clamp all runtime meters, cap every output with metadata, maintain session-level suspended-memory accounting, and publish typed meter updates.

9. **Major — `crates/kittens-code-core/src/rlm/exec.rs:624-651,839-850,968-1052`.**  
   **Problem (Fact):** Review-19 #12/#13 remain. `ask` over `Whole` or `Range` supplies empty context. Grep/count/regex partition still use literal substring semantics, `--kind` is ignored, and turn partitioning counts records rather than user turns. This contradicts Q2/Q6 and the recorded dialect `1.0.0`.  
   **Fix:** Page-materialize raw ask selections and implement the pinned regex/kind/turn semantics with byte-stable G12 goldens.

10. **Major — `crates/kittens-code-protocol/src/event.rs:46-50`.**  
    **Problem (Fact):** Review-19 #18 remains. A derived closed serde enum cannot decode or preserve unknown event variants; `#[non_exhaustive]` affects Rust matching only.  
    **Fix:** Introduce the specified raw-preserving event envelope/custom codec and G2c decode→replay→re-encode fixtures.

11. **Major — `crates/kittens-code-core/src/window.rs:68-123`.**  
    **Problem (Fact):** Review-19 #20 remains. The constructor only rejects an orphan result; it accepts unmatched calls, duplicate call IDs, and duplicate terminals. `TailItem::ToolResult` also publicly accepts an uncapped `String`, retaining the review-19 #14 bypass.  
    **Fix:** Enforce a one-to-one ordered lifecycle and require branded capped result/digest types at insertion boundaries.

12. **Major — `crates/kittens-code-driver-tokio/src/appender.rs:144-160`; `crates/kittens-code-core/src/engine.rs:549,801-802,1241-1299`.**  
    **Problem (Fact):** One-writer locking is still absent. Existing logs can be opened by multiple appenders. Several monotonic increments remain unchecked; a valid near-exhaustion log can panic or wrap during open, epoch advance, request allocation, effect allocation, or commit.  
    **Fix:** Acquire an exclusive writer lock and replace all increments with checked allocation returning typed exhaustion errors.

13. **Major — `crates/kittens-code-driver-tokio/src/tools.rs:22-60,135-160`.**  
    **Problem (Fact):** Basic symlink rejection fixes review-19 #7, but containment remains check-then-use and is vulnerable to component swaps. Writes/edits are direct, non-atomic, and lack expected-generation comparison. `exec`, `apply_patch`, core Vfs contracts, and capability advertising are absent.  
    **Fix:** Use handle-relative/no-follow operations, atomic revision-aware replacement, and implement the complete K1/K2/K3 contracts and adversarial G7e suite.

14. **Major — `docs/kittens-code/SPEC.md:667-669`; `crates/kittens-code-protocol/src/op.rs:18`; `event.rs:50`; `crates/kittens-code-core/src/engine.rs:130,159`; `window.rs:70`.**  
    **Problem (Fact):** Publishing would freeze exactly the public shapes the spec declares open: `Op`, `Event`, `CoreInput`, `CoreAction`, and `WindowLayout`. Their public payload types and constructors amplify the compatibility surface.  
    **Fix:** Close D2/D4 first, then perform a public-API/wire compatibility review and choose the initial published version deliberately.

15. **Minor — `crates/kittens-code-core/src/lib.rs:12-17`; `crates/kittens-code-driver-tokio/src/lib.rs:9-15`; `.github/workflows/ci.yml:22-23`.**  
    **Problem (Fact):** Crate-level docs exist, but some are stale or contradictory: core says implemented modules land later, while driver claims ownership of a reactor loop that does not exist. No crate has a canonical runnable example; the default CLI requires a scenario file but ships no example. CI rustdocs only `kittens`, not the new crates.  
    **Fix:** Correct the crate docs, add examples/sample scenario and external-user quick starts, and rustdoc all four crates with warnings denied.

Review-19 disposition: fixes were verified for recursive `PersistFailed` handling (#1), fan-out bounds (#9), token overflow (#15), `%0` panic (#17), the optional live client (#19), and the no_std fixture/CI shape (#21). Basic symlink handling and ask-each scheduling improved. Findings #2, #12, #18, #20 remain; lifecycle, cancellation, resume, RLM integration, Q5, appender hardening, and regex semantics are only partial. Static inspection found no `std` leak in protocol/core, but the required build gates were not executed here.

**VERDICT: NOT-READY — top 3 blockers: the spec explicitly forbids implementation/publication and D2/D4 are open; damaged-tail recovery corrupts subsequent logs; core/driver lifecycle, cancellation, replay, and mandatory KC0 gates remain incomplete.**