//! What the two plots of the phase shift demo hold: what is drawn on each for a phase shift. Pure: the page draws it
//! with plot nodes of the library, and nothing here touches the DOM.
//!
//! The circle is the one of the sine and cosine demo, with point C on it. The sine plot has two sine waves in it. The
//! first is `sin(x)`, which never moves. The second is `sin(x + φ)`, where φ is the angle of C. At the left edge, where
//! `x` is 0, that wave is at the height of C, so a wave with a phase shift is the wave that C would draw if it had set
//! out from that angle.
//!
//! A positive shift moves the wave to the left, so it is ahead of the first, and gets to each crest sooner.

use crate::{
    circle_control::angles::{TURN, height, width},
    sine_cosine::plots::{
        HEIGHT, NAME_GAP, NAME_SIZE, POINT, SIDE, SPAN, TITLE_SIZE, TRACE_WIDTH, circle_config, circle_items, dotted,
        foot, name, point, sine_config,
    },
};
use svg_dom_graph::scene::{LabelAnchor, PlotConfig, PlotItem, PlotLabel, PlotLine, PlotPoint};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The colour of the second wave, which is not the colour of the first, and has enough contrast against the pale plot.
pub(crate) const SHIFTED: &str = "#c2410c";

/// The ink of the names of the two waves.
const FIRST_INK: &str = HEIGHT;
const SHIFTED_INK: &str = SHIFTED;

/// Where the names of the two waves are. The first line is this high, in values of the vertical axis, and the second is
/// below it. Neither wave is ever lower than -1, so the strip below that is where they are. The strip is 33 pixels
/// deep, and there are two lines in it, so they are close: 15 pixels apart, with the first 9 pixels below the line at
/// -1.
const NAMES_AT: f64 = -1.027;
const NAMES_GAP: f64 = 15.0;

/// How far the names are in from the left edge of the plot, in pixels.
const NAMES_INSET: f64 = NAME_GAP / 2.0;

/// The radius of the arc that shows the angle of C, and of the circle that its text is on, in units of the circle. The
/// arc is near the middle, and the text is outside it.
const ARC_RADIUS: f64 = 0.3;
const ARC_LABEL_RADIUS: f64 = 0.55;

/// The width of the arc, in pixels.
const ARC_WIDTH: f64 = 2.2;

/// The gap between the dotted line that shows the shift and its name, `φ`, in pixels.
const SHIFT_GAP: f64 = 12.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The circle plot: the one of the sine and cosine demo, described for this one.
pub(crate) fn phase_circle_config() -> PlotConfig {
    circle_config().with_description(
        "Unit circle with point C on it, which can be moved round it. The angle of C is the phase shift of the second \
         sine wave.",
    )
}

