#![no_std]
#![no_main]

use core::panic::PanicInfo;

use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use kittens_render::esp32s3_sh8601_async::{
    Waveshare18V1Sh8601Parts, Waveshare18V1Sh8601Transport,
};

const SCRATCH_BYTES: usize = 16_380;

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let (rx_bytes, rx_descriptors, tx_bytes, tx_descriptors) =
        esp_hal::dma_buffers!(SCRATCH_BYTES, SCRATCH_BYTES);
    let rx = DmaRxBuf::new(rx_descriptors, rx_bytes).expect("RX scratch");
    let tx = DmaTxBuf::new(tx_descriptors, tx_bytes).expect("TX scratch");

    // Positive control: construction and extraction preserve the exact public
    // SPI2/DMA_CH0/GPIO4..7/GPIO11/GPIO12 tuple order.
    let parts = Waveshare18V1Sh8601Parts::new(
        peripherals.SPI2,
        peripherals.DMA_CH0,
        peripherals.GPIO4,
        peripherals.GPIO5,
        peripherals.GPIO6,
        peripherals.GPIO7,
        peripherals.GPIO11,
        peripherals.GPIO12,
        rx,
        tx,
    );
    let (spi2, dma_ch0, gpio4, gpio5, gpio6, gpio7, gpio11, gpio12, rx, tx) = parts.into_parts();
    let parts = Waveshare18V1Sh8601Parts::new(
        spi2, dma_ch0, gpio4, gpio5, gpio6, gpio7, gpio11, gpio12, rx, tx,
    );

    // Both public result arms recover an owned value; the target check does
    // not execute either arm or claim physical configuration behavior.
    match Waveshare18V1Sh8601Transport::try_new(parts) {
        Ok(transport) => {
            let _transport = core::hint::black_box(transport);
        }
        Err(parts) => {
            let _rejected_parts = core::hint::black_box(parts.into_parts());
        }
    }

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
