//! ESP32-S3 blocking SH8601 region transport.
//!
//! This module is available only for Xtensa builds with the default-off
//! `esp32s3-sh8601-blocking` feature. It deliberately accepts an unbranded
//! same-source esp-hal SPI DMA driver: selecting SPI2, GDMA, pins, mode, and
//! frequency remains the caller's board-configuration obligation.

use esp_hal::{
    Blocking,
    dma::{DmaRxBuf, DmaTxBuf},
    spi::{
        Error,
        master::{Address, Command, DataMode, SpiDma, SpiDmaBus},
    },
};

use crate::{
    blocking::{
        self, BlockingRegionWrite, BlockingWritePermit, Sh8601RegionWriteError, Sh8601Wire,
        Sh8601WireMode, Sh8601WireTransfer, write_sh8601_region,
    },
    geometry::Region,
};

/// A sealed blocking SH8601 region writer over esp-hal's safe SPI DMA bus.
///
/// Construction requires fixed RX and TX scratch buffers. Each admitted call
/// copies one command payload or pixel chunk into the HAL-owned TX scratch and
/// waits at the blocking HAL boundary before continuing.
pub struct Esp32s3Sh8601BlockingTransport<'d> {
    bus: SpiDmaBus<'d, Blocking>,
}

impl<'d> Esp32s3Sh8601BlockingTransport<'d> {
    /// Admits a blocking transport only when both DMA scratch buffers satisfy
    /// the profile's fixed 16,380-byte reserve policy.
    ///
    /// Rejection returns the exact SPI driver and both scratch buffers. After
    /// admission, the TX descriptor chain is normalized to the same 16,380
    /// bytes even if its caller-visible logical length was previously shorter.
    ///
    /// # Errors
    ///
    /// Returns the untouched driver and both scratch buffers when either
    /// buffer's capacity is below the fixed profile reserve.
    pub fn try_new(
        spi: SpiDma<'d, Blocking>,
        rx: DmaRxBuf,
        mut tx: DmaTxBuf,
    ) -> Result<Self, (SpiDma<'d, Blocking>, DmaRxBuf, DmaTxBuf)> {
        if rx.capacity() < blocking::SH8601_DMA_CHUNK_BYTES
            || tx.capacity() < blocking::SH8601_DMA_CHUNK_BYTES
        {
            return Err((spi, rx, tx));
        }

        // `SpiDmaBus::half_duplex_write` checks the backing capacity but does
        // not relink a caller-shortened descriptor chain. Admission therefore
        // restores the exact maximum payload length before the bus owns it.
        tx.set_length(blocking::SH8601_DMA_CHUNK_BYTES);

        Ok(Self {
            bus: spi.with_buffers(rx, tx),
        })
    }

    /// Waits for the blocking bus to become idle and returns the exact SPI
    /// driver and DMA scratch buffers supplied at construction.
    pub fn into_parts(self) -> (SpiDma<'d, Blocking>, DmaRxBuf, DmaTxBuf) {
        self.bus.split()
    }
}

impl blocking::private::Sealed for Esp32s3Sh8601BlockingTransport<'_> {}

impl Sh8601Wire for Esp32s3Sh8601BlockingTransport<'_> {
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

impl BlockingRegionWrite for Esp32s3Sh8601BlockingTransport<'_> {
    type Error = Sh8601RegionWriteError<Error>;

    fn write_region_admitted(
        mut self,
        region: Region,
        pixels: &[u8],
        _permit: BlockingWritePermit<'_>,
    ) -> (Self, Result<(), Self::Error>) {
        let result = write_sh8601_region(&mut self, region, pixels);
        (self, result)
    }
}

fn wire_mode(mode: Sh8601WireMode) -> DataMode {
    match mode {
        Sh8601WireMode::Single => DataMode::Single,
        Sh8601WireMode::Quad => DataMode::Quad,
    }
}
