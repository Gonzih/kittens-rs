# Packaged registry-HAL implementation adversarial review

- Date: 2026-08-09
- Reviewer: Claude Code, `claude-opus-4-8`, maximum effort
- ATC dispatch:
  `prompt-1786337614356-16109-0@implement@1786337614356-4a35`
- Scope: uncommitted implementation of revision 12's local clean packaged-
  source + registry-HAL Xtensa consumer gate; read-only review
- Reviewed base: `10c31b7f0debcb9f5cb65bff5fb27ca5f45a82b6`

## Final verdict

**SOUND — zero P0–P2.** The standalone fixture, recurring CI job, and
documentation propagation are ready for the implementation commit. The
review makes no publication, target-runtime, or board-HIL claim.

The reviewer read `AGENTS.md`, the complete render SPEC revision 12, the full
implementation diff, the normalized package output, both fixture lockfiles,
the existing exact-git job, and the generated metadata/provenance surfaces.
It independently checked the parsed YAML scalar rather than treating raw YAML
indentation as shell input.

## Verified implementation points

- A clean package run must begin without a pre-existing or tracked archive or
  extracted directory. The generated `.cargo_vcs_info.json` must identify
  exact HEAD and `crates/kittens-render`, with no truthy dirty marker.
- The generated active manifest exposes registry `esp-hal =1.1.0` without a
  git source. Locked package and consumer graphs require the exact registry
  checksum, contain no git packages, and are hashed before and after Cargo
  inspection and target builds.
- The standalone fixture has an explicit empty `[workspace]`, no patch or
  replacement table, and an exact hashed and parsed Xtensa configuration. Its
  direct dependencies are the extracted path-and-version package and registry
  `esp-hal =1.1.0`; it has no direct `kittens` or `critical-section` edge.
- `linked_packaged_registry_parts` accepts direct registry-HAL SPI2, DMA_CH0,
  GPIO4–7/11/12, and DMA-buffer types and returns the packaged profile's
  branded parts. The optimized ELF must retain exactly one nonzero text symbol
  for that constructor boundary.
- The uncalled `linked_packaged_registry_start` hook crosses `try_new`, the
  target-owned starter, and `StripeTarget::start_flight`; CI requires its exact
  symbol and a nontrivial linked size. Both owning result paths are retained.
- The staged job runs direct packaged-library and consumer target Clippy from
  the fixture configuration, performs a locked fat-LTO Xtensa link, requires a
  static executable with an empty undefined table, and applies the established
  allocator-symbol scan.
- Ruby/Psych parsing places the quoted Python heredoc at column zero, and the
  validator's arguments, source-shape checks, lock/config checks, and metadata
  assertions match the committed fixture.
- The exact-git fixture remains a non-control for registry source identity.
  The retained start hook is not executed, crates.io 0.1.1 remains immutable
  older source, and publication, executor behavior, interrupts, cancellation,
  drop, allocation under arbitrary wakers, and hardware behavior stay outside
  this gate.

## Verdict

`VERDICT: SOUND`
