# Verdict: FAIL

Even granting pixel equivalence, the seam fixture, Xtensa, board HIL, kernel admission, and sealing as external OPEN/GATED work, the K2R-0 host slice does not pass. Findings **3–6, 14, and 15 remain unresolved**, including blocking transfer→coverage defects.

## Round-1 findings

1. **ADDRESSED.** The model now uses one reusable done slot with explicit disarm on recovery/drop. Evidence: [k2r0a_a_prime.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0a_a_prime.rs:64). Oracles: `late_completion_after_recovery_is_inert_via_disarm`, `dropped_pending_transfer_disarms_the_slot`, `sequential_transfers_reuse_the_same_slot`, `waker_replacement_wakes_only_the_newest`.

2. **ADDRESSED.** Cancellation stores its settlement at the locked completion-observation point and wakes the poller at [k2r0a_a_prime.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0a_a_prime.rs:194). Oracles: `cancel_then_late_completion_stays_cancelled`, `drain_racing_prior_completion_reports_completed`.

3. **NOT ADDRESSED — Important.** The source trait is corrected at [transfer.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:39), but the probe contains no corrected compile-ready adapter. The [supersession note](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/probes/esp32s3-spi2/README.md:13) supplies only two signatures, while the same README still calls historical `VERDICT.md` “compile-ready” and says a nonexistent probe implements the correction. [CHANGELOG.md](/Users/feral/mydev/kittens-render-wt/CHANGELOG.md:12) repeats that claim. No corresponding oracle or compilable fixture exists.

4. **NOT ADDRESSED — Blocking.** Coverage remains safely forgeable:

   - Every proof-bearing `Settled` field is public, including `outcome`, `epoch`, and `region`: [transfer.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:89).
   - `stripe_written(&self)` can mint unlimited witnesses: [transfer.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:104).
   - `InFlight::new` accepts the transfer and its claimed target independently: [transfer.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:141).

   Safe code can directly construct a never-started `Settled { outcome: Completed, ... }`, change a cancelled settlement to `Completed`, or relabel one completion for every planned region. The cooperative oracle `cancelled_and_failed_transfers_cannot_mark_coverage` does not exercise those paths.

5. **NOT ADDRESSED — Blocking.** The original fixed-panel and immutable-snapshot problems remain:

   - Public `SweepPlan::new` and `FrameDemand::new` still accept any caller-designated “panel,” including the original 1×1 counterexample: [sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:62), [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:69).
   - `snapshot() -> &S` does not make unconstrained `S` immutable; `Cell`, atomics, or shared handles can mutate between stripes: [sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:183). `snapshot_is_immutable_through_the_sweep_and_returned_at_the_end` tests only `u32`.
   - `StripeWritten` carries no demand/sweep provenance: [sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:25). Separate demands both start at epoch 0, so one settlement can advance matching foreign sweeps.

6. **NOT ADDRESSED — Blocking.** Immediate foreign terminal tokens are checked in release code at [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:172), but provenance uses a wrapping `AtomicU32` at [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:34). After 2³² constructors, demand IDs alias. The oracle `foreign_and_stale_settlement_is_rejected_without_mutation` exercises only one ordinary foreign swap—not stale settlement, duplicate/replay, or wrap—and checks only part of observable state.

7. **ADDRESSED.** The bool invalidation latch and discarded-settlement path are implemented at [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:107). Oracle: `invalidation_mid_sweep_discards_that_sweeps_settlement`.

8. **ADDRESSED.** Explicit dropped-sweep recovery exists at [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:214). Oracle: `abandon_recovers_a_dropped_sweep`. A new misuse hole in this API is listed below.

9. **ADDRESSED.** The implementation uses only written milestones—`StripeWritten`, `SweepWritten`, `finish_written`, and `last_written`: [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:62). Oracle coverage includes `effective_settlement_advances_throttle_and_clears_obligation`.

10. **ADDRESSED.** Increment-then-latch and clear/recheck are implemented at [touch.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/touch.rs:165). Oracles: `increment_then_latch_closes_idle_check_lost_wake` and negative control `negative_control_check_before_increment_loses_idle_wake`.

11. **ADDRESSED.** Persistent retry survives INT-only failure, budget exhaustion, and generation alias. Oracles: `startup_int_read_failure_retries_after_int_deasserts`, `budget_exhaustion_keeps_retry_latched_after_int_deasserts`, `seeded_two_to_the_32_produces_cannot_alias_pending_to_idle`.

