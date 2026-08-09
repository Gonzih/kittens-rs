# kittens-tui

Terminal-orchestration profile for the [Kittens](../kittens) reactor kernel:
the owned producers, render-gate protocol, and terminal lifecycle a
long-lived TUI reactor needs — as admitted kernel sources and tested runtime
protocols. The controlling contract is this crate's [`SPEC.md`](SPEC.md);
the canonical wiring is [`examples/status_line.rs`](examples/status_line.rs).

## What each guarantee rests on

| Component | Guarantee | Enforcement layer |
|---|---|---|
| `TerminalSession` | ordered best-effort restoration attempts on drop, including unwind | RAII `Drop` |
| `InputReader` | admitted selection-loss-preserving input source; reader exit is a typed `Closed` event; pause/park handshake | structural isolation + `close::Emit` |
| `FrameWriter` | in-order write+flush, one ack per frame, typed failure then exit, accepted frames drained before exit | runtime protocol + deterministic tests |
| `Presenter` | request coalescing, at most one frame in flight, monotonic sequences, stale-ack rejection, throttle deadline | private runtime state + exclusive `Draw` permit (two live draws are E0499) + deterministic tests |
| wiring | acks above render work, input protected, deadline armed in `before_poll` | `kittens::reactor!` declarations in *your* loop |

The terminal and input backends have private injectable seams for deterministic,
no-tty oracles. Those seams are test apparatus only: terminal restoration
attempts are still enforced by `TerminalSession`'s RAII `Drop`, and input
isolation is still enforced by the owned reader thread plus its admitted
channel edge.

## Coverage gate

Every crate source file must report 100% line coverage and 100% function
coverage under:

```console
cargo llvm-cov -p kittens-tui --all-features --summary-only
```

Coverage tests are behavioral oracles for `SPEC.md` obligations or documented
adversarial paths, never line-execution probes. This slice has no coverage
exemptions; any future genuinely unexecutable line must carry an inline
justification and be listed in `SPEC.md` section 9.

The readers' and writers' private optional thread slots exist only so teardown
can move out each join handle exactly once. No public path constructs an empty
live owner, so teardown asserts that private invariant instead of presenting an
impossible state as a behavioral coverage target.

## What this crate is not

Not a widget, layout, styling, cell-buffer, or diffing framework — a frame
payload is opaque bytes, and component libraries own everything above it.
Not a guarantee against raw writes: a `println!` bypasses the writer lane,
compiles, and is listed as a negative control in the spec. The kernel's
topology checks stay declared in your reactor, where they belong.

## Shape

```text
terminal ──(reader thread)──▶ InputSource ─┐
                                           ├─▶ kittens::reactor! ──▶ Presenter ── Draw::commit ──▶ WriterHandle
sink ◀──(writer thread, ack per frame)──── WriterSource ◀──────────────────────────────────────────┘
```

Teardown: park/drop the reader → `writer.finish(handle)` (drains accepted
frames) → drop `TerminalSession` last.
