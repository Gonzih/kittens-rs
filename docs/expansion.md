# Expansion model

The selected macro expansion is one ordinary async future:

1. generated, nonexecuting trait assertions;
2. optional one-time `initialize`;
3. loop-top `before_poll`;
4. lexical guard and buffered-yield snapshot;
5. one `core::future::poll_fn` polling enabled persistent sources in order;
6. a private owned-event enum that ends source borrows before handler transfer;
7. an ordinary handler match and optional allocation-free immediate drain;
8. one `after_event` after a fully continuing service window.

It contains no spawn, executor, task queue, runtime graph, dynamic dispatch,
unsafe pin projection, or arbitration allocation. Adapter-owned pinned boxes for
Tokio `Sleep`, cancellation waiters, and retained arbitrary futures are measured
adapter costs.

Doc-hidden comparison macros retain core-poll/tag-slot, Tokio-select/event, and
Tokio-select/tag-slot forms. The Tokio controls append an always-enabled
`pending()` sentinel to prevent Tokio's all-disabled panic. Runtime tests require
all four forms to produce the same scripted selections and hook counts under
the measured budget condition.

To inspect a downstream expansion:

```text
cargo expand --test reactor_runtime
```

The checked K0 measurements and representative Grok expansion are retained in
[`expansion-snapshot.md`](expansion-snapshot.md).

Generated identifiers use the `__kittens_` prefix. They are unsupported
implementation details and should not appear in ordinary application APIs.
