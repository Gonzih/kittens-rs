//! Sealed, synchronous SH8601 region writes (SPEC section 6.7).
//!
//! [`StripeTarget::write_region`] is the sole public consumer operation. It
//! binds one target, the exact caller-owned pixel slice, and an admitted
//! writer into [`BlockingSettled`], which returns all resources together with
//! exactly one owning-sweep settlement. The blocking path has no cancellation
//! transition: a returned error is always conservatively `Failed`.
//!
//! The SH8601 transaction engine and its wire seam are crate-private. Host
//! tests record that seam byte-for-byte; the target adapter maps the same
//! values onto the reviewed HAL. Raw HAL access remains a compiling bypass,
//! and calling this synchronous operation in a reactor handler can block every
//! other arm.

#![cfg_attr(
    not(all(feature = "esp32s3-sh8601-blocking", target_arch = "xtensa")),
    allow(dead_code)
)]

use crate::{
    geometry::Region,
    sweep::{StripeSettlement, StripeTarget, StripeUnwritten, StripeWritten},
    transfer::TransferOutcome,
};

/// Maximum pixel payload in one SH8601 DMA transaction.
///
/// This is four times the hardware's 4,095-byte TX-descriptor maximum. The
/// pinned HAL's 4,092-byte default chunking uses five descriptors for this
/// reserve. Its even size never divides one RGB565 pixel across chunks.
pub const SH8601_DMA_CHUNK_BYTES: usize = 16_380;

const SH8601_PANEL_WIDTH: u16 = 368;
const SH8601_PANEL_HEIGHT: u16 = 448;
const SH8601_WRITE_OPCODE: u8 = 0x02;
const SH8601_PIXEL_OPCODE: u8 = 0x32;
const SH8601_CASET_ADDRESS: u32 = 0x00_2a_00;
const SH8601_PASET_ADDRESS: u32 = 0x00_2b_00;
const SH8601_RAMWR_ADDRESS: u32 = 0x00_2c_00;
const SH8601_RAMWRC_ADDRESS: u32 = 0x00_3c_00;

/// Crate-issued authority for one admitted blocking dispatch.
///
/// The constructor and field are private, this value is non-`Clone`, and its
/// lifetime is tied to one [`StripeTarget::write_region`] call. It is public
/// only because an admitted implementation must name it in the trait method.
pub struct BlockingWritePermit<'a> {
    _key: &'a mut (),
}

impl<'a> BlockingWritePermit<'a> {
    const fn new(key: &'a mut ()) -> Self {
        Self { _key: key }
    }
}

/// Structural admission for blocking region writers.
///
/// The module is crate-visible only so the separately target-gated adapter can
/// implement the seal. External crates cannot name or implement it.
pub(crate) mod private {
    /// Crate-private seal for reviewed blocking-region implementations.
    pub trait Sealed {}
}

/// One operation-bound, admitted synchronous region writer.
///
/// Implementations are sealed to reviewed profile-owned adapters. The permit
/// separately prevents safe direct dispatch even for an admitted writer.
pub trait BlockingRegionWrite: private::Sealed + Sized {
    /// A complete preflight or wire failure.
    type Error;

    /// Performs the admitted synchronous write and returns the writer on every
    /// ordinary return.
    ///
    /// This is the visibly exceptional implementation hook. Callers use only
    /// [`StripeTarget::write_region`], whose private permit binds dispatch to a
    /// consumed target.
    fn write_region_admitted(
        self,
        region: Region,
        pixels: &[u8],
        permit: BlockingWritePermit<'_>,
    ) -> (Self, Result<(), Self::Error>);
}

/// The resource-carrying result of one blocking region write.
///
/// Fields are private so safe code cannot replace the result, target, or
/// resources before extracting the one owning-sweep settlement.
#[must_use = "recover the writer, pixels, result, and owning-sweep settlement"]
pub struct BlockingSettled<T, P, E> {
    writer: T,
    pixels: P,
    result: Result<(), E>,
    target: StripeTarget,
}

impl<T, P, E> BlockingSettled<T, P, E> {
    /// The exact consumed target region.
    pub const fn region(&self) -> Region {
        self.target.region()
    }

    /// Classifies the ordinary return as completed or failed.
    ///
    /// This synchronous path has no cancellation transition.
    pub fn outcome(&self) -> TransferOutcome {
        if self.result.is_ok() {
            TransferOutcome::Completed
        } else {
            TransferOutcome::Failed
        }
    }

