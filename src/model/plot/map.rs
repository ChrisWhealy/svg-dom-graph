//! The map between a value on an axis and a position on the page. Pure.

use super::{axis::AxisScale, config::PlotAxis};

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

    /// Where `value` is on the page, or `None` if it cannot be placed.
    pub(crate) fn page_of(&self, value: f64) -> Option<f64> {
        self.fraction(value).map(|f| self.from + f * (self.to - self.from))
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
