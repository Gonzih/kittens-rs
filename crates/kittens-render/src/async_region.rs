//! Target-neutral cores for the branded single-payload SH8601 adapter.
//!
//! The Xtensa shell owns peripheral construction, critical-section exclusion,
//! interrupt masking, and esp-hal resource movement. This module owns the
//! decisions those boundaries consume: exact region admission and shared
//! CASET/PASET envelopes, scratch admission, and completion-slot transitions.
//! Host tests exercise these same decisions without claiming that a modeled
//! level or wire call certifies the target mapping.

#![cfg_attr(
    not(all(feature = "esp32s3-sh8601-async", target_arch = "xtensa")),
    allow(dead_code)
)]

use core::task::Waker;

use crate::{
    blocking::{
        SH8601_DMA_CHUNK_BYTES, Sh8601RegionPlan, Sh8601RegionWriteError, Sh8601Wire,
        Sh8601WireCommand, plan_sh8601_region, validate_sh8601_exact_len,
    },
    geometry::Region,
    transfer::TransferOutcome,
};

/// Why the profile-owned SH8601 async adapter rejected one start.
#[derive(Debug)]
pub enum Sh8601AsyncStartFailure<E> {
    /// Shared geometry, exact-length, or window-I/O failure.
    Region(Sh8601RegionWriteError<E>),
    /// The exact RGB565 region does not fit the one owning DMA payload.
    AsyncPayloadTooLarge {
        /// Exact required RGB565 byte count.
        bytes: u32,
        /// Maximum admitted owning payload.
        max: usize,
    },
    /// esp-hal rejected RAMWR before accepting physical work.
    RamWriteStart {
        /// Concrete HAL start error.
        source: E,
    },
}

/// A validated single-payload start plan.
///
/// Private fields make the prescribed geometry, capacity, and descriptor-
/// length precedence the sole way for the target shell to obtain commands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Sh8601AsyncStartPlan {
    region: Sh8601RegionPlan,
}

impl Sh8601AsyncStartPlan {
    /// Exact logical pixel-buffer length admitted by this plan.
    pub(crate) const fn bytes(self) -> usize {
        self.region.expected_bytes() as usize
    }

    /// Shared RAMWR envelope for the owning HAL start.
    pub(crate) const fn ram_write_command(self) -> Sh8601WireCommand {
        self.region.ram_write_command()
    }
}

/// Applies async preflight in the exact revision-11 precedence.
pub(crate) fn plan_sh8601_async_start<E>(
    region: Region,
    actual: usize,
) -> Result<Sh8601AsyncStartPlan, Sh8601AsyncStartFailure<E>> {
    let plan = plan_sh8601_region(region).map_err(Sh8601AsyncStartFailure::Region)?;
    let bytes = plan.expected_bytes();
    if u64::from(bytes) > SH8601_DMA_CHUNK_BYTES as u64 {
        return Err(Sh8601AsyncStartFailure::AsyncPayloadTooLarge {
            bytes,
            max: SH8601_DMA_CHUNK_BYTES,
        });
    }
    validate_sh8601_exact_len(plan, actual).map_err(Sh8601AsyncStartFailure::Region)?;
    Ok(Sh8601AsyncStartPlan { region: plan })
}

/// Emits the two synchronous window boundaries for an admitted async start.
///
/// RAMWR remains separate because the target shell must first split its
/// blocking command bus, arm SPI2, and transfer ownership of the pixel buffer.
pub(crate) fn write_sh8601_async_windows<W: Sh8601Wire>(
    wire: &mut W,
    plan: Sh8601AsyncStartPlan,
) -> Result<(), Sh8601AsyncStartFailure<W::Error>> {
    let column = plan.region.column_transfer();
    let stage = column.stage;
    wire.write(column).map_err(|source| {
        Sh8601AsyncStartFailure::Region(Sh8601RegionWriteError::Io { stage, source })
    })?;

    let page = plan.region.page_transfer();
    let stage = page.stage;
    wire.write(page).map_err(|source| {
        Sh8601AsyncStartFailure::Region(Sh8601RegionWriteError::Io { stage, source })
    })
}

/// Maps the acceptance-atomic owning HAL boundary into the public failure.
pub(crate) const fn sh8601_ram_write_start_failure<E>(source: E) -> Sh8601AsyncStartFailure<E> {
    Sh8601AsyncStartFailure::RamWriteStart { source }
}

/// Construction action selected before any peripheral configuration.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Sh8601ScratchAdmission {
    /// Return the untouched board-resource bundle.
    Reject,
    /// Normalize the TX descriptor chain to the fixed scratch reserve, then
    /// configure the board peripheral.
    NormalizeTx {
        /// Exact logical descriptor length to install.
        len: usize,
    },
}

/// Checks both scratch capacities before permitting TX normalization.
pub(crate) const fn decide_sh8601_scratch_admission(
    rx_capacity: usize,
    tx_capacity: usize,
) -> Sh8601ScratchAdmission {
    if rx_capacity < SH8601_DMA_CHUNK_BYTES || tx_capacity < SH8601_DMA_CHUNK_BYTES {
        Sh8601ScratchAdmission::Reject
    } else {
        Sh8601ScratchAdmission::NormalizeTx {
            len: SH8601_DMA_CHUNK_BYTES,
        }
    }
}

/// State protected by the target adapter's global SPI2 critical section.
///
/// The concrete `Waker` shape is deliberate: this is the production SPI2 slot,
/// not a generic container, and keeping one instantiation makes the coverage
/// gate exercise the exact code linked by the target shell.
pub(crate) struct CompletionSlotCore {
    active: bool,
    event_seen: bool,
    waker: Option<Waker>,
}

impl CompletionSlotCore {
    /// Creates one inactive, empty completion slot.
    pub(crate) const fn new() -> Self {
        Self {
            active: false,
            event_seen: false,
            waker: None,
        }
    }

