# kittens-tui profile specification (K1-TUI slice)

- Status: controlling contract for the first TUI profile slice; authorized 2026-08-08 as the first post-K0 profile under root `SPEC.md` sections 9.4 and 37.13 step 10
- Parent contracts: root [`SPEC.md`](../../SPEC.md) (kernel semantics, section 9.4 profile rules, section 2.1 coverage thesis) and [`RESEARCH.md`](../../RESEARCH.md) section 4 (the inspected Grok Build TUI architecture, pinned commit `393430ee`)
- Evidence basis: the K0 presenter parity oracles (`K0-REPORT.md`, behavioral oracles list) — request coalescing, no-payload draw, last-payload gating, stale acknowledgement, deadline under delayed acknowledgement — all of which ran against an application-owned presenter in both the raw and generated Grok fixtures
- The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHOULD**, and **MAY** are normative within this crate's boundary only; where this document conflicts with root `SPEC.md` kernel semantics, the root controls

This document follows the monorepo convention this crate establishes: the root `SPEC.md` remains the kernel/K0 contract; every profile crate carries its own `SPEC.md` recording scope, enforcement layers, oracles, and deferred work. A profile spec is the crate's rehydration artifact — a fresh agent given only this directory recovers what the crate guarantees, at which layer, and what it refuses to own.

## 1. One-sentence definition

`kittens-tui` is the terminal-orchestration profile of Kittens: it supplies the owned producers (terminal input reader, frame writer), the render-gate protocol (presenter with coalescing, one-frame-in-flight, acknowledgement, draw deadlines), and the terminal lifecycle (raw-mode/alternate-screen session with ordered teardown) that a long-lived TUI reactor needs — as admitted kernel sources and runtime protocols, so that component libraries, harnesses, and next-generation TUI engines build their rendering and input topology on declared law instead of conventions.

## 2. Problem statement

Every hand-rolled async TUI reproduces the same five structures, and each is a defect class when reproduced informally (all five are inspected facts in Grok Build, RESEARCH section 4):

1. **Terminal input**: the terminal event stream is not admissible as a repeated race (a dropped losing `next()` can strand its waker), so correct loops isolate it behind a dedicated reader thread and a channel — or lose keystrokes under load.
2. **Frame writing**: terminal writes are blocking I/O; correct loops move them to an owned writer and need a completion signal — or block the reactor per frame.
3. **Render gating**: draw requests arrive faster than frames should be written; correct loops coalesce requests, keep exactly one frame in flight, gate on acknowledgement, and throttle via a deadline — or exhibit tearing, queue growth, and stale-frame overwrite races.
4. **Ordering**: writer acknowledgements must outrank the render firehose, and input must not starve behind streaming — reactor topology, which the kernel already checks when declared.
5. **Teardown**: input must be parked, accepted frames drained and the writer joined, and only then terminal restoration attempted — or the shell is left in raw mode / the alternate screen on the failure paths that matter most.

The profile makes structures 1, 2, 3, and 5 library types with tested protocols, and leaves 4 to the kernel where it already is. Under the coverage thesis (root section 2.1): the profile shrinks the escape surface of a TUI codebase by giving the awkward producers and the render protocol one reviewed spelling each.

## 3. Consumers

Per root section 3: coding agents building one TUI harness; component-library authors building composable widget/layout/render systems **above** this crate (they own cells, diffing, styling, and layout — this crate hands them a byte-oriented frame lane and the orchestration law); and meta-harnesses that emit TUI harnesses and need one canonical spelling per structure. The public surface is deliberately emittable: constructors take explicit policy values, no builder mazes, no context-dependent sugar.

## 4. Non-goals

`kittens-tui` is not:

- a widget, layout, styling, or component framework;
- a cell buffer, damage/diffing engine, or renderer — a frame payload is opaque bytes produced by the caller;
- a replacement for the kernel's topology checks — arm order, yields, and drains stay declared in `reactor!`;
- the deferred generic `SingleFlight<Ticket, Merge>` gate of root section 11.9 — the `Presenter` here is the *concrete* TUI protocol promoted from the K0 parity oracles; the generic-gate row in root section 37.14 remains open, and this concrete presenter is one comparison arm for it, not its resolution;
- a full `$EDITOR`/`$PAGER` terminal-handoff implementation (the pause/park handshake ships; the drained-handoff choreography is deferred, section 12);
- a guarantee that a handler cannot write to the terminal directly — a raw `println!` bypasses everything here, compiles, and is listed as a negative control.

