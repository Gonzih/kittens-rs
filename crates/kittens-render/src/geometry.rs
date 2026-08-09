//! Panel geometry and frame identity (SPEC section 6, normative).

/// A rectangular panel region in global panel coordinates.
///
/// Global, never stripe-local: a stripe target that reported a stripe-local
/// bounding box would change layout semantics (SPEC 6.1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Region {
    /// Left edge, panel coordinates.
    pub x: u16,
    /// Top edge, panel coordinates.
    pub y: u16,
    /// Width in pixels.
    pub width: u16,
    /// Height in pixels.
    pub height: u16,
}

/// Identity of one logically immutable scene snapshot.
///
/// Monotonic within one demand machine's documented 2^64-sweep operating
/// horizon; minted only by the frame-demand policy, never by transports.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct FrameEpoch(pub(crate) u64);

impl FrameEpoch {
    /// Returns the raw epoch number.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// An admitted panel geometry. Binds "the full panel" to reviewed display
/// dimensions so a sweep plan cannot quietly cover a caller-invented 1×1
/// "panel" (exit-review round 2, finding 5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PanelGeometry {
    pub(crate) panel: Region,
}

impl PanelGeometry {
    /// The anchor board: Waveshare ESP32-S3 1.8" AMOLED V1, 368×448.
    pub const WAVESHARE_18_V1: Self = Self {
        panel: Region {
            x: 0,
            y: 0,
            width: 368,
            height: 448,
        },
    };

    /// An arbitrary panel — the documented compiling escape for hosts,
    /// tests, and boards not yet admitted. The name is deliberately loud:
    /// nothing validates that this matches physical hardware.
    pub const fn custom_unvalidated_panel(panel: Region) -> Self {
        Self { panel }
    }

    /// The full-panel region.
    pub const fn panel(&self) -> Region {
        self.panel
    }
}
