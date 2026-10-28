//! The pure side of plot nodes: what a plot is configured with, where its ticks and lines go, and what is drawn on it.
//!
//! None of it touches the DOM, so all of it is tested with a plain `cargo test`. The drawing is in `scene::node::plot`.
//!
//! - [`config`] is what a caller gives: the plane, the axes, the grid and the looks.
//! - [`items`] is what a caller draws on it: lines, dots and labels, placed by value.
//! - `ticks` finds where an axis's ticks are, and what they say.
//! - `map` turns a value into a position on the page and back again, for a linear axis and a logarithmic one.
//! - `clip` cuts what is plotted to the plotted area.
//! - `layout` puts the plotted area, the labels and the titles on the node.

mod axis;
mod clip;
mod config;
mod error;
mod items;
mod layout;
mod map;
mod ticks;

pub(crate) use clip::{Bounds, clip_polyline};
pub(crate) use layout::{ARROW_OVERHANG, PlotLayout, Side, Spot, arrow_head, default_description, resolved};
pub(crate) use ticks::{Tick, ticks};
pub use {
    axis::{AxisName, AxisPosition, AxisScale, MinorTicks, TickLabelPlacement, TickLabels},
    config::{
        Insets, LegendEntry, LegendMark, MAX_MINOR_DIVISIONS, MAX_TICKS, PlotAxis, PlotConfig, PlotGrid, PlotPlane,
    },
    error::PlotConfigError,
    items::{LabelAnchor, PlotClick, PlotItem, PlotLabel, PlotLine, PlotPoint, PlotPoints},
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
