//! The signal, the two products, the areas, and what they say about the signal.

use super::*;

fn near(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

#[test]
fn the_signal_is_the_sum_of_its_two_waves() {
    let t = 0.123;
    let by_hand = (TAU * 2.0 * t + 40.0_f64.to_radians()).sin() + 0.6 * (TAU * 5.0 * t + 110.0_f64.to_radians()).sin();
    assert!(near(signal(t), by_hand, 1e-12));
    assert_eq!(samples().len(), SEGMENTS + 1);
    assert!(near(seconds(SEGMENTS), SECONDS, 1e-12));
}

#[test]
fn a_flat_curve_above_the_axis_has_that_area_above_and_none_below() {
    let found = areas(&[0.5; 11]);
    assert!(near(found.above, 0.5, 1e-12) && found.below == 0.0, "{found:?}");
    let found = areas(&[-0.25; 11]);
    assert!(found.above == 0.0 && near(found.below, -0.25, 1e-12), "{found:?}");
}

#[test]
fn a_segment_that_crosses_the_axis_counts_for_each_side_as_far_as_it_goes() {
    // From 1 to -1 over the whole second: a triangle of 1/4 above, and one of 1/4 below.
    let found = areas(&[1.0, -1.0]);
    assert!(near(found.above, 0.25, 1e-12) && near(found.below, -0.25, 1e-12), "{found:?}");
    assert!(near(found.net(), 0.0, 1e-12));
    // From 3 to -1: the crossing is three quarters of the way, so 3 · 3/4 / 2 above, and -1 · 1/4 / 2 below.
    let found = areas(&[3.0, -1.0]);
    assert!(near(found.above, 1.125, 1e-12) && near(found.below, -0.125, 1e-12), "{found:?}");
}

#[test]
fn a_whole_cycle_of_a_sine_is_as_much_below_the_axis_as_above() {
    let wave: Vec<f64> = (0..=SEGMENTS).map(|i| (TAU * seconds(i)).sin()).collect();
    let found = areas(&wave);
    assert!(near(found.above, 1.0 / std::f64::consts::PI, 1e-4), "{found:?}");
    assert!(near(found.below, -1.0 / std::f64::consts::PI, 1e-4), "{found:?}");
    assert!(near(found.net(), 0.0, 1e-9));
}

#[test]
fn a_probe_of_the_frequency_of_a_wave_finds_its_amplitude_and_its_phase() {
    let samples = samples();
    for c in COMPONENTS {
        let found = Finding::new(&samples, c.frequency);
        assert!(
            near(found.amplitude(), c.amplitude, 1e-3),
            "{} Hz: {}",
            c.frequency,
            found.amplitude()
        );
        assert!(near(found.phase(), c.phase, 0.1), "{} Hz: {}", c.frequency, found.phase());
        assert!(near(found.a(), c.amplitude * c.phase.to_radians().sin(), 1e-3));
        assert!(near(found.b(), c.amplitude * c.phase.to_radians().cos(), 1e-3));
    }
}

#[test]
fn a_probe_of_a_whole_frequency_that_is_not_in_the_signal_finds_nothing() {
    let samples = samples();
    for f in [1.0, 3.0, 4.0, 6.0, 7.0, 8.0, 9.0, 10.0] {
        let found = Finding::new(&samples, f);
        assert!(found.amplitude() < 1e-3, "{f} Hz: {}", found.amplitude());
    }
}

#[test]
fn a_probe_of_a_frequency_between_the_waves_finds_some_but_less_than_the_wave() {
    let found = Finding::new(&samples(), 2.5);
    assert!(found.amplitude() > 0.05 && found.amplitude() < 1.0, "{}", found.amplitude());
}

#[test]
fn the_cosine_probe_alone_does_not_say_the_phase_but_the_pair_does() {
    // A cosine is a sine that is 90 degrees along, so the signal that is only a cosine has no sine in it.
    let cosine: Vec<f64> = (0..=SEGMENTS).map(|i| (TAU * 3.0 * seconds(i)).cos()).collect();
    let found = Finding::new(&cosine, 3.0);
    assert!(near(found.a(), 1.0, 1e-3) && near(found.b(), 0.0, 1e-3));
    assert!(near(found.phase(), 90.0, 0.1), "{}", found.phase());
}

#[test]
fn the_product_is_the_signal_times_the_probe_at_every_sample() {
    let samples = samples();
    let product = Product::new(&samples, Probe::Sine, 2.0);
    assert_eq!(product.heights.len(), samples.len());
    for i in [0, 1, 100, 511, 512] {
        assert!(near(product.heights[i], samples[i] * (TAU * 2.0 * seconds(i)).sin(), 1e-12));
    }
    assert!(near(product.areas.net(), areas(&product.heights).net(), 1e-12));
}

#[test]
fn the_spectrum_has_a_peak_at_each_wave_and_the_slider_steps_land_on_whole_frequencies() {
    let found = spectrum(&samples());
    assert_eq!(found.len(), 101);
    assert!(near(found[0].0, 0.0, 1e-12) && near(found[100].0, 10.0, 1e-12));
    let at = |f: f64| found.iter().find(|(x, _)| near(*x, f, 1e-9)).map(|&(_, a)| a);
    assert!(near(at(2.0).unwrap_or(f64::NAN), 1.0, 1e-3));
    assert!(near(at(5.0).unwrap_or(f64::NAN), 0.6, 1e-3));
    assert!(at(3.0).is_some_and(|a| a < 1e-3));
    // The two peaks are the only places that are higher than both their neighbours, and well above nothing.
    let peaks: Vec<f64> = found
        .windows(3)
        .filter(|w| w[1].1 > w[0].1 && w[1].1 > w[2].1 && w[1].1 > 0.3)
        .map(|w| w[1].0)
        .collect();
    assert_eq!(peaks.len(), 2, "{peaks:?}");
    // The leakage of each wave adds to the other, so a peak can be a step away from the frequency of its wave.
    assert!(near(peaks[0], 2.0, 0.15) && near(peaks[1], 5.0, 0.15), "{peaks:?}");
}

#[test]
fn a_phase_is_never_negative_nor_a_whole_turn() {
    let samples = samples();
    for n in 0..spectrum_len() {
        let phase = Finding::new(&samples, frequency(n)).phase();
        assert!((0.0..360.0).contains(&phase), "{phase}");
    }
}