    /// Returns both resources, the concrete operation result, and exactly one
    /// settlement for the consumed target.
    ///
    /// A successful adapter return yields `Written`; every error, including a
    /// preflight rejection, yields `Unwritten(Failed)`. The cooperative caller
    /// delivers the settlement to its owning sweep.
    pub fn into_parts(self) -> (T, P, Result<(), E>, StripeSettlement) {
        let settlement = if self.result.is_ok() {
            StripeSettlement::Written(StripeWritten {
                demand_id: self.target.demand_id,
                epoch: self.target.epoch,
                region: self.target.region,
            })
        } else {
            StripeSettlement::Unwritten(StripeUnwritten {
                demand_id: self.target.demand_id,
                epoch: self.target.epoch,
                region: self.target.region,
                outcome: TransferOutcome::Failed,
            })
        };
        (self.writer, self.pixels, self.result, settlement)
    }
}

impl StripeTarget {
    /// Writes this target's exact region through one admitted synchronous
    /// writer.
    ///
    /// The operation always returns a private-field [`BlockingSettled`], so an
    /// ordinary success or failure carries the same writer and mutable pixel
    /// slice back together with one settlement for the owning sweep. Dropping
    /// that value instead is the documented resource/proof escape; recover by
    /// dropping the old sweep and using
    /// [`crate::demand::FrameDemand::abandon_active`].
    #[allow(clippy::needless_lifetimes)] // Revision 10 fixes this public spelling.
    pub fn write_region<'pixels, W>(
        self,
        pixels: &'pixels mut [u8],
        writer: W,
    ) -> BlockingSettled<W, &'pixels mut [u8], W::Error>
    where
        W: BlockingRegionWrite,
    {
        let mut permit_key = ();
        let permit = BlockingWritePermit::new(&mut permit_key);
        let (writer, result) = writer.write_region_admitted(self.region, pixels, permit);
        BlockingSettled {
            writer,
            pixels,
            result,
            target: self,
        }
    }
}

/// Coordinate axis whose exclusive end overflowed during preflight.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sh8601Axis {
    /// Horizontal panel coordinate.
    X,
    /// Vertical panel coordinate.
    Y,
}

/// Pixel-memory command selected for one chunk.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sh8601PixelCommand {
    /// First chunk, emitted with `RAMWR`.
    RamWriteStart,
    /// A later chunk, emitted with `RAMWRC`.
    RamWriteContinue,
}

/// Exact transaction stage at which an SH8601 wire error occurred.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sh8601WriteStage {
    /// Column-window (`CASET`) transaction.
    ColumnAddress,
    /// Page-window (`PASET`) transaction.
    PageAddress,
    /// One pixel payload transaction.
    Pixel {
        /// Whether this is the initial `RAMWR` or a continuing `RAMWRC`.
        command: Sh8601PixelCommand,
        /// Zero-based pixel chunk index.
        chunk: usize,
        /// Byte offset into the supplied pixel slice.
        offset: usize,
        /// Byte count in this chunk.
        len: usize,
    },
}

/// Preflight or wire failure from the reviewed SH8601 region transaction.
#[derive(Debug)]
pub enum Sh8601RegionWriteError<E> {
    /// The region width is zero.
    EmptyWidth,
    /// The region height is zero.
    EmptyHeight,
    /// A coordinate's exclusive end overflowed `u16`.
    CoordinateOverflow {
        /// Axis whose exclusive end overflowed.
        axis: Sh8601Axis,
    },
    /// The region lies outside the fixed 368×448 anchor panel.
    OutOfBounds {
        /// Rejected region.
        region: Region,
    },
    /// The supplied slice is not exactly one RGB565 region.
    WrongByteLength {
        /// Required byte count (`width * height * 2`).
        expected: u32,
        /// Supplied byte count.
        actual: usize,
    },
    /// The wire rejected one transaction; no later transaction was attempted.
    Io {
        /// Exact failed transaction stage.
        stage: Sh8601WriteStage,
        /// Concrete wire error.
        source: E,
    },
}

/// Line mode at the crate-private wire boundary.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Sh8601WireMode {
    /// One data line.
    Single,
    /// Four data lines.
    Quad,
}

/// One SH8601 half-duplex command envelope without its borrowed payload.
///
/// Keeping the envelope independent from the payload lets the blocking engine
/// borrow a slice while the async adapter gives ownership of its DMA buffer to
/// esp-hal. Both paths therefore consume the same opcode/address/mode truth.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Sh8601WireCommand {
    /// Semantic stage retained for deterministic traces and error mapping.
    pub stage: Sh8601WriteStage,
    /// Eight-bit command opcode.
    pub opcode: u8,
    /// Twenty-four-bit command address, stored in the low 24 bits.
    pub address: u32,
    /// Command line mode.
    pub command_mode: Sh8601WireMode,
    /// Address line mode.
    pub address_mode: Sh8601WireMode,
    /// Payload line mode.
    pub data_mode: Sh8601WireMode,
    /// Dummy cycles before the payload.
    pub dummy_cycles: u8,
}

