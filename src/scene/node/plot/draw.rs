//! Draws the fixed part of a plot: its box, the plotted area, the grid, the axes, the ticks and their labels, the
//! titles and the legend. What is plotted is drawn by [`layer`](super::layer), and can change while this stays.

use super::paint::{Painter, TextStyle};
use crate::{
    colours::{
        BOX_STROKE, MAJOR_GRID_DARKENING, MINOR_GRID_DARKENING, PLOT_AXIS, PLOT_OUTLINE_DARKENING, PLOT_TICK_INK,
        darken,
    },
    error::Error,
    model::plot::{
        ARROW_OVERHANG, AxisName, LegendEntry, LegendMark, PlotConfig, PlotLayout, Side, Spot, TickLabelPlacement,
        TickLabels, arrow_head, resolved, ticks,
    },
};
use svg_dom::{
    SvgNode,
    root::utils::{Point, Size},
};

/// The font sizes of tick labels, of titles and of the legend.
const TICK_SIZE: f64 = 10.0;
const TITLE_SIZE: f64 = 12.0;
const LEGEND_SIZE: f64 = 12.0;

/// How long a tick mark in the margin is.
const TICK_MARK: f64 = 4.0;

/// An estimate of how wide a character of the legend is, as a fraction of the font size. The font is monospace, so it
/// is a good one, and it is only used to space one entry from the next.
const LEGEND_CHAR_WIDTH: f64 = 0.6;

/// The length of a legend sample, and the gaps round it.
const SAMPLE_LENGTH: f64 = 26.0;
const SAMPLE_GAP: f64 = 6.0;
const ENTRY_GAP: f64 = 14.0;

/// Used where a colour that has to be worked out from the background cannot be. The background has been checked, so
/// this does not happen.
const FALLBACK_GRID: &str = "#c8d0dc";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The colours of the grid: those the configuration gave, and else the background made darker.
pub(super) struct GridColours {
    pub(super) major: String,
    pub(super) minor: String,
    pub(super) outline: String,
}

