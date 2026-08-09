## Verdict

Yes—for the ESP32-S3 render path, assuming it is sealed to TX-only SPI2 DMA writes. No esp-hal upstream change is required.

The honest implementation is candidate C: a profile-owned `SPI2` transfer-done ISR and waker slot, exposed through the A′-shaped `poll_done` boundary. Record the result as “C completion mechanism with an outer-`Unpin` carrier” or amend the candidate matrix into separate carrier/completion axes. Calling this pure A′ would contradict [SPEC §7](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/SPEC.md:55), where C is separately ordered.

This uses stable Rust, no allocation, and no self-reference. The esp-hal APIs are nevertheless marked unstable and require its Cargo `unstable` feature; that is API stability, not Rust nightly. The audited release is [esp-hal v1.1.0, commit `d48f747`](https://github.com/esp-rs/esp-hal/releases/tag/esp-hal-v1.1.0).

| Requirement | Exact esp-hal item |
|---|---|
| Owning transfer | `spi::master::SpiDmaTransfer<'d, Blocking, B>` |
| Level check | `SpiDmaTransfer::is_done()` |
| Recovery | `SpiDmaTransfer::wait()` |
| Cancellation | `SpiDmaTransfer::cancel()` |
| Handler installation | `SpiDma<Blocking>::set_interrupt_handler()` |
| Event arm/clear | `SpiDma<Blocking>::listen`, `unlisten`, `clear_interrupts` |
| Event | `SpiInterrupt::TransferDone` |
| Peripheral vector | `peripherals::Interrupt::SPI2`, bound indirectly by `set_interrupt_handler` |
| S3 status | `SPI2::regs().dma_int_raw().read().trans_done()` |
| S3 enable | `SPI2::regs().dma_int_ena().trans_done` |
| S3 acknowledge | `SPI2::regs().dma_int_clr().trans_done` |

The owning transfer’s public methods are documented in [`SpiDmaTransfer`](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32s3/esp_hal/spi/master/struct.SpiDmaTransfer.html); the interrupt is [`SpiInterrupt::TransferDone`](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32s3/esp_hal/spi/master/enum.SpiInterrupt.html), and the safe static register accessor is [`SPI2::regs()`](https://docs.espressif.com/projects/rust/esp-hal/1.1.0/esp32s3/esp_hal/peripherals/struct.SPI2.html).

Important boundaries:

- Keep the driver as `SpiDma<Blocking>`. `write`/`half_duplex_write` still starts DMA and returns immediately with an owning transfer; “Blocking” controls the HAL driver mode.
- Install and arm the handler before `write` consumes the `SpiDma`.
- Use the SPI `TransferDone` event, not GDMA TX EOF. GDMA EOF can precede FIFO/wire completion.
- This exact one-interrupt solution is TX-only. For RX/full-duplex, esp-hal’s own path first waits for DMA RX and then SPI completion, so C would also need the appropriate GDMA RX completion/error handling. The [tagged DMA implementation](https://github.com/esp-rs/esp-hal/blob/d48f747ba28accdc51779ba193eba923138e0382/esp-hal/src/spi/master/dma.rs) shows that ordering.

### Why esp-hal’s async support cannot be reused

`SpiDmaTransfer<Async, _>::wait_for_done(&mut self)` is the only public asynchronous completion entry point. Internally it reaches private `Driver`, `State::waker`, and completion-future types. That future registers the private waker and enables `TransferDone`; its `Drop` disables the listener. Consequently, constructing, polling, and dropping it on every outer poll really does lose the listener.

`into_async()` also installs esp-hal’s private SPI async handler, replacing the profile handler. The internal state is visible in the [tagged SPI master source](https://github.com/esp-rs/esp-hal/blob/d48f747ba28accdc51779ba193eba923138e0382/esp-hal/src/spi/master/mod.rs).

`esp_hal::asynch::AtomicWaker` is public, but the SPI instance’s `AtomicWaker` is not. A profile can create its own static `AtomicWaker`; I would instead use the explicit critical-section slot below because cancellation, drop, rearm, and stale-waker clearing are then directly auditable.

## Concrete implementation shape

This is the implementation I would target. It uses one static slot because the sealed adapter exclusively owns SPI2 and admits at most one SPI2 transfer.

```rust
use core::{
    cell::RefCell,
    task::{Context, Poll, Waker},
};

use critical_section::Mutex;
use esp_hal::{
    Blocking, handler,
    dma::DmaTxBuffer,
    interrupt::Priority,
    peripherals::SPI2,
    spi::{
        Error,
        master::{SpiDma, SpiDmaTransfer, SpiInterrupt},
    },
};

use kittens_render::transfer::{OwnedTransfer, Recovered, TransferOutcome};

struct DoneSlot {
    active: bool,
    event_seen: bool,
    waker: Option<Waker>,
}

static SPI2_DONE: Mutex<RefCell<DoneSlot>> =
    Mutex::new(RefCell::new(DoneSlot {
        active: false,
        event_seen: false,
        waker: None,
    }));

fn spi2_done_raw() -> bool {
    SPI2::regs()
        .dma_int_raw()
        .read()
        .trans_done()
        .bit()
}

fn mask_and_clear_spi2_done() {
    let regs = SPI2::regs();

    regs.dma_int_ena()
        .modify(|_, w| w.trans_done().clear_bit());

    regs.dma_int_clr()
        .write(|w| w.trans_done().clear_bit_by_one());
}

#[handler(priority = Priority::Priority1)]
fn spi2_transfer_done() {
    let wake = critical_section::with(|cs| {
        if !spi2_done_raw() {
            return None;
        }

        // Stop level retriggering before acknowledging the W1C event.
        mask_and_clear_spi2_done();

        let mut slot = SPI2_DONE.borrow(cs).borrow_mut();
        if !slot.active {
            return None;
        }

        slot.event_seen = true;
        slot.waker.take()
    });

    // RawWaker behavior is never invoked while holding the critical section.
    if let Some(waker) = wake {
        waker.wake();
    }
}

fn arm_spi2(spi: &mut SpiDma<'_, Blocking>) {
    spi.set_interrupt_handler(spi2_transfer_done);

    let stale = critical_section::with(|cs| {
        spi.unlisten(SpiInterrupt::TransferDone);
        spi.clear_interrupts(SpiInterrupt::TransferDone);

        let mut slot = SPI2_DONE.borrow(cs).borrow_mut();
        let stale = slot.waker.take();
        slot.active = true;
        slot.event_seen = false;

        // Arm before write()/half_duplex_write() consumes the driver.
        spi.listen(SpiInterrupt::TransferDone);
        stale
    });

    drop(stale);
}

fn disarm_active_transfer() -> Option<Waker> {
    critical_section::with(|cs| {
        mask_and_clear_spi2_done();

        let mut slot = SPI2_DONE.borrow(cs).borrow_mut();
        slot.active = false;
        slot.event_seen = false;
        slot.waker.take()
    })
}

fn disarm_recovered_driver(spi: &mut SpiDma<'_, Blocking>) {
    let stale = critical_section::with(|cs| {
        spi.unlisten(SpiInterrupt::TransferDone);
        spi.clear_interrupts(SpiInterrupt::TransferDone);

        let mut slot = SPI2_DONE.borrow(cs).borrow_mut();
        slot.active = false;
        slot.event_seen = false;
        slot.waker.take()
    });

    drop(stale);
}

pub struct Spi2TxTransfer<'d, B> {
    inner: Option<SpiDmaTransfer<'d, Blocking, B>>,
    settled: Option<TransferOutcome>,
}

pub fn start_spi2_write<'d, B: DmaTxBuffer>(
    mut spi: SpiDma<'d, Blocking>,
    bytes: usize,
    buffer: B,
) -> Result<Spi2TxTransfer<'d, B>, (Error, SpiDma<'d, Blocking>, B)> {
    arm_spi2(&mut spi);

    match spi.write(bytes, buffer) {
        Ok(inner) => Ok(Spi2TxTransfer {
            inner: Some(inner),
            settled: None,
        }),
        Err((error, mut spi, buffer)) => {
            disarm_recovered_driver(&mut spi);
            Err((error, spi, buffer))
        }
    }
}

impl<'d, B> OwnedTransfer for Spi2TxTransfer<'d, B> {
    type Transport = SpiDma<'d, Blocking>;
    type Buffer = B;

    fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<TransferOutcome> {
        if let Some(outcome) = self.settled {
            return Poll::Ready(outcome);
        }

        let transfer = self.inner.as_ref().expect("live transfer");

        let (ready, stale) = critical_section::with(|cs| {
            let mut slot = SPI2_DONE.borrow(cs).borrow_mut();
            debug_assert!(slot.active);

            // First level check: completion before this poll.
            if slot.event_seen || spi2_done_raw() || transfer.is_done() {
                mask_and_clear_spi2_done();
                slot.active = false;
                slot.event_seen = false;
                return (true, slot.waker.take());
            }

            // Register/replace under the same exclusion used by the ISR.
            let replace = match slot.waker.as_ref() {
                Some(old) => !old.will_wake(cx.waker()),
                None => true,
            };
            if replace {
                slot.waker = Some(cx.waker().clone());
            }

            // Second level check closes completion-during-registration.
            if slot.event_seen || spi2_done_raw() || transfer.is_done() {
                mask_and_clear_spi2_done();
                slot.active = false;
                slot.event_seen = false;
                (true, slot.waker.take())
            } else {
                (false, None)
            }
        });

        drop(stale);

        if ready {
            self.settled = Some(TransferOutcome::Completed);
            Poll::Ready(TransferOutcome::Completed)
        } else {
            Poll::Pending
        }
    }

    fn cancel(&mut self) {
        if self.settled.is_some() {
            return;
        }

        let transfer = self.inner.as_mut().expect("live transfer");

        // Keep the critical section short: quiesce the ISR and choose the
        // conservative outcome, then perform the synchronous HAL abort.
        let (already_completed, wake) = critical_section::with(|cs| {
            let mut slot = SPI2_DONE.borrow(cs).borrow_mut();

            let completed =
                slot.event_seen || spi2_done_raw() || transfer.is_done();

            mask_and_clear_spi2_done();
            slot.active = false;
            slot.event_seen = false;

            (completed, slot.waker.take())
        });

        let outcome = if already_completed {
            TransferOutcome::Completed
        } else {
            // This false completion observation is the cancellation
            // linearization point. A physical completion immediately after it
            // is conservatively classified Cancelled and forces repaint.
            transfer.cancel();

            // Listener is masked; remove any status produced during abort.
            SPI2::regs()
                .dma_int_clr()
                .write(|w| w.trans_done().clear_bit_by_one());

            TransferOutcome::Cancelled
        };

        self.settled = Some(outcome);

        // Cancellation made real progress and may produce no hardware IRQ.
        if let Some(waker) = wake {
            waker.wake();
        }
    }

    fn recover(mut self) -> Recovered<Self::Transport, Self::Buffer> {
        let outcome = self.settled.expect("recover before settlement");
        let transfer = self.inner.take().expect("live transfer");

        // After TransferDone/is_done or synchronous cancel, this should make
        // no busy-wait iterations and performs the HAL's final cleanup/fence.
        let (mut spi, buffer) = transfer.wait();
        disarm_recovered_driver(&mut spi);

        Recovered {
            transport: spi,
            buffer,
            outcome,
        }
    }
}

impl<B> Drop for Spi2TxTransfer<'_, B> {
    fn drop(&mut self) {
        if self.inner.is_some() {
            // Resource recovery remains intentionally lost on ordinary drop,
            // but a stale SPI2 IRQ/waker must not survive it.
            drop(disarm_active_transfer());
        }
    }
}
```

For the board’s actual QSPI path, replace `write` with the concrete `half_duplex_write(DataMode, Command, Address, dummy, bytes, buffer)` call. The interrupt logic is unchanged.

## Corrections to A-prime and its oracles

The current “host-model PASS” entry in [K2R0A-LOG.md](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/K2R0A-LOG.md:8) should be downgraded. The six serial tests are useful smoke tests, but they do not meet the normative pass criteria.

1. The model has the exact lost-wake race §8 requires testing. It checks `done` at [lines 100–104](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/tests/k2r0a_a_prime.rs:100), then installs the waker at line 113. Completion between those operations observes no waker, after which polling returns `Pending` forever. Use register-then-recheck and add a deterministic adversarial oracle.

2. `cancel()` sets `cancelled` but does not wake the waker from the preceding pending poll. The test manually repolls immediately and hides the defect. Required trace: pending poll → cancel → assert one progress wake → repoll/recover.

3. §7 requires transport, sent buffer, and spare. [Recovered](/Users/feral/mydev/kittens-render-wt/crates/kittens-render/src/transfer.rs:71) contains no spare, and the oracle checks only transport and sent buffer. Either carry `S` in the outer in-flight state/result or prove by identity that it remained outside and was preserved.

4. Raw `Context` tests do not prove delivery into Kittens. `InFlight` is not a `ReactorSource`, while [`ReactorSource` is sealed](/Users/feral/mydev/kittens-render-wt/crates/kittens/src/source/mod.rs:37). The overall K2R-0A gate still needs a kernel-admitted no-std completion source and a real `reactor!` fixture.

5. `poll_complete` discards the outcome from `poll_done` and trusts a second value from `recover`. Prefer `poll_done -> Poll<()>`, making recovery the sole outcome authority, or require and check equality.

6. `is_draining()` remains true after recovery despite saying “has not yet settled.” Clear it on recovery or return `self.draining && self.transfer.is_some()`.

7. Real `SpiDmaTransfer` reports no post-start failure: `wait()` returns `(SpiDma, B)`, not `Result`. Start failure returns `(Error, SpiDma, B)` before an in-flight transfer exists. Keep `Failed` for the abstract boundary only if another reviewed fault source can produce it; do not claim the esp-hal adapter does.

8. Add waker-replacement, late-IRQ-after-drop/recovery, and transfer-N/transfer-N+1 reuse traces. The static interrupt slot makes these load-bearing.

9. The selected backend capability must be sealed before freeze; the current public `OwnedTransfer` remains externally implementable.

## Xtensa-gated probe

The fixture must be a linked firmware binary, not `cargo check`.

- Pin the exact audited source, preferably:

  ```toml
  esp-hal = {
      git = "https://github.com/esp-rs/esp-hal",
      rev = "d48f747ba28accdc51779ba193eba923138e0382",
      default-features = false,
      features = ["esp32s3", "rt", "unstable"]
  }
  critical-section = "1"
  ```

  A reusable library dependency should request `requires-unstable`; the final firmware enables `unstable`.

- Use `#![no_std]`, `#![no_main]`, `#[esp_hal::main]`, a panic handler, no allocator, and put the adapter under `#![forbid(unsafe_code)]`.
- Instantiate real `SPI2`, a real GDMA channel such as `DMA_CH0`, actual SCK/SIO pins, descriptors, and two `DmaTxBuf`s.
- Monomorphize the actual SH8601 TX-only `half_duplex_write` path, not merely an unused generic function.
- Compile the static slot, `#[handler]`, `SPI2::regs()` status/enable/clear expressions, `poll_done`, cancellation, `wait` recovery, drop cleanup, and a second transfer using the recovered driver.
- Assert the concrete transfer wrapper and `InFlight<_>` are `Unpin`.
- Carry and identity-check the spare buffer in the outer state.
- Build and link with:

  ```text
  cargo +esp build --release --target xtensa-esp32s3-none-elf
  ```

- Include the real admitted Kittens source/reactor shape. Without that, the fixture closes only the HAL/language feasibility question.
- Retain reusable traces for completion during registration, waker replacement, cancel wake, stale IRQ/rearm, second-transfer reuse, and zero self-wakes. Include a negative-control check-then-register implementation that fails.

Compilation establishes that the API, vector binding, ownership, stable-language, no-alloc, and no-self-reference shape exists. It cannot prove silicon interrupt delivery. A small board HIL must additionally demonstrate pending produces no wakes, SPI2 completion produces one wake, next poll is ready, completion-before-first-poll is level-visible, and cancel-and-drain returns transport, sent buffer, and spare.

If ownership of the SPI2 handler/PAC register path is rejected, then the answer changes to “not possible on esp-hal 1.1.0.” The minimal upstream addition is:

```rust
pub fn poll_for_done(
    &mut self,
    cx: &mut core::task::Context<'_>,
) -> core::task::Poll<()>;
```

on `SpiDmaTransfer<Async, B>`, implemented with esp-hal’s existing RX-DMA and SPI wakers, persistent listeners, check-register-recheck, TX cleanup, and cancel/drop disarming. Exposing another borrowing future would not solve A′.

A long-lived `async move` future that owns the transfer and awaits one `wait_for_done()` is also safe and allocation-free, but it is candidate A/B—a pinned future or named task—not a fourth direct `OwnedTransfer::poll_done` implementation.

No repository files were changed.