## 5. Architecture and enforcement layers

Five components, each with a named enforcement layer per root section 9.4. No component redefines kernel semantics; the reactor-facing edge of every component is an ordinary admitted `kittens::source::Mpsc` — the profile adds producers and protocols, never new source contracts.

| Component | Provides | Guarantee | Enforcement layer |
|---|---|---|---|
| `TerminalSession` | raw mode + optional alternate screen | ordered best-effort restoration attempts on drop, including unwind | ordinary RAII `Drop` |
| `InputReader` | owned reader thread, bounded polls, timestamped events | reactor edge is an admitted selection-loss-preserving channel source; reader exit is a typed `Closed` event | structural isolation (root section 20.5 precedent) + `close::Emit` |
| `FrameWriter` | owned writer thread over a generic sink | frames written and flushed in submission order; each acknowledged with its sequence; failure is a typed terminal event; accepted frames drained before exit | runtime protocol + deterministic tests |
| `Presenter` | render gate | coalescing, at most one frame in flight, monotonic sequences, stale-ack rejection, throttle deadline | private runtime state + `&mut` draw permit + deterministic tests |
| wiring guidance | canonical reactor shape | writer acks above render work; input protected; deadline armed in `before_poll`; present in `initialize`/`after_event` | kernel `reactor!` declarations in the caller's loop |

The dependency direction is strict: `kittens-tui` depends on `kittens` (with the Tokio integration) and on the terminal backend; nothing in `kittens` knows this crate exists.

## 6. Public API (K1-TUI surface)

Signatures below are normative for this slice in shape; exact names freeze after the crate's first external consumer or agent trial, whichever comes first.

### 6.1 Frame sequences

```rust
/// Monotonic frame identity issued by the presenter and echoed by the writer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct FrameSeq(u64);
```

`FrameSeq` is `Copy + Ord`; ordering is the acknowledgement contract. One presenter and one writer form one sequence domain; feeding a foreign sequence into a presenter is outside the protocol guarantee (root section 11.9 generation rule, inherited).

### 6.2 Terminal session

```rust
pub struct TerminalSession { /* private */ }

impl TerminalSession {
    /// Enables raw mode, optionally enters the alternate screen.
    pub fn begin(alternate_screen: bool) -> io::Result<TerminalSession>;
}
// Drop: attempts to leave the alternate screen if entered, then attempts to
// disable raw mode and flush. Best-effort; errors during restore are ignored
// because Drop has no channel to report.
```

The production backend is crossterm over stdout. Its operations are held behind
a private injectable seam so lifecycle oracles can drive raw-mode and
alternate-screen success and failure without owning a real tty. The seam is
test apparatus, not an enforcement layer: `TerminalSession` still owns the
entered state and ordinary RAII `Drop` still enforces the ordered restoration
attempts.

### 6.3 Input reader

```rust
pub struct InputReader { /* owns the reader thread */ }

#[derive(Debug)]
pub struct InputEvent {
    pub at: std::time::Instant,
    pub event: crossterm::event::Event,
}

pub type InputSource = kittens::source::Mpsc<InputEvent, kittens::source::close::Emit>;

impl InputReader {
    /// Spawns the owned reader thread. `poll_interval` bounds how often the
    /// thread observes pause and shutdown; it is an explicit policy value.
    pub fn spawn(poll_interval: Duration) -> io::Result<(InputReader, InputSource)>;

    /// Requests that the reader stop consuming terminal events.
    pub fn pause(&self);
    /// Reports whether the reader has acknowledged the pause and parked.
    pub fn is_parked(&self) -> bool;
    /// Resumes event consumption.
    pub fn resume(&self);
}
// Drop: signals shutdown and joins the thread (bounded by poll_interval).
```

