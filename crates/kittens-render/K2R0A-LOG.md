# K2R-0A experiment log

Per SPEC section 7. This log is the experiment record; the spec is amended
only from what is demonstrated here.

## Selected mechanism (2026-08-08)

**C completion mechanism in an A′ carrier.** The reviewer's engineering
verdict (retained in full at `probes/esp32s3-spi2/VERDICT.md`): on esp-hal
v1.1.0 (`d48f747`), a profile-owned SPI2 `TransferDone` ISR with a
critical-section waker slot honestly implements the `poll_done` boundary —
stable Rust, no alloc, no unsafe self-reference, **no upstream changes** —
using `SpiDma<Blocking>::set_interrupt_handler`/`listen`/`unlisten`,
`SpiInterrupt::TransferDone`, `SpiDmaTransfer::{is_done, cancel, wait}`, and
`SPI2::regs()` status/enable/clear. The HAL's borrowing async future and
`into_async()` are unusable for this boundary (private wakers; the handler
would be replaced; per-poll construction loses the listener). TX-only; the
board's SH8601 path is TX-only, so that seal is acceptable and recorded.

| Candidate | Status | Evidence |
|---|---|---|
| A′ carrier + C completion | **selected**; host-model PASS with 13 tests: 12 positive traces (including starter rejection, adversarial registration race, cancel-wake, settlement-gated abort, waker replacement, late-IRQ inertness, reuse, and spare identity) plus the check-then-register lost-wake negative control | `src/transfer.rs`, `tests/k2r0a_a_prime.rs`, `probes/esp32s3-spi2/VERDICT.md` |
| A — kernel pin admission | not needed for this boundary | — |
| B — named task + channel boundary | not needed for this boundary | — |
| ∅ | not reached | — |

The generic `InFlight<X, S>` carrier implements `Unpin` exactly when
`X: OwnedTransfer + Unpin` and `S: Unpin` (no bound on `X::Transport` or
`X::Buffer`). The host-model types meet those bounds; the same assertions
for the concrete Xtensa wrapper and buffers remain part of the target
compile gate below.

## Reviewer corrections applied (2026-08-08)

All nine adopted: (1) register-then-recheck mandated; my model had the exact
lost-wake race and now carries an adversarial oracle plus a deliberately
broken negative control proving the oracle bites; (2) `cancel` wakes the
pending poller; (3) the spare buffer is carried in `InFlight` and returned
in `Settled`; (4) kernel-admitted source + real `reactor!` fixture remain
open (below); (5) recovery is the sole outcome authority, `poll_done` is
`Poll<()>`; (6) `is_draining` clears at settlement; (7) `Failed` documented
as abstract-boundary-only — the esp-hal adapter never produces it; (8)
waker-replacement/late-IRQ/reuse traces added; (9) sealing recorded below.

## Open before spec amendment / freeze

1. **Xtensa compile probe** (`probes/esp32s3-spi2/`): linked firmware for
   `xtensa-esp32s3-none-elf` per the verdict's checklist (real SPI2 + DMA
   channel + two `DmaTxBuf`s, monomorphized `half_duplex_write`, `Unpin`
   assertions, second-transfer reuse) — gated on espup approval.
2. **Board HIL**: silicon interrupt delivery (pending → one wake → ready;
   completion-before-first-poll level visibility; cancel-and-drain returns
   transport + sent buffer + spare).
3. **Kernel-admitted completion source + `reactor!` fixture**: raw-`Context`
   oracles do not prove delivery into Kittens; the kernel admission path for
   this profile's sources is the remaining kernel conversation.
4. **Seal `FlightStarter` and `OwnedTransfer`** to reviewed integrations before
   any freeze.

## Toolchain gate

ESP32-S3 is Xtensa LX7: target probes need the Espressif Rust toolchain
(`espup`). Until approved and installed, host-model + `thumbv7em-none-eabi`
portability checks stand in; no HAL-fidelity claim is made from them.

## Exit review round 1 (2026-08-08)

