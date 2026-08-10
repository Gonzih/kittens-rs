//! Branded ESP32-S3 SH8601 single-payload async region transport.
//!
//! Construction consumes the exact Waveshare 1.8-inch AMOLED V1 SPI2, GDMA,
//! and GPIO singleton roles. The transport keeps its HAL bus private; the
//! exceptional idle-command facade exposes synchronous commands for board
//! initialization without allowing the branded bus to be replaced.

use core::{
    cell::RefCell,
    task::{Context, Poll, Waker},
};

use critical_section::Mutex;
use esp_hal::{
    Blocking,
    dma::{DmaRxBuf, DmaTxBuf},
    interrupt::Priority,
    peripherals::{DMA_CH0, GPIO4, GPIO5, GPIO6, GPIO7, GPIO11, GPIO12, SPI2},
    spi::{
        Error, Mode,
        master::{
            Address, Command, Config as SpiConfig, DataMode, Spi, SpiDma, SpiDmaBus,
            SpiDmaTransfer, SpiInterrupt,
        },
    },
    time::Rate,
};

pub use crate::async_region::Sh8601AsyncStartFailure;

use crate::{
    async_region::{
        CompletionSlotCore, Sh8601ScratchAdmission, decide_sh8601_scratch_admission,
        plan_sh8601_async_start, sh8601_ram_write_start_failure, write_sh8601_async_windows,
    },
    blocking::{Sh8601Wire, Sh8601WireMode, Sh8601WireTransfer},
    geometry::Region,
    transfer::{FlightStarter, OwnedTransfer, Recovered, StartPermit, TransferOutcome},
};

/// Validation-free bundle of the exact Waveshare 1.8-inch AMOLED V1 bus
/// resources and command-scratch reserves.
pub struct Waveshare18V1Sh8601Parts<'d> {
    spi2: SPI2<'d>,
    dma_ch0: DMA_CH0<'d>,
    sio0_gpio4: GPIO4<'d>,
    sio1_gpio5: GPIO5<'d>,
    sio2_gpio6: GPIO6<'d>,
    sio3_gpio7: GPIO7<'d>,
    sck_gpio11: GPIO11<'d>,
    cs_gpio12: GPIO12<'d>,
    rx_scratch: DmaRxBuf,
    tx_scratch: DmaTxBuf,
}

impl<'d> Waveshare18V1Sh8601Parts<'d> {
    /// Binds the exact peripheral singleton types to their board roles.
    ///
    /// Scratch capacity is deliberately checked only by
    /// [`Waveshare18V1Sh8601Transport::try_new`], so callers can always build
    /// and recover this bundle without an implicit rejection or panic.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        spi2: SPI2<'d>,
        dma_ch0: DMA_CH0<'d>,
        sio0_gpio4: GPIO4<'d>,
        sio1_gpio5: GPIO5<'d>,
        sio2_gpio6: GPIO6<'d>,
        sio3_gpio7: GPIO7<'d>,
        sck_gpio11: GPIO11<'d>,
        cs_gpio12: GPIO12<'d>,
        rx_scratch: DmaRxBuf,
        tx_scratch: DmaTxBuf,
    ) -> Self {
        Self {
            spi2,
            dma_ch0,
            sio0_gpio4,
            sio1_gpio5,
            sio2_gpio6,
            sio3_gpio7,
            sck_gpio11,
            cs_gpio12,
            rx_scratch,
            tx_scratch,
        }
    }

    /// Returns every singleton and scratch buffer in constructor order.
    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        SPI2<'d>,
        DMA_CH0<'d>,
        GPIO4<'d>,
        GPIO5<'d>,
        GPIO6<'d>,
        GPIO7<'d>,
        GPIO11<'d>,
        GPIO12<'d>,
        DmaRxBuf,
        DmaTxBuf,
    ) {
        (
            self.spi2,
            self.dma_ch0,
            self.sio0_gpio4,
            self.sio1_gpio5,
            self.sio2_gpio6,
            self.sio3_gpio7,
            self.sck_gpio11,
            self.cs_gpio12,
            self.rx_scratch,
            self.tx_scratch,
        )
    }
}

