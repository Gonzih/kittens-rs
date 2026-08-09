# K2R-0 trace manifest

SPEC section 8 requires each suite row to map to a named positive oracle
with an adjacent negative control, and exit-review finding 14 requires this
manifest. Statuses: ✓ (oracle in CI), **OPEN** (host work remaining),
**GATED** (blocked on a named external gate).

| SPEC §8 requirement | Status | Oracle(s) / gate |
|---|---|---|
| completion polled-then-lost | ✓ | `polled_then_lost_arbitration_gets_exactly_one_wake` |
| completion unpolled-below-winner | ✓ | `unpolled_below_winner_recovers_on_first_poll` |
| completion during waker registration | ✓ | `completion_during_registration_is_not_lost`; negative control `negative_control_check_then_register_loses_the_wake` |
| cancel-and-drain on every in-flight state | ✓ | `cancel_then_late_completion_stays_cancelled` (linearization), `drain_racing_prior_completion_reports_completed`, `spare_is_writable_during_flight_and_drain_flag_clears` |
| ordinary-drop runtime oracle | ✓ | `dropped_pending_transfer_disarms_the_slot` (transfer), `abandon_recovers_a_dropped_sweep` (demand) |
| resource recovery under injected failure | ✓ (model boundary) / **GATED** (enumerated command/chunk trace) | `failure_settles_returns_resources_and_mints_no_witness`; the enumerated per-command/chunk trace belongs to the concrete transport integration — Xtensa/board gate |
| waker replacement, late IRQ, slot reuse | ✓ | `waker_replacement_wakes_only_the_newest`, `late_completion_after_recovery_is_inert_via_disarm`, `sequential_transfers_reuse_the_same_slot` |
| sweep coverage is a construction | ✓ | `cancelled_and_failed_transfers_cannot_mark_coverage`, `out_of_order_witnesses_are_rejected`, `plan_tiles_the_panel_exactly_including_partial_last_stripe`, `invalid_plans_are_rejected_including_overflow` |
| full-frame vs stripe pixel equivalence (FrameEpoch reconstruction) | **OPEN** | requires the draw-target integration layer (embedded-graphics stripe target); lands with that slice, not fabricated before it |
| demand-policy state table | ✓ | one oracle per table row in `k2r0_demand_sweep.rs`: coalescing/monotonic epochs, one-in-flight, request-during-sweep, throttle/eligibility, failed-retains, invalidation-discards, effective-clears, abandon-recovers |
| stale/foreign/duplicate finish | ✓ | `foreign_and_stale_settlement_is_rejected_without_mutation` |
| snapshot immutability through the sweep | ✓ | `snapshot_is_immutable_through_the_sweep_and_returned_at_the_end` |
| touch interleavings (findings 10–13 set) | ✓ | `k2r0_touch.rs` (16 oracles): `increment_then_latch_closes_idle_check_lost_wake` + `negative_control_check_before_increment_loses_idle_wake`, `startup_int_read_failure_retries_after_int_deasserts`, `budget_exhaustion_keeps_retry_latched_after_int_deasserts`, `seeded_two_to_the_32_produces_cannot_alias_pending_to_idle`, `stuck_int_identical_snapshots_emit_no_false_movement_edges`, `service_budget_is_nonzero_by_construction`, plus the round-1 set |
| Outcome-B receiver/task traces | not applicable | mechanism C selected (`K2R0A-LOG.md`); B was not needed |
| external-consumer seam fixture | **GATED** | bilateral seam co-sign with the `kittens-code` workstream (SPEC section 10) |
| target compile/link against the chosen HAL SHA | **GATED** | Xtensa toolchain approval (`probes/esp32s3-spi2/`) |
| real `kittens::reactor!` integration fixture | **GATED** | kernel-admitted source carrier (K2R-0A open item 3; root SPEC 37.6 comparison) |
| crate `no_std` CI gate | ✓ | `cargo build -p kittens-render --target thumbv7em-none-eabi` in CI |

Silent caps rule (root AGENTS.md): nothing above is claimed beyond its
status column; OPEN and GATED rows are the honest remainder of the slice.
