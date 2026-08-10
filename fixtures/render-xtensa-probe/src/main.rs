#![no_std]
#![no_main]

mod adapter;

use core::{
    panic::PanicInfo,
    task::{Context, Poll, Waker},
};

use adapter::{Spi2RegionStart, Spi2TxTransfer, StartError};
use esp_hal::{
    Blocking,
    dma::{DmaRxBuf, DmaTxBuf},
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi, SpiDma},
    },
    time::Rate,
};
use kittens_render::{
    blocking::SH8601_DMA_CHUNK_BYTES,
    demand::{FrameDemand, Tick},
    esp32s3_sh8601::Esp32s3Sh8601BlockingTransport,
    geometry::PanelGeometry,
    sweep::SweepPlan,
    transfer::{InFlight, StartFlightError, TransferOutcome},
};

const STRIPE_HEIGHT: u16 = 1;
const STRIPE_BYTES: usize = 368 * STRIPE_HEIGHT as usize * 2;
const BLOCKING_STRIPE_HEIGHT: u16 = 112;
const BLOCKING_REGION_BYTES: usize = 368 * BLOCKING_STRIPE_HEIGHT as usize * 2;

fn assert_unpin<T: Unpin>() {}

fn assert_probe_types_are_unpin() {
    assert_unpin::<Spi2TxTransfer<'static, DmaTxBuf>>();
    assert_unpin::<InFlight<Spi2TxTransfer<'static, DmaTxBuf>, DmaTxBuf>>();
}

/// Pins the documented configuration-honesty escape: admission accepts any
/// same-source blocking SPI DMA driver and suitably sized scratch buffers.
fn admit_unbranded_transport<'d>(
    spi: SpiDma<'d, Blocking>,
    rx: DmaRxBuf,
    tx: DmaTxBuf,
) -> Result<Esp32s3Sh8601BlockingTransport<'d>, (SpiDma<'d, Blocking>, DmaRxBuf, DmaTxBuf)> {
    Esp32s3Sh8601BlockingTransport::try_new(spi, rx, tx)
}

fn observe_start_failure<'d>(failure: StartFlightError<StartError<'d>, DmaTxBuf>) -> ! {
    let (error, spare, target) = failure.into_parts();

    match error {
        StartError::RegionTooLarge { spi, buffer } => {
            core::hint::black_box(spi);
            core::hint::black_box(buffer);
        }
        StartError::Hal { error, spi, buffer } => {
            core::hint::black_box(error);
            core::hint::black_box(spi);
            core::hint::black_box(buffer);
        }
    }

    core::hint::black_box(spare);
    core::hint::black_box(target);
    panic!("SH8601 transfer rejected before acceptance")
}

fn poll_to_settlement<'d>(
    flight: &mut InFlight<Spi2TxTransfer<'d, DmaTxBuf>, DmaTxBuf>,
) -> kittens_render::transfer::Settled<SpiDma<'d, Blocking>, DmaTxBuf, DmaTxBuf> {
    let mut cx = Context::from_waker(Waker::noop());
    loop {
        match flight.poll_complete(&mut cx) {
            Poll::Ready(settled) => break settled,
            Poll::Pending => core::hint::spin_loop(),
        }
    }
}

