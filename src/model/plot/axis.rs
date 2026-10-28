use std::fmt;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AxisName {
    X,
    Y,
}

impl fmt::Display for AxisName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::X => "horizontal",
            Self::Y => "vertical",
        })
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How the values on an axis are spaced
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum AxisScale {
    /// Equal steps of value are equal distances. This is the default.
    #[default]
    Linear,
    /// Equal ratios of value are equal distances.
    /// Both ends of the axis must be greater than zero, and a value of zero or less cannot be placed on it.
    Logarithmic,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The minor ticks of an axis, which are the lighter lines between the major ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum MinorTicks {
    /// No minor ticks. This is the default for a linear axis.
    #[default]
    None,
    /// Each major interval is split into this many equal parts, so `Divisions(5)` puts four minor ticks between two
    /// major ones. For a linear axis only. It must be from 1 to [`MAX_MINOR_DIVISIONS`](super::MAX_MINOR_DIVISIONS).
    Divisions(u32),
    /// At 2, 3 … 9 times each power of ten, as on logarithmic graph paper.
    /// For a logarithmic axis only, where it is the default.
    Decade,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Major tick labels
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum TickLabels {
    /// The value of each major tick, with the axis's suffix after it. This is the default.
    #[default]
    Auto,
    /// No labels.
    None,
    /// Use exactly these tick labels.
    /// They replace the axis' own major ticks and its interval, so the position must be caller-defined.
    /// A value beyond the axis' end value is not drawn.
    Custom(Vec<(f64, String)>),
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where tick labels are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TickLabelPlacement {
    /// In the margin outside the plotted area: along its bottom and down its left. This is what a chart does, and is
    /// the default of a [`Real`](super::PlotPlane::Real) plot.
    Edge,
    /// Beside the axis lines, inside the plotted area. This is what an Argand diagram does, and is the default of a
    /// [`Complex`](super::PlotPlane::Complex) plot.
    Axis,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where an axis line is drawn: a value on the other axis.
///
/// The horizontal axis line is drawn across the plot at a certain height, and the vertical one at a position along it.
/// So `AtZero` for the horizontal axis puts it where `y` = zero.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[non_exhaustive]
pub enum AxisPosition {
    /// At zero on the other axis. Where that axis does not span zero, or is logarithmic, the line is drawn at the
    /// nearest edge. This is the default.
    #[default]
    AtZero,
    /// At the least value of the other axis, which is the left edge or the bottom edge.
    AtMinimum,
    /// At the greatest value of the other axis.
    AtMaximum,
    /// At this value of the other axis. A value outside it is drawn at the nearest edge.
    At(f64),
}
