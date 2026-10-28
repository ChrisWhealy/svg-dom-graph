//! Where everything of a plot goes on the node, worked out from its configuration. Pure.
//!
//! Positions are in the node's own coordinates, with the top-left corner of the node at the origin. The plotted area is
//! the rectangle inside the margins. The margins hold the tick labels, the legend and the titles of a chart, and are
//! only as big as what they hold.

use super::{
    axis::{AxisName, AxisPosition, AxisScale, TickLabelPlacement, TickLabels},
    clip::Bounds,
    config::{PlotAxis, PlotConfig, PlotConfigError, PlotPlane},
    map::AxisMap,
    ticks::format_number,
};
use svg_dom::root::utils::Size;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The margin where nothing is written, and the room a row of tick labels, a title and a legend each take.
const PLAIN_MARGIN: f64 = 14.0;
const Y_LABEL_ROOM: f64 = 40.0;
const X_LABEL_ROOM: f64 = 22.0;
const TITLE_ROOM: f64 = 18.0;
const LEGEND_ROOM: f64 = 26.0;

/// How far past the end of an axis line the tip of its arrowhead is.
pub(crate) const ARROW_OVERHANG: f64 = 4.0;

/// How long the strokes of an arrowhead are, and how far each is turned from the axis, in radians.
const ARROW_LENGTH: f64 = 8.0;
const ARROW_SPREAD: f64 = 0.45;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which side of its position a piece of text lies on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    Start,
    Middle,
    End,
}

/// A piece of text's position and anchor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Spot {
    pub(crate) at: (f64, f64),
    pub(crate) anchor: Side,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The placement of tick labels `config` has, whether it chose it or the plane did.
pub(crate) fn label_placement(config: &PlotConfig) -> TickLabelPlacement {
    config.label_placement.unwrap_or(match config.plane {
        PlotPlane::Real => TickLabelPlacement::Edge,
        PlotPlane::Complex => TickLabelPlacement::Axis,
    })
}

/// The title of an axis: the one it was given, or the plane's own.
pub(crate) fn title(config: &PlotConfig, name: AxisName) -> Option<&str> {
    let (axis, own) = match name {
        AxisName::X => (&config.x, "Re"),
        AxisName::Y => (&config.y, "Im"),
    };
    axis.title
        .as_deref()
        .or((config.plane == PlotPlane::Complex).then_some(own))
        .filter(|text| !text.is_empty())
}

/// What an axis puts after an automatic tick label: the one it was given, or the plane's own.
pub(crate) fn suffix(config: &PlotConfig, name: AxisName) -> String {
    let axis = match name {
        AxisName::X => &config.x,
        AxisName::Y => &config.y,
    };
    match (&axis.tick_suffix, config.plane, name) {
        (Some(text), _, _) => text.clone(),
        (None, PlotPlane::Complex, AxisName::Y) => "i".to_owned(),
        (None, _, _) => String::new(),
    }
}

