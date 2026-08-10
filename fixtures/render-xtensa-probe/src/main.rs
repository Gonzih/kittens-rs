#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use core::{
    convert::Infallible,
    future::Future,
    panic::PanicInfo,
    task::{Context, Poll, Waker},
};

use esp_hal::{
    Blocking,
    dma::{DmaRxBuf, DmaTxBuf},
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi, SpiDma},
    },
    time::Rate,
};
use kittens::{reactor::Control, source::OptionalInlineOneShot};
use kittens_render::{
    blocking::SH8601_DMA_CHUNK_BYTES,
    demand::{FrameDemand, Tick},
    esp32s3_sh8601::Esp32s3Sh8601BlockingTransport,
    esp32s3_sh8601_async::{
        Waveshare18V1Sh8601Parts, Waveshare18V1Sh8601StartError, Waveshare18V1Sh8601Transfer,
        Waveshare18V1Sh8601Transport,
    },
    geometry::PanelGeometry,
    sweep::{Sweep, SweepPlan},
    transfer::{InFlight, StartFlightError, TransferOutcome},
};

const ASYNC_STRIPE_HEIGHT: u16 = 16;
const ASYNC_STRIPE_BYTES: usize = 368 * ASYNC_STRIPE_HEIGHT as usize * 2;
const BLOCKING_STRIPE_HEIGHT: u16 = 112;
const BLOCKING_REGION_BYTES: usize = 368 * BLOCKING_STRIPE_HEIGHT as usize * 2;

type AsyncFlight = InFlight<Waveshare18V1Sh8601Transfer<'static>, DmaTxBuf>;
type AsyncCompletion = OptionalInlineOneShot<AsyncFlight>;
type AsyncReactorHook =
    fn(Waveshare18V1Sh8601Parts<'static>, DmaTxBuf, DmaTxBuf) -> Poll<Result<(), Infallible>>;
type AsyncDropHook = fn(Waveshare18V1Sh8601Parts<'static>, DmaTxBuf, DmaTxBuf);

struct AsyncSources {
    transfer_done: AsyncCompletion,
}

fn assert_unpin<T: Unpin>() {}

fn assert_async_types_are_unpin() {
    assert_unpin::<Waveshare18V1Sh8601Transfer<'static>>();
    assert_unpin::<AsyncFlight>();
}

/// Pins the documented blocking configuration-honesty escape: admission
/// accepts any same-source SPI DMA driver with suitably sized scratch.
fn admit_unbranded_blocking_transport<'d>(
    spi: SpiDma<'d, Blocking>,
    rx: DmaRxBuf,
    tx: DmaTxBuf,
) -> Result<Esp32s3Sh8601BlockingTransport<'d>, (SpiDma<'d, Blocking>, DmaRxBuf, DmaTxBuf)> {
    Esp32s3Sh8601BlockingTransport::try_new(spi, rx, tx)
}

fn admit_branded_async_transport(
    parts: Waveshare18V1Sh8601Parts<'static>,
) -> Waveshare18V1Sh8601Transport<'static> {
    match Waveshare18V1Sh8601Transport::try_new(parts) {
        Ok(transport) => transport,
        Err(parts) => {
            // This is a compile/link witness for the exact inverse tuple. The
            // retained hook is never invoked, so it records no runtime branch.
            core::hint::black_box(parts.into_parts());
            panic!("exact async command scratch was rejected")
        }
    }
}

fn start_async_flight(
    sweep: &mut Sweep<()>,
    transport: Waveshare18V1Sh8601Transport<'static>,
    sent: DmaTxBuf,
    spare: DmaTxBuf,
) -> AsyncFlight {
    let target = sweep.next_target().expect("another 16-row stripe target");
    match target.start_flight(spare, transport.into_start(sent)) {
        Ok(flight) => flight,
        Err(failure) => observe_async_start_failure(failure),
    }
}

fn observe_async_start_failure(
    failure: StartFlightError<Waveshare18V1Sh8601StartError<'static>, DmaTxBuf>,
) -> ! {
    let (error, spare, target) = failure.into_parts();
    core::hint::black_box(error.failure());
    let (failure, transport, pixels) = error.into_parts();
    core::hint::black_box((failure, transport, pixels, spare, target));
    panic!("profile async transfer rejected before acceptance")
}

