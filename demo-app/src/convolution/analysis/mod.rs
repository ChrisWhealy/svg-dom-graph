//! What the convolution demo works out: a signal made of two sine waves, how much of a chosen frequency is in it, and
//! the same for every frequency. Pure: nothing here touches the DOM.
//!
//! To find out how much of a frequency `f` a signal holds, multiply the signal by a cosine and by a sine of that
//! frequency, and add up the result over the time that is looked at, which is the integral. Where the signal rises and
//! falls with the probe, the product is mostly above the axis (or mostly below it) and the integral is large. Where it
//! does not, the product is as much below the axis as above it, and the integral is nothing.
//!
//! For a wave `A · sin(2π · f · t + φ)` that is looked at for a whole number of cycles, the integral of its product with
//! the cosine is `A/2 · sin φ`, and the integral of its product with the sine is `A/2 · cos φ`, since the average of a
//! sine squared is a half. Twice each integral, `a` and `b`, is therefore `A · sin φ` and `A · cos φ`, and together they
//! give the amplitude and the phase. The factor of 2 is `2/T` for a window of `T` = 1 second.
//!
//! Zero frequency is a special case. The cosine probe is 1 and the sine probe is 0, so the integral is the mean of the
//! signal, and the amplitude is that mean, not twice it: the factor is 1 and not `2/T`. It is the size of the constant
//! that the signal is offset by, which is not a wave, so there is no phase. In the transform, the bin for it is
//! `X = N · a`, and the others are `X = N/2 · (a − i·b)`.
//!
//! Strictly this is a correlation, or a projection of the signal onto the probe, and not a convolution: nothing is
//! slid or flipped, and each frequency gives one number.
//!
//! The conventions are those of the Fast Fourier Transform demo: frequencies are in cycles for each second, so the
//! angle is `2π · f · t`, a wave is `A · sin(2π · f · t + φ)` so that a positive phase moves it to the left, and the
//! forward transform has `e^(−2πi·f·t)` in it. So the bin of the transform for the frequency `f` of the `N` samples of
//! the window is `X = N/2 · (a − i·b)`, whose size is `N/2 · A` and whose angle is `φ − 90°`.

use std::f64::consts::TAU;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How long the signal is looked at, in seconds.
pub(crate) const SECONDS: f64 = 1.0;

/// How many straight segments a curve of the signal, or of a product, is made of. At 10 Hz that is 51 for each cycle.
pub(crate) const SEGMENTS: usize = 512;

/// The least and the greatest frequency of the probe, in Hz, and the distance between the frequencies that it can take.
pub(crate) const FREQUENCIES: (f64, f64) = (0.0, 10.0);
pub(crate) const FREQUENCY_STEP: f64 = 0.1;

/// The least amplitude that is taken to be a wave in the signal, and not what is left of the waves that are not there,
/// so that a phase is only said for something.
pub(crate) const MINIMUM_AMPLITUDE: f64 = 0.02;

