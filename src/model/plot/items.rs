//! What is drawn on a plot: lines, points and labels, placed by the values they stand for and not by page positions.

use crate::complex::Complex;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A place on a plot, as a value of each axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotPoint {
    pub x: f64,
    pub y: f64,
}

impl PlotPoint {
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl From<(f64, f64)> for PlotPoint {
    fn from((x, y): (f64, f64)) -> Self {
        Self { x, y }
    }
}

impl From<Complex> for PlotPoint {
    /// The real part is `x` and the imaginary part is `y`.
    fn from(z: Complex) -> Self {
        Self { x: z.re, y: z.im }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A line through points, drawn as straight segments.
///
/// A segment that leaves the plotted area is cut at its edge. A point that cannot be placed, because it is not finite
/// or is not above zero on a logarithmic axis, breaks the line, so that a gap in the data is a gap in the line.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PlotLine {
    pub points: Vec<PlotPoint>,
    /// The colour, as any SVG paint.
    pub colour: String,
    /// The width in user units.
    pub width: f64,
    /// The dash pattern, as SVG's `stroke-dasharray`. `None` is solid.
    pub dash: Option<String>,
}

impl PlotLine {
    /// A solid line `1.5` wide.
    #[must_use]
    pub fn new(points: impl IntoIterator<Item = impl Into<PlotPoint>>, colour: impl Into<String>) -> Self {
        Self {
            points: points.into_iter().map(Into::into).collect(),
            colour: colour.into(),
            width: 1.5,
            dash: None,
        }
    }

    #[must_use]
    pub fn with_width(mut self, width: f64) -> Self {
        self.width = width;
        self
    }

    #[must_use]
    pub fn with_dash(mut self, dash: impl Into<String>) -> Self {
        self.dash = Some(dash.into());
        self
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Dots at points. A dot whose centre is outside the plotted area, or cannot be placed, is not drawn.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PlotPoints {
    pub points: Vec<PlotPoint>,
    /// The fill, as any SVG paint.
    pub colour: String,
    /// The radius in user units.
    pub radius: f64,
    /// The outline, which keeps a dot apart from a line that it is on. White unless set.
    pub outline: String,
}

impl PlotPoints {
    /// Dots of radius `4.5`.
    #[must_use]
    pub fn new(points: impl IntoIterator<Item = impl Into<PlotPoint>>, colour: impl Into<String>) -> Self {
        Self {
            points: points.into_iter().map(Into::into).collect(),
            colour: colour.into(),
            radius: 4.5,
            outline: "#ffffff".to_owned(),
        }
    }

    #[must_use]
    pub fn with_radius(mut self, radius: f64) -> Self {
        self.radius = radius;
        self
    }

    #[must_use]
    pub fn with_outline(mut self, outline: impl Into<String>) -> Self {
        self.outline = outline.into();
        self
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which end of a label is at its position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum LabelAnchor {
    /// The start of the text is at the position, so the text runs away from it.
    #[default]
    Start,
    Middle,
    /// The end of the text is at the position.
    End,
}

/// A line of text at a point.
///
/// The text is drawn on a halo of the plot's background, so that it can be read where it crosses a line. It is not
/// drawn if its point cannot be placed, or is outside the plotted area.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PlotLabel {
    pub at: PlotPoint,
    pub text: String,
    /// The ink, as any SVG paint.
    pub colour: String,
    /// The font size in user units.
    pub size: f64,
    pub anchor: LabelAnchor,
    /// How far the text is moved from `at`, to the right and downward, in user units. It is in page units and not in
    /// values, so a label stays the same distance from its point however the axes are scaled.
    pub offset: (f64, f64),
    /// How far the text is turned about its anchor, clockwise, in degrees.
    pub rotation: f64,
    /// Whether the text has a halo. It does by default.
    pub halo: bool,
}

impl PlotLabel {
    /// A label of size `12` in dark ink, starting at `at`.
    #[must_use]
    pub fn new(at: impl Into<PlotPoint>, text: impl Into<String>) -> Self {
        Self {
            at: at.into(),
            text: text.into(),
            colour: "#1a202c".to_owned(),
            size: 12.0,
            anchor: LabelAnchor::Start,
            offset: (0.0, 0.0),
            rotation: 0.0,
            halo: true,
        }
    }

    #[must_use]
    pub fn with_colour(mut self, colour: impl Into<String>) -> Self {
        self.colour = colour.into();
        self
    }

    #[must_use]
    pub fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub fn with_anchor(mut self, anchor: LabelAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    #[must_use]
    pub fn with_offset(mut self, dx: f64, dy: f64) -> Self {
        self.offset = (dx, dy);
        self
    }

    #[must_use]
    pub fn with_rotation(mut self, degrees: f64) -> Self {
        self.rotation = degrees;
        self
    }

    #[must_use]
    pub fn with_halo(mut self, halo: bool) -> Self {
        self.halo = halo;
        self
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One thing drawn on a plot. Items are drawn in the order given, so a later one is over an earlier one.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum PlotItem {
    Line(PlotLine),
    Points(PlotPoints),
    Label(PlotLabel),
}

impl From<PlotLine> for PlotItem {
    fn from(line: PlotLine) -> Self {
        Self::Line(line)
    }
}

impl From<PlotPoints> for PlotItem {
    fn from(points: PlotPoints) -> Self {
        Self::Points(points)
    }
}

impl From<PlotLabel> for PlotItem {
    fn from(label: PlotLabel) -> Self {
        Self::Label(label)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A click on a plot, as the values of its axes at the place that was clicked.
///
/// A click in the margin round the plotted area is not reported, since it is not at any value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlotClick {
    /// The value of the horizontal axis. On a logarithmic axis, it is the value that the position stands for, and so
    /// always above zero.
    pub x: f64,
    /// The value of the vertical axis.
    pub y: f64,
}

impl PlotClick {
    /// The place clicked, as a point.
    #[must_use]
    pub const fn point(self) -> PlotPoint {
        PlotPoint::new(self.x, self.y)
    }

    /// The place clicked, as a complex number: `x` is the real part and `y` is the imaginary part. This is what a click
    /// on a [`Complex`](super::PlotPlane::Complex) plot means.
    #[must_use]
    pub const fn complex(self) -> Complex {
        Complex::new(self.x, self.y)
    }
}
