//! What the plot of the amplitude, frequency and phase demo holds: how it is configured, and what is drawn for a wave.
//! Pure: the page draws it with a plot node of the library, and nothing here touches the DOM.
//!
//! The wave is `y = A · sin(2π · f · t + φ)`. `A` is the amplitude, from 0 to 1, which is how tall it is. `f` is the
//! frequency, from 1 Hz to 8 Hz, which is how many cycles it makes in a second. `φ` is the phase, in degrees, which is
//! how far along its cycle it is at time 0. The horizontal axis is time, from 0 to 1 second, and does not change, so a
//! higher frequency packs more cycles into the same second.
//!
//! The top of the scene is a strip for the three sliders, which the page draws. The plot is below it, so that the
//! sliders are never over the wave.

use crate::sine_cosine::plots::{END_MARGIN, HEIGHT, NAME_GAP, TITLE_SIZE, TRACE_WIDTH, dotted, size_axis};
use svg_dom::root::utils::Size;
use svg_dom_graph::scene::{
    AxisPosition, Insets, LabelAnchor, PlotAxis, PlotConfig, PlotItem, PlotLabel, PlotLine, PlotPoint,
    TickLabelPlacement, TickLabels,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size of the scene, in pixels: 800 wide, and 400 high.
pub(crate) const SIZE: (f64, f64) = (800.0, 400.0);

/// The height of the strip at the top of the scene that the sliders are in, and the height that is left for the plot.
pub(crate) const STRIP: f64 = 90.0;

/// The room at the left of the plot, which holds the labels of the height axis, since they are written to the left of
/// that axis.
pub(crate) const LEFT_MARGIN: f64 = 40.0;

/// How long the time axis is, in seconds.
pub(crate) const SECONDS: f64 = 1.0;

/// The distance between the ticks of the time axis, in seconds.
const TIME_STEP: f64 = 0.1;

/// What each slider takes: its least and greatest values, and the distance between the values it can take.
pub(crate) const AMPLITUDES: (f64, f64) = (0.0, 1.0);
pub(crate) const AMPLITUDE_STEP: f64 = 0.01;
pub(crate) const FREQUENCIES: (f64, f64) = (1.0, 8.0);
pub(crate) const FREQUENCY_STEP: f64 = 0.1;
pub(crate) const PHASES: (f64, f64) = (0.0, 360.0);
pub(crate) const PHASE_STEP: f64 = 1.0;

/// How many straight segments make the wave. At 8 Hz that is 64 for each cycle, which is smooth.
const SEGMENTS: usize = 512;

/// The colour of the lines that mark the amplitude, which is not the colour of the wave.
const GUIDE: &str = "#64748b";

/// How far the formula is from the left edge of the plot, in pixels, and where it is, in values of the vertical axis.
/// No wave goes lower than -1, so the strip below that is where it is.
const FORMULA_AT: f64 = -1.1;
const FORMULA_INSET: f64 = NAME_GAP / 2.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One sine wave: how tall it is, how fast it goes, and where in its cycle it starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Wave {
    /// From 0 to 1.
    pub(crate) amplitude: f64,
    /// In cycles for each second, from 1 to 8.
    pub(crate) frequency: f64,
    /// In degrees, from 0 to 360.
    pub(crate) phase: f64,
}

impl Wave {
    /// The wave that the page starts with.
    pub(crate) const fn first() -> Self {
        Self {
            amplitude: 1.0,
            frequency: 2.0,
            phase: 0.0,
        }
    }

    /// The height of the wave at `seconds`.
    pub(crate) fn at(&self, seconds: f64) -> f64 {
        self.amplitude * (std::f64::consts::TAU * self.frequency * seconds + self.phase.to_radians()).sin()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The values of the three sliders as the text that is written beside each: `2.0 Hz`, `0.80` and `90°`.
pub(crate) fn frequency_text(frequency: f64) -> String {
    format!("{frequency:.1} Hz")
}

pub(crate) fn amplitude_text(amplitude: f64) -> String {
    format!("{amplitude:.2}")
}

pub(crate) fn phase_text(phase: f64) -> String {
    format!("{phase:.0}\u{00B0}")
}

/// The wave as a formula, which is written in the plot: `y = 0.80 · sin(2π · 2.0 · t + 90°)`.
pub(crate) fn formula(wave: &Wave) -> String {
    format!(
        "y = {} \u{00B7} sin(2\u{03C0} \u{00B7} {:.1} \u{00B7} t + {})",
        amplitude_text(wave.amplitude),
        wave.frequency,
        phase_text(wave.phase)
    )
}

/// What assistive technology reads for the plot.
pub(crate) fn description(wave: &Wave) -> String {
    format!(
        "Graph of a sine wave against time from 0 to 1 second, with an amplitude of {}, a frequency of {} and a phase \
         of {} degrees. The wave makes {:.1} cycles in the second.",
        amplitude_text(wave.amplitude),
        frequency_text(wave.frequency),
        phase_text(wave.phase).trim_end_matches('\u{00B0}'),
        wave.frequency
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The ticks of the time axis: one every [`TIME_STEP`] seconds, written as seconds, and none at 0, where the axes
/// cross.
fn time_ticks() -> TickLabels {
    let last = (SECONDS / TIME_STEP).round() as u32;
    TickLabels::Custom(
        (1..=last)
            .map(|n| {
                let seconds = f64::from(n) * TIME_STEP;
                (seconds, format!("{seconds:.1} s"))
            })
            .collect(),
    )
}

/// The plot of the wave: time along the bottom from 0 to 1 second, and the height from -1.2 to 1.2 up the left, with
/// room at the left for the labels of the height, room at the right for the last label of the time, and the strip of
/// the sliders above it.
pub(crate) fn config(wave: &Wave) -> PlotConfig {
    let time = PlotAxis::linear(0.0, SECONDS).with_tick_labels(time_ticks());
    let height = size_axis().with_position(AxisPosition::AtMinimum);
    PlotConfig::new(Size::new(SIZE.0, SIZE.1), time, height)
        .with_label_placement(TickLabelPlacement::Axis)
        .with_insets(Insets::new(LEFT_MARGIN, STRIP, END_MARGIN, 0.0))
        .with_description(description(wave))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The points of `wave` over the whole second.
fn curve(wave: &Wave) -> Vec<PlotPoint> {
    (0..=SEGMENTS)
        .map(|i| {
            let seconds = SECONDS * i as f64 / SEGMENTS as f64;
            PlotPoint::new(seconds, wave.at(seconds))
        })
        .collect()
}

/// What is drawn on the plot for `wave`: dotted lines at the top and the bottom of the wave, which are as far from the
/// axis as the amplitude, the wave itself, and the formula, which is at the bottom left.
pub(crate) fn items(wave: &Wave) -> Vec<PlotItem> {
    let mut items: Vec<PlotItem> = Vec::new();
    if wave.amplitude > 0.0 {
        for height in [wave.amplitude, -wave.amplitude] {
            items.push(dotted(PlotPoint::new(0.0, height), PlotPoint::new(SECONDS, height), GUIDE));
        }
    }
    items.push(PlotLine::new(curve(wave), HEIGHT).with_width(TRACE_WIDTH).into());
    items.push(
        PlotLabel::new(PlotPoint::new(0.0, FORMULA_AT), formula(wave))
            .with_size(TITLE_SIZE)
            .with_anchor(LabelAnchor::Start)
            .with_offset(FORMULA_INSET, TITLE_SIZE / 3.0)
            .into(),
    );
    items
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
