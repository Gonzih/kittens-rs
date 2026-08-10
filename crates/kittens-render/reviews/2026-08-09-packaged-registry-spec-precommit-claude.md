# Revision-12 package/staging spec adversarial review

- Date: 2026-08-09
- Reviewer: Claude Code 2.1.224, `claude-opus-4-8`, maximum effort
- Scope: uncommitted spec-first revision-12 documentation; read-only review of
  the acceptance map and selected local packaged-source/registry-HAL Xtensa
  consumer gate
- Reviewed base: `6681a3ac4ad4d99ed62bc9356f97fe528b27db38`

## Final verdict

**SOUND — zero P0–P2.** Revision 12 is ready for its spec-first commit. The
package compatibility row is correctly **OPEN — CONTRACT SELECTED, EVIDENCE
PENDING**; the fixture and CI job do not yet exist and no passing claim is
made.

The reviewer read `AGENTS.md`, the complete diff, the full render SPEC and
status/evidence documents, the workspace and crate manifests/lock, the Xtensa
workflow and existing exact-git fixture, and the historical 0.1.1 publication
record. A focused final pass re-read every status/provenance correction after
the repository-native reviews' P2 wording findings were repaired.

## Verified contract points

- K2R-0A is consistently closed only with host + portable-link + exact-Xtensa-
  link scope. The bilateral seam and generic async capability seals gate the
  K2R-0 protocol freeze without reopening K2R-0A. Target execution,
  coordination, HIL, and measurements belong to K2R-1.
- The locally generated `kittens-render-0.1.1.crate` is honestly distinguished
  from crates.io 0.1.1, which is immutable older source. The local filename
  reflects the current workspace declaration only; it is not a republish
  candidate. Any future release remains correctly versioned and human-ordered.
- The selected package gate is emittable: Cargo 1.96 packaging can produce a
  clean archive whose `.cargo_vcs_info.json` records exact HEAD and
  `path_in_vcs`, with `git.dirty` absent or false. The normalized active
  manifest exposes registry `esp-hal =1.1.0`; `Cargo.toml.orig` is deliberately
  not the inspected surface.
- The standalone consumer's fixed relative path can be preserved by extracting
  the archive and copying the fixture into a matching temporary layout outside
  the checkout. No manifest rewrite, source patch, or git HAL dependency is
  admitted.
- Both direct packaged-library and consumer Xtensa Clippy runs are required to
  execute from the staged fixture working directory, so its committed
  `build-std = ["core"]` and target linker configuration govern both. The
  packaged-library run names its extracted manifest with `--manifest-path`.
- The locked consumer graph must contain exactly one registry HAL identity and
  no git source. Direct registry singleton values cross the packaged public
  constructor, and the retained uncalled start hook reaches the target-owned
  `start_flight` spelling.
- The separate job retains the exact-git fixture as the audited-SHA control and
  adds structural provenance/metadata checks, both target Clippy gates, the
  optimized link, empty undefined table, allocator-symbol absence, and a
  nonzero retained-hook symbol.
- Host normalization, a dirty archive, copied source, `[patch]`, the exact-git
  fixture, and uncalled linked code are explicit non-controls. The gate cannot
  prove publication/index/download, target execution, arbitrary-waker
  allocation, interrupts, cancellation/drop runtime behavior, or silicon.

## Nonblocking implementation watch items

- Make the fixture actually reach the `rt`-dependent interrupt/start path so a
  registry-versus-git feature-shape divergence cannot pass silently.
- Implement the dirty check as absent-or-false, while failing closed on an
  emitted `git.dirty: true`.
- Preserve the release guard: substantial current public API still carries the
  already-published workspace version 0.1.1, so any future human-authorized
  publication must first use an appropriate new version.

## Verdict

`VERDICT: SOUND`
