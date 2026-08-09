I found 12 blocking defects, 5 important defects, and one minor cut.

1. **Blocking — header, §6, and RESEARCH §§9–10 — the candidate API is normative before its feasibility gate.**

   [§6](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:48) calls the shapes controlling contracts, while [RESEARCH §10](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/RESEARCH.md:104) says they are leading candidates pending K2R-0A. That biases the experiment and permits K2R-0 to freeze a carrier the spike must be free to reject.

   **Recommendation:** mark §§6.2–6.3 provisional K2R-0A candidates; require K2R-0A to amend the spec with the selected, demonstrated API before K2R-0 begins.

2. **Blocking — §§6.2 and 7 — `type Completion` cannot wrap the ESP-HAL waiter as intended.**

   ESP-HAL starts an owning transfer, but completion is `wait_for_done(&mut transfer)`, followed by consuming `wait()` to recover SPI and buffer. The safe adapter is a compiler-generated `async move` state machine that owns and borrows the transfer. Its type is anonymous; `type Completion = impl Future` remains unstable, while boxing violates no-alloc and a hand-written owner-plus-borrower is the self-reference the gate rejects. Pin admission solves polling after construction, not construction or naming. See the official [`SpiDmaTransfer` API](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32/esp_hal/spi/master/struct.SpiDmaTransfer.html) and Rust’s [`impl_trait_in_assoc_type` status](https://doc.rust-lang.org/unstable-book/language-features/impl-trait-in-assoc-type.html).

   A named `is_done()` future that self-wakes continuously could satisfy the literal signature, so the gate also accidentally admits busy polling.

   **Recommendation:** require an exact safe, no-alloc target compile probe using RPITIT, an upstream named transfer state, or another demonstrated shape—and explicitly reject self-waking/busy-poll completion.

3. **Blocking — §§6.2, 7, 8.1, and 10 — cancellation cannot return resources through `Future::Output`.**

   Sequence: start succeeds, completion is pending, shutdown wins, and the completion is dropped. Drop produces no `Output`; therefore neither `Returned` nor `Failed` reaches the caller. ESP-HAL cancels a dropped transfer and drops its SPI/buffer; it does not return them. `StripeInFlight` also drops its spare. The API only guarantees recovery when the future reaches `Ready`.

   **Recommendation:** define an explicit cancel-and-drain transition that is driven to settlement and returns transport, sent buffer, and spare; document ordinary drop separately as a non-returning boundary.

4. **Blocking — §7 — K2R-0A’s outcomes are not exhaustive or decision-complete.**

   Root [§37.6](/Users/feral/mydev/kittens-render-wt/SPEC.md:3556) permits an outer `Unpin` admitted adapter containing caller-pinned inner storage. That is neither receiver-changing Outcome A nor a task/channel Outcome B. A custom interrupt-backed transfer state is another possibility. There is also no “neither works” result and no tie-break if both A and B pass.

   Pass criteria such as “all resources recovered on cancellation” and “no lost wake” are undefined over which cancellation operation and which finite schedules, so they are not currently decidable.

   **Recommendation:** publish a candidate matrix including caller-pinned storage and a no-solution outcome, plus an ordered selection rule and finite trace set before running the spike.

5. **Blocking — §7 Outcome B — the required channel source and task lifecycle are unspecified.**

   Current no-std `Latched` and `FixedQueue` sources are explicitly local and non-waking; Tokio channels are host-only, and `ReactorSource` is sealed. See [source/mod.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens/src/source/mod.rs:177). Outcome B therefore also needs a kernel admission change, not merely a profile task.

   The spec does not define who spawns, owns, stops, or joins the task; whether it is per display or per transfer; channel capacity/backpressure; or recovery when send/receiver closure occurs.

   **Recommendation:** make a caller-owned `run` future, fixed-capacity endpoints, close semantics, and a sealed no-std channel adapter part of Outcome B’s required spike artifact and root-spec amendment.

6. **Blocking — §6.3 — the typestate surface cannot be driven externally or completed.**

   `StartFailed` is undefined. Both structs have private fields but no constructors. There is no `spare_mut`, poll/source interface, cancellation transition, or completion transition that reunites returned transport, sent buffer, and spare into the next `PreparedStripe`.

   Worse, `region` disappears during `start`: `StripeInFlight` retains the epoch but not the region, so generic code cannot emit the mandated `StripeWritten { epoch, region }`. Pinning the aggregate with `C: !Unpin` also requires a specified safe projection mechanism before the spare can remain independently movable/writable.

   **Recommendation:** after K2R-0A, specify the complete outcome-specific transition API: construction, spare access, retained region, pin strategy, start failure, completion/failure, cancellation, and reconstruction of the next prepared state.

7. **Blocking — §§6.2, 9, and 11 — `BlockingRegionWrite` freezes before the SH8601 transport gate.**

   Stock `sh8601-rs` has `flush(&mut self)` over a private framebuffer; `partial_flush` copies from that framebuffer into an allocating `Vec`. It cannot consume the supplied external stripe buffer honestly. See the cached exact [0.1.8 source](/Users/feral/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sh8601-rs-0.1.8/src/lib.rs:681).

   RESEARCH §6 says the upstream/fork `write_region` decision is required before the SPEC freezes, but SPEC §9 leaves it as a K2R-1 gap.

   **Recommendation:** remove `BlockingRegionWrite` from the K2R-0 frozen surface unless K2R-0A first compiles a no-allocation adapter against an exact fork/upstream SHA.

8. **Important — §§5 and 6.2 — the traits permit the temporal violations their prose forbids.**

   An unsealed `start_region` implementation may perform the entire blocking write and return an immediately-ready future. Conversely, `write_region` may return before completion. Ownership does not distinguish those behaviors, contrary to the enforcement table.

   **Recommendation:** seal the capability traits and admit only reviewed backend adapters; treat raw backend access as the documented compiling escape surface.

9. **Blocking — §6.4 — no rule determines sweep completion or full repaint.**

   There is no panel extent, validated region list, tiling/order rule, overlap/gap rule, or coverage state. Two overlapping writes that omit a row can be called “every stripe.” After failure, one arbitrary successful region can likewise be declared a full repaint.

   Failure does not explicitly terminate the current epoch, and idle transport reset/epoch discontinuity cannot set the obligation because `FrameDemand` has no general invalidation method. Successful completion also does not tell `FrameDemand` whether the caller actually performed a full repaint.

   **Recommendation:** keep K2R-0 minimal by defining one validated, fixed full-panel sweep plan whose consuming progress token alone can produce sweep completion; defer damage sweeps/history.

10. **Blocking — §§6.4–6.5 — `FrameDemand` does not define its central transitions.**

   The sequence `request → begin(E1) → request → presented(E1)` does not state whether the second request survives, although §6.4 requires it to target the next epoch. A copied stale epoch can later be passed to `sweep_presented` while another sweep is active; unit-returning methods specify neither rejection nor no-op behavior.

   `last_present` also cannot mean presentation time because `sweep_presented(epoch)` receives no time. A long sweep therefore has two incompatible throttle interpretations. `on_eligible` has no normative transition, and the initial/clearing state of `full_repaint_required` is unstated.

   **Recommendation:** replace raw epoch callbacks with an unforgeable active-sweep token and a single `finish(token, now, outcome)` transition, backed by a complete state table including request-during-sweep and stale/duplicate outcomes.

11. **Blocking — §6.6.4 and §8.5 — latest-state coalescing contradicts lossless edges.**

   If `Down(1)` and then `Up(1)` are successfully snapshotted before the consumer runs, retaining only the newest state loses observed edges. Repeated toggles require unbounded transition history; the fixed two-point capacity bounds simultaneous contacts, not temporal backlog. An even faster down/up that occurs before any snapshot is fundamentally unobservable.

   The oracle strengthens the already-impossible promise from “observed edges survive” to unqualified “no down/up edge is lost.”

   **Recommendation:** choose honest latest-state semantics for this slice: intermediate transitions may coalesce, while every surfaced report is complete and untorn. Defer lossless transitions to a separately bounded queue with explicit overflow policy.

12. **Important — §§6.6 and 7 — the generation-latch service algorithm is missing and independent of DMA pinning.**

   No rule defines observed versus serviced generation, wrapping, startup with INT already asserted, or IRQ arrival during a successful read followed by INT deassertion. “Re-service while INT remains asserted” permits an unbounded loop on a stuck line and can monopolize the harness reactor.

   Pin admission does not create the required ISR-side wake-aware producer handle; touch needs its own admitted source decision.

   **Recommendation:** specify an atomic `produced_generation`/`serviced_generation` state machine, service at most a fixed number of snapshots per activation, and re-latch/yield on generation change, asserted INT, or failure.

13. **Blocking — §§8.1 and 11 — the dropped-permit oracle is impossible as written.**

   No permit type exists in §6. Rust permits any owned value to be explicitly dropped; this cannot be made a compile error. A post-drop reuse failure would prove only ordinary move semantics, not cancellation recovery.

   **Recommendation:** remove “dropped permit” from compile-fail acceptance and replace it with a runtime oracle for the explicitly specified drop/cancel transition.

14. **Important — §8 — the decisive oracle cases are missing or underspecified.**

   Missing cases include:

   - both selection-loss positions: completion polled before another winner and left unpolled below an earlier winner;
   - completion before first poll and during waker registration;
   - request during sweep, stale/duplicate epoch outcomes, and slow-sweep throttling;
   - exact region coverage, failure aborting the epoch, and full-repaint clearing;
   - Outcome B receiver closure, task shutdown, and resource-return backpressure;
   - a real `kittens::reactor!` integration fixture;
   - target compile/link against the chosen HAL SHA.

   “Failure at every command/chunk boundary” is also vacuous until the reference command/chunk trace is enumerated. All-host tests alone cannot establish the claimed exact ESP-HAL compatibility.

   **Recommendation:** replace §8 with a named trace matrix plus an exact target compile fixture, with each state transition and transport boundary independently observable.

15. **Important — §3 and §11 — the merge surface is not mechanically emit-able.**

   `StripeWritten`, `BusIdle`, `FramePresented`, touch reports, source builders, task endpoints, readiness markers, ordering, shutdown, and canonical wiring are prose rather than types or declarations. This is materially weaker than TUI [§6.7](/Users/feral/mydev/kittens-render-wt/crates/kittens-tui/SPEC.md:227).

   The sibling contract says drivers own one reactor per session and currently negotiates only a protocol-event frontend seam; it does not yet authorize renderer ownership or a second task loop. See [kittens-code §6](/Users/feral/mydev/kittens-render-wt/docs/kittens-code/SPEC.md:183) and [§13](/Users/feral/mydev/kittens-render-wt/docs/kittens-code/SPEC.md:406).

   **Recommendation:** add one bilateral seam section to both specs and gate acceptance on an external-consumer canonical reactor fixture covering construction, ordering, task ownership, and teardown.

16. **Important — §§3, 6.4, and 6.5 — the advertised `DrawTarget`/snapshot and time boundaries do not exist.**

   Section 3 says consumers build on the profile’s `DrawTarget` contracts, but no target type, global-coordinate bounding-box behavior, pixel format, or constructor is specified. `begin_sweep` returns only an epoch; nothing binds that epoch to an immutable scene snapshot, so callers may mutate the scene mid-sweep while satisfying all types.

   Likewise, “Tokio instant on host, platform instant on target” does not identify a stable public path or portable bare-metal type.

   **Recommendation:** define one crate-owned `Sweep<S>` value containing the immutable snapshot, target geometry, repaint mode, and crate-owned monotonic tick representation.

17. **Important — §6.4.4 — the milestone names overclaim the available observations.**

   Both transport APIs expose one completion boundary. There is no independent producer, consumer, or oracle for `BusIdle`. `FramePresented` is emitted when GRAM writes complete even though §6.2 admits scanout may continue and TE synchronization is deferred; the name implies a physical fact the slice cannot observe.

   **Recommendation:** expose only `StripeWritten` and `SweepWritten` in these slices; defer `BusIdle` and physical presentation until hardware evidence identifies distinct milestones.

18. **Minor — §6.5 — `on_eligible` has not earned public syntax.**

   It has no specified state transition beyond “call from the timer handler,” and `begin_sweep(now)` can consume/clear an elapsed schedule itself.

   **Recommendation:** remove `on_eligible`; make `begin_sweep(now)` the sole operation that acknowledges elapsed eligibility.

## Verdict

K2R-0A may start only as a non-freezing feasibility experiment. It cannot honestly start as implementation of the current normative §6 API, and K2R-0 must not start yet.

First change:

1. Make the owning transport and typestate shapes provisional.
2. Define explicit cancellation/resource recovery.
3. Make K2R-0A exhaustive, outcome-specific, and capable of returning “no viable shape.”
4. Choose honest touch delivery semantics.
5. Define deterministic sweep/demand transitions before rewriting the oracle matrix.

Verification completed: all requested repository contracts, exact cached `sh8601-rs` 0.1.8 source, official ESP-HAL 1.1 API, and a local Rust 1.96 compile probe confirming associated-type `impl Trait` remains unstable. The full HAL adapter could not be compiled because the spec intentionally provides no selected SHA or implementation artifact yet. GitKB’s worktree index was unavailable under read-only permissions, so kernel relationship checks were completed directly against `source/mod.rs`.
