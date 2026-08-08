# Kittens K0 implementation report

Date: 2026-08-07

## Decision

The implemented candidate is **architecture B: direct lexical core polling with
an owned event enum**. The direct Tokio/event expansion and both tag-plus-slot
forms remain doc-hidden test controls. The public macro is expression-position
lean K0 syntax.

This is a strong implementation candidate, but **formal K0 closure is not
claimed**. Three registered gates remain open:

1. Tokio 1.53 exposes only `has_budget_remaining()`, while section 37.7 asks for
   the numeric remaining cooperative budget at every boundary. The retained
   tests compare the available boolean state and force an exhaustion boundary,
   but do not satisfy that literal numeric requirement.
2. The blind agent ablation and context-reset rehydration trials in section
   37.11 have not run. Diagnostic prose and the lean grammar therefore remain
   provisional.
3. The repeated, pinned optimized text/poll-cost comparison and the mandated
   three-way source-diagnostic mechanism experiment have not run. The core
   future-size delta described below is itself an architecture-review trigger.

Publishing the source repository is compatible with this result. Publishing a
stable crates.io API is a separate decision and is not performed by this
implementation run.

## Implemented boundary

- `kittens-macros`: parser, topology validator, core/event expansion, and three
  retained comparison expansions.
- `kittens`: `#![no_std]` facade, `Control`, sealed persistent source contract,
  readiness markers, drain/backlog traits, allocation-free latch/fixed queue,
  and feature/target-gated Tokio adapters.
- Tokio adapters: bounded/unbounded mpsc, optional mpsc, static dormant or
  emit-once close policy, cancellation, absolute optional deadline, retained
  one-shot, and optional retained one-shot.
- No scope, resource, capability, rendering, simulation, Embassy/HAL, unchecked
  source escape, hidden executor, or runtime scheduler was added.

Adapter-owned `Box::pin` storage is explicit for Tokio `Sleep`, cancellation
waiters, and arbitrary retained one-shot futures. Mpsc and the no-std sources do
not add adapter allocations. Expansion inspection found no arbitration box or
other generated allocation.

## Expansion and scaling evidence

Measurements used `rustc 1.96.0 (ac68faa20 2026-05-25)` and
`cargo-expand 1.0.124` on Apple Silicon. The checked excerpt and reproduction
commands are in `docs/expansion-snapshot.md`.

| Selected core/event expansion | Arms | Lines | Words | Bytes | Package-clean incremental check, median of 3 |
|---|---:|---:|---:|---:|---:|
| minimal example | 2 | 126 | 251 | 6,225 | 0.15 s |
| Grok comparison target | 23 | 1,212 | 2,222 | 67,140 | 0.52 s |

Arm count grows 11.5x; expansion lines, words, and bytes grow 9.6x, 8.9x,
and 10.8x. No `recursion_limit` or `type_length_limit` is present. The compile
time is useful only as a local exploratory ceiling: the Grok target also
compiles the raw oracle, presenter scenario, and all four expansion forms, so
0.52/0.15 is not a clean per-mechanism regression number. Dependencies were
warm and `cargo clean -p kittens` preceded every sample; the runner was not a
pinned performance host.

### Generated storage

| Fixture | Core/event | Core/slots | Tokio/event | Tokio/slots |
|---|---:|---:|---:|---:|
| two-arm scripted equivalence future | 176 B | 184 B | 176 B | 176 B |
| 23-arm Grok future | 432 B | 656 B | 256 B | 288 B |

The 23-arm owned event enum itself is 2 bytes for this fixture, as reported by
rustc's diagnostic `-Zprint-type-sizes` output. Event transfer saves 224 bytes
(34.1%) against core slots and 32 bytes (11.1%) against Tokio slots, so the
event form wins the transfer-axis comparison.