/// Links one multichunk blocking region write, then two owning transfers.
#[esp_hal::main]
fn main() -> ! {
    assert_probe_types_are_unpin();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    // These two macro invocations instantiate distinct static descriptor arrays
    // and static DMA storage, then the explicit constructors create both owned
    // DmaTxBuf values without an allocator.
    let (_rx_a, _rx_descriptors_a, tx_a, tx_descriptors_a) = esp_hal::dma_buffers!(0, STRIPE_BYTES);
    let (_rx_b, _rx_descriptors_b, tx_b, tx_descriptors_b) = esp_hal::dma_buffers!(0, STRIPE_BYTES);
    let mut sent = DmaTxBuf::new(tx_descriptors_a, tx_a).expect("first DMA buffer");
    let mut spare = DmaTxBuf::new(tx_descriptors_b, tx_b).expect("second DMA buffer");

    // The blocking adapter owns symmetric fixed DMA scratch reserves. The
    // separate pixel storage is larger than one scratch chunk so the real HAL
    // path must issue RAMWR followed by five RAMWRC calls.
    let (blocking_rx_bytes, blocking_rx_descriptors, blocking_tx_bytes, blocking_tx_descriptors) =
        esp_hal::dma_buffers!(SH8601_DMA_CHUNK_BYTES, SH8601_DMA_CHUNK_BYTES);
    let (_pixel_rx, _pixel_rx_descriptors, blocking_pixels, _pixel_tx_descriptors) =
        esp_hal::dma_buffers!(0, BLOCKING_REGION_BYTES);
    let blocking_rx =
        DmaRxBuf::new(blocking_rx_descriptors, blocking_rx_bytes).expect("blocking RX scratch");
    let mut blocking_tx =
        DmaTxBuf::new(blocking_tx_descriptors, blocking_tx_bytes).expect("blocking TX scratch");
    // Pin the capacity-only admission regression: the public constructor must
    // restore a caller-shortened descriptor chain before the first HAL write.
    blocking_tx.set_length(1);

    sent.as_mut_slice().fill(0x5a);
    spare.as_mut_slice().fill(0xc3);
    blocking_pixels.fill(0x96);
    sent.set_length(STRIPE_BYTES);
    spare.set_length(STRIPE_BYTES);

    let sent_identity = sent.as_slice().as_ptr();
    let spare_identity = spare.as_slice().as_ptr();
    let blocking_rx_identity = blocking_rx.as_slice().as_ptr();
    let blocking_tx_identity = blocking_tx.as_slice().as_ptr();
    let blocking_pixels_identity = blocking_pixels.as_ptr();

    // Waveshare ESP32-S3 1.8-inch AMOLED V1 QSPI routing: SPI2/GDMA_CH0,
    // SIO0..3 GPIO4..7, SCK GPIO11, and CS GPIO12 at the audited 40 MHz mode.
    let spi = Spi::new(
        peripherals.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(40))
            .with_mode(Mode::_0),
    )
    .expect("SPI2 configuration")
    .with_sio0(peripherals.GPIO4)
    .with_sio1(peripherals.GPIO5)
    .with_sio2(peripherals.GPIO6)
    .with_sio3(peripherals.GPIO7)
    .with_cs(peripherals.GPIO12)
    .with_sck(peripherals.GPIO11)
    .with_dma(peripherals.DMA_CH0);

    // Exercise the complete admitted region operation before handing the
    // recovered SpiDma to the pre-existing interrupt-driven async probe.
    let blocking_writer = match admit_unbranded_transport(spi, blocking_rx, blocking_tx) {
        Ok(writer) => writer,
        Err((_spi, _rx, _tx)) => panic!("exact blocking scratch was rejected"),
    };
    let blocking_plan =
        SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, BLOCKING_STRIPE_HEIGHT)
            .expect("admitted 112-row blocking plan");
    let mut blocking_demand = FrameDemand::new(0, blocking_plan);
    blocking_demand.request();
    let mut blocking_sweep = blocking_demand
        .begin_sweep(Tick(0), ())
        .expect("blocking sweep starts");
    let blocking_target = blocking_sweep
        .next_target()
        .expect("first blocking stripe target");
    let blocking_settled = blocking_target.write_region(blocking_pixels, blocking_writer);

    if blocking_settled.outcome() != TransferOutcome::Completed {
        panic!("blocking region did not settle completed");
    }

    let (blocking_writer, returned_pixels, blocking_result, blocking_witness) =
        blocking_settled.into_parts();
    if blocking_result.is_err() {
        panic!("blocking SH8601 transaction failed");
    }
    if returned_pixels.as_ptr() != blocking_pixels_identity {
        panic!("blocking pixel slice identity changed");
    }
    if blocking_sweep.settle(blocking_witness) != Ok(TransferOutcome::Completed) {
        panic!("blocking settlement witness mismatch");
    }

    let (spi, blocking_rx, blocking_tx) = blocking_writer.into_parts();
    if blocking_rx.as_slice().as_ptr() != blocking_rx_identity
        || blocking_tx.as_slice().as_ptr() != blocking_tx_identity
        || blocking_tx.len() != SH8601_DMA_CHUNK_BYTES
    {
        panic!("blocking scratch identity or admitted TX length changed");
    }
    core::hint::black_box(returned_pixels);
    core::hint::black_box(blocking_rx);
    core::hint::black_box(blocking_tx);
    core::hint::black_box(blocking_sweep);

    let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, STRIPE_HEIGHT)
        .expect("admitted one-row plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let mut sweep = demand
        .begin_sweep(Tick(0), ())
        .expect("requested sweep starts");

    // The target-bound public entry point moves the actual first transfer and
    // the distinct spare into InFlight, so this is not an unused generic probe.
    let first_target = sweep.next_target().expect("first stripe target");
    let mut first_flight =
        match first_target.start_flight(spare, Spi2RegionStart { spi, buffer: sent }) {
            Ok(flight) => flight,
            Err(failure) => observe_start_failure(failure),
        };
    let first_settled = poll_to_settlement(&mut first_flight);
    let (spi, sent, mut spare, first_witness) = first_settled.into_parts();

    if spare.as_slice().as_ptr() != spare_identity {
        panic!("outer spare identity changed during first transfer");
    }
    if sent.as_slice().as_ptr() != sent_identity {
        panic!("sent buffer identity changed during first transfer");
    }
    if sweep.settle(first_witness) != Ok(TransferOutcome::Completed) {
        panic!("first transfer did not settle completed");
    }

    // Reuse the recovered driver for a second real half-duplex transfer. Rotate
    // the two recovered buffers so the former sent buffer is now the outer
    // spare, then cancel-and-recover it through the current trait contract.
    spare.as_mut_slice().fill(0xa5);
    spare.set_length(STRIPE_BYTES);
    let second_target = sweep.next_target().expect("second stripe target");
    let mut second_flight =
        match second_target.start_flight(sent, Spi2RegionStart { spi, buffer: spare }) {
            Ok(flight) => flight,
            Err(failure) => observe_start_failure(failure),
        };
    second_flight.begin_drain();
    let second_settled = poll_to_settlement(&mut second_flight);
    let second_outcome = second_settled.outcome();
    let (spi, second_sent, second_spare, second_witness) = second_settled.into_parts();

    if second_spare.as_slice().as_ptr() != sent_identity
        || second_sent.as_slice().as_ptr() != spare_identity
    {
        panic!("buffer identity changed during second transfer");
    }
    if sweep.settle(second_witness) != Ok(second_outcome) {
        panic!("second settlement witness mismatch");
    }

    // Keep the fully recovered ownership graph observable in the linked image.
    core::hint::black_box(spi);
    core::hint::black_box(second_sent);
    core::hint::black_box(second_spare);
    core::hint::black_box(second_outcome);

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