impl GridColours {
    pub(super) fn of(config: &PlotConfig) -> Self {
        let from_background =
            |amount: f64| darken(&config.background, amount).unwrap_or_else(|| FALLBACK_GRID.to_owned());
        Self {
            major: config
                .grid
                .major_colour
                .clone()
                .unwrap_or_else(|| from_background(MAJOR_GRID_DARKENING)),
            minor: config
                .grid
                .minor_colour
                .clone()
                .unwrap_or_else(|| from_background(MINOR_GRID_DARKENING)),
            outline: from_background(PLOT_OUTLINE_DARKENING),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws the plot's box and everything fixed on it. Returns the outer box, which a focus ring is drawn on.
pub(super) fn draw_frame(p: &mut Painter<'_>, config: &PlotConfig, layout: &PlotLayout) -> Result<SvgNode, Error> {
    let area = layout.area;
    let outer = p.rect(Point::origin(), layout.size, &config.background, BOX_STROKE, 1.5)?;
    let colours = GridColours::of(config);
    p.rect(
        Point::new(area.left, area.top),
        Size::new(area.right - area.left, area.bottom - area.top),
        &config.background,
        &colours.outline,
        1.0,
    )?;

    let (x_axis, y_axis) = (resolved(config, AxisName::X), resolved(config, AxisName::Y));
    let (x_ticks, y_ticks) = (ticks(&x_axis), ticks(&y_axis));
    draw_grid(p, config, layout, &colours, &x_ticks, &y_ticks)?;

    // Each axis line is drawn where its position says, across the other axis.
    let line_y = layout.x_axis_height(&x_axis);
    let line_x = layout.y_axis_position(&y_axis);
    p.line((area.left, line_y), (area.right, line_y), PLOT_AXIS, 1.5, None)?;
    p.line((line_x, area.bottom), (line_x, area.top), PLOT_AXIS, 1.5, None)?;
    if x_axis.arrow {
        p.path(&arrow_head((area.right + ARROW_OVERHANG, line_y), (1.0, 0.0)), PLOT_AXIS, 1.5)?;
    }
    if y_axis.arrow {
        p.path(&arrow_head((line_x, area.top - ARROW_OVERHANG), (0.0, -1.0)), PLOT_AXIS, 1.5)?;
    }

    let halo = (layout.placement == TickLabelPlacement::Axis).then_some(config.background.as_str());
    let tick_style = TextStyle {
        size: TICK_SIZE,
        fill: PLOT_TICK_INK,
        halo,
        rotation: 0.0,
    };
    for tick in x_ticks.iter().filter(|t| t.major) {
        let Some(at) = layout.x.page_of(tick.value) else { continue };
        if layout.placement == TickLabelPlacement::Edge {
            p.line((at, area.bottom), (at, area.bottom + TICK_MARK), PLOT_AXIS, 1.0, None)?;
        }
        // Beside the axes, a label at zero would sit where the two axes cross.
        if let Some(text) = tick.label.as_deref().filter(|_| !(halo.is_some() && tick.value == 0.0)) {
            p.text(layout.x_label(at, line_y), text, &tick_style)?;
        }
    }
    for tick in y_ticks.iter().filter(|t| t.major) {
        let Some(at) = layout.y.page_of(tick.value) else { continue };
        if layout.placement == TickLabelPlacement::Edge {
            p.line((area.left - TICK_MARK, at), (area.left, at), PLOT_AXIS, 1.0, None)?;
        }
        if let Some(text) = tick.label.as_deref().filter(|_| !(halo.is_some() && tick.value == 0.0)) {
            p.text(layout.y_label(at, line_x), text, &tick_style)?;
        }
    }

    let title_style = TextStyle {
        size: TITLE_SIZE,
        fill: PLOT_AXIS,
        halo,
        rotation: 0.0,
    };
    if let Some(title) = &x_axis.title {
        let labels_shown = x_axis.tick_labels != TickLabels::None;
        p.text(layout.x_title(line_y, labels_shown), title, &title_style)?;
    }
    if let Some(title) = &y_axis.title {
        p.text(layout.y_title(line_x), title, &title_style)?;
    }

    draw_legend(p, config, layout)?;
    Ok(outer)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The grid lines: the minor ones first, so that a major line is over a minor one that coincides with it.
fn draw_grid(
    p: &mut Painter<'_>,
    config: &PlotConfig,
    layout: &PlotLayout,
    colours: &GridColours,
    x_ticks: &[crate::model::plot::Tick],
    y_ticks: &[crate::model::plot::Tick],
) -> Result<(), Error> {
    let area = layout.area;
    for major in [false, true] {
        let (show, colour, width) = if major {
            (config.grid.major, &colours.major, config.grid.major_width)
        } else {
            (config.grid.minor, &colours.minor, config.grid.minor_width)
        };
        if !show {
            continue;
        }
        for tick in x_ticks.iter().filter(|t| t.major == major) {
            if let Some(at) = layout.x.page_of(tick.value) {
                p.line((at, area.top), (at, area.bottom), colour, width, None)?;
            }
        }
        for tick in y_ticks.iter().filter(|t| t.major == major) {
            if let Some(at) = layout.y.page_of(tick.value) {
                p.line((area.left, at), (area.right, at), colour, width, None)?;
            }
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The key: one row along the top of the node, above the plotted area, from its left edge.
fn draw_legend(p: &mut Painter<'_>, config: &PlotConfig, layout: &PlotLayout) -> Result<(), Error> {
    let y = LEGEND_SIZE;
    let mut x = layout.area.left;
    let style = TextStyle {
        size: LEGEND_SIZE,
        fill: PLOT_TICK_INK,
        halo: None,
        rotation: 0.0,
    };
    for entry in &config.legend {
        draw_sample(p, entry, x, y)?;
        let text_x = x + SAMPLE_LENGTH + SAMPLE_GAP;
        p.text(
            Spot {
                at: (text_x, y),
                anchor: Side::Start,
            },
            &entry.label,
            &style,
        )?;
        x = text_x + entry.label.chars().count() as f64 * LEGEND_SIZE * LEGEND_CHAR_WIDTH + ENTRY_GAP;
    }
    Ok(())
}

/// The sample of one legend entry, in the space of `SAMPLE_LENGTH` that starts at `x`.
fn draw_sample(p: &mut Painter<'_>, entry: &LegendEntry, x: f64, y: f64) -> Result<(), Error> {
    match entry.mark {
        LegendMark::Line => p.line(
            (x, y),
            (x + SAMPLE_LENGTH, y),
            &entry.colour,
            entry.width,
            entry.dash.as_deref(),
        ),
        LegendMark::Dot => p.dot((x + SAMPLE_LENGTH / 2.0, y), 4.5, &entry.colour, "#ffffff"),
    }
}
