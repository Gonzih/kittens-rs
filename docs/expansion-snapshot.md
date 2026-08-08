# Checked expansion snapshot

Snapshot date: 2026-08-07. Tooling: `rustc 1.96.0` and
`cargo-expand 1.0.124` on `aarch64-apple-darwin`.

The two-arm `minimal::run` expansion is 126 lines, 251 whitespace-delimited
words, and 6,225 bytes. Its SHA-256 is
`3169c6f44f08650a5b805f3635b14fba3c30f9183c15a2831f7512010a92a328`.

The selected 23-arm `App::run_core_event` implementation is 1,212 lines, 2,222
words, and 67,140 bytes. Its SHA-256 is
`21eb6dc063fec6e9222d9804bc767d63f1f880434906be08c6a7c15b122d5a3f`.
The arm count grows by 11.5x while these three expansion measures grow by
9.6x, 8.9x, and 10.8x respectively. Neither fixture sets `recursion_limit` or
`type_length_limit`.

Reproduce the small snapshot with:

```text
cargo expand -p kittens --example minimal run
```

The Grok test contains four generated `impl App` controls. The selected
core/event implementation is the second `impl App` in expanded output. The
measurement used:

```text
cargo expand -p kittens --test grok_shape \
  | awk '/^impl App \{$/{n++; if(n==3) exit} n==2{print}'
```

Representative excerpt (middle arms omitted only in this document):

```rust,ignore
#[allow(non_camel_case_types)]
enum __KittensEvent<T0, T1, /* ... */, T22> {
    Source0(T0),
    Source1(T1),
    // ...
    Source22(T22),
}

let __kittens_event = core::future::poll_fn(|__kittens_cx| {
    if __kittens_enabled_0 {
        match ::kittens::source::ReactorSource::poll_next(
            &mut sources.connection_cancel,
            __kittens_cx,
        ) {
            core::task::Poll::Ready(item) => {
                return core::task::Poll::Ready(__KittensEvent::Source0(item));
            }
            core::task::Poll::Pending => {}
        }
    }
    // The remaining enabled sources repeat this lexical Poll path.
    core::task::Poll::Pending
})
.await;

match __kittens_event {
    __KittensEvent::Source0(item) => {
        // The source borrow has ended before the user handler runs.
        let _ = item;
        // ...
    }
    // ...
}
```

Inspection found one owned event enum, one lexical `poll_fn`, direct per-arm
polls, and an external handler match. It found no generated spawn, runtime
graph, dynamic dispatch, box, unsafe projection, or arbitration allocation.
