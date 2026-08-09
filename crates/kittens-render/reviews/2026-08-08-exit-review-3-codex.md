# Verdict: FAIL

The K2R-0 host slice does not pass round 3. Pixel equivalence, seam co-sign, Xtensa, board HIL, kernel admission, and sealing can remain honestly outside this slice; they are not the cause of failure. Host-core defects remain.

## Must-fix findings

1. **Blocking — transfer completion is still independent of the target it certifies.**

   [`OwnedTransfer`](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:33) carries no region or target identity. [`InFlight::new`](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:164) separately accepts an already-started transfer and a `StripeTarget`. Safe code can therefore write region A, attach target B, and mint B’s witness after A completes.

   The external no-std fixture is already a counterexample: it constructs a transfer preclassified `Completed`, performs no start or region write, then mints accepted coverage at [main.rs](/Users/feral/mydev/kittens-render-wt/fixtures/render-no-std/src/main.rs:63). The canonical example likewise relies on a cooperative assertion between separate `start` and `InFlight::new` calls at [host_sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/examples/host_sweep.rs:183).

   Sealing would block arbitrary trait implementations, but it would not stop pairing a reviewed transfer for A with target B. This contradicts [SPEC §6.2](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:71).

2. **Blocking — cancellation or failure does not terminate the epoch.**

   [`Settled::stripe_written`](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:122) merely returns `None` for `Cancelled`/`Failed`. The sweep remains usable and can remint the same target, retry successfully, and finish the same epoch. [`Sweep::abort`](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:311) is optional.

   The oracle explicitly accepts this behavior: after both a cancellation and failure it says the remaining paths include “more Completed transfers or abort” at [k2r0_demand_sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0_demand_sweep.rs:163). That contradicts [SPEC §5.3](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:37) and §8: any failure terminates the epoch and forces full repaint.

3. **Blocking — target issuance and terminal transitions do not own outstanding operations.**

   [`Sweep::next_target(&self)`](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:249) can mint unlimited identical targets. The source calls this harmless, but it is harmless only to the coverage counter. A duplicate target can survive sweep settlement and later cause a stale physical write.

   The test suite itself pre-mints such a duplicate at [k2r0_demand_sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0_demand_sweep.rs:370), then proves only that its witness is rejected after the physical transfer completes.

   Premature abandonment is therefore not fully resolved. The documentation requires draining transfers that already exist before replacement, but does not prohibit retaining E0 and starting new E0 transfers after E1 begins. `Sweep::abort` has the same problem: it can settle demand while an independently retained target/transfer remains live.

4. **Important — finding 3’s corrected adapter still does not exist.**

   [adapter-blueprint.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/probes/esp32s3-spi2/adapter-blueprint.rs:1) is 39 lines of documentation comments and partial pseudocode—no imports, concrete type, `OwnedTransfer` implementation, `cancel`, or `recover`.

   Yet the [probe README](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/probes/esp32s3-spi2/README.md:32) calls it corrected and shape-complete, while [CHANGELOG.md](/Users/feral/mydev/kittens-render-wt/CHANGELOG.md:12) implies it compiles behind the Xtensa gate. The Xtensa build may remain gated, but this artifact must either become the claimed source or be labeled a pseudocode delta.

5. **Important — finding 14’s evidence manifest remains inaccurate.**

   - The manifest defines ✓ as “oracle in CI,” then marks `host_sweep` ✓ at [TRACE-MANIFEST.md](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/TRACE-MANIFEST.md:38). [CI](/Users/feral/mydev/kittens-render-wt/.github/workflows/ci.yml:19) builds the example as a zero-test harness but never runs `main`.
   - [SPEC §8](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:177) still requires adjacent negative controls; the manifest weakens this to “where meaningful,” and most runtime rows name none.
   - There is no foreign/stale rejection oracle for `finish_failed`, no cross-demand `StripeWritten` rejection oracle, and existing rejection tests do not verify all observable state remains unchanged.
   - The sweep-coverage ✓ row is false because findings 1–3 remain.
   - The normative `write_region` exact-SHA/no-allocation gate in [SPEC §9](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:181) is absent from the manifest.

6. **Important — epoch exhaustion remains build-profile-dependent.**

   Demand-ID exhaustion is correctly sticky. Epochs still use unchecked [`next_epoch += 1`](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:166): debug panics before returning epoch `u64::MAX`, while release returns it and wraps internal state. That is not one precise 2^64-sweep horizon.

## Round-2 disposition

| Item | Result |
|---|---|
| 3 — corrected HAL blueprint | **NOT ADDRESSED** |
| 4 — transfer→coverage proof chain | **NOT ADDRESSED** |
| 5 — admitted panel/snapshot/provenance | **ADDRESSED**; custom geometry and interior mutability are honestly published escapes |
| 6 — demand-token provenance | **ADDRESSED in core**; demand IDs and release-path rejection are fixed, though oracle coverage is incomplete |
| 14 — contract/oracle/fixture suite | **NOT ADDRESSED** |
| 15 — profile checklist | **NOT ADDRESSED overall**; the artifacts now exist, but the required enforcement table still falsely says never-started stripes are unmarkable |
| Proof-carrier rewriting/replay | **ADDRESSED narrowly**; fields are private, minting is single-use, and witnesses are move-only. Equivalent targets can still be reminted—finding 3 above |
| Premature abandonment | **NOT ADDRESSED completely** |
| Provenance exhaustion | **NOT ADDRESSED completely**; demand IDs fixed, epochs not |
| Forgeable time | **ADDRESSED** as a documented trusted-time boundary with regression clamping |
| `#[must_use]` | **ADDRESSED** on `produce`, `Activation`, and sweep witnesses |
| Prose drift | **NOT ADDRESSED**; original Unpin/count/stage drift is fixed, but probe, manifest, and enforcement claims remain false |

## Advisory

- Throttle deadlines use saturating addition at [demand.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/demand.rs:159). Near `u64::MAX`, a positive minimum interval can be shortened or become repeatedly eligible at the same tick.
- The generic “independently writable spare” claim does not exclude safe shared/interior-mutable backing storage between the sent buffer and spare; publish this escape or constrain the buffer types.
- Add explicit `.clone()` compile-fail controls and unchanged-progress/state assertions so the UI/runtime suite catches future regressions.

## Verification

- `cargo fmt --all -- --check`: passed.
- Package tests: passed—46 runtime tests, 15 compile-fail snapshots, 5 compile-pass controls.
- Workspace tests: passed.
- No masked trybuild snapshot found.
- Existing host example binary ran successfully through four stripes.
- Existing thumb ELF is statically linked ARM and retains `_start`, `FrameDemand::new`, and `DEMAND_IDS`.
- Fresh clippy/thumb rebuilds were blocked by read-only Cargo-lock creation.
- GitKB could not open its read-only worktree index; exact source/use inspection was used.
- Worktree remained clean.

The next round should not start until the transfer, target, settlement, and sweep are one operation-bound lifecycle: single outstanding target, target consumed by transfer start, every settlement reconciled with its owning sweep, failure forcing abort, and no terminal demand transition while stale writes remain possible.
