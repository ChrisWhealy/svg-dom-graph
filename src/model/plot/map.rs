//! The map between a value on an axis and a position on the page. Pure.

use super::{axis::AxisScale, config::PlotAxis};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The furthest a fraction along an axis is taken. It is far past any plot, and small enough that clipping a segment
/// between two such fractions cannot overflow.
pub(crate) const FRACTION_LIMIT: f64 = 1e300;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where an axis's values land: `min` at `from` and `max` at `to`, both in user units along the page. For a vertical
/// axis `from` is the bottom, which is the greater of the two coordinates, so the greater values are higher up.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AxisMap {
    min: f64,
    max: f64,
    scale: AxisScale,
    from: f64,
    to: f64,
}

impl AxisMap {
    pub(crate) fn new(axis: &PlotAxis, from: f64, to: f64) -> Self {
        Self {
            min: axis.min,
            max: axis.max,
            scale: axis.scale,
            from,
            to,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How far along the axis `value` is, from `0.0` at `min` to `1.0` at `max`. `None` if it cannot be placed: it is
    /// not finite, or it is not above zero on a logarithmic axis. A value off either end is placed beyond `0.0` or
    /// `1.0`.
    fn fraction(&self, value: f64) -> Option<f64> {
        if !value.is_finite() {
            return None;
        }
        match self.scale {
            AxisScale::Linear => Some((value - self.min) / (self.max - self.min)),
            AxisScale::Logarithmic if value > 0.0 => {
                Some((value.log10() - self.min.log10()) / (self.max.log10() - self.min.log10()))
            },
            AxisScale::Logarithmic => None,
        }
    }

    /// How far along the axis `value` is, as [`fraction`](Self::fraction) but always finite: a value so far off the axis
    /// that its fraction would overflow is placed at [`FRACTION_LIMIT`] on the side it is on. `None` if it cannot be
    /// placed.
    pub(crate) fn finite_fraction(&self, value: f64) -> Option<f64> {
        let fraction = self.fraction(value)?;
        (!fraction.is_nan()).then(|| fraction.clamp(-FRACTION_LIMIT, FRACTION_LIMIT))
    }

    /// Where a fraction along the axis is on the page.
    pub(crate) fn page_at(&self, fraction: f64) -> f64 {
        self.from + fraction * (self.to - self.from)
    }

    /// Where `value` is on the page, or `None` if it cannot be placed. The position is always finite.
    pub(crate) fn page_of(&self, value: f64) -> Option<f64> {
        let position = self.page_at(self.fraction(value)?);
        (!position.is_nan()).then(|| position.clamp(-f64::MAX, f64::MAX))
    }

    /// The value at `position` on the page. The inverse of [`page_of`](Self::page_of).
    pub(crate) fn value_at(&self, position: f64) -> f64 {
        let fraction = (position - self.from) / (self.to - self.from);
        match self.scale {
            AxisScale::Linear => self.min + fraction * (self.max - self.min),
            AxisScale::Logarithmic => 10f64.powf(self.min.log10() + fraction * (self.max.log10() - self.min.log10())),
        }
    }

    /// Where the line of a position on this axis is drawn: `wanted`, kept within the axis, or if it cannot be placed on
    /// it, the end named by `fallback_to_max`.
    pub(crate) fn clamped_page(&self, wanted: f64, fallback_to_max: bool) -> f64 {
        match self.fraction(wanted) {
            Some(f) => self.from + f.clamp(0.0, 1.0) * (self.to - self.from),
            None => self.from + if fallback_to_max { 1.0 } else { 0.0 } * (self.to - self.from),
        }
    }
}
