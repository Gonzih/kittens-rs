# kittens-code-protocol

The wire contract for the [kittens-code](../../docs/kittens-code/SPEC.md)
coding-agent harness family: the only crate a frontend or external client
links. Pure `serde` data, `#![no_std]` + `alloc`.

Controlling contract: [`docs/kittens-code/SPEC.md`](../../docs/kittens-code/SPEC.md)
section 4 (frozen KC0).

## What it contains

| Module | Type | Role |
|---|---|---|
| `op` | `Op`, `Submission` | client → driver requests (`resume` is a startup mode, not an op) |
| `event` | `Event` | driver → client events, with preview/authoritative model deltas |
| `error` | `ErrorEvent`, `ErrorCode` | the closed KC0 error taxonomy; class is data, not caller judgment |
| `policy` | `SandboxPolicy`, `ApprovalPolicy` | approval/sandbox policy as data (mechanism lives in drivers) |
| `config` | `SessionConfig`, `SessionConfigPatch` | the patchable, logged, replayable session configuration |
| `budgets` | `Budgets`, `BudgetKind` | the Q5 budget set as declared numbers (enforcement is core law) |
| `ids` | `SessionId`, `EffectId`, `TurnEpoch`, … | plain-array/integer identities (no `uuid`/`semver` deps) |

## Deliberate absences

No engine types, no `WindowLayout` or cap-types (those are
`kittens-code-core` law), no bootstrap configuration (endpoints/auth/TLS/store
paths are driver-only and never logged), no `uuid`/`semver`/checksum
dependencies. Wire evolution is additive-only within `0.x`; every enum is
`#[non_exhaustive]`.

## Status

Experimental `0.x` evidence release from the frozen KC0 slice. The wire shapes
are the frozen contract but the harness is early; see the SPEC for scoped
deferrals.

Licensed under Apache-2.0 or MIT, at your option.
