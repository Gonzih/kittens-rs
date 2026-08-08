# kittens

Kittens is an experimental `no_std` reactor kernel for explicit,
compile-checked async orchestration in Rust.

It keeps application state in ordinary Rust, polls persistent sources in biased
lexical order, and turns selected scheduling facts into compiler inputs:

- shutdown sources form an unguarded leading prefix;
- `before`, global `last`, and cycle relationships are validated;
- may-remain-ready sources cannot precede protected sources without a direct
  buffered yield;
- macro-managed draining has a literal per-item bound and no batch allocation;
- required `initialize`, `before_poll`, and `after_event` positions cannot be
  deleted independently of their policy declarations;
- only reviewed, sealed source adapters enter repeated arbitration.

Kittens is not an executor, a task scheduler, structured-concurrency runtime,
rendering protocol, hardware abstraction layer, or sandbox. Handlers and phases
are ordinary Rust and remain capable of blocking the whole reactor, running
unchecked loops, or bypassing Kittens with raw runtime calls.

## Smallest reactor

```rust
use core::convert::Infallible;
use kittens::reactor::Control;
use kittens::source::{FixedQueue, Latched};

struct Sources {
    stop: Latched<()>,
    events: FixedQueue<u8, 8>,
}

async fn run(sources: &mut Sources) -> Result<u8, Infallible> {
    kittens::reactor! {
        policy {
            selection: biased;
            required_phases: [after_event];
        }

        #[source(stop)]
        #[readiness(quiescent)]
        #[shutdown]
        _ = sources.stop => {
            Ok(0)
        }

        #[source(events)]
        #[readiness(may_remain_ready)]
        #[starvation(allowed, reason = "this example has no lower interactive source")]
        #[drain(max = 4)]
        #[last]
        event = sources.events => {
            if event == 42 {
                Ok(Control::Stop(event))
            } else {
                Ok(Control::Continue)
            }
        }

        after_event {
            // One application hook after a successful service window.
            Ok(())
        }
    }
}
# let _ = run;
```

With default features, Kittens also supplies Tokio adapters for mpsc channels,
optional mpsc channels, cancellation tokens, absolute optional deadlines, and
retained one-shot futures. The crate itself is always `#![no_std]`; host-only
Tokio code is target-gated so the kernel remains bare-metal-linkable even when
Cargo unifies the `tokio` feature elsewhere in the graph.

## Status

This repository implements the K0 evidence slice defined by [SPEC.md](SPEC.md),
not the older speculative full-v0.1 architecture in that document. The public
surface is intentionally small and still experimental. Formal K0 closure and a
stable crates.io release are not claimed; the remaining gates are explicit. See
[K0-REPORT.md](K0-REPORT.md) for evidence, falsifiers, and unresolved release
work, and [docs/agent-guide.md](docs/agent-guide.md) for the compact canonical
grammar.

## Development

```text
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
cargo doc -p kittens --all-features --no-deps
```

The bare-metal gate additionally builds the `kittens-no-std-fixture` and
`kittens-feature-unifier` packages together for `thumbv7em-none-eabi`.

Licensed under either Apache-2.0 or MIT, at your option.