impl Sh8601WireCommand {
    /// Attaches a synchronous payload borrow to this shared command envelope.
    pub(crate) const fn with_data(self, data: &[u8]) -> Sh8601WireTransfer<'_> {
        Sh8601WireTransfer {
            stage: self.stage,
            opcode: self.opcode,
            address: self.address,
            command_mode: self.command_mode,
            address_mode: self.address_mode,
            data_mode: self.data_mode,
            dummy_cycles: self.dummy_cycles,
            data,
        }
    }
}

/// One exact half-duplex write at the crate-private wire boundary.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sh8601WireTransfer<'a> {
    /// Semantic stage, retained for byte-exact host traces.
    pub stage: Sh8601WriteStage,
    /// Eight-bit command opcode.
    pub opcode: u8,
    /// Twenty-four-bit command address, stored in the low 24 bits.
    pub address: u32,
    /// Command line mode.
    pub command_mode: Sh8601WireMode,
    /// Address line mode.
    pub address_mode: Sh8601WireMode,
    /// Payload line mode.
    pub data_mode: Sh8601WireMode,
    /// Dummy cycles before the payload.
    pub dummy_cycles: u8,
    /// Synchronously borrowed payload bytes.
    pub data: &'a [u8],
}

/// Crate-private synchronous wire implemented by the target adapter and host
/// recorder.
pub(crate) trait Sh8601Wire {
    /// Concrete HAL or recorder error.
    type Error;

    /// Emits one complete half-duplex transaction and returns only after its
    /// payload borrow is no longer retained.
    fn write(&mut self, transfer: Sh8601WireTransfer<'_>) -> Result<(), Self::Error>;
}

/// Validated SH8601 geometry, byte count, and inclusive window bytes.
///
/// This is the sole source of geometry and CASET/PASET encoding truth for the
/// blocking engine and the profile-owned async adapter.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Sh8601RegionPlan {
    columns: [u8; 4],
    pages: [u8; 4],
    expected_bytes: u32,
}

impl Sh8601RegionPlan {
    /// Exact RGB565 payload byte count for the validated region.
    pub(crate) const fn expected_bytes(self) -> u32 {
        self.expected_bytes
    }

    /// Shared CASET envelope paired with its inclusive big-endian payload.
    pub(crate) const fn column_transfer(&self) -> Sh8601WireTransfer<'_> {
        Sh8601WireCommand {
            stage: Sh8601WriteStage::ColumnAddress,
            opcode: SH8601_WRITE_OPCODE,
            address: SH8601_CASET_ADDRESS,
            command_mode: Sh8601WireMode::Single,
            address_mode: Sh8601WireMode::Single,
            data_mode: Sh8601WireMode::Single,
            dummy_cycles: 0,
        }
        .with_data(&self.columns)
    }

    /// Shared PASET envelope paired with its inclusive big-endian payload.
    pub(crate) const fn page_transfer(&self) -> Sh8601WireTransfer<'_> {
        Sh8601WireCommand {
            stage: Sh8601WriteStage::PageAddress,
            opcode: SH8601_WRITE_OPCODE,
            address: SH8601_PASET_ADDRESS,
            command_mode: Sh8601WireMode::Single,
            address_mode: Sh8601WireMode::Single,
            data_mode: Sh8601WireMode::Single,
            dummy_cycles: 0,
        }
        .with_data(&self.pages)
    }

    /// Shared single-payload RAMWR envelope.
    pub(crate) const fn ram_write_command(self) -> Sh8601WireCommand {
        Sh8601WireCommand {
            stage: Sh8601WriteStage::Pixel {
                command: Sh8601PixelCommand::RamWriteStart,
                chunk: 0,
                offset: 0,
                len: self.expected_bytes as usize,
            },
            opcode: SH8601_PIXEL_OPCODE,
            address: SH8601_RAMWR_ADDRESS,
            command_mode: Sh8601WireMode::Single,
            address_mode: Sh8601WireMode::Single,
            data_mode: Sh8601WireMode::Quad,
            dummy_cycles: 0,
        }
    }
}

fn wire_write<W: Sh8601Wire>(
    wire: &mut W,
    transfer: Sh8601WireTransfer<'_>,
) -> Result<(), Sh8601RegionWriteError<W::Error>> {
    let stage = transfer.stage;
    wire.write(transfer)
        .map_err(|source| Sh8601RegionWriteError::Io { stage, source })
}