/// Branded idle SPI2 transport for the exact anchor-board configuration.
///
/// This type intentionally has no `into_parts`: the HAL builder cannot return
/// the consumed GPIO singleton tokens, so an erased driver would not be an
/// honest inverse of construction.
pub struct Waveshare18V1Sh8601Transport<'d> {
    bus: SpiDmaBus<'d, Blocking>,
}

impl<'d> Waveshare18V1Sh8601Transport<'d> {
    /// Admits exact-size command scratch before consuming SPI2 into the fixed
    /// mode-0, 40 MHz Waveshare QSPI binding.
    ///
    /// # Errors
    ///
    /// Returns the untouched parts when either scratch capacity is below
    /// [`crate::blocking::SH8601_DMA_CHUNK_BYTES`]. The fixed HAL configuration
    /// is a reviewed invariant; an unexpected `ConfigError` after SPI2
    /// consumption panics and is not an ordinary resource-returning rejection.
    ///
    /// # Panics
    ///
    /// Panics only if esp-hal rejects the fixed mode-0, 40 MHz configuration
    /// after consuming SPI2. That is the contract's non-returning internal-
    /// invariant boundary because the HAL cannot return the singleton.
    pub fn try_new(
        parts: Waveshare18V1Sh8601Parts<'d>,
    ) -> Result<Self, Waveshare18V1Sh8601Parts<'d>> {
        let tx_len = match decide_sh8601_scratch_admission(
            parts.rx_scratch.capacity(),
            parts.tx_scratch.capacity(),
        ) {
            Sh8601ScratchAdmission::Reject => return Err(parts),
            Sh8601ScratchAdmission::NormalizeTx { len } => len,
        };

        let Waveshare18V1Sh8601Parts {
            spi2,
            dma_ch0,
            sio0_gpio4,
            sio1_gpio5,
            sio2_gpio6,
            sio3_gpio7,
            sck_gpio11,
            cs_gpio12,
            rx_scratch,
            mut tx_scratch,
        } = parts;

        // The HAL validates capacity but does not relink a caller-shortened TX
        // descriptor chain in its blocking bus path.
        tx_scratch.set_length(tx_len);

        let spi = Spi::new(
            spi2,
            SpiConfig::default()
                .with_frequency(Rate::from_mhz(40))
                .with_mode(Mode::_0),
        )
        .unwrap_or_else(|_| panic!("fixed Waveshare SPI2 configuration was rejected"))
        .with_sio0(sio0_gpio4)
        .with_sio1(sio1_gpio5)
        .with_sio2(sio2_gpio6)
        .with_sio3(sio3_gpio7)
        .with_sck(sck_gpio11)
        .with_cs(cs_gpio12)
        .with_dma(dma_ch0);

        Ok(Self {
            bus: spi.with_buffers(rx_scratch, tx_scratch),
        })
    }

    /// Runs an exceptional synchronous board-coordinator command while the
    /// transport is idle.
    pub fn with_idle_commands<R>(
        &mut self,
        f: impl FnOnce(&mut Waveshare18V1Sh8601IdleCommands<'_, 'd>) -> R,
    ) -> R {
        let mut commands = Waveshare18V1Sh8601IdleCommands { bus: &mut self.bus };
        f(&mut commands)
    }

    /// Consumes the idle transport and one caller-filled logical pixel buffer
    /// into the only public starter for this adapter.
    pub fn into_start(self, pixels: DmaTxBuf) -> Waveshare18V1Sh8601Start<'d> {
        Waveshare18V1Sh8601Start {
            transport: self,
            pixels,
        }
    }
}