    /// Activates a fresh transfer and returns any stale registration.
    #[must_use = "drop the stale registration only after leaving exclusion"]
    pub(crate) fn arm(&mut self) -> Option<Waker> {
        self.active = true;
        self.event_seen = false;
        self.waker.take()
    }

    /// Records one sampled interrupt level.
    ///
    /// A true level must be masked and acknowledged by the target shell even
    /// if no transfer is active. Any wake registration is returned so waking
    /// occurs after exclusion.
    pub(crate) fn interrupt(&mut self, level: bool) -> CompletionInterruptExit {
        if !level {
            return CompletionInterruptExit {
                acknowledge: false,
                wake: None,
            };
        }

        if self.active {
            self.event_seen = true;
            CompletionInterruptExit {
                acknowledge: true,
                wake: self.waker.take(),
            }
        } else {
            CompletionInterruptExit {
                acknowledge: true,
                wake: None,
            }
        }
    }

    /// Registers a candidate, with a completion observation on both sides.
    ///
    /// The caller clones `candidate` before exclusion. `completion_visible`
    /// is called before registration and again afterward unless the first
    /// observation settles. All unused, replaced, and completed registrations
    /// remain in the candidate or returned exit and therefore leave exclusion
    /// before their destructor can run.
    pub(crate) fn register_then_recheck(
        &mut self,
        candidate: &mut Option<Waker>,
        completion_visible: &mut dyn FnMut() -> bool,
    ) -> CompletionPollExit {
        debug_assert!(self.active, "registration requires an active slot");

        if self.event_seen || completion_visible() {
            self.active = false;
            self.event_seen = false;
            return CompletionPollExit {
                ready: true,
                replaced: None,
                registered: self.waker.take(),
            };
        }

        let replace = match self.waker.as_ref() {
            Some(current) => {
                !current.will_wake(candidate.as_ref().expect("candidate supplied by caller"))
            }
            None => true,
        };
        let replaced = if replace {
            self.waker
                .replace(candidate.take().expect("candidate supplied by caller"))
        } else {
            None
        };

        if self.event_seen || completion_visible() {
            self.active = false;
            self.event_seen = false;
            CompletionPollExit {
                ready: true,
                replaced,
                registered: self.waker.take(),
            }
        } else {
            CompletionPollExit {
                ready: false,
                replaced,
                registered: None,
            }
        }
    }

    /// Linearizes cancellation against one final completion observation.
    ///
    /// The target shell masks the listener inside the same exclusion, then
    /// synchronously aborts only for `Cancelled`. The returned registration is
    /// woken after exclusion because either outcome is progress.
    pub(crate) fn cancel(&mut self, completion_visible: bool) -> CompletionCancelExit {
        let completed = self.event_seen || completion_visible;
        self.active = false;
        self.event_seen = false;
        CompletionCancelExit {
            outcome: if completed {
                TransferOutcome::Completed
            } else {
                TransferOutcome::Cancelled
            },
            wake: self.waker.take(),
        }
    }

    /// Clears the slot and returns any registration for out-of-lock drop.
    #[must_use = "drop the stale registration only after leaving exclusion"]
    pub(crate) fn disarm(&mut self) -> Option<Waker> {
        self.active = false;
        self.event_seen = false;
        self.waker.take()
    }

    /// Whether a target transfer currently owns this slot.
    pub(crate) const fn is_active(&self) -> bool {
        self.active
    }
}

/// Target actions after one ISR-side slot transition.
pub(crate) struct CompletionInterruptExit {
    /// Whether the sampled level must be masked and acknowledged.
    pub(crate) acknowledge: bool,
    /// Registration to wake after leaving exclusion.
    pub(crate) wake: Option<Waker>,
}

/// Target actions after register-then-recheck.
pub(crate) struct CompletionPollExit {
    /// Whether completion won either observation position.
    pub(crate) ready: bool,
    /// Previous registration displaced by the candidate.
    pub(crate) replaced: Option<Waker>,
    /// Registration removed because completion won.
    pub(crate) registered: Option<Waker>,
}

/// Target actions after cancellation linearization.
pub(crate) struct CompletionCancelExit {
    /// Completed if the final observation was true, otherwise cancelled.
    pub(crate) outcome: TransferOutcome,
    /// Registration to wake after leaving exclusion.
    pub(crate) wake: Option<Waker>,
}

#[cfg(test)]
mod tests {
    extern crate std;

    use core::{
        cell::Cell,
        task::{Context, Poll, Waker},
    };
    use std::{
        boxed::Box,
        rc::Rc,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        task::Wake,
        vec::Vec,
    };

    use crate::{
        blocking::{Sh8601Axis, Sh8601PixelCommand, Sh8601WireMode, Sh8601WriteStage},
        demand::{FrameDemand, Tick},
        geometry::PanelGeometry,
        sweep::SweepPlan,
        transfer::{FlightStarter, OwnedTransfer, Recovered, StartPermit},
    };

    use super::*;

