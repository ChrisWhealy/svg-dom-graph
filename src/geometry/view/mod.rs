//! The pan/zoom state of a [`Scene`](crate::scene::Scene)'s content layer, as pure arithmetic.
//!
//! Kept free of any DOM dependency, so it stays testable with a plain `cargo test`.

use std::fmt::Write as _;
use svg_dom::root::utils::Point;

/// The smallest scale factor a [`ViewTransform`] will zoom out to.
pub(crate) const MIN_SCALE: f64 = 0.25;

/// The largest scale factor a [`ViewTransform`] will zoom in to.
pub(crate) const MAX_SCALE: f64 = 4.0;

/// The factor by which one zoom-in step multiplies the scale — and one zoom-out step divides it.
pub(crate) const ZOOM_STEP: f64 = 1.25;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The zoom factor for a wheel event's own `deltaY`, as [`ViewTransform::zoomed_about`] takes it.
///
/// One typical mouse-wheel notch — 100 pixels — is exactly one [`ZOOM_STEP`]. Smaller deltas, such as a trackpad
/// pinch (browsers report it as a stream of tiny ctrl+wheel events), zoom in proportion. Scrolling up (a negative
/// `delta_y`) zooms in.
///
/// `delta_mode` is the event's own `deltaMode`: `0` for pixels, `1` for lines (which Firefox reports for a mouse
/// wheel), and `2` for pages. A single event is limited to about four notches, so a runaway delta cannot jump the zoom
/// straight to its limit.
///
/// Returns `1.0`, meaning no change, if `delta_y` is not finite.
pub(crate) fn wheel_zoom_factor(delta_y: f64, delta_mode: u32) -> f64 {
    if !delta_y.is_finite() {
        return 1.0;
    }
    let pixels = match delta_mode {
        0 => delta_y,
        1 => delta_y * 40.0,
        _ => delta_y * 800.0,
    };
    ZOOM_STEP.powf(-pixels.clamp(-400.0, 400.0) / 100.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A uniform scale followed by a translation, applied to a `Scene`'s content layer.
///
/// A content-space point `p` lands at viewport point `p * scale + (tx, ty)`. The default is the identity: scale `1.0`,
/// no translation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ViewTransform {
    pub scale: f64,
    pub tx: f64,
    pub ty: f64,
}

impl ViewTransform {
    /// The transform that leaves content exactly where it is drawn.
    pub(crate) const IDENTITY: Self = Self { scale: 1.0, tx: 0.0, ty: 0.0 };

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// This transform after multiplying its scale by `factor`, keeping viewport point `pivot` fixed on screen.
    ///
    /// The resulting scale is clamped to [`MIN_SCALE`]..=[`MAX_SCALE`]. The translation is adjusted using the factor
    /// that was actually applied after clamping, so hitting a limit never makes the pivot drift.
    ///
    /// Returns `self` unchanged if `factor` is not a finite positive number.
    pub(crate) fn zoomed_about(self, factor: f64, pivot: Point) -> Self {
        if !factor.is_finite() || factor <= 0.0 {
            return self;
        }

        let scale = (self.scale * factor).clamp(MIN_SCALE, MAX_SCALE);
        let applied = scale / self.scale;

        Self {
            scale,
            tx: pivot.x - (pivot.x - self.tx) * applied,
            ty: pivot.y - (pivot.y - self.ty) * applied,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where content point `p` is drawn under this transform, in the `<svg>`'s own user space.
    pub(crate) fn apply(self, p: Point) -> Point {
        Point::new(p.x * self.scale + self.tx, p.y * self.scale + self.ty)
    }

    /// The content point that is drawn at `v` under this transform: the inverse of [`apply`](Self::apply).
    pub(crate) fn unapply(self, v: Point) -> Point {
        Point::new((v.x - self.tx) / self.scale, (v.y - self.ty) / self.scale)
    }

    /// Re-reads a content point under a different view.
    ///
    /// `p` is a content point that was worked out while this transform was the view, from a pointer position. If the
    /// view has since become `now`, the same pointer position is over a different content point: this returns it.
    ///
    /// A pointer gesture that measured something at its start, and cannot rely on the view staying as it was, uses
    /// this to stay true to where the pointer really is. It is `p` itself if `now` equals this transform.
    pub(crate) fn reinterpret(self, p: Point, now: Self) -> Point {
        now.unapply(self.apply(p))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// This transform moved by `(dx, dy)` in viewport space, leaving the scale alone.
    ///
    /// Viewport space, not content space: a pan of 10 moves the content 10 on screen, whatever the current zoom.
    pub(crate) fn translated(self, dx: f64, dy: f64) -> Self {
        Self {
            tx: self.tx + dx,
            ty: self.ty + dy,
            ..self
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether another zoom-in step would change anything.
    pub(crate) fn can_zoom_in(self) -> bool {
        self.scale < MAX_SCALE
    }

    /// Whether another zoom-out step would change anything.
    pub(crate) fn can_zoom_out(self) -> bool {
        self.scale > MIN_SCALE
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes this transform as an SVG `transform` attribute value into `out`, replacing its previous content.
    ///
    /// Writes into a caller-owned buffer, like the other `_into` helpers in this crate, so repeated zoom steps reuse
    /// one allocation.
    pub(crate) fn write_attr(self, out: &mut String) {
        out.clear();
        let _ = write!(out, "translate({}, {}) scale({})", self.tx, self.ty, self.scale);
    }
}

impl Default for ViewTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
