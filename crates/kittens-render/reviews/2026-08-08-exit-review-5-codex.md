# Verdict: FAIL

The manifest’s OPEN/GATED rows were excluded as instructed. They do not cause this verdict.

- **Finding 1 — NOT ADDRESSED.** The old callback was removed, but `FlightStarter::start` remains publicly callable with only a public `Region` ([transfer.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:65)). Sealing restricts implementations, not callers. A caller can start and drive an `OwnedTransfer` without consuming a `StripeTarget`, contradicting the recorded “invoked by the crate” design ([K2R0A-LOG.md](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/K2R0A-LOG.md:216)). This bypass survives sealing.

- **Finding 3 — NOT ADDRESSED overall.** The internal repair is sound: `next_target` records `Outstanding`, settlement clears it, abort rejects it, and idle invalidation is sticky ([sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/sweep.rs:340)). But direct `FlightStarter::start` bypasses that state entirely: a sweep remains `Ready`, can abort and authorize replacement, while the directly started transfer remains live. Additionally, the categorical “every settlement reconciles through its owning sweep” claim is false: an existing test consumes a left settlement against the right sweep, then abandons the still-outstanding left sweep ([k2r0_demand_sweep.rs](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0_demand_sweep.rs:762)).

- **Finding 5 — NOT ADDRESSED.** Round 4 explicitly required both constructor and struct-literal privacy controls ([review 4](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/reviews/2026-08-08-exit-review-4-codex.md:73)); only `from_started` is tested ([UI fixture](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/ui/in_flight_from_started_is_not_public.rs:4)). There is also no compile-fail control pinning rejection of the removed raw closure API. Finally, every future-throttle rejection check starts with no prior successful write ([test helper](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0_demand_sweep.rs:46)), so a rejection that conditionally moves an existing throttle anchor would pass while violating the manifest’s unchanged-future-state claim ([manifest](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/TRACE-MANIFEST.md:24)).

Must fix:

1. Make starter invocation require an unforgeable crate-issued permit/private dispatch; sealing alone is insufficient. Add compile-fail controls for direct invocation and raw closures.
2. Add the required `InFlight { ... }` struct-literal privacy fixture and snapshot.
3. Exercise every demand-rejection path after a successful write established a non-`None` throttle anchor, proving exact future eligibility and successor epoch.
4. Either enforce owning-sweep reconciliation or narrow SPEC/source/manifest claims and explicitly publish lost or misapplied settlement plus abandonment as an escape.
5. Correct the completeness claims in the manifest, K2R0A log, and CHANGELOG.

Formatting, package/workspace tests, all 59 runtime tests, 25 compile-fail and 7 compile-pass controls, the host example, and the host fixture passed. No other batch-7 functional regression was found.
