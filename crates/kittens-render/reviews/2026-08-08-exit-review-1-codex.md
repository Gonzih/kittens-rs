No—the current K2R-0 host slice does not pass the exit gate.

## Findings

1. **Important — transfer correction 8 is claimed, but its oracle is not faithful.**  
   **File/section:** [K2R0A-LOG.md](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/K2R0A-LOG.md:22), [k2r0a_a_prime.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0a_a_prime.rs:51).  
   `ModelHw::complete()` itself removes the waker, so `late_completion_after_recovery_is_inert` would pass even if recovery performed no disarm. The N→N+1 test creates a fresh `Arc<ModelHw>`, so it cannot expose stale state in the real adapter’s single static ISR slot. There is also no pending-transfer drop → late IRQ trace. Waker replacement itself is covered correctly.  
   **Recommendation:** model one shared reusable done slot, implement model drop/disarm, and run recovery, ordinary-drop, late-IRQ, and N→N+1 traces against that same slot.

2. **Blocking — cancel-first followed by late hardware completion gets the wrong outcome.**  
   **File/section:** [k2r0a_a_prime.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0a_a_prime.rs:109), [transfer.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:61).  
   `cancel()` sets `cancelled`, but not `settled`. If hardware sets `done` before the progress wake is repolled, `poll_done` skips the `Cancelled` branch and reports `Completed`. That contradicts the documented cancellation-observation linearization point. The present test repolls immediately and misses this race.  
   **Recommendation:** classify and store the settlement atomically inside `cancel()` after its completion observation, then add `pending → cancel → late complete → repoll` as an adversarial oracle.

3. **Important — correction 5 is applied in code but not in the advertised HAL blueprint.**  
   **File/section:** [transfer.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:36), [VERDICT.md](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/probes/esp32s3-spi2/VERDICT.md:189), [probe README](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/probes/esp32s3-spi2/README.md:3).  
   The trait correctly uses `poll_done -> Poll<()>`, leaving `recover` as sole outcome authority. The “compile-ready adapter blueprint” still implements and returns `Poll<TransferOutcome>`, so it no longer implements the trait it documents.  
   **Recommendation:** retain the historical text as explicitly superseded and add a corrected compile-ready blueprint using `Poll<()>`.

4. **Blocking — sweep progress is still a caller assertion, disconnected from transfer outcome.**  
   **File/section:** [sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:143), [transfer.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:83), [k2r0_demand_sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0_demand_sweep.rs:22).  
   `mark_written` accepts only a `Region`. `Settled` carries neither epoch nor region, and no code converts `TransferOutcome::Completed` into progress. Consequently a caller can mark a cancelled, failed, or never-started stripe written; the test helper does exactly the latter for every stripe.  
   **Recommendation:** make progress consume an unforgeable `StripeWritten { epoch, region }` witness produced only by recovery of a matching `Settled { outcome: Completed, .. }`; cancellation/failure must instead produce the abort path.

5. **Blocking — `CompletedSweep` proves coverage only of an arbitrary caller-selected plan, not the fixed panel or immutable snapshot.**  
   **File/section:** [sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:47), [sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:123), SPEC §5.3/§6.  
   A board token can be attached to a caller-created 1×1 `SweepPlan`, marked once, and accepted by `finish_presented`. The advertised crate-owned `Sweep<S>` binding snapshot, target geometry, repaint mode, and epoch does not exist, so scene mutation between stripes is also unconstrained. Public plans additionally accept coordinate extents whose `y + offset` can overflow.  
   **Recommendation:** have the renderer mint one crate-owned `Sweep<S>` containing the fixed validated panel plan, immutable snapshot, repaint mode, and token; make raw `SweepProgress::new(plan, token)` unavailable externally.

