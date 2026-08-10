# Blocking-region revision-10 spec adversarial review

- Date: 2026-08-09
- Reviewer: Claude Code, `claude-opus-4-8`, maximum effort
- Scope: uncommitted spec-first revision-10 documentation; read-only static
  review, with repository verification run separately

## Final verdict

**SOUND — 0 P0–P2 issues.** The final current diff is ready for its spec-first
commit; implementation evidence remains correctly OPEN pending the named host
matrix and exact-HAL Xtensa link.

The reviewer read the repository law, root kernel contract and open-gate
report, the complete render contract and evidence records, the full diff, and
the locally cached `esp-hal` 1.1.0 and `sh8601-rs` 0.1.8 sources. It independently
re-derived the 82,432-byte reference transaction, all six pixel chunks, both
window payloads, and the eight failure boundaries.

## Findings adopted

- The contract now gives complete module paths, public declarations, private
  provenance storage, constructor/teardown signatures, error payloads, and the
  exact validation precedence. The unreachable byte-count-overflow oracle was
  removed; admitted geometry bounds the `u32` count to 329,728 bytes.
- `BlockingSettled` retains the consumed target as its sole witness-provenance
  source, can report only `Completed` or `Failed`, and publishes ordinary drop
  as a compiling resource/proof escape with full-repaint recovery.
- Structural sealing rejects external success reporters; the separate private
  permit prevents admitted dispatch outside the consuming target operation.
  The async capabilities remain independently open until their integrations
  migrate.
- The 16,380-byte compatibility chunk is derived as four 4,095-byte maximum
  TX-descriptor payloads and remains below the pinned HAL's 32,736-byte SPI DMA
  ceiling. Symmetric RX/TX scratch is explicitly a profile admission and memory
  budget policy, not a TX-only HAL requirement.
- The existing owning-DMA probe is explicitly a non-control for the new
  blocking `SpiDmaBus`/`split` path. Crate-wide `forbid(unsafe_code)` remains
  load-bearing for the new adapter.
- The exact target-gated Cargo stanza uses Cargo's documented multiple-location
  form: the git revision locally, checked against `=1.1.0`, and the registry
  version only as a publication fallback. Git- and registry-source HAL types
  are not conflated; published target consumption remains a later publication
  gate.
- The constructor's unbranded SPI/GDMA/pin/mode/frequency configuration is a
  caller obligation and compiling control, not something sealing proves.
- Ordinary implementation work is `OPEN`, not hardware-`GATED`; the feature
  name is narrowly `esp32s3-sh8601-blocking`; README and research boundaries
  now say the crate owns only the minimal region transaction, not a complete
  display driver.
- The final focused correction states exclusive ends are invalid only when
  `x + width > 368` or `y + height > 448`; equality is valid. That makes the
  predicate agree with the full-width reference trace and bottom/right
  boundary controls.

## Reviewer correction

The first review incorrectly classified Cargo `version` + `git` as
unpublishable. The follow-up retracted that finding after checking Cargo's
[multiple-locations rule](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations),
which exists specifically to use git/path locally and a registry version on
publication. The pinned checkout's package version is 1.1.0 in the existing
fixture lockfile, satisfying Cargo's compatibility check.

## Evidence scope retained

The reviewer confirmed that sealed admission plus deterministic implementation
evidence may mint `StripeWritten` at the documented HAL-return boundary without
claiming panel acceptance or visible output. Board initialization, physical
placement and RGB565 fidelity, RAMWRC behavior on silicon, TE/tearing, latency,
and presentation remain HIL-only. The blocking row does not close target-side
reactor execution, the bilateral seam, or `FlightStarter`/`OwnedTransfer`
sealing.
