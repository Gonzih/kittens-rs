# Async-region revision-11 spec adversarial review

- Date: 2026-08-09
- Reviewer: Claude Code 2.1.224, `claude-opus-4-8`, maximum effort
- Scope: uncommitted spec-first revision-11 documentation; read-only static
  review against the pinned HAL, with repository verification run separately

## Final verdict

**SOUND — 0 P0–P2 issues.** The revision-11 contract is ready for its
spec-first commit. Its concrete async-adapter evidence row correctly remains
OPEN pending implementation, host oracles, and the exact Xtensa reactor/drop-
glue link.

The reviewer read `AGENTS.md`, the complete render SPEC and evidence records,
the full diff, the current transfer/blocking/target fixture code, and the
locally cached `esp-hal` revision
`d48f747ba28accdc51779ba193eba923138e0382`.

## Verified contract points

- The pinned package is `esp-hal` 1.1.0 with Rust 1.88, and the named
  `esp32s3`, `unstable`, and `rt` features plus `critical-section = "1"` are
  compatible with the selected manifest shape.
- `SPI2<'d>`, `DMA_CH0<'d>`, GPIO4–GPIO7/GPIO11/GPIO12 singleton types, the
  SIO0–SIO3/SCK/CS builders, `Command`, `Address`, `SpiDmaBus`, `split`, both
  blocking and owning half-duplex writes, and `SpiDmaTransfer` all exist with
  the spellings required by section 6.8.
- The rejected `Waveshare18V1Sh8601Parts` bundle has an exact consuming
  extractor, while successful admission intentionally becomes static branded
  board ownership. Driven transfer recovery rebuilds the brand; idle transport
  drop is explicitly non-returning.
- The public idle-command facade cannot expose, move, replace, or reconfigure
  the underlying erased bus, so safe code cannot swap SPI3 beneath an SPI2-
  branded ISR contract. Its arbitrary commands and blocking remain published
  escapes.
- The existing fixture-local starter really ignores X/Y and emits RAMWR without
  CASET/PASET, while its completion slot reads SPI2 registers. Replacing it is
  therefore motivated by source evidence rather than a hypothetical gap.
- The 368×16 reference arithmetic is exact: CASET `[00 00 01 6f]`, PASET
  `[00 00 00 0f]`, and 11,776 RGB565 bytes beneath the 16,380-byte cap.
- The selected preflight order, partial-window failure semantics,
  acceptance-atomic RAMWR rejection, and Completed-versus-Cancelled
  linearization are mutually consistent and resource-carrying.
- The noinline generated-reactor hook performs only one opaque noop-waker poll;
  its handlers are linked code paths, not observed completions. The separate
  noinline armed-source drop hook retains target drop glue without claiming it
  executed. Both remain symbol gates rather than executor or HIL evidence.
- The allocator-symbol gate is scoped to the linked noop waker. Arbitrary
  executor `RawWaker` clone/drop/wake callbacks remain unchecked and may
  allocate.
- Synchronous CASET/PASET work during `FlightStarter::start` is correctly
  disclosed as able to block every reactor arm because handler interiors are
  not preempted.
- The open generic `FlightStarter`/`OwnedTransfer` traits, raw HAL calls,
  direct polling, async RAMWRC, target execution, published-registry
  consumption, and all physical-panel behavior remain explicit non-guarantees.

## Non-blocking watch items

- Record the revision-11 contract selection in the unreleased changelog if
  contract changes are treated as user-visible; this repository does, so the
  spec commit adopts that note.
- Keep the blocking feature free of HAL `rt`; only the async feature adds it
  for the concrete interrupt handler.
- The implementation must retain RX/TX command scratch outside the owning
  `SpiDmaTransfer`, because `split` returns scratch before RAMWR consumes only
  the pixel `DmaTxBuf`.

## Verdict

`VERDICT: SOUND`
