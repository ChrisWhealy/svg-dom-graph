//! Where an axis's ticks are and what they say. Pure.

use super::{AxisScale, MinorTicks, TickLabels, config::PlotAxis};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One axis tick
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tick {
    pub(crate) value: f64,
    /// A major tick has a grid line, a tick mark and perhaps a label. A minor tick has only a lighter grid line.
    pub(crate) major: bool,
    pub(crate) label: Option<String>,
}

/// How many major intervals an automatic interval aims for.
const TARGET_INTERVALS: f64 = 6.0;

/// What a very small span is multiplied by to be worked on: two to the 600th power.
const SMALL_SPAN_SCALE: f64 = 4.149_515_568_880_993e180;

/// A tolerance, as a fraction of an interval, for deciding that a tick is at the end of an axis and not just past it.
const EDGE_TOLERANCE: f64 = 1e-9;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A round interval, 1, 2 or 5 times a power of ten, that makes about six of `span`.
pub(crate) fn nice_interval(span: f64) -> f64 {
    // A span so small that a sixth of it is subnormal has too few digits left to round. It is worked out as a bigger
    // one, by a power of two, which is both exact and scaled back.
    if span > 0.0 && span / TARGET_INTERVALS < f64::MIN_POSITIVE {
        return nice_interval(span * SMALL_SPAN_SCALE) / SMALL_SPAN_SCALE;
    }
    let raw = span / TARGET_INTERVALS;
    let power = 10f64.powf(raw.log10().floor());
    let fraction = raw / power;
    let rounded = if fraction < 1.5 {
        1.0
    } else if fraction < 3.0 {
        2.0
    } else if fraction < 7.0 {
        5.0
    } else {
        10.0
    };
    rounded * power
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The major interval of `axis`: the one it was given, or a round one.
fn interval(axis: &PlotAxis) -> f64 {
    axis.major_interval.unwrap_or_else(|| match axis.scale {
        AxisScale::Linear => nice_interval(axis.max - axis.min),
        AxisScale::Logarithmic => 1.0,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// About how many ticks `axis` has, worked out without making them, so that an absurd interval can be refused before it
/// is used. It never under-counts.
pub(crate) fn count_estimate(axis: &PlotAxis) -> usize {
    if let TickLabels::Custom(ticks) = &axis.tick_labels {
        return ticks.len();
    }
    let step = interval(axis);
    let (span, per_step) = match axis.scale {
        AxisScale::Linear => (axis.max - axis.min, step),
        AxisScale::Logarithmic => (axis.max.log10() - axis.min.log10(), step),
    };
    let majors = (span / per_step).ceil() + 1.0;
    let minors = match axis.minor {
        MinorTicks::None => 0.0,
        MinorTicks::Divisions(n) => majors * f64::from(n),
        MinorTicks::Decade => (span.ceil() + 1.0) * 8.0,
    };
    let total = majors + minors;
    if total.is_finite() && total < 1e9 { total as usize } else { usize::MAX }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A number as a tick label: at most six decimals with the zeros trimmed, a true minus sign, and an exponent for very
/// large or very small numbers.
pub(crate) fn format_number(value: f64) -> String {
    if value == 0.0 || !value.is_finite() {
        return "0".to_owned();
    }
    let magnitude = value.abs();
    let text = if !(1e-4..1e6).contains(&magnitude) {
        format!("{value:e}")
    } else {
        let fixed = format!("{value:.6}");
        fixed.trim_end_matches('0').trim_end_matches('.').to_owned()
    };
    if text == "-0" { "0".to_owned() } else { text.replace('-', "\u{2212}") }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether `value` is on the axis, allowing for rounding at either end.
fn within(axis: &PlotAxis, value: f64) -> bool {
    let slack = match axis.scale {
        AxisScale::Linear => (axis.max - axis.min) * EDGE_TOLERANCE,
        AxisScale::Logarithmic => 0.0,
    };
    let relative = match axis.scale {
        AxisScale::Linear => 0.0,
        AxisScale::Logarithmic => EDGE_TOLERANCE,
    };
    value >= axis.min - slack - axis.min.abs() * relative && value <= axis.max + slack + axis.max.abs() * relative
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The label of a major tick at `value`.
fn label_for(axis: &PlotAxis, value: f64) -> Option<String> {
    match axis.tick_labels {
        TickLabels::Auto => Some(format!("{}{}", format_number(value), axis.tick_suffix.as_deref().unwrap_or(""))),
        TickLabels::None | TickLabels::Custom(_) => None,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every tick of `axis`, from the least value to the greatest. The ticks of a configuration that has been checked.
pub(crate) fn ticks(axis: &PlotAxis) -> Vec<Tick> {
    if let TickLabels::Custom(custom) = &axis.tick_labels {
        let mut ticks: Vec<Tick> = custom
            .iter()
            .filter(|(value, _)| within(axis, *value))
            .map(|(value, text)| Tick {
                value: *value,
                major: true,
                label: Some(text.clone()),
            })
            .collect();
        ticks.sort_by(|a, b| a.value.total_cmp(&b.value));
        return ticks;
    }
    let mut ticks = match axis.scale {
        AxisScale::Linear => linear_ticks(axis),
        AxisScale::Logarithmic => logarithmic_ticks(axis),
    };
    ticks.sort_by(|a, b| a.value.total_cmp(&b.value));
    ticks
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `multiple` times `step`, with the rounding error of the multiplication taken out: `0.1 * 3.0` is `0.3`.
///
/// The product is kept to 15 significant digits of its own, which is the most a double has to give. So the artefacts of
/// the multiplication in the 16th and 17th digits go, and no two values that a double can tell apart are made the same,
/// however small they are. A cleanup to a fixed number of decimal places would do that, and would make the ticks of a very
/// small axis collapse onto one another.
fn clean(multiple: f64, step: f64) -> f64 {
    let value = multiple * step;
    if value == 0.0 || !value.is_finite() {
        return value + 0.0;
    }
    // Writing a number to 15 digits and reading it back never overflows, however large or small the number is.
    format!("{value:.14e}").parse::<f64>().map_or(value, |rounded| rounded + 0.0)
}

/// [`clean`], for the tests of this module's neighbours.
#[cfg(test)]
pub(crate) fn clean_for_test(multiple: f64, step: f64) -> f64 {
    clean(multiple, step)
}

fn linear_ticks(axis: &PlotAxis) -> Vec<Tick> {
    let step = interval(axis);
    let first = (axis.min / step).floor() as i64 - 1;
    let last = (axis.max / step).ceil() as i64 + 1;
    let divisions = match axis.minor {
        MinorTicks::Divisions(n) => n,
        MinorTicks::None | MinorTicks::Decade => 1,
    };
    let mut ticks = Vec::new();
    for k in first..=last {
        let major = clean(k as f64, step);
        if within(axis, major) {
            ticks.push(Tick {
                value: major,
                major: true,
                label: label_for(axis, major),
            });
        }
        for j in 1..divisions {
            let minor = clean(k as f64 + f64::from(j) / f64::from(divisions), step);
            if within(axis, minor) {
                ticks.push(Tick {
                    value: minor,
                    major: false,
                    label: None,
                });
            }
        }
    }
    ticks
}

fn logarithmic_ticks(axis: &PlotAxis) -> Vec<Tick> {
    let step = interval(axis);
    let (low, high) = (axis.min.log10(), axis.max.log10());
    let mut ticks = Vec::new();
    let first = (low / step).floor() as i64 - 1;
    let last = (high / step).ceil() as i64 + 1;
    for m in first..=last {
        let value = 10f64.powf(m as f64 * step);
        // A power of ten computed this way can differ from the whole number in its last digit.
        let value = if (value.log10() - value.log10().round()).abs() < 1e-9 {
            10f64.powi(value.log10().round() as i32)
        } else {
            value
        };
        if within(axis, value) {
            ticks.push(Tick {
                value,
                major: true,
                label: label_for(axis, value),
            });
        }
    }
    if axis.minor == MinorTicks::Decade {
        for decade in (low.floor() as i32 - 1)..=(high.ceil() as i32 + 1) {
            for multiple in 2..=9 {
                let value = f64::from(multiple) * 10f64.powi(decade);
                let on_a_major = ticks.iter().any(|t| t.major && (t.value / value - 1.0).abs() < 1e-9);
                if within(axis, value) && !on_a_major {
                    ticks.push(Tick {
                        value,
                        major: false,
                        label: None,
                    });
                }
            }
        }
    }
    ticks
}
