//! What the graph of the three waves and their sum is: how its plot node is configured, and what is drawn on it. Pure:
//! the page draws it with a plot node of the library, and nothing here touches the DOM.
//!
//! The vertical axis stands at the left edge and runs from the lowest value to the highest, so it shows negative and
//! positive values. Time runs from 0 to 1 along the horizontal axis, so it shows positive values only. The scale never
//! changes, so a wave that is made taller moves up the graph and the axes stay put.
//!
//! Colour is never the only way to tell the lines apart. Wave A is solid, B is dashed and C is dotted, and the sum is
//! thick.

use super::waves::{FREQUENCIES, MAX_AMPLITUDE, NAMES, Wave, sum_at, wave_at};
use svg_dom::root::utils::Size;
use svg_dom_graph::scene::{
    AxisPosition, LegendEntry, MinorTicks, PlotAxis, PlotConfig, PlotItem, PlotLine, PlotPoint, PlotPoints, TickLabels,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size of the plot node.
pub(super) const SIZE: (f64, f64) = (660.0, 340.0);

/// The values at the bottom and the top of the graph: all three waves at their tallest.
const RANGE: f64 = 3.0 * MAX_AMPLITUDE;

/// How many straight segments make a curve. Enough that the fastest wave still looks round.
const SEGMENTS: usize = 240;

/// The colours of the three waves, in the order of [`NAMES`], and of the sum. The sum is dark, since the graph is pale.
const COLOURS: [&str; 3] = ["#e5534b", "#2f9e44", "#3b82f6"];
const SUM: &str = "#1a202c";

/// The dash patterns of the three waves: solid, dashed and dotted.
const DASHES: [Option<&str>; 3] = [None, Some("7 4"), Some("2 4")];

/// The fill of a sample mark.
const SAMPLE: &str = "#f5a623";

/// The widths of the lines of a wave, and of the sum.
const WAVE_WIDTH: f64 = 1.6;
const SUM_WIDTH: f64 = 3.2;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The ticks of the horizontal axis: at each quarter of the window, named as fractions, since that is what they are.
fn time_ticks() -> TickLabels {
    TickLabels::Custom(
        ["0", "\u{00BC}", "\u{00BD}", "\u{00BE}", "1"]
            .into_iter()
            .enumerate()
            .map(|(quarter, text)| (quarter as f64 / 4.0, text.to_owned()))
            .collect(),
    )
}

/// The key above the graph: one entry for each wave, the sum, and the sample marks.
fn legend() -> Vec<LegendEntry> {
    let mut entries: Vec<LegendEntry> = (0..3)
        .map(|index| {
            let cycles = FREQUENCIES[index];
            let entry = LegendEntry::line(
                format!("{} ({cycles} cycle{})", NAMES[index], if cycles == 1 { "" } else { "s" }),
                COLOURS[index],
            )
            .with_width(WAVE_WIDTH + 0.4);
            DASHES[index].map_or(entry.clone(), |dash| entry.with_dash(dash))
        })
        .collect();
    entries.push(LegendEntry::line("A + B + C", SUM).with_width(SUM_WIDTH + 0.3));
    entries.push(LegendEntry::dot("samples", SAMPLE));
    entries
}

/// The plot node of the graph. Its description is written from the waves, and is not part of this.
pub(super) fn config() -> PlotConfig {
    let time = PlotAxis::linear(0.0, 1.0)
        .with_title("time, over one window")
        .with_tick_labels(time_ticks())
        .with_minor(MinorTicks::None);
    let value = PlotAxis::linear(-RANGE, RANGE)
        .with_major_interval(1.0)
        .with_title("value")
        .with_position(AxisPosition::AtMinimum);
    PlotConfig::new(Size::new(SIZE.0, SIZE.1), time, value).with_legend(legend())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The points of the curve of `value` over the whole window.
fn curve(value: impl Fn(f64) -> f64) -> Vec<PlotPoint> {
    (0..=SEGMENTS)
        .map(|i| {
            let t = i as f64 / SEGMENTS as f64;
            PlotPoint::new(t, value(t))
        })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is drawn on the graph: the three waves, the sum over them, and a mark on the sum for each of `samples`.
pub(super) fn items(waves: &[Wave; 3], samples: usize) -> Vec<PlotItem> {
    let mut items: Vec<PlotItem> = (0..3)
        .map(|index| {
            let (wave, frequency) = (waves[index], FREQUENCIES[index]);
            let line = PlotLine::new(curve(|t| wave_at(wave, frequency, t)), COLOURS[index]).with_width(WAVE_WIDTH);
            PlotItem::from(DASHES[index].map_or(line.clone(), |dash| line.with_dash(dash)))
        })
        .collect();
    items.push(PlotLine::new(curve(|t| sum_at(waves, t)), SUM).with_width(SUM_WIDTH).into());
    let marks = (0..samples).map(|i| {
        let t = i as f64 / samples as f64;
        PlotPoint::new(t, sum_at(waves, t))
    });
    items.push(PlotPoints::new(marks, SAMPLE).with_outline(SUM).into());
    items
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The lowest and the highest the sum goes.
fn extremes(waves: &[Wave; 3]) -> (f64, f64) {
    (0..=SEGMENTS)
        .map(|i| sum_at(waves, i as f64 / SEGMENTS as f64))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), v| (low.min(v), high.max(v)))
}

/// What a screen reader says for the graph.
pub(super) fn label(waves: &[Wave; 3], samples: usize) -> String {
    let (low, high) = extremes(waves);
    let each: Vec<String> = waves
        .iter()
        .zip(NAMES)
        .zip(FREQUENCIES)
        .map(|((wave, name), cycles)| {
            format!(
                "wave {name}, {cycles} cycle{}, amplitude {:.2}, phase {:.0} degrees",
                if cycles == 1 { "" } else { "s" },
                wave.amplitude,
                wave.phase
            )
        })
        .collect();
    format!(
        "Graph of three sine waves and their sum over one window of time: {}. The sum ranges from {low:.2} to {high:.2}. \
         The sum is sampled at {samples} evenly spaced points.",
        each.join("; ")
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
