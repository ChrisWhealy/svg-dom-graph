use super::{AxisName, MAX_TICKS};
use std::fmt;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Various reasons for why a [`super::PlotConfig`] cannot be drawn.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PlotConfigError {
    /// The node is not a finite size greater than zero, or the margins leave no room for the plotted area.
    Size,
    /// The margins are not finite numbers of zero or more.
    Insets,
    /// An axis does not run from a finite least value to a greater one.
    Range(AxisName),
    /// A logarithmic axis starts at zero or below.
    LogRange(AxisName),
    /// A major interval is not finite and greater than zero.
    Interval(AxisName),
    /// The axis would have more than [`MAX_TICKS`] ticks.
    TooManyTicks(AxisName),
    /// [`MinorTicks::Divisions`](super::MinorTicks::Divisions) has zero divisions or too many, or was given to a logarithmic axis, or
    /// [`MinorTicks::Decade`](super::MinorTicks::Decade) was given to a linear one.
    Minor(AxisName),
    /// A custom tick is at a value that is not finite, or is not above zero on a logarithmic axis.
    CustomTick(AxisName),
    /// The axis line's position is not a finite number.
    Position(AxisName),
    /// The background is not `#rgb` or `#rrggbb`.
    Background,
    /// A grid line width, or a legend line width, is not finite and greater than zero.
    Width,
    /// The description is empty, or only whitespace.
    EmptyDescription,
}

impl fmt::Display for PlotConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Size => f.write_str("the plot's size must be finite and leave room for the plotted area"),
            Self::Insets => f.write_str("the plot's margins must be finite and zero or more"),
            Self::Range(axis) => write!(f, "the {axis} axis must run from a finite value to a greater one"),
            Self::LogRange(axis) => write!(f, "the logarithmic {axis} axis must start above zero"),
            Self::Interval(axis) => write!(f, "the {axis} axis's major interval must be finite and above zero"),
            Self::TooManyTicks(axis) => write!(f, "the {axis} axis would have more than {MAX_TICKS} ticks"),
            Self::Minor(axis) => write!(f, "the {axis} axis's minor ticks do not suit its scale, or are out of range"),
            Self::CustomTick(axis) => write!(f, "a custom tick of the {axis} axis is not a usable value"),
            Self::Position(axis) => write!(f, "the {axis} axis line's position is not a finite number"),
            Self::Background => f.write_str("the plot's background must be #rgb or #rrggbb"),
            Self::Width => f.write_str("a line width of the plot must be finite and above zero"),
            Self::EmptyDescription => f.write_str("a plot's description must not be empty"),
        }
    }
}

impl std::error::Error for PlotConfigError {}