/// The sine plot: the one of the sine and cosine demo, described for this one.
pub(crate) fn phase_sine_config() -> PlotConfig {
    sine_config().with_description(
        "Graph of two sine waves against the angle in degrees. The first is sine of x, and does not move. The second \
         is sine of x plus the phase shift, which is the angle of point C on the circle.",
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The sine of `degrees`.
fn sine(degrees: f64) -> f64 {
    height(degrees)
}

/// The points of the wave `sin(x + shift)` at each whole degree from 0 to 360. A shift of 0 is the first wave.
fn wave(shift: f64) -> Vec<PlotPoint> {
    (0..=TURN as u32)
        .map(|x| PlotPoint::new(f64::from(x), sine(f64::from(x) + shift)))
        .collect()
}

/// Where the second wave rises through zero: the angle between 0 and 360 where `x + shift` is a whole turn. The first
/// wave does so at 360, so the distance between the two is the shift. `None` if there is no shift.
pub(crate) fn rising_zero(shift: f64) -> Option<f64> {
    (shift > 0.0 && shift < TURN).then_some(TURN - shift)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The writing of the second wave's name: `sin(x + 45°)`. A shift of 0 is written as it is.
pub(crate) fn shifted_name(shift: f64) -> String {
    format!("sin(x + {shift:.0}\u{00B0})")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The points of the arc from the positive horizontal axis to `degrees`, round the middle of the circle,
/// anti-clockwise. It goes the long way when the angle is more than 180°, so it always shows the angle that C has been
/// through, and never the smaller angle outside it.
fn arc(degrees: f64) -> Vec<PlotPoint> {
    let at = |t: f64| PlotPoint::new(ARC_RADIUS * width(t), ARC_RADIUS * height(t));
    let whole = degrees.floor() as u32;
    let mut points: Vec<PlotPoint> = (0..=whole).map(|d| at(f64::from(d))).collect();
    if degrees > f64::from(whole) {
        points.push(at(degrees));
    }
    points
}

/// What is drawn on the circle for a phase shift of `shift` degrees: what the sine and cosine demo draws, and an arc
/// near the middle that shows the angle of C, with the angle written beside it. The arc is the colour of the second
/// wave, since the angle is the shift of that wave. The text is on the line that splits the angle in two, so that it is
/// inside the angle however big it is, and for an angle of more than 180° that is on the far side of the middle from C.
pub(crate) fn phase_circle_items(shift: f64) -> Vec<PlotItem> {
    let mut items = circle_items(shift);
    items.push(PlotLine::new(arc(shift), SHIFTED).with_width(ARC_WIDTH).into());
    let half = shift / 2.0;
    let at = PlotPoint::new(ARC_LABEL_RADIUS * width(half), ARC_LABEL_RADIUS * height(half));
    items.push(name(
        &angle_text(shift),
        at,
        SHIFTED,
        LabelAnchor::Middle,
        (0.0, NAME_SIZE / 3.0),
    ));
    items
}

/// The angle of C as the text beside the arc: `φ=45°`. It is short, so that it fits between the arc and the dotted line
/// from C.
pub(crate) fn angle_text(shift: f64) -> String {
    format!("\u{03C6}={shift:.0}\u{00B0}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What is drawn on the sine plot for a phase shift of `shift` degrees, which is a whole number from 0 up to 360: the
/// two waves, C where the second starts, the shift as a dotted line between where the waves rise through zero, and the
/// names of the waves.
pub(crate) fn phase_items(shift: f64) -> Vec<PlotItem> {
    let start = PlotPoint::new(0.0, sine(shift));
    let mut items: Vec<PlotItem> = vec![
        PlotLine::new(wave(0.0), FIRST_INK).with_width(TRACE_WIDTH).into(),
        PlotLine::new(wave(shift), SHIFTED).with_width(TRACE_WIDTH).into(),
    ];
    if let Some(crossing) = rising_zero(shift) {
        let (from, to) = (PlotPoint::new(crossing, 0.0), PlotPoint::new(TURN, 0.0));
        items.push(dotted(from, to, SHIFTED));
        items.push(foot(from, SHIFTED));
        items.push(name(
            "\u{03C6}",
            PlotPoint::new((crossing + TURN) / 2.0, 0.0),
            SHIFTED,
            LabelAnchor::Middle,
            (0.0, -SHIFT_GAP),
        ));
    }
    items.push(point(start));
    items.push(name("C", start, POINT, LabelAnchor::Start, (NAME_GAP / 2.0, -NAME_GAP / 2.0)));
    items.extend(names(shift));
    items
}

/// The names of the two waves, one under the other at the bottom left, each in the colour of its wave.
fn names(shift: f64) -> Vec<PlotItem> {
    let drop = NAMES_GAP / (SIDE / (2.0 * SPAN));
    [("sin(x)", FIRST_INK), (&*shifted_name(shift), SHIFTED_INK)]
        .into_iter()
        .enumerate()
        .map(|(n, (text, ink))| {
            PlotLabel::new(PlotPoint::new(0.0, NAMES_AT - drop * n as f64), text)
                .with_colour(ink)
                .with_size(TITLE_SIZE)
                .with_anchor(LabelAnchor::Start)
                .with_offset(NAMES_INSET, TITLE_SIZE / 3.0)
                .into()
        })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