/// `axis` as it is drawn, with the plane's titles and suffixes written into it.
pub(crate) fn resolved(config: &PlotConfig, name: AxisName) -> PlotAxis {
    let mut axis = match name {
        AxisName::X => config.x.clone(),
        AxisName::Y => config.y.clone(),
    };
    axis.title = title(config, name).map(str::to_owned);
    axis.tick_suffix = Some(suffix(config, name));
    axis
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The margins of `config`: the ones it gave, or what its labels, titles and legend need.
fn insets(config: &PlotConfig) -> (f64, f64, f64, f64) {
    if let Some(i) = config.insets {
        return (i.left, i.top, i.right, i.bottom);
    }
    if label_placement(config) == TickLabelPlacement::Axis {
        return (PLAIN_MARGIN, PLAIN_MARGIN, PLAIN_MARGIN, PLAIN_MARGIN);
    }
    let shown = |axis: &PlotAxis| axis.tick_labels != TickLabels::None;
    let left = if shown(&config.y) { Y_LABEL_ROOM + PLAIN_MARGIN } else { PLAIN_MARGIN };
    let mut bottom = PLAIN_MARGIN;
    if shown(&config.x) {
        bottom += X_LABEL_ROOM;
    }
    if title(config, AxisName::X).is_some() {
        bottom += TITLE_ROOM;
    }
    let mut top = PLAIN_MARGIN;
    if !config.legend.is_empty() {
        top += LEGEND_ROOM;
    }
    if title(config, AxisName::Y).is_some() {
        top += TITLE_ROOM;
    }
    (left, top, PLAIN_MARGIN, bottom)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The layout of one plot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PlotLayout {
    pub(crate) size: Size,
    /// The plotted area.
    pub(crate) area: Bounds,
    pub(crate) x: AxisMap,
    pub(crate) y: AxisMap,
    pub(crate) placement: TickLabelPlacement,
}

impl PlotLayout {
    /// The layout of `config`.
    ///
    /// # Errors
    ///
    /// Returns the reason `config` cannot be drawn: any that [`PlotConfig::validate`] finds, and
    /// [`PlotConfigError::Size`] if the margins leave no plotted area.
    pub(crate) fn new(config: &PlotConfig) -> Result<Self, PlotConfigError> {
        config.validate()?;
        let (left, top, right, bottom) = insets(config);
        let area = Bounds {
            left,
            top,
            right: config.size.width - right,
            bottom: config.size.height - bottom,
        };
        if area.right - area.left < 1.0 || area.bottom - area.top < 1.0 {
            return Err(PlotConfigError::Size);
        }
        Ok(Self {
            size: config.size,
            area,
            x: AxisMap::new(&config.x, area.left, area.right),
            // The greater values are higher up the page, which is the smaller coordinate.
            y: AxisMap::new(&config.y, area.bottom, area.top),
            placement: label_placement(config),
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where the point `(x, y)` is on the node, or `None` if either value cannot be placed.
    pub(crate) fn point(&self, x: f64, y: f64) -> Option<(f64, f64)> {
        Some((self.x.page_of(x)?, self.y.page_of(y)?))
    }

    /// The values at the position `local` on the node, or `None` if it is not inside the plotted area.
    pub(crate) fn values_at(&self, local: (f64, f64)) -> Option<(f64, f64)> {
        self.area
            .contains(local)
            .then(|| (self.x.value_at(local.0), self.y.value_at(local.1)))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The height, on the node, of the horizontal axis line.
    pub(crate) fn x_axis_height(&self, axis: &PlotAxis) -> f64 {
        line_position(self.y, axis.position)
    }

    /// The position, on the node, of the vertical axis line, given the vertical axis's own settings.
    pub(crate) fn y_axis_position(&self, axis: &PlotAxis) -> f64 {
        line_position(self.x, axis.position)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where the label of a tick of the horizontal axis at page position `at` goes. `line` is the height of the
    /// horizontal axis line.
    pub(crate) fn x_label(&self, at: f64, line: f64) -> Spot {
        match self.placement {
            TickLabelPlacement::Edge => Spot {
                at: (at, self.area.bottom + 14.0),
                anchor: Side::Middle,
            },
            TickLabelPlacement::Axis => Spot {
                at: (at, line + 11.0),
                anchor: Side::Middle,
            },
        }
    }

    /// Where the label of a tick of the vertical axis at page position `at` goes. `line` is the position of the
    /// vertical axis line.
    pub(crate) fn y_label(&self, at: f64, line: f64) -> Spot {
        match self.placement {
            TickLabelPlacement::Edge => Spot {
                at: (self.area.left - 8.0, at),
                anchor: Side::End,
            },
            TickLabelPlacement::Axis => Spot {
                at: (line - 7.0, at),
                anchor: Side::End,
            },
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Where the title of the horizontal axis goes. In the margin it is centred under the tick labels. Beside the axes
    /// it is at the right-hand end, above the axis line.
    pub(crate) fn x_title(&self, line: f64, labels_shown: bool) -> Spot {
        match self.placement {
            TickLabelPlacement::Edge => Spot {
                at: (
                    (self.area.left + self.area.right) / 2.0,
                    self.area.bottom + if labels_shown { 14.0 + TITLE_ROOM } else { 14.0 },
                ),
                anchor: Side::Middle,
            },
            TickLabelPlacement::Axis => Spot {
                at: (self.area.right - 4.0, line - 12.0),
                anchor: Side::End,
            },
        }
    }

    /// Where the title of the vertical axis goes. In the margin it is above the plotted area, at its left edge. Beside
    /// the axes it is at the top end, to the right of the axis line.
    pub(crate) fn y_title(&self, line: f64) -> Spot {
        match self.placement {
            TickLabelPlacement::Edge => Spot {
                at: (self.area.left, self.area.top - 10.0),
                anchor: Side::Start,
            },
            TickLabelPlacement::Axis => Spot {
                at: (line + 8.0, self.area.top + 6.0),
                anchor: Side::Start,
            },
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where on its own axis (`across`) an axis line with `position` is drawn.
fn line_position(across: AxisMap, position: AxisPosition) -> f64 {
    match position {
        AxisPosition::AtZero => across.clamped_page(0.0, false),
        AxisPosition::AtMinimum => across.clamped_page(f64::NEG_INFINITY, false),
        AxisPosition::AtMaximum => across.clamped_page(f64::INFINITY, true),
        AxisPosition::At(value) => across.clamped_page(value, false),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The path of an arrowhead with its tip at `tip`, on a line that runs in `direction`, which is a unit vector. Two
/// short strokes go back from the tip, a little either side of the line.
pub(crate) fn arrow_head(tip: (f64, f64), direction: (f64, f64)) -> String {
    let back = (-direction.0, -direction.1);
    let turned = |angle: f64| {
        (
            back.0 * angle.cos() - back.1 * angle.sin(),
            back.0 * angle.sin() + back.1 * angle.cos(),
        )
    };
    let (a, b) = (turned(ARROW_SPREAD), turned(-ARROW_SPREAD));
    format!(
        "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2}",
        tip.0 + a.0 * ARROW_LENGTH,
        tip.1 + a.1 * ARROW_LENGTH,
        tip.0,
        tip.1,
        tip.0 + b.0 * ARROW_LENGTH,
        tip.1 + b.1 * ARROW_LENGTH
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a screen reader is told about a plot that has no description of its own.
pub(crate) fn default_description(config: &PlotConfig) -> String {
    let span = |axis: &PlotAxis| {
        format!(
            "{} from {} to {}",
            if axis.scale == AxisScale::Logarithmic { "logarithmic" } else { "linear" },
            format_number(axis.min),
            format_number(axis.max)
        )
    };
    let plane = match config.plane {
        PlotPlane::Real => "Real plane",
        PlotPlane::Complex => "Complex plane",
    };
    let (horizontal, vertical) = match config.plane {
        PlotPlane::Real => ("horizontal axis", "vertical axis"),
        PlotPlane::Complex => ("real axis", "imaginary axis"),
    };
    format!("{plane} plot: {horizontal} {}, {vertical} {}", span(&config.x), span(&config.y))
}
