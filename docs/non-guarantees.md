# K0 guarantee boundary

Kittens checks declarations it can consume. It does not infer semantic intent
from names, handler code, comments, or external APIs.

In particular, K0 does not guarantee:

- external event arrival order or that an event ever arrives;
- fairness among incomparable sources;
- handler/phase termination, latency, cancellation safety, or side-effect
  atomicity;
- bounds on a manual loop inside a handler;
- renderer correctness, a single frame in flight, or writer acknowledgement;
- task ownership, cancellation, joining, resource cleanup, or process teardown;
- correctness of runtime guards or dynamic rearming;
- behavior of raw Tokio/Embassy selection, spawning, I/O, or side channels;
- production Embassy, ESP-HAL, board, interrupt, DMA, or power behavior;
- async cleanup after panic, process abort, `mem::forget`, or reactor drop.

Source admission means only that ordinary selection loss retains the operation
or event state promised by that reviewed adapter. Each adapter separately
documents whole-source drop and Tokio cooperative-budget behavior.
