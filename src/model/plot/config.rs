//! What a plot node is told about itself: which plane it plots, what its axes span, how they are ticked and gridded,
//! and what it looks like. Pure data with builders, and the checks that decide whether a configuration can be drawn at
//! all.

pub(crate) use super::{axis::*, error::PlotConfigError};
use crate::colours::{PLOT_BACKGROUND, parse_hex};
use svg_dom::root::utils::Size;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The most ticks one axis may have, major and minor together. A configuration that asks for more is refused, since it
/// would draw a solid band and take a long time to do it.
pub const MAX_TICKS: usize = 400;

/// The most subdivisions a major interval may be split into by [`MinorTicks::Divisions`].
pub const MAX_MINOR_DIVISIONS: u32 = 20;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which plane a plot shows.
///
/// A [`Real`](Self::Real) plot is an ordinary chart of `y` against `x`. A [`Complex`](Self::Complex) plot is an Argand
/// diagram: its horizontal axis is the real part and its vertical axis is the imaginary part. The difference is only in
/// what is assumed when a setting is left out.
///
/// | Setting left out | `Real` | `Complex` |
/// |---|---|---|
/// | Axis titles | none | `Re` and `Im` |
/// | Tick labels on the vertical axis | the number | the number and `i` |
/// | Where the tick labels are | in the margin | beside the axes |
/// | The accessible name | "Real plane" | "Complex plane" |
///
/// Points that a caller gives a plot are `(x, y)` pairs either way, and a complex number converts to one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum PlotPlane {
    /// A chart of `y` against `x`. This is the default.
    #[default]
    Real,
    /// The complex plane, with the real part along `x` and the imaginary part along `y`.
    Complex,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One axis of a plot.
///
/// Build one with [`linear`](Self::linear) or [`logarithmic`](Self::logarithmic), then adjust it.
///
/// ```
/// use svg_dom_graph::scene::{MinorTicks, PlotAxis};
/// let frequency = PlotAxis::logarithmic(1.0, 1000.0).with_title("Hz");
/// let time = PlotAxis::linear(0.0, 1.0).with_major_interval(0.25).with_minor(MinorTicks::Divisions(5));
/// assert_eq!(time.max, 1.0);
/// assert_eq!(frequency.title.as_deref(), Some("Hz"));
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PlotAxis {
    /// The least value on the axis: the left edge or the bottom edge. Finite, and less than [`max`](Self::max).
    pub min: f64,
    /// The greatest value on the axis.
    pub max: f64,
    /// How the axis spaces its values.
    pub scale: AxisScale,
    /// Text that names the axis. `None` leaves it to the plane: nothing for a real plot, and `Re` or `Im` for a complex
    /// one.
    pub title: Option<String>,
    /// The distance between major ticks. For a linear axis it is a difference of value. For a logarithmic one it is a
    /// number of decades, so `1.0` is a tick at each power of ten and `3.0` is a tick at each thousandfold.
    ///
    /// `None` chooses a round interval that gives about six. Ignored when [`tick_labels`](Self::tick_labels) is
    /// [`TickLabels::Custom`].
    pub major_interval: Option<f64>,
    /// The minor ticks.
    pub minor: MinorTicks,
    /// What is written beside the major ticks.
    pub tick_labels: TickLabels,
    /// Written after each automatic tick label. `None` leaves it to the plane: nothing, except `i` on the vertical axis
    /// of a complex plot.
    pub tick_suffix: Option<String>,
    /// Where the axis line is drawn. See [`AxisPosition`].
    pub position: AxisPosition,
    /// Whether the axis line ends in an arrowhead at its greater end. An Argand diagram has them.
    pub arrow: bool,
}

