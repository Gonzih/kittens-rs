# Diagnostic index

K0 diagnostic identifiers are provisional but semantic anchors are tested.

| Anchor | Meaning | Canonical repair |
|---|---|---|
| `KTR001` | duplicate source ID | rename one source and update relations |
| `KTR003` | `before`/shutdown/last cycle | remove or reverse the conflicting relation |
| `KTR004` | conflicting or non-final `last` | move one complete last arm to the end |
| `KTR005` | invalid shutdown contract | make it unguarded, undrained, protected, and leading |
| `KTR006` | declared readiness differs from adapter | state the adapter's conservative marker |
| `KTR007` | firehose can starve protected source | reorder, or add a direct buffered yield |
| `KTR008` | invalid or terminal drain | use a supported positive literal or remove it |
| `KTR009` | source is not drainable | use a stable drainable adapter or remove drain |
| `KTR010` | invalid yield/capability/cycle | use an observational backlog target and acyclic edge |
| `KTR011` | phase list/block mismatch | restore the block or deliberately remove both |
| `KTR014` | every arm carries `#[when]`; an all-false snapshot pends forever with no wake | keep one unguarded arm (shutdown qualifies) or use a dormant adapter |
| `KTR015` | temporary source expression | construct a persistent adapter or isolate producer |
| `KTR016` | lexical order violates relation | move the complete arm |
| `KTR019` | guard is not bool | return one synchronous bool |
| `KTR020` | same exact place has two IDs | keep one source identity |
| `SRC001` helper | source is not admitted | retained/latching adapter or owned signal/channel isolation |

Compile-fail fixtures under `crates/kittens/tests/ui` are the executable repair
reference. Compile-pass negative controls live beside them under `ui-pass`.
