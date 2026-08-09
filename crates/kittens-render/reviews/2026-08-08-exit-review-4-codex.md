# Verdict: FAIL

The K2R-0 host slice does not pass round 4. The honestly labeled pixel-equivalence, seam, Xtensa, board-HIL, kernel-admission, blocking-adapter, and sealing gates may remain outside the slice; they are not the reason for failure.

## Round-3 disposition

| Item | Result |
|---|---|
| 1 — transfer/target coupling | **NOT ADDRESSED — Blocking** |
| 2 — cancellation/failure terminates epoch | **ADDRESSED** |
| 3 — outstanding-operation ownership | **NOT ADDRESSED — Blocking** |
| 4 — corrected adapter artifact | **ADDRESSED** |
| 5 — evidence manifest/oracles | **NOT ADDRESSED overall — Important** |
| 6 — epoch exhaustion | **ADDRESSED** |
| Advisory — throttle saturation | **ADDRESSED** |
| Advisory — shared sent/spare backing | **ADDRESSED** |
| Advisory — clone and unchanged-state controls | **NOT ADDRESSED fully** |

### 1. Transfer completion is still independently pairable with a target

`start_flight` accepts an arbitrary caller-owned `FnOnce(Region) -> Result<X, E>` and attaches whatever `X` it returns to the target ([transfer.rs:177](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:177)). Safe code can still do:

```rust
let transfer_for_a = adapter.start(a);
target_b.start_flight(spare, |_| Ok(transfer_for_a));
```

The compiling control confirms the supplied region can be ignored ([dishonest_starter_ignores_region.rs:41](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/ui-pass/dishonest_starter_ignores_region.rs:41)). Sealing `OwnedTransfer` does not prevent pairing an honestly implemented, prestarted A-transfer with B.

Therefore SPEC §6.2’s statement that the old independent pairing “does not exist” is false ([SPEC.md:74](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:74)), as is the pseudocode delta’s categorical claim ([adapter-blueprint.rs:20](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/probes/esp32s3-spi2/adapter-blueprint.rs:20)).

### 2. Failure poisoning is addressed

`Settled::into_parts` necessarily returns one `StripeSettlement`; cancellation/failure becomes `Unwritten` ([transfer.rs:114](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:114)). A matching unwritten settlement irreversibly poisons the sweep ([sweep.rs:355](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:355)), preventing further targets or successful finish. Both cancellation and failure have passing oracles ([k2r0_demand_sweep.rs:181](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0_demand_sweep.rs:181)).

### 3. Terminal transitions still do not own outstanding work

Single issuance is fixed: `Ready/Outstanding/Poisoned` prevents duplicate targets for one position.

The terminal half remains open. `Sweep::abort` can consume the sweep while its target or flight remains live ([sweep.rs:422](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:422)); `finish_failed` then permits replacement. `abandon_active` likewise permits a retained old sweep to mint after replacement begins—the repository publishes that exact counterexample ([old_sweep_survives_abandon.rs:15](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/ui-pass/old_sweep_survives_abandon.rs:15)).

Documenting this escape does not satisfy the round-3 invariant. It also contradicts SPEC §5.3’s absolute “Every started transfer settles through its owning sweep” rule ([SPEC.md:37](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:37)).

The proposed `invalidate()` mitigation is timing-sensitive: an invalidate performed after abort but before replacement does not mark that replacement invalidated, because `begin_sweep` clears `active_invalidated` ([demand.rs:167](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:167), [demand.rs:200](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:200)).

### 4. Probe labeling is addressed

The probe now unambiguously says that no compile-ready adapter exists and that `adapter-blueprint.rs` is pseudocode only ([README.md:1](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/probes/esp32s3-spi2/README.md:1), [adapter-blueprint.rs:1](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/probes/esp32s3-spi2/adapter-blueprint.rs:1)). The actual Xtensa adapter remains correctly gated. Its target-pairing sentence still inherits finding 1.

### 5. The manifest still overclaims

The CI example execution, `write_region` gate, and missing rejection rows were repaired. Remaining defects:

- “Sweep coverage is a construction” is marked ✓ despite findings 1 and 3 ([TRACE-MANIFEST.md:21](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/TRACE-MANIFEST.md:21)).
- The privacy fixture calls nonexistent `InFlight::new` ([in_flight_new_is_not_public.rs:5](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/ui/in_flight_new_is_not_public.rs:5)), while the real private constructor is `from_started` ([transfer.rs:220](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:220)). Publishing `from_started` would reopen the bypass while this compile-fail continued passing.
- `StartFlightError` is normatively move-only but has no explicit clone compile-fail.
- Demand rejection tests snapshot four immediate getters, but do not behaviorally verify future-observable epoch/throttle state after every rejection ([k2r0_demand_sweep.rs:35](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0_demand_sweep.rs:35)).

### 6. Epoch exhaustion is addressed

Epoch state is `Option<u64>` with checked increment, so `u64::MAX` is minted once and exhaustion remains sticky with profile-independent behavior ([demand.rs:69](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:69)).

## New batch-6 defects

1. **Blocking: acceptance-atomic `Err` is unenforced.** A starter can start a reviewed transfer, return that live transfer inside `E`, recover the unchanged target, and retry while the first write remains live. This directly contradicts the “no later physical write” contract ([transfer.rs:170](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:170)). Trait sealing cannot close an arbitrary public callback.

2. **Important: the constructor compile-fail targets the obsolete spelling**, allowing a future public `from_started` regression to escape its oracle.

3. **Important: clone coverage omitted the newly introduced `StartFlightError`.** I also recommend controls for `InFlight` and `Sweep`.

4. **Minor drift:** [K2R0A-LOG.md:22](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/K2R0A-LOG.md:22) still says 12 A-prime tests; there are now 13.

## Must fix before round 5

- Replace the arbitrary transfer-returning callback with a constrained, operation-bound starter/admission capability that prevents prestarted A/B pairing and start-then-`Err`.
- Make sweep continuation own the outstanding target/flight lifecycle; do not permit abort, failed settlement, or abandonment to authorize replacement while old work can still start or write.
- Repair the manifest and UI suite: test `from_started` and struct-literal privacy, add start-then-`Err` and `StartFlightError::clone` controls, and strengthen unchanged-state behavior checks.
- Reconcile the conflicting SPEC claims and update the stale evidence count.

Verification: rustfmt, package tests, and workspace tests passed. Current artifacts contain 55 runtime tests, 24 compile-fail controls, and 7 compile-pass controls; cached batch-6 release tests, host example, downstream fixture, and thumb ARM ELF also passed. Fresh clippy and thumb rebuilds were blocked by the read-only Cargo build-lock restriction. GitKB was unavailable for the same read-only-index reason, so relationships were checked directly. The worktree remained clean.
