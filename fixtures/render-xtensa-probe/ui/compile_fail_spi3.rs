#![no_std]
#![no_main]

use core::panic::PanicInfo;

use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use kittens_render::esp32s3_sh8601_async::Waveshare18V1Sh8601Parts;

const SCRATCH_BYTES: usize = 16_380;

#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let (rx_bytes, rx_descriptors, tx_bytes, tx_descriptors) =
        esp_hal::dma_buffers!(SCRATCH_BYTES, SCRATCH_BYTES);
    let rx = DmaRxBuf::new(rx_descriptors, rx_bytes).expect("RX scratch");
    let tx = DmaTxBuf::new(tx_descriptors, tx_bytes).expect("TX scratch");

    // Exact negative control: the branded profile accepts SPI2, never SPI3.
    let parts = Waveshare18V1Sh8601Parts::new(
        peripherals.SPI3,
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
    core::hint::black_box(parts);

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