impl PlotAxis {
    /// A linear axis from `min` to `max`. It has automatic major ticks and labels, no minor ticks, and no arrowhead.
    #[must_use]
    pub fn linear(min: f64, max: f64) -> Self {
        Self {
            min,
            max,
            scale: AxisScale::Linear,
            title: None,
            major_interval: None,
            minor: MinorTicks::None,
            tick_labels: TickLabels::Auto,
            tick_suffix: None,
            position: AxisPosition::AtZero,
            arrow: false,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A logarithmic axis from `min` to `max`, both greater than zero. It has a major tick at each power of ten and
    /// minor ones at 2 to 9 times each.
    #[must_use]
    pub fn logarithmic(min: f64, max: f64) -> Self {
        Self {
            scale: AxisScale::Logarithmic,
            minor: MinorTicks::Decade,
            ..Self::linear(min, max)
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Returns `self` with the title `title`.
    #[must_use]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Returns `self` with major ticks `interval` apart. See [`major_interval`](Self::major_interval).
    #[must_use]
    pub fn with_major_interval(mut self, interval: f64) -> Self {
        self.major_interval = Some(interval);
        self
    }

    /// Returns `self` with the minor ticks `minor`.
    #[must_use]
    pub fn with_minor(mut self, minor: MinorTicks) -> Self {
        self.minor = minor;
        self
    }

    /// Returns `self` with the tick labels `labels`.
    #[must_use]
    pub fn with_tick_labels(mut self, labels: TickLabels) -> Self {
        self.tick_labels = labels;
        self
    }

    /// Returns `self` with `suffix` written after each automatic tick label.
    #[must_use]
    pub fn with_tick_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.tick_suffix = Some(suffix.into());
        self
    }

    /// Returns `self` with the axis line at `position`.
    #[must_use]
    pub fn with_position(mut self, position: AxisPosition) -> Self {
        self.position = position;
        self
    }

    /// Returns `self` with an arrowhead at the end of the axis line, or without.
    #[must_use]
    pub fn with_arrow(mut self, arrow: bool) -> Self {
        self.arrow = arrow;
        self
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The grid lines behind a plot: a heavier line at each major tick and a lighter line at each minor tick.
///
/// Unless a colour is given, a line is the background of the plot made a little darker, and the major lines are the
/// darker of the two.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PlotGrid {
    /// Whether major grid lines are drawn. They are by default.
    pub major: bool,
    /// Whether minor grid lines are drawn. They are by default, though an axis only has minor ticks if it asks for
    /// them.
    pub minor: bool,
    /// The colour of the major lines as any SVG paint, or `None` for the background made darker.
    pub major_colour: Option<String>,
    /// The colour of the minor lines, or `None` for the background made a little less darker.
    pub minor_colour: Option<String>,
    /// The widths of the lines in user units.
    pub major_width: f64,
    pub minor_width: f64,
}

impl Default for PlotGrid {
    fn default() -> Self {
        Self {
            major: true,
            minor: true,
            major_colour: None,
            minor_colour: None,
            major_width: 1.0,
            minor_width: 0.6,
        }
    }
}

impl PlotGrid {
    /// Returns `self` showing or hiding the major lines.
    #[must_use]
    pub fn with_major(mut self, show: bool) -> Self {
        self.major = show;
        self
    }

    /// Returns `self` showing or hiding the minor lines.
    #[must_use]
    pub fn with_minor(mut self, show: bool) -> Self {
        self.minor = show;
        self
    }

    /// Returns `self` with major lines of `colour`.
    #[must_use]
    pub fn with_major_colour(mut self, colour: impl Into<String>) -> Self {
        self.major_colour = Some(colour.into());
        self
    }

    /// Returns `self` with minor lines of `colour`.
    #[must_use]
    pub fn with_minor_colour(mut self, colour: impl Into<String>) -> Self {
        self.minor_colour = Some(colour.into());
        self
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How a legend entry draws its sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum LegendMark {
    /// A short line, in the colour, width and dash of the line it stands for. This is the default.
    #[default]
    Line,
    /// A dot, as a plotted point is.
    Dot,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One entry of the key above a plot: a sample, and a name.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LegendEntry {
    pub label: String,
    /// The colour of the sample, as any SVG paint.
    pub colour: String,
    /// The dash pattern of a line sample, as SVG's `stroke-dasharray`. `None` is solid.
    pub dash: Option<String>,
    /// The width of a line sample.
    pub width: f64,
    pub mark: LegendMark,
}

impl LegendEntry {
    /// An entry for a solid line of `colour`.
    #[must_use]
    pub fn line(label: impl Into<String>, colour: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            colour: colour.into(),
            dash: None,
            width: 2.0,
            mark: LegendMark::Line,
        }
    }

    /// An entry for a dot of `colour`.
    #[must_use]
    pub fn dot(label: impl Into<String>, colour: impl Into<String>) -> Self {
        Self {
            mark: LegendMark::Dot,
            ..Self::line(label, colour)
        }
    }

    /// Returns `self` with the dash pattern `dash`.
    #[must_use]
    pub fn with_dash(mut self, dash: impl Into<String>) -> Self {
        self.dash = Some(dash.into());
        self
    }

    /// Returns `self` with a sample line `width` wide.
    #[must_use]
    pub fn with_width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The room between the edge of the node and the plotted area, on each side.
///
/// A plot works this out for itself from what it has to write in the margins, and a caller that knows better can give
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Insets {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Insets {
    #[must_use]
    pub const fn new(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self { left, top, right, bottom }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything a plot node needs to be drawn.
///
/// Build one with [`new`](Self::new), which takes the size of the node and its two axes, and adjust it with the `with_`
/// methods. A struct literal does not compile outside this crate.
///
/// ```
/// use svg_dom::root::utils::Size;
/// use svg_dom_graph::scene::{PlotAxis, PlotConfig, PlotPlane};
/// let argand = PlotConfig::new(Size::new(400.0, 400.0), PlotAxis::linear(-5.5, 5.5), PlotAxis::linear(-5.5, 5.5))
///     .with_plane(PlotPlane::Complex)
///     .with_background("#fbfcfe");
/// assert_eq!(argand.plane, PlotPlane::Complex);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PlotConfig {
    /// Which plane is plotted. The default is [`PlotPlane::Real`].
    pub plane: PlotPlane,
    /// The horizontal axis: the real axis of a complex plot.
    pub x: PlotAxis,
    /// The vertical axis: the imaginary axis of a complex plot.
    pub y: PlotAxis,
    /// The size of the whole node: the plotted area and the margins round it, in user units.
    pub size: Size,
    /// The background of the plot, as `#rgb` or `#rrggbb`. The default is the pale blue of an ordinary node.
    ///
    /// It has to be a hex colour because the grid lines are worked out from it.
    pub background: String,
    /// The grid lines.
    pub grid: PlotGrid,
    /// The key above the plot. Empty draws none, and takes no room.
    pub legend: Vec<LegendEntry>,
    /// Where tick labels go. `None` leaves it to the plane.
    pub label_placement: Option<TickLabelPlacement>,
    /// The margins. `None` works them out.
    pub insets: Option<Insets>,
    /// What assistive technology reads in place of the picture. `None` writes a sentence from the axes.
    pub description: Option<String>,
}

impl PlotConfig {
    /// A real plot `size` big, with the axes `x` and `y`, in the default colours, with a grid and no legend.
    #[must_use]
    pub fn new(size: Size, x: PlotAxis, y: PlotAxis) -> Self {
        Self {
            plane: PlotPlane::Real,
            x,
            y,
            size,
            background: PLOT_BACKGROUND.to_owned(),
            grid: PlotGrid::default(),
            legend: Vec::new(),
            label_placement: None,
            insets: None,
            description: None,
        }
    }

    /// Returns `self` plotting `plane`.
    #[must_use]
    pub fn with_plane(mut self, plane: PlotPlane) -> Self {
        self.plane = plane;
        self
    }

    /// Returns `self` with the background `background`.
    #[must_use]
    pub fn with_background(mut self, background: impl Into<String>) -> Self {
        self.background = background.into();
        self
    }

    /// Returns `self` with the grid `grid`.
    #[must_use]
    pub fn with_grid(mut self, grid: PlotGrid) -> Self {
        self.grid = grid;
        self
    }

    /// Returns `self` with the legend `legend`.
    #[must_use]
    pub fn with_legend(mut self, legend: Vec<LegendEntry>) -> Self {
        self.legend = legend;
        self
    }

    /// Returns `self` with the tick labels at `placement`.
    #[must_use]
    pub fn with_label_placement(mut self, placement: TickLabelPlacement) -> Self {
        self.label_placement = Some(placement);
        self
    }

    /// Returns `self` with the margins `insets`.
    #[must_use]
    pub fn with_insets(mut self, insets: Insets) -> Self {
        self.insets = Some(insets);
        self
    }

    /// Returns `self` with the description `description`.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl PlotConfig {
    /// Whether this configuration can be drawn. Nothing is drawn, so a refusal changes nothing.
    ///
    /// # Errors
    ///
    /// Returns the first [`PlotConfigError`] found: the size, the margins, each axis in turn, the background, and the
    /// widths.
    pub fn validate(&self) -> Result<(), PlotConfigError> {
        let (w, h) = (self.size.width, self.size.height);
        if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
            return Err(PlotConfigError::Size);
        }
        if let Some(i) = self.insets {
            if [i.left, i.top, i.right, i.bottom].iter().any(|v| !v.is_finite() || *v < 0.0) {
                return Err(PlotConfigError::Insets);
            }
        }
        validate_axis(&self.x, AxisName::X)?;
        validate_axis(&self.y, AxisName::Y)?;
        if parse_hex(&self.background).is_none() {
            return Err(PlotConfigError::Background);
        }
        if self.description.as_deref().is_some_and(|d| d.trim().is_empty()) {
            return Err(PlotConfigError::EmptyDescription);
        }
        let widths = [self.grid.major_width, self.grid.minor_width]
            .into_iter()
            .chain(self.legend.iter().map(|entry| entry.width));
        if widths.into_iter().any(|v| !v.is_finite() || v <= 0.0) {
            return Err(PlotConfigError::Width);
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Checks one axis.
pub(super) fn validate_axis(axis: &PlotAxis, name: AxisName) -> Result<(), PlotConfigError> {
    // The span must be finite too: an axis from -1e308 to 1e308 has a span that overflows, and no value could be placed on it.
    let finite_range =
        axis.min.is_finite() && axis.max.is_finite() && axis.min < axis.max && (axis.max - axis.min).is_finite();
    if !finite_range {
        return Err(PlotConfigError::Range(name));
    }
    let logarithmic = axis.scale == AxisScale::Logarithmic;
    if logarithmic && axis.min <= 0.0 {
        return Err(PlotConfigError::LogRange(name));
    }
    if axis.major_interval.is_some_and(|i| !i.is_finite() || i <= 0.0) {
        return Err(PlotConfigError::Interval(name));
    }
    match axis.minor {
        MinorTicks::None => {},
        MinorTicks::Divisions(n) if !logarithmic && (1..=MAX_MINOR_DIVISIONS).contains(&n) => {},
        MinorTicks::Decade if logarithmic => {},
        _ => return Err(PlotConfigError::Minor(name)),
    }
    if let TickLabels::Custom(ticks) = &axis.tick_labels {
        if ticks.len() > MAX_TICKS {
            return Err(PlotConfigError::TooManyTicks(name));
        }
        if ticks.iter().any(|(v, _)| !v.is_finite() || (logarithmic && *v <= 0.0)) {
            return Err(PlotConfigError::CustomTick(name));
        }
    }
    if let AxisPosition::At(v) = axis.position {
        if !v.is_finite() {
            return Err(PlotConfigError::Position(name));
        }
    }
    if super::ticks::count_estimate(axis) > MAX_TICKS {
        return Err(PlotConfigError::TooManyTicks(name));
    }
    Ok(())
}