12. **ADDRESSED.** Unchanged points produce no movement edge at [touch.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/touch.rs:86). Oracle: `stuck_int_identical_snapshots_emit_no_false_movement_edges`.

13. **ADDRESSED.** `TouchService::new` requires `NonZeroU8`: [touch.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/touch.rs:275). Oracle: `service_budget_is_nonzero_by_construction`.

14. **NOT ADDRESSED — Blocking.** Revision 3 and a direct no-std library build landed, but the host evidence remains incomplete or false:

   - [lib.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/lib.rs:3), [geometry.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/geometry.rs:1), and [Cargo.toml](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/Cargo.toml:3) still call section 6 provisional/K2R-0A.
   - [TRACE-MANIFEST.md](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/TRACE-MANIFEST.md:19) claims request-during-sweep and complete demand-row coverage, but no request occurs during an active sweep.
   - There is no slow-successful-sweep throttle oracle.
   - The stale/foreign/duplicate row names only a foreign-only test.
   - The manifest promises adjacent negative controls, but most rows have none.
   - Building the generic rlib is not the requested external no-std consumer/link fixture.
   - The normative host/full boundary is contradictory: [SPEC §5](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:35) says capabilities are sealed, while [§6](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:59) says they are open; §7/§11 require the still-gated target probe before K2R-0A completion.

   Also, contrary to the question’s premise, TRACE-MANIFEST has no standalone board-HIL or sealing rows.

15. **NOT ADDRESSED — Important.** The README’s enforcement table, non-goals, escape surfaces, deferred gates, and changelog landed. There is still no canonical runnable example and no `tests/ui`, `tests/ui-pass`, or `tests/ui-fail` suite. The table’s “unforgeable witness mint” claim at [README.md](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/README.md:19) is disproved by the public `Settled` fields above.

## New findings introduced by the fixes

- **Blocking — proof-carrier rewriting and replay.** Public mutable `Settled` proof fields plus non-consuming `stripe_written` permit direct construction, outcome rewriting, target relabeling, and unlimited witness minting.

- **Blocking — premature abandonment permits concurrent sweeps.** A caller can retain sweep E0, call `abandon_active`, begin E1, and drive both write streams. Terminal rejection of E0 does not prevent physical interleaving.

- **Important — provenance exhaustion.** `DEMAND_IDS` aliases after 2³² constructions; `next_epoch += 1` likewise lacks explicit exhaustion handling despite unconditional uniqueness/monotonicity claims.

- **Important — throttle time is caller-forgeable.** `Tick(pub u64)` is described as crate-owned and monotonic but accepts arbitrary or regressing time at [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:14). A caller can bypass slow-sweep throttling by supplying a false finish time.

- **Minor — liveness-critical results lack `#[must_use]`.** Ignoring `TouchGenerations::produce` or `Activation::{BudgetExhausted, ReadFailed}` can strand latched work while later wakes deduplicate.

- **Minor — remaining contract/prose drift.** Generic `InFlight<X,S>` is only conditionally `Unpin`; several comments cite obsolete section numbers; `full_repaint` says “always true” although subsequent sweeps receive false; the transfer log says 11 traces while 12 tests exist.

## Must-fix versus advisory

Must-fix before another exit review:

- Make the transfer→stripe→sweep proof chain private, operation-bound, provenance-branded, and single-use.
- Bind “full panel” to an admitted display geometry, and either enforce snapshot immutability or narrow/document the compiling escape.
- Prevent `abandon_active` while the old sweep remains live, or explicitly demote one-sweep-in-flight to a documented caller obligation.
- Handle demand/epoch exhaustion and define the trusted monotonic-time boundary.
- Add the missing host traces, negative controls, external no-std consumer fixture, UI controls, and canonical example.
- Reconcile revision/stage/host-versus-full acceptance language and stop advertising a nonexistent corrected probe as compile-ready.

Advisory: add `#[must_use]`, state the exact `Unpin` bounds, repair stale citations/counts, and strengthen rejection oracles to prove state/progress remains unchanged.

## Verification

- `cargo fmt --all --check`: passed.
- `cargo test -p kittens-render --all-targets --all-features`: **41/41 passed**.
- Clippy and a fresh thumb rebuild could not acquire Cargo’s build lock in the read-only sandbox.
- GitKB was attempted but could not create its worktree index read-only; direct definition/use inspection was used.
- Worktree remained clean; no files changed.