Full text: `reviews/2026-08-08-exit-review-1-codex.md`. Verdict: **FAIL**,
15 findings, must-fix 1–11 and 14–15; finding 12 accepted as must-fix by
the author as well; 13 advisory (adopting `NonZeroU8` regardless).
Disposition: **all fifteen accepted.** Fix plan, in order: (batch 1)
transfer/sweep/demand semantic redesign — cancel-settlement linearization
(F2), unforgeable `StripeWritten` witness wiring transfer→sweep (F4),
crate-owned `Sweep` with the fixed panel plan (F5), token provenance
branding + checked fallible finish (F6), invalidation terminating the
affected epoch without a wrapping counter (F7), `abandon_active` recovery
(F8), written-milestone renaming (F9); (batch 2) touch redesign — separate
pending latch with swap-based wake dedup + negative control (F10),
persistent retry latch across INT-only failure and wrap alias (F11), no
`Moved` on identical points (F12), `NonZeroU8` budget (F13), shared
done-slot model + drop traces (F1), superseded-blueprint note (F3);
(batch 3) spec §6 amendment, trace manifest, README/profile checklist,
changelog, seam/drop/no-std fixtures (F14, F15). Re-review follows.

## Exit-review batches landed (2026-08-08)

Batch 1 (author: session agent): witness-driven transfer/sweep/demand
composition — findings 2, 4–9, F1 model, F3 note. Batch 2 (**author: the
reviewing engineer, Codex gpt-5.6-sol ultra, workspace-write session**;
reviewed and independently verified by the session agent): the touch
pending-latch protocol — findings 10–13, including its own refinement of
claiming the latch before the read so an exact-2^32 during-read alias
cannot erase arrivals (tradeoff: at most one redundant wake). Batch 3
(author: session agent): SPEC revision 3 amendment, trace manifest,
profile README checklist, changelog, crate no_std CI gate. All gates
green: 41 oracles, fmt, clippy zero, thumbv7em-none-eabi release build.

## Exit review round 2 (2026-08-08)

Full text: `reviews/2026-08-08-exit-review-2-codex.md`. Verdict: **FAIL**.
Findings 1, 2, 7–13 verified ADDRESSED with evidence; 3–6, 14, 15 remain,
sharpened, plus six new findings from the fixes themselves. Disposition:
**all accepted.** Batch 4 (core, author: session agent): privatize the
proof chain — `Settled` fields private with a consuming single-use
witness mint; `StripeTarget` minted by the `Sweep` binds demand/epoch/
region into `InFlight::new` so targets cannot be claimed independently;
admitted panel geometry (anchor-board constant; arbitrary panels become a
named escape); demand IDs kept thumbv7em-compatible with an
exhaustion-checked `AtomicU32` and widened to `u64` in witnesses, plus a
documented 2^64-sweep epoch horizon; `Tick` regression clamping plus a
documented trusted-time boundary; `abandon_active` documented as
witness-terminal only (live transfers still physically write — caller
drains first) with a rejection oracle; missing request-during-sweep,
slow-successful-sweep/clamp, duplicate/replay, and unchanged-state rejection
oracles; liveness-critical `#[must_use]` annotations; SPEC §5.2/§6.2
sealing language reconciled; stale stage prose in lib.rs/geometry/Cargo.toml
fixed; board-HIL and sealing manifest rows added; probes README/CHANGELOG
attempted to distinguish the historical and corrected blueprints without
claiming either built; round 3 later found that the "corrected" file was only
pseudocode, and batch 6 records the honest relabel below. Batch 5
(author: the reviewing engineer): canonical runnable host lifecycle;
trybuild UI failures for private/move-only proof boundaries with compiling
escape-surface controls; a separate external no-std consumer/link fixture;
exact conditional-`Unpin`, citation, trace-count, snapshot, Tick, and stage
prose repairs; and a sticky demand-id exhaustion transition after review
found that panic-unwind could reopen the original wrapping counter. Round-3
review follows.

