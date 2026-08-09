# Kittens K0 agent guide

Use the lean expression-position grammar only. The maximal grammar in
`SPEC.md` section 11 is an ablation artifact and is not implemented.

```text
kittens::reactor! {
    policy {
        selection: biased;
        required_phases: [initialize, before_poll, after_event];
    }
    initialize { ... Result<(), E> ... }
    before_poll { ... Result<(), E> ... }
    #[source(id)]
    #[readiness(quiescent | may_remain_ready)]
    [relations]
    item = persistent.place => { ... Result<Control<Exit>, E> ... }
    after_event { ... Result<(), E> ... }
}
```

Canonical relations:

- `#[shutdown]`: terminal, unguarded, undrained, protected, and in the leading
  lexical prefix. Its handler returns `Result<Exit, E>`.
- `#[terminal]`: successful handler exits with `Result<Exit, E>`.
- `#[before(other)]`: freezes one source-order edge.
- `#[last]`: makes the source globally final.
- `#[when(bool_expr)]`: snapshots one ordinary bool once per arbitration.
- `#[yields_to(input, when = buffered)]`: disables the higher source while the
  target adapter reports backlog, including between drained items.
- `#[drain(max = N)]`: handles the selected item plus immediate items, at most
  the unsuffixed literal `N` (`1..=4096`), with no allocation.
- `#[starvation(allowed, reason = "...")]`: explicitly weakens the default
  starvation protection. The compiler checks non-emptiness, not truth.

Sources must be persistent path/field expressions in separate storage. Use a
reviewed Kittens adapter. For a cancellation-awkward producer, give an ordinary
owned task/thread the operation and expose its output through an admitted mpsc
source. K0 has no hidden spawn, scope helper, or unchecked escape.

For a portable, no-allocation one-shot operation whose future is `Unpin`, use
`source::OptionalInlineOneShot<F>`. It is locally armed only: create it dormant
with `new`, install work with `arm`, and rearm from a handler or phase whose
successful continuation begins the next arbitration. `future_mut` gives
optional borrowed access for an operation-specific drain request. Because it
returns ordinary `&mut F`, replacing the installed future is a compiling raw
escape; the canonical render path uses the borrow only for `begin_drain`. The
carrier does not schedule a wake when armed, validate the inner future's
producer semantics, or return resources when the source/reactor is dropped.
The heap-pinned Tokio `OptionalOneShot` remains the spelling for retained
`!Unpin` host futures.

The source borrow ends before each handler. `after_event` runs once only after a
whole service window whose handlers all return `Ok(Control::Continue)`. It does
not run after `Stop`, `Err`, or panic. Priority never preempts a handler or
phase.

When repairing an error, preserve the declaration carrying the invariant.
Deleting `shutdown`/`last`, adding a waiver, moving work into an opaque helper,
or using raw selection may compile while weakening the architecture.