6. **Blocking — active-sweep tokens are not bound to their `FrameDemand`, and finish validation disappears in release builds.**  
   **File/section:** [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:132), [sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:15).  
   Two demand instances both mint epoch 0. Their completed tokens can be swapped and both `debug_assert_eq!` checks pass. For unequal tokens, release builds mutate state despite the mismatch. Non-`Clone` prevents an ordinary duplicate, but it does not establish provenance or the required stale-outcome behavior. No stale/foreign/duplicate oracle exists.  
   **Recommendation:** brand each token with an unforgeable demand-instance/active-sweep identity and make one checked `finish` transition return an error without mutation for non-active tokens.

7. **Blocking — invalidation does not terminate the affected epoch and incorrectly advances success throttling.**  
   **File/section:** [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:81), [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:132), [k2r0_demand_sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0_demand_sweep.rs:149).  
   `invalidate()` includes transport reset, panel reinitialization, and epoch discontinuity, yet the affected sweep may subsequently finish through the success path and update `last_present`, delaying the necessary repaint. It merely leaves dirty/full-repaint set. Also, after exactly 2³² invalidations, the wrapping generation equals the mint value and wrongly clears `full_repaint`.  
   **Recommendation:** latch `active_sweep_invalidated` without a wrapping counter and route settlement of such a token through failed/aborted semantics with no throttle advancement.

8. **Important — dropping a token or progress value permanently wedges demand.**  
   **File/section:** [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:35), [sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:123).  
   After `begin_sweep`, ordinary early return, panic, or accidental drop leaves `sweeping = Some(_)`. Even `invalidate()` cannot clear it, and no future sweep can begin. This conflicts with the rule that failures terminate the current epoch and force recovery.  
   **Recommendation:** add a conservative `abandon_active` transition that clears the active epoch, retains demand, forces full repaint, and does not advance throttling; publish a dropped-progress recovery oracle.

9. **Important — `finish_presented` overclaims the observable milestone.**  
   **File/section:** [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:128), [transfer.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:57), SPEC §4/§5.5.  
   Transfer completion explicitly says nothing about physical presentation, while the demand API and `last_present` call the result “presented.” The normative vocabulary permits only `StripeWritten` and `SweepWritten`.  
   **Recommendation:** rename the transition and throttle state to `finish_written`/`last_written`, and expose only the two permitted written milestones.

10. **Blocking — the touch wake-dedup protocol has a real idle-check TOCTOU lost wake.**  
    **File/section:** [touch.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/touch.rs:135), [touch.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/touch.rs:242).  
    Concrete interleaving: start with produced=1, serviced=0; producer observes “already pending”; service marks serviced=1 and exits idle; producer increments produced to 2 and returns `false`. Work is pending, the producer suppressed the wake, and the service is quiescent. Serial “produce during read” tests do not exercise this after-flag-sample race.  
    **Recommendation:** use a separate atomic scheduled/pending latch: producer increments first and wakes on `pending.swap(true) == false`; the consumer clears only with a clear-then-recheck/re-latch protocol, tested with deterministic barriers and a deliberately broken negative control.

11. **Blocking — touch failure restoration and generation-wrap claims are incomplete.**  
    **File/section:** [touch.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/touch.rs:244), [touch.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/touch.rs:264), [k2r0_touch.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0_touch.rs:167).  
    A startup activation driven only by asserted INT has equal generations. If its read fails and INT then deasserts, no pending generation was “restored”; the next activation returns idle. Separately, equality is not unconditionally wrap-safe: 2³² outstanding produces alias serviced and look idle. The wrap test services after every single produce, so it never exercises that ABA condition.  
    **Recommendation:** make a persistent pending/retry latch authoritative across INT-only activation, failure, budget exhaustion, and counter wrap; add combined `startup INT → failure → deassert` and seeded wrap-alias oracles.