/// Polls a generated reactor exactly once with the allocation-free noop waker.
///
/// This is a link shim, not an executor: a pending result is returned directly
/// and no loop invents scheduling or target-runtime evidence.
#[inline(never)]
fn poll_generated_reactor_once<F: Future>(future: F) -> Poll<F::Output> {
    let mut future = core::pin::pin!(future);
    let mut cx = Context::from_waker(Waker::noop());
    core::hint::black_box(future.as_mut()).poll(&mut cx)
}

/// Retains real generated-reactor handler paths for two completed stripes,
/// same-carrier rearm, and an honestly raced third-flight drain.
///
/// The firmware entry point only retains this function pointer. Even if a
/// board owner called it, the shim would poll once and return; this function is
/// deliberately not a target executor.
#[inline(never)]
fn linked_async_reactor_paths(
    parts: Waveshare18V1Sh8601Parts<'static>,
    mut sent: DmaTxBuf,
    mut spare: DmaTxBuf,
) -> Poll<Result<(), Infallible>> {
    sent.set_length(ASYNC_STRIPE_BYTES);
    spare.set_length(ASYNC_STRIPE_BYTES);

    let transport = admit_branded_async_transport(parts);
    let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, ASYNC_STRIPE_HEIGHT)
        .expect("admitted 16-row async plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let mut sweep = Some(
        demand
            .begin_sweep(Tick(0), ())
            .expect("requested async sweep starts"),
    );
    let first_flight = start_async_flight(
        sweep.as_mut().expect("active async sweep"),
        transport,
        sent,
        spare,
    );
    let mut sources = AsyncSources {
        transfer_done: AsyncCompletion::from_future(first_flight),
    };
    let mut completed_stripes = 0_u8;
    let mut drain_requested = false;

    let future = async {
        kittens::reactor! {
            policy {
                selection: biased;
                required_phases: [before_poll];
            }

            /// The third accepted flight must settle through the same carrier;
            /// cancellation is requested in place so ownership still returns
            /// through the completion handler.
            before_poll {
                if completed_stripes == 2 && !drain_requested {
                    let flight = sources
                        .transfer_done
                        .future_mut()
                        .expect("third accepted flight remains armed");
                    flight.begin_drain();
                    drain_requested = true;
                }
                Ok(())
            }

            /// Each settlement owns transport, sent buffer, spare, and the
            /// move-only sweep witness before the same carrier can be rearmed.
            #[source(transfer_done)]
            #[readiness(quiescent)]
            settled = sources.transfer_done => {
                let outcome = settled.outcome();
                let (transport, sent, spare, witness) = settled.into_parts();
                let reconciled = sweep
                    .as_mut()
                    .expect("active async sweep")
                    .settle(witness)
                    .expect("settlement belongs to the active stripe");

                if completed_stripes < 2 {
                    if outcome != TransferOutcome::Completed
                        || reconciled != TransferOutcome::Completed
                    {
                        panic!("the first two linked paths require completed writes");
                    }

                    completed_stripes += 1;
                    let next_flight = start_async_flight(
                        sweep.as_mut().expect("active async sweep"),
                        transport,
                        spare,
                        sent,
                    );
                    if let Err(already_armed) = sources.transfer_done.arm(next_flight) {
                        core::hint::black_box(already_armed.into_inner());
                        panic!("completion carrier remained armed in its handler");
                    }
                    Ok(Control::Continue)
                } else {
                    match outcome {
                        TransferOutcome::Completed => {
                            if reconciled != TransferOutcome::Completed {
                                panic!("completed drain settlement changed classification");
                            }
                        }
                        TransferOutcome::Cancelled => {
                            if reconciled != TransferOutcome::Cancelled
                                || !sweep.as_ref().expect("active async sweep").is_poisoned()
                            {
                                panic!("cancelled drain must poison its owning sweep");
                            }
                        }
                        TransferOutcome::Failed => {
                            panic!("the concrete SPI2 adapter has no post-start fault source");
                        }
                    }

                    let (aborted, ()) = sweep
                        .take()
                        .expect("active async sweep")
                        .abort()
                        .expect("third settlement clears the outstanding stripe");
                    let _retained_resources =
                        core::hint::black_box((transport, sent, spare, aborted));
                    Ok(Control::Stop(()))
                }
            }
        }
    };

    poll_generated_reactor_once(future)
}

