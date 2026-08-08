# K0 fixture manifest

The fixture budgets below were declared before optimized expansion measurements
were collected. Raising one requires an architecture-review note in
`K0-REPORT.md`.

| Fixture | Predeclared budget | Purpose |
|---|---:|---|
| host embedded-shape generated future | 16 KiB | catches event/slot transfer growth before any MCU claim |
| `thumbv7em-none-eabi` kernel binary `.text` | 32 KiB | portable-core link gate, not board firmware |
| `thumbv7em-none-eabi` writable static data | 2 KiB | no allocator/runtime state in the linked kernel fixture |
| 23-arm host generated future | 128 KiB | catches unusable expansion growth under default rustc limits |

The Grok fixture substitutes fixed queues and latches for Notify, watch,
interval, task-completion, and voice primitives. It tests topology and borrowing,
not primitive parity. Task ownership, presentation, terminal handoff, and
teardown remain application-owned mechanisms.
