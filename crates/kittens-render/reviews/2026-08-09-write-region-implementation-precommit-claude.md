# Revision-10 blocking-region implementation review

- Date: 2026-08-09
- Reviewer: Claude Code 2.1.224, `claude-opus-4-8`, maximum effort
- Mode: read-only adversarial pre-commit review
- Scope: the complete diff and every untracked revision-10 implementation,
  oracle, UI snapshot, manifest/lockfile, workflow, target fixture, and evidence
  document against `AGENTS.md` and SPEC sections 6.7, 8, 9, and 11

## Initial verdict: NOT SOUND

The reviewer traced and accepted the core implementation:

- sealed `BlockingRegionWrite` admission and operation-bound permit lifetime;
- exact preflight precedence and first-row inclusive-window behavior;
- literal CASET/PASET/RAMWR/RAMWRC envelopes, patterned payload slicing, all
  eight failure boundaries, and stop-on-error prefixes;
- `BlockingSettled` resource/provenance retention, success/failure witness
  minting, and owning-sweep advance/poison/abort paths;
- the pinned HAL source identity and the required capacity-only TX descriptor
  normalization;
- the retained no-allocator Xtensa entry path and the package-source boundary;
- every new trybuild failure reaching its intended, unmasked diagnostic.

One P2 remained in the evidence checker. The CI allocator regex detected the
standard `__rust_*`, libc, and `alloc::*` spellings but missed realistic
ESP/global-allocator symbols such as `esp_alloc::`, `__rdl_alloc`,
`__rg_alloc`, and `<... as GlobalAlloc>::alloc`. The current ELF was clean, but
the checker enforced less than the specification claimed against a plausible
future regression.

The reviewer also recorded non-gating P3 observations:

- the concrete wire-symbol grep is a fail-closed retention tripwire whose local
  symbol can disappear under a different LTO/toolchain decision;
- the original allocator regex could falsely match an inert nested `Vec` type;
- empty `nm -u` is intentionally stricter than rejecting only strong undefined
  symbols;
- the 16,380-byte rationale needed to distinguish the 4,095-byte hardware
  maximum from the pinned HAL's 4,092-byte default descriptor chunk;
- two evidence passages used runtime verbs for a linked but unexecuted image;
- the 204,700-byte first run and 204,292-byte later freshness rebuild needed an
  explicit chronology distinction.

## Disposition

All P2 and practical P3 findings were adopted before the follow-up review:

1. CI now filters callable `T/t/W/w` symbols and rejects:
   `__rust_{alloc,dealloc,realloc,alloc_zeroed}`;
   `__rdl_*` and `__rg_*` allocation shims; exact libc allocation entry points;
   top-level `esp_alloc` and `alloc::{alloc,vec,raw_vec}` code; and
   `GlobalAlloc` allocation methods.
2. The filter was exercised against the real ELF and synthetic symbols. It
   detected `esp_alloc`, `__rdl_alloc`, and a custom `GlobalAlloc` method while
   ignoring `core::ptr::drop_in_place<alloc::vec::Vec<_>>`.
3. The Xtensa CI job now runs target Clippy with warnings denied before the
   locked release link.
4. Workflow comments distinguish the pinned retention tripwire from the
   source-level reachability argument and state that the undefined-symbol check
   deliberately fails closed.
5. The descriptor rationale now records five 4,092-byte default HAL
   descriptors for the reserve while retaining 4,095 as the hardware maximum.
6. README/log evidence says the entry path is linked and unexecuted.
7. Deployment evidence distinguishes the original and later revision-9
   artifact chronology.
8. The SPEC records the implementation-discovered descriptor normalization
   requirement explicitly.

## Follow-up verdict: SOUND

The same Claude session re-read and executed the exact remediated checker.
It reported:

- all 20 realistic allocator-entry test spellings detected;
- all eight inert/non-allocator controls ignored;
- zero allocator matches and zero undefined symbols in the real ELF;
- every defined symbol's type in the second `nm` field, with only non-callable
  linker-section `?` symbols excluded by the `T/t/W/w` filter;
- YAML and shell quoting preserved the exact extended regular expression;
- target Clippy, descriptor wording, linked-versus-executed wording,
  retention/undefined comments, chronology, and SPEC drift record all correct;
- no implementation regression and all local render tests, Clippy, fmt, and
  rustdoc checks green.

Final reviewer verdict: **SOUND — zero unresolved P0–P2 findings.**

One residual P3 remains accepted: the concrete wire-symbol grep is deliberately
fail-closed under the pinned toolchain and may require maintenance if a future
authorized toolchain change inlines that local symbol. It cannot create false
positive evidence.