pub(crate) fn plan_sh8601_region<E>(
    region: Region,
) -> Result<Sh8601RegionPlan, Sh8601RegionWriteError<E>> {
    if region.width == 0 {
        return Err(Sh8601RegionWriteError::EmptyWidth);
    }
    if region.height == 0 {
        return Err(Sh8601RegionWriteError::EmptyHeight);
    }

    let Some(x_end) = region.x.checked_add(region.width) else {
        return Err(Sh8601RegionWriteError::CoordinateOverflow {
            axis: Sh8601Axis::X,
        });
    };
    let Some(y_end) = region.y.checked_add(region.height) else {
        return Err(Sh8601RegionWriteError::CoordinateOverflow {
            axis: Sh8601Axis::Y,
        });
    };

    if region.x >= SH8601_PANEL_WIDTH
        || region.y >= SH8601_PANEL_HEIGHT
        || x_end > SH8601_PANEL_WIDTH
        || y_end > SH8601_PANEL_HEIGHT
    {
        return Err(Sh8601RegionWriteError::OutOfBounds { region });
    }

    let expected_bytes = u32::from(region.width) * u32::from(region.height) * 2;
    Ok(Sh8601RegionPlan {
        columns: inclusive_window(region.x, x_end),
        pages: inclusive_window(region.y, y_end),
        expected_bytes,
    })
}

/// Applies the blocking path's exact-length check after shared geometry.
pub(crate) fn validate_sh8601_exact_len<E>(
    plan: Sh8601RegionPlan,
    actual: usize,
) -> Result<(), Sh8601RegionWriteError<E>> {
    let actual_u64 = u64::try_from(actual).unwrap_or(u64::MAX);
    if u64::from(plan.expected_bytes) != actual_u64 {
        return Err(Sh8601RegionWriteError::WrongByteLength {
            expected: plan.expected_bytes,
            actual,
        });
    }
    Ok(())
}

fn inclusive_window(start: u16, exclusive_end: u16) -> [u8; 4] {
    let start = start.to_be_bytes();
    let inclusive_end = (exclusive_end - 1).to_be_bytes();
    [start[0], start[1], inclusive_end[0], inclusive_end[1]]
}

