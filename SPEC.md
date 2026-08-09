# Kittens preimplementation specification

- Status: ready for one reversible evidence-producing kernel slice after explicit authorization; not a frozen v0.1 API
- Initial specification date: 2026-08-06
- Preimplementation challenge review: 2026-08-07
- Embedded generality review: 2026-08-07
- Executor-boundary and build-surface review: 2026-08-07 (sections 20.2.1, 20.2.2, 37.3.1; cooperative budget, handler-panic drain state, expansion scaling, feature unification)
- Implementation-readiness refinement: 2026-08-07 (K0 normativity imports; lean readiness vocabulary fixed; adapter budget disclosure imported into 37.6; positive K0 gate checklist; `K0-REPORT.md` evidence artifact; K0 toolchain policy; `Control` import; predeclared embedded footprint budget; annotated-baseline representation rule; stale cross-reference and KTR014 reconciliation)
- Usage-sketch enrichment: 2026-08-07 (section 38: lean-surface usage sketches — minimal reactor, Grok-shape excerpt, dynamic source lifecycle, edit-and-repair loop, embedded shape, producer isolation, rehydration walkthrough; each with an explicit checked/not-checked boundary)
- Coverage-thesis and consumer-expansion refinement: 2026-08-07 (section 2.1 layered defect-elimination model with the six-nines goal as a falsifiable per-class claim; escape-surface terminology and benchmark metric; meta-harness and engine-author consumer tiers in sections 3, 5, 9.4; re-weighted post-K0 priorities in sections 21.1, 34.2, 37.14; `RESEARCH.md` section 20B)
- Profile-driven source extension: 2026-08-09 (section 37.6.1 admits one sealed, locally armed, allocation-free inline one-shot for the render completion gate; selection-loss retention is separated from inner-future honesty and raw mutation/drop escapes)
- Research basis: [`RESEARCH.md`](./RESEARCH.md), Grok Build commit [`393430ee4934bc791b0d538f304a21691c517433`](https://github.com/xai-org/grok-build/commit/393430ee4934bc791b0d538f304a21691c517433), and the revision-keyed embedded fixtures in section 37

The Rust blocks in this document are specification sketches and proposed test fixtures. They are documentation, not implementation source.

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative only within the implementation boundary defined in section 37 and within the subsections section 37 explicitly imports: 20.2 (generated arbitration semantics and the Tokio control sentinel), 20.2.1 (cooperative scheduling budget), and 20.2.2 (handler panic and reactor state), whose obligations are K0-normative as referenced by sections 37.5, 37.7, and 37.9. Earlier full-v0.1 sections are retained as a challenged architecture record. Their detailed APIs and acceptance criteria are candidate hypotheses, not authorization to implement them. If an earlier section conflicts with section 37, section 37 controls.

## 0. Evidence and commitment boundary

This document now separates three things that the earlier draft conflated:

1. **Observed constraints:** behavior supported by Rust, Tokio, Embassy/ESP-HAL contracts or pinned north-star code. These are stable inputs to the design.
2. **Kernel decisions:** reversible choices needed to run the first high-information implementation slice. They are concrete enough to implement and test, but they are not a semver promise.
3. **Extension hypotheses:** plausible APIs for rendering, scope, resources, capabilities, simulation, observability, and protocols. They remain documented so evidence and rejected alternatives are not lost, but they MUST NOT be implemented until a section-37 graduation gate promotes them.

### 0.1 Kernel, compiler, runtime, and profile direction

The long-term architecture is one semantic Kittens kernel with multiple runtime and domain profiles. The kernel is the smallest `no_std`, preferably no-alloc vocabulary for persistent sources, topology, ordering, phases, bounded service, control flow, and the local contracts that the compiler can consume. The procedural macro is a host-side compiler for that vocabulary. Tokio, Embassy, and other executor integrations supply concrete source adapters and lowering boundaries. TUI, embedded-UI, agent-harness, and future service profiles supply reviewed domain sources, protocols, capabilities, and utilities on top.

Profiles MUST share the kernel's semantic rules. They are not separate Kittens dialects with subtly different meanings for priority, dormancy, cancellation, or phase ordering. A profile MAY add domain vocabulary—such as terminal input and writer acknowledgements, interrupt and DMA completion, or model streams and approvals—but each addition must identify whether its guarantee is provided by ordinary Rust ownership, a kernel trait/admission check, macro validation, runtime state, or deterministic testing.

The term **language** in this specification means a Rust-embedded constraint language: familiar Rust expressions, structs, traits, attributes, and macro blocks carrying machine-consumed orchestration meaning. It does not mean a replacement general-purpose programming language. Raw Rust remains available, but effects expressed only through raw Rust are outside Kittens' ordering and authority guarantees unless a profile API explicitly mediates them.

The package split remains provisional. The architectural dependency direction is stable:

```text
kittens semantic kernel (`no_std`, preferably no-alloc)
        ↓
kittens compiler (`proc-macro`, host-side)
        ↓
runtime integrations (Tokio, Embassy, ...)
        ↓
domain profiles (TUI, embedded UI, agent harness, ...)
        ↓
application policy and raw Rust
```

K0 tests the kernel and compiler with a Tokio integration plus an embedded-shaped `no_std` fixture. It does not implement the profile layer yet. A future `kittens` facade, `kittens-core`, or profile crates are packaging decisions to be selected after K0 dependency and agent-discoverability evidence.

The previous draft froze exact grammar, public signatures, diagnostic numbers, module boundaries, and error taxonomies before the reactor macro, Grok-scale borrowing, source-trait errors, or agent repair behavior had been demonstrated. That confidence was too high. The behavioral oracles and invalid mutations are worth freezing; the public spellings that must survive those pressures are not.

### 0.2 Agent-first burden model

Human-first APIs have historically traded explicit semantic information for fewer keystrokes and an assumption that a programmer can retain the missing relationships in working memory. Kittens makes the opposite trade where it buys a real check. A coding agent may generate a plausible edit without recovering a distant comment about starvation, cancellation, ownership, or acknowledgement order. Those relationships should therefore live as close as possible to the edited source and be consumed by the compiler, generated control flow, runtime protocol, or test oracle.

The intended harness loop is:

```text
intent/specification
        ↓
agent-generated Rust
        ↓
compiler and macro checks
        ↓
tests/simulation/expansion inspection
        ↓
structured diagnostic
        ↺ agent repair
```

The compiler is part of the agent's reasoning environment, not merely a final gate. Kittens design quality therefore includes the size of the invalid-program space it removes and the quality of the repair path it provides. A diagnostic is successful when it lets an agent identify the violated relationship, understand the operational consequence, and make a constraint-preserving repair without reconstructing hidden policy from distant prose.

The governing surface rule is:

> Make intent explicit; automate mechanism.

Topology, precedence, fairness exceptions, authority, lifecycle ownership, phase placement, source readiness, cancellation/restart boundaries, and protocol state may be explicit when Kittens consumes them. Polling plumbing, wake registration, borrow bookkeeping, counters, adapter mechanics, and generated control flow should remain ordinary generated Rust. This is not a request to expose every implementation detail or to make Kittens a replacement language.

An agent-friendly abstraction often differs from a human-convenience abstraction: it asks the caller to state the policy that matters, then hides the mechanism that implements it. Canonical forms are consequently a search-space constraint. After evidence selects a spelling, Kittens SHOULD prefer one obvious composition over a family of equivalent aliases, because fewer valid idioms reduce API hallucination and make compiler feedback more predictable. Familiar Rust remains mandatory: canonicality must stay on the learned Rust manifold (`struct`, `enum`, traits, attributes, `async`, `.await`, ownership, and `Result`), not become a novel type calculus.

Semantic verbosity is not free verbosity. Tokens, context, compile time, and maintenance remain costs even when an agent writes the source. A declaration is justified only when it changes generated behavior, excludes a program, chooses a checkable weakening, improves a causal diagnostic, creates an independent integrity check against a type/adapter fact, supplies stable test identity, or measurably improves agent comprehension or repair. An annotation that merely repeats an unchecked fact is ceremony and MUST NOT be retained. The lean-versus-maximal ablation in section 27 is the evidence gate for this tradeoff.

Kittens extends Rust's existing constraint boundary rather than replacing it: Rust constrains memory and ownership topology; Kittens can constrain selected orchestration, lifecycle, authority, and protocol topology. Both remain deliberately partial. Raw Rust effects and external event order remain outside the Kittens proof boundary unless a supported profile mediates them.

### 0.3 Context-reconstructible artifacts

Kittens is designed for **context amnesia**. A future agent may not have the conversation, rationale, or working-memory state that produced a file. The codebase SHOULD therefore be able to re-educate a fresh agent about the local operating model from the source, its types, generated diagnostics, expansion, and nearby tests. High-level architecture documents remain useful summaries; they are not a substitute for placing modification-critical constraints near the code they govern.

The target property is **rehydratability**: after the original agent's context is discarded, another agent can recover the relevant sources, ordering, lifecycle, authority, protocol, and escape boundaries with few retrieval operations and without guessing from convention. A Kittens artifact is approximately stateless with respect to the agent that authored it when its important local invariants survive in machine-consumed declarations, type/method availability, compile-fail fixtures, reason-bearing metadata, or diagnostics.

Semantic redundancy is permitted as an architectural error-correcting measure. Duplicating implementation is still a maintenance defect. Duplicating a semantic fact can be valuable when the representations serve different consumers and are cross-checked: a source type may carry an adapter contract while a nearby reactor declaration carries the scheduling meaning; disagreement must be an error, not a silent preference. Repeating an unchecked label is not redundancy in this sense and remains ceremony. The source-contract and agent-ablation experiments decide which redundant forms earn promotion.

The preferred local recovery order is:

```text
compiler-enforced semantics
        ↓
type and method availability
        ↓
macro declarations and generated constraints
        ↓
tests and compile-fail artifacts
        ↓
reason-bearing metadata
        ↓
comments
        ↓
remote prose and conversation history
```

Reason strings MAY explain an explicit weakening or escape, but they are not proofs. They MUST be nonempty where the policy requires a rationale, MUST be safe to retain in diagnostics/indexes, and MUST NOT contain secrets or dynamic data. Names at orchestration boundaries SHOULD be descriptive enough to recover domain and authority (`terminal_input_source`, `validated_command`, `single_flight_presentation_gate`) when a shorter name would force a fresh agent to search elsewhere. This is a local-inferability rule, not a blanket requirement for long identifiers; naming cost and comprehension value remain benchmark variables.

No workspace Rust implementation, scaffold, fixture, or test code is authorized by this document. The embedded research pass explicitly authorized one disposable executor-neutral polling probe; it was kept outside the workspace, recorded in `RESEARCH.md`, and removed. When implementation is explicitly authorized, work starts with the section-37 kernel and stops at its evidence gate. It does not proceed through the older phase plan by default.

## 1. One-sentence definition

Kittens is an agent-first, context-reconstructible Rust-embedded constraint language and compiler for long-lived async orchestration: its `no_std` semantic kernel, familiar Rust declarations, typed persistent sources, and macro validation let coding agents express selected topology, side-effect boundaries, and ordering invariants, then lower them through runtime and domain profiles into ordinary executor-hosted code and local compiler feedback.

The broader lifecycle, authority, resource, protocol, TUI, embedded-UI, and agent-harness utilities are profile-layer candidates, not part of the minimum kernel needed to test the thesis.

## 2. Problem statement

Long-lived agent harnesses and embedded UI tasks combine streams or interrupts, human input, completions, timers, rendering, checkpoints, cancellation, and external resources. Rust, Tokio, Embassy, and HAL ownership already prevent memory unsafety and many lifetime/resource errors, but they still permit globally invalid orchestration:

- a firehose can precede and starve shutdown in a biased `select!`;
- a cancellation-unsafe operation can be reconstructed after every lost race;
- a closed optional receiver can remain immediately ready and hot-loop;
- a drain can be unbounded;
- a task handle can be dropped and its task detached;
- rendering can submit a second frame before the first is acknowledged;
- an unapproved command can reach an ambient shell;
- lifecycle requirements can exist only as comments adjacent to mutable code.

The current Grok Build loop makes the problem concrete. Its behavior relies on a biased 23-arm select, exact source order, loop-top housekeeping, post-event presentation, an ACP-to-input yield gate, bounded ACP draining, dynamically dormant timers and receivers, a deliberately last voice source, a terminal-reader isolation thread, a writer acknowledgement protocol, and ordered shutdown across the event loop, writer, agent, and child processes. Several of those invariants can be broken by an edit that still type-checks.

The embedded forcing fixture makes a narrower independent case. A nearby all-Rust ESP32-S3 watch loop uses Embassy's fixed-order `select3` over a dynamic timer and cancellation-unsafe ESP-HAL GPIO waits, changes cadence across Off/AOD/interactive/game states, and performs blocking display transfers. It confirms that source retention, ordered polling, dormancy, bounded service, and phase placement do not inherently require Tokio. It also shows that Tokio-style task scope, Grok writer tickets, one universal frame-in-flight model, handler responsiveness, and power behavior do not generalize merely because their diagrams look similar.

The kernel MUST make a useful subset of legal scheduler topology an explicit compiler input. It MUST NOT claim to determine when external events occur, preempt a handler, inspect arbitrary handler behavior, or schedule executor tasks. The objective is to test whether source ordering, persistent-source contracts, bounded macro-managed drains, buffered yields, dormancy, and loop phases can be constrained in an ordinary future without making normal Rust borrowing, pinning, code size, or diagnostics worse.

The primary optimization target is:

> agent success rate × meaningful static guarantees × ecosystem compatibility × diagnostic quality

Source-code length is not a primary metric.

### 2.1 Coverage thesis: where race and ordering defects go

The governing product question, stated as its user would state it: *what if I want Rust for a harness with a GUI/TUI, all async, on an embedded device, and an SDK that at the code and compilation level captures 99.9999% of the race conditions and ordering issues of my codebase?* This section makes that goal a falsifiable engineering claim instead of a slogan. It is design doctrine in the sense of section 4 — it governs prioritization and honest presentation; it adds no K0 scope and section 7's non-guarantees control wherever they appear to conflict.

Exactly three mechanisms make a concurrency defect disappear, with different ceilings:

1. **Inexpressibility** — the bug cannot be written. Coverage of its class is ~100%, permanently, at zero diagnostic cost.
2. **Static detection** — the bug compiles into an error. Near-complete for *declared* topology; zero for undeclared intent. The section 37.9 negative controls exist to measure exactly that boundary.
3. **Deterministic schedule exploration** — the bug is dynamic but reproducible under scripted schedules and paused time. Asymptotic, bounded by the scenario corpus.

Any credible total-coverage figure is the product of these layers, and the dominant contributor is inexpressibility — which is why the single-reactor architecture is itself the primary coverage mechanism, before any check runs: one owner of application state, handlers that run to completion between arbitrations, and concurrency entering only through declared sources make intra-reactor shared-state races structurally unwritable.

| Defect class | Eliminating layer | Coverage character |
|---|---|---|
| memory-level data races | safe Rust ownership, `Send`/`Sync` | inexpressible; complete before Kittens exists |
| intra-reactor shared-state logical races | single-owner reactor architecture | inexpressible by construction within one reactor |
| arbitration ordering: shutdown prefix, precedence, `last`, starvation, unbounded macro drains, missing yields | `reactor!` static validation | complete for declared relations; undeclared intent is the measured boundary |
| selection-loss and cancellation races at sources | sealed admission plus reviewed adapters | complete on the constrained path; adapter truth remains audit plus lost-race tests |
| dormancy and hot-loop defects | adapter-owned dormant state | inexpressible on the supported path |
| interrupt/DMA reuse races | existing HAL/Rust ownership, composed not claimed | complete where the selected API is ownership-bearing |
| dynamic protocol races: stale acknowledgements, double in-flight submission | runtime protocol state plus deterministic tests | runtime-checked; inherently not static |
| handler-interior defects: long awaits, unchecked loops, rearm liveness | deterministic scenarios, latency oracles, watchdogs | residual class; unreachable by static means |
| external event order, hardware timing | none | bounded and measured, never proven |

Three consequences are normative for how this project talks and prioritizes:

- **Coverage is bounded by escape surface, not check strength.** Raw spawns and selections, undeclared producers, handler side channels, and cross-task channel webs move behavior out of every layer above the residual ones. Escape surface (section 8) is measured and reported, never prohibited — Kittens is not a language-level prohibition mechanism (section 5). One reactor per executor task is the high-coverage topology; every task split reintroduces channel discipline between reactors as an application obligation, and cross-reactor races on Embassy remain outside any current guarantee (section 20.4, `RESEARCH.md` 20B).
- **The six-nines figure is admissible only as this per-class, layered, falsifiable claim.** Documentation, benchmarks, and any future marketing MUST NOT present it as static omniscience; that presentation would be semantic theater at product scale, failing the same admission test section 4.1 applies to individual declarations.
- **The residual classes re-weight the post-K0 queue.** K0 is unchanged — it is the falsification step for the static and admission layers. But under this thesis the deterministic scenario layer (section 21) and the escape-surface lint (section 34.2) graduate first among post-K0 extensions, because they are the only mechanisms that reach handler interiors, rearm liveness, and schedule-dependent protocol bugs. Section 37.14 records this priority.

**Gap: no field baseline exists for the distribution of real-world async firmware/TUI race and ordering bugs across these classes (no data — the section 37.9 mutation corpus is a designed proxy, not a field study; see `RESEARCH.md` section 20B).**

## 3. Primary consumer and persona

The primary consumer is a coding agent generating, modifying, compiling, and repairing a Rust agent harness. The secondary consumers are the humans who review and operate that harness.

The assumed agent already understands:

- stable Rust, structs, enums, traits, ownership, borrowing, and `Result`;
- `async fn`, futures, channels, Tokio tasks, and `tokio::select!`;
- attributes and procedural macros;
- RAII and consuming methods.

The agent is not assumed to know category theory, indexed monads, an effect calculus, session-type notation, or a Kittens-specific language. A first-time reader SHOULD infer the purpose of a declaration from local names and familiar Rust syntax.

The compiler is part of this persona's interactive environment. Kittens diagnostics MUST identify the local source ID, violated relationship, and a concrete repair direction whenever stable Rust permits it.

Two further consumer tiers follow from the coverage thesis (section 2.1, `RESEARCH.md` 20B.1). They are profile-direction statements, not K0 scope:

- **Meta-harnesses: harnesses that build harnesses.** An agent harness that generates, hosts, and supervises other harnesses consumes Kittens twice — as the reactor substrate of its own loop and as the target vocabulary of the code it emits. For a generator, canonical spellings (section 4.9) and machine-readable topology metadata (section 21.4) are load-bearing rather than convenient: emission and verification are mechanical, so one spelling per operation and a stable declaration schema matter more than for an agent editing one file interactively.
- **Engine authors: next-generation rendering and I/O engines.** These consumers need a declared-topology substrate beneath frame pacing, input pipelines, and device-completion handling. The section 5 boundaries stand unchanged: the engine owns pixels, buffers, compositing, transports, and device protocols; Kittens owns only the orchestration law the engine is built on, reached through profiles (section 9.4).

## 4. Design principles

### 4.1 Constraint-revealing verbosity

Additional syntax is desirable when it supplies machine-checkable information about priority, fairness, lifecycle, cancellation, readiness, draining, state availability, authority, ownership, or protocol state. Additional syntax that merely restates a type without enabling a check is boilerplate and MUST be removed.

A 20–40% increase in loop size is acceptable if illegal reorderings, unbounded drains, cancellation hazards, or protocol violations stop compiling.

The increase is not accepted in advance. Every mandatory declaration MUST do at least one of the following: change generated behavior, exclude a program, select an explicitly weaker policy, improve a diagnostic in a measured repair task, or provide test identity that cannot be recovered reliably elsewhere. Mandatory lifecycle, cancellation-safe, and close annotations fail this test in the current kernel because they merely restate sealed adapter facts; section 37 removes them.

The relevant comparison is not source length alone but semantic gain per added source token and per unit of agent context. The annotated-baseline experiment in section 27 separates the value of making intent locally visible from the value of enforcing it. Kittens MUST remove declarations that do not outperform both raw code and non-enforced local metadata on the task they claim to help.

### 4.2 Familiar Rust surface

Handlers MUST remain ordinary Rust blocks using ordinary values, method calls, `.await`, and `Result`. The reactor syntax SHOULD resemble familiar Rust selection plus attributes; runtime-neutrality does not justify a novel language. Kittens MUST NOT introduce a general `Effect<R, E, A>` representation or require application-wide generic plumbing.

### 4.3 Local properties in types; global properties in macros

- Ownership MUST enforce single consumption.
- State-specific inherent methods SHOULD enforce local workflow order.
- Concrete capability values MUST enforce possession of authority.
- Sealed source traits MUST admit only the adapters reviewed for the requested local polling capabilities. Rust proves membership and capability use; primitive documentation, adapter review, and tests establish whether an implementation actually satisfies its semantic contract.
- The reactor proc macro MUST validate whole-loop graphs, source relationships, phase presence, and policy compatibility.
- Private runtime state MUST enforce dynamic facts such as current acknowledgement tickets.
- Deterministic tests MUST cover named critical schedules and faults not excluded statically; Kittens does not claim exhaustive schedule coverage.

### 4.3.1 Side-effect language boundary

Kittens may describe side-effect ordering only at boundaries it can see and mediate. A source relation orders local arbitration; a phase declaration orders generated control flow; a capability or consuming protocol value restricts an API that opts into it. None of these declarations may be presented as ordering arbitrary method calls, I/O, rendering, power transitions, or external effects hidden inside ordinary Rust.

Every profile-level side-effect declaration MUST name its consumer. The consumer may be a type/ownership rule, a generated macro edge or phase, a runtime protocol state machine, or a deterministic test oracle. A declaration that only labels an effect without changing legal code, generated behavior, runtime validation, or test identity is semantic theater and MUST stay out of the profile API.

### 4.4 Use the weakest sufficient mechanism

Kittens MUST classify each invariant before implementation. It MUST prefer, in order of locality and diagnostic quality, ordinary Rust ownership, a small trait bound, typestate, a capability value, macro validation, a runtime assertion, deterministic simulation, and finally documentation. This is not a hierarchy of prestige; later mechanisms are correct when the fact is inherently dynamic.

### 4.5 Boring expansion

Sophisticated compile-time analysis SHOULD emit polling code an experienced Rust async engineer could have written manually. Expansion MUST NOT contain a runtime graph interpreter, executor, hidden task spawn, global registry lookup, or allocation merely to represent topology.

### 4.6 Explicit priority is not fairness

Polling precedence and starvation behavior are distinct. A priority edge MUST mean only: if both sources are ready during the same local arbitration, the predecessor is polled and selected first by the generated lexical poller. A fairness declaration MUST describe how a source yields or whether starvation is accepted. Neither statement promises handler preemption or executor task priority.

### 4.7 Conservative semantic contracts

Kittens MUST not infer selection-loss preservation, drop cleanup, or restartability from an arbitrary future body. These are distinct contracts. Reviewed source admission MUST be conservative. During K0, unreviewed integrations are isolated behind an explicitly owned producer and approved channel source. Kittens scope helpers and a reason-bearing escape remain extension hypotheses.

### 4.8 Progressive constraint

Leaf async functions MAY remain ordinary Tokio-compatible Rust. Long-lived orchestration boundaries SHOULD use a reactor. Full typed protocols and advanced state machines remain opt-in and MUST NOT infect unrelated signatures.

### 4.9 One canonical spelling

After evidence selects a public surface, each common operation SHOULD have one documented spelling. K0 deliberately compares macro forms and annotation sets before applying this rule; it must not publish equivalent aliases merely to preserve experiments.

Canonicality is an agent search-space control, not a prohibition on legitimate escape hatches. The guide MUST show the canonical path first, make alternate forms visibly exceptional, and keep equivalent spellings out of the kernel unless a benchmark demonstrates distinct semantic value. A more flexible API is not automatically more agent-usable: every additional valid composition is another opportunity for an agent to choose an untested or semantically weaker idiom.

### 4.10 Evidence before cleverness

Every uncertain type-system or macro claim MUST receive a retained compile-pass/compile-fail prototype before production implementation proceeds. Historical scratch results are evidence, not a substitute for the retained conformance suite required by this specification.

### 4.11 Freeze behavior after evidence, not before it

The first implementation may revise exact grammar, trait decomposition, helper names, diagnostic IDs, and generated borrow scopes while the kernel gate is open. It MUST preserve the named behavioral oracle when comparing alternatives. A public spelling freezes only after it passes Grok-scale compilation, expansion review, mutation rejection, rust-analyzer inspection, and diagnostic-only agent repair.

### 4.12 Enforced scope must be adjacent to the claim

Kittens MUST distinguish a supported-path guarantee from arbitrary Rust. A bounded-drain annotation constrains only the drain generated by the macro; a handler can still write an unbounded loop. An approved reactor source constrains the raced source operation; a handler can still await a cancellation-unsafe operation after selection. A phase hook controls generated placement; it cannot prevent a handler from calling an unrelated writer API. These limits MUST appear beside the feature and in its diagnostic documentation.

### 4.13 Context-reconstructible source

Every promoted profile SHOULD make the important “why is this allowed?” facts visible at the use site. A typed `ApprovedCommand` and an explicit shell capability should be visible at execution; a scope-owned spawn should identify its lifecycle owner; a render submission should expose the participating gate or ownership-returning transfer. This does not require wrapping every value. It requires that an agent modifying a consequential call can recover the authority, ownership, and protocol preconditions without replaying the original conversation.

### 4.14 Checked semantic redundancy

When a type-level contract and a reactor declaration intentionally repeat a fact, the macro or runtime/test boundary MUST independently consume both representations. For example, a declaration that says a source is repeating or may remain ready is useful only if it is checked against an admitted source contract or changes global validation. If the type already proves the fact and the declaration is not cross-checked, the declaration MUST be removed or demoted to ordinary documentation. The same rule applies to profile metadata, escape reasons, and phase labels.

## 5. Explicit non-goals and first-slice exclusions

Kittens is not:

- a replacement runtime, executor, task scheduler, timer wheel, interrupt controller, I/O stack, or HAL;
- a general functional-programming or effect-system framework;
- a replacement general-purpose language or a general-purpose macro DSL; its Rust-embedded declarations are intentionally a limited constraint language for selected orchestration facts;
- a theorem prover or proof that a handler terminates;
- a durable workflow database;
- a complete agent framework, LLM SDK, MCP stack, or actor framework;
- a sandbox merely because capability values are used;
- a full multiparty session-type system;
- a general fair/weighted scheduler;
- an implementation of async destructors;
- a guarantee that arbitrary third-party futures are cancellation-safe;
- an attempt to encode runtime sequence numbers, timestamps, queue lengths, or event arrival order in types;
- a prohibition mechanism for importing `tokio` or `std` directly.
- a UI, widget, graphics, display-driver, framebuffer, or power-management framework;
- a guarantee that `DMA complete` means a frame is visible or that a declared source state implies a measured power level.

These exclusions are boundaries, not indifference. Sections 2.1 and 3 position Kittens as the orchestration substrate on which rendering engines, I/O engines, and meta-harnesses are built; the pixels, buffers, transports, device protocols, and generated products remain theirs.

The first kernel slice additionally excludes `scope`, timeout, `resource`, `cap`, `flow`, public `sim`, stable tracing/serialization, Tower/process backends, a generic rendering gate, phase capability values, Embassy/ESP-HAL production adapters, and escape APIs. These exclusions keep independent hypotheses from obscuring the reactor result. They are not permanent architectural rejections.

The candidate architecture does not include a generic retry framework. Retry is operation-specific because cancellation safety, idempotency, approval consumption, and external commits differ. This remains research guidance, not a kernel deliverable.

## 6. Candidate end-state guarantee map

This section records the stronger architecture previously proposed. It is not the first implementation contract. Section 37 selects the subset the kernel must actually demonstrate. A future feature may adopt one of these guarantees only after defining its precise supported-path boundary and passing its own graduation gate.

### 6.1 Reactor topology

- Every declared source has a stable, unique source ID.
- Priority and explicit source-precedence graphs are acyclic.
- A source marked `last` is last in the generated poll order.
- Every shutdown source is terminal and starvation-protected, and all shutdown sources form the leading lexical/poll prefix before every non-shutdown source.
- Every source declaration exposes only metadata consumed by global validation; readiness declarations match sealed source-type contracts through generated trait checks.
- Every macro-managed drain has a positive, bounded literal maximum and uses a drainable source. Ordinary handler loops remain ordinary Rust and are outside this guarantee.
- A declared buffered-yield relationship targets a backlog-probeable source and is enforced before initial polling and between drained items.
- Required lifecycle phases are present exactly once.
- If a future Kittens-mediated frame gate is promoted, its submission API may require a non-cloneable phase permit. This would constrain only that gate, not arbitrary writer calls.
- Every successfully handled continuing event or allocation-free service window runs the declared `after_event` phase exactly once.
- Dynamic branch guards are explicit in the reactor declaration.

### 6.2 Source behavior

- When another source wins an internal reactor race, the persistent operation state of an admitted source is retained.
- Source admission does not by itself promise that destroying the whole reactor preserves an event, that Drop is asynchronous, or that a partially performed external operation is safe to restart.
- Optional deadlines and one-shot adapters disarm before exposing their event.
- Optional channel-like sources become dormant after closure unless their explicit close policy emits one close event; they do not repeatedly return a closed result.
- A dormant source remains pending without self-waking until explicitly armed.
- Channel/task isolation is available for operations that cannot satisfy the repeated-source contract directly.

### 6.3 Structured lifecycle

- A task spawned through a `Scope` remains registered until completion is drained.
- Dropping a typed task handle cannot detach the underlying task from its owning scope.
- Normal completion of `scope::run` does not return until remaining children have undergone the configured cooperative-cancel, grace, abort, and drain sequence.
- A Kittens timeout does not report completion before its nested scope has completed that shutdown sequence.
- New spawns are rejected after scope closing starts.

### 6.4 Authority and local workflow

- Kittens capability constructors establish an explicit trust boundary; ordinary capability operations require possession of a concrete value.
- Narrowing APIs cannot return broader Kittens authority than their receiver represents.
- Non-cloneable approvals and submission permits cannot be consumed twice in safe Rust.
- State-specific methods can make illegal local workflow transitions unavailable.
- Kittens approval values are bound at runtime to the approved action identity and policy context as well as being consumed statically.

### 6.5 Dynamic protocols

- The earlier candidate single-flight gate would never report a second submission permit while a ticket is in flight; this is not a K0 guarantee and its generic API was weakened by the embedded evidence.
- Requests received while a ticket is in flight are coalesced according to the configured merge policy.
- A stale acknowledgement cannot unlock a newer ticket.
- Dropping an uncommitted submission permit requeues its request.
- A reported no-output presentation clears the satisfied request without creating an in-flight ticket.
- A non-monotonic accepted ticket poisons the gate rather than silently weakening acknowledgement ordering.

### 6.6 Repairability and transparency

- Macro-owned failures use stable diagnostic IDs and point at the declaration that introduced the violated relation.
- Expansion contains direct, readable `Future::poll` control flow and stable declared IDs in test traces; the direct Tokio oracle remains available for comparison.
- Every static guarantee has at least one compile-pass and one compile-fail or macro-fail conformance fixture unless impossibility is documented.

## 7. Guarantees Kittens explicitly does not provide

Kittens does not guarantee:

- the runtime order in which external events become ready;
- fairness from Tokio, the OS, a remote service, or a handler that never yields;
- that a source declaration corresponds to honest external behavior after an escape hatch;
- that a `#[when(...)]` expression is pure, cheap, or stable unless the application makes it so;
- that a handler is bounded, nonblocking, deadlock-free, or free of logical races;
- that a may-remain-ready source eventually becomes quiescent;
- that application code will not explicitly rearm a quiescent dynamic source on every loop iteration; repeated immediate rearming is a runtime liveness risk that traces and deterministic scenarios must cover;
- delivery of channel messages after their receiver is deliberately disarmed or replaced;
- rollback or atomicity for arbitrary external side effects;
- acceptance atomicity for an arbitrary `SingleFlight::commit_with` closure that violates its documented result/unwind contract;
- asynchronous cleanup after process abort, `SIGKILL`, runtime destruction, `mem::forget`, or arbitrary drop of the outer Kittens future;
- asynchronous release on panic unwind; only normal synchronous `Drop` behavior is universal;
- abortability of already-running `spawn_blocking` work;
- prevention of raw `tokio::spawn`, `tokio::select!`, `std::fs`, `std::process`, or ambient network use outside the Kittens API;
- security isolation against malicious in-process Rust code;
- static validation of runtime frame tickets, reconnect generations, deadlines, queue contents, or restored durable records;
- that all valid programs are correct or that all invalid harness designs are rejected.

When a guarantee is conditional, API documentation MUST state the precondition beside the method, not only in a conceptual guide.

## 8. Terminology

| Term | Normative meaning |
|---|---|
| reactor | A long-lived loop with optional one-time initialize, then explicit before-arbitration, poll/select, handle, and after-event phases. |
| arbitration | One guard snapshot and selection future, beginning after `before_poll` and ending when one source is selected. It may span multiple executor calls to `Future::poll`. |
| source | A persistent object whose next event can participate in repeated reactor races. |
| source ID | A macro-local stable Rust identifier used in constraints, diagnostics, traces, and replay. |
| scheduler topology | The declared sources and graph of priority, precedence, yield, lifecycle, and phase constraints. |
| priority edge `A > B` | During one local arbitration, every source in A is polled before every source in B. It is not event-arrival order, handler preemption, or executor task priority. |
| source precedence | A source-specific edge added by `#[before(other)]`. |
| linear extension | The lexical branch order, which must be a valid total ordering of the declared acyclic constraints. |
| may-remain-ready | The source may produce unbounded consecutive ready events, such as a backlogged channel. |
| quiescent-after-event | The adapter disarms or otherwise cannot produce another event until explicit rearming or a new external change. |
| close behavior | Whether a producer cannot close, silently transitions dormant on close, or emits one typed close event before dormancy. |
| starvation-protected | Every preceding may-remain-ready source must yield to this source; starvation acceptance is forbidden. |
| starvation-allowed | The declaration explicitly accepts possible starvation and gives a nonempty reason. |
| buffered yield | A higher source is disabled while the target reports buffered work; the same test stops its bounded drain. |
| cancellation | A request to stop cooperative work; in raw futures, commonly implemented by ceasing to poll and dropping. |
| selection-loss preserving source | When another reactor source wins, this source retains the operation/event state required for a later poll. This says nothing by itself about destroying the reactor or restarting an external operation. |
| drop-clean operation | If the destructor is invoked and completes normally, it releases memory and resources according to its documented contract. `mem::forget`, process/runtime destruction, abort behavior outside that contract, and a panicking destructor are excluded. This does not imply logical progress preservation or restartability. |
| reconstructable waiter | A reviewed operation whose waiter may be dropped and recreated without losing required progress or events. This is stronger than drop cleanup and is not inferred from `async`. |
| repeat-safe operation | After cancellation or completion, starting the operation again is valid under its external semantics. This may additionally require idempotency or application policy. |
| cancellation-atomic | An operation-specific guarantee that cancellation exposes either the pre-operation or committed state, never a partial public state. Kittens does not infer it. |
| cancellation-deferred | Cancellation is recorded but not acted upon until a delimited operation or cleanup region completes. |
| cleanup-guaranteed | A term Kittens MUST qualify: synchronous cleanup depends on the destructor being invoked and completing normally; async cleanup is guaranteed only while a Kittens-owned future continues to be polled within its cleanup budget. |
| dormant | A dynamic source state that polls pending and does not self-wake until armed. |
| terminal branch | A branch whose successful handler returns the reactor's exit value rather than `Control::Continue`. |
| shutdown branch | A terminal, starvation-protected branch representing cancellation or graceful shutdown. |
| event | One selected source item. |
| service window | The first selected item plus up to `max - 1` immediately available items handled one at a time. K0 allocates no batch container. |
| continuing handler completion | The handler returns `Ok(Control::Continue)`. `Stop`, `Err`, panic, and abort do not enter `after_event`. |
| scope | The structural owner and shutdown coordinator for registered child tasks. |
| capability | A concrete, privately constructible value required to exercise Kittens-mediated authority. |
| approval | A non-cloneable, action-bound authorization value intended for one consuming operation. |
| escape hatch | A deliberately less-constrained operation under `kittens::escape`, requiring an audit reason. |
| escape surface | The concurrency-relevant behavior of a codebase expressed outside the declared Kittens vocabulary: raw spawns and selections, undeclared producers, handler side channels, cross-task channel webs. Coverage under section 2.1 is a function of minimizing it; it is measured and reported, never prohibited. |
| semantic verbosity | Extra syntax that enables validation or makes an accepted risk explicit. |
| boilerplate | Extra syntax that enables no check and carries no recoverable architectural intent. |

## 9. Candidate architecture and stable kernel boundary

The stable center is a Rust-native/proc-macro hybrid. The leading runtime boundary is now an executor-hosted future rather than Tokio selection itself:

```text
agent-generated harness
        │
        ├── KERNEL: reactor! + persistent source values
        │       ├── host proc macro validates declared topology
        │       └── generated core::future polling + Control
        │                              │
        │                              ▼
        │                   one ordinary Future
        │                     ├── Tokio executor + adapters (K0)
        │                     └── Embassy/HAL adapters (deferred)
        │
        └── CANDIDATE EXTENSIONS, each separately gated
            ├── rendering protocol
            ├── Tokio structured scope/resource lifetime
            ├── capability/approval values
            └── simulation/observability tooling
```

There is no Kittens executor or task scheduler/interpreter. The generated future performs only local branch arbitration with the enclosing executor's `Context` and waker. It owns no task queue, I/O driver, timer wheel, interrupt registration system, or sleep policy. The reactor graph exists during macro expansion. Scope registries, protocol gates, persisted topology metadata, and public observability records are not assumed until their own slices justify them.

### 9.1 Invariant placement methodology

| Invariant | Required mechanism | Reason |
|---|---|---|
| approval can be used once | ownership | a moved non-`Clone` value gives the clearest local error |
| execute unavailable before approval | typestate/inherent methods | method availability is clearer than a global trait proof |
| filesystem authority but no network | concrete capability values | possession is local and explicit |
| source state preserved when another branch wins | conservative trait admission plus adapter evidence | Rust rejects unadmitted types; the semantic property itself belongs to the reviewed adapter and primitive contract |
| priority/yield cycle | proc-macro graph validation | requires whole-reactor knowledge |
| `last` source placement | proc-macro validation | global ordering property |
| dynamic timer armed/dormant state | private runtime state in adapter | readiness changes at runtime |
| one frame currently in flight | application-owned private runtime state initially | ticket existence is dynamic; compare a later consuming permit against this baseline |
| external event order | not static | outside-world order is inherently dynamic |
| rare schedule/failure bug | deterministic tests initially | valid topology can still have bad schedules; a public simulator is not yet justified |
| arbitrary handler termination | documentation/operational watchdog | not reasonably provable by this library |

### 9.2 Compile-time boundary

The proc macro validates only facts present in its token input. It MUST emit generated trait assertions for type facts it cannot inspect. It MUST NOT pretend to inspect an arbitrary expression's trait implementations, cancellation semantics, side effects, or runtime readiness.

Stable procedural macros cannot provide the full nightly `proc_macro::Diagnostic` experience. The kernel will compare span-local `compile_error!` messages and generated trait-assertion anchors. Numeric IDs and exact helper names remain provisional until rustc and agent-repair evidence shows which form is causal and stable.

### 9.3 Generated control-flow boundary

The macro expansion target consists conceptually of the steps below, but the borrow-scoping strategy is deliberately provisional. The first slice MUST compare exactly two handler-transfer forms usable by both polling backends: (1) a private owned-event enum followed by an external match, and (2) a small selected-source tag plus one private per-arm `Option<Item>` slot, followed by an external match that takes the selected slot. Both must preserve hook semantics and diagnostics; their different future-size costs are measured.

1. compile-time-only contract assertions;
2. a borrow-ending representation of the selected item, possibly a private event enum;
3. the optional one-time `initialize` async block;
4. an ordinary loop;
5. the optional `before_poll` async block;
6. guard evaluation;
7. one ordered polling closure in the written lexical order, with direct Tokio selection retained only as the K0 comparison;
8. bounded synchronous draining where declared;
9. an ordinary match running the selected handler block one or more times;
10. the optional `after_event` async block once per successful event/service window;
11. explicit continue, normal-stop, or error propagation.

No step may spawn work implicitly.

### 9.4 Profile architecture

The eventual Kittens ecosystem is layered rather than monolithic:

| Layer | Purpose | What it may add | What it must not redefine |
|---|---|---|---|
| semantic kernel | `no_std`/no-alloc source and topology vocabulary | `Control`, source admission, phases, ordering, dormancy, bounded service | runtime task scheduling, UI policy, hardware ownership |
| compiler | parse and validate the embedded constraint language | graph diagnostics, generated polling, profile-specific checked declarations | hidden runtime interpretation or executor queues |
| runtime integration | connect sources to an executor | Tokio channels/timers, Embassy futures, HAL adapters, wake/registration details | the meaning of lexical precedence, dormancy, or declared relations |
| domain profile | make a use case locally expressible | TUI presenter/input utilities, embedded interrupt/DMA adapters, agent stream/approval utilities | claims about effects not consumed by a type, macro, runtime protocol, or test |
| application | choose policy and own domain state | rendering, power, model/tool behavior, product workflows | assumptions that a profile silently enforces |

Candidate profiles include:

- `kittens-tui`: terminal input isolation, writer acknowledgement, render-request coalescing, draw deadlines, and TUI-specific lifecycle guidance;
- `kittens-embedded`: fixed-capacity sources, interrupt/latch adapters, dynamic cadence, display-transfer completion, and low-power integration guidance;
- `kittens-agent`: model streams, tool completion, approval responses, human input, cancellation, budgets, and checkpoints;
- future service profiles for network, filesystem, or child-process orchestration where a repeated benchmark demonstrates a profile-specific benefit.

The same layers are the intended substrate for engine authorship and meta-orchestration: a next-generation rendering engine or async I/O engine builds its frame pacing, input pipeline, and device-completion topology on the kernel vocabulary and publishes its own domain protocols above it, and a meta-harness — a harness that generates and supervises other harnesses — consumes both the kernel for its own loop and the machine-readable topology metadata of the code it emits (section 21.4). Profile APIs SHOULD therefore be designed to be *emitted by programs*, not only written by agents: stable declaration schemas, no context-dependent sugar, and verification artifacts a generator can check mechanically.

These names are architectural placeholders, not K0 package commitments. A profile graduates only after it demonstrates that its vocabulary prevents or diagnoses mistakes beyond raw Rust/Tokio/HAL APIs. Profiles may be separate crates, feature-gated modules, or facade re-exports; the decision remains reversible until dependency size, compile-time, and agent-discoverability evidence exists.

## 10. Candidate full architecture boundaries

This section preserves the previously proposed full package/module map for later evaluation. It is not the first implementation boundary. Section 37 requires only `kittens`, `kittens-macros`, and the minimum `reactor`/`source` surface. No other module or dependency below may be scaffolded merely to reserve its shape.

### 10.1 Workspace packages

Any proc-macro implementation requires two packages:

| Package | Responsibility |
|---|---|
| `kittens` | one public facade; candidate no-std/no-alloc reactor/source base plus feature-gated runtime integrations |
| `kittens-macros` | the `reactor!` procedural macro and compile-time graph validator |

`kittens-macros` MUST NOT be a normal user dependency. `kittens` re-exports the macro. The proc macro is a host `std` program and may emit `core`-only target code. Public `kittens-core`, `kittens-tokio`, and `kittens-embassy` packages are deferred until an actual dependency-cycle, versioning boundary, or compile-time measurement justifies making agents understand them.

The eventual profile-oriented package shape may be:

| Candidate package | Role | Status |
|---|---|---|
| `kittens-core` | `no_std`, preferably no-alloc semantic kernel and source contracts | architectural target; package split deferred |
| `kittens-macros` | host-side parser, topology validator, and lowering compiler | required for the macro path, package already implied by K0 |
| `kittens-tokio` | Tokio source/runtime integration | first production integration, exact package boundary provisional |
| `kittens-embassy` | Embassy/ESP-HAL source integration | post-K0 hypothesis |
| `kittens-tui` | terminal/input/render-acknowledgement profile | post-K0 profile hypothesis |
| `kittens-embedded` | interrupt/DMA/cadence/power profile | post-K0 profile hypothesis; exact hardware adapters separately gated |
| `kittens-agent` | model/tool/approval/checkpoint profile | post-K0 profile hypothesis |

This table describes dependency direction, not a release promise. A single `kittens` facade may re-export several of these layers when that improves first-time agent comprehension; separate packages are justified only by actual target/dependency or compile-time pressure.

### 10.2 Candidate public modules

| Module | Candidate eventual responsibility |
|---|---|
| `kittens::reactor` | no-alloc `Control`, phase semantics, and generated local polling support; no rendering gate in K0 |
| `kittens::source` | core source contracts, readiness, draining, backlog probes, dynamic dormancy, and clearly named runtime-adapter submodules |
| `kittens::source::tokio` | Tokio channels, cancellation, deadlines, and retained-operation adapters; first production integration |
| `kittens::source::embassy` | possible future Embassy/HAL adapters after cancellation/pinning evidence; not K0 |
| `kittens::scope` | Tokio-owned `Send + 'static` tasks, cooperative cancellation, grace, abort, and drain; not claimed executor-neutral |
| `kittens::resource` | bracket-like cooperative acquire/use/release with bounded guarantees |
| `kittens::cap` | authority bootstrap, concrete capabilities, narrowing, revocation, consumed approvals |
| `kittens::flow` | small restoration/error helpers and guidance for app-owned typestate; no indexed computation type |
| `kittens::sim` | scripted sources, traces, replay, fault injection, paused-time helpers behind `test-util` |
| `kittens::tokio` | possible non-source Tokio integration such as external-token observation and process/Tower backends; not a prelude |
| `kittens::escape` | reason-bearing unchecked source, detached spawn, and non-structural blocking work |
| `kittens::profile::tui` | eventual TUI-specific sources and presenter/writer integration; not a widget or renderer framework |
| `kittens::profile::embedded` | eventual interrupt, DMA-completion, cadence, and low-power integration; not a HAL or display driver |
| `kittens::profile::agent` | eventual model/tool/approval/checkpoint sources and protocols; not an LLM SDK |

`kittens::protocol` is not a v0.1 public module. Binary endpoint typestate is recommended as an application-owned pattern. A future optional package may be added only after the protocol benchmark in section 34.

### 10.3 Candidate toolchain and dependency policy

The exact MSRV, dependency floors, and feature matrix are not frozen. The kernel begins on current stable and records which parts compile on Rust 1.85. Deferring `scope`, capabilities, Tower, process support, public simulation, tracing, serialization, and Embassy adapters removes their dependencies from the first slice. An MSRV is published only after the selected grammar/expansion is known.

- MSRV: Rust 1.85.0, edition 2024. This is the first stable release with async closures used by the scope entry API.
- Current-stable CI: Rust 1.97.1 at specification time; CI MUST track subsequent stable releases.
- Core target: candidate `#![no_std]` with no allocator requirement for `reactor`, `source` contracts, markers, and `Control`; this is a K0 compile gate, not a claim of shipped MCU support.
- First runtime integration: Tokio `1.53.1` or any semver-compatible `1.x` selected by Cargo, enabling only features used by K0 adapters/tests. Filesystem, I/O, process, Tower, and multi-thread scope features are not base-kernel dependencies.
- Cancellation substrate: tokio-util `0.7.19` or semver-compatible `0.7.x` with its `rt` feature.
- Filesystem capability substrate: cap-std `4.0.2` or semver-compatible `4.x`.
- Macro implementation: `syn 2`, `quote 1`, `proc-macro2 1`, and `proc-macro-crate 3` major lines. The macro MUST resolve a renamed `kittens` dependency correctly.
- Optional integration: `tracing 0.1`, `serde 1`, and `tower 0.5` behind features.
- Dev-only: `trybuild`, Tokio test utilities, and expansion snapshots. Loom is dev-only for internal synchronization models.
- K0 MUST run on Tokio current-thread and multi-thread runtimes. The core-poll candidate MUST also compile without `std` or `alloc`; an Embassy executor/backend is deliberately not an acceptance requirement.

Minimum compatible dependency versions MUST be tested. The implementation lockfile MUST record exact versions at implementation start; this specification intentionally names supported semver lines rather than pretending a transitive lockfile is an API contract.

### 10.4 Candidate features

| Feature | Default | Effect |
|---|---:|---|
| `macros` | yes | exports `reactor!` |
| `tokio` | yes for the first release candidate | enables first-party Tokio source adapters; disabling it leaves the candidate no-std/no-alloc kernel |
| `tracing` | no | emits structured lifecycle/reactor events |
| `serde` | no | serializes Kittens trace/test record types, never authority proof; application state tags remain application-owned |
| `tower` | no | request-boundary adapters only |
| `test-util` | no | scripted sources, paused-time helpers, trace/replay |
| `process` | no | Tokio process-backed reference shell executor; generic shell capability remains available without it |

Core source-admission and topology semantics MUST NOT change when a runtime integration or observability feature is toggled. Runtime-specific source availability may change explicitly through feature-gated modules.

## 11. Superseded maximal public API hypothesis

This section preserves the detailed surface proposed before the preimplementation challenge review. It is useful as a comparison condition and a record of decisions, but it does not fix the implementation surface. In particular, the eight base source declarations, priority-class DAG, item-position function macro, private event enum, full adapter family, phase permits, `SingleFlight`, scope, timeout, capability, and resource APIs are all provisional. The lean kernel contract in section 37 is the only authorized target after implementation approval.

### 11.1 Maximal `reactor!` grammar candidate

The earlier candidate is a function-like procedural macro invoked inside the module or `impl` position where the emitted function belongs:

```rust
kittens::reactor! {
    async fn run(&mut self, sources: &mut Sources, scope: &Scope)
        -> Result<Exit, AppError>
    {
        policy {
            selection: biased;
            priority {
                Shutdown > Interactive;
            }
            phases: [before_poll, after_event];
        }

        before_poll(_before) {
            self.apply_deferred_work(scope).await?;
            Ok(())
        }

        #[source(cancel)]
        #[priority(Shutdown)]
        #[lifecycle(one_shot)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(protected)]
        #[shutdown]
        _ = sources.cancel => {
            Ok(Exit::Cancelled)
        }

        #[source(input)]
        #[priority(Interactive)]
        #[lifecycle(repeating)]
        #[cancellation_safe]
        #[readiness(may_remain_ready)]
        #[close(dormant)]
        #[starvation(protected)]
        event = sources.input => {
            self.handle_input(event).await?;
            Ok(kittens::reactor::Control::Continue)
        }

        after_event(present) {
            self.present_if_ready(present).await?;
            Ok(())
        }
    }
}
```

The v0.1 parser MUST accept a normal Rust visibility, `async fn` signature, generics, `where` clause, arguments, and a return type that rustc can unify with `Result<Exit, E>`. It MUST accept invocation in an `impl` so `self` works normally. It MUST preserve user spans and attributes unrelated to Kittens on the emitted function.

Inside the function body the grammar is deliberately restricted:

1. exactly one `policy` block;
2. an `initialize(binding)` block if named by `phases`;
3. a `before_poll(binding)` block if named by `phases`;
4. one or more source arms;
5. an `after_event(binding)` block if named by `phases`.

The policy and phase declarations are not arbitrary Rust statements. Source and handler expressions are ordinary Rust. A source expression MUST be a persistent place expression consisting of a path, field access, or parenthesized form of those; calls and temporary source construction are rejected. Sources are constructed before entering the reactor.

The left side of an arm MUST be `_` or a single identifier in v0.1. Refutable patterns are rejected so Tokio pattern disabling cannot silently alter branch availability. The handler is wrapped in an internal async block, which means `?` and `return` are captured as the handler result and cannot bypass `after_event`.

### 11.2 Policy semantics

`selection: biased` is the only accepted v0.1 selection policy. Keeping the keyword makes the choice reviewable and leaves room for a future policy without changing the macro form. Omitting it is `KTR012`.

Each line in `priority` is a chain of directed class edges. Multiple lines form a directed acyclic graph. Every priority class used by a branch MUST occur in the graph, including an isolated class written as `Class;`. Every declared class MUST contain at least one source; an empty/stale class is rejected as `KTR012` rather than pretending to encode reachable policy.

The macro validates the lexical arm order as follows:

1. apply priority-class edges to all member sources;
2. add each `#[before(id)]` source edge;
3. add an edge from every `#[shutdown]` source to every non-shutdown source;
4. add global edges into a `#[last]` source and same-class edges into `#[last(within = Class)]`;
5. reject cycles;
6. require the written arm order to satisfy every resulting edge;
7. emit arms in exactly the order written.

The macro MUST NOT silently reorder arms. This keeps source text, diagnostics, expansion, and biased runtime order aligned. Incomparable nodes still receive lexical precedence at runtime because biased selection is total, but that precedence is not protected against a later legal edit; callers add `#[before]` when it is a correctness invariant. Starvation analysis conservatively uses every lexical predecessor.

Graph traversal is deterministic. Nodes use policy declaration order for classes and lexical declaration order for sources; outgoing edges use their written/generated order. A reported cycle is rotated to start at its earliest node under that order and follows the earliest discoverable closed path. Equivalent builds therefore produce the same `KTR003`/`KTR010` path.

`phases` is an exact set. A named phase MUST appear exactly once; an unnamed phase MUST NOT appear. The v0.1 phase names are `initialize`, `before_poll`, and `after_event`. `initialize` runs once; the other two are per-loop phases. Each phase binds one identifier or `_` to its phase capability.

### 11.3 Source-arm attributes

The maximal candidate required every arm to carry the eight base declarations below. Section 37 rejects `lifecycle`, `cancellation_safe`, and `close` as mandatory kernel syntax because they currently restate sealed adapter facts without contributing a global check. This table is retained for the semantic-verbosity ablation, not as an implementation requirement.

| Attribute | Accepted v0.1 forms | Meaning |
|---|---|---|
| source ID | `#[source(id)]` | unique stable ID; `id` is a Rust identifier |
| priority | `#[priority(Class)]` | membership in one declared priority class |
| lifecycle | `#[lifecycle(repeating)]`, `one_shot`, or `dynamic` | asserted adapter lifecycle marker |
| cancellation | `#[cancellation_safe]` | asserted `RestartSafeSource` contract; no unchecked spelling exists here |
| readiness | `#[readiness(may_remain_ready)]` or `quiescent_after_event` | asserted readiness marker |
| close | `#[close(not_applicable)]`, `dormant`, or `emit_once` | asserted close marker; `emit_once` changes the item to an event enum |
| starvation | `#[starvation(protected)]` or `#[starvation(allowed, reason = "...")]` | protection requirement or explicit risk acceptance |
| source expression | `binding = place => { ... }` | persistent source and handler |

Optional attributes are:

| Attribute | Static and generated behavior |
|---|---|
| `#[shutdown]` | implies terminal; must be protected and have no `when`/yield guard; handler returns `Result<Exit, E>`; generated edges require the shutdown arms to form the leading prefix |
| `#[terminal]` | handler returns `Result<Exit, E>` and successful completion stops the reactor |
| `#[before(other)]` | adds one source precedence edge; repeatable |
| `#[last]` | requires this to be the final lexical arm and adds an edge from every other source; at most one global last source |
| `#[last(within = Class)]` | requires this to be the final lexical member of its own priority class; at most one per class |
| `#[when(expr)]` | evaluates a synchronous Rust guard once before one arbitration, keeps the result fixed across pending executor repolls, and disables this branch when false |
| `#[yields_to(other, when = buffered)]` | disables this source while `other` has backlog and stops this source's drain when backlog appears |
| `#[drain(max = N)]` | processes the selected item and at most `N - 1` immediately available items, handler once per item |
| `#[drain(max = N, mode = batch)]` | collects at most `N` items in `reactor::Batch<T>` and runs the handler once |

`N` MUST be an unsuffixed integer literal in `1..=4096`. The bound is intentionally syntactic so code review and macro diagnostics do not depend on constant evaluation. A future const-expression form is deferred.

`#[starvation(allowed, reason = "...")]` requires a nonempty string literal of at least eight non-whitespace characters. It is not a compiler warning suppression; it is the explicit contract for a best-effort branch. `#[shutdown]` may never use it or any `last` attribute. Terminal and shutdown arms may not declare a drain because their first successful item exits the reactor.

The starvation annotation describes the service guarantee requested for the annotated source: `protected` constrains every dominating predecessor, while `allowed` accepts that this source itself may wait indefinitely. It is not permission for that source to starve others; lower protected sources still independently force a yield/reorder error.

Shutdown-attribute incompatibilities use `KTR005`; a drain on any terminal arm uses `KTR008`.

`#[close(not_applicable)]` is valid only for an adapter with no producer-close condition. `#[close(dormant)]` suppresses closure as an item and leaves the adapter dormant. `#[close(emit_once)]` requires the adapter's item type to expose one typed close/terminal event and then become dormant. The attribute does not decide whether that event stops the reactor; the ordinary handler does. Contract mismatch is anchored by `KTR006`.

For `#[last(within = Class)]`, `Class` MUST exactly match the branch's `#[priority(Class)]`. A global `#[last]` also counts as last within its class. Conflicting last declarations are `KTR004`.

Each source may declare at most one buffered-yield target in v0.1. Yield edges MUST be acyclic: mutual buffered yields could disable every member when both hold backlog. The yield target MUST implement `BacklogProbeSource`. A yield applies both to the initial select guard and between items in either drain mode.

### 11.4 Handler and phase result rules

A nonterminal handler MUST type-check as:

```rust
Result<kittens::reactor::Control<Exit>, E>
```

where:

```rust
pub enum Control<T> {
    Continue,
    Stop(T),
}
```

A `#[terminal]` or `#[shutdown]` handler MUST type-check as `Result<Exit, E>`; the macro maps success to `Control::Stop`. This prevents a successfully handled shutdown event from accidentally continuing.

`initialize`, `before_poll`, and `after_event` MUST type-check as `Result<(), E>`. Their async blocks may borrow the reactor arguments normally.

`initialize(binding)` and `after_event(binding)` bind `&mut reactor::PresentPermit<'_>`. `before_poll(binding)` binds `&mut reactor::BeforePoll<'_>`. These types have private fields, no supported public constructor, and are neither `Clone` nor `Copy`. Application APIs MAY require them to make a phase restriction compiler-visible. In particular, Kittens' single-flight submission can begin only through `PresentPermit::try_begin`; handlers can request or acknowledge a frame but cannot submit one through the supported gate API.

For example, an application method that must run only at loop top can accept `&mut BeforePoll<'_>`. The macro cannot inspect arbitrary method effects, so phase restriction is compile-time enforced only for APIs that require the corresponding capability value.

The proc macro necessarily reaches doc-hidden cross-crate plumbing to construct phase values. Calling `kittens::__private` directly is unsupported, outside semantic-versioning guarantees, and treated like bypassing Kittens with a raw writer; phase permits are misuse prevention for supported code, not an adversarial security boundary.

Execution order is normative:

1. run `initialize` once if declared; if it returns `Err`, return immediately;
2. run `before_poll` if declared;
3. if it returns `Err`, return that error without polling or running `after_event`;
4. select one event;
5. run its handler once, or once per item for an `each` drain;
6. stop a drain immediately on handler `Err` or `Control::Stop`;
7. if a handler returned `Control::Stop`, return its exit value without `after_event`;
8. if every executed handler returned `Ok(Control::Continue)`, run `after_event` exactly once if declared;
9. if `after_event` returns `Err`, return that error; otherwise start the next iteration.

On handler `Err`, `after_event` is not run. On panic or process abort no phase guarantee applies. Resource and scope shutdown are owned by their enclosing APIs, not smuggled into reactor expansion.

A drained batch is one hook unit. If a later item returns `Err` or `Control::Stop`, `after_event` is skipped for the whole batch even though earlier items completed; applications that require per-item commit/render semantics MUST omit draining or make the handler accumulate a nonterminal result as ordinary state.

### 11.5 Source trait family

**Superseded by sections 14 and 37:** the embedded pass invalidated `RestartSafeSource` as a name and showed that `Unpin`, waiter reconstruction, Drop cleanup, and external repeat safety must not be collapsed. The code below is retained only as the maximal earlier hypothesis and MUST NOT be implemented as written. K0 compares a `Pin<&mut Self>`-style poll boundary with simpler `Unpin` adapters and judges the result by both fixtures and diagnostics.

The public bounds use familiar traits, but semantic marker implementation is sealed:

```rust
pub trait Source: Unpin {
    type Item;
    type Lifecycle: source::Lifecycle;
    type Readiness: source::Readiness;
    type Close: source::CloseBehavior;

    #[doc(hidden)]
    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Self::Item>;
}

pub trait RestartSafeSource: Source + source::sealed::RestartSafe {}

pub trait DrainableSource: RestartSafeSource {
    fn try_next(&mut self) -> TryNext<Self::Item>;
}

pub enum TryNext<T> {
    Item(T),
    Empty,
    Dormant,
    Rearmed,
}

pub trait BacklogProbeSource: RestartSafeSource {
    fn has_backlog(&self) -> bool;
}
```

`has_backlog` is a non-consuming readiness hint used only for buffered yields. It MUST return true when the adapter already owns at least one immediately selectable item, including a pending one-time `Closed`/terminal item under `close::Emit`; after that close item is consumed and the source becomes dormant it returns false. It may become stale immediately because an external producer can race the probe, so the guarantee is bounded responsiveness—at most the already selected higher event plus its bounded handler/drain work—not prediction of future arrivals.

`DrainableSource` internally remembers the armed generation that produced the most recent selected `poll_next` item. `try_next` may drain only that generation: no immediately available item (or no active selected generation) is `Empty`, a terminally inactive source is `Dormant`, and explicit disarm/replace/rearm since selection is `Rearmed`. All three stop the current batch. This prevents an `each` handler from accidentally mixing a newly installed receiver into the batch selected from its predecessor.

The exact sealing layout is private. External types MAY implement `Source` for one-off interoperability but cannot implement the sealed approval required by `reactor!`. They must use a reviewed Kittens adapter, isolate production behind a scoped channel, or invoke `escape::unchecked_source`.

`poll_next` stores ongoing operation state in the persistent source. The short-lived waiter created for `select!` only borrows the source. Losing the race drops that waiter, not the persistent source or any owned in-progress one-shot future.

The generated contract assertion checks exact marker equality for lifecycle, readiness, and close behavior, plus `RestartSafeSource`. A branch cannot declare `quiescent_after_event` for an mpsc adapter merely to bypass starvation validation or declare a close event for an adapter that silently dormants.

### 11.6 Curated source adapters

The following v0.1 adapters are REQUIRED:

| Constructor/type | Item | Lifecycle | Readiness | Close behavior | Extra contracts |
|---|---|---|---|---|---|
| `source::cancellation(token)` | `()` | one-shot | quiescent | not applicable | disarms after firing; restart-safe |
| `source::notify(notify)` | `()` | repeating | may remain ready | not applicable | restart-safe |
| `source::mpsc(rx, close::Dormant)` | `T` | repeating | may remain ready | silently dormant after close | bounded or unbounded Tokio receiver; drainable, backlog probe |
| `source::mpsc(rx, close::Emit)` | `ChannelEvent<T>` | repeating | may remain ready | emits `Closed` once, then dormant | bounded or unbounded Tokio receiver; drainable, backlog probe |
| `source::watch(rx, close_policy)` | `WatchEvent<T>` | repeating | may remain ready | explicit policy | backlog semantics unavailable |
| `source::deadline(at)` | `Instant` | one-shot | quiescent | not applicable | disarms after firing; restart-safe |
| `source::OptionalDeadline` | `Instant` | dynamic | quiescent | not applicable | arm/disarm/set; disarms before event |
| `source::one_shot(future)` | `F::Output` | one-shot | quiescent | not applicable | owns and retains future across lost races |
| `source::OptionalOneShot<F>` | `F::Output` | dynamic | quiescent | not applicable | explicit cancel/replace; dormant after completion |
| `source::OptionalMpsc<T, C>` | policy-dependent | dynamic | may remain ready | dormant when absent or closed | drainable, backlog probe |
| `source::OptionalWatch<T, C>` | `WatchEvent<T>` | dynamic | may remain ready | dormant when absent or closed | restart-safe, not backlog-probeable |
| `source::interval(interval, missed_tick)` | `Instant` | repeating | may remain ready for every policy | not applicable | explicit missed-tick policy required; readiness marker stays conservative |
| `source::channel_task(scope, name, capacity, producer)` | `ProducerEvent<T, E>` | repeating | may remain ready | emits producer completion once, then dormant | scope-owned isolation adapter |
| `scope::TaskEvents<T>` | `TaskCompletion<T>` | repeating | may remain ready | dormant after group close and completion drain | typed task-group result source; drainable, backlog probe |
| `sim::ScriptedSource<T, L, R, C>` | policy-dependent | marker parameters from typed contract constructor | marker parameter | marker parameter | test-only; script operations checked at runtime |

There is no bare `source::optional(future_or_pending)` helper. Dynamic adapters own their dormant state.

`close::Dormant` and `close::Emit` are zero-sized policy values so the item type and close contract remain static. `ChannelEvent<T>` has exactly `Item(T)` and `Closed` variants.

`source::mpsc` is generic over a sealed receiver facade implemented for Tokio's bounded `mpsc::Receiver<T>` and `mpsc::UnboundedReceiver<T>`; it is one constructor, not two aliases. `OptionalMpsc` supports the same receiver forms while keeping one concrete receiver kind per source value. The unbounded form is required for a synchronous terminal-reader thread that cannot await capacity; the async `source::channel_task` repair remains bounded by design.

The Notify adapter MUST accept `Arc<tokio::sync::Notify>` and retain an owned notification waiter across lost races; it MUST NOT reconstruct the fairness-queued borrowed waiter on each poll. Cancellation and one-shot future adapters similarly retain owned/pinned operation state whenever the underlying primitive's documented cancellation contract is insufficient.

Dynamic adapters expose one canonical control surface:

```rust
let mut voice = source::OptionalMpsc::new(close::Dormant);
voice.arm(receiver)?;                  // fails and returns receiver if already armed
let old = voice.disarm();              // returns the live receiver, if any
let old = voice.replace(receiver);     // explicit replacement; returns old receiver

let mut appearance = source::OptionalWatch::new(close::Dormant);
appearance.arm(watch_receiver)?;

let mut draw_deadline = source::OptionalDeadline::new();
draw_deadline.set(presenter.deadline()); // Some arms/rearms; None disarms
```

For `OptionalOneShot`, replacing or disarming drops an in-progress future and therefore uses the visibly named `cancel_and_replace` or `cancel_and_disarm` methods with a `CancelReason`; plain `replace` is not provided. The adapter retains its future across ordinary lost select races.

All dormant implementations MUST return `Poll::Pending` without self-waking. `arm`, `set`, and `replace` require exclusive access and therefore occur between polls. External code that wants to arm a source MUST send a control event through another wake-capable source; v0.1 does not expose a concurrent optional-source control handle.

`source::channel_task` is the canonical repair for a producer whose repeated await is not approved for cancellation/restart. It explicitly spawns the producer through the supplied `Scope`, owns the resulting `Task` together with a bounded receiver, and exposes one `ProducerEvent` stream. Dropping the source requests producer cancellation; the scope still tracks and drains it. The function name and returned source make the spawn visible—`reactor!` itself never spawns. The producer receives a bounded sender plus a cancellation observer and MUST own its I/O resource.

Its conceptual signature and terminal ordering are:

```rust
pub fn channel_task<T, E, F, Fut>(
    scope: &Scope,
    name: impl Into<TaskName>,
    capacity: NonZeroUsize,
    producer: F,
) -> Result<ChannelTaskSource<T, E>, SpawnError>
where
    F: FnOnce(BoundedSender<T>, Cancellation) -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), E>> + Send + 'static,
    T: Send + 'static,
    E: Send + 'static;

pub enum ProducerEvent<T, E> {
    Item(T),
    Finished,
    Failed(E),
    TaskFailed(TaskJoinError),
}
```

Items already accepted by the bounded sender are delivered before the single terminal producer event. The source then becomes dormant. A panic, Tokio cancellation, or abort is `TaskFailed`; an ordinary producer `Err(E)` is `Failed(E)`.

`BoundedSender::send(item)` retains ownership of `item`, awaits channel capacity without racing/reconstructing Tokio's fairness-queued send waiter, and returns the item if the receiver closes. Cooperative cancellation is observed before starting a send and after that send resolves; the capacity wait is cancellation-deferred. Hard abort after scope grace may still drop the unsent item, which is reported as task abort rather than successful delivery.

`BoundedSender<T>` is non-`Clone` in v0.1, so normal producer completion determines when the item stream has closed and the terminal `ProducerEvent` may be emitted. Moving it into a raw detached task or forgetting it bypasses this ordering guarantee; concurrent producers use separately scoped producers and an application merge source rather than hidden sender clones.

### 11.7 Bounded batching

This is the superseded maximal drain hypothesis. K0 omits collected batches and `Rearmed` generation handling; section 37 admits per-item drain only for stable, non-rearmable installed adapters.

`reactor::Batch<T>` is a nonempty, bounded collection created only by the macro. Its public operations are `len`, `is_full`, `first`, `iter`, and consuming iteration. It MUST NOT expose a public unbounded constructor.

For `mode = each`, the macro MUST avoid allocation and invoke the same handler block for each item. For `mode = batch`, it MAY use a `Vec<T>` preallocated to at most `N`, but MUST never grow beyond `N`.

Draining uses only `DrainableSource::try_next`; it never awaits. The first selected item counts toward `N`. `after_event` runs once after the complete successful batch, which deliberately supports render coalescing.

For `mode = each`, source borrows end before every handler invocation and are reacquired only for the next `try_next`. A handler may therefore rearm any dynamic source, including the selected source; doing so makes the next drain probe return `Rearmed` and ends the batch. For `mode = batch`, all drain probes occur before the single handler. Neither mode crosses a dynamic source generation.

### 11.8 Maximal Grok migration candidate

The declaration below is a deliberately maximal Kittens migration exercise. It is not the behavior-faithful Grok fixture required by section 37: it changes terminal draining, task ownership, source wrappers, and rendering structure. The first implementation MUST maintain two separate fixtures—a fidelity oracle that preserves the observed Grok behavior and a Kittens migration candidate whose changes are measured rather than silently counted as captured behavior.

The following is the required design exercise, not a claim that Grok itself uses Kittens. Before entering it, the caller creates `scope.task_group::<TaskResult>("effects")`, stores the returned `TaskEvents<TaskResult>` in `GrokSources`, and passes the paired spawner as `effects`. This is the typed equivalent of Grok's homogeneous `JoinSet<TaskResult>`. The exercise retains all 23 current source classes and makes current ordering assumptions explicit. It also strengthens terminal-input protection by making every may-ready predecessor yield while input is buffered.

```rust
kittens::reactor! {
    async fn run(
        &mut self,
        sources: &mut GrokSources,
        scope: &Scope,
        effects: &TaskSpawner<TaskResult>,
    )
        -> Result<LoopExit, AppError>
    {
        policy {
            selection: biased;
            priority {
                Shutdown > Control > Stream > Interactive;
                Interactive > Render > Timers > Watchers > Background;
            }
            phases: [initialize, before_poll, after_event];
        }

        initialize(present) {
            self.presenter.request(RenderRequest::Initial)?;
            self.presenter
                .present_if_ready(present, &mut self.writer)
                .await?;
            Ok(())
        }

        before_poll(_before) {
            self.run_pending_terminal_handoffs(scope).await?;
            self.ensure_voice_pipeline(scope).await?;
            self.enforce_voice_session_ownership().await?;
            self.sync_keyboard_layer()?;
            self.rearm_roster_and_subscription(sources)?;
            sources.resize.set(self.resize_deadline());
            sources.deferred_render.set(self.presenter.deadline());
            sources.suspend_retry.set(self.suspend_retry_deadline());
            Ok(())
        }

        #[source(connection_cancel)]
        #[priority(Shutdown)]
        #[lifecycle(one_shot)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(protected)]
        #[before(graceful_quit)]
        #[shutdown]
        _ = sources.connection_cancel => {
            Ok(LoopExit::Disconnected)
        }

        #[source(graceful_quit)]
        #[priority(Shutdown)]
        #[lifecycle(one_shot)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(protected)]
        #[before(writer_event)]
        #[shutdown]
        _ = sources.graceful_quit => {
            self.dispatch_quit().await?;
            Ok(LoopExit::Quit)
        }

        #[source(writer_event)]
        #[priority(Control)]
        #[lifecycle(repeating)]
        #[cancellation_safe]
        #[readiness(may_remain_ready)]
        #[close(emit_once)]
        #[starvation(protected)]
        #[yields_to(terminal_input, when = buffered)]
        event = sources.writer_events => {
            let control = self.handle_writer_event(event)?;
            Ok(control)
        }

        #[source(acp_stream)]
        #[priority(Stream)]
        #[lifecycle(repeating)]
        #[cancellation_safe]
        #[readiness(may_remain_ready)]
        #[close(emit_once)]
        #[starvation(allowed, reason = "model streaming may wait behind control events")]
        #[yields_to(terminal_input, when = buffered)]
        #[drain(max = 32)]
        #[before(task_completion)]
        #[when(self.session_accepts_acp())]
        message = sources.acp_stream => {
            let control = self.handle_acp_event(message, effects).await?;
            Ok(control)
        }

        #[source(task_completion)]
        #[priority(Stream)]
        #[lifecycle(repeating)]
        #[cancellation_safe]
        #[readiness(may_remain_ready)]
        #[close(dormant)]
        #[starvation(allowed, reason = "effect completions may wait behind model streaming")]
        #[yields_to(terminal_input, when = buffered)]
        #[before(restore_progress)]
        completion = sources.task_events => {
            self.handle_task_completion(completion).await?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(restore_progress)]
        #[priority(Stream)]
        #[lifecycle(repeating)]
        #[cancellation_safe]
        #[readiness(may_remain_ready)]
        #[close(dormant)]
        #[starvation(allowed, reason = "restore progress is informational")]
        #[yields_to(terminal_input, when = buffered)]
        #[before(background_update)]
        progress = sources.restore_progress => {
            self.handle_restore_progress(progress)?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(background_update)]
        #[priority(Stream)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "update completion is best effort")]
        update = sources.background_update => {
            self.finish_background_update(update).await?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(terminal_input)]
        #[priority(Interactive)]
        #[lifecycle(repeating)]
        #[cancellation_safe]
        #[readiness(may_remain_ready)]
        #[close(emit_once)]
        #[starvation(protected)]
        #[drain(max = 256, mode = batch)]
        events = sources.terminal_input => {
            let control = self.coalesce_and_dispatch_input(events, effects).await?;
            Ok(control)
        }

        #[source(resize_deadline)]
        #[priority(Render)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "resize may wait behind model streaming")]
        _ = sources.resize => {
            self.apply_debounced_resize()?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(deferred_render)]
        #[priority(Render)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "frame throttle deliberately delays drawing")]
        _ = sources.deferred_render => {
            self.presenter.on_deadline();
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(suspend_retry)]
        #[priority(Timers)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "retry only opens next loop handoff")]
        _ = sources.suspend_retry => {
            self.enable_suspend_retry();
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(scroll_clock)]
        #[priority(Timers)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "scroll animation is best effort")]
        _ = sources.scroll_clock => {
            self.advance_scroll()?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(animation_tick)]
        #[priority(Timers)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "animation and recovery are periodic")]
        _ = sources.animation_tick => {
            self.animate_and_reconcile_lost_events(effects).await?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(billing_poll)]
        #[priority(Timers)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "billing refresh is periodic")]
        _ = sources.billing_poll => {
            self.poll_billing(effects)?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(gate_poll)]
        #[priority(Timers)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "access gate refresh is periodic")]
        _ = sources.gate_poll => {
            self.poll_access_gate(effects)?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(subscription_watch)]
        #[priority(Watchers)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "subscription refresh is background work")]
        _ = sources.subscription_watch => {
            self.poll_subscription(effects)?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(roster_poll)]
        #[priority(Watchers)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "roster refresh is dashboard-only")]
        roster = sources.roster_poll => {
            self.apply_roster(roster)?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(recap_poll)]
        #[priority(Watchers)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "away recap is periodic")]
        recap = sources.recap_poll => {
            self.apply_recap(recap)?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(config_watch)]
        #[priority(Watchers)]
        #[lifecycle(repeating)]
        #[cancellation_safe]
        #[readiness(may_remain_ready)]
        #[close(emit_once)]
        #[starvation(allowed, reason = "config reload is background work")]
        config = sources.config_watch => {
            let control = self.handle_config_event(config)?;
            Ok(control)
        }

        #[source(appearance_watch)]
        #[priority(Watchers)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(may_remain_ready)]
        #[close(emit_once)]
        #[starvation(allowed, reason = "appearance changes are background work")]
        appearance = sources.appearance_watch => {
            let control = self.handle_appearance_event(appearance)?;
            Ok(control)
        }

        #[source(leader_status)]
        #[priority(Watchers)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(may_remain_ready)]
        #[close(emit_once)]
        #[starvation(allowed, reason = "leader status is informational")]
        status = sources.leader_status => {
            let control = self.handle_leader_status_event(status)?;
            Ok(control)
        }

        #[source(reconnect_reinit)]
        #[priority(Background)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(quiescent_after_event)]
        #[close(not_applicable)]
        #[starvation(allowed, reason = "reconnect result is generation checked")]
        result = sources.reconnect_reinit => {
            self.finish_reconnect(result, scope).await?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(voice_stt)]
        #[priority(Background)]
        #[lifecycle(dynamic)]
        #[cancellation_safe]
        #[readiness(may_remain_ready)]
        #[close(emit_once)]
        #[starvation(allowed, reason = "interim voice transcripts are best effort")]
        #[last]
        transcript = sources.voice_stt => {
            let control = self.handle_voice_event(transcript)?;
            Ok(control)
        }

        after_event(present) {
            self.presenter
                .present_if_ready(present, &mut self.writer)
                .await?;
            Ok(())
        }
    }
}
```

This version is intentionally longer than the raw select. It turns source ordering, the ACP batch of 32, user-input yield, dynamic timers, background treatment, the last voice source, loop-top work, and post-event rendering into compiler inputs. The source methods shown are application methods and do not imply Kittens ships Grok-specific APIs.

The bounded terminal-input batch of 256 is an intentional proposed migration change, not a description of current Grok. Current Grok drains all immediately available terminal events and has a separate paste-extension iteration cap of 5,000. A real migration MUST benchmark paste correctness, scroll coalescing, and input latency before selecting the Kittens bound; the macro prevents leaving that choice implicit or unbounded.

`graceful_quit` is constructed before the reactor from an owned, retained Notify waiter (or a one-shot channel), not from a temporary `quit_notify.notified()` call. This removes the repeated reconstruction of Tokio's fairness-queued waiter from the select loop.

### 11.9 Provisional single-flight/coalescing API

This API is an extension hypothesis. The runtime state-machine topology is evidence-backed; the generic parameters, merge traits, phase permit, borrowing, poisoning, and error taxonomy are not. The first rendering comparison uses this design only as one contender against an application-owned Grok-like presenter.

The render protocol is generalized as a runtime gate:

```rust
let mut gate = reactor::SingleFlight::new(merge_policy)
    .minimum_interval(frame_interval);

gate.request(RenderRequest::Dirty)?;

if let Some(submission) = present.try_begin(&mut gate, clock.now())? {
    submission.commit_with(|request| writer.queue(request))?;
}

match gate.acknowledge(written_ticket)? {
    reactor::Ack::Accepted => {}
    reactor::Ack::Stale => {}
}

draw_deadline.set(gate.deadline()?);
```

`SingleFlight<Ticket, Merge>` owns pending coalesced request state, an optional in-flight ticket, an optional last-presentation time, an optional throttle deadline, and a healthy/poisoned protocol state. `Ticket: Copy + Ord`; `Merge: MergePolicy` has an associated `Request` type and determines how requests combine. v0.1 supplies `merge::Flag` and `merge::Latest<T>`.

`MergePolicy` is a normal public trait because an incorrect merge is a domain logic bug, not a false scheduler contract. Grok would use an application merge in which full repaint dominates ordinary dirty requests; `Latest<T>` alone would not preserve that sticky requirement.

The permit method is conceptually:

```rust
pub trait MergePolicy {
    type Request;
    fn merge(&mut self, current: &mut Self::Request, next: Self::Request);
}

impl<Ticket, Merge> SingleFlight<Ticket, Merge>
where
    Ticket: Copy + Ord,
    Merge: MergePolicy,
{
    pub fn request(
        &mut self,
        request: Merge::Request,
    ) -> Result<(), GateError<Ticket>>;

    pub fn acknowledge(
        &mut self,
        ticket: Ticket,
    ) -> Result<Ack, GateError<Ticket>>;

    pub fn deadline(&self) -> Result<Option<Instant>, GateError<Ticket>>;
}

impl PresentPermit<'_> {
    pub fn try_begin<'gate, Ticket, Merge>(
        &mut self,
        gate: &'gate mut SingleFlight<Ticket, Merge>,
        now: Instant,
    ) -> Result<Option<Submission<'gate, Ticket, Merge>>, GateError<Ticket>>
    where
        Ticket: Copy + Ord,
        Merge: MergePolicy;
}

impl<'gate, Ticket, Merge> Submission<'gate, Ticket, Merge>
where
    Ticket: Copy + Ord,
    Merge: MergePolicy,
{
    pub fn commit_with<E>(
        self,
        submit: impl FnOnce(&Merge::Request)
            -> Result<SubmitOutcome<Ticket>, E>,
    ) -> Result<SubmitOutcome<Ticket>, CommitError<E, Ticket>>;
}

pub enum SubmitOutcome<Ticket> {
    Accepted(Ticket),
    NoOutput,
}

pub enum Ack {
    Accepted,
    Stale,
}

pub enum GateError<Ticket> {
    NonMonotonicSubmission { previous: Ticket, received: Ticket },
    Poisoned,
}

pub enum CommitError<E, Ticket> {
    Rejected(E),
    Protocol(GateError<Ticket>),
}
```

`SingleFlight::try_begin` is not public. The submission is non-`Clone`, holds only the exclusive gate borrow, and is consumed by `commit_with`, which gives the request by shared reference to a synchronous closure. `SubmitOutcome::Accepted(ticket)` records one in-flight ticket; `NoOutput` records the presentation attempt and clears the request without creating an in-flight ticket, matching Grok's legitimate no-payload draw. A reported `Err` or dropping `Submission` without committing merges the request back into pending state. There is no public “queue asynchronously, then commit a ticket” split because cancellation between external acceptance and recording the ticket would create an unknown in-flight frame. An async writer is isolated behind a channel whose queue/reserve operation is synchronous, as in Grok.

The submission closure has an explicit acceptance-atomic integration contract: `Ok(Accepted(ticket))` means exactly one logical request was synchronously accepted, `Ok(NoOutput)` means no request was accepted and the presentation was nevertheless satisfied, `Err(error)` means nothing was accepted and the request remains pending, and the closure MUST NOT unwind after acceptance. One logical request may enqueue several ordered payloads; its returned ticket MUST be the completion barrier for the last payload, and an acknowledgement at or above it MUST imply that every earlier payload is complete. Kittens cannot prove those sink/order contracts for an arbitrary closure. A writer integration claiming the single-flight guarantee MUST satisfy them and test failure plus multi-payload ordering; Grok's synchronous queue/reserve operation and last queued sequence are the reference shape. The gate remembers the last committed ticket and requires each newly accepted ticket to be strictly greater. If an accepted closure returns a non-monotonic ticket, the gate enters a private poisoned state, retains no claim that the request is pending, rejects future submissions, and returns `CommitError::Protocol(GateError::NonMonotonicSubmission { .. })`; it cannot roll back the already accepted external write. A closure that violates the acceptance, ordering, or unwind contract is outside the single-flight guarantee.

Two simultaneously live permits fail through ordinary mutable borrowing; after successful commit, another call in a later permitted phase compiles but returns `None` until a sufficient acknowledgement arrives.

An acknowledgement lower than the active ticket is stale and does not unlock. An equal or higher ticket unlocks, matching Grok's monotonic writer sequence. Acknowledgement with no ticket is stale. An accepted submission and `NoOutput` both advance the throttle timestamp; rejection/drop does not. Ticket comparison, acceptance atomicity, and writer failure are runtime facts and MUST be traced/tested, not encoded as type parameters.

All tickets supplied to one gate MUST belong to one prefix-ordered writer generation. On writer restart, the application either proves the old in-flight outcome and constructs a fresh gate or uses a generation-bearing `Ticket` whose ordering rejects old acknowledgements. Feeding unrelated ticket domains into one gate is outside the protocol guarantee.

The complete runtime transition contract is:

```text
Idle --request--> Pending --try_begin--> Reserved
Reserved --Accepted(ticket)--> InFlight(ticket)
Reserved --NoOutput--> Idle
Reserved --Rejected/drop--> Pending
InFlight --request--> InFlight + Pending
InFlight --ack >= ticket--> Pending or Idle
InFlight --stale ack--> unchanged
any healthy state --non-monotonic accepted ticket--> Poisoned
Poisoned --any public operation--> GateError::Poisoned
```

`request`, `acknowledge`, `deadline`, and `PresentPermit::try_begin` return `GateError` when poisoned. `try_begin` otherwise returns `None` when there is no pending request, a ticket is in flight, or the minimum interval has not elapsed. `deadline` returns the earliest eligible presentation instant only when a pending request is throttle-blocked and no ticket is in flight; otherwise it returns `None`. Accepted and no-output presentation attempts establish the next interval from the `now` captured in `Submission`.

### 11.10 Provisional scope entry and task API

The canonical scope API is:

```rust
let output = kittens::scope::run(config, async |scope| {
    let research = scope.spawn("research", move |cancel| {
        research_agent(cancel)
    })?;
    let review = scope.spawn("review", move |cancel| {
        review_agent(cancel)
    })?;
    let tests = scope.spawn("tests", move |cancel| {
        test_agent(cancel)
    })?;

    let (research, review, tests) = tokio::try_join!(
        research.join(),
        review.join(),
        tests.join(),
    )?;

    Ok::<_, HarnessError>((research, review, tests))
}).await?;
```

The conceptual public surface is:

```rust
pub async fn run<T, E, F>(config: ScopeConfig, body: F)
    -> Result<T, ScopeRunError<E>>
where
    F: for<'scope> AsyncFnOnce(&'scope Scope) -> Result<T, E>;

impl Scope {
    pub fn spawn<T, F, Fut>(
        &self,
        name: impl Into<TaskName>,
        task: F,
    ) -> Result<Task<T>, SpawnError>
    where
        F: FnOnce(Cancellation) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static;

    pub fn cancellation(&self) -> Cancellation;
    pub fn cancel(&self, reason: CancelReason);

    pub fn task_group<T>(
        &self,
        name: impl Into<TaskName>,
    ) -> Result<(TaskSpawner<T>, TaskEvents<T>), SpawnError>
    where
        T: Send + 'static;
}

impl<T> Task<T> {
    pub async fn join(self) -> Result<T, TaskJoinError>;
    pub async fn cancel(self, reason: CancelReason) -> Result<(), TaskJoinError>;
}

impl Cancellation {
    pub fn is_cancelled(&self) -> bool;
    pub fn reason(&self) -> Option<CancelReason>;
    pub async fn cancelled(&self) -> CancelReason;
}

impl<T> TaskSpawner<T>
where
    T: Send + 'static,
{
    pub fn spawn<F, Fut>(
        &self,
        name: impl Into<TaskName>,
        task: F,
    ) -> Result<TaskId, SpawnError>
    where
        F: FnOnce(Cancellation) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static;

    pub fn close(self);
}

pub struct TaskCompletion<T> {
    pub id: TaskId,
    pub name: TaskName,
    pub outcome: Result<T, TaskJoinError>,
}
```

`Task<T>` is `#[must_use]` and not `Clone`. Its drop requests cancellation of that task but never removes it from scope tracking. `Scope::spawn` accepts only `Send + 'static` closures/futures and supplies the child-specific cancellation observer. `Cancellation` is a cloneable, observer-only value; `cancelled()` is sticky and cancellation-safe because it returns immediately after the first reason has been recorded. Ignoring the observer is legal but means the child receives only hard abort after grace. Borrowed concurrency is achieved by ordinary `join!` of local futures, not by lifetime-erased spawning.

`Scope::task_group` creates one homogeneous result lane before the reactor starts. `TaskSpawner<T>` is non-`Clone` and `#[must_use]`; its `spawn` has the same factory and cancellation rules as `Scope::spawn` but routes the result exclusively to `TaskEvents<T>` and returns only a stable `TaskId`. `TaskCompletion<T>` is also `#[must_use]`, and receiving it is the observation point for that result. Use separate groups or an application enum for heterogeneous work.

Consuming or dropping the spawner closes that group's spawn gate but does not detach or cancel already registered tasks. Dropping `TaskEvents<T>` closes the group, rejects later group spawns, requests cancellation of its remaining tasks, and marks buffered/unreceived results unobserved; the parent scope still drains them. The source becomes dormant only after the spawner/group is closed, every group task has terminated, and every queued completion has been delivered. A reactor that joins every result directly does not create a task group.

### 11.11 Provisional timeout API

Kittens timeout creates a nested structural boundary:

```rust
let outcome = parent.with_timeout(
    Duration::from_secs(30),
    ScopeConfig::default(),
    async |child| run_tool_session(child, approved, &shell),
).await?;
```

Its conceptual signature is:

```rust
impl Scope {
    pub async fn with_timeout<T, E, F>(
        &self,
        limit: Duration,
        config: ScopeConfig,
        body: F,
    ) -> Result<TimeoutOutcome<T, E>, ScopeRunError<E>>
    where
        F: for<'child> AsyncFnOnce(&'child Scope) -> Result<T, E>;
}

pub enum TimeoutOutcome<T, E> {
    Completed(T),
    TimedOut {
        body: BodyAfterCancel<E>,
        shutdown: ShutdownReport,
    },
}

pub enum BodyAfterCancel<E> {
    NotStarted,
    Completed,
    Failed(E),
    AbandonedAfterGrace,
}
```

Before the deadline, body errors, parent cancellation, and structural shutdown failures remain `ScopeRunError<E>` rather than being disguised as timeout values. `TimedOut` is produced only when the nested deadline is the terminating cause and only after cooperative cancellation, grace, abort of remaining async children, and completion draining. `BodyAfterCancel` records whether the body never started, completed without returning its now-late value, failed during cooperative cleanup, or had to be dropped at grace expiry; this preserves a release error without reclassifying a late result as success. The shutdown report may contain expected aborts after grace and MUST be inspected. Both enums are `#[must_use]`. A timeout does not mean every external side effect was rolled back.

The nested driver gives an already-observed parent cancellation precedence over its deadline, and the deadline precedence over a body completion observed in the same poll cycle. A zero duration therefore yields `TimedOut { body: NotStarted, .. }` without polling the body. This ordering is deterministic and covered by paused-time tests.

### 11.12 Provisional capability and approval API

Bootstrap is an explicit process-edge operation:

```rust
let root = cap::Bootstrap::from_process(
    config,
    AuditReason::new("main trust boundary")?,
)?;
let read_workspace = root.read_workspace(workspace_root)?;
let shell = root.shell(shell_policy)?;

let child_read = read_workspace.narrow("research/corpus")?;
let worker = scope.spawn("reader", move |cancel| async move {
    analyze_files(&child_read, cancel).await
})?;
```

The worker receives no `Network` or `Shell` value. Kittens-mediated APIs for those effects are therefore unavailable to it.

Approval is both consuming and target-bound:

```rust
let run = Run::<Proposed>::new(command);
let run = run.validate(&policy).await?;
let (run, approval) = run.request_approval(&human_gate).await?;
let finished = run.execute(&shell, approval).await?;
```

`execute` exists only on `Run<Approved>`. `Approval<CommandExecution>` is private-field, non-`Clone`, non-`Copy`, non-serializable, and consumed by execution. Execution also compares its runtime action digest, principal, policy version, and expiry with the run before performing the side effect.

### 11.13 Provisional resource API

The canonical bracket-like API is explicit about cooperative cancellation and cleanup budget:

```rust
let result = resource::using(
    scope.cancellation(),
    ResourcePolicy::new(Duration::from_secs(2)),
    || async { acquire_session().await },
    |session| async { use_session(session).await },
    |session, exit| async { session.close(exit).await },
).await?;
```

All three callbacks are required. `using` owns the acquired resource, gives the use callback `&mut R`, and consumes `R` into the release callback. Detailed behavior is fixed in section 15.

### 11.14 Research guidance: application-owned typed protocol pattern

Full session typing is deferred, but Kittens recommends ordinary endpoint typestate for short binary protocols:

```rust
let worker = Worker::<Ready>::new(tx, rx);
let worker = worker.send_task(task).await?;       // Worker<Waiting>
let (worker, report) = worker.recv_report().await?; // Worker<Ready>
worker.shutdown().await?;
```

`recv_report` does not exist for `Worker<Ready>`, and `send_task` does not exist for `Worker<Waiting>`. This pattern uses ordinary consuming methods and channels; no Kittens protocol DSL is required.

### 11.15 Provisional escape hatch

Every unchecked bypass is visibly namespaced and reason-bearing:

```rust
let source = kittens::escape::unchecked_source(
    AuditReason::new("vendor future audited against release 4.2")?,
    declared_contract,
    vendor_source,
);

let handle = kittens::escape::spawn_detached(
    AuditReason::new("process-wide crash reporter owns shutdown")?,
    crash_reporter(),
);
```

These calls are safe Rust because violating their promises is a logic error rather than automatically undefined behavior. They MUST emit an audit event when tracing is enabled and MUST retain caller location with `#[track_caller]`.

## 12. Candidate extension semantics: ownership and lifetimes

Sections 12 through 24 preserve the full-architecture semantics considered during research. Except where section 37 explicitly imports a principle, they are not kernel deliverables or frozen public contracts. Each subsystem requires an independent graduation slice against an ordinary Rust/Tokio baseline. Three subsections are explicit exceptions with K0-normative force: 20.2 (generated arbitration semantics, including the always-enabled pending sentinel in the Tokio control expansion), 20.2.1 (cooperative scheduling budget), and 20.2.2 (handler panic and reactor state). Section 37 references them rather than restating them in full; a conflict is resolved in section 37's favor per section 0.

### 12.1 Persistent sources

Source values MUST outlive every short-lived select waiter. Applications SHOULD put them in a dedicated `Sources` struct rather than rebuilding futures from fields on the main application object. This has three purposes:

- losing a race drops only a waiter;
- dynamic dormant/armed/closed state has a single owner;
- the select can mutably borrow disjoint source fields, release those borrows, and then let handlers mutate application state.

v0.1 approved sources MUST be `Unpin` at their public boundary. An adapter containing a `!Unpin` future or Tokio timer MUST pin it internally. The application never handles source pins.

The macro MUST borrow every source only for selection and individual bounded-drain probes. All select borrows end before a handler. In `mode = each`, the selected source is reborrowed only between handlers; in `mode = batch`, its borrow ends before the batch handler. A handler MAY therefore rearm any dynamic source. The selected source's retained drain generation then prevents the current batch from crossing that rearm. All borrows end before `after_event` and before the next iteration.

### 12.2 Owned child tasks

Every `Scope::spawn` child future and output MUST be `Send + 'static`. This means a child owns or shares everything it uses. Kittens MUST NOT use unsafe lifetime erasure to offer borrowed spawned tasks.

Local borrowed concurrency remains normal Rust:

```rust
let (plan, policy, context) = tokio::try_join!(
    build_plan(&request),
    load_policy(&request),
    retrieve_context(&request),
)?;
```

Those futures cannot silently outlive their enclosing async frame because they are not spawned. Use `Scope::spawn` only when independent scheduling/task identity is required.

### 12.3 Handle ownership

- `Task<T>`, `Approval<A>`, `Submission`, and owned protocol endpoints MUST NOT implement `Copy` or `Clone`.
- `Task<T>` is the typed result handle; the scope holds separate structural ownership of the underlying Tokio task.
- Dropping `Task<T>` requests task cancellation and marks its result unobserved. It does not detach, start an independent timer, or remove the task from the scope registry.
- `Task::join(self)` consumes the handle. Dropping that join future also drops the handle and therefore requests cancellation.
- `Task::cancel(self, reason)` requests cancellation, applies the owning scope's cooperative grace followed by abort if needed, and awaits its terminal drainage.
- Direct `Task<T>` results are observed only by `join`; task-group results are observed only by consuming `TaskCompletion<T>` from their typed `TaskEvents<T>` lane. Structural trace records never substitute for either path.

### 12.4 Forget and abort boundary

`mem::forget`, process abort, runtime destruction, and `SIGKILL` can defeat destructors. Kittens MUST state this rather than use the word “guaranteed” without qualification. Safe Rust ownership makes accidental forget uncommon; it cannot make it impossible.

## 13. Structured concurrency semantics

### 13.1 Scope state machine

A scope has private runtime states:

```text
Open ──body finishes / cancel──▶ Closing ──grace expires──▶ Aborting
  │                                  │                         │
  └────all tasks finish──────────────┴────────drain────────────┘
                                      ▼
                                    Closed
```

The states are runtime values because task completion and time are dynamic. Public methods enforce which actions are available where:

- direct and task-group `spawn` succeed only in `Open` and in an open group;
- the first cancellation reason wins and moves the scope toward `Closing`;
- `Closing` rejects new spawns with `SpawnError::ScopeClosing`; the task factory is synchronously dropped without invocation;
- `Aborting` invokes Tokio abort handles for remaining async tasks;
- `Closed` means the registry is empty and all Tokio completion records have been observed.

The internal state MUST be private. Tests MUST model its synchronization under Loom.

Each direct or task-group spawn first reserves a task ID and every required registry slot atomically while the scope/group is open, then invokes the factory outside registry locks, and finally installs/spawns the returned future. A reservation guard removes all slots if factory invocation panics. Closing waits for or cancels every reserved/installed slot, so concurrent close cannot lose a child between gate check and Tokio spawn.

The factory MUST only construct and return its future; it must not block or perform external side effects. This cannot be type-checked. A blocking factory can delay scope close before Tokio owns the child and is a documented application bug.

### 13.2 `scope::run` exit algorithm

`scope::run` MUST perform this algorithm on every normal Rust return from its body, whether `Ok` or `Err`:

1. close the spawn gate atomically;
2. record `ScopeExit::BodyComplete` or `BodyError`;
3. cancel every remaining child through its child cancellation handle;
4. continue polling task completion for `cooperative_grace`;
5. if tasks remain when grace expires, abort them;
6. drain every completion/JoinError until the registry is empty;
7. run registered synchronous bookkeeping destructors;
8. return the body outcome plus a `ShutdownReport` when it contains noteworthy cancellation, panic, or abort data.

`Scope::cancel(reason)` closes the spawn gate, records the first reason, signals the body observer and every child, and wakes the `scope::run` driver. The method is synchronous and idempotent; later reasons are traced but do not replace the first. Cancellation then follows a separate normative path:

1. continue polling the body and child completions during one shared `cooperative_grace` interval so cancellation-aware work and resource release can finish;
2. record a body result that arrives during grace as `BodyAfterCancel::Completed` or `Failed(E)`, but do not turn it back into ordinary success because cancellation won;
3. at grace expiry, drop an unfinished body future and record `AbandonedAfterGrace`;
4. abort remaining async children when grace expires;
5. drain every child completion until the registry is empty;
6. return `ScopeRunError::Cancelled` with the first reason, body-after-cancel status, and shutdown report.

If the body poll that requests cancellation also returns `Ready`, the already-recorded cancellation takes precedence over that body value or error, except that an unwinding panic is never caught. The result is recorded in `BodyAfterCancel`, making the explicit cancellation request terminal and deterministic while preserving a post-cancellation error. Async body cleanup is guaranteed only while the driver remains polled and within the shared grace deadline.

Because the spawn gate is already closed, cooperative cleanup MUST run inline or through work owned by an enclosing still-open scope; it cannot start a new child in the cancelling scope. `SpawnError::ScopeClosing` remains the deterministic result if it tries.

The v0.1 `ScopeConfig::default()` is:

| Setting | Default |
|---|---|
| `cooperative_grace` | 5 seconds of Tokio time |
| child panic policy | record and surface through joins/events; do not abort process |
| cancel leftovers on body exit | true and not configurable |
| task names | required, unique numeric ID added automatically |

An application SHOULD set an explicit grace value at production boundaries; the default exists for small programs and examples.

Every normally returned `ShutdownReport` contains the scope ID, terminating cause, configured and elapsed grace, `registry_empty = true`, aggregate counts, and one `TaskShutdownRecord` per child that did not finish cleanly before shutdown began or that panicked/was aborted. A task record contains stable task ID/name, observed-versus-unobserved result status, completion class, cancellation category, panic summary when safe, and monotonic timing offsets. Successful task values and application payloads are never copied into the report. Record order is stable task-ID order, not nondeterministic completion order.

If the body panics, normal unwind runs synchronous destructors and the scope drop guard signals cancellation and aborts known tasks. It cannot asynchronously drain them while unwinding. Kittens MUST NOT catch all panics by secretly moving the body into another task. Applications needing panic containment place the whole scope under an explicit supervisor/process boundary.

If the `scope::run` future itself is dropped, its synchronous guard closes spawning, signals cancellation, and aborts registered tasks. It cannot await drainage. This is a fallback safety action, not the normal structured guarantee.

### 13.3 Cancellation hierarchy

Each scope owns a cancellation controller. `Scope::spawn` creates a child controller and passes its cloneable observation handle into the task factory. A nested scope receives a child of its parent cancellation context. Parent cancellation propagates downward and wakes the nested scope driver; task/child cancellation never propagates upward unless application code maps a child outcome into parent failure.

The public observer is `kittens::scope::Cancellation`, not a raw control handle. It exposes `is_cancelled`, `reason`, and `cancelled`, and it can be moved into `source::cancellation`. Requesting cancellation occurs through `Scope::cancel(reason)` or an owning timeout/supervisor. Internally it MAY wrap tokio-util `CancellationToken`.

The first cancellation reason wins. Later reasons are appended to trace metadata but do not alter handler behavior. `CancelReason` is a small owned value with a stable category and optional human-readable detail.

The non-exhaustive v0.1 `CancelKind` categories are `ScopeExit`, `Application`, `Deadline`, `HandleDropped`, `SourceDropped`, and `ExternalTokio`. Parent propagation preserves the original reason and records the parent scope ID separately rather than rewriting its category. `CancelReason::application(detail)` and `CancelReason::external_tokio(detail)` are public constructors; Kittens owns constructors for the other structural categories. Detail text is available to the application but is omitted from tracing by default.

### 13.4 Task failure and results

`Scope::spawn` and `TaskSpawner::spawn` treat the task output as opaque, so an output of `Result<T, E>` is not automatically a scope failure. The owner decides by joining a direct `Task` or handling a typed `TaskCompletion`. Panics and Tokio cancellation are structural task failures and appear in `TaskJoinError` and in the task-group completion outcome.

An automatic fail-fast policy based on arbitrary task error types is deferred. Applications implement fail-fast by joining the relevant task in a reactor branch and calling `scope.cancel` on error. This avoids type-erased error registries and surprising sibling cancellation.

### 13.5 Blocking work

v0.1 does not expose `Scope::spawn_blocking`. Tokio cannot abort already-running blocking work, which conflicts with the scope guarantee that normal return follows an empty task registry. Applications either wrap a cooperatively stoppable OS thread as an explicit owned resource and join it, or use `escape::spawn_blocking`. A future structured blocking API must choose explicitly between potentially unbounded join and a weakened lifecycle guarantee; it cannot inherit `Scope::spawn` semantics by name.

### 13.6 Typed task-group completion source

`Scope::task_group<T>` returns the sole `TaskEvents<T>` receiver together with its `TaskSpawner<T>`. Each `TaskCompletion<T>` contains stable task ID/name and either the typed output or `TaskJoinError`; it is therefore a homogeneous, reactor-compatible counterpart to Grok's `JoinSet<TaskResult>`, not a type-erased global event bus. Selection of the completion marks that result observed. If a completion is never selected, scope shutdown reports a structural error when applicable and drops any successful output safely.

Group and scope registration are atomic under coordinated close gates. A task is either installed in both registries and eventually drained, or its factory/future remains unspawned and is synchronously dropped on the `SpawnError` path; it cannot appear in only one. Task-group completion delivery order is actual Tokio completion order, with task ID used only for stable identity/report sorting.

## 14. Cancellation semantics

### 14.1 Terms and contracts

Kittens uses four deliberately different terms:

| Contract | What Kittens can implement |
|---|---|
| selection-loss preserving source | when another reactor source wins, the persistent source retains the operation/event state required for a later poll |
| reconstructable waiter | dropping and recreating this specific waiter preserves required progress/events under a reviewed primitive contract |
| drop-clean operation | if invoked and completed normally, synchronous Drop leaves memory/resources in its documented state; forgetting, process/runtime destruction, and a panicking destructor are outside the claim, and cleanup does not imply reconstruction or repeat safety |
| repeat-safe operation | a new attempt after cancellation is allowed by the external operation's semantics; never inferred from cleanup alone |
| cancellation-atomic operation | a specific API documents all-or-nothing external visibility; never inferred from `async` generally |
| cancellation-deferred region | a cancellation request is remembered and observed immediately after a delimited future completes |
| cooperative async cleanup | cancellation causes a release future to be driven while the owner remains polled and within its cleanup budget |

The provisional K0 source-admission trait says only the first. It does not claim reconstructability, repeat safety, handler atomicity, or external rollback.

### 14.2 Reactor cancellation ordering

Every reactor with a long-lived source SHOULD declare at least one `#[shutdown]` branch. A shutdown branch MUST be starvation-protected. The macro adds precedence edges from every shutdown arm to every non-shutdown arm, so shutdown sources form the leading lexical/poll prefix and cannot require backlog probes or yield guards. Cancellation sources SHOULD also occupy the highest priority class; a contradictory graph is rejected as a cycle and a misplaced arm as `KTR016`.

The shutdown handler may perform synchronous state updates and return an exit value. Long async shutdown belongs to the enclosing scope/resource owner so a reactor branch cannot indefinitely delay structural cleanup.

### 14.3 Lost races

Each approved source documents both why internal selection loss preserves its required state and what whole-source Drop does:

- Tokio mpsc/watch/token adapters rely on reviewed versioned primitive contracts and keep durable state alive;
- one-shot future adapters retain the pinned inner future inside the source only when that future can be stored without self-reference and its long borrow is compatible with handler access; otherwise the operation is awaited outside arbitration or isolated behind an owned producer/channel;
- deadline adapters retain an absolute deadline/timer and disarm before yielding rather than resetting a relative delay after every race;
- task events use an admitted channel facade where that task model exists;
- terminal input and cancellation-awkward producers are isolated in an explicitly owned producer whose output channel is admitted;
- fairness-queued waits, ESP-HAL GPIO waits, and generic `embedded-hal-async` calls receive no blanket admission.

Kittens MUST NOT expose `source::repeat(|| arbitrary_future())` as a safe constructor. Recreating an arbitrary losing future is one hazard the source layer exists to prevent. The core-poll candidate instead polls the persistent source directly; this avoids internal waiter drop only when the adapter actually keeps the underlying operation state.

### 14.4 Deferred operations

Kittens uses cancellation deferral internally during acquisition and release. A general public “uncancelable forever” combinator is not part of v0.1. Application code that intentionally ignores cancellation performs an ordinary await and documents its boundedness; Kittens does not add ceremonial syntax without a stronger guarantee.

### 14.5 Timeouts

A raw Tokio timeout drops its inner future. A Kittens scope timeout instead:

1. records `CancelReason::Deadline`;
2. signals the nested body observer and children and continues polling both during cooperative grace;
3. records any body completion or error in `BodyAfterCancel`;
4. drops an unfinished body and aborts remaining async children when grace expires;
5. drains every child completion;
6. returns `TimedOut { body, shutdown }`.

The body future itself may ultimately be dropped after grace. Therefore operations inside it still need source/resource contracts. The timeout may exceed its nominal deadline by cleanup grace and scheduler delay; the returned report MUST include both requested deadline and actual completion time.

### 14.6 Cancellation testing

Every cancellable API MUST test every applicable cancellation phase below and document why any phase is not applicable. The retained deterministic cases are named critical interleavings, not a proof over every possible schedule:

- before first poll;
- during acquisition or arming;
- after an operation becomes ready but before its handler completes;
- during use;
- during cleanup;
- at the cleanup deadline;
- concurrently with normal completion.

Tests MUST assert both result classification and remaining task/resource state.

## 15. Resource semantics

### 15.1 Why RAII is insufficient by itself

Ordinary `Drop` is the universal synchronous fallback and SHOULD close file descriptors, release memory, and send best-effort stop signals. It cannot await a network close handshake, terminal writer drain, remote lease release, or child process reap. Kittens therefore supplies a bracket-like cooperative path while stating exactly when it is driven.

### 15.2 `resource::using` state machine

```text
NotAcquired ──acquire ok──▶ Acquired ──use outcome/cancel──▶ Releasing ──done──▶ Released
     │                          │                                │
 acquire error                 └──outer drop/panic──────────────┴──sync Drop only
     ▼
   Failed
```

The default algorithm is normative:

1. If cancellation is already requested, do not start acquisition and return `CancelledBeforeAcquire`.
2. Once acquisition starts, defer cooperative cancellation until acquisition resolves. This avoids dropping an arbitrary partially initialized acquisition future.
3. If acquisition fails, return `Acquire` and do not call release because no `R` exists.
4. If cancellation arrived while acquisition was pending, skip use and call release with `ExitStatus::Cancelled`.
5. Otherwise poll use against cancellation.
6. On use success, use error, or cancellation, drop the use future before starting release.
7. Move `R` exactly once into release with `Success`, `Error`, or `Cancelled` status.
8. Ignore further cooperative cancellation while release runs, but enforce `cleanup_grace`.
9. On cleanup deadline, drop the release future, which synchronously drops any `R` it still owns, and return `CleanupTimedOut`.

`ResourcePolicy::new(cleanup_grace)` requires the cleanup budget explicitly and `ResourcePolicy` has no `Default` implementation in v0.1. A zero duration is allowed and means “attempt one release poll, then fall back to synchronous drop”; it does not mean unbounded cleanup.

Race precedence is fixed for reproducibility. A cancellation already observed before a use poll, or ready in the same driver cycle as use completion, wins and the use future is dropped before release. A release completion ready in the same cycle as its cleanup deadline wins over the deadline. This does not make the use operation atomic: the losing use future may already have performed an external side effect.

Acquisition can therefore exceed a cancellation deadline if it never completes. v0.1 chooses honest deferred cancellation over pretending an arbitrary acquisition is safe to drop. Curated cancel-safe acquisition wrappers are deferred until concrete needs exist.

### 15.3 Use cancellation is not atomicity

When cancellation wins during use, Kittens drops the use future and runs release. This does not undo a partially sent request, shell side effect, filesystem write, or model call. APIs that need atomicity MUST supply an operation-specific idempotency/commit protocol. `ResourcePolicy` has no `atomic = true` flag because a flag cannot establish the fact.

### 15.4 Result precedence

The public error is structurally equivalent to:

```rust
pub enum ExitStatus {
    Success,
    Error,
    Cancelled(CancelReason),
}

pub enum ResourceError<AcquireE, UseE, ReleaseE> {
    CancelledBeforeAcquire { reason: CancelReason },
    Acquire(AcquireE),
    Use(UseE),
    Release(ReleaseE),
    UseAndRelease { use_error: UseE, release_error: ReleaseE },
    Cancelled { reason: CancelReason, release_error: Option<ReleaseE> },
    CleanupTimedOut { prior: ExitStatus, elapsed: Duration },
}
```

Outcome rules:

| Use outcome | Release outcome | Returned result |
|---|---|---|
| success | success | use value |
| success | error | `Release` |
| error | success | `Use` |
| error | error | `UseAndRelease` preserving both |
| cancelled | success | `Cancelled { reason, release_error: None }` |
| cancelled | error | `Cancelled { reason, release_error: Some(...) }` |
| any | cleanup deadline | `CleanupTimedOut` with prior category |

Errors MUST retain both primary and cleanup failures. Display text may be concise, but structured access cannot discard either.

### 15.5 Panic and outer-drop behavior

If acquire, use, or release panics, normal synchronous unwinding drops owned values. Kittens does not guarantee the async release callback. If the entire `using` future is dropped, the same limitation applies. API docs and examples MUST say “cooperative async release,” never unqualified “release always runs.”

The strongest normal composition is `resource::using` inside `scope::run` or `Scope::with_timeout`, where Kittens continues polling cooperative cleanup during the configured grace interval.

When a resource cleanup deadline is nested inside a scope/timeout grace deadline, the effective available time is the earlier outer deadline. Documentation MUST require the enclosing grace to be at least the intended resource cleanup grace; reports identify which boundary actually stopped polling.

### 15.6 Grok teardown mapping

The Grok outer cleanup sequence should be represented as nested ownership, not one magical resource:

1. reactor returns;
2. log resource flushes;
3. terminal resource stops accepting frames, drains and joins writer, then restores terminal modes;
4. agent resource cancels and joins the agent within grace;
5. process-scope resource kills/reaps remaining child processes.

The ordering is dynamic application policy. Kittens can enforce nesting/consumption and cooperative scope drainage; it cannot infer that terminal escape sequences should precede process teardown.

## 16. Typestate model

### 16.1 Decision

Kittens recommends application-owned typestate for short, meaningful, mostly linear workflows. It does not ship `Computation<Before, After, Output>` or an indexed monad. The type carrying domain data remains an ordinary struct:

```rust
struct Run<State> {
    id: RunId,
    command: Command,
    state: State,
}

struct Proposed;
struct Validated { digest: CommandDigest }
struct Approved { digest: CommandDigest }
struct Finished { outcome: CommandOutcome }
```

Transitions consume `self` and are implemented only for the valid source state:

```rust
impl Run<Proposed> {
    async fn validate(self, policy: &Policy) -> Result<Run<Validated>, ValidationError>;
}

impl Run<Validated> {
    async fn request_approval(
        self,
        gate: &ApprovalGate,
    ) -> Result<(Run<Approved>, Approval<CommandExecution>), ApprovalError>;
}

impl Run<Approved> {
    async fn execute(
        self,
        shell: &Shell,
        approval: Approval<CommandExecution>,
    ) -> Result<Run<Finished>, ExecuteError>;
}
```

Calling `execute` on `Run<Proposed>` produces the desired local E0599 “method not found” shape. Async transitions are ordinary consuming async methods; ownership survives `.await` without special Kittens machinery.

### 16.2 What `flow` provides

`kittens::flow` v0.1 contains only:

- `TransitionError<From, Cause>` for preserving a recoverable source value where useful;
- `RestoreError` and documentation helpers for validating durable runtime tags;
- marker traits used only by optional diagnostics, with no blanket state-machine framework;
- canonical examples and compile-fail fixtures.

It MUST NOT require states to implement a Kittens trait merely to use ordinary typestate.

### 16.3 Runtime state in reactors

A long-lived reactor commonly has a runtime enum such as `Connected`, `Reconnecting`, or `ShuttingDown`. Encoding each event arrival as a new reactor type would make the loop unusable. v0.1 uses private runtime state and explicit `#[when(self.state.allows_acp())]` guards.

The macro makes source availability visible but cannot prove that the guard is a complete state predicate or that handlers mutate state only through legal transitions. Compile-time active-state tables and transition permits are deferred pending Grok-scale borrow/diagnostic prototypes.

### 16.4 Persistence and restart

Phantom state is not durable proof. `Run<Approved>` and `Approval<_>` MUST NOT implement `Deserialize` as a way to recreate authority.

Applications persist an ordinary record:

```rust
struct RunRecord {
    id: RunId,
    state: RunStateTag,
    command: Command,
    command_digest: CommandDigest,
    policy_version: PolicyVersion,
}
```

Restoration validates the record and returns a runtime enum containing typed variants. Approval-bearing states MUST normally restore as “validated, approval required again” unless an application verifies a separately signed, unexpired durable authorization under current policy.

## 17. Capability model

### 17.1 Authority values, not permission booleans

Kittens-mediated effects require concrete values such as `ReadWorkspace`, `WriteWorkspace`, `Network`, `Shell`, or `SpawnAgent`. APIs MUST accept the narrow capability they need rather than a broad environment object.

```rust
async fn index_workspace(read: &ReadWorkspace) -> Result<Index, IndexError>;
async fn call_model(network: &Network, request: Request) -> Result<Response, ModelError>;
async fn execute(
    shell: &Shell,
    command: ApprovedCommand,
) -> Result<CommandOutcome, ExecuteError>;
```

A `ReadWorkspace` argument cannot satisfy `&Network`; normal type mismatch is the diagnostic.

### 17.2 Bootstrap boundary

Capability fields and broad constructors are private. `Bootstrap::from_process` is the single public ambient trust boundary and requires an `AuditReason`. It validates configuration before creating authority. Calling it is not `unsafe`, but it is searchable and traced.

Libraries below the process entry point SHOULD receive capabilities as parameters and MUST NOT call bootstrap. Kittens documentation MUST show bootstrap only in top-level binaries/tests.

### 17.3 Filesystem authority

`ReadWorkspace` and `WriteWorkspace` MUST be backed by cap-std directory handles rather than path-prefix string checks. `narrow(relative)` MUST reject absolute paths and parent traversal and return a handle rooted beneath the receiver. Narrowing consumes no ambient authority.

Read and write authority are distinct types. A write capability MUST offer an explicit fallible `as_read()` narrowing operation that derives a read handle to the same or narrower root; no reverse operation exists.

`ReadWorkspace::open(relative)` performs capability-relative resolution/open and returns `tokio::fs::File` by converting the already opened standard handle; it does not hand the original path back to ambient Tokio filesystem APIs. The capability-relative open itself is synchronous and can block on a slow filesystem. Documentation MUST tell reactor handlers to move large directory traversal/metadata workloads behind an explicitly owned service/process boundary; Kittens does not hide them in an untracked blocking task.

### 17.4 Network and shell authority

`Network` wraps an application-provided connector/service plus host, scheme, and port policy. Narrowing intersects allowlists. DNS rebinding and proxy behavior are runtime security concerns; adapters MUST document which resolved endpoint is checked.

`Shell` wraps an executor backend and a policy covering executable identities, working-directory capability, environment allowlist, argument limits, and resource limits. There is no API accepting an unvalidated command string and a boolean `approved` flag.

The `tower` feature MUST provide adapters that wrap Tower services behind these capabilities. A `Service` alone is not authority because any holder able to construct another ambient client could bypass it.

Tower readiness/call futures are request operations, not reactor sources. An adapter MUST document whether dropping `ready`/`call` releases reservations, whether a remote request may already have committed, and whether retry is idempotent. Kittens does not infer those facts from `Service`.

### 17.5 Delegation, narrowing, and revocation

- Borrowing delegates authority temporarily.
- Moving delegates it exclusively.
- `Arc<C>` is an explicit shared delegation chosen by the application; capabilities are not `Clone` merely for convenience.
- `narrow` can only intersect scope/policy.
- `Revocable<C>` pairs a capability with shared runtime revocation state. Each Kittens-mediated operation checks the current generation before side effects.
- Revocation is runtime, not typestate: an outstanding external operation may already have passed its check.

### 17.6 Consumed authorization

`Approval<A>` and `ApprovedCommand` MUST be non-`Clone`, private-field values. Approval creation records:

- action kind and canonical action digest;
- run/session identity;
- approving principal/gate;
- policy version;
- issuance and expiry;
- optional usage constraints.

The consuming operation validates those dynamic fields immediately before the side effect. Ownership prevents reuse; runtime binding prevents using a valid token for a different command.

At the low-level shell boundary, the only accepted command value is `ApprovedCommand`. An application workflow method such as `Run<Approved>::execute` consumes its `Approval<CommandExecution>` through the public consuming method `approval.bind(validated_command)`, receives `ApprovedCommand` after the runtime target checks, and passes that value to `Shell::execute`. Applications cannot construct either value through public fields. The `Run<Approved>` marker expresses workflow position; the consumed approval/approved command carries the authority.

### 17.7 Security boundary disclaimer

Capabilities constrain APIs that choose to require them. In-process code can still call `std::fs`, create a raw socket, or start a process if OS permissions allow. Strong hostile-code isolation requires a process, container, WASI, seccomp, sandbox, or equivalent external boundary. Kittens MUST never market capability typing as such a sandbox.

## 18. Protocol/session model

### 18.1 v0.1 decision

A general `kittens::protocol` module is deferred. Current Rust session-type libraries prove useful concepts but introduce recursive type machinery, generated channel types, compile-time cost, and diagnostics that are disproportionate for the v0.1 agent persona.

Kittens v0.1 recommends app-owned binary endpoint typestate when ordering has material value. It is appropriate for orchestrator/worker, approval-gate, and supervisor/child conversations with a small finite state graph.

### 18.2 Required pattern semantics

An endpoint wrapper owns its transport and state marker. Sending or receiving consumes the endpoint and returns the next state. A failure MUST return a terminal `ProtocolFailure` or a recovery value that explicitly contains the still-valid endpoint state; it must not silently guess whether a partially completed I/O transition occurred.

For the example:

```text
Ready ──send Task──▶ Waiting ──receive Report──▶ Ready ──shutdown──▶ Closed
```

- `send_task` exists only on `Worker<Ready>`;
- `recv_report` exists only on `Worker<Waiting>`;
- `shutdown` exists only on a state where the protocol permits it;
- the transport's cancellation behavior is documented separately;
- durable restart uses a runtime protocol tag and re-handshake, not deserialization of a marker.

### 18.3 What is not encoded

v0.1 does not encode multiparty global progress, deadlock freedom, distributed peer conformance, arbitrary recursion, message delivery, or runtime participant discovery. Reactor event ordering and session message ordering are distinct problems.

### 18.4 Gate for a future protocol package

A protocol package may enter the main architecture only if a retained prototype demonstrates all of:

- an orchestrator/worker protocol with branching and cancellation;
- errors that name the attempted method and current state rather than recursive aliases;
- compile-time overhead acceptable under section 36;
- serialization/re-handshake semantics;
- no more than two protocol-related generic parameters in ordinary handler signatures;
- a statistically meaningful agent benchmark improvement over hand-written typestate.

## 19. Error model

### 19.1 Runtime errors

Kittens public APIs MUST return concrete, non-exhaustive error enums or structs. The public API MUST NOT require `anyhow::Error`, boxed dynamic errors, or a Kittens-wide error type. Applications may erase errors at their outer boundary.

Every error MUST implement `Debug`, `Display`, and `std::error::Error` when its generic causes do. Errors SHOULD be `Send + Sync + 'static` where the causes permit. Structured fields MUST be accessible without parsing display strings.

Primary families are:

| Family | Required information |
|---|---|
| `ScopeRunError<E>` | body error if any, shutdown report, unobserved structural task failures |
| `TaskJoinError` | task ID/name, cancelled/panicked/aborted classification, panic payload summary if safe |
| `SpawnError` | scope/group closing cause; any supplied task factory/future was left unspawned and synchronously dropped |
| `ResourceError<A,U,R>` | acquisition/use/release failures without discarding concurrent cleanup failure |
| `SourceArmError<T>` | already-armed state and the uninstalled receiver/future/deadline where recoverable |
| `CommitError<E, Ticket>` / `GateError<Ticket>` | rejected submission cause; non-monotonic accepted ticket or poisoned gate state |
| `CapabilityError` | denied/narrowing/revoked/expired/target-mismatch category and policy identity |
| `InvalidAuditReason` | empty or nonspecific reason supplied at a trust/escape boundary |
| `RestoreError` | schema/state/policy mismatch and required recovery action |

`ScopeRunError<E>` distinguishes:

- `Body { source: E, shutdown: ShutdownReport }`;
- `Shutdown { report: ShutdownReport }` when the body succeeded but structural failure occurred;
- `Cancelled { reason: CancelReason, body: BodyAfterCancel<E>, report: ShutdownReport }` when scope or parent cancellation terminated the body.

Normal cancellation of leftover children after a successful body is not itself an error if every child terminates within grace. `Task<T>` is `#[must_use]`, but dropping it is an explicit cancellation request rather than a runtime error. Aborts after grace, task panic, or failed drainage are structural errors.

### 19.2 Reactor errors

`reactor!` does not wrap the application's runtime error type. Handler and phase errors remain `E`. All user sources may be dynamically dormant. The direct-poll form then remains pending; the Tokio control expansion includes a macro-owned, always-enabled `pending()` sentinel so Tokio cannot take its “all branches disabled” panic path. The sentinel can never win and is not a user source.

A `#[shutdown]` source MUST have neither `#[when]` nor `#[yields_to]`. This protects the declared shutdown topology; it is not a general requirement that every reactor have an unguarded branch or eventually make progress.

### 19.3 Closed and dormant are not errors by default

Close behavior is part of the source type. `close::Dormant` makes closure a state transition; `close::Emit` exposes `ChannelEvent::Closed` once. The reactor macro MUST NOT invent a generic closed-channel error.

### 19.4 Panic policy

Panics are bugs or explicit containment events, not a typed error channel. Child-task panics are captured by Tokio and reported structurally. A panic in the reactor/body unwinds normally. Kittens MUST avoid `catch_unwind` in core control flow unless a future supervisor API explicitly promises isolation.

## 20. Runtime integration and executor boundary

### 20.1 Runtime relationship

Tokio remains the only K0 production runtime integration. The reactor kernel's leading candidate is nevertheless one ordinary `Future` built from `core::future`, `Pin`, `Context`, and `Poll`. Kittens MUST work inside the caller's executor and MUST NOT create a global runtime, start a runtime thread, own an executor run queue, or call `block_on` internally.

K0 compares direct core polling with a direct biased Tokio-select oracle before freezing the mechanism. Executor neutrality is retained only if it preserves Grok behavior, borrow quality, diagnostics, transparency, and performance. It is an architectural boundary, not a commitment to ship multiple executors immediately.

The supported K0 Tokio surface is deliberately limited to the source adapters named in section 37.3 and test-only oracle support. Earlier candidates for external-token observation, `ProcessShell`, Tower adapters, and Tokio scope integration remain deferred extension hypotheses; they are not supported K0 APIs.

Kittens does not re-export `spawn`, `select!`, locks, channels, or an alternate Tokio prelude. Tokio receivers and timers are consumed only by the reviewed `source::tokio` constructors admitted to the current slice.

The facade MAY re-export narrowly required Tokio types under `kittens::__private` solely so macro expansion works when users rename dependencies. It MUST NOT expose `__private` as supported API.

### 20.2 Generated local arbitration

The leading expansion polls persistent source objects directly in lexical order inside a small ordinary future. A ready source returns one event in the K0 candidate. If all enabled sources are pending, each enabled source has received the same executor `Context` so its adapter can arrange a wake.

An **arbitration** begins after one `before_poll` execution and guard snapshot and ends when one source is selected; the future that performs it may receive any number of executor `poll` calls. A guard value is fixed for that arbitration and reevaluated only when the outer reactor starts the next arbitration. `before_poll` likewise does not rerun merely because the executor repolls a pending arbitration.

The K0 comparison expansion emits `tokio::select! { biased; ... }` with a macro-owned `poll_fn` waiter for each admitted persistent source. Such a waiter only delegates the current `Context` to stored source state; constructing or dropping the waiter MUST NOT create, own, reset, or cancel the underlying operation. Both forms MUST:

- evaluate user `#[when]` expressions once per arbitration before source polling;
- preserve their source-declaration order;
- combine a user guard and yield guard with logical AND;
- never evaluate a yield backlog probe more than once before select and once between drained items;
- ensure a shutdown branch has no user guard;
- end every temporary arbitration borrow and branch-delegate future before invoking a handler while retaining adapter-owned pending operation/event state.

Enablement snapshots are deterministic. Branches are visited in lexical order. For each branch, the ordinary user guard is evaluated first; false short-circuits its yield probe. If still enabled, that branch's declared backlog probe is evaluated once. Probes are per relation edge and are not cached across two sources that yield to the same target. The snapshot is then fixed across pending executor repolls. During a drain window the user guard is not reevaluated; only the selected source's one yield probe runs once after each successful handler and before the next immediate item probe. Reviewed backlog adapters must treat probing as observational, but the explicit order remains defined because Rust cannot prove purity.

The Tokio control form appends an always-enabled, side-effect-free `core::future::pending` sentinel after every user branch. It prevents Tokio's all-disabled panic and cannot affect selection. The core-poll form MUST retain each source object when a different source wins and MUST NOT allocate or box solely for arbitration. On the curated deterministic fixtures and their declared readiness/wake traces, both forms MUST select the same source and produce the same wake-driven progress. This is a bounded oracle claim, not equivalence under every executor schedule or arbitrary third-party source.

The earlier candidate recursively whitelisted guard expression forms. That whitelist is not evidence-backed and may reject familiar Rust without proving purity. The kernel evaluates one ordinary synchronous Rust expression exactly once per arbitration and requires `bool`; it rejects only syntax that would suspend or escape guard evaluation when that can be identified reliably. Macro calls, blocks, and control-flow expressions remain a prototype decision. The macro cannot prove that an accepted call is pure, cheap, or stable.

A false guard is only a polling filter; it does not register a waker and is not resnapshotted merely because the executor wakes the pending arbitration. In K0, guards depend on reactor-owned state changed between arbitrations. An external change must arrive through an enabled admitted source that becomes ready and ends the current arbitration; a wake by itself is insufficient. Source-local external arming belongs in a dormant adapter with its own wake contract rather than a guard. Kittens can prevent an all-disabled panic, but it cannot infer liveness from a boolean expression.

Handlers and phase blocks run outside selection. While any of them awaits, no source in that reactor is polled, though independently spawned executor tasks may continue. Priority therefore applies only at arbitration boundaries and does not preempt long `initialize`, `before_poll`, handler, or `after_event` work. Long work SHOULD move to an explicitly owned task where that runtime supports the required lifecycle, or be bounded/tested at runtime; K0 supplies no portable spawn rule.

### 20.2.1 Cooperative scheduling budget

Tokio maintains a per-task cooperative scheduling budget (`tokio::task::coop`; see `RESEARCH.md` section 20A). A budget-aware Tokio operation MAY return `Poll::Pending` while an item is available. This is an executor-level condition, not a Kittens source state: it is not dormancy, not closure, and not selection loss.

The kernel MUST NOT model, reimplement, defeat, or wrap the budget. Generated code MUST NOT call `tokio::task::coop::unconstrained`, which would trade a measurement confound for a starvation hazard in exactly the firehose-versus-shutdown topology this specification exists to protect. Readiness metadata makes no claim about budget state, and starvation analysis does not model it.

The following obligations apply instead:

- every reviewed Tokio adapter MUST state in its documentation whether polling its operation consumes cooperative budget, because the primitive pages themselves generally do not disclose this;
- the equivalence oracle in section 20.2 holds **under equal budget conditions**. The core-poll and Tokio-select forms need not consume budget at the same points, particularly across a drain window, so a divergence in selected source MUST be classified against instrumented budget state before it is attributed to either expansion mechanism;
- `drain(max = N)` bounds Kittens-managed service work only. It does not bound budget consumption, and a budget-induced empty immediate probe ends a service window for an executor reason the declaration does not express. This limitation MUST appear beside the feature per section 4.12;
- the budget has no Embassy or bare-metal counterpart and MUST NOT enter the `no_std` kernel vocabulary.

### 20.2.2 Handler panic and reactor state

Section 19.4 fixes the panic policy: a panic unwinds and Kittens does not `catch_unwind` in core control flow. Two consequences specific to generated arbitration MUST be stated in the expansion documentation rather than left to inference.

A panic inside a handler unwinds through the generated reactor future. Sources are dropped by ordinary Rust drop order; their adapter drop contracts — which section 37.6 already requires to be documented separately from selection-loss preservation — determine what pending operations and undelivered events are lost. Kittens promises nothing beyond those adapter contracts.

A panic during a **drain service window** additionally means `after_event` does not run for items already handled in that window. This matches the `Err` and `Stop` rule, but the state differs: earlier items in the window have already mutated application state. Generated code MUST NOT attempt compensating cleanup, and the drain documentation MUST state that a mid-window panic can leave application state advanced without its after-event hook.

If a caller resumes an unwound reactor future, that is ordinary Rust future-after-panic behavior and carries no Kittens guarantee. The kernel makes no fusing claim.

### 20.3 Tokio primitive policy

- K0 wraps only the Tokio channels, deadlines, and cancellation primitives named in section 37.3 as reviewed sources.
- Tokio `JoinSet`, Kittens `Scope`, typed task handles, and task groups remain post-K0 lifecycle candidates. A fixture may use an ordinary explicitly application-owned `JoinSet` or RAII task/thread owner, which carries no Kittens scope guarantee.
- Fairness-queued waits such as `Mutex::lock`, `RwLock::read/write`, `Semaphore::acquire`, and `Notify::notified` MUST NOT be accepted as arbitrary reconstructed reactor futures. Curated adapters must preserve waiter state or isolate them.
- `read_exact`, `read_to_end`, `read_to_string`, `write_all`, and other documented cancellation-unsafe operations MUST be awaited outside races, retained only when the one-shot storage/borrow preconditions in section 14.3 hold, or isolated behind an explicitly application-owned producer/channel.
- `spawn_blocking` is not wrapped by K0; its non-abortability remains a deferred interop concern.

### 20.4 Embassy and HAL boundary

Embassy/ESP-HAL production adapters are deferred. Future admission MUST be primitive- and version-specific:

- `embassy-futures` losing futures are dropped and its selectors are left-biased;
- `embedded-hal-async` traits do not establish a general drop/reconstruction contract;
- current ESP-HAL GPIO waits explicitly lose an edge that arrives after the waiter is dropped;
- cleanup-on-drop for an owning DMA handle does not make a partial external transfer repeat-safe;
- Embassy spawned tasks have static pools and no Tokio-like join/abort handle.

K0 MAY contain a host-only interrupt-like source and an ownership-returning transfer fixture to test borrows. It MUST NOT label that fixture an Embassy or ESP32 adapter. A real adapter graduates only with target compilation, upstream-contract citation, lost-race tests, and—where the fact is physical—hardware validation.

### 20.5 Terminal-input precedent

The terminal integration fixture follows the inspected Grok workaround: an owned producer thread/task polls the terminal API, forwards events through an admitted mpsc source, and participates in a bounded pause/park handoff before another process receives the tty. K0 ships no crossterm-specific code or cookbook and requires an explicitly application-owned task or RAII thread guard. A later `Scope` integration must earn promotion separately.

### 20.6 Raw interop

Leaf libraries MAY expose ordinary futures, HAL operations, and runtime services. A harness adapts them at the orchestration boundary. Kittens cannot prevent raw `tokio::spawn`, `tokio::select!`, Embassy selection, or direct peripheral access; repository policy and a future lint can discourage them. Raw use is not automatically wrong, but any Kittens guarantee stops at that boundary.

## 21. Provisional simulation and testing integration

### 21.1 Candidate scope

The first kernel uses crate-private controllable sources, ordinary Tokio channels, selected-source assertions, and paused time. A public scenario API, wake coordinator, persisted replay, source-ID binding, and fault framework are not justified yet. The design below is retained as a candidate only. Its priority, though not its K0 status, changed under the section 2.1 coverage thesis: deterministic schedule exploration is the only mechanism that reaches the handler-interior, rearm-liveness, and schedule-dependent protocol classes that static checks cannot, so this layer and the escape-surface lint (section 34.2) graduate first among post-K0 extensions.

The production macro expansion remains the same selected K0 polling form. Test behavior changes only because controllable source adapters and a trace sink are injected.

### 21.2 Scripted sources

`sim::Scenario` owns virtual event scripts and creates typed `ScriptedSource<T, L, R, C>` values with the same declared IDs used by the reactor:

```rust
let mut scenario = sim::Scenario::new(Seed::from_u64(7));
let (acp, acp_script) = scenario.source(
    "acp_stream",
    sim::contract::repeating_may_ready(close::Emit),
);
let (input, input_script) = scenario.source(
    "terminal_input",
    sim::contract::repeating_may_ready(close::Emit),
);

scenario.at(ms(0), acp_script.item(token("a")));
scenario.at(ms(0), input_script.item(key("q")));
scenario.at(ms(5), acp_script.close());

let trace = sim::run_paused(scenario, async move {
    app.run(&mut sources, scope).await
}).await?;
```

This is a test sketch. A contract constructor returns a zero-sized `ScriptContract<L, R, C>` whose type parameters become the `ScriptedSource` associated markers; lifecycle/readiness/close behavior is therefore checked by the same generated trait assertions as a production adapter, not chosen from a runtime enum. `Scenario::source` returns the non-`Clone` reactor source and a cloneable test-only `ScriptHandle` used to schedule events. The scenario validates runtime script legality—for example, no second one-shot item, no item after terminal close, and only declared rearm transitions—and validates the declared source ID against generated reactor metadata. Events at the same virtual timestamp are simultaneously ready; biased reactor topology determines selection. A seed controls explicitly modeled fault/release choices, not hidden production scheduling.

### 21.3 Required injection points

The test utilities MUST support:

- source item, close, dormancy, and rearm events;
- cancellation at a virtual timestamp or after a selected source ID;
- handler/tool/model faults supplied by application fakes;
- delayed and stale acknowledgement tickets;
- task success, panic, cooperative delay, and refusal to stop;
- cleanup success, error, and timeout;
- retry decisions and deterministic jitter through an injected randomness trait;
- Tokio paused time and explicit advancement.

Kittens SHOULD define small `Clock` and `Random` traits only at policy boundaries that require injection. It MUST NOT wrap all Tokio time or randomness globally.

### 21.4 Trace and replay

A deterministic trace record MUST include:

- schema version and reactor identity;
- stable source ID and priority class;
- poll-cycle number and virtual/real timestamp;
- branch guard state where nonsensitive;
- sources known ready by `Scenario` (test sources only);
- selected source;
- batch count and whether yield/max/empty/dormant/rearm stopped it;
- handler result category and control decision;
- phase start/result;
- task lifecycle events;
- cancellation reason;
- single-flight request/submit/ack state transition;
- injected fault ID.

Production traces cannot in general know the complete ready set because a biased select may stop polling after a ready branch. They MUST label the field unavailable rather than infer it. `Scenario` knows the complete scripted ready set and records it.

Replay keys sources by declared ID, never lexical branch index. A trace header carries a canonical topology descriptor plus its quick fingerprint. Descriptor schema v1 is a length-prefixed UTF-8 encoding, in written order, of the reactor identity, selection policy, phase set, priority edges, and for each source its ID/class/lifecycle/readiness/close/starvation/guard/yield/drain/last/precedence declarations and normalized persistent source path. Handler bodies and runtime values are excluded. The proc macro emits this descriptor as static data.

The v1 quick fingerprint is FNV-1a-128 over the descriptor bytes and is explicitly non-security-sensitive. Exact descriptor equality—not fingerprint equality—is authoritative for replay. The algorithm and descriptor schema version are recorded, and replay MUST fail clearly when IDs/contracts/topology differ instead of silently applying events to changed code. No Rust `Hash`/`DefaultHasher` output is persisted.

### 21.5 External tools

- Tokio paused time is the default async time facility.
- Loom MUST model Kittens' own scope registry, wake-on-arm, cancellation reason, and single-flight synchronization if those types use atomics/locks.
- Shuttle MAY be offered as a dev adapter for task schedule exploration after the core API is stable.
- MadSim and Turmoil are optional network/system simulation integrations, not dependencies.
- FoundationDB-style simulation is design inspiration; Kittens does not reproduce its architecture.

Every reported random seed and scenario file MUST reproduce the same modeled choices under the same Kittens patch version.

## 22. Escape-hatch design

### 22.1 Requirements

Escape hatches MUST be:

- under the single searchable namespace `kittens::escape`;
- absent from prelude-style imports;
- named for the weakened guarantee;
- passed an `AuditReason` constructed from a static string;
- annotated `#[must_use]` when they return an ownership handle;
- instrumented with call site, reason, and operation under `tracing`;
- represented in the benchmark and documentation as last-resort operations.

`AuditReason` is exported at `kittens::AuditReason`. `AuditReason::new(&'static str) -> Result<AuditReason, InvalidAuditReason>` accepts a nonempty static string of at least eight non-whitespace characters. Static text prevents a runtime value from hiding the architectural rationale and makes repository searches useful; fallible construction avoids a hidden panic.

### 22.2 v0.1 escape operations

| Operation | Guarantee weakened |
|---|---|
| `escape::unchecked_source(reason, contract, source)` | source contract is trusted rather than sealed/reviewed |
| `escape::spawn_detached(reason, future)` | child is not structurally owned by a Kittens scope |
| `escape::spawn_blocking(reason, closure)` | already-running blocking work may ignore cancellation and outlive its caller until return |

`cap::Bootstrap::from_process` is the supported audited trust boundary that creates constrained authority, not an unchecked escape. `OptionalOneShot::cancel_and_disarm` is likewise an explicit supported lifecycle transition: it reports that the retained operation is being abandoned and requires a `CancelReason`, but it does not assert a false source contract. Both remain searchable and traced in their owning modules.

`unchecked_source` requires a complete typed `UncheckedContract<L, R, C, D, B>` builder: lifecycle, readiness, close behavior, drain support, backlog support, and an explicit final `claim_restart_safe()` step. Its marker parameters become the wrapper source's associated types; these claims are not runtime booleans. `with_drain` requires a concrete nonblocking function/closure with the shape `&mut S -> TryNext<S::Item>`, and `with_backlog_probe` requires `&S -> bool`; only builders carrying those operations make the returned wrapper implement `DrainableSource` or `BacklogProbeSource`. The functions remain statically dispatched and the wrapper introduces no required allocation.

The reactor's explicit annotations are checked against the typed wrapper exactly as for curated sources. Kittens trusts that the inner public `Source` and supplied operations actually obey the claimed cancellation, readiness, close, drain, and backlog semantics; false claims are why this API is under `escape`. This design prevents the escape from becoming either a metadata-free raw future or an impossible runtime value that purports to choose associated types.

`spawn_detached` returns `DetachedTask<T>` rather than Tokio's type alias. Dropping it detaches; `join` and `abort` remain available. The type and audit event make the choice visible, but Kittens cannot recover structural ownership.

`spawn_blocking` returns `BlockingTask<T>` and repeats Tokio's non-abortability in its type documentation and trace event. Dropping the handle does not stop already-running work. Production callers SHOULD retain and join it from a longer-lived process owner.

### 22.3 No fake `unsafe`

These functions are not `unsafe fn` unless their implementation has an actual Rust memory-safety precondition. “Unsafe orchestration” is not Rust undefined behavior. Visual severity comes from namespace, long name, reason, tracing, and review policy rather than misusing the language keyword.

## 23. Serialization and durable-state considerations

### 23.1 Never serialize proof objects

The following MUST NOT implement `Serialize` or `Deserialize` in v0.1:

- capabilities and bootstrap roots;
- approvals and approved commands;
- `Scope`, `Task`, cancellation handles, or task events;
- source adapters and pending futures;
- submission permits or an in-flight `SingleFlight` gate;
- typed protocol endpoints;
- phantom typestate values as authorization evidence.

Serialization of private fields through a derive would manufacture authority or pretend an in-memory protocol survived a process boundary.

### 23.2 Runtime records

Applications MAY serialize domain records, runtime state tags, command/action digests, policy versions, checkpoint positions, and trace records. Restoration MUST validate those records into fresh in-memory values.

For an in-flight frame or external request, restart state is application-specific. The safe default is `UnknownOutcome`: re-handshake or query the external system before resubmitting. Kittens MUST not infer that retry is safe.

### 23.3 Schema rules

Kittens-owned serialized trace/test formats use:

- an explicit integer schema version;
- stable string source IDs;
- canonical reactor topology descriptor and versioned fingerprint;
- exhaustive documented migration/rejection behavior;
- no raw memory addresses, generic type names, or branch indexes as identity.

Adding a source is a topology change. Renaming a source ID is a replay-breaking change unless a test-only alias migration is supplied.

### 23.4 Durable approval

The v0.1 approval token is deliberately ephemeral. A future durable approval record would require signature verification, principal identity, expiry, nonce/use ledger, target digest, current policy revalidation, and replay protection. It is not equivalent to serializing `Approval<A>` and is deferred.

## 24. Provisional observability considerations

### 24.1 Stable event vocabulary

With the `tracing` feature, Kittens MUST emit structured events for:

- reactor start/stop and topology fingerprint;
- poll cycle, selected source ID, priority class, and batch size;
- source arm/disarm/close/dormant transition;
- buffered yield and bounded-drain stop reason;
- before-poll and after-event duration/outcome;
- task spawned, cancellation requested, completion, panic, abort, and drain;
- scope closing/grace expiry/closed and shutdown report;
- resource acquire/use/release phase and cleanup deadline;
- capability bootstrap/narrow/revoke/deny and approval consume outcome;
- single-flight request/coalesce/submit/stale-ack/accepted-ack;
- every escape-hatch invocation.

Field names and categorical values become compatibility surface once published. They MUST be listed in an observability reference and versioned deliberately.

### 24.2 Data minimization

Payloads, model tokens, terminal text, command arguments, file contents, secrets, and approval rationale MUST NOT be recorded by default. Events use IDs, counts, durations, policy identifiers, and outcome categories. Applications explicitly opt into redacted payload fields.

The canonical topology descriptor necessarily contains declared identifiers, source paths, starvation reasons, and normalized guard tokens. Reactor declarations MUST NOT embed credentials or sensitive runtime values in those static tokens; dynamic guard results may be traced only as booleans unless the application explicitly opts into more.

### 24.3 Disabled cost

With tracing disabled, the core path MUST not allocate or format strings for observability. Static source metadata may remain in read-only data. Trace sinks used by `sim` are a test feature and do not affect production builds.

### 24.4 Expansion transparency

Documentation MUST include a supported `cargo expand` workflow and one checked expansion snapshot of the Grok-like fixture. Generated identifiers use a `__kittens_` prefix and are doc-hidden. Runtime backtraces SHOULD preserve user handler spans wherever macro hygiene permits.

## 25. Compiler-diagnostic design guidelines and provisional catalog

The causal content rules in section 25.1 are stable. The numeric assignments, exact prose, validation precedence, cycle rotation, assertion helper names, and snapshot text in later subsections are starting hypotheses. They MUST remain revisable during the kernel repair pilot. Early tests snapshot semantic anchors—source IDs, violated relation, consequence, and primary span—rather than treating prose as a public compatibility surface.

### 25.1 Diagnostic rules

Every macro-owned error MUST:

1. begin with a stable identifier such as `KTR003`;
2. name declared source IDs/classes, never generated variant indexes;
3. point at the attribute or policy edge that introduced the violation;
4. state the violated contract in one sentence;
5. give one safe repair when semantics are unambiguous, or explain the distinct policy-preserving alternatives when they are not;
6. avoid category-theory terms and internal generic type names;
7. have an exact stderr fixture and a documentation page keyed by ID.

For an agent, this message is a reasoning channel. It MUST connect the local declaration to the operational consequence and the smallest safe repair; “trait bound not satisfied” without that causal bridge is insufficient when the macro owns the relationship. Diagnostics SHOULD state whether the repair preserves the existing policy, weakens it explicitly, or changes behavior. They MUST NOT encourage deletion of the declaration, addition of an unexplained waiver, or bypass through raw runtime code merely to make the next compile pass.

Stable `compile_error!` does not support a rich multi-span `help` object. The complete repair text MUST therefore be part of the error message. Where two declarations are involved, the macro MUST emit one primary error at the newest/conflicting declaration and include the other source ID and relation in text.

### 25.2 Macro diagnostic catalog

The following IDs and message templates were proposed for the maximal candidate. The kernel may reuse them as temporary fixture anchors, but they are not frozen until agent repair trials stabilize the taxonomy:

| ID | Condition | Message template |
|---|---|---|
| `KTR000` | invalid/unsupported reactor grammar | `KTR000 invalid reactor declaration near '{token}': {detail}. Repair: use the canonical grammar from the reactor reference.` |
| `KTR001` | duplicate source ID | `KTR001 duplicate reactor source id '{id}'. Repair: rename one #[source(...)] declaration.` |
| `KTR002` | unknown class/source reference | `KTR002 unknown {kind} '{name}' referenced by '{owner}'. Repair: declare it or correct the identifier.` |
| `KTR003` | priority/precedence cycle | `KTR003 reactor scheduling cycle: {path}. Repair: remove or reverse one listed priority/before relation.` |
| `KTR004` | multiple/conflicted last | `KTR004 source '{id}' cannot be last because {conflict}. Repair: keep one #[last] source and remove conflicting edges.` |
| `KTR005` | invalid shutdown branch | `KTR005 shutdown source '{id}' must be terminal, unguarded, and starvation-protected. Repair: remove #[when], use #[starvation(protected)], and return Exit.` |
| `KTR007` | starvation exposure | `KTR007 may-remain-ready source '{dominant}' can starve protected source '{victim}'. Repair: move '{victim}' earlier or add #[yields_to({victim}, when = buffered)] when it is backlog-probeable.` |
| `KTR008` | invalid drain | `KTR008 drain max for '{id}' must be an integer literal from 1 through 4096. Repair: provide a positive bounded literal.` |
| `KTR010` | invalid yield/cycle | `KTR010 invalid yield relation {path}. Repair: target a backlog-probeable source and remove mutual yield cycles.` |
| `KTR011` | phase mismatch | `KTR011 policy requires phase '{phase}' exactly once. Repair: add one {phase} block or remove it from policy phases.` |
| `KTR012` | incompatible/missing policy | `KTR012 unsupported reactor policy: {detail}. Repair: use selection: biased and a declared acyclic priority graph.` |
| `KTR014` | all sources guarded | `KTR014 reactor has no unguarded source and could enter Tokio's all-disabled path. Repair: keep shutdown unguarded or add another persistent unguarded source.` |
| `KTR015` | temporary source expression | `KTR015 source '{id}' must be a persistent path or field, not a temporary expression. Repair: construct the source before entering the reactor.` |
| `KTR016` | lexical order violates graph | `KTR016 source '{predecessor}' must appear before '{successor}' because of {relation}. Repair: move the complete source arm without changing its attributes.` |
| `KTR017` | missing/duplicate base contract | `KTR017 source '{id}' must declare exactly one #[{attribute}(...)] contract. Repair: add one valid declaration or remove the duplicate.` |
| `KTR018` | invalid starvation declaration | `KTR018 source '{id}' has an invalid starvation contract: {detail}. Repair: use #[starvation(protected)] or provide a specific nonempty allowed reason.` |

Numbers `KTR006`, `KTR009`, `KTR013`, and `KTR019` are reserved for generated rustc type checks below so documentation IDs remain stable.

`KTR014`'s original rationale predates the always-enabled pending sentinel of sections 19.2 and 20.2, which already prevents Tokio's all-disabled panic path in the control expansion. The K0 implementation retained `KTR014` as a conservative zero-poll guard check: under K0 guard semantics a fully `#[when]`-guarded reactor whose guards all snapshot false polls no source and registers no source wake in that arbitration, so the macro requires at least one unguarded arm (section 37.5). This is not a general liveness proof: it guarantees that an arm is polled, not that the arm is wake-capable, and a permanently dormant source does not repair the operational risk. The implementation also added `KTR020` for the section 37.5 duplicate-place rule, which this catalog had not numbered: `KTR020 source place is declared under both '{first}' and '{second}'. Repair: keep one source ID for this exact persistent place.` The decision and exact prose for both remain provisional pending the section 37.11 pilot.

Two condition-specific variants are also normative: `KTR005 shutdown source '{id}' cannot be #[last]. Repair: keep shutdown in the leading prefix and remove #[last].` and `KTR008 terminal source '{id}' cannot be drained because its first successful item exits. Repair: remove #[drain(...)].`

When one declaration violates several rules, the macro MUST validate and emit its primary error in this order: syntax (`KTR000`); duplicate/missing base declarations (`KTR001`/`KTR017`); unknown references (`KTR002`); shutdown/last/drain/temporary/starvation-declaration compatibility (`KTR004`, `KTR005`, `KTR008`, `KTR015`, `KTR018`); priority/precedence cycles (`KTR003`); yield validity/cycles (`KTR010`); lexical graph order (`KTR016`); unguarded progress (`KTR014`); starvation exposure (`KTR007`); then generated type assertions. Within one stage, source declarations are visited lexically and policy edges in written order. This precedence is part of snapshot stability: moving a shutdown arm below a stream reports `KTR016` before the derivative starvation exposure.

### 25.3 Type-check diagnostic anchors

The macro cannot know a source expression's type. It MUST emit small, one-bound assertion calls whose function names appear in rustc diagnostics:

- `assert_KTR006_declared_source_contract_matches`;
- `assert_KTR009_source_is_drainable`;
- `assert_KTR010_yield_target_has_backlog_probe`;
- `assert_KTR013_handler_result_type`;
- `assert_KTR019_guard_result_is_bool`;
- `assert_SRC001_reactor_source_is_restart_safe__repair_use_channel_task`.

A cancellation-unsafe integration should consequently contain a normalized diagnostic substring like:

```text
the trait bound `VendorSource: RestartSafeSource` is not satisfied
required by `assert_SRC001_reactor_source_is_restart_safe__repair_use_channel_task`
```

The exact surrounding E0277 wording varies by rustc and cannot honestly be frozen across MSRV/current stable. Compile tests MUST assert the Kittens anchor and relevant trait/type names on both toolchains.

### 25.4 Warnings

v0.1 does not synthesize compiler warnings through deprecated-item tricks. A semantic issue is either a macro error, a required explicit `#[starvation(allowed, reason = "...")]`, a runtime trace event, or a documented limitation. A future `cargo kittens lint` may add advisory findings without weakening compile semantics.

### 25.5 Rustc-native local errors

Kittens SHOULD rely on normal rustc errors where they are better:

- E0599 for a state-specific method called in the wrong typestate;
- E0382 for a consumed approval used again;
- E0308 for the wrong capability type;
- E0499 for two simultaneous mutable single-flight permits;
- E0597/E0521 for a borrowed value escaping into a `'static` child.

Documentation MUST show the smallest invalid program, the relevant diagnostic excerpt, and the repaired program.

## 26. Agent-oriented documentation guidelines

The principles apply during K0, but the complete documentation set and machine-readable schema below are publication candidates. K0 produces only the compact reactor/source guide, mutation repairs, non-guarantee page, and expansion walkthrough needed by its agent pilot. The lean-surface usage sketches in section 38 are the seed of that guide and of the canonical example set below; they follow every rule in section 26.2 except CI compilation, which becomes possible only after implementation authorization.

### 26.1 Documentation set

The release MUST include:

- a five-minute “first reactor” guide;
- a Grok-class long-lived harness guide;
- one canonical page each for source contracts, fairness, scope, cancellation, resources, capabilities, typestate, testing, and escapes;
- a diagnostic index keyed by every published Kittens identifier (`KTR` and `SRC` in v0.1, with other families added only when they are actually emitted);
- compile-pass and compile-fail examples side by side;
- a raw-Tokio migration cookbook;
- a `cargo expand` debugging guide;
- an agent-oriented compact reference no longer than necessary to retrieve canonical forms.

### 26.2 Canonical example rules

- Use harness scenarios: model tokens, tools, approval, human input, child agents, checkpoints, and render/status updates.
- Show one supported spelling first; alternatives and escapes appear later.
- Keep names consistent across guides (`scope`, `source`, `approval`, `Control`, `AuditReason`).
- Put cancellation and close behavior beside each adapter example.
- Include the invalid mutation that each constraint rejects.
- Do not hide required annotations with helper macros in introductory reactor examples.
- Examples MUST compile or intentionally compile-fail in CI; ellipses appear only in explicitly noncompiled conceptual fragments.
- The first example for a concept MUST be the canonical repair path, not a flexible “choose any of these APIs” menu. An alternative is documented only when it represents a real policy or ownership tradeoff and its diagnostic boundary is shown.

### 26.3 Machine-readable retrieval artifacts

The repository MUST ship `docs/agent-index.json` with a versioned schema containing:

- concept ID and canonical term;
- public items;
- one canonical example path;
- anti-pattern example path;
- relevant diagnostic IDs;
- guarantee and non-guarantee summaries;
- escape hatch, if any.

It MUST also ship a concise `docs/agent-guide.md` suitable for repository instructions. Generated API inventories MAY be derived from rustdoc JSON when stable enough, but the curated index remains authoritative for intended usage.

### 26.4 Error-driven repair docs

Each diagnostic page starts with “What happened,” “Why Kittens rejects it,” and “Canonical repair,” followed by alternatives and the explicit escape. Search terms MUST include the exact compiler message and source attribute names.

Repair documentation MUST also state what the diagnostic does not prove. This prevents an agent from generalizing a local topology check into a claim about arbitrary handler effects, external event order, or hardware behavior.

### 26.5 Vocabulary discipline

Documentation MUST use “source,” not interchangeably “branch producer,” “subscription,” or “event future.” It MUST use “scope” for structural task ownership and “capability” for authority. “Cancellation-safe,” “atomic,” “deferred,” and “cleanup” may not be used as synonyms.

### 26.6 Rehydration artifacts

Each consequential example and promoted profile API MUST carry enough local context for a fresh agent to reconstruct its operating boundary. The documentation set SHOULD link the source declaration, type/capability precondition, generated expansion or phase rule, compile-fail mutation, and reason-bearing exception in one retrieval path. A remote architecture page MAY explain the larger system, but a repair-critical invariant MUST NOT exist only there.

The repository SHOULD include a compact local “architecture curriculum” for each benchmark fixture: what the reactor is, which sources are protected, what may remain ready, which lifecycle owns children, what is intentionally outside Kittens, and which escapes are permitted. This curriculum is a retrieval aid; the executable declaration and oracle remain authoritative.

## 27. Agent ergonomics and Grok benchmarks

### 27.1 Success metrics

Kittens is not evaluated primarily by LOC. The benchmark records:

```text
comments carrying correctness invariants ↓
implicit scheduler assumptions ↓
runtime-only invariants ↓
illegal reorderings that compile ↓
cancellation hazards that compile ↓
agent-generated concurrency mistakes ↓
repair iterations ↓
machine-checkable constraints ↑
semantic explicitness ↑
compiler diagnostic quality ↑
agent success rate ↑
```

The benchmark additionally records:

```text
first-attempt correct repair rate ↑
constraint deletion or unjustified weakening ↓
invented API/alias usage ↓
time to identify the violated relationship ↓
diagnostic-to-repair causal accuracy ↑
semantic constraints rejected per added source token ↑
first-time local inferability ↑
agent rehydration success ↑
architecture-reconstruction retrievals ↓
invariants accidentally weakened after context reset ↓
escape surface (concurrency-relevant behavior outside the declared vocabulary) ↓
```

LOC, token count, compile time, and generic/type complexity are guardrail metrics rather than the objective. A larger source file is acceptable only when its additional declarations reduce the legal program space or improve repair enough to justify their context cost.

### 27.2 Comparative conditions

The kernel pilot MUST distinguish local information from compiler enforcement. Reactor tasks therefore run under four conditions:

1. **Raw Tokio:** idiomatic Tokio with access to official Tokio cancellation/select guidance.
2. **Annotated baseline:** the same source-local topology facts as the Kittens condition, expressed as structured comments or inert annotations, but no Kittens enforcement.
3. **Lean Kittens:** the section-37 kernel grammar, compact guide, and compiler diagnostics.
4. **Maximal Kittens:** the eight-base-declaration form retained in section 11, implemented only as much as needed for the ablation and never as a parallel production API.

The annotated condition isolates local-context value; lean versus maximal Kittens isolates semantic verbosity from ceremony. Broader future feature trials add their own best-practice baseline, such as application typestate, `JoinSet`, or an application-owned presenter.

The comparison is deliberately about burden placement. Raw Tokio asks the agent to recover policy from library contracts and surrounding code. The annotated baseline puts policy in the edit window without changing the legal program space. Lean Kittens adds only checks that consume that policy. Maximal Kittens tests whether additional explicit declarations provide independent integrity or merely make the source look more formal. If the annotated baseline performs as well as lean Kittens on a claimed invariant, the macro's value is not established by readability alone.

For the rehydration track, the annotated baseline is especially important: it tests whether local semantic context alone re-teaches the architecture, while lean Kittens tests whether enforcement prevents the newly rehydrated agent from making the corresponding illegal edit. A successful Kittens design should improve both reconstruction and constraint preservation, but it need not win every task on raw token count.

### 27.3 Task corpus

The list below is the eventual architecture corpus, not one release train. The kernel pilot includes only tasks 10 through 13 and 15 plus the reactor mutations in section 37. Tasks 1 through 9 and 16 are activated only when scope, resource, capability, protocol, or simulation is being considered for graduation. Task 14 begins as a comparison between an application-owned presenter and the generic-gate candidate.

1. create three concurrent scoped workers and collect results;
2. add a timeout without allowing children to outlive it;
3. ensure validation and approval precede execution;
4. reject execution without approval;
5. give one worker read authority but no network or shell authority;
6. narrow and delegate a filesystem capability;
7. implement the small typed orchestrator/worker protocol;
8. add operation-aware retry without corrupting cancellation semantics;
9. reproduce a schedule failure with scripted sources and a fixed seed;
10. repair an invalid program from compiler output alone;
11. build a model/input/shutdown reactor with starvation protection;
12. add a dynamically armed deadline and receiver without a closed-source loop;
13. add bounded model-stream draining that yields to buffered human input;
14. add coalesced single-flight status rendering with acknowledgement;
15. add before-poll state application and after-event rendering;
16. migrate a raw detached worker into a scope-owned task event source.

Each task has a hidden executable oracle covering both ordinary output and lifecycle state after success, error, cancellation, timeout, and injected failure.

### 27.4 Experiment protocol

- Use the same model build, reasoning setting, system prompt, repository context, tool permissions, and token budget across conditions.
- Record the exact model identifier and date; do not label an unspecified family simply “Codex.”
- Run the full release gate with the current target Codex build and repeat the canonical subset with at least one independently trained coding-model build when available; report cross-model variance.
- Begin with a small recorded repair pilot as soon as four diagnostics exist. Choose full-trial sample sizes after observing task variance and documenting a power rationale; ten heterogeneous trials do not automatically justify a pooled architectural claim.
- Begin from clean, isolated worktrees and block internet access except when the condition explicitly supplies documentation.
- Give the Kittens condition only the compact guide, compiler diagnostics, and normal source navigation after the initial prompt; do not hand it the repair.
- Stop after 10 compile/repair iterations or the common token budget.
- Preserve prompts, patches, rustc JSON, test output, expansion, token counts, elapsed time, and escape-hatch use as benchmark artifacts.
- Manually blind-review final programs for semantic errors that the executable oracle cannot detect.
- For rehydration trials, discard the creating agent's conversation and working notes before the repair agent starts; do not leak the intended invariant through the task prompt.
- For any later confirmatory release study, publish confidence intervals and per-task results. Label kernel-pilot results exploratory and do not treat a small point estimate as architectural evidence.

Required kernel metrics are first-attempt compile rate, final behavioral success, repair iterations, tokens, elapsed tool iterations, invented APIs, diagnostic size, invalid-program rejection, constraint deletion/weakening, audited-waiver use, semantic constraints rejected per added source token, compile time, and expansion complexity. Feature-specific lifecycle/resource metrics are added only to the corresponding later slice.

The benchmark adds a rehydration track. Agent A receives the full task and creates or repairs the fixture. Agent A's context is then discarded. Agent B receives only the repository, the next modification request, normal source-navigation tools, and the same compiler/test environment. The study records how many relevant invariants B states correctly, how many architecture-retrieval operations it needs, whether it attempts an invalid reordering or bypass, whether it weakens a declaration, and whether the final oracle still passes. Rehydration results are compared across raw, annotated, lean, and maximal conditions; they are not inferred from source length or documentation sentiment.

The harness itself is part of the product boundary. Every pilot artifact SHOULD preserve the compiler output, macro expansion, mutation, repair patch, and final oracle result together so a later reviewer can distinguish “the agent understood the policy” from “the agent guessed until tests passed.” A lower repair count that is achieved by deleting a declaration or accepting a waiver is not a success.

### 27.5 Existing Grok invariants versus Kittens

“Compile-time enforced” in this table always means *after the programmer declares the branch/relationship through the supported Kittens path*. The macro cannot discover that an unmarked branch is semantically shutdown, that a source ought to be last, or that an application still requires rendering. Section 37 adds negative controls that deliberately erase declarations so this boundary is measured rather than hidden.

| Invariant | Current Grok representation | Kittens classification |
|---|---|---|
| connection cancellation first | biased lexical order | compile-time enforced |
| graceful quit above ACP firehose | lexical order + explanatory comment | compile-time enforced |
| writer acknowledgement before new work | lexical order + presenter fields | order compile-time; ticket behavior runtime enforced |
| ACP yields to buffered terminal input | manual `input_rx.is_empty()` gate | compile-time required relationship; runtime backlog value |
| ACP drains at most 32 | constant + manual loop | macro-managed form has a compile-time literal and drainable bound; a manual handler loop remains unchecked |
| lower timers may wait behind ACP | implicit/observed ordering | explicit `starvation(allowed, reason)` |
| terminal receive survives lost races | dedicated reader thread + detailed cancellation comment | compile-time approved channel source; producer lifecycle runtime/scope |
| terminal handoff parks input before writer drain | manual protocol/order + atomics | encapsulated runtime protocol; simulation/Loom verified |
| optional timers/receivers become pending | `Option` + `pending()` convention | adapter type plus runtime dormant state |
| closed voice receiver does not hot-loop | manually set `None` | adapter runtime invariant |
| voice source is globally last | lexical order + comment | compile-time enforced |
| initial frame precedes polling | presentation call before loop | one-time phase placement; presentation permit remains provisional |
| loop-top terminal/deferred work | statement placement + comments | required `before_poll` phase |
| one render opportunity per successfully continuing handled batch | statement after `select!` | required `after_event` phase |
| draw requests coalesce | booleans/options | private gate runtime invariant |
| one frame in flight | optional sequence target + tests | application runtime state initially; generic gate is a comparison candidate |
| writer ack unlocks newer frame | monotonic sequence comparison | runtime gate + deterministic test |
| task completion ownership | mostly `JoinSet`, with a few raw spawns | later scope experiment; outside reactor-kernel guarantee |
| outer cleanup order | guard/drop/manual call order | application integration tests initially; later resource experiment |
| ACP message high-water/dedup | runtime IDs/state | runtime enforced and simulation verified |
| actual external event sequence | runtime | not reasonably enforceable statically |

There are no synthesized compile-time warnings in v0.1. Rows are either compile-time errors, explicitly accepted risks, runtime invariants, simulation properties, or non-enforceable facts.

### 27.6 Static coverage of requested reactor properties

| Property | Classification | Boundary |
|---|---|---|
| cancellation priority | compile-time for branches declared `shutdown` | graph/order validation cannot discover an unmarked cancellation branch |
| graceful shutdown priority | compile-time for branches declared `shutdown` | shutdown branch rules; declaration erasure is a negative control |
| starvation-sensitive streams | compile-time enforced for declared contracts | sealed readiness + protected/yield analysis; adversarial explicit rearming remains runtime |
| user-input responsiveness | compile-time topology + runtime readiness | every preceding may-ready source must yield; handler duration remains runtime |
| bounded stream draining | compile-time enforced for macro-managed drain | literal bound + drainable source; manual handler loops remain unchecked |
| cancellation-safe terminal input | compile-time admitted-source membership | producer isolation semantics remain audited/runtime-tested |
| dynamically enabled timers | compile-time adapter capability + runtime state | semantic contract is reviewed; arm/fire/disarm behavior is tested |
| dynamically enabled receivers | compile-time adapter capability + runtime state | semantic contract is reviewed; arm/close/dormant behavior is tested |
| source availability by reactor state | explicit runtime guard; simulation verified | compile-time state tables deferred because handlers hold runtime state |
| background-source priority | compile-time enforced | priority/order graph |
| source must be last | compile-time enforced | lexical and graph validation |
| render coalescing | runtime in the first fixture | compare application-owned state with a candidate gate |
| writer acknowledgement gating | runtime enforced | ticket comparisons cannot be static |
| one frame in flight | runtime in the first fixture | candidate permit borrowing is not yet established |
| before-poll work | compile-time phase presence/order | body semantics remain application code |
| rendering allowed only at initialization/after event | generated phase placement only in kernel | a candidate permit could restrict one participating gate; raw writer bypass remains possible |
| after-event rendering | compile-time phase presence/order | exactly once after a successful continuing event/batch |
| legal external event sequence | not reasonably enforceable | external dynamics |
| starvation caused by non-yielding handler | runtime watchdog/simulation | types cannot prove handler termination |

### 27.7 Grok mutation benchmark

This is the earlier full-architecture mutation catalog. Section 37.9 controls K0 and deliberately adds compiling negative controls. Scope, approval, resource, and generic-gate mutations remain dormant until those extensions are candidates for promotion.

The implementation MUST create a minimal Grok-like fixture and the following mutations. “Specified” means the expected behavior below is normative but has not been executed because this design phase contains no code. No release claim may mark it passed until the retained suite and diagnostic-only Codex trials run.

| Mutation | Expected outcome | Required diagnostic/behavior | Repair target |
|---|---|---|---:|
| move shutdown below `acp_stream` | compile-time failure | `KTR016 source 'connection_cancel' must appear before 'acp_stream' because of priority Shutdown > Stream.` | median ≤1 iteration |
| remove ACP's yield to protected input | compile-time failure | `KTR007 may-remain-ready source 'acp_stream' can starve protected source 'terminal_input'.` | median ≤1 |
| add `Stream > Interactive > Stream` | compile-time failure | `KTR003 reactor scheduling cycle: Stream -> Interactive -> Stream.` | median ≤1 |
| add mutual buffered yields | compile-time failure | `KTR010 invalid yield relation acp_stream -> terminal_input -> acp_stream.` | median ≤1 |
| race a reconstructed cancellation-unsafe future | rustc type failure | contains `RestartSafeSource` and `assert_SRC001_reactor_source_is_restart_safe__repair_use_channel_task` | median ≤2 |
| hold two submission permits simultaneously | rustc borrow failure | E0499 naming the `SingleFlight` mutable borrow | median ≤1 |
| begin again after committing a frame | compiles; runtime returns `Ok(None)` | deterministic test asserts no second in-flight ticket | no repair; correct dynamic behavior |
| writer accepts a request but returns `Err` | compiles; violates documented sink precondition | integration oracle observes duplicate/requeued logical submission; classified not statically enforceable | repair sink to report acceptance atomically |
| writer returns a reused/lower accepted ticket | compiles; runtime failure | gate returns `NonMonotonicSubmission`, becomes poisoned, and permits no later submission | median ≤1 |
| consume one approval twice | rustc move failure | E0382 naming `approval` | median ≤1 |
| spawn a borrowed child through `Scope` | rustc lifetime failure | E0597 or E0521 plus `'static` bound at `Scope::spawn` | median ≤1 |
| replace `Scope::spawn` with owned `tokio::spawn` | compiles outside Kittens | benchmark marks structural escape; future repository lint may reject | must identify limitation |
| omit release callback from `resource::using` | rustc arity failure | E0061; raw resource code outside Kittens still compiles | median ≤1 |
| place `#[last] voice_stt` before another arm | compile-time failure | `KTR004 source 'voice_stt' cannot be last because source '{later}' follows it.` | median ≤1 |
| build optional receiver with raw temporary `recv()` | compile-time failure in reactor | `KTR015` for temporary or `SRC001` for unapproved source | median ≤2 |
| close approved `OptionalMpsc` | compiles; runtime goes dormant | virtual-time test observes no repeated wake/selection | no repair; correct dynamic behavior |
| rearm an elapsed optional deadline every `before_poll` | compiles; simulation failure | bounded scenario shows repeated timer selection and missed protected work; explicit-rearm liveness is not statically provable | median ≤2 |
| omit required `after_event` | compile-time failure | `KTR011 policy requires phase 'after_event' exactly once.` | median ≤1 |
| bypass the gate and mutate private in-flight state | compile-time failure | field/method privacy E0616/E0599 | median ≤1 |

The diagnostic-only repair trial supplies only the invalid fixture and compiler output after each attempt. Codex is not given this table. Required iterations and whether the intended invariant survived the repair are recorded separately.

### 27.8 Agent benchmark release threshold

The numeric thresholds below are candidate minimum-value hypotheses for a later public release, not established evidence and not the kernel gate. The kernel pilot is designed to estimate repair behavior and expose bad metrics before confirmatory thresholds or sample sizes are frozen.

Before a stable v0.1 release:

- every mutation classified static MUST fail in the intended layer;
- at least 90% of diagnostic-only trials MUST reach the canonical repair within two iterations;
- Kittens MUST reduce semantically invalid final programs by at least 20% relative to raw Tokio on constraint-focused tasks, with the reported 95% interval excluding no improvement;
- valid-task final success MUST not be more than five percentage points below the better baseline;
- no more than 5% of successful Kittens trials may use an escape hatch unless the task requires one;
- the report MUST publish negative results and per-task data, not only aggregates.

If those thresholds fail, v0.1 remains pre-release and the API/diagnostics are revised. LOC reduction is not a release threshold.

## 28. Candidate full-architecture compile-pass examples

These sketches define retained fixtures to create only after implementation is authorized. Each fixture MUST compile on MSRV and current stable and MUST execute its runtime assertions where applicable.

### 28.1 Approval ordering and consumed authorization

```rust
async fn approved_run(
    command: Command,
    policy: &Policy,
    gate: &ApprovalGate,
    shell: &Shell,
) -> Result<Run<Finished>, HarnessError> {
    let run = Run::<Proposed>::new(command);
    let run = run.validate(policy).await?;
    let (run, approval) = run.request_approval(gate).await?;
    let finished = run.execute(shell, approval).await?;
    Ok(finished)
}
```

### 28.2 Read authority without network or shell

```rust
async fn reader_worker(read: ReadWorkspace) -> Result<Index, IndexError> {
    let mut file = read.open("Cargo.toml")?;
    let mut manifest = String::new();
    file.read_to_string(&mut manifest).await?;
    build_index(manifest)
}

let corpus = workspace.narrow("research/corpus")?;
let reader = scope.spawn("reader", move |_cancel| reader_worker(corpus))?;
let index = reader.join().await??;
```

No `Network` or `Shell` value enters `reader_worker`.

### 28.3 Three structurally owned children

```rust
let reports = kittens::scope::run(scope_config, async |scope| {
    let a = scope.spawn("agent-a", move |cancel| run_agent(agent_a, cancel))?;
    let b = scope.spawn("agent-b", move |cancel| run_agent(agent_b, cancel))?;
    let c = scope.spawn("agent-c", move |cancel| run_agent(agent_c, cancel))?;

    let (a, b, c) = tokio::try_join!(a.join(), b.join(), c.join())?;
    Ok::<_, HarnessError>([a?, b?, c?])
}).await?;
```

The runtime fixture MUST assert the scope registry is empty before `scope::run` returns.

### 28.4 Timeout plus cooperative release

```rust
let outcome = parent.with_timeout(
    Duration::from_secs(10),
    ScopeConfig::default(),
    async |child| {
        resource::using(
            child.cancellation(),
            ResourcePolicy::new(Duration::from_secs(1)),
            || connect_tool(),
            |tool| async { tool.run(request).await },
            |tool, exit| async { tool.close(exit).await },
        ).await
    },
).await?;

match outcome {
    TimeoutOutcome::Completed(value) => consume(value),
    TimeoutOutcome::TimedOut { body, shutdown } => {
        assert!(shutdown.registry_empty());
        inspect_post_deadline_body(body);
    }
}
```

The fixture injects cancellation during use and proves the release future completes before `TimedOut` when it stays within grace. A release error after the deadline appears as `BodyAfterCancel::Failed`; a body error before the deadline or parent cancellation takes the outer `ScopeRunError` path through `?` and is tested separately.

### 28.5 Binary typed protocol

```rust
let worker = Worker::<Ready>::new(tx, rx);
let worker = worker.send_task(Task::Research(query)).await?;
let (worker, report) = worker.recv_report().await?;
assert_eq!(report.task_id(), expected);
worker.shutdown().await?;
```

### 28.6 Dynamic dormant source

```rust
let mut voice = source::OptionalMpsc::new(source::close::Dormant);
assert!(voice.is_dormant());
voice.arm(voice_rx)?;
// The reactor consumes transcripts. Channel close transitions back to Dormant.
```

The paused-time fixture closes `voice_rx`, advances time, and proves poll/selection count does not increase until a new receiver is armed.

### 28.7 Audited escape

```rust
let reporter = kittens::escape::spawn_detached(
    AuditReason::new("process supervisor owns crash reporter lifetime")?,
    run_crash_reporter(),
);

reporter.join().await?;
```

This compiles and emits an escape audit event; its presence is counted by the benchmark.

## 29. Candidate full-architecture compile-fail examples

Compile-fail fixtures MUST use the smallest code that exposes one invariant. UI snapshots normalize paths, line numbers, and rustc-version prose but retain the Kittens ID, relevant type/source names, and primary error code where stable.

### 29.1 Execute before approval

```rust,compile_fail
let run = Run::<Proposed>::new(command);
run.execute(&shell, approval).await?;
```

Required core diagnostic: E0599, no `execute` on `Run<Proposed>`, with rustc noting the method exists for `Run<Approved>` if available.

### 29.2 Reuse approval

```rust,compile_fail
run_a.execute(&shell, approval).await?;
run_b.execute(&shell, approval).await?;
```

Required core diagnostic: E0382, use of moved value `approval`.

### 29.3 Wrong authority

```rust,compile_fail
call_model(&read_workspace, request).await?;
```

Required core diagnostic: E0308, expected `&Network`, found `&ReadWorkspace`.

### 29.4 Borrow escapes into child

```rust,compile_fail
let prompt = String::from("research");
let prompt_ref = &prompt;
scope.spawn("worker", move |_cancel| async move {
    use_prompt(prompt_ref).await
});
```

Required core diagnostic: E0597 or E0521 at `Scope::spawn`, with the child future's `'static` requirement visible.

### 29.5 Temporary repeated source

```rust,compile_fail
#[source(packet)]
#[priority(Stream)]
#[lifecycle(repeating)]
#[cancellation_safe]
#[readiness(may_remain_ready)]
#[close(dormant)]
#[starvation(allowed, reason = "vendor traffic is best effort")]
packet = vendor_reconstructed_future() => { /* handler */ }
```

Required diagnostic: `KTR015`, because a call constructs a new temporary at the branch site. The canonical repair first stores a reviewed persistent source before entering the reactor.

### 29.6 Persistent but unapproved repeated source

```rust,compile_fail
#[source(packet)]
#[priority(Stream)]
#[lifecycle(repeating)]
#[cancellation_safe]
#[readiness(may_remain_ready)]
#[close(dormant)]
#[starvation(allowed, reason = "vendor traffic is best effort")]
packet = sources.vendor_packets => { /* handler */ }
```

Here `sources.vendor_packets` is a persistent application wrapper that implements `Source` but not sealed `RestartSafeSource`. Required diagnostic anchor: `assert_SRC001_reactor_source_is_restart_safe__repair_use_channel_task`, with `VendorPackets` and `RestartSafeSource` visible. The canonical repair is `source::one_shot` for a retained one-shot or a scope-owned producer plus `source::mpsc`/`source::channel_task` for repetition.

### 29.7 Unbounded drain

```rust,compile_fail
#[drain(max = LIMIT)]
message = sources.model => { /* handler */ }
```

Required diagnostic: `KTR008`, because v0.1 requires a literal bound.

### 29.8 Priority cycle

```rust,compile_fail
priority {
    Shutdown > Stream;
    Stream > Interactive;
    Interactive > Stream;
}
```

Required diagnostic: `KTR003 reactor scheduling cycle: Stream -> Interactive -> Stream.`

### 29.9 Missing input yield

The fixture places `acp_stream` before protected `terminal_input`, declares ACP may-remain-ready, and omits `#[yields_to]`. Required diagnostic: `KTR007` naming both sources and the canonical buffered-yield repair.

### 29.10 Last source followed by another

The fixture places `#[last] voice_stt` before `telemetry`. Required diagnostic: `KTR004` naming both source IDs.

### 29.11 Two live submission permits

```rust,compile_fail
let first = present.try_begin(&mut gate, now).unwrap().unwrap();
let second = present.try_begin(&mut gate, now).unwrap().unwrap();
consume(first, second);
```

Required core diagnostic: E0499, because `first` retains the exclusive borrow until commit/drop.

### 29.12 Missing cleanup callback

```rust,compile_fail
resource::using(cancel, policy, acquire, use_resource).await?;
```

Required core diagnostic: E0061. This proves only that users of `using` must state release; raw code can opt out and remains outside this guarantee.

## 30. Superseded broad v0.1 scope

This all-at-once scope is retained as a record of the earlier architecture. It is not an implementation plan. Bundling the reactor with scope, authority, cleanup, public simulation, and rendering would delay the evidence most likely to falsify the core thesis. Section 37 replaces it with a kernel gate and separately promoted extensions.

### 30.1 Required deliverables

1. `kittens` and `kittens-macros` packages with MSRV/current-stable CI.
2. The exact `reactor!` grammar, validation, direct expansion, phase semantics, metadata, and diagnostic catalog in sections 11 and 25.
3. Sealed source contracts and every adapter in section 11.6, including dynamic dormancy, bounded drain, and backlog probes.
4. `Control`, `Batch`, and the `SingleFlight` gate with flag/latest coalescers and throttle deadline.
5. `scope::run`, `Scope`, `Task`, typed `TaskSpawner`/`TaskEvents`/`TaskCompletion`, hierarchical cancellation, timeout, and grace/abort/drain reports.
6. `resource::using` and its complete outcome/cleanup semantics.
7. Capability bootstrap, cap-std-backed read/write workspace authority, generic network/shell policy wrappers, narrowing, revocation, approval, and approved-command consumption.
8. The intentionally small `flow` helpers and app-owned typestate examples.
9. Scripted source scenarios, paused-time runner, trace/replay schema, fault injection, and Loom models for Kittens synchronization.
10. `escape` operations and audit telemetry.
11. Grok-like compile-pass fixture, all mutations in section 27.7, expansion snapshot, runtime protocol tests, and benchmark harness.
12. The complete agent-oriented documentation set and machine-readable index.

### 30.2 Required quality characteristics

- Kittens package source MUST use `#![forbid(unsafe_code)]`. Any discovered need for unsafe requires a separate reviewed design amendment; dependency internals are outside that lint.
- The production reactor path has no graph allocation, dynamic dispatch, registry lookup, or hidden spawn.
- Source IDs and topology metadata are static.
- Public handler signatures do not expose internal marker types or macro event enums.
- Ordinary application errors remain ordinary `Result` errors.
- All APIs are documented with cancellation and drop behavior.

### 30.3 Package name

The project name remains Kittens. Before publication, maintainers MUST recheck crates.io and perform appropriate legal/name review. If `kittens` is unavailable or confusing, the package fallback is `kittens-orchestrate`, while the Rust facade namespace remains `kittens` through dependency renaming. `kittens-reactor` is the second fallback. No trademark clearance is asserted.

## 31. Explicitly deferred features

The following are outside K0 even if prototypes are promising. Their omission does not reject the profile architecture; it keeps the kernel experiment from being confounded by domain-specific utilities. A future profile must build on the same semantic kernel and compiler rules rather than introducing a parallel scheduling language.

- a general `Effect<R, E, A>` or effect-row system;
- a Kittens async runtime or dynamic scheduler interpreter;
- fair, weighted, deficit-round-robin, or work-conserving scheduling policies beyond biased lexical polling plus buffered yield;
- automatic liveness proofs or handler time-budget enforcement;
- compile-time reactor active-state tables and generated transition permits; embedded Off/AOD/Game modes remain runtime guards because K0 does not own exhaustive application transitions;
- multiparty/full recursive session types and a `kittens::protocol` package;
- borrowed spawned tasks or any unsafe lifetime-erased scope;
- automatic fail-fast based on arbitrary heterogeneous child error types;
- a scoped `spawn_blocking` API that pretends running blocking work can be aborted;
- a public general uncancelable region;
- a general retry/schedule abstraction divorced from operation idempotency;
- arbitrary cancellation-safe acquisition claims;
- async-destructor emulation after outer future drop;
- durable approval tokens;
- automatic distributed idempotency or side-effect rollback;
- actor supervision trees beyond nested scopes;
- a general deterministic executor or exhaustive scheduler;
- automatic rewriting/linting of all raw Tokio usage;
- procedural-macro compiler warnings implemented through deprecated-item hacks;
- arbitrary custom safe implementation of semantic source marker traits;
- a prelude that glob-imports the Kittens vocabulary;
- production Embassy/ESP-HAL adapters, exact Waveshare board support, and hardware-in-loop claims;
- executor-neutral structured scope semantics;
- collected batches whose storage policy is implicit or silently depends on `alloc`.

The no-std/no-alloc **kernel compile boundary is not deferred**; it is a K0 architectural gate. Shipping useful embedded adapters remains deferred. Deferred features require a new specification section, retained prototype, diagnostic review, agent benchmark, and explicit versioning decision. They are not TODOs implementers may add opportunistically.

## 32. Migration path from raw Tokio

The reactor/source steps remain useful K0 guidance. Scope, protocol, authority, and simulation steps are candidate follow-ons and MUST NOT be inferred as one mandatory migration sequence.

Migration is incremental and preserves leaf libraries.

### 32.1 Inventory the orchestration contract

For each long-lived loop, record:

- every select branch and its lexical order;
- comments that constrain ordering/cancellation;
- whether it can stay ready;
- close behavior and dynamic enablement;
- current drain/coalescing bounds;
- shutdown and resource owners;
- tasks spawned by handlers;
- pre-select and post-select work;
- runtime protocol flags/options/tickets.

Do not start by wrapping calls mechanically. The Grok table in section 27.5 is the model.

### 32.2 Move event producers into persistent sources

Create a separate `Sources` owner. Convert Tokio receivers/timers/tokens with curated adapters. Replace `Option + pending()` with dynamic adapters. Isolate cancellation-awkward terminal/I/O producers behind a scope-owned channel task. Preserve existing runtime behavior and tests before adding stronger policies.

### 32.3 Declare the reactor without changing handlers

Translate the raw `select!` arm order directly into `reactor!`. Declare honest lifecycle/readiness/starvation metadata and required phases. Initially use `starvation(allowed, reason)` where the old code genuinely allowed it. The macro should expose missing assumptions rather than silently redesign scheduling.

### 32.4 Strengthen relationships one at a time

Add shutdown protection, source precedence, bounded drain, buffered yields, and `last`. Run the original integration tests after each relation. A relationship that changes runtime behavior needs a targeted deterministic fixture and an architecture note.

### 32.5 Adopt structural task ownership

Replace raw application `tokio::spawn`/discarded handles with `Scope::spawn`. Route completion through direct typed handles or a homogeneous `TaskSpawner<T>`/`TaskEvents<T>` group. Preserve raw spawn only under `escape::spawn_detached` with an owner/reason.

### 32.6 Encapsulate dynamic protocols

Move presenter booleans/options into `SingleFlight`, keep external ticket values runtime, and test ack/failure/retry order. Encapsulate terminal handoff and similar multi-step runtime protocols behind private APIs before trying typestate.

### 32.7 Add authority and workflow states

Replace permission booleans at side-effect APIs with concrete capabilities. Introduce consumed approvals and state-specific methods only at valuable workflow boundaries. Do not refactor unrelated data types into generic state machines.

### 32.8 Add deterministic scenarios

Reproduce shutdown-under-firehose, buffered input, closed optional sources, timer rearming, task refusal to stop, and stale writer acknowledgements. Store traces plus topology descriptors/fingerprints as test artifacts.

Leaf futures, Tower services, existing Tokio channels, and domain error types need not be rewritten. The constrained boundary is the long-lived harness.

## 33. Risks and tradeoffs

| Risk/tradeoff | Consequence | Mitigation/decision |
|---|---|---|
| proc-macro editing/IDE limitations | incomplete rust-analyzer assistance inside custom tokens | keep grammar small, Rust expressions ordinary, compile fixtures in docs, publish expansion |
| stable diagnostic API limits | no rich multi-span help/warnings | stable IDs in `compile_error!`, named trait assertions, diagnostic pages |
| explicit contracts can be verbose | longer loops and potential copy/paste mistakes | require only check-bearing fields, generate contract mismatch errors, benchmark semantic density |
| sealed source semantics limit extension | vendor integrations need an adapter | persistent one-shot, scoped channel isolation, visible unchecked escape, contribution path |
| declared readiness may be conservative | more yield/accepted-starvation annotations | prefer conservative safety; add reviewed specialized adapter only with evidence |
| macro cannot prove two arbitrary guards are mutually exclusive | starvation analysis can reject a safe state-specific ordering | reorder protected source, split reactors by state, or defer state-table support; v0.1 has no trusted exclusivity annotation |
| biased select is not general fairness | protected sources need gates/reordering; best-effort sources may starve | explicit protected/allowed classification; defer custom fair scheduler |
| lexical order still affects incomparable sources | a legal reorder may change runtime behavior | add `#[before]` for correctness, trace topology fingerprint/descriptor, mutation tests |
| handler await blocks reactor polling | latency failures remain possible | spawn long work in scope, handler-duration traces, simulation/watchdogs |
| source borrow complexity at Grok scale | expansion may trigger confusing borrow errors | separate `Sources`, event enum ends borrows before handlers, retained Grok compile fixture |
| macro compile time | agent repair loop slows | linear graph algorithms, no deep generic recursion, compile-time budget |
| scopes accept only `'static` spawned children | some borrowed concurrency cannot be spawned | ordinary local `join!`; reject unsafe lifetime erasure |
| scope future can itself be dropped | async drain cannot be universal | synchronous abort guard and precise guarantee language |
| acquisition cancellation is deferred | hung acquisition delays cancellation | time-bound acquisition at operation layer; future curated safe acquisition only |
| capability values are not a sandbox | raw std/Tokio bypass remains possible | explicit disclaimer, process/WASI sandbox for hostile code, repository policy |
| runtime protocol state is not compile-time | ticket/generation bugs remain | private state, consuming permits, assertions, deterministic tests |
| trace payload may leak sensitive data | privacy/security exposure | IDs/categories only by default, explicit redaction policy |
| exact crate name has ecosystem collisions | confusion with Typelevel Kittens/Kitty/ThunderKittens | publication recheck and package fallbacks |
| broad v0.1 surface | implementation and audit cost | ordered gates in section 35; no deferred-feature creep |

The largest strategic risk is semantic theater: annotations that appear rigorous but are not tied to generated checks or private runtime enforcement. Acceptance review MUST delete such declarations or implement the check before release.

## 34. Recorded feasibility questions

These questions demonstrate why the earlier public surface is not fixed. Section 37 pulls the reactor/source questions into the first evidence gate and defers unrelated subsystem questions until those features are candidates for promotion.

### 34.1 Earlier broad feasibility list

1. **Grok-scale expansion and borrowing.** Does the separate `Sources` plus private event enum compile for a 23-arm method borrowing `self`, sources, scope, and presenter? Required fallback: split only the selected item from source borrows before entering handlers; do not introduce a runtime scheduler.
2. **MSRV async-closure signature.** Confirm `for<'scope> AsyncFnOnce(&'scope Scope)` ergonomics on Rust 1.85. Required fallback if the exact public spelling is not expressible: a `ScopeRunner::run(async |scope| ...)` façade with the same call site and hidden helper trait, not boxed user futures.
3. **Trait-error repairability.** Compare associated-marker equality assertions with purpose-specific extension traits. Choose the form whose rustc output contains the source concrete type and `SRC001/KTR006` anchor; do not expose nested marker tuples.
4. **Macro span behavior.** Confirm each graph error points at the conflicting attribute on MSRV/current stable and rust-analyzer expands the full fixture.
5. **Drain expansion.** Confirm `mode = each` can reuse a handler async block per item without moving captures incorrectly and that early `Stop` skips `after_event` for the whole batch. Required fallback: generate a private async closure borrowing captures, not a boxed/dynamic handler.
6. **Single-flight borrowing.** Confirm submission drop-requeue, error paths, and `Ticket: Ord` diagnostics without unsafe code.
7. **Script-source wake race.** Loom-model the shared scenario coordinator publishing/closing a scripted source while the reactor registers its waker; no lost wake or self-wake loop is acceptable.
8. **Scope closing race.** Loom-model direct and task-group spawn versus group/scope close so a task is either registered in every required registry and drained or left unspawned, never lost.
9. **Dependency/MSRV floor.** Resolve the minimum versions and feature sets in section 10.3 on Rust 1.85 for both Tokio runtime flavors. If a named dependency floor is incompatible, select the newest API-compatible MSRV-supporting release, record the exact reason/version in the source ledger, and amend section 10 before implementing public behavior; raising the MSRV is not an automatic fallback.

10. **Cooperative-budget attribution.** Can budget state be instrumented at arbitration and drain-item boundaries cheaply enough to hold the equivalence oracle at equal budget? Required fallback if not: report the oracle as valid only for single-item arbitrations and treat drain-window equivalence as unestablished; do not call `unconstrained` to force agreement.
11. **Expansion scaling.** Does the 23-arm fixture compile within default `recursion_limit`/`type_length_limit`, and how do expansion tokens, monomorphized type length, and compile time scale from the probe fixture to 23 arms? Required fallback: report the supported arm ceiling as a public limitation rather than raising limits silently in fixture configuration.
12. **Feature-unification cleanliness.** Does the kernel path link free of `std`/allocator/runtime symbols with the Tokio feature enabled elsewhere in the same graph? Required fallback: split `kittens-core` out of the facade and record the split as evidence-driven, not preference-driven.

These checks are evidence needs, not permission to continue through a predetermined architecture. A failed core reactor check can require a different macro surface or source boundary, not merely a preselected fallback. Checks for deferred subsystems run only when that subsystem is separately authorized.

### 34.2 Post-v0.1 research

- Can a work-conserving fairness policy compile into still-boring Tokio code and outperform explicit buffered yields for model/tool/input reactors?
- Can priority peers be selected fairly without a runtime interpreter or surprising nested selects?
- Do compile-time active-state tables and transition permits materially improve agent success over runtime enums plus `#[when]`?
- Can an extension certification mechanism permit third-party safe source adapters without turning a semantic claim into an unchecked public trait?
- Would a `cargo kittens lint` reliably find raw detached spawns, raw repeated races, or missing audit reasons across macro expansion? Under section 2.1 this question is re-weighted: the lint's product is an escape-surface report — the measured share of concurrency-relevant behavior outside the declared vocabulary — and it graduates first among post-K0 extensions together with the deterministic scenario layer, because total coverage is bounded by escape surface rather than check strength.
- Does a binary protocol helper outperform app-owned typestate in diagnostics and compile time?
- How should Kittens adapt if stable Rust gains async drop or language-level structured concurrency?
- Which external simulator provides the best agent-facing schedule diagnostics after scripted sources: Shuttle, MadSim, Turmoil, or a combination?
- Does constraint-revealing verbosity improve generation because architecture is locally recoverable, and where does it cross into distracting boilerplate?
- Can readiness contracts become less conservative through bounded-capacity/bounded-producer proofs without complex generics?

Every post-v0.1 experiment is judged by the API quality questions in the original brief: invalid program excluded, simpler alternative, diagnostic shape, familiarity, repair difficulty, generic spread, and Tokio coexistence.

## 35. Superseded breadth-first implementation sequence

No step in this historical sequence is authorized. The current workspace must remain documentation-only, and section 37 controls any future implementation order.

### Phase 0 — freeze fixtures and feasibility probes

- Convert sections 27–29 into test fixture manifests without implementing behavior.
- Recreate the historical local typestate/capability/protocol experiments in a retained test workspace.
- Run the feasibility checks in section 34.1 as minimal retained prototypes (checks 1–9 existed when this sequence was written; checks 10–12 were added by the executor-boundary review and belong to the section-37 gate).
- Record MSRV/current-stable diagnostics and expansion.

Exit gate: no unresolved v0.1 syntax, borrow, or diagnostic blocker; any fallback is incorporated into `SPEC.md` before Phase 1.

### Phase 1 — packages, markers, and dynamic sources

- Scaffold `kittens`/`kittens-macros` with CI and feature matrix.
- Implement hidden marker equality checks and public source traits.
- Implement mpsc, cancellation, Notify, watch, deadline, one-shot, optional, interval, and typed task-group event-source tests.
- Loom-model the shared scripted-source coordinator separately from exclusively armed production optional sources.

Exit gate: source compile-pass/fail suite, no hot-loop tests, and cancellation-lost-race tests pass.

### Phase 2 — macro parser and topology validator

- Parse the exact restricted grammar.
- Validate IDs, graph cycles, lexical order, last, phases, drain literals, guards, yield graph, and starvation.
- Implement stable diagnostic snapshots before runtime expansion.

Exit gate: every macro-owned mutation produces its specified ID/message/span on MSRV and current stable.

### Phase 3 — direct Tokio expansion

- Emit contract assertions, private event enum, loop, biased select, drains, handlers, phases, and control propagation.
- Preserve lexical source order and source spans.
- Add expansion snapshots and the full 23-arm Grok-like fixture.

Exit gate: generated code contains no runtime graph/hidden spawn, passes clippy/tests, and the Grok fixture borrow-checks.

### Phase 4 — single-flight rendering protocol

- Implement coalescers, submission permit, monotonic acknowledgement, throttle deadline, and trace events.
- Port the inspected Grok presenter test cases conceptually: coalescing until ack, sticky/latest pending request, immediate/late/stale ack, no-output/error submission, and last payload gating.

Exit gate: compile-fail simultaneous permit and deterministic runtime ticket suite pass.

### Phase 5 — scope and timeout

- Implement registry, tasks, observer-only cancellation, events, the fixed close/grace/abort/drain sequence, and nested timeout.
- Loom-model direct/group spawn, group-drop, scope-close, and cancellation races.
- Test normal, error, panic, outer drop, child refusal, and timeout.

Exit gate: no Kittens-spawned task survives normal scope return; reports preserve every structural failure.

### Phase 6 — resource

- Implement deferred acquisition, cancelable use, bounded deferred release, result precedence, and integration with nested timeout.
- Add cancellation matrix from section 14.6.

Exit gate: every outcome table cell is tested and docs never overstate outer-drop/panic guarantees.

### Phase 7 — capabilities and flow

- Implement audited bootstrap, cap-std workspace handles, narrowing, generic network/shell backends, revocation, approval binding/consumption, and restoration helpers.
- Add the exact local compile errors and bypass disclaimer tests/docs.

Exit gate: target-binding runtime tests and capability/approval compile-fail suite pass.

### Phase 8 — simulation, observability, and docs

- Implement scenario coordinator, scripted sources, paused runner, trace/replay topology descriptor/fingerprint, fault injection, and optional tracing.
- Publish diagnostic pages, agent guide/index, migration cookbook, and expansion guide.

Exit gate: fixed seeds reproduce, mismatched topology fails loudly, docs/examples are CI checked.

### Phase 9 — Grok and agent acceptance

- Run every mutation, runtime protocol scenario, and comparative agent task.
- Inspect repaired expansions and final semantic correctness.
- Publish results and revise pre-release APIs if thresholds fail.

Exit gate: all section 36 criteria pass; only then tag stable v0.1.

## 36. Candidate eventual release criteria

These criteria describe the earlier full release. They are not the readiness definition for the first implementation slice. Section 37 defines the kernel evidence gate; later features import and revise only the relevant criteria when promoted.

### 36.1 Earlier specification-readiness test

The earlier design called itself ready to enter Phase 0 if:

- all 36 required sections are present;
- every v0.1 public concept has ownership, cancellation, error, and drop semantics;
- the macro grammar, attributes, validation algorithm, expansion order, phase behavior, and diagnostics are fixed;
- Grok's 23 sources, render protocol, terminal input, dynamic sources, tasks, and shutdown are mapped;
- static/runtime/simulation/non-enforceable boundaries are explicit;
- MVP, deferred features, fallbacks, and implementation gates do not conflict;
- the workspace contains only research/specification documents and no Kittens implementation.

### 36.2 Compile-time conformance

- Every static mutation in section 27.7 fails at the intended layer.
- Every compile-pass fixture in section 28 builds on Rust 1.85 and current stable.
- Every compile-fail fixture in section 29 contains its diagnostic ID/type/error-code anchor.
- Source contract declarations cannot disagree with adapter marker types.
- All graph validators are deterministic and linear or near-linear in source/edge count.
- The 23-arm fixture compiles without application-visible generated generic types.
- No generated diagnostic mentions tuple indexes or opaque internal event variants.

### 36.3 Runtime lifecycle conformance

- Optional closed sources remain dormant under paused time and do not self-wake.
- Buffered yield applies before selection and between every drained item.
- Drains never exceed their literal bound or cross a rearmed source generation, including close/yield/error/stop races.
- `after_event` runs exactly once after every continuing event/allocation-free service window and never after stop, handler error, or panic.
- Scope normal return leaves an empty registry; grace/abort/drain behavior matches reports.
- Typed task groups deliver each successful or failed output at most once, and dropping their event source closes/cancels the group without detaching tasks.
- Scope cancellation continues polling the body during grace, records `BodyAfterCancel`, and preserves a cooperative cleanup error.
- Timeout returns only after nested scope shutdown completes.
- Resource outcome and cancellation matrices match section 15.
- Single-flight never has two in-flight tickets, requeues dropped/rejected submissions, handles no-output completion, coalesces requests, rejects stale unlocks, and poisons on a non-monotonic accepted ticket.
- Capability operations validate revocation/target/policy immediately before mediated effects.

### 36.4 North-star benchmark conformance

- The fixture preserves the documented source order and all 23 source IDs.
- Shutdown/quit cannot be starved by declared firehoses.
- ACP drains at most 32 and yields to buffered terminal input.
- Terminal input uses an admitted selection-loss-preserving facade and a bounded service window in the Kittens exercise.
- Every timer/optional receiver has explicit dynamic/dormant semantics.
- Voice is final and explicitly best effort.
- Initial presentation occurs in the one-time `initialize` phase; no phase permit is required in K0.
- loop-top work and post-event presentation are generated phases.
- render request/submit/ack/coalescing scenarios match the inspected protocol's invariants.
- terminal/writer/agent/process teardown order is covered by integration tests, with async-cleanup limits documented.
- The embedded-shape counterfixture compiles without `std` or `alloc`, retains an interrupt-like source across a lost race, exercises dormant mode changes and an ownership-returning completion, and does not claim exact hardware support.
- Existing HAL ownership failures are labeled as Rust/HAL baselines, not Kittens guarantees.

### 36.5 Expansion and performance conformance

- `cargo expand` shows one direct lexical core-poll path for the leading candidate and one direct biased Tokio-select control, with explicit handler match and no hidden spawn/executor/runtime scheduler.
- Disabled tracing performs no formatting or heap allocation.
- K0 per-item drain adds no allocation. Collected batch modes are deferred until their storage policy is explicit.
- On a pinned CI runner, incremental check time for the 23-arm Kittens fixture MUST be no more than 25% slower than its checked raw-Tokio counterpart after dependencies are built; clean-build data is published but not used to hide incremental cost.
- Macro peak memory and expansion token count are recorded; a regression over 20% requires review.

### 36.6 Diagnostic and agent conformance

- Exact macro messages in section 25 are snapshot-tested on both toolchains.
- Each diagnostic page's canonical repair removes the error without weakening an unrelated invariant.
- Section 27.8 benchmark thresholds pass and raw/escape usage is reported.
- A diagnostic-only Codex run repairs the cycle, missing yield, unsafe source, last placement, approval reuse, borrowed spawn, missing phase, and submission-permit mutations within the stated median targets.
- No benchmark result claims a static guarantee for raw Tokio bypass, external event order, handler termination, or arbitrary async cleanup.

### 36.7 Safety, compatibility, and documentation conformance

- Kittens packages contain no unsafe code unless this specification is explicitly amended and independently audited.
- Miri (where applicable), Loom models, clippy with warnings denied, rustdoc links, doctests, UI tests, integration tests, and feature matrix pass.
- Tokio current-thread and multi-thread runtime tests pass.
- Public API docs state cancellation, close, panic, and outer-drop behavior.
- The agent index covers every public concept and diagnostic.
- Name availability/legal review is repeated immediately before publication.
- No implementation claim is made before all mandatory evidence is retained in the repository.

The earlier candidate treated full-v0.1 acceptance as binary. Section 37 does not inherit that breadth: it first tests whether the smaller reactor kernel provides enough constraint value to justify any broader release.

## 37. Controlling implementation contract: K0 reactor kernel

This section supersedes the earlier implementation scope and sequence. K0 is an unpublished, reversible evidence slice. It is large enough to exercise both the Grok-class orchestration problem and the embedded no-std/interrupt ownership pressure, yet small enough that failure can reshape the architecture before unrelated modules create sunk cost.

K0 is not a stable v0.1 release. Passing K0 authorizes a design decision about the reactor/source center and its executor boundary; it does not automatically authorize an Embassy backend, scope, resources, capabilities, rendering abstractions, simulation, observability, or publication.

K0 also tests the foundation for the profile architecture. It must leave enough semantic surface in the `no_std` kernel for later TUI, embedded, and agent-harness profiles to share source/topology laws, while keeping profile-specific side effects and resource protocols outside the kernel until independently demonstrated.

### 37.1 What K0 must learn

K0 answers one question:

> Can a familiar Rust macro plus typed persistent sources make meaningful global properties of a Grok-scale and embedded-shape reactor fail locally at compile time, while compiling to one ordinary future with normal borrowing/pinning, understandable expansion, and agent-repairable diagnostics?

The slice must expose the design immediately to:

- an ordinary `async fn` using `self`, separate source storage, local state, `.await`, `?`, and `Result`;
- a 23-arm biased selection shape with dynamic sources, a firehose, human input, writer acknowledgements, timers, task completion, and a deliberately last source;
- a no-alloc embedded shape with a dynamic deadline, interrupt-like input, dormant state, and ownership-returning completion;
- real proc-macro spans and rustc trait errors;
- direct core polling, a Tokio-select control expansion, and lost-race/wake behavior;
- `no_std` compilation without claiming a production Embassy/HAL adapter;
- intentionally broken topology and source mutations;
- first-time coding-agent repairs;
- `cargo expand`, rustfmt, and rust-analyzer.

Behavioral oracles are fixed during K0. Exact public syntax, trait names, generated helper structure, diagnostic numbering, and package version are reversible until the gate closes.

### 37.2 Stable decisions imported into K0

The following have enough evidence to constrain the slice:

1. Tokio is the only K0 production runtime integration. The reactor/source base targets `core`; Embassy/ESP-HAL integration is deferred.
2. Selection is explicitly biased and lexical arm order is preserved. The leading candidate polls sources directly inside one future; the direct Tokio-select expansion is the behavioral/performance control. Kittens does not implement an executor or task scheduler.
3. Global declared relations require token-level procedural-macro analysis. Rust trait bounds prove only that a source belongs to the reviewed admission set and exposes requested capabilities; adapter semantics remain an audit-and-test obligation.
4. A selectable source is stably stored before the loop. Another source winning MUST NOT destroy the operation/event state required by its documented delivery contract. This does not imply that destroying the reactor preserves the operation or that cancellation is externally repeat-safe.
5. Safe reactor admission is sealed to reviewed adapters during K0. An arbitrary future is not accepted merely because a user labels it safe.
6. Dynamic optional adapters own dormant/armed/closed state. Dormant polling returns `Pending` without self-waking.
7. Buffered yield and bounded drain change generated runtime behavior; they are not comments.
8. `before_poll` and `after_event` are generated control-flow positions. The macro does not infer what their bodies mean.
9. External event order, guard truth, handler termination, manual handler loops, and render tickets remain runtime/application facts.
10. Expansion contains no hidden spawn, runtime graph interpreter, dynamic dispatch, boxing solely for arbitration, topology allocation, or executor-specific dependency in the core-poll path. A runtime adapter's disclosed pin-storage strategy is measured separately and is not disguised as macro overhead.
11. The kernel vocabulary remains profile-neutral: a future profile may add reviewed domain adapters and protocols, but K0 does not encode terminal, display, GPIO, model, tool, or power semantics into the core.

### 37.3 Required implementation boundary

After explicit code authorization, K0 contains only:

- package `kittens-macros`, containing the reactor parser, validator, and expansion;
- package `kittens`, re-exporting the macro and containing the minimum `reactor` and `source` runtime support;
- `reactor::Control`, imported unchanged from section 11.4 with exactly the variants `Continue` and `Stop(T)`, and only the internal helpers required by the selected expansion;
- one conservative approved-source contract, readiness metadata, and optional drain/backlog capability traits;
- a candidate `no_std`, no-alloc base containing only source polling contracts, `Control`, markers, and generated support;
- feature-gated Tokio adapters for cancellation, mpsc with a static dormant-or-emit-once close policy, optional mpsc, optional deadline, and retained one-shot/optional one-shot behavior. K0 begins with `Unpin` outer adapter objects; an adapter may use disclosed internal pin storage where its primitive requires it, but the no-alloc embedded fixture may not;
- crate-private controllable sources and Tokio paused-time support for tests;
- one raw-Tokio Grok fidelity fixture, one Kittens Grok-shape fixture, one host-only embedded-shape fixture, mutation fixtures, and expansion snapshots;
- the lean/maximal/annotated agent-repair ablation.

Notify, watch, interval, `JoinSet`, channel-task helpers, typed task groups, public scripted sources, and batch collection are added only if the fixtures cannot represent an essential K0 oracle without them. Each addition requires a one-sentence statement of the missing oracle; convenience is insufficient. The Grok-shape fixture MAY model watch, interval, and Notify-backed Grok sources with the minimum adapter set (mpsc, optional mpsc, optional deadline, one-shot) because the fixture tests topology shape, not primitive parity; each such substitution is recorded in the fixture manifest.

K0 toolchain policy: development and CI run on current stable Rust; the slice records which kernel parts also compile on Rust 1.85 but publishes no MSRV. The implementation lockfile is recorded at implementation start. Dependency floors follow the named semver lines in section 10.3 for the dependencies K0 actually uses (Tokio, tokio-util, `syn`/`quote`/`proc-macro2`/`proc-macro-crate`, `trybuild`, Tokio test utilities); excluded subsystems contribute no dependencies to the slice.

K0 explicitly excludes:

- `scope`, task groups, timeout, and spawn/detach APIs;
- `resource`;
- `cap`, approvals, `flow`, and protocol helpers;
- a generic `SingleFlight` or presentation-permit API;
- public `sim`, replay formats, topology fingerprints, tracing schemas, and Serde;
- Tower, cap-std, process backends, and publication-oriented feature matrices;
- stable escape APIs;
- public Embassy, ESP-HAL, display, graphics, DMA, framebuffer, or power-management adapters.

The fixtures may use ordinary `JoinSet`, explicitly owned Tokio tasks, and an RAII thread owner. Those remain visible application mechanisms and carry no Kittens guarantee.

### 37.3.1 Feature unification and the no-std gate

Section 0.1 keeps the kernel and the Tokio integration behind features in one facade, and section 37.14 makes bare-metal linking a stable K0 gate. Those two decisions meet at a Cargo behavior the specification has not yet named: **features are additive and unified across a dependency graph**. If any crate in a build enables the Tokio integration, it is enabled for every consumer of `kittens` in that build. A `default-features = false` declaration in one manifest does not protect a workspace where another dependency enables the integration.

The consequence is that "the kernel is `no_std` when you do not enable Tokio" is not a property a caller can rely on by discipline. K0 MUST therefore satisfy the stronger form:

- the kernel path — source polling contracts, `Control`, markers, and generated arbitration support — MUST remain free of `std`, allocator, and runtime-adapter symbols **even when the Tokio integration feature is enabled in the same graph**. Enabling an integration MAY add adapter modules; it MUST NOT change the kernel's own compilation or link surface;
- the bare-metal gate in section 37.14 MUST be run in at least one configuration where the Tokio feature is simultaneously enabled by another member of the build graph, not only in a clean `--no-default-features` build. A gate that passes only in isolation does not establish the property the profile architecture depends on;
- if that stronger form cannot be met inside one facade, it is evidence for the `kittens-core` package split rather than a reason to weaken the gate. K0 records which outcome occurred; section 0.1's provisional package split is decided by this result and by agent-discoverability evidence, not by aesthetics;
- feature names, not only their behavior, are part of the agent surface. A feature whose name implies the kernel is unavailable when it is merely unaugmented is a discoverability defect and is recorded in the ablation.

This is a build-system falsifier, not a topology one. It cannot invalidate architecture B, but it can invalidate the single-facade packaging that sections 0.1 and 10.1 currently prefer.

### 37.4 Declaration admission ledger

The lean K0 surface starts with the following declarations. Every entry names the check or runtime effect that earns its syntax.

| Declaration | Consumed by | Program/effect difference | K0 decision |
|---|---|---|---|
| `selection: biased` | expansion and ordering validator | emits deterministic top-to-bottom local polling and activates starvation analysis | required despite being the only K0 mode because it states an operationally dangerous choice |
| `source(id)` | graph, trace in tests, diagnostic | supplies stable local identity; duplicates are rejected | required |
| `shutdown` | graph and handler typing | generates edges before all non-shutdown arms and requires terminal success | supported |
| `terminal` | handler typing | successful handler exits instead of continuing | supported |
| `before(other)` | precedence graph | rejects cycles and lexical order that violates the edge | supported |
| `last` | graph | rejects every later arm | supported globally only |
| `readiness(...)` | starvation analysis plus generated trait assertion | a declaration that mismatches the sealed adapter marker fails; the marker's truth remains a maintainer audit/test obligation | required in the first lean comparison |
| `starvation(allowed, reason = "...")` | starvation validator | explicitly waives the default protection for that source | supported as an audited weakening; nonempty text is checked, prose quality is not |
| `when(expr)` | generated poll precondition | enables/disables a source at runtime | supported; correctness/purity remains asserted by the application |
| `yields_to(other, when = buffered)` | guard generation plus backlog trait assertion | disables the source before selection and between drained items while target backlog exists | supported, one target per source in K0 |
| `drain(max = N)` | parser, stable-drain capability assertion, expansion | handles at most `N` items including the selected item without permitting adapter rearm/replacement during the window | supported in per-item mode only |
| required phase list | phase validator | deleting a required block alone fails | supported for `initialize`, `before_poll`, and `after_event` |

The lean readiness declaration accepts exactly two tokens: `may_remain_ready` and `quiescent`. `quiescent` is the lean spelling of the quiescent-after-event contract defined in section 8; the maximal ablation grammar retains the longer `quiescent_after_event` spelling from section 11.3 so the two conditions remain visually distinct in the ablation. The lean parser accepts no other readiness token and no synonym, and the generated trait assertion checks the declared token against the sealed adapter marker exactly as in section 4.14.

The following maximal-candidate declarations are not admitted to lean K0:

| Declaration | Reason for exclusion |
|---|---|
| `lifecycle(...)` | adapter behavior is not used by a demonstrated global kernel rule |
| `cancellation_safe` | every admitted source already has the unconditional sealed source bound; the label could falsely appear to establish safety |
| `close(...)` | close/dormant behavior belongs to the adapter and runtime test in K0 |
| `priority(Class)` and class DAG | Grok establishes exact critical source relations, not the need for a class abstraction |
| `last(within = Class)` | depends on unproven priority classes |
| batch drain mode | adds collection/capture behavior not needed by the ACP oracle |
| phase capability binding | no promoted API consumes it during K0 |

This ledger is reviewed after the agent ablation. A declaration with no observed enforcement, runtime, diagnostic, testing, or comprehension value is removed rather than rationalized after the fact.

K0 starvation analysis is deliberately direct rather than transitive. Every source is protected unless it carries `starvation(allowed, reason = "...")`. For each protected victim, every lexical predecessor declared `may_remain_ready` must declare a direct buffered yield to that victim; otherwise the macro rejects the topology and suggests moving the victim above the predecessor or adding that yield. User guards are ignored because their runtime truth is not static. A predecessor's own starvation waiver does not exempt harm to another protected source. A reviewed `quiescent` predecessor is exempt. Shutdown sources are separately forced into the unguarded leading prefix.

### 37.5 Lean probe grammar

The first implementation attempt is an expression-position macro inside an ordinary `async fn`. This leaves the Rust function signature, generics, attributes, and return type visible to rust-analyzer even if reactor parsing fails. The previous item-position whole-function form is the comparison fallback, not the default.

The concrete probe shape is:

```rust
async fn run(&mut self, sources: &mut Sources) -> Result<Exit, AppError> {
    kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [initialize, before_poll, after_event];
        }

        initialize {
            self.presenter.request(false);
            self.presenter.present_if_dirty()?;
            Ok(())
        }

        before_poll {
            self.apply_deferred_work().await?;
            Ok(())
        }

        #[source(cancel)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.cancel => {
            Ok(Exit::Cancelled)
        }

        #[source(model_stream)]
        #[readiness(may_remain_ready)]
        #[yields_to(terminal_input, when = buffered)]
        #[drain(max = 32)]
        token = sources.model_stream => {
            self.handle_token(token).await?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(terminal_input)]
        #[readiness(may_remain_ready)]
        input = sources.terminal_input => {
            self.handle_input(input).await?;
            Ok(kittens::reactor::Control::Continue)
        }

        #[source(voice)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "voice transcripts are best effort")]
        #[last]
        voice = sources.voice => {
            self.handle_voice(voice).await?;
            Ok(kittens::reactor::Control::Continue)
        }

        after_event {
            self.presenter.present_if_dirty()?;
            Ok(())
        }
    }
}
```

This is a probe grammar, not a published promise. The implementation MUST retain the input fixture and any competing expansion/grammar tried so the final decision is evidence-backed.

Rules for the lean probe:

- source expressions are persistent place expressions; calls and temporary future construction are rejected;
- the same normalized place expression may not appear under two source IDs; aliasing through distinct places is a known Rust limitation and the Tokio control fixture must expose any conflicting mutable borrow rather than silently dropping that oracle;
- each source has exactly one ID and one readiness declaration;
- sources are starvation-protected by default; `starvation(allowed, reason = ...)` is a visible weakening;
- shutdown sources are unguarded, undrained, terminal, and form the leading lexical prefix;
- global/source precedence and yield graphs are acyclic;
- written order must satisfy generated shutdown, `before`, and `last` edges; the macro never reorders arms;
- drain `N` is an unsuffixed positive literal with a conservative implementation limit recorded in the test; the final public maximum is not frozen by K0;
- the selected item counts as one; only after its handler returns `Ok(Control::Continue)` may generated code make a nonblocking immediate `try_next` probe, and drain never awaits for another item;
- `N` reached, target backlog/yield, immediate empty, source dormant/closed, handler `Stop`, or handler `Err` ends the service window;
- source borrows end before every per-item handler and are reacquired only for an immediate probe;
- phase and handler blocks are ordinary async Rust blocks returning `Result`;
- continuing handlers return `Result<Control<Exit>, E>`; terminal/shutdown handlers return `Result<Exit, E>`;
- `after_event` runs once after each successfully completed service window (one selected item plus any drained items), not once per drained item; any handler `Stop` or `Err` skips it even if earlier items in that window continued;
- a user guard is evaluated exactly once per arbitration, remains fixed across all executor repolls of that pending arbitration, and must produce `bool`; no claim of purity is made.
- a false guard registers no wake and is not resnapshotted on wake; K0 guards use reactor-owned state changed between arbitrations, while external changes must arrive as an enabled event that completes the current arbitration.
- at least one source arm must be unguarded (`KTR014`-anchored): if every arm carries `#[when]`, one all-false guard snapshot polls no source and registers no source wake in that arbitration. The check guarantees only that some source is polled; the application must use an unguarded wake-capable control source (commonly a shutdown channel/cancellation source), or encode external enablement in an adapter that registers the relevant wake. A permanently dormant source such as an empty local latch is not a liveness repair.
- `///` doc comments above phase blocks and source arms are accepted as source-side rationale and are intentionally not emitted into the expansion; every other non-Kittens attribute is rejected so `cfg`-style conditional topology cannot exist silently.
- enablement is snapshotted in lexical source order: evaluate that source's user guard first, short-circuit its yield probe when false, otherwise evaluate its backlog probe once per relation edge with no cross-edge cache; during draining, keep the user guard snapshot and reevaluate only the selected source's yield probe after each successful handler and before `try_next`.

The phase requirement list intentionally duplicates phase presence because it rules out deleting a required block accidentally. The negative-control benchmark also removes both the requirement and block to demonstrate that Kittens cannot infer architectural intent once the declaration is erased.

### 37.6 Minimum source contract

The stable requirement is selection-loss preservation for the source's documented delivery contract. Admission sealing, owned yielded items, eager producer latching, exact trait names/decomposition, and the pin boundary are K0 implementation hypotheses, not already-proven universal source semantics. `ReactorSource` remains a candidate neutral name. The earlier `RestartSafeSource` is rejected: an internally retained waiter need not be safe to reconstruct or repeat after whole-reactor cancellation.

An admitted source must satisfy all of the following:

1. it is stably stored before the loop;
2. polling the next item borrows or pins the persistent source through one local arbitration;
3. when another source wins, this source retains the state/event required by its documented delivery contract whether it was polled before the winner or remained unpolled below it; a lazy waiter that must first be polled to arm an otherwise lossy producer does not satisfy this by construction;
4. the initial K0 adapter yields an owned item that does not keep the source mutably borrowed during the handler; K0 records whether this restriction is necessary rather than declaring borrowed items impossible forever;
5. its readiness marker is sealed and matches the declaration;
6. if dormant, it returns `Poll::Pending` without self-waking until explicitly armed;
7. if it can be armed externally after returning `Pending`, that arming path schedules the registered reactor waker; a locally armed-only adapter documents that restriction;
8. if closable, close behavior is fixed by the adapter constructor: it either becomes dormant silently or emits one typed close event and then becomes dormant; it cannot repeatedly emit an immediately ready closed result. Non-closable sources have no close transition.

Source documentation MUST separately state what happens when the **entire source/reactor is dropped**. Selection-loss preservation does not promise drop cleanup beyond normal Rust, asynchronous cleanup, event delivery after destruction, cancellation atomicity, or safe external repetition. Any adapter that claims reconstruction or repeat safety needs a separately cited/tested contract; K0 does not need public marker traits for those facts unless generated code consumes them. Each Tokio adapter's documentation MUST additionally state whether polling its operation consumes the cooperative scheduling budget (section 20.2.1), because the underlying primitive pages generally do not disclose this and the K0 equivalence oracle depends on knowing it.

Drain and backlog operations are separate traits required only by `drain` and `yields_to`. K0 does not require lifecycle or close associated markers merely for description. K0 admits `drain` only for stable installed adapters whose supported API cannot rearm or replace the underlying source during a service window; optional/rearmable sources are not drainable in this slice. Arbitrary handler-side memory replacement is outside the generated drain guarantee and appears as an explicit compiling negative control rather than an unenforceable generation promise.

The first K0 pin boundary is explicit: every outer source adapter passed as a place must be `Unpin`. An adapter may encapsulate a pinned Tokio primitive in disclosed adapter-owned storage; such boxing/allocation is adapter cost and must be measured, while the generated arbitration itself may not box. If this boundary cannot represent the Tokio deadline/retained-one-shot fixture without unacceptable allocation or diagnostics, K0 compares exactly one alternative: the caller supplies an explicit `Pin<&mut S>` from storage it has already pinned. The macro MUST NOT synthesize unsafe pinning or promise that an arbitrary field place will not later move. Neither spelling becomes public before the comparison is recorded.

For macro-managed drain, a **service window** starts with the selected item and handles at most `N` items. After each `Ok(Control::Continue)` and only while below `N`, generated code may perform one nonblocking immediate probe; it never awaits another item. Immediate empty, target backlog/yield, dormant or closed state, `N` reached, `Control::Stop`, or `Err` terminates the window. `Stop` and `Err` skip `after_event`; one `after_event` runs after the whole window only when every handled item continued successfully. These rules apply identically to core polling and the Tokio control.

No safe external implementation or unchecked escape is required in K0. Failure to represent either fixture with the minimum curated/test adapters is evidence about the source boundary and must be recorded before adding an escape.

Sealing is admission control, not a theorem about the adapter implementation. Kittens maintainers still establish the contract through primitive documentation, code review, lost-race/runtime tests, and target tests where hardware semantics matter. The compiler can prove only that reactor code selected an admitted adapter and requested capabilities that its type exposes.

#### 37.6.1 Profile-driven inline one-shot carrier

The first post-K0 source extension is the no-allocation carrier required by
the `kittens-render` completion gate. Its one public spelling is
`source::OptionalInlineOneShot<F>`, where `F: Future + Unpin`. There is no
second always-armed inline type. The optional form is required by the real
long-running shape: it begins dormant, owns one in-flight operation, becomes
dormant before delivering that operation's output, and can then be armed with
the next operation after the handler has recovered its resources.

The carrier stores `Option<F>` inline and exposes exactly `new`,
`from_future`, `arm`, `future_mut(&mut self) -> Option<&mut F>`, and
`is_dormant`. `arm` requires exclusive access and returns `AlreadyArmed<F>`
with the rejected future rather than replacing or dropping live work.
`future_mut` returns `None` while dormant and provides borrowed access for an
operation-specific drain request while armed; the method itself does not
consume the future. Ordinary Rust still permits `mem::replace` through the
returned `&mut F`, so handler-side replacement/removal is a documented
compiling escape rather than a structural guarantee. The canonical render
path uses this borrow only for `begin_drain`.
The carrier is a sealed
`ReactorSource<Item = F::Output, Readiness = Quiescent>`. It polls the same
stored future with the reactor's current `Context`, removes the completed
future before yielding its owned output, and returns dormant `Pending` without
self-waking. It implements neither `DrainableSource` nor `BacklogSource`.

This is a locally armed-only source under section 37.6 point 7. `arm` schedules
no wake: the supported rearm points are before reactor entry or inside a
handler/phase whose successful continuation begins the next arbitration.
Arming after a pending reactor poll from another execution context is not
supported. Dropping the carrier or whole reactor synchronously drops an
installed future and returns no resources; a resource-owning operation that
requires recovery must be drained to completion before teardown. For the
reviewed render integration specifically, dropping that future drops
`InFlight`, whose `OwnedTransfer` contract synchronously cancels the operation
and disarms its completion registration. That is a reviewed adapter contract,
not silicon evidence and not a cleanup claim for arbitrary `F`.

The enforcement layers are split deliberately. The sealed carrier plus
ordinary Rust ownership prove inline retention, exclusive arm/rearm, and owned
output. The inner future's producer-latching, wake registration, cancellation,
and drop behavior remain that future's reviewed documentation and runtime-test
obligations. Wrapping an arbitrary future therefore does not make a lazy or
lossy producer valid. A compiling inert/broken inner future is one adjacent
negative control; a compile-fail declaration that labels this quiescent
carrier `may_remain_ready` pins the sealed readiness check independently; and
a compile-pass `future_mut` replacement pins the raw handler-side mutation
escape. The render integration must separately prove both
selection-loss positions with its level-visible, register-then-recheck
`InFlight` implementation. This extension does not unseal `ReactorSource`, add
a callback/poll-function escape, allocate, or change the existing heap-pinned
Tokio `OneShot`/`OptionalOneShot` APIs. The canonical split is mechanical:
`OptionalInlineOneShot` is the portable no-allocation spelling when the inner
future is `Unpin`; `OptionalOneShot` remains the heap-pinned Tokio spelling
when a `!Unpin` future must be retained or host-side allocation is accepted.

### 37.7 Expansion experiment

The first high-risk decision now has two independent axes. K0 MUST retain and compare:

1. a `core::future::poll_fn`-style ordered source poller against a direct `tokio::select! { biased; ... }` oracle; and
2. a private owned-event enum followed by an external handler match against a selected-source tag plus private per-arm `Option<Item>` slots followed by an external match/take.

The chosen form must:

- compile the 23-arm fixture without boxing, dynamic dispatch, hidden tasks, or a runtime scheduler;
- compile the embedded-shape fixture without `std` or allocation in the kernel;
- end every temporary arbitration borrow and branch-delegate future before the handler while retaining adapter-owned pending operation/event state;
- keep generated types out of ordinary user errors;
- support handlers that borrow `self` while sources live in separate storage;
- support a retained interrupt-like waiter and a completion event that returns an exclusively owned resource;
- support `.await`, `?`, normal stop, handler error, and phase error;
- show one small lexical `Poll` path under the leading expansion and one lexical `tokio::select! { biased; ... }` in the control fixture;
- pass the same current `Context` to each enabled source reached before selection; when all are pending, every enabled source is polled, and when an earlier source wins, unpolled lower source state is retained for the next iteration;
- on scripted fixtures, record selected source ID, source poll order/count, each pending/ready result, and wake registration so equivalence is judged on observable traces rather than an abstract claim about arbitrary futures;
- use direct bounded per-item drain control flow without allocation;
- preserve useful spans in rustc and rust-analyzer.

The comparison records optimized text size, generated future and event-enum size, embedded fixture stack/static footprint, compile time, idle and all-ready poll behavior, allocations (including adapter pin storage separately), source poll counts, and generated lines/tokens. Zero allocation is not evidence of acceptable memory use: an owned completion variant can enlarge the whole future.

The comparison MUST instrument Tokio's cooperative scheduling budget (section 20.2.1) at every arbitration boundary and at each drain-window item, recording remaining budget alongside the selected source ID and poll trace. Without this, a budget-induced divergence between the two forms is indistinguishable from a defect in the core-poll mechanism, and architecture B could be falsified for an executor reason rather than an architectural one. K0 MUST also record whether the 23-arm fixture with a 32-item drain reaches a budget boundary under its scripted message rates; a negative result is reported, not omitted.

Decision rules are registered before measuring the candidate. Any lost wake/event, wrong selected source on a defined trace **at equal budget**, hidden spawn/runtime/unsafe pinning, or core arbitration allocation is a hard failure. A divergence explained by unequal budget consumption is recorded as a finding about the executor boundary and triggers architecture review of the drain/arbitration interaction; it is neither a hard failure nor a result that may be discarded.

Expansion scaling is measured, not assumed. The 23-arm fixture MUST record whether it compiles without raising `recursion_limit` or `type_length_limit`, and MUST report how expansion size, generated-future size, and compile time scale between the small probe fixture and the 23-arm fixture. Superlinear growth in generated tokens or monomorphized type length is an architecture-review trigger even when the absolute numbers pass, because it bounds the reactor size Kittens can ever support. If either limit must be raised, that requirement is a public API burden and MUST be recorded as such rather than hidden in fixture configuration. An embedded footprint above the fixture's predeclared task-stack/static budget is also a hard failure. A greater than 20% regression in optimized text size, generated-future size, or median idle/all-ready poll cost against the relevant hand-written oracle, or greater than 25% in incremental compile time or expansion tokens, triggers architecture review rather than automatic rationalization. Timing uses the same pinned runner and repeated median; tiny baselines must also report absolute deltas. Keeping a regression requires naming the additional enforced semantic that pays for it; otherwise direct Tokio expansion remains the fallback.

If expression-position ownership of control flow fails, the item-position form may be tested. If direct core polling fails, direct Tokio expansion is the explicit fallback and runtime-specific expansion may be reconsidered only with the retained counterexample. Switching forms requires a failure fixture and an explanation of the borrow, pin, wake, or diagnostic improvement; no fallback is automatic.

### 37.8 North-star reality fixtures

K0 maintains distinct fixtures so changed behavior is never mislabeled as fidelity.

#### A. Raw Grok fidelity oracle

This hand-written Tokio fixture preserves the inspected semantics relevant to the experiment:

- the actual 23-source lexical order;
- the initial presentation before the first poll;
- cancellation, quit, and writer acknowledgement above ACP;
- ACP gated on buffered terminal input and drained at most 32;
- terminal input's current application-owned immediate drain, including its lack of one total numeric bound;
- loop-top deferred work and post-event presentation;
- optional timers/receivers becoming pending;
- task completion through an ordinary owned application mechanism;
- voice/STT last;
- an application-owned presenter with dirty/coalescing, one in-flight target, acknowledgement, and draw deadline.

The fidelity oracle demonstrates boundaries as well as successes: its manual input drain, presenter state, task ownership, terminal handoff, and teardown are not Kittens kernel guarantees.

The raw and Kittens Grok-shape fixtures MUST run the same application-level presenter scenarios: repeated requests coalesce; a draw that emits no payload does not invent an acknowledgement target; a multi-payload draw waits for the last emitted sequence; an earlier or delayed acknowledgement does not open the gate prematurely; and a scheduled draw deadline remains armed and fires under delayed acknowledgement. These are control-flow parity oracles, not static Kittens rendering guarantees.

#### B. Kittens Grok-shape fixture

This fixture preserves source order and handler ownership as closely as the minimum adapters permit while moving selection into the probe macro. It must include initial presentation, dynamic voice arming, reconnect replacement, writer acknowledgement, a firehose/input gate, loop-top source updates, post-event presentation, and handlers borrowing both application state and separate sources.

A bounded-terminal-input migration is a named experimental variant, not the fidelity fixture. It may become recommended only after paste/coalescing behavior and latency are measured.

#### C. Embedded-shape counterfixture

This host-buildable fixture is not an ESP32 BSP and does not claim hardware execution. It preserves the independently observed pressures:

- a dynamic absolute frame/housekeeping deadline whose cadence changes across Off, AOD, Interactive, and Game runtime modes;
- protected touch/button interrupt-like sources;
- an optional source that can be dormant or closed without self-waking;
- a may-remain-ready sensor source that must not dominate touch;
- an ordinary Rust ownership-returning transfer type whose completion event carries the resource back to the handler; the type, not Kittens, makes resubmission unavailable before return;
- per-item bounded draining with no allocation;
- explicit before-poll arming and an after-event application hook; K0 assigns no display-commit semantics to the hook;
- a small `#![no_std]`/no-allocator binary containing the kernel path that compiles and links for stable bare-metal target `thumbv7em-none-eabi`; dependency and symbol inspection must show no `std`, allocator, or runtime-adapter path. This is a portable-core gate, not an ARM-device or ESP32 hardware claim.

The fixture MUST include a source whose producer is armed/latched before selection, then show both cases: it is polled pending before another source wins, and an earlier source wins before it is polled. In both cases its promised event survives for a later iteration. This prevents the fixture from “proving” retention with a lazy waiter that would not have been armed. It MUST also include a compile-fail ownership mutation analogous to buffer/peripheral reuse, clearly credited to ordinary Rust fixture ownership rather than Kittens. Actual ESP-HAL GPIO and DMA adapters remain deferred because their pinning, target toolchain, and hardware semantics are not reproduced by a host model.

The embedded fixture is revision-keyed in documentation: 1.8 V1 is SH8601/FT3168 and current V2 is CO5300/CST820. Nearby 2.06-inch source is an orchestration oracle only, never an exact-board compatibility claim.

The task-stack and static-footprint budget that section 37.7 treats as a hard failure boundary MUST be declared and recorded in the fixture manifest before the first Kittens expansion is measured. A budget selected or adjusted after measurement is not a gate; changing it requires the same architecture-review record as any other decision-rule change.

Rendering abstraction work is removed from K0. After the reactor result is reported, a separately authorized comparison may test capacity-returning protocol shapes against application-owned presenter state and existing HAL ownership. Genericity itself is the hypothesis: the result may demonstrate one genuinely shared legal API or retain separate Grok-ticket and ownership-returning shapes. Every candidate must name the exact completion milestone it enforces.

### 37.9 Mutation matrix and negative controls

K0 requires the following primary mutations:

| Mutation | Expected layer | Required consequence |
|---|---|---|
| move a declared shutdown below the firehose | macro | error names both sources and leading-order consequence |
| add a `before` cycle | macro | error reports the source cycle at the relation that closes it |
| put an arm after global `last` | macro | error names the last source and later source |
| remove ACP's yield while it precedes protected input | macro | error explains possible starvation and distinguishes reorder from buffered yield |
| use a temporary/reconstructed arbitrary future as a source | parser or source trait | error points at the source expression and recommends persistent channel isolation |
| use a nonliteral, zero, or over-limit macro drain | macro | error requests a supported positive literal |
| request drain on a non-drainable source | rustc assertion | error contains the concrete source type and drain capability, not marker tuples |
| omit a phase that remains listed as required | macro | error names the missing phase |
| close optional mpsc | runtime paused-time test | source becomes dormant with no repeated selection/self-wake |
| fire optional deadline | runtime paused-time test | source disarms before exposing its event |
| produce more than the drain maximum | runtime test | no more than the declared total is handled before `after_event` |
| place a may-ready sensor above protected touch without yield | macro | error names the dominant and protected sources; actual latency remains a runtime measurement |
| rebuild a non-admitted interrupt waiter on each race | parser or source trait | error recommends a retained/latching adapter or owned signal/channel isolation |
| close or disarm an embedded optional source | host behavior test plus no-std compile fixture | source becomes dormant and does not self-wake; the behavioral test does not pretend its host harness is `no_std` |
| allow an alloc-backed collection in core drain | feature/compile fixture | no-std/no-alloc build continues to compile because K0 drain is per-item |
| fail to re-arm a dynamic deadline | deterministic test | scenario exposes missed frame/maintenance event; not claimed static |
| reconstruct a relative timer on every lost race | deterministic test | scenario exposes deadline reset/liveness failure; persistent absolute deadline is the repair |
| start a second ownership-returning transfer | ordinary Rust fixture type | compile failure is recorded as a composition baseline, not a Kittens diagnostic |
| use a generic async display-interface operation as a repeated source | source admission | rejected unless a reviewed adapter establishes its selection-loss contract; `async` alone is insufficient |
| name the same source place under two IDs | parser, with rustc as aliasing backstop | an exact duplicate place is rejected; indirect aliases remain a documented limitation and must also compile under the Tokio control |
| flip a false guard externally and only wake the reactor | host behavior test | guard remains false for the current arbitration; repair is an enabled control event that completes it or source-local dormant arming, not wake-only mutation |
| exhaust the Tokio cooperative budget inside a 32-item drain window | instrumented runtime test | the window ends early with budget recorded; both expansions are compared at equal budget and any divergence is classified as executor-boundary, not mechanism |
| panic in a handler during a drain window | host behavior test | unwind propagates, `after_event` does not run for already-handled items, and the test asserts no compensating cleanup is invented |

Negative controls are equally important. The following are expected to compile or remain outside static enforcement, and the benchmark must record that fact:

| Constraint erasure/bypass | Honest boundary demonstrated |
|---|---|
| remove `shutdown`, then reorder the cancellation-looking branch | macro cannot infer semantic shutdown from handler code |
| remove `last` | macro cannot infer that voice ought to be last |
| add `starvation(allowed, reason = ...)` to input | an audited waiver weakens the guarantee; prose truth is not proved |
| remove both a phase block and its requirement | macro cannot infer that the application still needs the phase |
| write an unbounded `try_recv` loop inside a handler | drain bounds apply only to macro-managed drains |
| replace a stable drain source through raw handler-side memory operations | K0 constrains the admitted adapter API and generated probes, not arbitrary mutation through ambient Rust access |
| await a cancellation-unsafe operation inside a handler | source approval applies to the raced source, not arbitrary handler awaits |
| await an eight-second network operation in a touch handler | priority cannot preempt handler code or imply responsiveness |
| await indefinitely in `before_poll` or `after_event` | phase placement does not imply a time bound or allow shutdown to preempt application code |
| call a raw writer twice | the kernel does not own rendering authority or single-flight state |
| claim a double buffer in comments but never swap it | descriptive intent does not reduce the legal program space |
| keep a frame source armed in Off mode | runtime guard/state correctness remains a deterministic-test and power-measurement concern |
| use raw Tokio/Embassy selection or spawning elsewhere | Kittens is not a language-level prohibition mechanism |
| a declared `drain(max = 32)` handles fewer items because the task budget was exhausted | the bound is on Kittens-managed service work, not on executor budget; the declaration cannot express or control this |
| enable both the `no_std` kernel and the Tokio integration in one dependency graph | feature unification is a Cargo fact; the kernel path must remain `no_std`-clean under section 37.3.1 rather than rely on callers not combining features |

An implementation report that lists only rejected mutations fails K0; it must publish the compiling negative controls beside them.

### 37.10 Diagnostic evidence gate

Before any K0 diagnostic becomes stable, it must demonstrate:

- the declared source IDs and violated relation;
- the operational consequence, such as starvation, wrong poll order, or unbounded generated work;
- a primary span on the declaration that introduced the conflict;
- a safe direct repair when one exists;
- policy tradeoffs when repairs differ semantically;
- no tuple indexes, anonymous event variants, nested marker tuples, or irrelevant generated generics.

Numeric IDs may be used to correlate fixtures and documentation, but their numbering and exact prose remain provisional during the pilot. Tests match semantic anchors on MSRV candidates and current stable. Exact prose is frozen only after diagnostic-only agent trials show that it leads to constraint-preserving repairs.

The source-admission diagnostic experiment is finite. It MUST compare exactly: (1) associated-marker equality in a generated helper, (2) a purpose-specific trait bound, and (3) a constructor-returned sealed admitted newtype with no semantic assertion at the arm. The selected form must expose the concrete source type and actionable alternatives: a retained/latching reviewed adapter when one exists, or owned producer/signal/channel isolation. It MUST NOT call an operation “cancellation-safe” merely because its Drop releases resources. The trait or wrapper name is part of the agent test. Testing another mechanism requires first amending the recorded experiment, not silently expanding the MUST.

The embedded mutation's target semantic content, with exact prose still provisional, is:

```text
error: source `touch_irq` is not admitted for repeated reactor selection
the operation does not establish preservation when another source wins

help: use a reviewed retained/latching source adapter
      or isolate the interrupt producer behind an owned signal/channel

note: cleanup when the operation is dropped is not the same contract
```

If stable rustc cannot attach that explanation to the trait error causally, the constructor/parser boundary must reject earlier or the source API must change. A remote documentation link alone does not pass the diagnostic gate.

### 37.11 Agent-first ablation

As soon as shutdown/order, cycle, last, yield, drain, and source-admission failures exist, run a small blind pilot; include both a Grok firehose mutation and the embedded interrupt-like source-admission mutation. Do not postpone it until the rest of the architecture exists.

Each core task compares:

1. the raw runtime idiom for the fixture (Tokio for Grok; an ordinary host-modeled select/poll future for embedded shape);
2. the same local topology facts in non-enforced comments/metadata;
3. lean K0 syntax;
4. the maximal eight-base-declaration syntax.

The pilot fixes one concrete representation for condition 2 before trials begin — structured comments or inert attributes, not a mixture — and records the choice, so condition differences are attributable to enforcement rather than formatting.

The pilot measures:

- whether the invalid edit is accepted;
- whether the first repair compiles and preserves the hidden behavioral oracle;
- repair iterations and tool turns;
- constraint deletion or weakening;
- use of starvation waivers or raw bypass;
- invented Kittens APIs;
- diagnostic and source token cost;
- whether the agent correctly explains what Kittens does not enforce.

The pilot also records whether the agent treats the compiler as a causal reasoning aid or as an obstacle to bypass. A repair is constraint-preserving only when the named invariant remains represented and the hidden behavioral oracle still passes. Deleting an attribute, replacing a canonical adapter with an unreviewed raw future, adding an unexplained starvation waiver, or moving the effect into an opaque helper counts as a semantic regression even if compilation succeeds.

The pilot MUST include a context-reset variant. After a source and its local constraints have been established, the creating agent's context is discarded and a fresh agent is asked to make a semantically risky but ordinary change, such as improving voice latency or adding a render source. The fresh agent receives no architecture explanation beyond the repository. The result measures rehydration rather than generation: recovered invariant count, retrieval/tool turns, invalid edits attempted, declarations deleted or weakened, diagnostic-guided repairs, and hidden-oracle preservation.

The first pilot may use five fresh trials per core mutation/condition to expose gross failures; it is exploratory, not a statistical release claim. Promotion requires at least four of five diagnostic-only trials for each core error to reach a constraint-preserving repair within two iterations. Any failure pattern is reviewed qualitatively before increasing sample size.

If maximal syntax does not improve correct generation or repair over lean K0, the additional annotations are rejected. If non-enforced local metadata performs as well as Kittens on final correctness, the macro's marginal value is in doubt even if it rejects toy mutations.

### 37.12 Architectural falsifiers

K0 closes successfully when all of the following hold: the 23-arm Grok-shape fixture and the embedded-shape counterfixture compile and borrow naturally under the selected expansion; generated code remains recognizable `Future::poll` control flow with no hidden spawn, runtime scheduler, boxing, or arbitration allocation; the Tokio-select control remains behaviorally faithful and the two forms agree on the scripted oracles at equal cooperative budget; the kernel path links `no_std`/no-alloc on the bare-metal target, including under feature unification per section 37.3.1; every primary mutation in section 37.9 fails at its intended layer while the negative controls compile and are published beside them; and the diagnostic-only agent pilot reaches constraint-preserving repairs within the thresholds of section 37.11. Anything less triggers the falsifier review below.

K0 materially changes or rejects the current architecture if any of the following occurs:

- the borrow-realistic 23-arm fixture requires boxing, hidden task spawning, a runtime scheduler, or broad application-state restructuring;
- the embedded-shape fixture requires allocation in the core, handwritten unsafe pin projection, self-referential source storage, or application restructuring merely to retain a waiter;
- direct core polling loses a wake/event, selects differently from the declared lexical semantics, or cannot retain a losing source without compromising source access;
- the core-poll expansion materially regresses optimized code size, idle/all-ready poll cost, compile time, or diagnostics relative to direct Tokio selection;
- the claimed base cannot compile without `std` and `alloc` for reasons intrinsic to topology rather than a runtime adapter;
- normal handler errors prominently expose generated event/marker machinery;
- source contract failures cannot lead from the concrete type to a usable repair;
- important mutations can be rejected only by trusting an unverified descriptive annotation;
- the macro cannot preserve observed Grok behavior without silently changing input, rendering, shutdown, or source-lifecycle semantics;
- expansion is materially less intelligible than the hand-written Tokio oracle;
- rust-analyzer, formatting, or parse recovery makes first-time editing materially worse than the raw baseline;
- agents commonly repair by deleting declarations, adding waivers, or bypassing the constrained path;
- the maximal annotation form adds ceremony without measurable benefit;
- checked semantic redundancy cannot produce a useful disagreement diagnostic, or context-reset agents recover no more local architecture than the inert-metadata baseline;
- priority classes add no useful mutation coverage or comprehension over exact source relations;
- the two expansions cannot be compared on a stable behavioral oracle because cooperative-budget effects cannot be instrumented or held equal, leaving mechanism divergence unattributable;
- the 23-arm fixture requires raising `recursion_limit` or `type_length_limit`, or expansion/compile cost grows superlinearly in arm count, bounding the reactor sizes Kittens can support below the north-star fixture;
- the kernel path cannot remain `no_std`-clean under feature unification inside a single facade, forcing a package split before any profile work begins.

Possible consequences include retaining direct Tokio expansion, using narrowly runtime-specific expansion, shrinking the macro to a topology lint, changing item/expression position, replacing the event-enum expansion, collapsing source traits, removing readiness annotations, adopting a different relation surface, or rejecting the reactor compiler thesis. A failed gate is evidence, not an invitation to preserve the design through hidden runtime machinery or more generic parameters.

### 37.13 Information-first implementation order

No step begins without explicit implementation authorization.

1. **Freeze both oracles, not Kittens syntax.** Retain the Grok fidelity description, the embedded-shape counterfixture, mutation intent, and negative controls.
2. **Build two thinnest arbitration paths.** For the same tiny source set, retain direct core polling and direct biased Tokio selection before adding polished syntax.
3. **Resolve borrowing and pinning immediately.** Exercise the 23-arm field-borrow shape and the retained-waiter/ownership-returning shape; compare event-enum and direct-control forms.
4. **Falsify runtime neutrality.** Run no-std/no-alloc compilation, wake/lost-race tests, source poll-count equivalence, optimized size/latency checks, and expansion inspection. Choose B, A, or narrowly C here and record why.
5. **Inspect real tooling.** Record `cargo expand`, rustc JSON, rust-analyzer behavior, formatting, compile time, and error spans for the chosen path.
6. **Add the six global/source checks.** Shutdown/order, cycle, last, readiness/yield, bounded drain, and approved source admission.
7. **Run desktop and embedded mutations plus negative controls.** Fix semantic bugs in the kernel only; do not add extension modules or hardware adapters.
8. **Run the agent ablation.** Compare raw, annotated, lean, and maximal forms while the grammar is still cheap to change.
9. **Make a reactor decision.** Freeze, simplify, make the expansion runtime-specific, reduce it to a lint, or reject the macro/source surface; amend this document with retained evidence.
10. **Stop and report.** The report is a retained repository artifact, `K0-REPORT.md`, containing: the decision and evidence for every provisional row in section 37.14; the expansion-comparison measurements of section 37.7, including cooperative-budget instrumentation and arm-count scaling; the mutation and negative-control results of section 37.9, with the compiling negative controls published beside the rejections; the ablation and rehydration results of section 37.11; the falsifier assessment against section 37.12; and the selected architecture (B, A, or narrowly C) with retained counterexamples for every rejected alternative. Rendering, Embassy adapters, scope, and every other extension require a new explicit decision and authorization.

This order intentionally avoids maximum parallel progress. The borrow/expansion result precedes polished topology validation because it can invalidate the entire surface. Agent repair precedes public API freeze because compiler output is part of the product.

### 37.14 Stability and graduation map

| Decision | Status through K0 | Evidence required to change/promote |
|---|---|---|
| Tokio as first production integration; no Kittens executor | stable | contrary ecosystem evidence plus a new architecture review |
| runtime-independent topology semantics | stable objective, mechanism provisional | dual-fixture failure may retain only a Tokio implementation while preserving terminology |
| persistent admitted sources | stable behavioral boundary | fixture failure or better selection-loss composition with clearer diagnostics |
| direct core polling rather than Tokio selection | leading provisional candidate | 23-arm/embedded pinning, wake equivalence, no-alloc, code-size, and diagnostic evidence |
| biased lexical order | stable semantics | an explicit later fairness design, not silent backend defaults |
| exact macro position and grammar | provisional | dual-fixture borrow, tooling, expansion, and agent ablation |
| priority classes | deferred | concrete relation duplication or better agent results |
| source trait names/decomposition, sealing, and pin/`Unpin` boundary | provisional | rustc diagnostic, retained waiter, and adapter implementation evidence |
| no-std/no-alloc kernel compilation | stable K0 gate | a `--no-default-features` binary must compile and link for stable bare-metal target `thumbv7em-none-eabi`; dependency/symbol inspection must show no `std`, allocator, or runtime adapter path, while host tests separately exercise behavior |
| no-std facade/package/feature spelling | provisional implementation boundary | dependency graph, downstream usage, and agent discoverability evidence |
| kernel stays no-std-clean under feature unification | stable K0 gate | the bare-metal link must also pass with the Tokio feature enabled elsewhere in the graph; failure is evidence for a `kittens-core` split, not for weakening the gate |
| cooperative budget stays outside the kernel and outside generated code | stable | adapter-level disclosure and instrumented comparison only; `unconstrained` is never generated |
| supported reactor arm ceiling | provisional, measured by K0 | 23-arm expansion/compile scaling and whether default rustc limits suffice |
| one semantic kernel with runtime/domain profiles | stable architectural direction; package split provisional | K0 must keep topology laws profile-neutral; each promoted profile must show domain-specific constraint or diagnostic value over its raw baseline |
| context-reconstructible artifacts and agent rehydration | stable objective; representation provisional | context-reset trials must show whether local declarations, types, and diagnostics recover invariants better than raw or inert-metadata baselines |
| exact guard grammar | provisional | parse/tooling evidence tied to an enforceable control-flow benefit |
| diagnostic IDs/prose/order | provisional | cross-toolchain snapshots and repair pilot |
| compile-time source-state availability table | deferred | a real invalid transition rejected without macro-owned application state or generic explosion |
| generic render gate and phase permit | weakened extension hypothesis | comparison across Grok acknowledgement and at least one nonblocking ownership-oriented display protocol |
| Tokio structured scope/timeout | extension hypothesis | independent baseline comparison proving non-detachment and shutdown UX value |
| executor-neutral/Embassy scope | rejected as a shared current abstraction | a concrete Embassy lifecycle design with join/cancel semantics, not generic wrappers |
| Embassy/ESP-HAL source adapters | post-K0 integration hypothesis | target compilation, primitive-specific contract audit, lost-race tests, and hardware tests where required |
| resource API | extension hypothesis | two real resources and honest non-yielding cleanup behavior |
| capabilities/approval/flow | extension hypothesis | improvement over application-owned Rust types in an agent benchmark |
| public simulation/observability | extension hypothesis; first-priority post-K0 under the section 2.1 coverage thesis | demonstrated reproduction/repair value over private fixtures and ordinary tracing |
| escape-surface lint (`cargo kittens lint`) | extension hypothesis; first-priority post-K0 with public simulation | reliable detection of raw spawns/selections and undeclared producers across expansion at an acceptable false-positive rate, reported as a measured escape surface rather than a prohibition |

### 37.15 Readiness assessment

The project is ready to begin K0 after explicit authorization. It is not ready to implement or publish the earlier full v0.1 unchanged. The subjective planning assessment is **90/100 for beginning the reversible K0 evidence slice** and **55/100 for freezing a public v0.1 API today**. These are calibration scores, not statistical probabilities of architecture success. The first is high because the learning slice is narrow and a no-std ordered-poll probe passed; the second remains deliberately lower because pinning, macro expansion, and diagnostics have not faced both fixtures.

Genuinely stable objectives are one semantic kernel with runtime/domain profiles, Tokio as the first production integration, no Kittens executor, token-level analysis for declared global relationships, an agent-facing compiler/error surface that makes policy locally inferable and repairable, context-reconstructible artifacts that survive agent amnesia, selection-loss-preserving source behavior, biased lexical semantics, allocation-free bounded per-item service, explicit control-flow phases, and honest non-guarantees. The exact macro/trait split, canonical spellings, naming policy, and package split remain falsifiable. K0 preserves application, framework, and HAL rendering boundaries and claims no ownership or protocol guarantee for render resources; where a selected HAL API already enforces exclusivity, Kittens only composes with it.

The direct-core-poll mechanism, exact macro surface, source trait family and pinning strategy, borrow-scoping expansion, no-std feature packaging, readiness syntax, and diagnostic taxonomy remain provisional. Phase capabilities, a generic rendering gate, Embassy adapters, state-associated source tables, and every non-reactor module remain deferred.

The most important unknown is whether one generated core-polling future can satisfy Grok-scale field borrowing **and** retain an interrupt-like/ownership-returning source without self-referential pinning or nonlocal errors.

The implementation-readiness refinement pass closed the remaining specification-completeness gaps: the lean readiness vocabulary is now fixed, the K0-normative subsections outside section 37 are named, the adapter budget-disclosure obligation is inside the controlling source contract, the positive close conditions and the `K0-REPORT.md` evidence artifact are specified, and the toolchain/`Control`/footprint-budget details an implementer would otherwise have to guess are stated. Remaining uncertainty is empirical — pinning, borrowing, expansion scaling, diagnostics, and agent behavior — not specificational.

The executor-boundary review added a second-order unknown that does not change that ranking but does affect how the primary result will be read. Tokio's cooperative scheduling budget is consumed differently by the two candidate expansions and can return `Pending` while an item is available. Until it is instrumented, a divergence between core polling and `tokio::select!` is not attributable to either mechanism. This lowers confidence in the *cleanliness of the comparison*, not in the architecture: the 90/100 readiness to begin K0 is unchanged, because the fix is instrumentation the slice can add cheaply, while the 55/100 readiness to freeze a public API is now additionally constrained by the unmeasured arm-count ceiling and the unverified no-std-under-unification gate. Both are build-level facts that can force a package split or a documented reactor-size limit without touching the topology thesis.

## 38. Agent-facing usage sketches: what building with Kittens looks like

This section is documentation, not additional normative surface. Every Rust block below is a specification sketch in the lean K0 grammar of section 37.5 — the only grammar an agent should learn first. Type names, adapter constructors, and diagnostic prose remain provisional until the K0 gate closes (sections 37.6, 37.10); the declarations, their consumers, and the checked/unchecked boundaries shown beside each sketch are the stable content. When implementation is authorized, these sketches become the seed of the CI-checked canonical example set required by section 26.2, and any spelling that K0 evidence changes is updated here in the same commit.

Reading rules for an agent consuming this section:

- these examples use only the lean K0 surface: no priority classes, no `lifecycle`/`cancellation_safe`/`close` arm attributes, no batch drain, no `scope`, no `SingleFlight`, no capabilities, no escapes. If retrieval surfaces the section 11 maximal grammar, that form exists only for the ablation and MUST NOT be generated;
- one canonical spelling per operation (section 4.9). Where two APIs could express something, the sketch shows the supported one and names the alternative as excluded;
- each sketch ends with an explicit boundary listing. Copying the code without the boundary is how over-claiming starts; the boundary is part of the example.

### 38.1 The smallest complete reactor

A minimal tool-runner harness: cancellation, a model stream that must not starve the user, and user input. This is the five-minute shape — every larger reactor in this document is this shape with more arms.

```rust
use kittens::reactor::Control;
use kittens::source::{self, close, ChannelEvent};

enum Exit {
    Cancelled,
    ModelStreamEnded,
}

// Sources live in their own struct so the reactor can borrow them
// disjointly from application state (section 12.1). Adapter type
// names are provisional; the constructors and close policies are
// the canonical part.
struct Sources {
    cancel: source::Cancellation,
    model_stream: source::Mpsc<ModelEvent, close::Emit>,
    user_input: source::Mpsc<InputEvent, close::Dormant>,
}

fn build_sources(
    cancel: CancellationToken,
    model_rx: tokio::sync::mpsc::Receiver<ModelEvent>,
    input_rx: tokio::sync::mpsc::UnboundedReceiver<InputEvent>,
) -> Sources {
    Sources {
        cancel: source::cancellation(cancel),
        // close::Emit: the stream ending is an event the handler sees once.
        model_stream: source::mpsc(model_rx, close::Emit),
        // close::Dormant: input closing is a silent state transition.
        user_input: source::mpsc(input_rx, close::Dormant),
    }
}

impl Harness {
    async fn run(&mut self, sources: &mut Sources) -> Result<Exit, HarnessError> {
        kittens::reactor! {
            policy {
                selection: biased;
                required_phases: [after_event];
            }

            // Shutdown arms form the leading poll prefix. The macro
            // rejects any reordering that puts a stream above this arm.
            #[source(cancel)]
            #[readiness(quiescent)]
            #[shutdown]
            _ = sources.cancel => {
                Ok(Exit::Cancelled)
            }

            // A firehose: may stay ready indefinitely. It is legal above
            // user_input only because it declares the yield; deleting
            // that attribute is a compile error, not a comment rot.
            #[source(model_stream)]
            #[readiness(may_remain_ready)]
            #[yields_to(user_input, when = buffered)]
            #[drain(max = 32)]
            event = sources.model_stream => {
                match event {
                    ChannelEvent::Item(token) => {
                        self.apply_model_event(token)?;
                        Ok(Control::Continue)
                    }
                    ChannelEvent::Closed => Ok(Control::Stop(Exit::ModelStreamEnded)),
                }
            }

            // Protected by default: no waiver, so every may-remain-ready
            // arm above it must yield to it or the reactor does not compile.
            #[source(user_input)]
            #[readiness(may_remain_ready)]
            input = sources.user_input => {
                self.apply_input(input).await?;
                Ok(Control::Continue)
            }

            after_event {
                // Application-owned presenter. Kittens owns the position
                // of this hook, not what rendering means.
                self.presenter.present_if_dirty()?;
                Ok(())
            }
        }
    }
}
```

What each declaration buys (the section 4.3.1 consumer, spelled out once):

| Declaration | Consumed by | What stops compiling without it |
|---|---|---|
| `#[shutdown]` on `cancel` | macro graph | any arm order in which a stream precedes shutdown |
| `#[readiness(may_remain_ready)]` | starvation analysis + generated trait assertion | a declaration that contradicts the sealed adapter marker |
| `#[yields_to(user_input, when = buffered)]` | generated guard + backlog assertion | `model_stream` above protected `user_input` with no yield |
| `#[drain(max = 32)]` | expansion + drain-capability assertion | an unbounded or nonliteral macro-managed drain |
| `required_phases: [after_event]` | phase validator | silently deleting the render hook |
| `close::Emit` vs `close::Dormant` | adapter type | treating stream-end and input-close as the same condition |

Boundary — not checked here: handler bodies are ordinary Rust (an unbounded `try_recv` loop inside a handler compiles); `apply_input` awaiting for eight seconds blocks the whole reactor (priority is arbitration order, not preemption); external event arrival order; whether the presenter is correct.

### 38.2 A full agent-harness reactor (Grok-shape excerpt)

What the production shape looks like: two shutdown arms, a writer acknowledgement lane, the model firehose with a runtime guard, task completions, protected input, a dynamic draw deadline, and a deliberately last voice source. This is eight arms of the 23-arm fixture; the remaining arms repeat these patterns.

```rust
async fn run(&mut self, sources: &mut Sources) -> Result<LoopExit, HarnessError> {
    kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [initialize, before_poll, after_event];
        }

        initialize {
            self.presenter.request_initial();
            self.presenter.present_if_dirty(&mut self.writer)?;
            Ok(())
        }

        before_poll {
            self.run_deferred_terminal_work().await?;
            // Dynamic sources are rearmed here, at the declared loop-top
            // position, not scattered through handlers.
            sources.draw_deadline.set(self.presenter.deadline());
            Ok(())
        }

        #[source(connection_cancel)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.connection_cancel => {
            Ok(LoopExit::Disconnected)
        }

        #[source(graceful_quit)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.graceful_quit => {
            self.dispatch_quit().await?;
            Ok(LoopExit::Quit)
        }

        // Writer acknowledgements unlock the next frame. Writer death is
        // fatal and arrives as an ordinary typed close event.
        #[source(writer_events)]
        #[readiness(may_remain_ready)]
        #[yields_to(terminal_input, when = buffered)]
        event = sources.writer_events => {
            match event {
                ChannelEvent::Item(ack) => {
                    self.presenter.acknowledge(ack);
                    Ok(Control::Continue)
                }
                ChannelEvent::Closed => Err(HarnessError::WriterExited),
            }
        }

        // The firehose. The runtime guard is snapshotted once per
        // arbitration; the drain bound and the input yield are generated
        // behavior, not comments. `before(task_events)` freezes an
        // ordering that would otherwise be a legal-but-breaking reorder.
        #[source(acp_stream)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "model streaming may wait behind control events")]
        #[when(self.session_accepts_acp())]
        #[yields_to(terminal_input, when = buffered)]
        #[drain(max = 32)]
        #[before(task_events)]
        message = sources.acp_stream => {
            match message {
                ChannelEvent::Item(m) => {
                    self.handle_acp_event(m).await?;
                    Ok(Control::Continue)
                }
                ChannelEvent::Closed => {
                    self.on_acp_stream_closed()?;
                    Ok(Control::Continue)
                }
            }
        }

        // Effect completions. In K0 these arrive on an admitted mpsc fed
        // by an application-owned JoinSet forwarder task — a visible
        // mechanism outside Kittens guarantees (section 37.3).
        #[source(task_events)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "effect completions may wait behind model streaming")]
        #[yields_to(terminal_input, when = buffered)]
        completion = sources.task_events => {
            self.handle_task_completion(completion)?;
            Ok(Control::Continue)
        }

        // Fed by the owned reader thread in 38.6; close::Emit makes
        // "the reader thread exited" a typed, fatal event.
        #[source(terminal_input)]
        #[readiness(may_remain_ready)]
        input = sources.terminal_input => {
            match input {
                ChannelEvent::Item(event) => {
                    self.coalesce_and_dispatch_input(event).await?;
                    Ok(Control::Continue)
                }
                ChannelEvent::Closed => Err(HarnessError::TerminalReaderExited),
            }
        }

        // Dynamic deadline: dormant unless before_poll armed it,
        // disarms before firing, cannot hot-loop after firing.
        #[source(draw_deadline)]
        #[readiness(quiescent)]
        #[starvation(allowed, reason = "frame throttling deliberately delays drawing")]
        _ = sources.draw_deadline => {
            self.presenter.on_deadline();
            Ok(Control::Continue)
        }

        // Interim transcripts at 5–20 Hz. The waiver is the explicit,
        // audited version of the comment Grok keeps above its last arm;
        // `last` makes "someone appended an arm below voice" a compile error.
        #[source(voice)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "interim voice transcripts are best effort")]
        #[last]
        transcript = sources.voice => {
            self.handle_voice(transcript)?;
            Ok(Control::Continue)
        }

        after_event {
            self.presenter.present_if_dirty(&mut self.writer)?;
            Ok(())
        }
    }
}
```

The declaration block is longer than the equivalent `tokio::select!`. That is the trade this project makes deliberately (section 0.2): every ordering fact that Grok keeps in comments — shutdown first, ACP yields to input, ACP drains 32, voice last — is a compiler input here, locally visible to the agent editing the arm it governs.

Note the waivers on `acp_stream`, `task_events`, and `draw_deadline`. Under the direct starvation rule of section 37.4, every may-remain-ready predecessor must yield to each protected victim individually, and a source carries at most one yield edge in K0. A stream lane whose single yield edge protects `terminal_input` cannot also protect the lanes below it, so those arms carry audited waivers — which is exactly the service hierarchy Grok's comments imply: input is sacred, everything else in the stream tier is best effort relative to what precedes it. An earlier revision of this example omitted the waivers and did not compile under the implemented checker; the correction is recorded here deliberately as an instance of the spec-example-versus-checker drift this project treats as a first-class defect.

Boundary — not checked here: `self.session_accepts_acp()` is trusted as an ordinary boolean (its truth is a runtime fact); the presenter's one-frame-in-flight protocol is application-owned runtime state in K0; the JoinSet forwarder is an application mechanism; nothing bounds how long `handle_acp_event` awaits.

### 38.3 Dynamic source lifecycle: arming, dormancy, replacement

Grok's `Option<Receiver>` / `Option<Instant>` / `pending()` convention becomes owned adapter state. Dormancy is a type behavior, not a per-loop discipline, so forgetting the convention cannot create a hot loop.

```rust
struct Sources {
    // dormant until armed; silently dormant again when the channel closes
    voice: source::OptionalMpsc<Transcript, close::Dormant>,
    // dormant until set; disarms before exposing its event
    draw_deadline: source::OptionalDeadline,
    // retains its future across lost races; replacement is explicit
    reconnect: source::OptionalOneShot<ReconnectOutcome>,
}
```

Inside handlers — source borrows have ended by handler time (section 12.1), so handlers rearm freely:

```rust
InputEvent::StartVoice => {
    let rx = self.voice_pipeline.start()?;   // app-owned producer, visible mechanism
    sources.voice.arm(rx)?;                   // Err returns the receiver if already armed
    Ok(Control::Continue)
}
InputEvent::StopVoice => {
    self.voice_pipeline.stop();
    let _old = sources.voice.disarm();        // live receiver handed back, not leaked
    Ok(Control::Continue)
}
LeaderEvent::Reconnect(generation) => {
    // Replacing an in-progress one-shot visibly cancels it; there is no
    // silent `replace` spelling for an owned in-flight operation.
    sources.reconnect.cancel_and_replace(
        CancelReason::superseded(generation),
        self.begin_reconnect(generation),
    );
    Ok(Control::Continue)
}
```

Contract recap an agent can rely on: a dormant source polls `Pending` and does not self-wake; a closed `close::Dormant` source never yields a repeated "closed" result; an `OptionalDeadline` holds an absolute instant, so losing races does not reset it the way rebuilding a relative `sleep` would. Arming from outside the reactor task is not supported in K0 — external arming arrives as an event on another admitted source (section 20.2).

Boundary — not checked: that the application ever rearms anything (liveness of rearming is a deterministic-test concern, section 37.9); that `begin_reconnect`'s external side effects are safe to abandon on replacement.

### 38.4 The edit-and-repair loop as an agent experiences it

The compiler is part of the interface (section 0.2). Two representative failures, with diagnostic prose still provisional (section 37.10) but semantic anchors fixed.

An agent "optimizes" by moving the model stream to the top of the loop:

```text
error: shutdown source `connection_cancel` must precede `acp_stream`
  shutdown sources form the leading poll prefix; a backlogged stream
  polled first could starve shutdown indefinitely
help: move the complete `acp_stream` arm below every #[shutdown] arm
      without changing its attributes
```

An agent wires a vendor stream directly into an arm:

```rust
// rejected: constructs a temporary future at the arm
packet = vendor.next_packet() => { ... }
```

```text
error: source `packets` is not admitted for repeated reactor selection
  `vendor.next_packet()` constructs a new temporary at the arm; losing an
  arbitration would drop it, and its progress is not known to survive that
help: store a reviewed persistent source before the loop, or isolate the
      producer behind an explicitly owned task and an admitted mpsc source
note: cleanup when the operation is dropped is not the same contract
```

The canonical repair for the second failure is the isolation pattern in 38.6 — never wrapping the future in a local `pending()` trick, and never `escape` (which does not exist in K0). What counts as a *bad* repair is defined by section 37.11 and worth internalizing: deleting the `#[shutdown]` attribute, adding an unexplained `starvation(allowed, ...)` waiver, or moving the operation into an opaque helper all make the error disappear and all count as semantic regressions even though compilation succeeds. Repair the topology, not the declaration.

### 38.5 The embedded-shape loop

The same grammar describing an ESP32-class UI task, host-modeled (section 37.8C — fixture-local adapters, no hardware claim). This is what makes the kernel a kernel: nothing below is Tokio.

```rust
kittens::reactor! {
    policy {
        selection: biased;
        required_phases: [before_poll, after_event];
    }

    before_poll {
        // Cadence is runtime state: 30 s Off, 1 s watchface, 16 ms game.
        let now = self.clock.now();
        sources.frame_deadline.set(self.mode.next_frame_at(now));
        Ok(())
    }

    #[source(stop)]
    #[readiness(quiescent)]
    #[shutdown]
    _ = sources.stop => {
        Ok(FirmwareExit::Stop)
    }

    #[source(frame_deadline)]
    #[readiness(quiescent)]
    fired_at = sources.frame_deadline => {
        self.tick(fired_at)?;
        Ok(Control::Continue)
    }

    // Latched interrupt-like source: the latch is armed before selection,
    // so an edge that arrives while another arm wins is retained, not lost.
    // A raw HAL wait that loses edges on drop is not admissible here.
    #[source(touch)]
    #[readiness(quiescent)]
    event = sources.touch => {
        self.handle_touch(event)?;
        Ok(Control::Continue)
    }

    // The original K0 fixture uses a synthetic Latched<TransferDone> here.
    // The deployment profile replaces it with the retained InFlight source
    // described below; this arm still receives one owned completion value.
    #[source(transfer_done)]
    #[readiness(quiescent)]
    done = sources.transfer_done => {
        let (display, framebuffer) = done;
        self.idle_display = Some((display, framebuffer));
        Ok(Control::Continue)
    }

    #[source(sensor)]
    #[readiness(may_remain_ready)]
    #[starvation(allowed, reason = "sensor telemetry is best effort")]
    #[last]
    sample = sources.sensor => {
        self.record_sample(sample);
        Ok(Control::Continue)
    }

    after_event {
        // App hook. K0 assigns no display-commit semantics here.
        if self.dirty {
            if let Some((display, buf)) = self.idle_display.take() {
                self.submit_frame(display, buf)?;  // consumes both; ownership gates reuse
            }
        }
        Ok(())
    }
}
```

The topology vocabulary is identical to 38.2 — same shutdown prefix, same readiness tokens, same waiver, same phases — which is the executor-neutrality claim K0 exists to falsify. Boundary — not checked: sleep entry, power draw, DMA-versus-scanout milestones, and whether `Off` mode actually disarms the frame source (runtime guard + deterministic test, section 37.9).

The real K2R-0 completion fixture replaces the synthetic local latch with this
consumer-owned source shape (the profile crate itself keeps no normal
dependency on the kernel):

```rust
struct RenderSources<X, S>
where
    X: kittens_render::transfer::OwnedTransfer + Unpin,
    S: Unpin,
{
    transfer_done: kittens::source::OptionalInlineOneShot<
        kittens_render::transfer::InFlight<X, S>,
    >,
}
```

`InFlight<X, S>` implements `Future<Output = Settled<...>>` under those same
outer-`Unpin` bounds by delegating to its existing `poll_complete` operation.
The `transfer_done` handler consumes `Settled`, recovers transport/sent/spare,
and delivers the exactly-one settlement to the owning sweep. Only after that
handler borrow ends may application code create the next target-bound flight
and call `arm`; the required fixture rearms the same carrier for a second
stripe. Graceful shutdown uses
`if let Some(flight) = transfer_done.future_mut() { flight.begin_drain(); }`;
`None` means there is no flight to drain. It continues polling an armed source
until settlement before stopping. Direct `.await` or manual `poll_complete`
remains ordinary Rust outside declared reactor topology, and dropping the
whole source remains the documented non-returning resource boundary.

### 38.6 Building around the reactor: producer isolation

The canonical repair for anything that cannot satisfy the source contract, modeled on Grok's terminal reader (section 4.4 of `RESEARCH.md`). The producer owns the awkward API; the reactor sees only an admitted channel.

```rust
// Terminal events: crossterm's stream is not admitted (dropping a losing
// `next()` can strand its waker), so a dedicated thread owns it.
let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
let reader = TerminalReaderThread::spawn(tx)?;   // RAII owner; joined in teardown
let terminal_input = source::mpsc(rx, close::Emit);
// `Closed` now means "the reader thread exited" — a real, typed event.
```

The spawn is visible at the construction site, the thread has a named owner with a join in teardown, and the reactor arm looks like every other mpsc arm. In K0 there is no `scope` and no `source::channel_task` helper; the owner is explicit application code, and that visibility is the point — a fresh agent reading `build_sources` sees every producer the reactor depends on.

The honest converse, which the benchmark publishes rather than hides (section 37.9): Kittens narrows the supported path only. This compiles:

```rust
input = sources.terminal_input => {
    self.apply(input)?;
    while let Ok(more) = raw_side_channel.try_recv() {  // unbounded, unchecked
        self.apply(more)?;
    }
    Ok(Control::Continue)
}
```

The `drain` bound governs macro-managed drains; a handler remains ordinary Rust. An agent that needs bounded service writes `#[drain(max = N)]` on the arm — the checked spelling — instead of a handler loop.

### 38.7 Rehydration walkthrough: a fresh agent, no conversation history

The context-amnesia scenario this project is designed for (section 0.3): agent B receives only the repository and the request *"voice feels laggy — make voice transcripts faster."* The intended recovery path, in the section 0.3 order:

1. **Open the reactor.** The `voice` arm carries `#[last]` and `#[starvation(allowed, reason = "interim voice transcripts are best effort")]` — the topology position and its rationale are at the edit site, not in a distant design doc.
2. **Attempt the naive edit.** Moving `voice` above `terminal_input` fails: `last` rejects the reorder, and above a protected source, `voice`'s `may_remain_ready` marker forces a yield or a reorder error naming both sources.
3. **Read the diagnostic, not the git history.** The error states the starvation consequence and the legal alternatives: leave topology alone and reduce voice latency in the producer, or explicitly re-tier voice with a yield edge to input — a policy change now visible in review as an attribute diff, not a silent reorder.
4. **Check the boundary.** The compile-fail fixtures beside the reactor (section 26.2) show which invariants are enforced and which — handler latency, transcript batching in the producer — are application facts the agent must reason about with tests.

Under raw Tokio the same request is served by moving a select arm, which compiles, ships, and starves input under load. The delta between those two outcomes is the entire thesis, and it is what the section 37.11 rehydration trials measure rather than assume.

The earliest result that materially moves confidence is a side-by-side core-poll/Tokio expansion that passes the 23-arm and embedded-shape borrow fixtures, no-std/no-alloc and wake-equivalence tests, then rejects shutdown/yield/cycle/source/drain/last mutations at useful spans. Success materially raises confidence in a runtime-independent reactor kernel. A lost wake, bad pin boundary, large code regression, or non-repairable source error moves the implementation back toward a Tokio-specific expansion before broader work.