Direct core/event is 176 bytes (68.8%) larger than the equivalent generated
Tokio/event future. That exceeds section 37.7's 20% review threshold even
though both absolute sizes are small. Executor-neutral, no-alloc arbitration is
the additional semantic currently paying for the delta; optimized text and
poll-cost measurements are still required before freezing this choice.

The host embedded-shape future is 376 bytes against its predeclared 16 KiB
model budget.

## Bare-metal and feature-unification gate

The no-std fixture and a second workspace package that enables `kittens/tokio`
were built together for `thumbv7em-none-eabi --release`. Target-specific Cargo
inspection excludes Tokio and tokio-util from the bare-metal dependency path.

| ELF | `.text` | `.rodata` | writable static data | undefined symbols |
|---|---:|---:|---:|---:|
| spin-loop baseline | 14 B | 0 B | 0 B | 0 |
| polled Kittens kernel path | 2,496 B | 304 B | 0 B | 0 |
| absolute delta | +2,482 B | +304 B | 0 B | 0 |

The fixture deliberately polls the generated future once from `_start`; an
earlier dead-stripped version was rejected as non-evidence. The linked kernel
is below the predeclared 32 KiB text and 2 KiB writable-static budgets, uses no
allocator, and retains the single-facade package design under feature
unification. This is a portable-core link result, not a hardware/ESP32 claim.

The kernel and all-feature facade also compile on Rust 1.85. No MSRV is
published by K0.

## Behavioral oracles

The retained tests establish:

- core/event and Tokio/event poll the same current `Context` in lexical order,
  produce the trace `pending(1), ready(2), ready(1)`, and register the losing
  source waker once;
- all four expansion forms select `[input=9, stream=1, 2, 3]` with the same two
  successful service windows;
- an armed retained one-shot survives being polled pending before another arm
  wins, and a lower latched event survives an earlier winner without being
  polled;
- an absolute Tokio deadline survives a lost race and fires at its original
  instant rather than being reconstructed relatively;
- guard and buffered-yield snapshots are once per arbitration; a wake-only
  external guard change does not alter the current snapshot;
- bounded per-item drain, between-item buffered yield, `Stop`, `Err`, and panic
  have the specified `after_event` behavior and invent no compensation;
- optional mpsc closure becomes dormant without self-wake, emit-once closure is
  emitted once, and optional deadlines disarm before delivery;
- the Grok fixture compiles all four 23-arm forms, preserves the declared raw
  Tokio order, dynamically arms voice, replaces reconnect work, and drains 32
  scripted ACP items identically in core and Tokio controls;
- the raw and generated Grok controls pass the same application-owned
  presenter scenario: repeated request coalescing, no-payload draw, last-payload
  gating, stale acknowledgement, and a deadline firing while acknowledgement
  is delayed;
- the embedded fixture re-arms two consecutive absolute deadlines, protects
  touch from ready telemetry, models dormancy, and returns exclusive display
  plus framebuffer ownership before resubmission.

Application presentation, writer ordering, resource ownership, dynamic mode
truth, terminal handoff, task ownership, and teardown remain ordinary Rust
fixture behavior—not Kittens kernel guarantees.

## Cooperative-budget finding

The mpsc drain adapter checks Tokio's public boolean budget state before each
synchronous `try_recv`; it neither calls `unconstrained` nor models budget in
kernel vocabulary. A forced 32-item drain test records:

```text
(item, has_budget): [(1, true), (boundary, false), (2, true), (3, true), (4, true)]
successful after_event windows: [1]
```

Both core/event and Tokio/event produce that result when run in fresh Tokio
tasks. The first window ends after one item while buffered messages remain, and
the next arbitration resumes after Tokio replenishes the task budget. The
ordinary Grok 32-item script does not reach an exhausted-budget boundary; its
handler yields between items, and both controls handle all 32 in one Kittens
service window.