Batch 5 verification (2026-08-08): `cargo test -p kittens-render` passed
46 runtime oracles plus 15 compile-fail and 5 compile-pass UI controls,
first with `TRYBUILD=overwrite` and then clean; the canonical example ran
through all four anchor-panel stripes; package clippy passed with warnings
denied; rustfmt and rustdoc were clean; the library and separate downstream
consumer both linked for `thumbv7em-none-eabi --release`. The consumer ELF
is a statically linked ARM executable retaining `_start`,
`FrameDemand::new`, and the `kittens-render` demand-id state. The library's
normal target dependency tree remains empty. No Xtensa, board-HIL, kernel
admission, sealing, or bilateral-seam gate is claimed by this result.

## Exit review round 3 (2026-08-08)

Full text: `reviews/2026-08-08-exit-review-3-codex.md`. Verdict: **FAIL** —
externally gated rows are not the cause; six host-core findings are, and
the reviewer's sharpest observation is that our own external no-std
fixture was the counterexample for finding 1 (a preclassified transfer
minting coverage it never wrote). Disposition: **all six accepted**, plus
the three advisories. Batch 6 was delegated to the reviewing engineer with
the author's agreed shapes; its landed form is recorded below. Round 4
follows.

## Exit-review batch 6 landed (2026-08-08)

SPEC revision 4 records the implemented host contract. Public
`InFlight::new` is gone: the consuming `StripeTarget::start_flight` supplies
the exact target region to the starter, and `StartFlightError` returns the
starter error, untouched spare, and same target when no transfer was accepted.
That is structural identity/start coupling; whether the admitted adapter
really writes the supplied region remains the explicitly sealed-integration
obligation.

Resource extraction now returns a companion settlement:
`Settled::into_parts` returns transport, sent buffer, spare, and exactly one
move-only `StripeSettlement`. `Written(StripeWritten)` is the sole coverage
path; `Unwritten(StripeUnwritten)` preserves the real cancelled/failed outcome
and irreversibly poisons its owning sweep **when the matching owner accepts
it**. `Sweep::next_target(&mut self)` admits only one outstanding target per
plan position, and `Sweep::settle` alone clears it. Batch 8 later corrected the
overbroad delivery claim: Rust cannot force the caller to deliver rather than
drop or misapply that witness. A poisoned sweep cannot mint or finish; only
abort remains.

Abort is intentionally bookkeeping-terminal rather than physical revocation:
an outstanding transfer may still write after a replacement starts. Accepting
the abort retains a forced full repaint; draining closes the window when
possible, and `FrameDemand::invalidate()` prevents an overlapped replacement
from clearing the obligation. Epochs 0 through `u64::MAX` are minted once with
a sticky, profile-independent exhaustion boundary; throttle eligibility uses
checked addition and reports the finite `Tick::MAX` horizon rather than
saturating. Explicit clone failures cover the move-only proof carriers, and
the safe shared/interior-mutable backing alias between sent and spare buffers
is published as a compiling, documentation-enforced escape.

The evidence repair runs the canonical host lifecycle in CI rather than only
building its zero-test harness, adds the missing foreign `finish_failed` and
cross-demand stripe-settlement rejections, and asserts all observable state on
every rejection path. `TRACE-MANIFEST.md` now records those oracles, the
normative exact-SHA/no-allocation `write_region` gate, and an explicit adjacent
negative control or an honest reason none exists for each row. The matching-
settlement state machine is covered against the repaired lifecycle; batch 8
separates that result from cooperative owning-sweep delivery, which remains a
published documentation boundary.

The probe's `adapter-blueprint.rs` is now labeled accurately: it is a
non-compile-ready pseudocode delta over the retained `VERDICT.md`, not
shape-complete or compile-ready adapter source. The real pinned-SHA Xtensa
adapter/link gate, board HIL, kernel admission, sealing, and bilateral seam
remain open; batch 6 does not claim any of them closed.

## Exit review round 4 (2026-08-08)

