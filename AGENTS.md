# Agent instructions for the Kittens monorepo

This file is the unified system prompt for every coding harness working in
this repository (Claude Code reads it through `CLAUDE.md`, which imports this
file; Codex, Cursor, and anything else reading `AGENTS.md` gets it directly).
It is itself an instance of the project's core doctrine: everything an agent
needs must be recoverable from the repository, because your conversation
history will not survive but this file will.

## What this repository is

The Kittens monorepo: an LLM-first Rust SDK for explicit, compile-checked
async orchestration, plus every profile crate built on it. The product frame
(root `SPEC.md` section 2.1): eliminate race conditions and ordering defects
by layers — inexpressibility first, static detection second, deterministic
schedule exploration third — with total coverage bounded by *escape surface*,
the share of concurrency behavior expressed outside the declared vocabulary.

| Crate | Role | Controlling contract |
|---|---|---|
| `crates/kittens` | `no_std` reactor kernel + Tokio source adapters | root `SPEC.md` §37 + `K0-REPORT.md` |
| `crates/kittens-macros` | the `reactor!` compiler (parser, topology validator, expansion) | root `SPEC.md` §37 |
| `crates/kittens-tui` | terminal-orchestration profile (input isolation, frame writer + acks, presenter gate, terminal lifecycle) | `crates/kittens-tui/SPEC.md` |
| `fixtures/*` | build gates (bare-metal link, feature unification, renamed dependency) | root `SPEC.md` §37.3.1, §37.14 |

Intended consumers, in ascending ambition: agents building one harness;
component-library and engine authors building rendering/IO systems on the
profiles; meta-harnesses — harnesses that generate and supervise other
harnesses — which consume both the kernel and the machine-readable topology
metadata of the code they emit.

## The law: spec first

Nothing gets implemented without a controlling contract. The root `SPEC.md`
is the kernel/K0 contract; **every profile crate carries its own `SPEC.md`**
recording scope, enforcement layers, normative protocols, required oracles,
negative controls, and deferred gates. If you are asked to build something
new: write or extend the spec first, commit it first, then implement against
it. If implementation teaches you the spec is wrong, fix the spec in the same
change and record the drift explicitly — spec-versus-checker drift is a
first-class defect here, never silently patched.

Know the normativity boundary: in the root `SPEC.md`, only §37 plus its
explicit imports (§20.2, §20.2.1, §20.2.2) are K0-normative. Sections 11–36
are deliberately retained superseded/candidate hypotheses — do **not**
implement from them or treat their APIs as current. Each profile spec states
its own boundary in its header.

## House rules

1. **Name the enforcement layer.** Every guarantee is backed by exactly one
   of: ordinary Rust ownership, a sealed trait/admission check, macro
   validation, private runtime state + deterministic tests, or documentation.
   A claim without a named layer is semantic theater and gets deleted.
2. **Declarations must earn their syntax.** A declaration exists only if a
   checker, generator, runtime protocol, or test consumes it (root §4.1).
3. **One canonical spelling per operation.** Alternatives are documented as
   visibly exceptional or not at all (root §4.9).
4. **State the negative controls.** Every constraint ships beside what it
   does *not* prevent (raw bypasses compile; handler interiors are
   unchecked). A report that lists only rejections is incomplete.
5. **Honest non-guarantees.** Never claim static coverage of external event
   order, handler termination, or anything behind an escape surface.
6. **Doc comments are load-bearing.** `///` rationale above reactor arms and
   at orchestration boundaries is the mandated practice (the macro accepts
   and discards them); write the *why* at the edit site.
7. **Evidence labels.** Research/reports use Fact / Observation / Hypothesis
   / Recommendation, and mark data-free questions as
   `**Gap: ... (no data exists)**`.

## Reading order when you arrive cold

1. This file.
2. The `SPEC.md` of the crate you are touching (root `SPEC.md` §37 + §38 for
   the kernel; §38 is the lean-grammar example set with checked/not-checked
   boundaries).
3. `K0-REPORT.md` — implemented decisions, measurements, and the **open
   gates**; never claim a gate closed that it lists as open.
4. `docs/agent-guide.md` (compact canonical grammar), `docs/diagnostics.md`
   (KTR/SRC index), `docs/agent-index.json` (machine-readable concept map).
5. The `tests/ui/` compile-fail fixtures of the crate — they are the
   executable repair reference; `tests/ui-pass/` are the negative controls.

## Working in the repo

- **Verification gates before any commit:**
  `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  `cargo test --workspace --all-features`,
  and for kernel changes the bare-metal gate:
  `cargo build -p kittens-no-std-fixture -p kittens-feature-unifier --target thumbv7em-none-eabi --release`.
- Trybuild snapshots: regenerate with `TRYBUILD=overwrite`, and **read every
  regenerated `.stderr` diff** — a silently rewritten snapshot can mask an
  oracle (this happened; it's recorded in `K0-REPORT.md`).
- New macro validation stages must check existing fixtures for masked
  oracles (a fixture failing on your new error instead of the one it exists
  to exercise).
- Branch + PR into `main`; commit messages explain the *why*; the changelog
  gets an entry per user-visible change.
- Everything lives in this monorepo — new Kittens-based crates go under
  `crates/`, with their `SPEC.md`, `README.md` (enforcement-layer table),
  oracles, negative controls, and a canonical example.

## Profile-crate checklist (what "done" means for a new crate)

Spec first, then: enforcement-layer table in the README; the crate's oracle
suite from its spec section passing in CI; compile-fail fixtures for every
type-level claim; negative controls published beside them; a canonical
runnable example with doc-comment rationale; non-goals stated (what the
crate refuses to own); deferred features listed **with gates**, not TODOs.
Profiles add domain producers and protocols on top of admitted kernel
sources — they never redefine kernel semantics (root §9.4).

## Publication policy

crates.io publication is an explicit, human-ordered decision — never
publish, yank, or bump versions on your own initiative. Order for new
crates: dependency crates first (`kittens-macros` → `kittens` → profiles),
wait for index availability between steps, dry-run before each publish,
record the publication in the changelog and a docs commit. Versions are
workspace-inherited.

## What not to do

- Don't implement from superseded spec sections (root §11–§36) or "improve"
  the kernel beyond its slice boundary without a new spec section.
- Don't delete a declaration, add a starvation waiver, or bypass the
  constrained path to make a build pass — that is the exact failure mode the
  benchmarks measure. Repair the topology, not the declaration.
- Don't add dependencies casually; the kernel path must stay `no_std`-clean
  under feature unification.
- Don't present coverage claims as static omniscience; the layered claim in
  root §2.1 is the only admissible form.
- Don't leave spec and code disagreeing — whichever is wrong, fix it in the
  same change and say so.