Tokio's numeric `Budget(Option<u8>)` and getter are crate-private in 1.53; the
stable public API exposes only the boolean probe. Consequently this report does
not claim the numeric instrumentation required by section 37.7. Per the
registered fallback, single-item and observed boolean-boundary equivalence are
established, while full numeric drain-window attribution remains open. No
`unconstrained` workaround is used.

## Mutations and diagnostics

Eighteen compile-fail fixtures retain rustc output. They cover:

| Mutation | Retained result |
|---|---|
| `before` cycle | `KTR003`, closing relation and cycle consequence |
| non-final global `last` | `KTR004`, last and later IDs |
| missing buffered yield / sensor-over-touch shape | `KTR007`, dominant and protected IDs plus reorder/yield repairs |
| nonliteral, zero, over-limit drain | `KTR008`, supported literal boundary |
| drain on non-drainable source | concrete source type and `DrainableSource` repair |
| missing required phase | `KTR011`, missing phase |
| temporary source expression | `KTR015`, persistent adapter/channel isolation repair |
| shutdown below firehose | `KTR016`, leading-prefix consequence |
| duplicate exact source place | `KTR020`, both IDs and alias limitation |
| every arm guarded by `#[when]` | `KTR014`, zero-source-poll/no-source-wake consequence and wake-capable unguarded-arm repair |
| non-Kittens arm attribute | `KTR000`, confirms doc attributes are the sole source-side exception |
| non-bool guard | bool helper type error |
| readiness mismatch | concrete expected readiness marker |
| unadmitted arbitrary future/display-like operation | concrete type, retained/latching or owned channel repair, and drop-cleanup distinction |
| non-backlog yield target | concrete type and backlog-capability repair |
| second ownership transfer | ordinary Rust `E0382`, explicitly not credited to Kittens |

Runtime mutations cover close/disarm dormancy, deadline fire/rearm, lost-race
retention, drain overflow, forced cooperative exhaustion, wake-only guard
mutation, handler error, and handler panic.

Compile-pass negative controls are published beside the failures. They show
that the following remain legal or outside macro inference:

- removing `shutdown`, `last`, or both a phase and its requirement;
- weakening starvation protection with a reason-bearing waiver;
- an unbounded raw handler drain loop or indefinite handler/phase await;
- raw handler-side source replacement;
- awaiting an unreviewed operation inside a handler;
- duplicate raw writer calls;
- descriptive double-buffer comments and an armed Off-mode source;
- raw Tokio selection and spawning;
- feature unification, which is handled by the build gate rather than rejected.

Trybuild snapshots pass on current stable. Rust-analyzer 1.96 completes a full
workspace diagnostic scan with only expected host-side inactive-code notices
for bare-metal `cfg` blocks. The purpose-specific trait-bound diagnostic is the
implemented candidate; the separately mandated three-way comparison against
associated-marker equality and an admitted newtype has not been performed.

## Agent ablation and rehydration

No protocol-conforming blind fresh-agent or context-reset trials have run. The
informal smoke exercise below reports eight lean-condition repair attempts, but
retains only four mutated inputs and eight final patch excerpts. It omits the
exact prompts, model identifiers, rustc JSON, test/oracle output, token/tool/time
counts, and per-trial transcripts required by section 27.4. With two reported
attempts per mutation, it also cannot meet section 37.11's four-of-five
promotion threshold. Repository-local guides, diagnostic anchors, mutation
snapshots, machine-readable index, and the expansion snapshot are present, but
their repair advantage over raw code or inert metadata is unmeasured.
Therefore:

- lean syntax is not promoted over maximal or annotated conditions by agent
  evidence;
- diagnostic wording/numbering remains provisional;
- the section 37.11 four-of-five/two-iteration promotion threshold remains unmet for every diagnostic and condition;
- context reconstructibility is an objective, not a demonstrated result.

## Falsifier assessment

