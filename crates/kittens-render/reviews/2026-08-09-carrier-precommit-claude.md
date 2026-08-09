# Carrier implementation adversarial review

- Date: 2026-08-09
- Reviewer: Claude Code, `claude-opus-4-8`, maximum effort
- Comparison: uncommitted implementation against spec-first commit `f51600c`
- Mode: read-only static review; repository verification ran separately after
  this verdict

## Verdict

**SOUND — ISSUES: 0 P0–P2.** No required fix.

The reviewer read `AGENTS.md`, root SPEC sections 37 and 38, `K0-REPORT.md`,
the full render SPEC, every changed/new file, and the generated arbitration
loop. It independently traced the four real-reactor tests and found each
required position non-vacuous.

## Load-bearing findings

- `OptionalInlineOneShot<F>` exposes exactly the contracted five methods,
  safely pins only `F: Unpin`, becomes dormant before yielding owned output,
  rejects replacement through `arm`, does not self-wake while dormant, and
  implements neither drain nor backlog.
- `InFlight`'s conditional `Future` implementation delegates to the existing
  `poll_complete` authority under the exact bounds required by `Pin::get_mut`;
  it creates no second settlement path.
- The real-reactor traces genuinely cover poll-then-loss, an earlier winner
  before completion's first poll, same-carrier second-stripe rearm, graceful
  drain through a registered reactor waker, and post-exit carrier drop/disarm.
- The compile-pass inert future pins both arbitrary-inner-future dishonesty and
  the `future_mut`/`mem::replace` escape. The two compile-fail controls
  independently pin `Unpin` and sealed readiness without masking one another.
- `kittens` and Tokio remain dev-only dependencies of `kittens-render`; the
  downstream fixture uses the no-default kernel plus host-side macros and
  remains a real no-alloc consumer on Thumb and wasm.
- Documentation consistently closes only the host + portable-link reactor
  gate. The still-manual Xtensa fixture is explicitly a non-control for
  target-side reactor execution. Formal K0 gates, board HIL/silicon,
  `write_region`, bilateral seam, and capability sealing remain open.

## Non-blocking notes and disposition

1. **P3 verification hygiene:** run fmt, clippy, tests, trybuild, and inspect
   every regenerated stderr. **Accepted:** these are mandatory post-review
   gates and are recorded in the implementation handoff.
2. **P3 optional symmetry:** same-carrier rearm is paired with the
   poll-then-loss trace rather than repeated in the earlier-winner trace. The
   contract requires both loss positions and one rearm proof, so no change was
   made; the external two-stripe fixture independently exercises rearm.
3. **P3 cosmetic wrapping:** one root README line was long. **Accepted:** it
   was reflowed before commit.

The reviewer could not invoke Cargo under its read-only tool allowlist. That
constraint is not treated as test evidence; all dynamic gates run separately.

## Post-verification follow-up

After the dynamic evidence was recorded, the same reviewer re-read the current
diff and returned **VERDICT UNCHANGED, no P0–P2 issue**. It confirmed the fresh
Xtensa metadata, Thumb/wasm artifact facts, scope language, and this retained
review were internally consistent. Its sole new P3 note was that the render
SPEC status header still described the carrier evidence as pending; that
wording lag was corrected before commit.