/// Retains the explicit resource-losing escape and concrete transfer drop
/// glue for an armed profile flight.
///
/// The required recovery order is visible: drop the complete source owner,
/// drop the old outstanding sweep, then abandon the active demand epoch.
#[inline(never)]
#[allow(clippy::drop_non_drop)] // Explicit old-owner drop is the recovery protocol boundary.
fn linked_async_drop_path(
    parts: Waveshare18V1Sh8601Parts<'static>,
    mut sent: DmaTxBuf,
    mut spare: DmaTxBuf,
) {
    sent.set_length(ASYNC_STRIPE_BYTES);
    spare.set_length(ASYNC_STRIPE_BYTES);

    let transport = admit_branded_async_transport(parts);
    let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, ASYNC_STRIPE_HEIGHT)
        .expect("admitted 16-row async plan");
    let mut demand = FrameDemand::new(0, plan);
    demand.request();
    let mut sweep = demand
        .begin_sweep(Tick(0), ())
        .expect("requested async drop sweep starts");
    let flight = start_async_flight(&mut sweep, transport, sent, spare);
    let sources = AsyncSources {
        transfer_done: AsyncCompletion::from_future(flight),
    };

    drop(sources);
    drop(sweep);
    demand.abandon_active();
    core::hint::black_box(demand);
}

/// Executes the previously closed multichunk blocking-region gate and retains
/// the two uncalled async evidence hooks in the same linked image.
#[esp_hal::main]
fn main() -> ! {
    assert_async_types_are_unpin();

    let peripherals = esp_hal::init(esp_hal::Config::default());

    // The blocking adapter owns symmetric fixed DMA scratch reserves. The
    // separate pixel storage spans six pixel commands (RAMWR + five RAMWRC).
    let (blocking_rx_bytes, blocking_rx_descriptors, blocking_tx_bytes, blocking_tx_descriptors) =
        esp_hal::dma_buffers!(SH8601_DMA_CHUNK_BYTES, SH8601_DMA_CHUNK_BYTES);
    let (_pixel_rx, _pixel_rx_descriptors, blocking_pixels, _pixel_tx_descriptors) =
        esp_hal::dma_buffers!(0, BLOCKING_REGION_BYTES);
    let blocking_rx =
        DmaRxBuf::new(blocking_rx_descriptors, blocking_rx_bytes).expect("blocking RX scratch");
    let mut blocking_tx =
        DmaTxBuf::new(blocking_tx_descriptors, blocking_tx_bytes).expect("blocking TX scratch");
    // Capacity admission must restore a deliberately shortened descriptor
    // chain before the first HAL write.
    blocking_tx.set_length(1);
    blocking_pixels.fill(0x96);

    let blocking_rx_identity = blocking_rx.as_slice().as_ptr();
    let blocking_tx_identity = blocking_tx.as_slice().as_ptr();
    let blocking_pixels_identity = blocking_pixels.as_ptr();

    // Waveshare ESP32-S3 1.8-inch AMOLED V1 QSPI routing: SPI2/GDMA_CH0,
    // SIO0..3 GPIO4..7, SCK GPIO11, and CS GPIO12 at 40 MHz mode 0.
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

    let blocking_writer = match admit_unbranded_blocking_transport(spi, blocking_rx, blocking_tx) {
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
    core::hint::black_box((
        spi,
        returned_pixels,
        blocking_rx,
        blocking_tx,
        blocking_sweep,
    ));

    // Function-pointer coercion, not a zero-sized function item, makes both
    // unexecuted evidence hooks reachable to the linker. Neither is called.
    let reactor_hook: AsyncReactorHook = linked_async_reactor_paths;
    let drop_hook: AsyncDropHook = linked_async_drop_path;
    core::hint::black_box(reactor_hook);
    core::hint::black_box(drop_hook);

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