/// Borrowed, private-field facade for commands outside proof-bearing writes.
pub struct Waveshare18V1Sh8601IdleCommands<'bus, 'd> {
    bus: &'bus mut SpiDmaBus<'d, Blocking>,
}

impl Waveshare18V1Sh8601IdleCommands<'_, '_> {
    /// Performs one synchronous half-duplex command on the idle branded bus.
    ///
    /// # Errors
    ///
    /// Returns the concrete esp-hal SPI error for this unchecked command.
    pub fn half_duplex_write(
        &mut self,
        data_mode: DataMode,
        command: Command,
        address: Address,
        dummy_cycles: u8,
        data: &[u8],
    ) -> Result<(), Error> {
        self.bus
            .half_duplex_write(data_mode, command, address, dummy_cycles, data)
    }
}

impl Sh8601Wire for Waveshare18V1Sh8601Transport<'_> {
    type Error = Error;

    fn write(&mut self, transfer: Sh8601WireTransfer<'_>) -> Result<(), Self::Error> {
        self.bus.half_duplex_write(
            wire_mode(transfer.data_mode),
            Command::_8Bit(u16::from(transfer.opcode), wire_mode(transfer.command_mode)),
            Address::_24Bit(transfer.address, wire_mode(transfer.address_mode)),
            transfer.dummy_cycles,
            transfer.data,
        )
    }
}

fn wire_mode(mode: Sh8601WireMode) -> DataMode {
    match mode {
        Sh8601WireMode::Single => DataMode::Single,
        Sh8601WireMode::Quad => DataMode::Quad,
    }
}

/// One operation-bound start carrying the branded transport and exact sent
/// buffer.
pub struct Waveshare18V1Sh8601Start<'d> {
    transport: Waveshare18V1Sh8601Transport<'d>,
    pixels: DmaTxBuf,
}

/// Acceptance-atomic start rejection with every operation resource retained.
pub struct Waveshare18V1Sh8601StartError<'d> {
    failure: Sh8601AsyncStartFailure<Error>,
    transport: Waveshare18V1Sh8601Transport<'d>,
    pixels: DmaTxBuf,
}

impl<'d> Waveshare18V1Sh8601StartError<'d> {
    /// Borrows the exact preflight, window, or RAMWR-start classification.
    pub fn failure(&self) -> &Sh8601AsyncStartFailure<Error> {
        &self.failure
    }

    /// Recovers the failure, branded idle transport, and unchanged pixel
    /// buffer together.
    pub fn into_parts(
        self,
    ) -> (
        Sh8601AsyncStartFailure<Error>,
        Waveshare18V1Sh8601Transport<'d>,
        DmaTxBuf,
    ) {
        (self.failure, self.transport, self.pixels)
    }
}

static SPI2_DONE: Mutex<RefCell<CompletionSlotCore>> =
    Mutex::new(RefCell::new(CompletionSlotCore::new()));

/// Reads the ESP32-S3 SPI2 transfer-done level through the safe PAC accessor.
fn spi2_done_raw() -> bool {
    SPI2::regs().dma_int_raw().read().trans_done().bit()
}

/// Masks the level interrupt before acknowledging its write-one-to-clear bit.
fn mask_and_clear_spi2_done() {
    let registers = SPI2::regs();

    registers
        .dma_int_ena()
        .modify(|_, write| write.trans_done().clear_bit());
    registers
        .dma_int_clr()
        .write(|write| write.trans_done().clear_bit_by_one());
}

/// Owns SPI2 completion delivery because esp-hal's async waker is private.
#[esp_hal::handler(priority = Priority::Priority1)]
fn spi2_transfer_done() {
    let exit = critical_section::with(|critical_section| {
        let mut slot = SPI2_DONE.borrow(critical_section).borrow_mut();
        let exit = slot.interrupt(spi2_done_raw());
        if exit.acknowledge {
            mask_and_clear_spi2_done();
        }
        exit
    });

    // Invoking an executor's RawWaker remains outside global exclusion.
    if let Some(waker) = exit.wake {
        waker.wake();
    }
}

