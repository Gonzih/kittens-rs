//! ESP32-S3 SPI2 TX-only completion adapter under the profile contract.

#![forbid(unsafe_code)]

use core::{
    cell::RefCell,
    task::{Context, Poll, Waker},
};

use critical_section::Mutex;
use esp_hal::{
    Blocking,
    dma::{DmaTxBuf, DmaTxBuffer},
    interrupt::Priority,
    peripherals::SPI2,
    spi::{
        Error,
        master::{Address, Command, DataMode, SpiDma, SpiDmaTransfer, SpiInterrupt},
    },
};
use kittens_render::{
    geometry::Region,
    transfer::{FlightStarter, OwnedTransfer, Recovered, StartPermit, TransferOutcome},
};

/// The board path seals SPI2 to at most one TX-only display transfer.
struct DoneSlot {
    active: bool,
    event_seen: bool,
    waker: Option<Waker>,
}

static SPI2_DONE: Mutex<RefCell<DoneSlot>> = Mutex::new(RefCell::new(DoneSlot {
    active: false,
    event_seen: false,
    waker: None,
}));

/// Reads the ESP32-S3 SPI2 transfer-done level through the safe PAC accessor.
fn spi2_done_raw() -> bool {
    SPI2::regs().dma_int_raw().read().trans_done().bit()
}

/// Masks the level interrupt before acknowledging its write-one-to-clear bit.
fn mask_and_clear_spi2_done() {
    let regs = SPI2::regs();

    regs.dma_int_ena().modify(|_, w| w.trans_done().clear_bit());
    regs.dma_int_clr()
        .write(|w| w.trans_done().clear_bit_by_one());
}

/// Owns SPI2 completion delivery because esp-hal's async waker is private.
#[esp_hal::handler(priority = Priority::Priority1)]
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

    // A RawWaker implementation is never invoked inside the critical section.
    if let Some(waker) = wake {
        waker.wake();
    }
}

/// Installs and arms the profile handler before the write consumes the driver.
fn arm_spi2(spi: &mut SpiDma<'_, Blocking>) {
    spi.set_interrupt_handler(spi2_transfer_done);

    let stale = critical_section::with(|cs| {
        spi.unlisten(SpiInterrupt::TransferDone);
        spi.clear_interrupts(SpiInterrupt::TransferDone);

        let mut slot = SPI2_DONE.borrow(cs).borrow_mut();
        let stale = slot.waker.take();
        slot.active = true;
        slot.event_seen = false;

        spi.listen(SpiInterrupt::TransferDone);
        stale
    });

    drop(stale);
}

/// Disarms a transfer whose driver is still owned by the HAL transfer value.
fn disarm_active_transfer() -> Option<Waker> {
    critical_section::with(|cs| {
        mask_and_clear_spi2_done();

        let mut slot = SPI2_DONE.borrow(cs).borrow_mut();
        slot.active = false;
        slot.event_seen = false;
        slot.waker.take()
    })
}

/// Disarms both the HAL listener and the profile slot after driver recovery.
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

/// An owning, movable wrapper around the concrete esp-hal transfer.
pub struct Spi2TxTransfer<'d, B> {
    inner: Option<SpiDmaTransfer<'d, Blocking, B>>,
    settled: Option<TransferOutcome>,
}

