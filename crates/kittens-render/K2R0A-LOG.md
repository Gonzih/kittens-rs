# K2R-0A experiment log

Per SPEC section 7. This log is the experiment record; the spec is amended
only from what is demonstrated here.

## Candidate status

| Candidate | Status | Evidence |
|---|---|---|
| A′ — outer-`Unpin` adapter over waker-registering `poll_done` boundary | **host-model PASS** (2026-08-08): all six trace oracles green — both selection-loss positions, cancel-and-drain full recovery, drain-vs-completion race, failure settlement, spent-slot inertness, zero self-wakes | `src/transfer.rs`, `tests/k2r0a_a_prime.rs` |
| A — kernel pin admission | not started; only needed if A′ fails the HAL-fidelity check | — |
| B — named task + channel boundary | not started; last per selection rule | — |
| C — interrupt-backed transfer state | design reserve: if the real HAL cannot express `poll_done` without the borrowing future, C *becomes* the implementation of the A′ boundary (an interrupt-registered waker + `is_done` check is exactly `poll_done`) | — |
| ∅ | not reached | — |

## The load-bearing open question

A′ replaces the unnameable borrowing completion future with a
`poll_done(&mut self, cx)` boundary the integration must implement. On the
host model this is trivially honest. On real esp-hal it requires either:

1. `is_done()`-style state checks plus a **transfer-done interrupt that wakes
   a registered waker** (candidate C folded in as A′'s implementation), or
2. some HAL API that registers a waker without constructing/dropping the
   borrowing `wait_for_done` future per poll (constructing it per poll is the
   rejected lost-listener/busy pattern).

External contribution requested from the reviewer (Codex) on exactly this
question against esp-hal 1.1 documentation; result recorded here when it
lands. The exact-target compile probe remains gated on the Xtensa toolchain
(user approval pending).

## Toolchain gate

ESP32-S3 is Xtensa LX7: target probes need the Espressif Rust toolchain
(`espup`). Until approved and installed, host-model + `thumbv7em-none-eabi`
portability checks stand in; no HAL-fidelity claim is made from them.
