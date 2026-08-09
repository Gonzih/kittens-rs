# kittens-code-core

The sans-io coding-agent engine for the
[kittens-code](../../docs/kittens-code/SPEC.md) family: a synchronous
`handle(CoreInput) -> Transition` state machine that owns the turn loop,
context engine, RLM query engine, and budget law. `#![no_std]` + `alloc`, no
runtime, no clock, no entropy, no IO — every side effect leaves as an `Effect`
a driver discharges.

Controlling contract: [`docs/kittens-code/SPEC.md`](../../docs/kittens-code/SPEC.md)
sections 6–9 (frozen KC0).

## The bet

The agent's transcript is an append-only log the model queries in-loop through
a small Unix-flavored verb surface (`grep`/`slice`/`head`/`tail`/`count`/
`partition`/`ask`/`final`), while the live window is compacted continuously.
The RLM continuation executor runs those queries as suspendable, budgeted
computations; the turn engine drives them via effects.

## Enforcement layers

| Guarantee | Mechanism |
|---|---|
| budget caps on model-visible data | sealed `Capped<K>` types (`caps`) — truncating constructors only, bypass is a compile error (gate G3) |
| tail atomicity in the window | `WindowLayout` constructor enforces one-to-one call/terminal lifecycle |
| exactly-once effect completion | the turn engine's ownership + epoch ledger; late/unowned completions drop with a trace |
| deterministic replay | integer-only behavioral math; append-only records; resume reconstructs state |
| bounded RLM queries | the full Q5 meter set charged as the executor runs |

## Portability

Proven to link on `thumbv7em-none-eabi` and `wasm32-unknown-unknown` (gate
G1). The `l2`, `swarm-port`, and `exec` cargo features are removable and off
by default.

## Status

Experimental `0.x` evidence release. Depends on `kittens-code-protocol`.

Licensed under Apache-2.0 or MIT, at your option.
