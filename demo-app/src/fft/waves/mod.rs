//! Three sine waves, their sum, and what a transform of the sum says about them. Pure: nothing here touches the DOM.
//!
//! Wave A completes one cycle in the window, wave B two, and wave C three. Each has an amplitude and a phase. The phase
//! is how far along its cycle the wave is at the left edge, so a phase of 90° starts at the crest. The demo samples the
//! sum at `N` evenly spaced times, and the transform of those samples says how much of each frequency is present.
//!
//! A wave `a·sin(2πkn/N + φ)` has `X[k] = (N·a/2)·e^(i(φ − 90°))`. So the amplitude is `2·|X[k]|/N`, and the phase is
//! the angle of `X[k]` plus 90°. [`read`] undoes the transform that way.

use demo_common::complex::Complex;
use std::f64::consts::{FRAC_PI_2, TAU};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many cycles each wave completes in the window: the bins of the transform where they appear.
pub(super) const FREQUENCIES: [usize; 3] = [1, 2, 3];

/// The names of the waves, in the same order.
pub(super) const NAMES: [&str; 3] = ["A", "B", "C"];

/// The largest amplitude a slider offers. The plot is tall enough for all three at their largest.
pub(super) const MAX_AMPLITUDE: f64 = 1.0;

/// An amplitude below this is too small to show at two decimals, so the phase of such a wave is not worth reporting.
const SILENT: f64 = 0.0005;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One wave: how tall it is, and where in its cycle it starts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Wave {
    pub(super) amplitude: f64,
    /// In degrees, from 0 up to but not including 360.
    pub(super) phase: f64,
}

impl Wave {
    /// A wave whose amplitude is held between 0 and [`MAX_AMPLITUDE`] and whose phase is turned into `[0, 360)`. A part
    /// that is not a number becomes zero.
    pub(super) fn new(amplitude: f64, phase: f64) -> Self {
        let amplitude = if amplitude.is_finite() { amplitude.clamp(0.0, MAX_AMPLITUDE) } else { 0.0 };
        let phase = if phase.is_finite() { turn(phase) } else { 0.0 };
        Self { amplitude, phase }
    }
}

/// The waves the demo starts with.
pub(super) fn initial() -> [Wave; 3] {
    [Wave::new(1.0, 0.0), Wave::new(0.6, 90.0), Wave::new(0.4, 210.0)]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// An angle in degrees, turned into `[0, 360)`. Rounding first stops a value a hair below 360 from showing as 360, and
/// adding zero stops a negative zero from showing as one.
pub(super) fn turn(degrees: f64) -> f64 {
    let rounded = (degrees * 1e6).round() / 1e6;
    let turned = rounded.rem_euclid(360.0);
    if turned < 360.0 { turned + 0.0 } else { 0.0 }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The shortest angle between two phases, in degrees: 350° and 10° are 20° apart.
pub(super) fn apart(a: f64, b: f64) -> f64 {
    let difference = (a - b).rem_euclid(360.0);
    difference.min(360.0 - difference)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The wave at time `t`, where the window runs from 0 to 1.
pub(super) fn wave_at(wave: Wave, frequency: usize, t: f64) -> f64 {
    wave.amplitude * (TAU * frequency as f64 * t + wave.phase.to_radians()).sin()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The sum of the three waves at time `t`.
pub(super) fn sum_at(waves: &[Wave; 3], t: f64) -> f64 {
    waves
        .iter()
        .zip(FREQUENCIES)
        .map(|(wave, frequency)| wave_at(*wave, frequency, t))
        .sum()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `n` samples of the sum, taken at `t = 0, 1/n, 2/n …`, as complex numbers with no imaginary part.
pub(super) fn samples(waves: &[Wave; 3], n: usize) -> Vec<Complex> {
    (0..n).map(|i| Complex::from_real(sum_at(waves, i as f64 / n as f64))).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the transform says about one frequency.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Reading {
    pub(super) bin: usize,
    /// The transform's own number for the bin.
    pub(super) value: Complex,
    pub(super) amplitude: f64,
    /// In degrees, in `[0, 360)`, or `None` if the amplitude is too small for a phase to mean anything.
    pub(super) phase: Option<f64>,
}

/// What the `spectrum` of `n = spectrum.len()` samples says about the sine wave in `bin`.
pub(super) fn read(spectrum: &[Complex], bin: usize) -> Reading {
    let value = spectrum[bin];
    let amplitude = 2.0 * value.norm() / spectrum.len() as f64;
    let phase = (amplitude >= SILENT).then(|| turn((value.arg() + FRAC_PI_2).to_degrees()));
    Reading { bin, value, amplitude, phase }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The readings of the three frequencies, and the strongest amplitude found anywhere else.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Findings {
    pub(super) readings: [Reading; 3],
    /// The largest amplitude in a bin that none of the waves uses, up to and including the half-way bin. The bins above
    /// it only mirror the bins below.
    pub(super) elsewhere: f64,
}

/// The findings in the `spectrum` of a transform of at least eight samples.
pub(super) fn findings(spectrum: &[Complex]) -> Findings {
    let n = spectrum.len();
    let readings = FREQUENCIES.map(|bin| read(spectrum, bin));
    let elsewhere = (0..=n / 2)
        .filter(|bin| !FREQUENCIES.contains(bin))
        .map(|bin| {
            // The first and half-way bins have no mirror, so their share is not doubled.
            let share = if bin == 0 || bin == n / 2 { 1.0 } else { 2.0 };
            share * spectrum[bin].norm() / n as f64
        })
        .fold(0.0, f64::max);
    Findings { readings, elsewhere }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