Full text: `reviews/2026-08-08-exit-review-4-codex.md`. Verdict: **FAIL**;
findings 2, 4, 6 and two advisories ADDRESSED; 1 and 3 remain blocking,
5 partially. Disposition: accepted, with the author's resolutions for the
two blockers recorded here before delegation: (1) the closure starter can
never be sealed — replace it with a `FlightStarter` trait invoked BY the
crate with the target's region, marked seal-at-freeze like `OwnedTransfer`;
pairing becomes structural under sealed integrations, and SPEC/blueprint
prose states the experiment-phase boundary honestly instead of claiming
nonexistence; (3) `abort` requires settlement — `Err(sweep)` while a
target is outstanding — which has no liveness cost because cancel-and-
drain settles by contract; drop-cancellation is bounded by the adapter `Drop`
contract and remains one explicit non-returning drop-plus-abandon escape,
while the cooperative driven path delivers each recovered settlement to its
owning sweep. Round 5 later established that safe Rust cannot force that
delivery; dropped or wrong-owner-consumed settlements are additional escapes.
`invalidate`'s
idle-time timing hole closes by making the pending invalidation stick to
the next mint instead of being cleared by it. Batch 7 delegated.

## Exit-review batch 7 landed and verified (2026-08-08)

SPEC revision 5 is implemented in the host surface. The closure starter is
replaced by the operation-bound `FlightStarter` trait: the crate invokes
`start` with the target's region, and `FlightStarter` now shares
`OwnedTransfer`'s seal-at-freeze gate. Target/start pairing is therefore
structural under sealed, reviewed integrations. During the experiment both
traits remain deliberately open; a dishonest safe starter can ignore the
region, return an unrelated prestarted transfer, or start and then report
rejection. That integration-honesty boundary is published beside the
`TouchReader` untorn-snapshot obligation rather than claimed away.

`Sweep::abort` now returns the sweep unchanged while its target is outstanding
and succeeds only from ready or poisoned state. On the cooperative path an
accepted flight can begin drain, poll completion, recover the move-only
settlement, deliver it through the matching `Sweep::settle`, then abort, so the
restriction adds no liveness cost on that path. One explicit escape addressed
by batch 7 is drop plus `FrameDemand::abandon_active`: reviewed adapters must
synchronously cancel the physical operation and disarm completion registration
when a flight is dropped, while safe Rust cannot force a caller to drop an old
sweep rather than retain and drive it. Round 5 added dropped and wrong-owner-
consumed settlements to the published escape set.

Idle invalidation is now sticky. `invalidate()` records a pending latch that
only the next successful `begin_sweep` transfers into that minted epoch's
discard state; rejected, throttled, or panicking begin attempts cannot erase
it. Thus invalidation between abort/abandon and replacement cannot be lost.
The remaining finding-5 evidence repairs cover the private `from_started`
constructor, `StartFlightError`'s move-only boundary, and every observable
epoch/throttle state after demand-settlement rejection. Those repairs corrected
the immediate rejection-state evidence; round 5 found that the manifest's
owning-delivery completeness claim and unanchored future-throttle evidence were
still overbroad, and batch 8 corrects both.

Batch 7 verification passed: 59 runtime oracles plus 25 compile-fail and 7
compile-pass UI controls; trybuild first ran with `TRYBUILD=overwrite`, every
changed snapshot was read, and then passed clean. `cargo fmt --all --check`,
workspace/all-target/all-feature clippy with warnings denied, the full
workspace/all-feature test suite, and workspace rustdoc with warnings denied
all passed. The canonical example ran both a written frame and settled
shutdown; the downstream fixture ran on host; and both the profile library and
that external consumer built for `thumbv7em-none-eabi --release`. No external
Xtensa, board-HIL, kernel-admission, sealing, or bilateral-seam result is
claimed by those host/ARM gates.

## Exit review round 5 (2026-08-08)

Full text: `reviews/2026-08-08-exit-review-5-codex.md`. Verdict: **FAIL**;
batch-7 internals sound, but `FlightStarter::start` is publicly callable
(sealing restricts implementors, not callers), which also reopens sweep
accounting; plus two missing privacy/regression fixtures and
throttle-anchored rejection oracles. Disposition: **all five must-fixes
accepted**; for item 4 the narrow-and-publish arm is chosen — lost or
misapplied settlements and abandonment become explicitly published
escapes, since enforcing settlement delivery would require linear types
Rust does not have. Batch 8 (delegated): crate-issued unforgeable
StartPermit parameter on FlightStarter::start with direct-invocation and
raw-closure compile-fail controls; InFlight struct-literal privacy
fixture; rejection oracles re-run against an established throttle anchor
with exact future-eligibility and successor-epoch assertions; claims
narrowed in SPEC/manifest/log/CHANGELOG.

