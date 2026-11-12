use super::{
    area::{Piece, pieces},
    axis::AxisName,
    clip::{Bounds, clip_polygon, clip_polyline, clip_segment},
    config::MAX_TICKS,
    layout::{self, Side},
    map::AxisMap,
    ticks::{count_estimate, format_number, nice_interval},
    *,
};
use crate::complex::Complex;
use svg_dom::root::utils::Size;

fn real(x: PlotAxis, y: PlotAxis) -> PlotConfig {
    PlotConfig::new(Size::new(400.0, 300.0), x, y)
}

fn values(axis: &PlotAxis) -> Vec<f64> {
    ticks(axis).iter().map(|t| t.value).collect()
}

fn majors(axis: &PlotAxis) -> Vec<f64> {
    ticks(axis).iter().filter(|t| t.major).map(|t| t.value).collect()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9 * (1.0 + b.abs())
}

fn unit() -> PlotConfig {
    real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(0.0, 1.0))
}

mod axis_map;
mod clipping;
mod items_and_defaults;
mod plot_layout;
mod shaded_areas;
mod tick_placement;
mod validation;