    const REFERENCE_REGION: Region = Region {
        x: 0,
        y: 0,
        width: 368,
        height: 16,
    };
    const REFERENCE_BYTES: usize = 11_776;

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct InjectedIo {
        boundary: usize,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum FailureKind {
        EmptyWidth,
        EmptyHeight,
        CoordinateOverflow(Sh8601Axis),
        OutOfBounds(Region),
        WrongByteLength { expected: u32, actual: usize },
        Io(Sh8601WriteStage, InjectedIo),
        AsyncPayloadTooLarge { bytes: u32, max: usize },
        RamWriteStart(InjectedIo),
    }

    fn failure_kind(failure: &Sh8601AsyncStartFailure<InjectedIo>) -> FailureKind {
        match failure {
            Sh8601AsyncStartFailure::Region(Sh8601RegionWriteError::EmptyWidth) => {
                FailureKind::EmptyWidth
            }
            Sh8601AsyncStartFailure::Region(Sh8601RegionWriteError::EmptyHeight) => {
                FailureKind::EmptyHeight
            }
            Sh8601AsyncStartFailure::Region(Sh8601RegionWriteError::CoordinateOverflow {
                axis,
            }) => FailureKind::CoordinateOverflow(*axis),
            Sh8601AsyncStartFailure::Region(Sh8601RegionWriteError::OutOfBounds { region }) => {
                FailureKind::OutOfBounds(*region)
            }
            Sh8601AsyncStartFailure::Region(Sh8601RegionWriteError::WrongByteLength {
                expected,
                actual,
            }) => FailureKind::WrongByteLength {
                expected: *expected,
                actual: *actual,
            },
            Sh8601AsyncStartFailure::Region(Sh8601RegionWriteError::Io { stage, source }) => {
                FailureKind::Io(*stage, *source)
            }
            Sh8601AsyncStartFailure::AsyncPayloadTooLarge { bytes, max } => {
                FailureKind::AsyncPayloadTooLarge {
                    bytes: *bytes,
                    max: *max,
                }
            }
            Sh8601AsyncStartFailure::RamWriteStart { source } => {
                FailureKind::RamWriteStart(*source)
            }
        }
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    struct RecordedCall {
        stage: Sh8601WriteStage,
        opcode: u8,
        address: u32,
        command_mode: Sh8601WireMode,
        address_mode: Sh8601WireMode,
        data_mode: Sh8601WireMode,
        dummy_cycles: u8,
        data: Vec<u8>,
    }

    impl RecordedCall {
        fn from_transfer(transfer: crate::blocking::Sh8601WireTransfer<'_>) -> Self {
            Self {
                stage: transfer.stage,
                opcode: transfer.opcode,
                address: transfer.address,
                command_mode: transfer.command_mode,
                address_mode: transfer.address_mode,
                data_mode: transfer.data_mode,
                dummy_cycles: transfer.dummy_cycles,
                data: transfer.data.to_vec(),
            }
        }
    }

    struct Identity(Box<u8>);

    impl Identity {
        fn new(value: u8) -> Self {
            Self(Box::new(value))
        }

        fn ptr(&self) -> *const u8 {
            core::ptr::from_ref(self.0.as_ref())
        }
    }

    struct ModelTransport {
        identity: Identity,
        rx_scratch: Identity,
        tx_scratch: Identity,
        calls: Vec<RecordedCall>,
        fail_at: Option<usize>,
    }

    impl ModelTransport {
        fn new(fail_at: Option<usize>) -> Self {
            Self {
                identity: Identity::new(0x11),
                rx_scratch: Identity::new(0x22),
                tx_scratch: Identity::new(0x33),
                calls: Vec::new(),
                fail_at,
            }
        }

        fn identities(&self) -> (*const u8, *const u8, *const u8) {
            (
                self.identity.ptr(),
                self.rx_scratch.ptr(),
                self.tx_scratch.ptr(),
            )
        }
    }

    impl Sh8601Wire for ModelTransport {
        type Error = InjectedIo;

        fn write(
            &mut self,
            transfer: crate::blocking::Sh8601WireTransfer<'_>,
        ) -> Result<(), Self::Error> {
            let boundary = self.calls.len();
            self.calls.push(RecordedCall::from_transfer(transfer));
            if self.fail_at == Some(boundary) {
                Err(InjectedIo { boundary })
            } else {
                Ok(())
            }
        }
    }

    struct ModelPixels {
        identity: Identity,
        bytes: Vec<u8>,
    }

    impl ModelPixels {
        fn patterned(len: usize, seed: u8) -> Self {
            let bytes = (0..len)
                .map(|index| index.to_le_bytes()[0].wrapping_mul(37).wrapping_add(seed))
                .collect();
            Self {
                identity: Identity::new(seed),
                bytes,
            }
        }

        fn ptr(&self) -> *const u8 {
            self.identity.ptr()
        }
    }

    struct ModelAccepted {
        transport: ModelTransport,
        pixels: ModelPixels,
    }

    type ModelRejected = (
        Sh8601AsyncStartFailure<InjectedIo>,
        ModelTransport,
        ModelPixels,
    );

    #[allow(clippy::result_large_err)] // The rejection trace deliberately owns every resource.
    fn model_start(
        mut transport: ModelTransport,
        pixels: ModelPixels,
        region: Region,
    ) -> Result<ModelAccepted, ModelRejected> {
        let plan = match plan_sh8601_async_start(region, pixels.bytes.len()) {
            Ok(plan) => plan,
            Err(failure) => return Err((failure, transport, pixels)),
        };
        if let Err(failure) = write_sh8601_async_windows(&mut transport, plan) {
            return Err((failure, transport, pixels));
        }

        let ram_write = plan.ram_write_command().with_data(&pixels.bytes);
        if let Err(source) = transport.write(ram_write) {
            return Err((sh8601_ram_write_start_failure(source), transport, pixels));
        }

        Ok(ModelAccepted { transport, pixels })
    }

    fn reference_success() -> ModelAccepted {
        let pixels = ModelPixels::patterned(REFERENCE_BYTES, 0x5a);
        model_start(ModelTransport::new(None), pixels, REFERENCE_REGION)
            .ok()
            .expect("reference start must be admitted")
    }

    fn assert_window_envelope(call: &RecordedCall, stage: Sh8601WriteStage, address: u32) {
        assert_eq!(call.stage, stage);
        assert_eq!(call.opcode, 0x02);
        assert_eq!(call.address, address);
        assert_eq!(call.command_mode, Sh8601WireMode::Single);
        assert_eq!(call.address_mode, Sh8601WireMode::Single);
        assert_eq!(call.data_mode, Sh8601WireMode::Single);
        assert_eq!(call.dummy_cycles, 0);
    }

    #[test]
    fn reference_trace_is_literal_single_payload_and_pattern_exact() {
        let accepted = reference_success();
        assert_eq!(accepted.transport.calls.len(), 3);

        let column = &accepted.transport.calls[0];
        assert_window_envelope(column, Sh8601WriteStage::ColumnAddress, 0x00_2a_00);
        assert_eq!(column.data, [0x00, 0x00, 0x01, 0x6f]);

        let page = &accepted.transport.calls[1];
        assert_window_envelope(page, Sh8601WriteStage::PageAddress, 0x00_2b_00);
        assert_eq!(page.data, [0x00, 0x00, 0x00, 0x0f]);

        let pixels = &accepted.transport.calls[2];
        assert_eq!(
            pixels.stage,
            Sh8601WriteStage::Pixel {
                command: Sh8601PixelCommand::RamWriteStart,
                chunk: 0,
                offset: 0,
                len: REFERENCE_BYTES,
            }
        );
        assert_eq!(pixels.opcode, 0x32);
        assert_eq!(pixels.address, 0x00_2c_00);
        assert_eq!(pixels.command_mode, Sh8601WireMode::Single);
        assert_eq!(pixels.address_mode, Sh8601WireMode::Single);
        assert_eq!(pixels.data_mode, Sh8601WireMode::Quad);
        assert_eq!(pixels.dummy_cycles, 0);
        assert_eq!(pixels.data, accepted.pixels.bytes);
    }

    #[test]
    fn every_async_boundary_failure_stops_classifies_and_returns_exact_resources() {
        let expected_calls = model_start(
            ModelTransport::new(None),
            ModelPixels::patterned(REFERENCE_BYTES, 0xa7),
            REFERENCE_REGION,
        )
        .ok()
        .expect("comparison start must be admitted")
        .transport
        .calls;

        for (fail_at, expected_failure) in [
            FailureKind::Io(Sh8601WriteStage::ColumnAddress, InjectedIo { boundary: 0 }),
            FailureKind::Io(Sh8601WriteStage::PageAddress, InjectedIo { boundary: 1 }),
            FailureKind::RamWriteStart(InjectedIo { boundary: 2 }),
        ]
        .into_iter()
        .enumerate()
        {
            let transport = ModelTransport::new(Some(fail_at));
            let transport_ids = transport.identities();
            let pixels = ModelPixels::patterned(REFERENCE_BYTES, 0xa7);
            let pixels_id = pixels.ptr();

            let (failure, transport, pixels) = model_start(transport, pixels, REFERENCE_REGION)
                .err()
                .expect("injected boundary must reject");

            assert_eq!(transport.identities(), transport_ids);
            assert_eq!(pixels.ptr(), pixels_id);
            assert_eq!(transport.calls, expected_calls[..=fail_at]);

            assert_eq!(failure_kind(&failure), expected_failure);
        }
    }

    fn assert_preflight(
        region: Region,
        actual: usize,
        check: impl FnOnce(Sh8601AsyncStartFailure<InjectedIo>),
    ) {
        let transport = ModelTransport::new(None);
        let transport_ids = transport.identities();
        let pixels = ModelPixels::patterned(actual, 0x41);
        let pixels_id = pixels.ptr();
        let (failure, transport, pixels) = model_start(transport, pixels, region)
            .err()
            .expect("preflight case must reject");
        assert!(transport.calls.is_empty(), "preflight performed wire I/O");
        assert_eq!(transport.identities(), transport_ids);
        assert_eq!(pixels.ptr(), pixels_id);
        check(failure);
    }

    #[test]
    #[allow(clippy::too_many_lines)] // One test pins the complete ordered preflight matrix.
    fn async_preflight_precedence_is_exact_including_cap_before_length() {
        assert_preflight(
            Region {
                x: u16::MAX,
                y: u16::MAX,
                width: 0,
                height: 0,
            },
            1,
            |failure| {
                assert_eq!(failure_kind(&failure), FailureKind::EmptyWidth);
            },
        );
        assert_preflight(
            Region {
                x: u16::MAX,
                y: u16::MAX,
                width: 1,
                height: 0,
            },
            1,
            |failure| {
                assert_eq!(failure_kind(&failure), FailureKind::EmptyHeight);
            },
        );
        assert_preflight(
            Region {
                x: u16::MAX,
                y: u16::MAX,
                width: 1,
                height: 1,
            },
            1,
            |failure| {
                assert_eq!(
                    failure_kind(&failure),
                    FailureKind::CoordinateOverflow(Sh8601Axis::X)
                );
            },
        );
        assert_preflight(
            Region {
                x: 0,
                y: u16::MAX,
                width: 1,
                height: 1,
            },
            1,
            |failure| {
                assert_eq!(
                    failure_kind(&failure),
                    FailureKind::CoordinateOverflow(Sh8601Axis::Y)
                );
            },
        );

        for region in [
            Region {
                x: 368,
                y: 0,
                width: 1,
                height: 1,
            },
            Region {
                x: 0,
                y: 448,
                width: 1,
                height: 1,
            },
            Region {
                x: 367,
                y: 0,
                width: 2,
                height: 1,
            },
            Region {
                x: 0,
                y: 447,
                width: 1,
                height: 2,
            },
        ] {
            assert_preflight(region, 2, |failure| {
                assert_eq!(failure_kind(&failure), FailureKind::OutOfBounds(region));
            });
        }

        let above_cap = Region {
            x: 0,
            y: 0,
            width: 368,
            height: 23,
        };
        assert_preflight(above_cap, 0, |failure| {
            assert_eq!(
                failure_kind(&failure),
                FailureKind::AsyncPayloadTooLarge {
                    bytes: 16_928,
                    max: SH8601_DMA_CHUNK_BYTES,
                }
            );
        });

        for actual in [REFERENCE_BYTES - 2, REFERENCE_BYTES + 2] {
            assert_preflight(REFERENCE_REGION, actual, |failure| {
                assert_eq!(
                    failure_kind(&failure),
                    FailureKind::WrongByteLength {
                        expected: 11_776,
                        actual,
                    }
                );
            });
        }
    }

    #[test]
    fn positive_async_boundaries_share_inclusive_big_endian_windows() {
        let cases = [
            (
                Region {
                    x: 0,
                    y: 0,
                    width: 368,
                    height: 1,
                },
                736,
                [0x00, 0x00, 0x01, 0x6f],
                [0x00, 0x00, 0x00, 0x00],
            ),
            (
                REFERENCE_REGION,
                REFERENCE_BYTES,
                [0x00, 0x00, 0x01, 0x6f],
                [0x00, 0x00, 0x00, 0x0f],
            ),
            (
                Region {
                    x: 0x0102,
                    y: 0x0104,
                    width: 2,
                    height: 3,
                },
                12,
                [0x01, 0x02, 0x01, 0x03],
                [0x01, 0x04, 0x01, 0x06],
            ),
            (
                Region {
                    x: 53,
                    y: 422,
                    width: 315,
                    height: 26,
                },
                SH8601_DMA_CHUNK_BYTES,
                [0x00, 0x35, 0x01, 0x6f],
                [0x01, 0xa6, 0x01, 0xbf],
            ),
        ];

        for (region, bytes, columns, pages) in cases {
            let plan = plan_sh8601_async_start::<InjectedIo>(region, bytes)
                .expect("positive boundary must be admitted");
            assert_eq!(plan.bytes(), bytes);
            let mut wire = ModelTransport::new(None);
            write_sh8601_async_windows(&mut wire, plan).expect("window writes succeed");
            assert_eq!(wire.calls.len(), 2);
            assert_eq!(wire.calls[0].data, columns);
            assert_eq!(wire.calls[1].data, pages);
            assert_eq!(
                plan.ram_write_command().stage,
                Sh8601WriteStage::Pixel {
                    command: Sh8601PixelCommand::RamWriteStart,
                    chunk: 0,
                    offset: 0,
                    len: bytes,
                }
            );
        }
    }

    struct Scratch {
        token: Identity,
        capacity: usize,
        logical_len: usize,
    }

    struct BoardParts {
        singleton_tokens: [Identity; 8],
        rx: Scratch,
        tx: Scratch,
    }

    impl BoardParts {
        fn new(rx_capacity: usize, tx_capacity: usize) -> Self {
            Self {
                singleton_tokens: core::array::from_fn(|index| {
                    Identity::new(index.to_le_bytes()[0])
                }),
                rx: Scratch {
                    token: Identity::new(0x80),
                    capacity: rx_capacity,
                    logical_len: 7,
                },
                tx: Scratch {
                    token: Identity::new(0x81),
                    capacity: tx_capacity,
                    logical_len: 1,
                },
            }
        }

        fn identities(&self) -> ([*const u8; 8], *const u8, *const u8) {
            (
                core::array::from_fn(|index| self.singleton_tokens[index].ptr()),
                self.rx.token.ptr(),
                self.tx.token.ptr(),
            )
        }
    }

    fn model_construct(
        mut parts: BoardParts,
        configured: &Cell<bool>,
    ) -> Result<BoardParts, BoardParts> {
        match decide_sh8601_scratch_admission(parts.rx.capacity, parts.tx.capacity) {
            Sh8601ScratchAdmission::Reject => Err(parts),
            Sh8601ScratchAdmission::NormalizeTx { len } => {
                parts.tx.logical_len = len;
                configured.set(true);
                Ok(parts)
            }
        }
    }

    #[test]
    fn scratch_admission_rejects_independently_and_normalizes_only_after_both() {
        for (rx_capacity, tx_capacity) in [
            (SH8601_DMA_CHUNK_BYTES - 1, SH8601_DMA_CHUNK_BYTES),
            (SH8601_DMA_CHUNK_BYTES, SH8601_DMA_CHUNK_BYTES - 1),
        ] {
            let parts = BoardParts::new(rx_capacity, tx_capacity);
            let identities = parts.identities();
            let configured = Cell::new(false);
            let returned = model_construct(parts, &configured)
                .err()
                .expect("one short scratch must reject");
            assert!(!configured.get(), "rejection reached configuration");
            assert_eq!(returned.identities(), identities);
            assert_eq!(returned.tx.logical_len, 1, "rejection normalized TX");
        }

        let parts = BoardParts::new(SH8601_DMA_CHUNK_BYTES, SH8601_DMA_CHUNK_BYTES);
        let identities = parts.identities();
        let configured = Cell::new(false);
        let admitted = model_construct(parts, &configured)
            .ok()
            .expect("exact capacities must be admitted");
        assert!(configured.get());
        assert_eq!(admitted.identities(), identities);
        assert_eq!(admitted.tx.logical_len, SH8601_DMA_CHUNK_BYTES);
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum ProbeEventKind {
        Wake,
        Drop,
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct ProbeEvent {
        id: u8,
        kind: ProbeEventKind,
        in_exclusion: bool,
    }

    struct WakeProbe {
        id: u8,
        in_exclusion: Arc<AtomicBool>,
        events: Arc<Mutex<Vec<ProbeEvent>>>,
    }

    impl WakeProbe {
        fn record(&self, kind: ProbeEventKind) {
            self.events
                .lock()
                .expect("probe event lock")
                .push(ProbeEvent {
                    id: self.id,
                    kind,
                    in_exclusion: self.in_exclusion.load(Ordering::SeqCst),
                });
        }
    }

    impl Wake for WakeProbe {
        fn wake(self: Arc<Self>) {
            self.record(ProbeEventKind::Wake);
        }
    }

    impl Drop for WakeProbe {
        fn drop(&mut self) {
            self.record(ProbeEventKind::Drop);
        }
    }

    fn probe_waker(
        id: u8,
        in_exclusion: &Arc<AtomicBool>,
        events: &Arc<Mutex<Vec<ProbeEvent>>>,
    ) -> Waker {
        Waker::from(Arc::new(WakeProbe {
            id,
            in_exclusion: Arc::clone(in_exclusion),
            events: Arc::clone(events),
        }))
    }

    fn assert_all_probe_actions_left_exclusion(events: &Arc<Mutex<Vec<ProbeEvent>>>) {
        assert!(
            events
                .lock()
                .expect("probe event lock")
                .iter()
                .all(|event| !event.in_exclusion),
            "a Waker callback ran inside modeled exclusion"
        );
    }

    #[test]
    fn completion_register_then_recheck_closes_both_selection_loss_positions() {
        let in_exclusion = Arc::new(AtomicBool::new(false));
        let events = Arc::new(Mutex::new(Vec::new()));

        let mut before = CompletionSlotCore::new();
        drop(before.arm());
        let mut candidate = Some(probe_waker(1, &in_exclusion, &events));
        let checks = Cell::new(0);
        let mut completion_visible = || {
            checks.set(checks.get() + 1);
            true
        };
        in_exclusion.store(true, Ordering::SeqCst);
        let exit = before.register_then_recheck(&mut candidate, &mut completion_visible);
        assert!(exit.ready);
        assert_eq!(checks.get(), 1);
        assert!(
            candidate.is_some(),
            "first observation must precede registration"
        );
        assert!(exit.replaced.is_none());
        assert!(exit.registered.is_none());
        assert!(events.lock().expect("probe event lock").is_empty());
        in_exclusion.store(false, Ordering::SeqCst);
        drop(candidate);

        let mut during = CompletionSlotCore::new();
        drop(during.arm());
        let mut candidate = Some(probe_waker(2, &in_exclusion, &events));
        let checks = Cell::new(0);
        let mut completion_visible = || {
            let this = checks.get();
            checks.set(this + 1);
            this == 1
        };
        in_exclusion.store(true, Ordering::SeqCst);
        let exit = during.register_then_recheck(&mut candidate, &mut completion_visible);
        assert!(exit.ready);
        assert_eq!(checks.get(), 2);
        assert!(
            candidate.is_none(),
            "candidate must be installed before recheck"
        );
        assert!(exit.replaced.is_none());
        assert!(exit.registered.is_some());
        assert_all_probe_actions_left_exclusion(&events);
        in_exclusion.store(false, Ordering::SeqCst);
        drop(exit.registered);
        assert_all_probe_actions_left_exclusion(&events);
        assert_eq!(
            *events.lock().expect("probe event lock"),
            [
                ProbeEvent {
                    id: 1,
                    kind: ProbeEventKind::Drop,
                    in_exclusion: false,
                },
                ProbeEvent {
                    id: 2,
                    kind: ProbeEventKind::Drop,
                    in_exclusion: false,
                },
            ]
        );
    }

    #[test]
    fn completion_replacement_and_equivalent_candidate_leave_exclusion_before_drop() {
        let in_exclusion = Arc::new(AtomicBool::new(false));
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut slot = CompletionSlotCore::new();
        drop(slot.arm());

        let mut first = Some(probe_waker(1, &in_exclusion, &events));
        let mut never_complete = || false;
        in_exclusion.store(true, Ordering::SeqCst);
        let first_exit = slot.register_then_recheck(&mut first, &mut never_complete);
        in_exclusion.store(false, Ordering::SeqCst);
        assert!(!first_exit.ready);
        assert!(first.is_none());
        drop(first_exit.replaced);
        drop(first_exit.registered);

        let canonical = probe_waker(2, &in_exclusion, &events);
        let mut replacement = Some(canonical.clone());
        let mut never_complete = || false;
        in_exclusion.store(true, Ordering::SeqCst);
        let replacement_exit = slot.register_then_recheck(&mut replacement, &mut never_complete);
        assert!(replacement_exit.replaced.is_some());
        assert!(events.lock().expect("probe event lock").is_empty());
        in_exclusion.store(false, Ordering::SeqCst);
        drop(replacement_exit.replaced);
        drop(replacement_exit.registered);

        let mut equivalent_candidate = Some(canonical.clone());
        let mut never_complete = || false;
        in_exclusion.store(true, Ordering::SeqCst);
        let equivalent_exit =
            slot.register_then_recheck(&mut equivalent_candidate, &mut never_complete);
        assert!(equivalent_exit.replaced.is_none());
        assert!(equivalent_candidate.is_some());
        in_exclusion.store(false, Ordering::SeqCst);
        drop(equivalent_candidate);
        drop(equivalent_exit.registered);

        in_exclusion.store(true, Ordering::SeqCst);
        let registered = slot.disarm();
        assert!(registered.is_some());
        assert_all_probe_actions_left_exclusion(&events);
        in_exclusion.store(false, Ordering::SeqCst);
        drop(registered);
        drop(canonical);
        assert_all_probe_actions_left_exclusion(&events);
        assert_eq!(
            *events.lock().expect("probe event lock"),
            [
                ProbeEvent {
                    id: 1,
                    kind: ProbeEventKind::Drop,
                    in_exclusion: false,
                },
                ProbeEvent {
                    id: 2,
                    kind: ProbeEventKind::Drop,
                    in_exclusion: false,
                },
            ]
        );
    }

    #[test]
    fn interrupt_cancel_race_disarm_and_reuse_are_conservative() {
        let in_exclusion = Arc::new(AtomicBool::new(false));
        let events = Arc::new(Mutex::new(Vec::new()));

        let mut interrupt_first = CompletionSlotCore::new();
        drop(interrupt_first.arm());
        let mut registration = Some(probe_waker(7, &in_exclusion, &events));
        let mut never_complete = || false;
        let pending = interrupt_first.register_then_recheck(&mut registration, &mut never_complete);
        assert!(!pending.ready);
        let interrupt = interrupt_first.interrupt(true);
        assert!(interrupt.acknowledge);
        assert!(interrupt.wake.is_some());
        interrupt.wake.expect("registered wake").wake();
        let cancelled = interrupt_first.cancel(false);
        assert_eq!(cancelled.outcome, TransferOutcome::Completed);
        assert!(cancelled.wake.is_none());
        assert!(!interrupt_first.is_active());

        let mut cancel_first = CompletionSlotCore::new();
        drop(cancel_first.arm());
        let mut registration = Some(probe_waker(9, &in_exclusion, &events));
        let mut never_complete = || false;
        let pending = cancel_first.register_then_recheck(&mut registration, &mut never_complete);
        assert!(!pending.ready);
        let cancelled = cancel_first.cancel(false);
        assert_eq!(cancelled.outcome, TransferOutcome::Cancelled);
        cancelled.wake.expect("cancellation wake").wake();
        let late_interrupt = cancel_first.interrupt(true);
        assert!(late_interrupt.acknowledge);
        assert!(late_interrupt.wake.is_none());
        assert!(!cancel_first.is_active());

        drop(cancel_first.arm());
        let no_level = cancel_first.interrupt(false);
        assert!(!no_level.acknowledge);
        assert!(no_level.wake.is_none());
        let completed = cancel_first.cancel(true);
        assert_eq!(completed.outcome, TransferOutcome::Completed);
        assert!(cancel_first.disarm().is_none());

        drop(cancel_first.arm());
        let mut registration = Some(probe_waker(11, &in_exclusion, &events));
        let mut never_complete = || false;
        let pending = cancel_first.register_then_recheck(&mut registration, &mut never_complete);
        assert!(!pending.ready);
        let registered = cancel_first.disarm();
        assert!(registered.is_some());
        drop(registered);
        assert!(!cancel_first.is_active());
        drop(cancel_first.arm());
        let mut next = Some(probe_waker(12, &in_exclusion, &events));
        let checks = Cell::new(0);
        let mut completion_visible = || {
            let count = checks.get();
            checks.set(count + 1);
            count == 1
        };
        let ready = cancel_first.register_then_recheck(&mut next, &mut completion_visible);
        assert!(ready.ready, "rearmed slot must remain reusable");
        assert!(ready.registered.is_some());
        drop(ready.registered);

        assert_all_probe_actions_left_exclusion(&events);
        let events = events.lock().expect("probe event lock");
        for id in [7, 9] {
            assert!(events.iter().any(|event| {
                event.id == id && event.kind == ProbeEventKind::Wake && !event.in_exclusion
            }));
        }
        for id in [7, 9, 11, 12] {
            assert!(events.iter().any(|event| {
                event.id == id && event.kind == ProbeEventKind::Drop && !event.in_exclusion
            }));
        }
    }

    struct LifecycleTransfer {
        transport: Option<ModelTransport>,
        pixels: Option<ModelPixels>,
        completion: Rc<Cell<bool>>,
        slot: CompletionSlotCore,
        outcome: Option<TransferOutcome>,
        drops: Rc<Cell<u32>>,
    }

    impl LifecycleTransfer {
        fn new(accepted: ModelAccepted, completion: Rc<Cell<bool>>, drops: Rc<Cell<u32>>) -> Self {
            let mut slot = CompletionSlotCore::new();
            drop(slot.arm());
            Self {
                transport: Some(accepted.transport),
                pixels: Some(accepted.pixels),
                completion,
                slot,
                outcome: None,
                drops,
            }
        }
    }

    impl OwnedTransfer for LifecycleTransfer {
        type Transport = ModelTransport;
        type Buffer = ModelPixels;

        fn poll_done(&mut self, cx: &mut Context<'_>) -> Poll<()> {
            if self.outcome.is_some() {
                return Poll::Ready(());
            }
            let mut candidate = Some(cx.waker().clone());
            let completion = Rc::clone(&self.completion);
            let mut completion_visible = || completion.get();
            let exit = self
                .slot
                .register_then_recheck(&mut candidate, &mut completion_visible);
            drop(exit.replaced);
            drop(exit.registered);
            drop(candidate);
            if exit.ready {
                self.outcome = Some(TransferOutcome::Completed);
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        }

        fn cancel(&mut self) {
            if self.outcome.is_some() {
                return;
            }
            let exit = self.slot.cancel(self.completion.get());
            self.outcome = Some(exit.outcome);
            if let Some(waker) = exit.wake {
                waker.wake();
            }
        }

        fn recover(mut self) -> Recovered<Self::Transport, Self::Buffer> {
            let outcome = self.outcome.expect("recover requires settlement");
            drop(self.slot.disarm());
            Recovered {
                transport: self.transport.take().expect("live transport"),
                buffer: self.pixels.take().expect("live pixels"),
                outcome,
            }
        }
    }

    impl Drop for LifecycleTransfer {
        fn drop(&mut self) {
            if self.transport.is_none() {
                return;
            }
            if self.outcome.is_none() {
                self.cancel();
            }
            drop(self.slot.disarm());
            self.drops.set(self.drops.get() + 1);
        }
    }

    struct ModelStarter {
        transport: ModelTransport,
        pixels: ModelPixels,
        completion: Rc<Cell<bool>>,
        drops: Rc<Cell<u32>>,
    }

    struct ModelStartError {
        failure: Sh8601AsyncStartFailure<InjectedIo>,
        transport: ModelTransport,
        pixels: ModelPixels,
    }

    impl FlightStarter for ModelStarter {
        type Transfer = LifecycleTransfer;
        type Error = ModelStartError;

        fn start(
            self,
            region: Region,
            _permit: StartPermit<'_>,
        ) -> Result<Self::Transfer, Self::Error> {
            match model_start(self.transport, self.pixels, region) {
                Ok(accepted) => Ok(LifecycleTransfer::new(
                    accepted,
                    self.completion,
                    self.drops,
                )),
                Err((failure, transport, pixels)) => Err(ModelStartError {
                    failure,
                    transport,
                    pixels,
                }),
            }
        }
    }

    fn ready_value<T>(poll: Poll<T>) -> Option<T> {
        match poll {
            Poll::Ready(value) => Some(value),
            Poll::Pending => None,
        }
    }

    #[test]
    fn shaped_starter_rejection_returns_target_spare_and_inner_resources() {
        let plan =
            SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, 16).expect("16-row anchor plan");
        let mut demand = FrameDemand::new(0, plan);
        demand.request();
        let mut sweep = demand
            .begin_sweep(Tick(0), ())
            .expect("requested sweep starts");
        let target = sweep.next_target().expect("first target");
        let region = target.region();
        let transport = ModelTransport::new(None);
        let transport_ids = transport.identities();
        let sent = ModelPixels::patterned(REFERENCE_BYTES - 2, 0x31);
        let sent_id = sent.ptr();
        let spare = ModelPixels::patterned(REFERENCE_BYTES, 0x32);
        let spare_id = spare.ptr();

        let failure = target
            .start_flight(
                spare,
                ModelStarter {
                    transport,
                    pixels: sent,
                    completion: Rc::new(Cell::new(false)),
                    drops: Rc::new(Cell::new(0)),
                },
            )
            .err()
            .expect("wrong-sized sent buffer must reject");
        let (inner, spare, returned_target) = failure.into_parts();
        assert_eq!(
            failure_kind(&inner.failure),
            FailureKind::WrongByteLength {
                expected: 11_776,
                actual: REFERENCE_BYTES - 2,
            }
        );
        assert_eq!(inner.transport.identities(), transport_ids);
        assert_eq!(inner.pixels.ptr(), sent_id);
        assert_eq!(spare.ptr(), spare_id);
        assert_eq!(returned_target.region(), region);
    }

    #[test]
    fn concrete_shaped_flights_return_resources_and_drive_sweep_outcomes() {
        let plan =
            SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, 16).expect("16-row anchor plan");
        let mut demand = FrameDemand::new(0, plan);
        demand.request();
        let mut sweep = demand
            .begin_sweep(Tick(0), ())
            .expect("requested sweep starts");

        let transport = ModelTransport::new(None);
        let transport_ids = transport.identities();
        let sent = ModelPixels::patterned(REFERENCE_BYTES, 0x17);
        let sent_id = sent.ptr();
        let spare = ModelPixels::patterned(REFERENCE_BYTES, 0x29);
        let spare_id = spare.ptr();
        let first_done = Rc::new(Cell::new(false));
        let drops = Rc::new(Cell::new(0));
        let first_target = sweep.next_target().expect("first target");
        let mut first = first_target
            .start_flight(
                spare,
                ModelStarter {
                    transport,
                    pixels: sent,
                    completion: Rc::clone(&first_done),
                    drops: Rc::clone(&drops),
                },
            )
            .ok()
            .expect("reference start accepted");

        let mut cx = Context::from_waker(Waker::noop());
        assert!(ready_value(first.poll_complete(&mut cx)).is_none());
        first_done.set(true);
        let first_settled =
            ready_value(first.poll_complete(&mut cx)).expect("visible completion must settle");
        assert_eq!(first_settled.outcome(), TransferOutcome::Completed);
        let (transport, sent, spare, witness) = first_settled.into_parts();
        assert_eq!(transport.identities(), transport_ids);
        assert_eq!(sent.ptr(), sent_id);
        assert_eq!(spare.ptr(), spare_id);
        assert_eq!(sweep.settle(witness), Ok(TransferOutcome::Completed));

        let second_done = Rc::new(Cell::new(false));
        let second_target = sweep.next_target().expect("second target");
        let mut second = second_target
            .start_flight(
                sent,
                ModelStarter {
                    transport,
                    pixels: spare,
                    completion: second_done,
                    drops: Rc::clone(&drops),
                },
            )
            .ok()
            .expect("second start accepted");
        second.begin_drain();
        second.begin_drain();
        let second_settled =
            ready_value(second.poll_complete(&mut cx)).expect("cancelled transfer must settle");
        assert_eq!(second_settled.outcome(), TransferOutcome::Cancelled);
        let (transport, second_sent, second_spare, witness) = second_settled.into_parts();
        assert_eq!(transport.identities(), transport_ids);
        assert_eq!(second_sent.ptr(), spare_id);
        assert_eq!(second_spare.ptr(), sent_id);
        assert_eq!(sweep.settle(witness), Ok(TransferOutcome::Cancelled));
        assert!(sweep.is_poisoned());
        assert!(sweep.abort().is_ok());
        assert_eq!(drops.get(), 0, "driven recovery is not ordinary drop");

        drop(transport);
        drop(second_sent);
        drop(second_spare);
    }

    #[test]
    fn ordinary_transfer_drop_disarms_without_returning_resources() {
        let accepted = reference_success();
        let drops = Rc::new(Cell::new(0));
        let completion = Rc::new(Cell::new(false));
        let mut transfer = LifecycleTransfer::new(accepted, completion, Rc::clone(&drops));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(transfer.poll_done(&mut cx).is_pending());
        drop(transfer);
        assert_eq!(drops.get(), 1);

        let accepted = reference_success();
        let mut recovered =
            LifecycleTransfer::new(accepted, Rc::new(Cell::new(false)), Rc::clone(&drops));
        recovered.cancel();
        recovered.cancel();
        assert!(recovered.poll_done(&mut cx).is_ready());
        let recovered = recovered.recover();
        assert_eq!(recovered.outcome, TransferOutcome::Cancelled);

        let accepted = reference_success();
        let mut settled =
            LifecycleTransfer::new(accepted, Rc::new(Cell::new(false)), Rc::clone(&drops));
        settled.cancel();
        drop(settled);
        assert_eq!(drops.get(), 2);
    }
}
