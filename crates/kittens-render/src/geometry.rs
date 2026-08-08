//! Panel geometry and frame identity (SPEC section 6 candidates).

/// A rectangular panel region in global panel coordinates.
///
/// Global, never stripe-local: a stripe target that reported a stripe-local
/// bounding box would change layout semantics (SPEC 6.4 rule 3).
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

/// Identity of one immutable scene snapshot.
///
/// Monotonic; minted only by the frame-demand policy, never by transports.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct FrameEpoch(pub(crate) u64);

impl FrameEpoch {
    /// Returns the raw epoch number.
    pub const fn get(self) -> u64 {
        self.0
    }
}