| Registered concern | Result |
|---|---|
| 23-arm borrowing requires boxing, hidden task, scheduler, or broad state rewrite | not observed |
| embedded retention requires allocation, unsafe projection, or self-reference | not observed |
| lost wake/event or wrong lexical selection | not observed on retained traces |
| no-std path fails under feature unification | falsified; single facade links cleanly |
| generated machinery dominates ordinary handler errors | not observed in retained UI output |
| source error cannot lead from concrete type to safe repair | not observed for selected trait-bound form |
| presenter/input/source lifecycle silently changes | not observed in retained application oracles |
| default rustc limits fail or token growth is superlinear through 23 arms | not observed |
| core mechanism exceeds registered size threshold | **triggered** for generated-future size versus Tokio/event; review remains open |
| equal-budget drain attribution is impossible | **partially triggered** because only boolean, not numeric, state is public |
| rust-analyzer/formatting rejects ordinary use | not observed; parse-recovery UX was not separately benchmarked |
| agents bypass or delete constraints | open; the retained smoke-exercise patches show no bypass, but the required trial artifacts and comparative ablation are absent |
| optimized text and idle/all-ready poll regression | open; no pinned repeated benchmark |

The positive mechanism evidence supports continuing with architecture B/event,
but the triggered/open rows prevent a freeze or a statement that K0 closed.

## Section 37.14 graduation map update

| Provisional decision | K0 implementation status |
|---|---|
| direct core polling | selected candidate; behavior/no-alloc/borrow gates pass, size/budget/perf review open |
| expression-position lean grammar | implemented and rustfmt/rust-analyzer compatible; agent ablation open |
| source trait split, sealing, and outer `Unpin` | represents both fixtures with actionable selected-form errors; three-way diagnostic experiment open |
| no-std single facade and feature spelling | feature-unification link passes; discoverability ablation open |
| supported arm ceiling | 23 arms pass default compiler limits with sublinear measured expansion growth; higher ceiling unknown |
| guard grammar | synchronous bool expression works and snapshot behavior is tested; still provisional |
| diagnostic IDs/prose/order | snapshots pass on current stable; Rust 1.85 compilation passes, but its UI snapshots and agent repair validation remain open |
| context-reconstructible artifacts | artifacts exist; context-reset evidence open |

Stable K0 directions remain intact: Tokio is the first host integration but not
an executor owned by Kittens; topology semantics stay profile-neutral;
persistent admitted sources and biased lexical order are the behavioral
boundary; the kernel remains no-std/no-alloc; and cooperative budget stays out
of generated kernel code.

## Post-review improvements (2026-08-07, second pass)

A review pass against the SPEC section 37 contract added two code/test
improvements and fixed one specification defect the implementation itself
exposed:

1. **`KTR014` all-guarded zero-poll check.** Under K0 guard semantics a
   reactor whose every arm carries `#[when]` can take one all-false guard
   snapshot that polls no source and therefore registers no source wake in
   that arbitration. The macro rejects that directly visible topology with a
   wake-capable unguarded-control-source repair. This is deliberately not
   presented as a full liveness proof: source wake behavior is not encoded in
   the admission traits, and a permanently dormant source can still return
   `Pending` without registering. New compile-fail fixture
   `tests/ui/all_guarded.rs`. Adding the check exposed a masked-oracle hazard
   worth remembering whenever a validation stage is added: two existing
   fixtures whose only arm was guarded (the guard-snapshot runtime test and
   the `guard_not_bool` UI fixture) started failing on `KTR014` instead of
   the oracle they exist to exercise. Both now use an unguarded retained
   cancellation source; unlike an empty local latch, its pending poll registers
   a wake.
2. **Doc comments accepted as arm/phase rationale.** `///` above a source arm
   previously died as `KTR000 unsupported attribute doc` — punishing exactly
   the context-reconstructible-source practice the specification mandates.
   Doc attributes are now accepted and intentionally not emitted (the
   expansion has no item to attach them to); all other non-Kittens attributes
   remain rejected so `cfg`-style conditional topology cannot exist silently.
   New pass fixture `tests/ui-pass/doc_comment_rationale.rs`; compile-fail
   fixture `tests/ui/non_kittens_attribute.rs` pins that `cfg` remains
   rejected.