## Exit-review batch 8 landed and verified (2026-08-08)

SPEC revision 6 is implemented in the host surface. `FlightStarter::start`
now requires a crate-issued `StartPermit<'_>` as well as the target region.
The permit has a private constructor, is non-`Clone`, and borrows a dispatch-
local key; safe external code cannot mint it, invoke a starter without it, or
return the received permit in the starter's fixed associated error type. Only
`StripeTarget::start_flight` issues one. This closes direct safe dispatch while
leaving the separately published open-integration honesty boundary intact: the
permit cannot prove that an implementation uses the supplied region or reports
rejection atomically.

The UI suite now pins direct one-argument starter invocation, private permit
construction, permit cloning, lifetime escape through `Error`, the removed raw
closure start path, both the private `InFlight::from_started` constructor and
the four-field `InFlight { ... }` literal, and the pre-existing move-only proof
boundaries. All example, model, downstream fixture, and pseudocode-blueprint
implementations name the permit parameter.

All four external `FrameDemand` settlement-rejection oracles now begin after a
real successful epoch-0 write installs a non-`None` throttle anchor. The two
foreign tests preserve equal epoch numbers across demands; the stale/abandoned
tests advance through their replacement epochs. After each rejection, the
tests compare immediate state and then prove the exact original eligibility
instant plus successor epoch (2 for foreign, 3 for stale/abandoned), so a
conditional mutation of an existing throttle anchor no longer escapes.

The owning-sweep claim is deliberately narrower. `Settled::into_parts` returns
exactly one unforgeable, move-only, non-relabelable settlement, and matching
`Sweep::settle` acceptance is the sole progress/poison path, but delivery is a
cooperative caller contract. Ordinary drop or a consuming wrong-owner
rejection leaves the owner outstanding. Recovery is to drop all old values,
call `FrameDemand::abandon_active` (retaining demand and forcing full repaint),
then call idle `invalidate` before replacement when stale physical work or
external invalidation may overlap. `TRACE-MANIFEST.md` reports that delivery
row as documentation rather than complete static enforcement; the SPEC,
README, source docs, example, probe prose, and CHANGELOG publish the same
boundary.

Batch 8 verification passed: 59 runtime oracles, 31 compile-fail controls, and
7 compile-pass controls. Trybuild ran first with `TRYBUILD=overwrite`; every
new snapshot and the line-only change to the moved-target snapshot were read,
then the suite passed clean. Package tests and the full workspace/all-feature
test suite passed; `cargo fmt --all --check`, workspace/all-target/all-feature
clippy with warnings denied, and workspace rustdoc with warnings denied were
clean. The canonical host example completed its written frame and settled
shutdown. Both `kittens-render` and the downstream
`kittens-render-no-std-fixture` built for `thumbv7em-none-eabi --release`. No
Xtensa, board-HIL, kernel-admission, capability-sealing, or bilateral-seam gate
is claimed by these host/ARM results.

## Exit review round 6 (2026-08-09): PASS

Full text: `reviews/2026-08-09-exit-review-6-codex.md`. **Verdict: PASS.**
All five round-5 must-fixes verified ADDRESSED with evidence; no batch-8
regressions; "the slice is ready for the branch PR." Three non-blocking
advisories were issued and all three applied before the PR: the
`StartPermit` struct-literal privacy pin (new E0451 fixture), the
revision-number/manifest-legend/attribution drift repairs, and the
explicit drop-the-old-Sweep guidance on `abandon_active` plus the SPEC
section-10 clarification that the seam gates full K2R-0 acceptance, not
this host slice. Loop exit condition met: the codebase is done for the
host slice, the reviewer passed it, and the author agrees with every
outstanding proposal. Open beyond this slice, unchanged and honestly
labeled: pixel equivalence (draw-target slice), bilateral seam co-sign,
Xtensa probe (espup gate), board HIL (hardware in transit), kernel
source admission, capability sealing at freeze, and the write_region
transport gate.