/// The frequency of the probe that the page starts with, which is that of the first wave in the signal.
pub(crate) const FIRST_PROBE: f64 = 3.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One sine wave of the signal: `amplitude · sin(2π · frequency · t + phase)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Component {
    /// In Hz.
    pub(crate) frequency: f64,
    pub(crate) amplitude: f64,
    /// In degrees.
    pub(crate) phase: f64,
}

/// The two waves that the signal is made of. Their frequencies are whole numbers, so that each makes a whole number of
/// cycles in the second that is looked at.
pub(crate) const COMPONENTS: [Component; 2] = [
    Component {
        frequency: 2.0,
        amplitude: 1.0,
        phase: 40.0,
    },
    Component {
        frequency: 5.0,
        amplitude: 0.6,
        phase: 110.0,
    },
];

/// The signal at `seconds`: the sum of its waves.
pub(crate) fn signal(seconds: f64) -> f64 {
    COMPONENTS
        .iter()
        .map(|c| c.amplitude * (TAU * c.frequency * seconds + c.phase.to_radians()).sin())
        .sum()
}

/// The time of sample `i`, in seconds.
pub(crate) fn seconds(i: usize) -> f64 {
    SECONDS * i as f64 / SEGMENTS as f64
}

/// The signal at each of the [`SEGMENTS`] + 1 times of [`seconds`].
pub(crate) fn samples() -> Vec<f64> {
    (0..=SEGMENTS).map(|i| signal(seconds(i))).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The two waves that the signal is probed with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Probe {
    Cosine,
    Sine,
}

impl Probe {
    /// The height of the probe of `frequency` Hz at `seconds`.
    pub(crate) fn at(self, frequency: f64, seconds: f64) -> f64 {
        let angle = TAU * frequency * seconds;
        match self {
            Self::Cosine => angle.cos(),
            Self::Sine => angle.sin(),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The area between a curve and the axis, apart for the part above the axis and the part below it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Areas {
    /// Not below 0.
    pub(crate) above: f64,
    /// Not above 0: the area below the axis counts against the integral.
    pub(crate) below: f64,
}

impl Areas {
    /// The integral: what is above the axis, less what is below it.
    pub(crate) fn net(&self) -> f64 {
        self.above + self.below
    }
}

/// The areas between the axis and the line that goes straight through `heights`, which are spread evenly over
/// [`SECONDS`]. A segment that crosses the axis counts for each side as far as it goes on that side.
pub(crate) fn areas(heights: &[f64]) -> Areas {
    let width = SECONDS / (heights.len() - 1) as f64;
    let mut areas = Areas { above: 0.0, below: 0.0 };
    let mut add = |area: f64| {
        if area >= 0.0 {
            areas.above += area;
        } else {
            areas.below += area;
        }
    };
    for pair in heights.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        // The signs are compared, since the product of two small heights is zero.
        if (a > 0.0 && b < 0.0) || (a < 0.0 && b > 0.0) {
            // The segment crosses at this fraction of the way along it, and is a triangle on each side.
            let crossing = a / (a - b);
            add(a * crossing * width / 2.0);
            add(b * (1.0 - crossing) * width / 2.0);
        } else {
            add((a + b) / 2.0 * width);
        }
    }
    areas
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The signal multiplied by one probe, and the areas under that.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Product {
    /// The height at each of the times of [`seconds`].
    pub(crate) heights: Vec<f64>,
    pub(crate) areas: Areas,
}

impl Product {
    /// The product of `samples`, which are of the signal, with `probe` of `frequency` Hz.
    pub(crate) fn new(samples: &[f64], probe: Probe, frequency: f64) -> Self {
        let heights: Vec<f64> = samples
            .iter()
            .enumerate()
            .map(|(i, signal)| signal * probe.at(frequency, seconds(i)))
            .collect();
        let areas = areas(&heights);
        Self { heights, areas }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How much of one frequency is in the signal: its two products, and what they say.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Finding {
    pub(crate) frequency: f64,
    pub(crate) cosine: Product,
    pub(crate) sine: Product,
}

impl Finding {
    /// Probes `samples` with the cosine and the sine of `frequency` Hz.
    pub(crate) fn new(samples: &[f64], frequency: f64) -> Self {
        Self {
            frequency,
            cosine: Product::new(samples, Probe::Cosine, frequency),
            sine: Product::new(samples, Probe::Sine, frequency),
        }
    }

    /// Whether this is zero frequency, which is a special case. The cosine probe is 1 all the time and the sine probe is
    /// 0 all the time, so the integral of the product with the cosine is the mean of the signal, and the product with
    /// the sine is nothing. What is found is the constant that the signal is offset by, which is not a wave.
    pub(crate) fn is_constant(&self) -> bool {
        self.frequency == 0.0
    }

    /// What the integrals are multiplied by to find the amplitude: 2 for a wave, which is `2/T` for a window of `T` = 1
    /// second, and 1 for a constant. A wave is half as tall as the integral of its product with the probe, since the
    /// average of a sine squared is a half. A constant is not, since the average of 1 squared is 1.
    pub(crate) fn factor(&self) -> f64 {
        if self.is_constant() { 1.0 } else { 2.0 }
    }

    /// The integral of the product with the cosine, times [`Finding::factor`]. For a wave of that frequency it is
    /// `A · sin φ`, and for a constant it is the constant. The integral on its own is only half of the first.
    pub(crate) fn a(&self) -> f64 {
        self.factor() * self.cosine.areas.net()
    }

    /// The integral of the product with the sine, times [`Finding::factor`]. For a wave of that frequency it is
    /// `A · cos φ`, and for a constant it is nothing.
    pub(crate) fn b(&self) -> f64 {
        self.factor() * self.sine.areas.net()
    }

    /// How tall the wave of that frequency is in the signal, or for zero frequency how far the signal is from zero on
    /// average, which is the size of its constant part.
    pub(crate) fn amplitude(&self) -> f64 {
        self.a().hypot(self.b())
    }

    /// How far along its cycle that wave is at time 0, in degrees from 0 up to but not including 360. Only meaningful
    /// where [`Finding::has_phase`] is true.
    pub(crate) fn phase(&self) -> f64 {
        self.a().atan2(self.b()).to_degrees().rem_euclid(360.0)
    }

    /// Whether there is a wave of that frequency in the signal, to have a phase. A wave of no height has none, and a
    /// constant is not a wave, so has none either.
    pub(crate) fn has_phase(&self) -> bool {
        !self.is_constant() && self.amplitude() >= MINIMUM_AMPLITUDE
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many frequencies the spectrum has: every [`FREQUENCY_STEP`] from the least to the greatest, inclusive.
pub(crate) fn spectrum_len() -> usize {
    ((FREQUENCIES.1 - FREQUENCIES.0) / FREQUENCY_STEP).round() as usize + 1
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The frequency of the `n`th slider position, in Hz, without the error of adding a tenth `n` times.
pub(crate) fn frequency(n: usize) -> f64 {
    FREQUENCIES.0 + n as f64 * FREQUENCY_STEP
}

/// The amplitude found for every frequency of the slider: the frequency and the amplitude.
pub(crate) fn spectrum(samples: &[f64]) -> Vec<(f64, f64)> {
    (0..spectrum_len())
        .map(|n| {
            let f = frequency(n);
            (f, Finding::new(samples, f).amplitude())
        })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