12. **Important — unchanged contacts are reported as `Moved` edges.**  
    **File/section:** [touch.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/touch.rs:84), [k2r0_touch.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0_touch.rs:139).  
    Every contact present in consecutive reports produces `Moved`, even when coordinates are identical. A stuck-INT trace therefore surfaces false movement repeatedly; its test discards deltas and misses this. The down/up coalescing boundary is otherwise stated honestly.  
    **Recommendation:** emit no edge when the complete `TouchPoint` is unchanged; add an identical-snapshot negative oracle.

13. **Minor — a zero service budget creates permanent zero-progress activation.**  
    **File/section:** [touch.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/touch.rs:211).  
    `TouchService::new(0)` is accepted, but pending work always returns `BudgetExhausted { surfaced: 0 }`.  
    **Recommendation:** accept `NonZeroU8` or return an explicit invalid-budget error.

14. **Blocking — this is not yet the SPEC §8 K2R-0 suite or an amended K2R-0 contract.**  
    **File/section:** [SPEC.md](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:71), [SPEC.md](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:94), [lib.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/lib.rs:3).  
    The spec still says K2R-0 must not begin before amendment; §6 remains provisional, and the crate/README still identify the stage as K2R-0A. Beyond the four acknowledged open items, the host suite lacks:

    - an external-consumer seam fixture and concrete `StripeWritten`/`SweepWritten` event construction;
    - an enumerated command/chunk reference trace with failure injection at each boundary;
    - the ordinary-drop runtime oracle and corresponding negative control;
    - stale/foreign/duplicate finish and slow-sweep throttle traces;
    - the adversarial touch flag-sample race and combined INT-only failure trace;
    - a profile-specific no-std target fixture/CI gate;
    - compile-fail/pass controls for the claimed unforgeable/type-level boundaries.

    There are also only **three** test files under `tests/`, not the four named in the request. The four already-recorded open gates remain: Xtensa linked probe, board HIL, kernel-admitted sources/real `reactor!`, and sealing.  
    **Recommendation:** amend §6 into the selected normative K2R-0 surface and add a checked trace manifest mapping every §8 row to one positive oracle and adjacent negative control before calling the host slice complete.

15. **Important — the profile does not meet the repository’s “done” checklist or enforcement-layer rule.**  
    **File/section:** [README.md](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/README.md:1), AGENTS.md profile checklist.  
    The five-line README has no enforcement-layer table, non-goals, escape surfaces, deferred gates, or canonical runnable example. No changelog entry or profile UI-pass/UI-fail suite exists. `TouchReader` can be implemented by anyone while its “complete, untorn snapshot” property is only prose; the current documentation does not name that as a documentation-only guarantee or compiling escape.  
    **Recommendation:** complete the profile checklist in-repo: enforcement table, honest negative controls/escape surfaces, canonical example with rationale comments, compile-fail/pass fixtures, deferred gates, and changelog entry.

## Transfer-correction accounting

Corrections 1, 2, 3, 5, 6, and 7 are faithfully reflected in the core host API/tests. Corrections 4 and 9 are honestly recorded as open. Correction 8 is not established by the current model, and correction 5 remains stale in the retained HAL blueprint. The separate cancel/late-completion outcome race is also untested.

## Verdict

**Current verdict: FAIL.**

The K2R-0 host slice can pass this exit review after must-fix findings **1–11 and 14–15** are resolved. Finding **12** should also be fixed before claiming honest edge semantics. Finding **13** is advisory.

Even after the host exit passes, full K2R-0 freeze remains blocked by the four acknowledged external gates: Xtensa compile/link, board HIL, kernel-admitted source plus real reactor fixture, and sealing.

Verification completed:

- `cargo fmt --all --check`: passed.
- `cargo test -p kittens-render`: passed, 31 integration tests across the three present test files.
- Clippy and a fresh `thumbv7em-none-eabi` build could not run because the read-only environment prevented Cargo from creating its build lock/target directory.
- GitKB was attempted as required, but its worktree index was absent and could not be initialized read-only; relationship analysis therefore used direct source and exact-use inspection.
- No files were changed.
