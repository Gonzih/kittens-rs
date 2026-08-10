# Revision-11 async-region implementation review

- Date: 2026-08-09
- Reviewer: Claude Code 2.1.224, `claude-opus-4-8`, maximum effort
- Mode: read-only adversarial pre-commit review
- Scope: the complete diff and every untracked revision-11 implementation,
  oracle, target control, manifest/lockfile, workflow, fixture, and evidence
  document against `AGENTS.md`, SPEC sections 6.8, 8, 9, and 11, and exact
  `esp-hal` revision `d48f747ba28accdc51779ba193eba923138e0382`

## Final verdict

**SOUND — zero P0–P2 defects.** The reviewer independently traced the exact
HAL APIs, host transition core, target shell, generated-reactor fixture, and
CI enforcement. The only P3 note was the intentionally stale “evidence
pending” wording at review time, awaiting the parent-owned final run and ledger
update recorded in the implementation commit.

## Verified implementation points

- The owning SPI DMA start, completion, cancel, wait, and drop spellings match
  the pinned HAL. `SpiDma::is_done` observes the driver's busy state, so the
  reviewed `spi2_done_raw() || transfer.is_done()` recheck remains a completion
  backstop after the interrupt handler clears the transfer-done flag.
- The SPI2 interrupt enable/raw/clear registers, handler priority, exact
  SPI2/DMA_CH0/GPIO singleton construction, DMA buffers, and critical-section
  implementation all resolve at the locked source revision.
- The concrete `Waker` slot clones before exclusion and performs every replaced
  or completed waker drop/wake after exclusion. Register-then-recheck, inactive
  interrupt acknowledgement, completion-versus-cancel linearization, ordinary
  drop, and start-error recovery are conservative and resource-carrying.
- Async preflight follows the normative precedence exactly: region-derived
  payload cap before logical descriptor length. CASET/PASET encoding is shared
  with the blocking engine rather than duplicated, and RAMWR rejection is
  acceptance-atomic.
- The SPI3, DMA_CH1, and swapped-SIO controls reach their intended E0308
  diagnostics without masking; the exact Parts constructor/extractor control
  compiles. Dishonest generic `FlightStarter` implementations and raw HAL use
  remain explicit compiling non-controls.
- Host coverage exercises the concrete `CompletionSlotCore<Waker>` transition
  body used by the target. The Xtensa shell is honestly target-Clippy/link/symbol
  evidence rather than host execution evidence; CI includes a direct exact-
  target `kittens-render --lib` Clippy gate in addition to fixture Clippy.
- The retained generated-reactor and armed-source-drop hooks, one opaque noop-
  waker poll, allocator/undefined-symbol scans, and exact source-identity check
  match the contract. They do not claim an executor, IRQ delivery, drop
  execution, arbitrary-`RawWaker` allocation behavior, or silicon truth.

## Remaining gaps

Target-side executor scheduling, silicon interrupt/wake/cancel behavior,
physical panel initialization and command acceptance, placement/color/TE,
latency, generic capability sealing, async RAMWRC, registry publication, and
the bilateral seam remain separately named gates.

Final reviewer line: `VERDICT: SOUND`.
