//! The rectangle every circle must stay inside.

use super::Vec2;

/// The rectangle a particle's whole circle must stay inside: the same rectangle the scene confines a dragged node to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Bounds {
    /// The bounds of a rectangle with its top-left corner at `(left, top)`.
    pub fn from_origin_and_size(left: f64, top: f64, width: f64, height: f64) -> Self {
        Self {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }

    pub(super) fn is_valid(&self) -> bool {
        // The width and height are derived, and two finite edges can be further apart than any number can say.
        [self.left, self.top, self.right, self.bottom].iter().all(|v| v.is_finite())
            && self.right > self.left
            && self.bottom > self.top
            && (self.right - self.left).is_finite()
            && (self.bottom - self.top).is_finite()
    }

    /// `centre` moved inside the rectangle that a circle of `radius` can have its centre in.
    pub fn clamp_centre(&self, centre: Vec2, radius: f64) -> Vec2 {
        Vec2::new(
            clamp(centre.x, self.left + radius, self.right - radius),
            clamp(centre.y, self.top + radius, self.bottom - radius),
        )
    }

    pub(super) fn holds_circle(&self, radius: f64) -> bool {
        self.right - self.left > 2.0 * radius && self.bottom - self.top > 2.0 * radius
    }
}

/// `value` limited to `low..=high`. If `high < low` the result is `low`, so a region too small for a circle still gives
/// a finite answer.
fn clamp(value: f64, low: f64, high: f64) -> f64 {
    value.min(high).max(low)
}
