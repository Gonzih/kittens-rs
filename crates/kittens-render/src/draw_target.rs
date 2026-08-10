//! Optional `embedded-graphics` integration for full-scene stripe replay.
//!
//! [`Rgb565StripeDrawTarget`] borrows one caller-owned byte buffer for the
//! owning [`Sweep`]'s outstanding stripe. Drawing keeps
//! global panel coordinates and full-panel [`Dimensions`] while clipping
//! writes into that stripe. This is the critical distinction between a
//! memory window and a layout viewport: reporting stripe-local bounds would
//! recenter or otherwise relayout the scene on every stripe.
//!
//! The target does not retain spatial history and does not clear scratch
//! storage automatically. Callers reconstruct the background and complete
//! ordered scene from [`Sweep::snapshot`] for
//! every stripe. Host tests pin byte-exact RGB565 reconstruction; the future
//! physical transport's byte/channel interpretation and panel color fidelity
//! remain exact-adapter and board-HIL questions.

use core::convert::Infallible;

use ::embedded_graphics::{
    Pixel,
    draw_target::DrawTarget,
    geometry::{Dimensions, Point, Size},
    pixelcolor::{IntoStorage, Rgb565},
    primitives::Rectangle,
};

use crate::{
    geometry::Region,
    sweep::{StripeTarget, Sweep},
};

/// Construction failure for [`Rgb565StripeDrawTarget`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StripeDrawTargetError {
    /// The target is not the supplied sweep's currently outstanding stripe.
    TargetMismatch,
    /// The exact `width * height * 2` byte count cannot fit in `usize`.
    BufferSizeOverflow,
    /// The caller's byte slice is not exactly one RGB565 stripe long.
    WrongBufferLength {
        /// Required byte count for the outstanding stripe.
        expected: usize,
        /// Byte count supplied by the caller.
        actual: usize,
    },
}

/// A global-coordinate RGB565 draw target backed by exactly one sweep stripe.
///
/// Pixels are stored row-major as two bytes per pixel, most-significant byte
/// first, matching the pinned anchor driver's host framebuffer encoding.
/// Drawing outside the outstanding stripe is clipped. [`Dimensions`] reports
/// the owning sweep's complete panel, not the stripe, so target-derived
/// centering and layout remain identical across the sweep.
pub struct Rgb565StripeDrawTarget<'a> {
    panel: Region,
    stripe: Region,
    bytes: &'a mut [u8],
}

fn validate_rgb565_byte_len(
    width: usize,
    height: usize,
    actual: usize,
) -> Result<(), StripeDrawTargetError> {
    let expected = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(2))
        .ok_or(StripeDrawTargetError::BufferSizeOverflow)?;
    if actual != expected {
        return Err(StripeDrawTargetError::WrongBufferLength { expected, actual });
    }
    Ok(())
}

impl<'a> Rgb565StripeDrawTarget<'a> {
    /// Binds an exact caller buffer to the supplied sweep's outstanding target.
    ///
    /// The returned target borrows only `bytes`; it copies the private admitted
    /// panel and stripe regions. After the draw target is dropped, the same
    /// [`StripeTarget`] can be consumed together with the rendered buffer by
    /// either [`StripeTarget::start_flight`] or
    /// [`StripeTarget::write_region`].
    ///
    /// # Errors
    ///
    /// - [`StripeDrawTargetError::TargetMismatch`] if `target` is foreign,
    ///   stale, or not `sweep`'s current outstanding target;
    /// - [`StripeDrawTargetError::BufferSizeOverflow`] if the exact RGB565
    ///   byte count is not representable on this target;
    /// - [`StripeDrawTargetError::WrongBufferLength`] unless `bytes` contains
    ///   exactly two bytes for every pixel in the stripe.
    pub fn new<S>(
        sweep: &Sweep<S>,
        target: &StripeTarget,
        bytes: &'a mut [u8],
    ) -> Result<Self, StripeDrawTargetError> {
        let (panel, stripe) = sweep
            .draw_target_regions(target)
            .ok_or(StripeDrawTargetError::TargetMismatch)?;
        validate_rgb565_byte_len(
            usize::from(stripe.width),
            usize::from(stripe.height),
            bytes.len(),
        )
        .map(|()| Self {
            panel,
            stripe,
            bytes,
        })
    }
}

impl Dimensions for Rgb565StripeDrawTarget<'_> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(
            Point::new(i32::from(self.panel.x), i32::from(self.panel.y)),
            Size::new(u32::from(self.panel.width), u32::from(self.panel.height)),
        )
    }
}

impl DrawTarget for Rgb565StripeDrawTarget<'_> {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        let left = i32::from(self.stripe.x);
        let top = i32::from(self.stripe.y);
        let right = left + i32::from(self.stripe.width);
        let bottom = top + i32::from(self.stripe.height);
        let row_bytes = usize::from(self.stripe.width) * 2;

        for Pixel(point, color) in pixels {
            if point.x < left || point.x >= right || point.y < top || point.y >= bottom {
                continue;
            }

            // Constructor admission proved `width * height * 2` fits `usize`
            // and equals `bytes.len()`. Clipping above proves each nonnegative
            // local coordinate is strictly inside its `u16` stripe dimension,
            // so conversion succeeds, `byte <= bytes.len() - 2`, and the
            // two-byte slice is in bounds. The former defensive fallbacks were
            // therefore unreachable for every publicly constructible target.
            let local_x = usize::try_from(point.x - left).expect("clipped x is nonnegative");
            let local_y = usize::try_from(point.y - top).expect("clipped y is nonnegative");
            let byte = local_y * row_bytes + local_x * 2;
            let end = byte + 2;
            self.bytes[byte..end].copy_from_slice(&color.into_storage().to_be_bytes());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{StripeDrawTargetError, validate_rgb565_byte_len};

    #[test]
    fn rgb565_byte_length_is_checked_at_usize_exhaustion_edges() {
        // SPEC 6.6: exact `width * height * 2` arithmetic must reject, never
        // wrap, at either multiplication boundary.
        assert_eq!(validate_rgb565_byte_len(3, 2, 12), Ok(()));
        assert_eq!(
            validate_rgb565_byte_len(3, 2, 11),
            Err(StripeDrawTargetError::WrongBufferLength {
                expected: 12,
                actual: 11,
            })
        );
        assert_eq!(
            validate_rgb565_byte_len(usize::MAX, 2, 0),
            Err(StripeDrawTargetError::BufferSizeOverflow)
        );
        assert_eq!(
            validate_rgb565_byte_len(usize::MAX, 1, 0),
            Err(StripeDrawTargetError::BufferSizeOverflow)
        );
    }
}