/// Starts the SH8601 RAM-write quad-data phase on the board's SPI2 path.
pub fn start_sh8601_write<'d, B: DmaTxBuffer>(
    mut spi: SpiDma<'d, Blocking>,
    bytes: usize,
    buffer: B,
) -> Result<Spi2TxTransfer<'d, B>, (Error, SpiDma<'d, Blocking>, B)> {
    arm_spi2(&mut spi);

    match spi.half_duplex_write(
        DataMode::Quad,
        Command::_8Bit(0x32, DataMode::Single),
        Address::_24Bit(0x2c_u32 << 8, DataMode::Single),
        0,
        bytes,
        buffer,
    ) {
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

    fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if self.settled.is_some() {
            return Poll::Ready(());
        }

        let transfer = self.inner.as_ref().expect("live transfer");

        let (ready, replaced, registered) = critical_section::with(|cs| {
            let mut slot = SPI2_DONE.borrow(cs).borrow_mut();
            debug_assert!(slot.active);

            // A completion before this poll remains visible as a level.
            if slot.event_seen || spi2_done_raw() || transfer.is_done() {
                mask_and_clear_spi2_done();
                slot.active = false;
                slot.event_seen = false;
                return (true, slot.waker.take(), None);
            }

            // Register or replace under the same exclusion used by the ISR.
            let replaced = match slot.waker.as_ref() {
                Some(old) if old.will_wake(cx.waker()) => None,
                Some(_) => slot.waker.replace(cx.waker().clone()),
                None => {
                    slot.waker = Some(cx.waker().clone());
                    None
                }
            };

            // The second level check closes completion-during-registration.
            if slot.event_seen || spi2_done_raw() || transfer.is_done() {
                mask_and_clear_spi2_done();
                slot.active = false;
                slot.event_seen = false;
                (true, replaced, slot.waker.take())
            } else {
                (false, replaced, None)
            }
        });

        // Waker clone/drop behavior is kept outside the critical section.
        drop(replaced);
        drop(registered);

        if ready {
            self.settled = Some(TransferOutcome::Completed);
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }

    fn cancel(&mut self) {
        if self.settled.is_some() {
            return;
        }

        let transfer = self.inner.as_mut().expect("live transfer");
        let (already_completed, wake) = critical_section::with(|cs| {
            let mut slot = SPI2_DONE.borrow(cs).borrow_mut();

            let completed = slot.event_seen || spi2_done_raw() || transfer.is_done();

            // This masks the ISR before the cancellation outcome is chosen.
            mask_and_clear_spi2_done();
            slot.active = false;
            slot.event_seen = false;

            (completed, slot.waker.take())
        });

        let outcome = if already_completed {
            TransferOutcome::Completed
        } else {
            // The false completion observation above is the linearization point.
            transfer.cancel();

            // The listener is masked; discard status produced during abort.
            SPI2::regs()
                .dma_int_clr()
                .write(|w| w.trans_done().clear_bit_by_one());
            TransferOutcome::Cancelled
        };

        self.settled = Some(outcome);

        // Cancellation is progress even when hardware produces no later IRQ.
        if let Some(waker) = wake {
            waker.wake();
        }
    }

    fn recover(mut self) -> Recovered<Self::Transport, Self::Buffer> {
        let outcome = self.settled.expect("recover before settlement");
        let transfer = self.inner.take().expect("live transfer");

        // After completion or synchronous cancellation, wait is the HAL's final
        // cleanup/fence and returns ownership of the driver and sent buffer.
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
        let Some(_) = self.inner.as_ref() else {
            return;
        };

        // The current OwnedTransfer contract requires ordinary drop to
        // synchronously cancel pending physical work, not merely lose resources.
        if self.settled.is_none() {
            OwnedTransfer::cancel(self);
        } else {
            drop(disarm_active_transfer());
        }

        let transfer = self.inner.take().expect("checked live transfer");
        let (mut spi, buffer) = transfer.wait();
        disarm_recovered_driver(&mut spi);
        drop(buffer);
        drop(spi);
    }
}

/// Acceptance-atomic failure from the concrete target-bound starter.
pub enum StartError<'d> {
    /// The target requires more bytes than the statically owned DMA buffer.
    RegionTooLarge {
        spi: SpiDma<'d, Blocking>,
        buffer: DmaTxBuf,
    },
    /// esp-hal rejected the transfer before accepting physical work.
    Hal {
        error: Error,
        spi: SpiDma<'d, Blocking>,
        buffer: DmaTxBuf,
    },
}

/// One operation-bound start carrying the driver and exact sent buffer.
pub struct Spi2RegionStart<'d> {
    pub spi: SpiDma<'d, Blocking>,
    pub buffer: DmaTxBuf,
}

impl<'d> FlightStarter for Spi2RegionStart<'d> {
    type Transfer = Spi2TxTransfer<'d, DmaTxBuf>;
    type Error = StartError<'d>;

    /// The profile target controls the byte count accepted by this operation.
    fn start(
        mut self,
        region: Region,
        _permit: StartPermit<'_>,
    ) -> Result<Self::Transfer, Self::Error> {
        let bytes = usize::from(region.width)
            .checked_mul(usize::from(region.height))
            .and_then(|pixels| pixels.checked_mul(2));

        let Some(bytes) = bytes.filter(|bytes| *bytes <= self.buffer.capacity()) else {
            return Err(StartError::RegionTooLarge {
                spi: self.spi,
                buffer: self.buffer,
            });
        };

        self.buffer.set_length(bytes);
        start_sh8601_write(self.spi, bytes, self.buffer)
            .map_err(|(error, spi, buffer)| StartError::Hal { error, spi, buffer })
    }
}
