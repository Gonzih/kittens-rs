#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use core::panic::PanicInfo;

use esp_hal::dma::{DmaRxBuf, DmaTxBuf};
use esp_hal::peripherals::{DMA_CH0, GPIO4, GPIO5, GPIO6, GPIO7, GPIO11, GPIO12, SPI2};
use kittens_render::{
    blocking::SH8601_DMA_CHUNK_BYTES,
    demand::{FrameDemand, Tick},
    esp32s3_sh8601_async::{Waveshare18V1Sh8601Parts, Waveshare18V1Sh8601Transport},
    geometry::PanelGeometry,
    sweep::SweepPlan,
};

const STRIPE_HEIGHT: u16 = 16;
const STRIPE_BYTES: usize = 368 * STRIPE_HEIGHT as usize * 2;

type PackagedRegistryStartHook = fn(Waveshare18V1Sh8601Parts<'static>, DmaTxBuf, DmaTxBuf);
type PackagedRegistryPartsHook = fn(
    SPI2<'static>,
    DMA_CH0<'static>,
    GPIO4<'static>,
    GPIO5<'static>,
    GPIO6<'static>,
    GPIO7<'static>,
    GPIO11<'static>,
    GPIO12<'static>,
    DmaRxBuf,
    DmaTxBuf,
) -> Waveshare18V1Sh8601Parts<'static>;

/// Retains the cross-crate registry-HAL singleton constructor boundary.
///
/// The explicit parameter types come from the fixture's direct registry
/// dependency. Returning the packaged profile's branded parts proves that
/// Cargo resolved one compatible HAL type identity across the crate boundary.
#[allow(clippy::too_many_arguments)]
#[inline(never)]
fn linked_packaged_registry_parts(
    spi2: SPI2<'static>,
    dma_ch0: DMA_CH0<'static>,
    sio0_gpio4: GPIO4<'static>,
    sio1_gpio5: GPIO5<'static>,
    sio2_gpio6: GPIO6<'static>,
    sio3_gpio7: GPIO7<'static>,
    sck_gpio11: GPIO11<'static>,
    cs_gpio12: GPIO12<'static>,
    rx: DmaRxBuf,
    tx: DmaTxBuf,
) -> Waveshare18V1Sh8601Parts<'static> {
    Waveshare18V1Sh8601Parts::new(
        spi2,
        dma_ch0,
        sio0_gpio4,
        sio1_gpio5,
        sio2_gpio6,
        sio3_gpio7,
        sck_gpio11,
        cs_gpio12,
        rx,
        tx,
    )
}

/// Retains the normalized package's registry-HAL async start path.
///
/// This hook is never called. It proves only that the packaged public adapter
/// accepts the registry source identity and links through target-owned start;
/// it is not executor, interrupt, wake, cancellation, or board-HIL evidence.
#[inline(never)]
fn linked_packaged_registry_start(
    parts: Waveshare18V1Sh8601Parts<'static>,
    mut sent: DmaTxBuf,
    mut spare: DmaTxBuf,
) {
    sent.set_length(STRIPE_BYTES);
    spare.set_length(STRIPE_BYTES);

    let transport = match Waveshare18V1Sh8601Transport::try_new(parts) {
        Ok(transport) => transport,
        Err(parts) => {
            core::hint::black_box(parts.into_parts());
            return;
        }
    };
    let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, STRIPE_HEIGHT)
        .expect("admitted 16-row packaged-source plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let mut sweep = demand
        .begin_sweep(Tick(0), ())
        .expect("requested packaged-source sweep starts");
    let target = sweep
        .next_target()
        .expect("packaged-source sweep has a first stripe");

    match target.start_flight(spare, transport.into_start(sent)) {
        Ok(flight) => {
            core::hint::black_box(flight);
        }
        Err(failure) => {
            let (error, spare, target) = failure.into_parts();
            core::hint::black_box(error.failure());
            let (failure, transport, sent) = error.into_parts();
            core::hint::black_box((failure, transport, sent, spare, target));
        }
    }

    core::hint::black_box((demand, sweep));
}

/// Constructs the exact board-resource bundle from the direct registry HAL.
///
/// The owned values and retained hook pointer are black-boxed separately. The
/// entrypoint deliberately never calls the hook and therefore claims no
/// target-runtime or physical-board behavior.
#[esp_hal::main]
fn main() -> ! {
    let peripherals = esp_hal::init(esp_hal::Config::default());
    let (rx_bytes, rx_descriptors, tx_bytes, tx_descriptors) =
        esp_hal::dma_buffers!(SH8601_DMA_CHUNK_BYTES, SH8601_DMA_CHUNK_BYTES);
    let (_sent_rx_bytes, _sent_rx_descriptors, sent_bytes, sent_descriptors) =
        esp_hal::dma_buffers!(0, STRIPE_BYTES);
    let (_spare_rx_bytes, _spare_rx_descriptors, spare_bytes, spare_descriptors) =
        esp_hal::dma_buffers!(0, STRIPE_BYTES);

    let rx = DmaRxBuf::new(rx_descriptors, rx_bytes).expect("packaged-source RX scratch");
    let tx = DmaTxBuf::new(tx_descriptors, tx_bytes).expect("packaged-source TX scratch");
    let sent = DmaTxBuf::new(sent_descriptors, sent_bytes).expect("packaged-source sent buffer");
    let spare =
        DmaTxBuf::new(spare_descriptors, spare_bytes).expect("packaged-source spare buffer");

    // Constructor order is the public board brand: SPI2, DMA_CH0,
    // SIO0..3 GPIO4..7, SCK GPIO11, CS GPIO12, then RX/TX command scratch.
    let parts = linked_packaged_registry_parts(
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

    core::hint::black_box(parts);
    core::hint::black_box(sent);
    core::hint::black_box(spare);
    let parts_hook: PackagedRegistryPartsHook = linked_packaged_registry_parts;
    let start_hook: PackagedRegistryStartHook = linked_packaged_registry_start;
    core::hint::black_box(parts_hook);
    core::hint::black_box(start_hook);

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