3. **Specification example drift fixed.** SPEC section 38.2's Grok-shape
   sketch omitted starvation waivers on `acp_stream`, `task_events`, and
   `draw_deadline` and did not compile under the implemented direct starvation
   rule — a source carries one yield edge, so a lane protecting
   `terminal_input` cannot also protect the lanes below it. The spec example
   now carries the waivers, and the correction is recorded in the spec itself
   as a first-class instance of example-versus-checker drift. `KTR014` and
   `KTR020` are now also recorded in the SPEC section 25.2 catalog notes.

## Diagnostic-only repair smoke exercise (non-gating, reported 2026-08-07)

The retained record reports eight lean-condition attempts against four core
mutations. The reported method copied the repository without `.git` into
eight isolated workspaces, installed one mutated `examples/pilot.rs` in each,
and asked a fresh coding agent to repair the build while preserving intended
behavior and declared policy. The four mutated inputs and eight reported final
patch excerpts are retained in `docs/pilot-2026-08-07/`.

| Mutation | Diagnostic | Reported attempts | Reported iterations | Patch shown | Patch preserves named constraint |
|---|---|---:|---:|---|---|
| shutdown below firehose | `KTR016` | 2 | 1, 1 | moved the complete arm, attributes untouched | yes |
| missing buffered yield | `KTR007` | 2 | 1, 1 | added `#[yields_to(input, when = buffered)]` | yes |
| temporary source expression | `KTR015` | 2 | 1, 1 | retained `source::one_shot(...)` constructed before the loop | yes |
| drain on non-drainable latch | `KTR009` bound | 2 | 1, 1 | removed `#[drain]` | yes |

What the retained files support:

- Each retained patch excerpt is the canonical local repair for its mutation,
  and none deletes a declaration, adds a starvation waiver, or bypasses the
  constrained path.
- The record reports one compile iteration for each attempt, but the missing
  compiler/test artifacts prevent an independent reviewer from verifying that
  count or the final hidden-oracle result.
- The `KTR016`/`KTR007` fixtures produced a transient
  `unused import: Control` warning while macro failure suppressed arm
  expansion. Removing that decoy from future fixtures/error expansion remains
  a useful diagnostic-quality improvement.

This exercise is not a section 27.4/37.11 pilot and supplies no promotion or
release-gate evidence. Besides the absent artifacts listed earlier, it covers
only the lean condition, uses two reported attempts per mutation rather than
five, has no raw/annotated/maximal comparison, has no context-reset
rehydration variant, and does not retain a blinded semantic review. It is kept
as seed material for a reproducible harness and as a qualitative preview of
the intended repair path; the agent-ablation gate remains fully open.

## Crates.io publication (2026-08-08)

The user authorized publication of the manifest-declared `0.1.0` packages
after PR #1 merged. Both were published from commit
`c909b2121895329c034638a81702821c2fadf545` in dependency order:

- [`kittens-macros 0.1.0`](https://crates.io/crates/kittens-macros/0.1.0)
  passed Cargo's complete package verification and uploaded 5 files,
  52.3 KiB unpacked / 11.8 KiB compressed.
- [`kittens 0.1.0`](https://crates.io/crates/kittens/0.1.0) was verified only
  after Cargo resolved and compiled the published macro crate, then uploaded
  54 files, 134.7 KiB unpacked / 29.2 KiB compressed.

A fresh project outside the workspace downloaded both exact versions from the
crates.io registry and passed `cargo check`. Publication distributes the
experimental K0 evidence slice; it does not close or waive the open
architecture, ablation, performance, or stable-API gates recorded above. No
Git tag or GitHub release was created.