/// Installs and arms the reviewed handler before RAMWR consumes the driver.
fn arm_spi2(spi: &mut SpiDma<'_, Blocking>) {
    spi.set_interrupt_handler(spi2_transfer_done);

    let stale = critical_section::with(|critical_section| {
        spi.unlisten(SpiInterrupt::TransferDone);
        spi.clear_interrupts(SpiInterrupt::TransferDone);

        let mut slot = SPI2_DONE.borrow(critical_section).borrow_mut();
        debug_assert!(!slot.is_active(), "SPI2 completion slot already active");
        let stale = slot.arm();

        spi.listen(SpiInterrupt::TransferDone);
        stale
    });

    drop(stale);
}

/// Disarms a slot whose SPI driver is still owned by the HAL transfer value.
fn disarm_active_transfer() -> Option<Waker> {
    critical_section::with(|critical_section| {
        mask_and_clear_spi2_done();
        SPI2_DONE.borrow(critical_section).borrow_mut().disarm()
    })
}

/// Disarms both the HAL listener and slot after recovering the SPI driver.
fn disarm_recovered_driver(spi: &mut SpiDma<'_, Blocking>) {
    let stale = critical_section::with(|critical_section| {
        spi.unlisten(SpiInterrupt::TransferDone);
        spi.clear_interrupts(SpiInterrupt::TransferDone);

        SPI2_DONE.borrow(critical_section).borrow_mut().disarm()
    });

    drop(stale);
}

impl<'d> FlightStarter for Waveshare18V1Sh8601Start<'d> {
    type Transfer = Waveshare18V1Sh8601Transfer<'d>;
    type Error = Waveshare18V1Sh8601StartError<'d>;

    fn start(
        self,
        region: Region,
        _permit: StartPermit<'_>,
    ) -> Result<Self::Transfer, Self::Error> {
        let Self {
            mut transport,
            pixels,
        } = self;

        let plan = match plan_sh8601_async_start::<Error>(region, pixels.len()) {
            Ok(plan) => plan,
            Err(failure) => {
                return Err(Waveshare18V1Sh8601StartError {
                    failure,
                    transport,
                    pixels,
                });
            }
        };

        if let Err(failure) = write_sh8601_async_windows(&mut transport, plan) {
            return Err(Waveshare18V1Sh8601StartError {
                failure,
                transport,
                pixels,
            });
        }

        let command = plan.ram_write_command();
        let bytes = plan.bytes();
        let (mut spi, rx_scratch, tx_scratch) = transport.bus.split();
        arm_spi2(&mut spi);

        match spi.half_duplex_write(
            wire_mode(command.data_mode),
            Command::_8Bit(u16::from(command.opcode), wire_mode(command.command_mode)),
            Address::_24Bit(command.address, wire_mode(command.address_mode)),
            command.dummy_cycles,
            bytes,
            pixels,
        ) {
            Ok(inner) => Ok(Waveshare18V1Sh8601Transfer {
                inner: Some(inner),
                rx_scratch: Some(rx_scratch),
                tx_scratch: Some(tx_scratch),
                settled: None,
            }),
            Err((source, mut spi, pixels)) => {
                disarm_recovered_driver(&mut spi);
                Err(Waveshare18V1Sh8601StartError {
                    failure: sh8601_ram_write_start_failure(source),
                    transport: Waveshare18V1Sh8601Transport {
                        bus: spi.with_buffers(rx_scratch, tx_scratch),
                    },
                    pixels,
                })
            }
        }
    }
}

/// One owning, movable RAMWR transfer plus both branded command-scratch
/// buffers and the adapter's exclusive SPI2 completion registration.
pub struct Waveshare18V1Sh8601Transfer<'d> {
    inner: Option<SpiDmaTransfer<'d, Blocking, DmaTxBuf>>,
    rx_scratch: Option<DmaRxBuf>,
    tx_scratch: Option<DmaTxBuf>,
    settled: Option<TransferOutcome>,
}

