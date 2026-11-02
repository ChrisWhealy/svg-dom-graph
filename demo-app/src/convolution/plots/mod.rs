//! What the four plots of the convolution demo hold: how each is configured, and what is drawn for a probe frequency.
//! Pure: the page draws it with plot nodes of the library, and nothing here touches the DOM.
//!
//! - The signal plot has the signal, and the cosine and the sine that it is probed with. It is the one with the slider.
//! - Below it, side by side, are the signal multiplied by the cosine and the signal multiplied by the sine. The area
//!   between each product and the axis is shaded, one colour where it is above the axis and another where it is below,
//!   so that the integral can be seen: what is above, less what is below.
//! - The last plot has the amplitude that was found for every frequency, with a dot at the one that the probe has.

use crate::{
    convolution::analysis::{FREQUENCIES, Finding, Probe, SECONDS, SEGMENTS, seconds},
    sine_cosine::plots::{END_MARGIN, NAME_GAP, TITLE_SIZE, TRACE_WIDTH, dotted},
};
use svg_dom::root::utils::Size;
use svg_dom_graph::scene::{
    AxisPosition, Insets, LabelAnchor, PlotArea, PlotAxis, PlotConfig, PlotItem, PlotLabel, PlotLine, PlotPoint,
    PlotPoints, TickLabelPlacement, TickLabels,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The sizes of the scenes, in pixels. The two products are half as wide as the others, so they are as wide together.
pub(crate) const SIGNAL_SIZE: (f64, f64) = (800.0, 330.0);
pub(crate) const PRODUCT_SIZE: (f64, f64) = (400.0, 250.0);
pub(crate) const SPECTRUM_SIZE: (f64, f64) = (800.0, 260.0);

/// The height of the strip at the top of the signal scene that the slider is in.
pub(crate) const STRIP: f64 = 70.0;

/// The room at the left of each plot, which holds the labels of the height axis.
const LEFT_MARGIN: f64 = 40.0;

/// The room at the right of a product plot, which holds the half of the last label of the time axis that is past it.
const PRODUCT_END_MARGIN: f64 = 16.0;

/// How far the heights go either way from the axis. The signal is never more than 1.6, the sum of its two waves.
const SPAN: f64 = 1.8;

/// The distance between the ticks of the time axis of the signal, in seconds, and of the products.
const SIGNAL_TIME_STEP: f64 = 0.1;
const PRODUCT_TIME_STEP: f64 = 0.25;

/// The distance between the ticks of the height axes, and the height of the amplitude axis of the spectrum.
const HEIGHT_STEP: f64 = 0.5;
const AMPLITUDE_TOP: f64 = 1.2;
const AMPLITUDE_STEP: f64 = 0.2;

/// How far below zero the amplitude axis goes, which is the room that the two lines of writing at the bottom of the
/// spectrum are in, and where each of them is.
const AMPLITUDE_ROOM: f64 = 0.6;
const RESULT_LINES: [f64; 2] = [-0.28, -0.405];

/// The colours of the signal, of the two probes, of what is above the axis in a product, and of what is below it.
pub(crate) const SIGNAL: &str = "#1a202c";
pub(crate) const COSINE: &str = "#7c3aed";
pub(crate) const SINE: &str = "#0d9488";
pub(crate) const ABOVE: &str = "#2563eb";
pub(crate) const BELOW: &str = "#ea580c";
const POINT: &str = "#d63c32";
const GUIDE: &str = "#64748b";

/// How much of the shading shows through.
pub(crate) const SHADING: f64 = 0.4;

/// The widths of the lines: the probes are thinner than the signal, and a product is thin so that the shading shows.
const PROBE_WIDTH: f64 = 1.6;
const PRODUCT_WIDTH: f64 = 1.6;

/// The radius of the dot for the frequency of the probe in the spectrum.
const MARKER_RADIUS: f64 = 6.0;

/// How far the writing in a plot is from its edge and from the line above, in pixels.
const INSET: f64 = NAME_GAP / 2.0;
const LINE_GAP: f64 = 18.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A number with its sign, to two places, with a proper minus. Nothing that rounds to 0 has a sign.
pub(crate) fn signed(value: f64) -> String {
    if value.abs() < 0.005 {
        "0.00".to_owned()
    } else if value < 0.0 {
        format!("\u{2212}{:.2}", -value)
    } else {
        format!("+{value:.2}")
    }
}

/// The frequency of the probe as text: `2.0 Hz`.
pub(crate) fn frequency_text(frequency: f64) -> String {
    format!("{frequency:.1} Hz")
}

/// A probe as the formula that is written in its plot: `cos(2π · 2.0 · t)`.
fn probe_formula(probe: Probe, frequency: f64) -> String {
    let name = match probe {
        Probe::Cosine => "cos",
        Probe::Sine => "sin",
    };
    format!("{name}(2\u{03C0} \u{00B7} {frequency:.1} \u{00B7} t)")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A number to two places, with a proper minus where it is negative, and no plus sign.
fn plain(value: f64) -> String {
    signed(value).trim_start_matches('+').to_owned()
}

/// What the writing in the spectrum says about the frequency that has been found, in two lines. The first says that `a`
/// and `b` are twice the integrals that are written in the products, which are only half of the wave that was found,
/// or for zero frequency once the integrals, since a constant is not half of its integral. The second says the
/// amplitude and the phase, which is only said when there is a wave of that frequency in the signal to have one.
pub(crate) fn result_lines(finding: &Finding) -> [String; 2] {
    let phase = if finding.has_phase() {
        format!("{:.0}\u{00B0}", finding.phase())
    } else {
        "\u{2014}".to_owned()
    };
    let factor = finding.factor();
    [
        format!(
            "{}:   a = {factor} \u{00D7} {} = {}   b = {factor} \u{00D7} {} = {}",
            frequency_text(finding.frequency),
            plain(finding.cosine.areas.net()),
            signed(finding.a()),
            plain(finding.sine.areas.net()),
            signed(finding.b()),
        ),
        format!(
            "amplitude = \u{221A}(a\u{00B2} + b\u{00B2}) = {:.2}   phase = {phase}",
            finding.amplitude()
        ),
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What assistive technology reads for the signal plot.
pub(crate) fn signal_description(frequency: f64) -> String {
    format!(
        "Graph of a signal against time from 0 to 1 second, made of a 2 Hz wave and a 5 Hz wave, with the cosine and \
         the sine of {} that it is probed with.",
        frequency_text(frequency)
    )
}

/// What assistive technology reads for the plot of a product: the integral, and the areas it is made of.
pub(crate) fn product_description(probe: Probe, finding: &Finding) -> String {
    let (name, product) = match probe {
        Probe::Cosine => ("cosine", &finding.cosine),
        Probe::Sine => ("sine", &finding.sine),
    };
    format!(
        "Graph of the signal multiplied by the {name} of {}, against time. The area above the axis is {} and the \
         area below it is {}, so the integral is {}.",
        frequency_text(finding.frequency),
        signed(product.areas.above),
        signed(product.areas.below),
        signed(product.areas.net())
    )
}

/// What assistive technology reads for the spectrum.
pub(crate) fn spectrum_description(finding: &Finding) -> String {
    format!(
        "Graph of the amplitude found in the signal at each probe frequency from 0 to 10 Hz, with peaks at 2 Hz and \
         5 Hz. The probe is at {}, where the amplitude is {:.2}.",
        frequency_text(finding.frequency),
        finding.amplitude()
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The ticks of the time axis: one every `step` seconds, written as seconds, and none at 0, where the axes cross.
fn time_ticks(step: f64) -> TickLabels {
    let last = (SECONDS / step).round() as u32;
    TickLabels::Custom(
        (1..=last)
            .map(|n| {
                let at = f64::from(n) * step;
                let text = if step < 0.2 {
                    format!("{at:.1} s")
                } else {
                    format!("{at:.2} s").replace(".00", "")
                };
                (at, text)
            })
            .collect(),
    )
}

/// The ticks of a vertical axis: one every `step` from `-top` to `top`, or from 0 if `from_zero`, and none at 0, where
/// the axes cross. The ends of the axis are only the margin, so the last tick is the last whole step below `top`.
fn value_ticks(top: f64, step: f64, from_zero: bool) -> TickLabels {
    let last = (top / step - 1e-9).ceil() as i32 - 1;
    let first = if from_zero { 1 } else { -last };
    TickLabels::Custom(
        (first..=last)
            .filter(|&n| n != 0)
            .map(|n| {
                let value = f64::from(n) * step;
                let sign = if n < 0 { "\u{2212}" } else { "" };
                (value, format!("{sign}{:.1}", value.abs()))
            })
            .collect(),
    )
}

fn height_axis() -> PlotAxis {
    PlotAxis::linear(-SPAN, SPAN)
        .with_tick_labels(value_ticks(SPAN, HEIGHT_STEP, false))
        .with_position(AxisPosition::AtMinimum)
}

/// The plot of the signal: time along the bottom from 0 to 1 second, and the height up the left, with the strip of the
/// slider above it.
pub(crate) fn signal_config(frequency: f64) -> PlotConfig {
    PlotConfig::new(
        Size::new(SIGNAL_SIZE.0, SIGNAL_SIZE.1),
        PlotAxis::linear(0.0, SECONDS).with_tick_labels(time_ticks(SIGNAL_TIME_STEP)),
        height_axis(),
    )
    .with_label_placement(TickLabelPlacement::Axis)
    .with_insets(Insets::new(LEFT_MARGIN, STRIP, END_MARGIN, 0.0))
    .with_description(signal_description(frequency))
}

/// The plot of one product: the same scale of height as the signal, over the same second.
pub(crate) fn product_config(probe: Probe, finding: &Finding) -> PlotConfig {
    PlotConfig::new(
        Size::new(PRODUCT_SIZE.0, PRODUCT_SIZE.1),
        PlotAxis::linear(0.0, SECONDS).with_tick_labels(time_ticks(PRODUCT_TIME_STEP)),
        height_axis(),
    )
    .with_label_placement(TickLabelPlacement::Axis)
    .with_insets(Insets::new(LEFT_MARGIN, 0.0, PRODUCT_END_MARGIN, 0.0))
    .with_description(product_description(probe, finding))
}

/// The plot of the spectrum: the probe frequency along the bottom, and the amplitude up the left, with room under the
/// axis for the writing.
pub(crate) fn spectrum_config(finding: &Finding) -> PlotConfig {
    let frequencies =
        TickLabels::Custom((1..=FREQUENCIES.1 as u32).map(|n| (f64::from(n), format!("{n} Hz"))).collect());
    PlotConfig::new(
        Size::new(SPECTRUM_SIZE.0, SPECTRUM_SIZE.1),
        PlotAxis::linear(FREQUENCIES.0, FREQUENCIES.1).with_tick_labels(frequencies),
        PlotAxis::linear(-AMPLITUDE_ROOM, AMPLITUDE_TOP)
            .with_tick_labels(value_ticks(AMPLITUDE_TOP, AMPLITUDE_STEP, true))
            .with_position(AxisPosition::AtMinimum),
    )
    .with_label_placement(TickLabelPlacement::Axis)
    .with_insets(Insets::new(LEFT_MARGIN, 0.0, END_MARGIN, 0.0))
    .with_description(spectrum_description(finding))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A line of writing in a plot that starts at `x` seconds (or hertz) and is `y` high, in `colour`.
fn writing(at: (f64, f64), text: impl Into<String>, colour: &str) -> PlotItem {
    PlotLabel::new(PlotPoint::new(at.0, at.1), text)
        .with_size(TITLE_SIZE)
        .with_colour(colour)
        .with_anchor(LabelAnchor::Start)
        .with_offset(INSET, TITLE_SIZE / 3.0)
        .into()
}

/// The points of a curve that has one height for each of the times of [`seconds`].
fn curve(heights: &[f64]) -> Vec<PlotPoint> {
    heights
        .iter()
        .enumerate()
        .map(|(i, &height)| PlotPoint::new(seconds(i), height))
        .collect()
}

/// How far down, in the units of the height, a line of writing at the top of a plot is: the first, and the second.
const TOP_LINES: [f64; 2] = [SPAN - 0.2, SPAN - 0.2 - LINE_GAP / 69.0];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is drawn on the signal plot: the cosine and the sine of the probe, thin, behind the signal, and the names of the
/// three along the bottom.
pub(crate) fn signal_items(samples: &[f64], frequency: f64) -> Vec<PlotItem> {
    let probe_points = |probe: Probe| -> Vec<PlotPoint> {
        (0..=SEGMENTS)
            .map(|i| PlotPoint::new(seconds(i), probe.at(frequency, seconds(i))))
            .collect()
    };
    let bottom = -SPAN + 0.2;
    vec![
        PlotLine::new(probe_points(Probe::Cosine), COSINE)
            .with_width(PROBE_WIDTH)
            .into(),
        PlotLine::new(probe_points(Probe::Sine), SINE).with_width(PROBE_WIDTH).into(),
        PlotLine::new(curve(samples), SIGNAL).with_width(TRACE_WIDTH).into(),
        writing((0.0, bottom), "signal s(t)", SIGNAL),
        writing((0.2, bottom), probe_formula(Probe::Cosine, frequency), COSINE),
        writing((0.45, bottom), probe_formula(Probe::Sine, frequency), SINE),
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is drawn on the plot of one product: the shading between it and the axis, one colour above and one below, the
/// product itself over that, and the writing at the top that says which product it is and what the areas are.
pub(crate) fn product_items(probe: Probe, finding: &Finding) -> Vec<PlotItem> {
    let product = match probe {
        Probe::Cosine => &finding.cosine,
        Probe::Sine => &finding.sine,
    };
    let points = curve(&product.heights);
    let name = format!("s(t) \u{00B7} {}", probe_formula(probe, finding.frequency));
    let areas = product.areas;
    vec![
        PlotArea::new(points.clone(), ABOVE)
            .with_below(BELOW)
            .with_opacity(SHADING)
            .into(),
        PlotLine::new(points, SIGNAL).with_width(PRODUCT_WIDTH).into(),
        writing((0.0, TOP_LINES[0]), name, SIGNAL),
        writing((0.0, TOP_LINES[1]), format!("{} above", signed(areas.above)), ABOVE),
        writing((0.32, TOP_LINES[1]), format!("{} below", signed(areas.below)), BELOW),
        writing((0.64, TOP_LINES[1]), format!("integral {}", signed(areas.net())), SIGNAL),
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is drawn on the spectrum: the amplitude found at each frequency, a dotted line down from the probe's frequency
/// to the axis, a dot at the amplitude that was found there, and the writing: a title at the top, and the result under
/// the axis.
pub(crate) fn spectrum_items(spectrum: &[(f64, f64)], finding: &Finding) -> Vec<PlotItem> {
    let at = PlotPoint::new(finding.frequency, finding.amplitude());
    vec![
        PlotLine::new(spectrum.iter().map(|&(f, a)| PlotPoint::new(f, a)), SIGNAL)
            .with_width(TRACE_WIDTH)
            .into(),
        dotted(PlotPoint::new(finding.frequency, 0.0), at, GUIDE),
        PlotPoints::new([at], POINT).with_radius(MARKER_RADIUS).into(),
        writing(
            (FREQUENCIES.0, AMPLITUDE_TOP - 0.1),
            "Amplitude found at each probe frequency",
            SIGNAL,
        ),
    ]
    .into_iter()
    .chain(
        result_lines(finding)
            .into_iter()
            .zip(RESULT_LINES)
            .map(|(line, at)| writing((FREQUENCIES.0, at), line, POINT)),
    )
    .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