The reactor-facing type is the K0-admitted unbounded `Mpsc` with `close::Emit`: `ChannelEvent::Closed` means "the reader thread exited" — a real, typed, fatal-or-handled event, exactly the Grok terminal-reader shape. The unbounded channel is deliberate and inherited from the inspected design: a synchronous reader thread cannot await capacity (root section 11.6 rationale).

### 6.4 Frame writer

```rust
pub struct FrameWriter { /* owns the writer thread */ }
pub struct WriterHandle { /* non-Clone frame sender */ }

#[derive(Debug)]
pub enum WriterEvent {
    /// The frame with this sequence was written and flushed.
    Written(FrameSeq),
    /// A write or flush failed; the writer exits after emitting this.
    Failed { seq: FrameSeq, error: io::Error },
}

pub type WriterSource = kittens::source::Mpsc<WriterEvent, kittens::source::close::Emit>;

impl FrameWriter {
    /// Spawns the owned writer thread over any sink. Production callers pass
    /// stdout; tests pass a Vec-backed sink.
    pub fn spawn<W: io::Write + Send + 'static>(sink: W)
        -> (FrameWriter, WriterHandle, WriterSource);

    /// Closes the frame lane and joins the writer after it drains accepted
    /// frames. The supported teardown path.
    pub fn finish(self, handle: WriterHandle) -> std::thread::Result<()>;
}
```

Writer protocol, normative:

1. frames are written and flushed in submission order;
2. every successfully flushed frame is acknowledged with exactly its sequence, in order;
3. a write/flush failure emits one `Failed { seq, .. }` and the thread exits; subsequent submissions are returned to the caller as errors;
4. closing the frame lane (dropping `WriterHandle`, or `finish`) causes the thread to drain already-accepted frames, acknowledge them, and exit — accepted work is never silently discarded (Grok teardown, RESEARCH section 4.5);
5. `ChannelEvent::Closed` on the writer source means the thread exited (after `Failed`, or after drain-on-close).

`WriterHandle` is non-`Clone`: one submission lane per writer, so "who can submit" is visible in ownership. `FrameWriter` dropped without `finish` while the handle is alive leaves the thread running until the handle drops, after which it drains and exits on its own; this self-terminating detachment is documented, and `finish` is the canonical spelling.

### 6.5 Presenter

`Instant` throughout the presenter is `tokio::time::Instant`: it is what
`kittens::source::OptionalDeadline` arms with, so `deadline()` feeds the
deadline source without conversion, and paused-time tests can drive the
throttle deterministically. It converts from `std` via
`tokio::time::Instant::from_std` where a caller needs it.

```rust
pub struct Presenter { /* private: dirty, force_full, in_flight,
                          last_present, scheduled, min_interval, next_seq */ }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderRequest { Dirty, FullRepaint }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ack { Accepted, Stale }

#[must_use]
pub struct Draw<'p> { /* exclusive &mut Presenter + captured now */ }

impl Presenter {
    /// `min_interval` is the frame throttle and is an explicit policy value;
    /// there is no `Default`.
    pub fn new(min_interval: Duration) -> Presenter;

    pub fn request(&mut self, request: RenderRequest);
    pub fn is_dirty(&self) -> bool;
    pub fn in_flight(&self) -> Option<FrameSeq>;

    /// `Some(draw)` only when dirty, nothing in flight, and the throttle has
    /// elapsed. When throttle-blocked, schedules and exposes `deadline()`.
    pub fn try_begin(&mut self, now: Instant) -> Option<Draw<'_>>;

    /// Earliest eligible presentation instant; `Some` only while a pending
    /// request is throttle-blocked and nothing is in flight. Arm an
    /// `OptionalDeadline` from this in `before_poll`.
    pub fn deadline(&self) -> Option<Instant>;
    /// Clears the fired schedule; call from the deadline arm's handler.
    pub fn on_deadline(&mut self);

    /// `Accepted` iff a frame is in flight and `seq` is at or beyond it.
    /// Anything else — lower, or no in-flight frame — is `Stale`.
    pub fn acknowledge(&mut self, seq: FrameSeq) -> Ack;
}

impl Draw<'_> {
    /// Whether this draw must repaint fully (a sticky FullRepaint request).
    pub fn full_repaint(&self) -> bool;

    /// Reserves the next sequence, submits the payload, records it in flight,
    /// clears the request, and advances the throttle. Fails if the writer
    /// lane is closed; the request is retained.
    pub fn commit(self, writer: &WriterHandle, bytes: Vec<u8>)
        -> Result<FrameSeq, WriterClosed>;

    /// The legitimate no-payload outcome: clears the request and advances the
    /// throttle without creating an in-flight frame.
    pub fn no_output(self);
}
// Dropping a Draw without commit/no_output retains the pending request.
```