impl<'d> OwnedTransfer for Waveshare18V1Sh8601Transfer<'d> {
    type Transport = Waveshare18V1Sh8601Transport<'d>;
    type Buffer = DmaTxBuf;

    fn poll_done(&mut self, context: &mut Context<'_>) -> Poll<()> {
        if self.settled.is_some() {
            return Poll::Ready(());
        }

        let transfer = self.inner.as_ref().expect("live SH8601 transfer");
        // Clone before exclusion because a RawWaker vtable is arbitrary
        // executor code. The slot core moves every obsolete registration back
        // out before any destructor can run.
        let mut candidate = Some(context.waker().clone());
        let exit = critical_section::with(|critical_section| {
            let mut slot = SPI2_DONE.borrow(critical_section).borrow_mut();
            let mut completion_visible = || spi2_done_raw() || transfer.is_done();
            let exit = slot.register_then_recheck(&mut candidate, &mut completion_visible);
            if exit.ready {
                mask_and_clear_spi2_done();
            }
            exit
        });

        drop(exit.replaced);
        drop(exit.registered);
        drop(candidate);

        if exit.ready {
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

        let transfer = self.inner.as_mut().expect("live SH8601 transfer");
        let exit = critical_section::with(|critical_section| {
            let mut slot = SPI2_DONE.borrow(critical_section).borrow_mut();
            let exit = slot.cancel(spi2_done_raw() || transfer.is_done());

            // Masking shares the same exclusion as the final completion
            // observation and outcome choice.
            mask_and_clear_spi2_done();
            exit
        });

        if exit.outcome == TransferOutcome::Cancelled {
            transfer.cancel();

            // Discard a level produced while the synchronous abort stopped the
            // hardware; the listener is already masked.
            SPI2::regs()
                .dma_int_clr()
                .write(|write| write.trans_done().clear_bit_by_one());
        }

        self.settled = Some(exit.outcome);

        // Cancellation and an already-visible completion are both progress.
        if let Some(waker) = exit.wake {
            waker.wake();
        }
    }

    fn recover(mut self) -> Recovered<Self::Transport, Self::Buffer> {
        let outcome = self.settled.expect("recover before SH8601 settlement");
        let transfer = self.inner.take().expect("live SH8601 transfer");
        let rx_scratch = self.rx_scratch.take().expect("owned RX command scratch");
        let tx_scratch = self.tx_scratch.take().expect("owned TX command scratch");

        // HAL wait is the final completion/abort fence and returns the exact
        // owning driver plus sent pixel buffer.
        let (mut spi, pixels) = transfer.wait();
        disarm_recovered_driver(&mut spi);

        Recovered {
            transport: Waveshare18V1Sh8601Transport {
                bus: spi.with_buffers(rx_scratch, tx_scratch),
            },
            buffer: pixels,
            outcome,
        }
    }
}

impl Drop for Waveshare18V1Sh8601Transfer<'_> {
    fn drop(&mut self) {
        if self.inner.is_none() {
            return;
        }

        if self.settled.is_none() {
            OwnedTransfer::cancel(self);
        } else {
            drop(disarm_active_transfer());
        }

        let transfer = self.inner.take().expect("checked live SH8601 transfer");
        let rx_scratch = self.rx_scratch.take().expect("owned RX command scratch");
        let tx_scratch = self.tx_scratch.take().expect("owned TX command scratch");
        let (mut spi, pixels) = transfer.wait();
        disarm_recovered_driver(&mut spi);

        let _owned_resources = (
            pixels,
            Waveshare18V1Sh8601Transport {
                bus: spi.with_buffers(rx_scratch, tx_scratch),
            },
        );
    }
}
