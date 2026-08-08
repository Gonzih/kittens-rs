Kittens is a small `no_std` reactor kernel for long-lived async orchestration.
Its [`reactor!`](macro@crate::reactor) macro preserves biased lexical polling
while checking declared shutdown order, precedence, bounded draining, buffered
yields, and required phases. Persistent [`source`] adapters retain their
documented event state when another source wins an arbitration.

Kittens is deliberately not an executor, task scheduler, rendering protocol,
or sandbox. Handlers and phases are ordinary Rust: they can block progress,
perform unchecked loops, or call raw runtime APIs. The guarantees end at those
boundaries.

See the repository README and `K0-REPORT.md` for the current experimental API,
evidence, and known limitations.