### 6.6 Presenter protocol, normative

State: `dirty: bool`, `force_full: bool`, `in_flight: Option<FrameSeq>`, `last_present: Option<Instant>`, `scheduled: Option<Instant>`, `next_seq`.

1. `request(Dirty)` sets `dirty`; `request(FullRepaint)` sets `dirty` and `force_full`. Requests coalesce; `force_full` is sticky until a draw is actually committed or reports no output (Grok's sticky force-repaint).
2. `try_begin(now)` returns `None` when not dirty, when a frame is in flight, or when `now < last_present + min_interval`; in the throttled case it records `scheduled = last_present + min_interval`. It never clears the request.
3. `Draw::commit` assigns `next_seq` (strictly increasing), sets `in_flight`, clears `dirty`/`force_full`/`scheduled`, and sets `last_present` to the `now` captured at `try_begin`. Submission to the writer is synchronous channel enqueue — the Grok queue/reserve step; there is no "queue asynchronously, commit later" split (root section 11.9 rationale, inherited).
4. `Draw::no_output` clears `dirty`/`force_full`/`scheduled` and advances `last_present` without an in-flight frame — a satisfied presentation with no payload never invents an acknowledgement target.
5. `acknowledge(seq)` with `in_flight = Some(t)`: `seq >= t` clears in-flight and returns `Accepted`; `seq < t` returns `Stale` and changes nothing. With no in-flight frame every acknowledgement is `Stale`.
6. `deadline()` is `Some(scheduled)` only while `dirty && in_flight.is_none()`.
7. Exactly one `Draw` can exist at a time — enforced by the exclusive `&mut` borrow; two live draws are a compile error (E0499), the root section 27.7 permit oracle.

What the presenter does not prove, stated here per root section 4.12: it cannot order writes performed outside its writer lane; it cannot verify that `bytes` correspond to the state that marked the presenter dirty; and acknowledgement means the sink accepted and flushed the bytes, not that a terminal rendered them.

### 6.7 Canonical wiring

The canonical reactor shape (shipped as the crate's example, doc-comment rationale included):

- shutdown/cancellation arms lead;
- `writer_events` above the render/stream tier, `#[yields_to(terminal_input, when = buffered)]`, handler: `Written` → `presenter.acknowledge`, `Closed`/`Failed` → terminal error or exit;
- `terminal_input` protected; every may-remain-ready predecessor yields to it;
- `draw_deadline: OptionalDeadline`, armed in `before_poll` from `presenter.deadline()`, handler calls `on_deadline()`;
- `initialize` and `after_event` run `try_begin` → render → `commit`/`no_output`;
- teardown after the reactor returns: confirm input parked (or drop the reader), `writer.finish(handle)`, then drop `TerminalSession` — writer drained before terminal restoration is attempted, session last.

## 7. Cancellation and drop semantics (per root section 26.1 rule: beside each type)

| Type | On drop |
|---|---|
| `TerminalSession` | attempts ordered terminal restoration, best-effort, including panic unwind |
| `InputReader` | signals shutdown, joins the thread (bounded by `poll_interval`); the input source then yields `Closed` |
| `WriterHandle` | closes the frame lane; the writer drains accepted frames, acks them, exits |
| `FrameWriter` | does not join by itself if the handle is alive (documented; `finish` is canonical); joins a finished thread |
| `Draw` | retains the pending request; no state advances |
| `Presenter` | plain state, nothing external |

No component promises delivery after drop, async cleanup, or rollback of bytes already flushed.

## 8. Error model

- `WriterClosed` — the frame lane is closed; carries the payload back (`Vec<u8>` returned to caller) so a caller can decide; the presenter retained the request.
- `WriterEvent::Failed { seq, error }` — typed, terminal, once.
- `io::Error` from constructors (`TerminalSession::begin`, `InputReader::spawn`).
- No crate-wide error enum; ordinary `Result` per root section 19.1.

## 9. Testing oracles (REQUIRED for this slice)

Deterministic, no tty required (writer generic over sink; reader and terminal
lifecycle over private backend seams with the crossterm implementations kept
thin):

1. the five K0 parity scenarios, now against the library presenter: repeated requests coalesce; no-payload draw invents no ack target; stale/early ack does not unlock; ack at-or-beyond unlocks; scheduled deadline survives delayed ack and fires;
2. writer roundtrip over a Vec sink: order, flush-per-frame, ack-per-frame, drain-on-close, typed failure then exit;
3. sticky `FullRepaint` until a committed draw;
4. `Draw` drop retains the request; throttle advances only on commit/no_output;
5. monotonic `FrameSeq` across commits;
6. reader pause/park/resume handshake and shutdown join via the test poller;
7. an integration test wiring presenter + writer + sources through a real `kittens::reactor!` and running scenario 1 end-to-end — the profile's value proof;
8. terminal lifecycle: raw-mode entry failure has no false restore, partial
   alternate-screen entry attempts to roll raw mode back, ordinary drop and
   panic unwind attempt restoration in leave-alternate → disable-raw → flush
   order, and restore errors do not prevent later restore steps;
9. compile-fail: two simultaneous live `Draw`s (E0499);
10. coverage closure: `cargo llvm-cov -p kittens-tui --all-features
    --summary-only` reports 100% line coverage and 100% function coverage for
    every crate source file. Coverage tests remain behavioral oracles for a
    requirement above or for an adversarial path documented at the code site;
    executing an otherwise meaningless path is not an oracle. Any genuinely
    unexecutable line is called out inline and listed in this document rather
    than silently omitted. There are no such exemptions in this slice.

Negative controls, published beside the tests per root section 37.9: a raw `write!` to the sink outside the writer lane compiles; a handler calling `presenter.request` in a tight loop compiles (coalescing bounds frames, not requests); nothing stops a caller from never presenting.

## 10. What this slice measures for the open generic-gate question

Root section 37.14 leaves "generic render gate and phase permit" open pending comparison across the Grok acknowledgement protocol and one ownership-returning display protocol. This crate contributes the first arm: a concrete presenter with the full oracle suite. The embedded fixture's ownership-returning transfer is the second arm. The comparison — whether one generic capacity-returning protocol beats both concrete forms on misuse rejection, borrowing, and agent repair — remains a separately authorized experiment; this spec deliberately does not preempt it.

## 11. Dependency policy

`kittens` (workspace, `tokio` feature), `tokio` (sync/time), `crossterm` (latest compatible line at implementation; the terminal-backend seam stays private so the dependency can be feature-gated later without an API break). Test-only: none beyond the workspace set. `#![forbid(unsafe_code)]`.

## 12. Deferred, with gates

- **Editor/pager handoff choreography** (park → drain accepted frames → hand tty → attempt restore): needs the pause/park primitive shipped here plus a drained-writer barrier; gate: a real handoff fixture with an assertion that no frame bytes interleave with the foreign process.
- **Generic `SingleFlight` promotion**: gate as in section 10.
- **Phase permits** (`try_begin` restricted to `initialize`/`after_event` by capability): deferred with the kernel's phase-capability row; wiring guidance carries the discipline until a permit proves value.
- **Loom models** for the pause/park and shutdown atomics: gate: any reported or suspected handshake race; the K1 handshake is deliberately simple enough to review by hand.
- **Damage-region/diff helpers**: explicitly out — component-library territory; revisit only if three independent consumers rebuild the same thing (root section 4.10 evidence rule).

## 13. Slice acceptance

K1-TUI is done when: all section 9 oracles pass in CI on the workspace
toolchains; the section 9 coverage command reports 100% lines and functions
for each crate source file; the canonical example compiles and runs against a
real terminal by hand; clippy/fmt/doc gates stay clean; the crate README states
the boundary table; and the root README presents the monorepo layout with this
crate as the first profile. Publication to crates.io is out of scope and
remains gated by the root K0-REPORT decision process.