/// Runs the single reviewed SH8601 region transaction over a crate-private
/// wire implementation.
///
/// Every preflight check completes before the first wire call. On an I/O
/// failure the engine returns immediately, preserving the failed stage and
/// attempting no later command.
pub(crate) fn write_sh8601_region<W: Sh8601Wire>(
    wire: &mut W,
    region: Region,
    pixels: &[u8],
) -> Result<(), Sh8601RegionWriteError<W::Error>> {
    let plan = plan_sh8601_region::<W::Error>(region)?;
    validate_sh8601_exact_len::<W::Error>(plan, pixels.len())?;

    wire_write(wire, plan.column_transfer())?;

    wire_write(wire, plan.page_transfer())?;

    for (chunk, payload) in pixels.chunks(SH8601_DMA_CHUNK_BYTES).enumerate() {
        let offset = chunk * SH8601_DMA_CHUNK_BYTES;
        let command = if chunk == 0 {
            Sh8601PixelCommand::RamWriteStart
        } else {
            Sh8601PixelCommand::RamWriteContinue
        };
        let address = if chunk == 0 {
            SH8601_RAMWR_ADDRESS
        } else {
            SH8601_RAMWRC_ADDRESS
        };
        wire_write(
            wire,
            Sh8601WireCommand {
                stage: Sh8601WriteStage::Pixel {
                    command,
                    chunk,
                    offset,
                    len: payload.len(),
                },
                opcode: SH8601_PIXEL_OPCODE,
                address,
                command_mode: Sh8601WireMode::Single,
                address_mode: Sh8601WireMode::Single,
                data_mode: Sh8601WireMode::Quad,
                dummy_cycles: 0,
            }
            .with_data(payload),
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::{boxed::Box, vec, vec::Vec};

    use crate::{
        demand::{FrameDemand, Tick},
        geometry::{FrameEpoch, PanelGeometry},
        sweep::{StripeSettlement, Sweep, SweepPlan},
    };

    use super::*;

    const REFERENCE_REGION: Region = Region {
        x: 0,
        y: 0,
        width: 368,
        height: 112,
    };
    const REFERENCE_BYTES: usize = 82_432;
    type WireEnvelope = (u8, u32, Sh8601WireMode, Sh8601WireMode, Sh8601WireMode, u8);
    const REFERENCE_ENVELOPES: [WireEnvelope; 8] = [
        (
            0x02,
            0x00_2a_00,
            Sh8601WireMode::Single,
            Sh8601WireMode::Single,
            Sh8601WireMode::Single,
            0,
        ),
        (
            0x02,
            0x00_2b_00,
            Sh8601WireMode::Single,
            Sh8601WireMode::Single,
            Sh8601WireMode::Single,
            0,
        ),
        (
            0x32,
            0x00_2c_00,
            Sh8601WireMode::Single,
            Sh8601WireMode::Single,
            Sh8601WireMode::Quad,
            0,
        ),
        (
            0x32,
            0x00_3c_00,
            Sh8601WireMode::Single,
            Sh8601WireMode::Single,
            Sh8601WireMode::Quad,
            0,
        ),
        (
            0x32,
            0x00_3c_00,
            Sh8601WireMode::Single,
            Sh8601WireMode::Single,
            Sh8601WireMode::Quad,
            0,
        ),
        (
            0x32,
            0x00_3c_00,
            Sh8601WireMode::Single,
            Sh8601WireMode::Single,
            Sh8601WireMode::Quad,
            0,
        ),
        (
            0x32,
            0x00_3c_00,
            Sh8601WireMode::Single,
            Sh8601WireMode::Single,
            Sh8601WireMode::Quad,
            0,
        ),
        (
            0x32,
            0x00_3c_00,
            Sh8601WireMode::Single,
            Sh8601WireMode::Single,
            Sh8601WireMode::Quad,
            0,
        ),
    ];

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
        fn from_transfer(transfer: Sh8601WireTransfer<'_>) -> Self {
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

        fn window(stage: Sh8601WriteStage, address: u32, data: &[u8]) -> Self {
            Self {
                stage,
                opcode: SH8601_WRITE_OPCODE,
                address,
                command_mode: Sh8601WireMode::Single,
                address_mode: Sh8601WireMode::Single,
                data_mode: Sh8601WireMode::Single,
                dummy_cycles: 0,
                data: data.to_vec(),
            }
        }

        fn pixel(command: Sh8601PixelCommand, chunk: usize, offset: usize, data: &[u8]) -> Self {
            Self {
                stage: Sh8601WriteStage::Pixel {
                    command,
                    chunk,
                    offset,
                    len: data.len(),
                },
                opcode: SH8601_PIXEL_OPCODE,
                address: if command == Sh8601PixelCommand::RamWriteStart {
                    SH8601_RAMWR_ADDRESS
                } else {
                    SH8601_RAMWRC_ADDRESS
                },
                command_mode: Sh8601WireMode::Single,
                address_mode: Sh8601WireMode::Single,
                data_mode: Sh8601WireMode::Quad,
                dummy_cycles: 0,
                data: data.to_vec(),
            }
        }
    }

    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    struct InjectedIo {
        boundary: usize,
    }

    #[derive(Debug)]
    struct RecordingWire {
        calls: Vec<RecordedCall>,
        fail_at: Option<usize>,
    }

    impl Sh8601Wire for RecordingWire {
        type Error = InjectedIo;

        fn write(&mut self, transfer: Sh8601WireTransfer<'_>) -> Result<(), Self::Error> {
            let boundary = self.calls.len();
            self.calls.push(RecordedCall::from_transfer(transfer));
            if self.fail_at == Some(boundary) {
                Err(InjectedIo { boundary })
            } else {
                Ok(())
            }
        }
    }

    #[derive(Debug)]
    struct TestWriter {
        identity: Box<u8>,
        wire: RecordingWire,
    }

    impl TestWriter {
        fn new(fail_at: Option<usize>) -> Self {
            Self {
                identity: Box::new(0xa5),
                wire: RecordingWire {
                    calls: Vec::new(),
                    fail_at,
                },
            }
        }

        fn identity_ptr(&self) -> *const u8 {
            core::ptr::from_ref(self.identity.as_ref())
        }
    }

    impl private::Sealed for TestWriter {}

    impl BlockingRegionWrite for TestWriter {
        type Error = Sh8601RegionWriteError<InjectedIo>;

        fn write_region_admitted(
            mut self,
            region: Region,
            pixels: &[u8],
            _permit: BlockingWritePermit<'_>,
        ) -> (Self, Result<(), Self::Error>) {
            let result = write_sh8601_region(&mut self.wire, region, pixels);
            (self, result)
        }
    }

    fn direct_target(region: Region) -> StripeTarget {
        StripeTarget {
            demand_id: 73,
            epoch: FrameEpoch(19),
            region,
        }
    }

    fn reference_sweep() -> (FrameDemand, Sweep<()>) {
        let plan = SweepPlan::for_panel(PanelGeometry::WAVESHARE_18_V1, 112)
            .expect("anchor reference plan");
        let mut demand = FrameDemand::new(0, plan);
        demand.request();
        let sweep = demand
            .begin_sweep(Tick(0), ())
            .expect("requested reference sweep");
        (demand, sweep)
    }

    fn expected_calls(region: Region, pixels: &[u8]) -> Vec<RecordedCall> {
        let x_end = region.x + region.width - 1;
        let y_end = region.y + region.height - 1;
        let x_start = region.x.to_be_bytes();
        let x_end = x_end.to_be_bytes();
        let y_start = region.y.to_be_bytes();
        let y_end = y_end.to_be_bytes();
        let mut calls = vec![
            RecordedCall::window(
                Sh8601WriteStage::ColumnAddress,
                SH8601_CASET_ADDRESS,
                &[x_start[0], x_start[1], x_end[0], x_end[1]],
            ),
            RecordedCall::window(
                Sh8601WriteStage::PageAddress,
                SH8601_PASET_ADDRESS,
                &[y_start[0], y_start[1], y_end[0], y_end[1]],
            ),
        ];
        for (chunk, payload) in pixels.chunks(SH8601_DMA_CHUNK_BYTES).enumerate() {
            calls.push(RecordedCall::pixel(
                if chunk == 0 {
                    Sh8601PixelCommand::RamWriteStart
                } else {
                    Sh8601PixelCommand::RamWriteContinue
                },
                chunk,
                chunk * SH8601_DMA_CHUNK_BYTES,
                payload,
            ));
        }
        calls
    }

    fn patterned_reference_pixels(seed: u32) -> Vec<u8> {
        (0..REFERENCE_BYTES)
            .map(|index| {
                let index = u32::try_from(index).expect("reference length fits u32");
                let mixed = index
                    .wrapping_mul(0x045d_9f3b)
                    .rotate_left(index & 31)
                    .wrapping_add(seed);
                let [a, b, c, d] = mixed.to_le_bytes();
                a ^ b ^ c ^ d
            })
            .collect()
    }

    fn assert_same_resources(
        writer: &TestWriter,
        writer_ptr: *const u8,
        pixels: &mut [u8],
        pixels_ptr: *mut u8,
        pixels_len: usize,
    ) {
        assert!(core::ptr::eq(writer.identity_ptr(), writer_ptr));
        assert!(core::ptr::eq(pixels.as_ptr(), pixels_ptr.cast_const()));
        assert_eq!(pixels.len(), pixels_len);
    }

    #[test]
    fn reference_trace_returns_exact_resources_and_advances_owning_sweep() {
        let (demand, mut sweep) = reference_sweep();
        let target = sweep.next_target().expect("first reference target");
        assert_eq!(target.region(), REFERENCE_REGION);

        let mut pixels = patterned_reference_pixels(0x5a17_c3e1);
        let pixels_ptr = pixels.as_mut_ptr();
        let pixels_len = pixels.len();
        let writer = TestWriter::new(None);
        let writer_ptr = writer.identity_ptr();
        let settled = target.write_region(pixels.as_mut_slice(), writer);

        assert_eq!(settled.region(), REFERENCE_REGION);
        assert_eq!(settled.outcome(), TransferOutcome::Completed);
        let (writer, returned_pixels, result, settlement) = settled.into_parts();
        assert!(result.is_ok());
        assert_same_resources(&writer, writer_ptr, returned_pixels, pixels_ptr, pixels_len);

        let expected = expected_calls(REFERENCE_REGION, returned_pixels);
        assert_eq!(writer.wire.calls, expected);
        assert_eq!(writer.wire.calls.len(), 8);
        assert_eq!(writer.wire.calls[0].data, [0x00, 0x00, 0x01, 0x6f]);
        assert_eq!(writer.wire.calls[1].data, [0x00, 0x00, 0x00, 0x6f]);
        let literal_envelopes: Vec<_> = writer
            .wire
            .calls
            .iter()
            .map(|call| {
                (
                    call.opcode,
                    call.address,
                    call.command_mode,
                    call.address_mode,
                    call.data_mode,
                    call.dummy_cycles,
                )
            })
            .collect();
        assert_eq!(literal_envelopes.as_slice(), REFERENCE_ENVELOPES.as_slice());
        let pixel_shapes: Vec<_> = writer.wire.calls[2..]
            .iter()
            .map(|call| call.stage)
            .collect();
        assert_eq!(
            pixel_shapes,
            vec![
                Sh8601WriteStage::Pixel {
                    command: Sh8601PixelCommand::RamWriteStart,
                    chunk: 0,
                    offset: 0,
                    len: 16_380,
                },
                Sh8601WriteStage::Pixel {
                    command: Sh8601PixelCommand::RamWriteContinue,
                    chunk: 1,
                    offset: 16_380,
                    len: 16_380,
                },
                Sh8601WriteStage::Pixel {
                    command: Sh8601PixelCommand::RamWriteContinue,
                    chunk: 2,
                    offset: 32_760,
                    len: 16_380,
                },
                Sh8601WriteStage::Pixel {
                    command: Sh8601PixelCommand::RamWriteContinue,
                    chunk: 3,
                    offset: 49_140,
                    len: 16_380,
                },
                Sh8601WriteStage::Pixel {
                    command: Sh8601PixelCommand::RamWriteContinue,
                    chunk: 4,
                    offset: 65_520,
                    len: 16_380,
                },
                Sh8601WriteStage::Pixel {
                    command: Sh8601PixelCommand::RamWriteContinue,
                    chunk: 5,
                    offset: 81_900,
                    len: 532,
                },
            ]
        );

        assert_eq!(settlement.outcome(), TransferOutcome::Completed);
        assert_eq!(sweep.settle(settlement), Ok(TransferOutcome::Completed));
        assert!(!sweep.is_poisoned());
        assert_eq!(
            sweep.next_region(),
            Some(Region {
                x: 0,
                y: 112,
                width: 368,
                height: 112,
            })
        );
        assert_eq!(demand.sweeping(), Some(sweep.epoch()));
    }

    #[test]
    fn every_reference_boundary_failure_stops_and_poisons_the_owning_sweep() {
        for failure_boundary in 0..8 {
            let (mut demand, mut sweep) = reference_sweep();
            let target = sweep.next_target().expect("first reference target");
            let boundary_seed =
                u32::try_from(failure_boundary).expect("reference boundary fits u32");
            let mut pixels = patterned_reference_pixels(0x3c91_72ab ^ boundary_seed);
            let pixels_ptr = pixels.as_mut_ptr();
            let pixels_len = pixels.len();
            let expected = expected_calls(REFERENCE_REGION, pixels.as_slice());
            let writer = TestWriter::new(Some(failure_boundary));
            let writer_ptr = writer.identity_ptr();

            let settled = target.write_region(pixels.as_mut_slice(), writer);
            assert_eq!(settled.region(), REFERENCE_REGION);
            assert_eq!(settled.outcome(), TransferOutcome::Failed);
            let (writer, returned_pixels, result, settlement) = settled.into_parts();
            assert_same_resources(&writer, writer_ptr, returned_pixels, pixels_ptr, pixels_len);

            let error = result.expect_err("injected boundary must fail");
            assert!(
                matches!(
                    error,
                    Sh8601RegionWriteError::Io { stage, source }
                        if stage == expected[failure_boundary].stage
                            && source
                                == (InjectedIo {
                                    boundary: failure_boundary,
                                })
                ),
                "boundary {failure_boundary} must report its exact I/O stage and source"
            );
            assert_eq!(
                &writer.wire.calls[..failure_boundary],
                &expected[..failure_boundary],
                "exact successful prefix before boundary {failure_boundary}"
            );
            assert_eq!(
                writer.wire.calls[failure_boundary], expected[failure_boundary],
                "exact attempted call at boundary {failure_boundary}"
            );
            assert_eq!(
                writer.wire.calls.len(),
                failure_boundary + 1,
                "no call follows failed boundary {failure_boundary}"
            );

            assert_eq!(settlement.outcome(), TransferOutcome::Failed);
            assert_eq!(sweep.settle(settlement), Ok(TransferOutcome::Failed));
            assert!(sweep.is_poisoned());
            assert!(sweep.next_target().is_none());
            let (aborted, ()) = sweep.abort().expect("poisoned sweep can abort");
            demand
                .finish_failed(aborted, Tick(0))
                .expect("owning demand accepts abort");
            assert!(demand.is_dirty());
            assert!(demand.full_repaint_required());
            assert_eq!(demand.sweeping(), None);
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum ExpectedPreflight {
        EmptyWidth,
        EmptyHeight,
        Overflow(Sh8601Axis),
        OutOfBounds,
        WrongByteLength { expected: u32, actual: usize },
    }

    fn assert_preflight_error(
        error: &Sh8601RegionWriteError<InjectedIo>,
        expected: ExpectedPreflight,
        region: Region,
    ) {
        match expected {
            ExpectedPreflight::EmptyWidth => {
                assert!(matches!(error, Sh8601RegionWriteError::EmptyWidth));
            }
            ExpectedPreflight::EmptyHeight => {
                assert!(matches!(error, Sh8601RegionWriteError::EmptyHeight));
            }
            ExpectedPreflight::Overflow(expected_axis) => {
                assert!(matches!(
                    error,
                    Sh8601RegionWriteError::CoordinateOverflow { axis }
                        if *axis == expected_axis
                ));
            }
            ExpectedPreflight::OutOfBounds => {
                assert!(matches!(
                    error,
                    Sh8601RegionWriteError::OutOfBounds { region: actual }
                        if *actual == region
                ));
            }
            ExpectedPreflight::WrongByteLength { expected, actual } => {
                assert!(matches!(
                    error,
                    Sh8601RegionWriteError::WrongByteLength {
                        expected: actual_expected,
                        actual: actual_len,
                    } if *actual_expected == expected && *actual_len == actual
                ));
            }
        }
    }

    #[test]
    #[allow(clippy::too_many_lines)] // Keep the complete precedence table in one oracle.
    fn preflight_precedence_is_exact_and_returns_resources_without_io() {
        let cases = [
            (
                Region {
                    x: u16::MAX,
                    y: u16::MAX,
                    width: 0,
                    height: 0,
                },
                0,
                ExpectedPreflight::EmptyWidth,
            ),
            (
                Region {
                    x: u16::MAX,
                    y: u16::MAX,
                    width: 1,
                    height: 0,
                },
                0,
                ExpectedPreflight::EmptyHeight,
            ),
            (
                Region {
                    x: u16::MAX,
                    y: u16::MAX,
                    width: 1,
                    height: 1,
                },
                0,
                ExpectedPreflight::Overflow(Sh8601Axis::X),
            ),
            (
                Region {
                    x: 0,
                    y: u16::MAX,
                    width: 1,
                    height: 1,
                },
                0,
                ExpectedPreflight::Overflow(Sh8601Axis::Y),
            ),
            (
                Region {
                    x: 368,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                1,
                ExpectedPreflight::OutOfBounds,
            ),
            (
                Region {
                    x: 0,
                    y: 448,
                    width: 1,
                    height: 1,
                },
                1,
                ExpectedPreflight::OutOfBounds,
            ),
            (
                Region {
                    x: 367,
                    y: 0,
                    width: 2,
                    height: 1,
                },
                1,
                ExpectedPreflight::OutOfBounds,
            ),
            (
                Region {
                    x: 0,
                    y: 447,
                    width: 1,
                    height: 2,
                },
                1,
                ExpectedPreflight::OutOfBounds,
            ),
            (
                Region {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                7,
                ExpectedPreflight::WrongByteLength {
                    expected: 8,
                    actual: 7,
                },
            ),
            (
                Region {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 2,
                },
                9,
                ExpectedPreflight::WrongByteLength {
                    expected: 8,
                    actual: 9,
                },
            ),
        ];

        for (region, actual_len, expected_error) in cases {
            let mut pixels = vec![0x77; actual_len];
            let pixels_ptr = pixels.as_mut_ptr();
            let writer = TestWriter::new(None);
            let writer_ptr = writer.identity_ptr();
            let settled = direct_target(region).write_region(pixels.as_mut_slice(), writer);
            assert_eq!(settled.region(), region);
            assert_eq!(settled.outcome(), TransferOutcome::Failed);
            let (writer, returned_pixels, result, settlement) = settled.into_parts();
            assert_same_resources(&writer, writer_ptr, returned_pixels, pixels_ptr, actual_len);
            assert!(writer.wire.calls.is_empty(), "preflight performs no I/O");
            assert_preflight_error(
                &result.expect_err("preflight case must reject"),
                expected_error,
                region,
            );
            assert_eq!(settlement.outcome(), TransferOutcome::Failed);
            assert_eq!(settlement.region(), region);
            assert_eq!(settlement.epoch(), FrameEpoch(19));
        }
    }

    #[test]
    fn valid_panel_boundaries_and_nonzero_coordinates_encode_big_endian() {
        let cases = [
            (
                "first row inclusive end is zero",
                Region {
                    x: 7,
                    y: 0,
                    width: 3,
                    height: 1,
                },
                [0x00, 0x07, 0x00, 0x09],
                [0x00, 0x00, 0x00, 0x00],
            ),
            (
                "origin one by one",
                Region {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
                [0x00, 0x00, 0x00, 0x00],
                [0x00, 0x00, 0x00, 0x00],
            ),
            (
                "exact right and bottom endpoint",
                Region {
                    x: 367,
                    y: 447,
                    width: 1,
                    height: 1,
                },
                [0x01, 0x6f, 0x01, 0x6f],
                [0x01, 0xbf, 0x01, 0xbf],
            ),
            (
                "nonzero multi-byte origin",
                Region {
                    x: 0x0123,
                    y: 0x0102,
                    width: 2,
                    height: 3,
                },
                [0x01, 0x23, 0x01, 0x24],
                [0x01, 0x02, 0x01, 0x04],
            ),
        ];

        for (name, region, columns, pages) in cases {
            let len = usize::from(region.width) * usize::from(region.height) * 2;
            let mut pixels = vec![0xc3; len];
            let settled =
                direct_target(region).write_region(pixels.as_mut_slice(), TestWriter::new(None));
            assert_eq!(settled.outcome(), TransferOutcome::Completed, "{name}");
            let (writer, returned_pixels, result, settlement) = settled.into_parts();
            assert!(result.is_ok(), "{name}");
            assert_eq!(writer.wire.calls, expected_calls(region, returned_pixels));
            assert_eq!(writer.wire.calls[0].data, columns, "{name}");
            assert_eq!(writer.wire.calls[1].data, pages, "{name}");
            assert_eq!(writer.wire.calls[2].address, SH8601_RAMWR_ADDRESS);
            assert!(matches!(settlement, StripeSettlement::Written(_)), "{name}");
        }
    }
}
