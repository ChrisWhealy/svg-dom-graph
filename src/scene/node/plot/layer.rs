//! Draws what is plotted: the items a caller gives, placed by value and cut to the plotted area.

use super::paint::{Painter, TextStyle};
use crate::{
    error::Error,
    model::plot::{
        Bounds, LabelAnchor, PlotArea, PlotItem, PlotLabel, PlotLayout, PlotLine, PlotPoint, PlotPoints, Side, Spot,
    },
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Draws `items`, in order, with `painter`, on the plot of `layout`. `background` is the colour of every halo.
pub(super) fn draw_items(
    painter: &mut Painter<'_>,
    layout: &PlotLayout,
    background: &str,
    items: &[PlotItem],
) -> Result<(), Error> {
    for item in items {
        match item {
            PlotItem::Line(line) => draw_line(painter, layout, line)?,
            PlotItem::Area(area) => draw_area(painter, layout, area)?,
            PlotItem::Points(points) => draw_points(painter, layout, points)?,
            PlotItem::Label(label) => draw_label(painter, layout, background, label)?,
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Each place a line goes, on the node. `None` where a point cannot be placed.
fn placed(layout: &PlotLayout, points: &[PlotPoint]) -> Vec<Option<(f64, f64)>> {
    points.iter().map(|p| layout.point(p.x, p.y)).collect()
}

fn draw_line(painter: &mut Painter<'_>, layout: &PlotLayout, line: &PlotLine) -> Result<(), Error> {
    let values: Vec<(f64, f64)> = line.points.iter().map(|p| (p.x, p.y)).collect();
    for run in layout.clip_line(&values) {
        painter.polyline(&run, &line.colour, line.width, line.dash.as_deref())?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The shading between a line and its baseline: each piece on one side of the baseline, cut to the plotted area. An
/// opacity that is not a number from 0 to 1 is taken as the nearest end.
fn draw_area(painter: &mut Painter<'_>, layout: &PlotLayout, area: &PlotArea) -> Result<(), Error> {
    let opacity = if area.opacity.is_nan() { 0.0 } else { area.opacity.clamp(0.0, 1.0) };
    for (above, polygon) in layout.clip_area(&area.points, area.baseline) {
        let fill = if above { &area.above } else { &area.below };
        painter.polygon(&polygon, fill, opacity)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether `at` is inside the area, or on its edge: a dot centred on an edge is still half seen.
fn inside(area: Bounds, at: (f64, f64)) -> bool {
    area.contains(at)
}

fn draw_points(painter: &mut Painter<'_>, layout: &PlotLayout, points: &PlotPoints) -> Result<(), Error> {
    for at in placed(layout, &points.points).into_iter().flatten() {
        if inside(layout.area, at) {
            painter.dot(at, points.radius, &points.colour, &points.outline)?;
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn draw_label(
    painter: &mut Painter<'_>,
    layout: &PlotLayout,
    background: &str,
    label: &PlotLabel,
) -> Result<(), Error> {
    let Some(at) = layout.point(label.at.x, label.at.y).filter(|at| inside(layout.area, *at)) else {
        return Ok(());
    };
    let spot = Spot {
        at: (at.0 + label.offset.0, at.1 + label.offset.1),
        anchor: match label.anchor {
            LabelAnchor::Start => Side::Start,
            LabelAnchor::Middle => Side::Middle,
            LabelAnchor::End => Side::End,
        },
    };
    painter.text(
        spot,
        &label.text,
        &TextStyle {
            size: label.size,
            fill: &label.colour,
            halo: label.halo.then_some(background),
            rotation: label.rotation,
        },
    )
}
