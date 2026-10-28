use super::*;
use crate::fft::algorithm::trace;

/// The phases the sliders offer are whole steps of this many degrees.
const PHASE_STEP: f64 = 15.0;

fn spectrum_of(waves: &[Wave; 3], n: usize) -> Vec<Complex> {
    trace(&samples(waves, n))
        .expect("a valid input")
        .states
        .last()
        .expect("a final state")
        .clone()
}

#[test]
fn a_wave_is_read_back_with_the_amplitude_and_phase_it_was_given() {
    for n in [8, 16] {
        for step in 0..24 {
            let phase = f64::from(step) * PHASE_STEP;
            let waves = [
                Wave::new(1.0, phase),
                Wave::new(0.6, phase + 45.0),
                Wave::new(0.35, 360.0 - phase),
            ];
            let found = findings(&spectrum_of(&waves, n));
            for (wave, reading) in waves.iter().zip(found.readings) {
                assert!((reading.amplitude - wave.amplitude).abs() < 1e-9, "n = {n}, phase {phase}");
                let got = reading.phase.expect("a wave that is there has a phase");
                assert!(apart(got, wave.phase) < 1e-6, "n = {n}: {got} against {}", wave.phase);
            }
            assert!(found.elsewhere < 1e-9, "n = {n}: {}", found.elsewhere);
        }
    }
}

#[test]
fn a_silent_wave_has_no_phase_and_the_others_are_unaffected() {
    let waves = [Wave::new(0.0, 120.0), Wave::new(0.5, 30.0), Wave::new(0.0, 0.0)];
    let found = findings(&spectrum_of(&waves, 8));
    assert_eq!(found.readings[0].phase, None);
    assert_eq!(found.readings[2].phase, None);
    assert!(apart(found.readings[1].phase.expect("present"), 30.0) < 1e-6);
}

#[test]
fn the_phase_is_where_the_wave_stands_at_the_left_edge() {
    // A phase of 90° starts at the crest, and 270° at the trough.
    assert!((wave_at(Wave::new(1.0, 90.0), 1, 0.0) - 1.0).abs() < 1e-12);
    assert!((wave_at(Wave::new(1.0, 270.0), 2, 0.0) + 1.0).abs() < 1e-12);
    assert!(wave_at(Wave::new(1.0, 0.0), 3, 0.0).abs() < 1e-12);
}

#[test]
fn the_sum_is_the_three_waves_added_and_each_wave_has_its_own_frequency() {
    let waves = initial();
    for t in [0.0, 0.1, 0.37, 0.9] {
        let by_hand: f64 = (0..3).map(|i| wave_at(waves[i], FREQUENCIES[i], t)).sum();
        assert!((sum_at(&waves, t) - by_hand).abs() < 1e-12);
    }
    // One cycle of A fits the window exactly, and B and C fit whole cycles too.
    assert!((wave_at(Wave::new(1.0, 40.0), 1, 1.0) - wave_at(Wave::new(1.0, 40.0), 1, 0.0)).abs() < 1e-9);
    assert!((wave_at(Wave::new(1.0, 40.0), 3, 1.0) - wave_at(Wave::new(1.0, 40.0), 3, 0.0)).abs() < 1e-9);
}

#[test]
fn the_samples_are_real_and_taken_at_even_times() {
    let waves = initial();
    let got = samples(&waves, 8);
    assert_eq!(got.len(), 8);
    for (i, z) in got.iter().enumerate() {
        assert_eq!(z.im, 0.0);
        assert!((z.re - sum_at(&waves, i as f64 / 8.0)).abs() < 1e-12);
    }
}

#[test]
fn the_ranges_hold_whatever_is_given() {
    assert_eq!(Wave::new(5.0, 0.0).amplitude, MAX_AMPLITUDE);
    assert_eq!(Wave::new(-1.0, 0.0).amplitude, 0.0);
    assert_eq!(Wave::new(f64::NAN, f64::INFINITY), Wave { amplitude: 0.0, phase: 0.0 });
    assert_eq!(Wave::new(0.5, 360.0).phase, 0.0);
    assert_eq!(Wave::new(0.5, -15.0).phase, 345.0);
    assert_eq!(turn(359.999_999_9), 0.0);
}

#[test]
fn a_bin_of_nothing_is_the_elsewhere_amplitude() {
    // A wave at bin 4 of 16 is not one of the three, so it must show up there and nowhere else.
    let n = 16;
    let input: Vec<Complex> = (0..n)
        .map(|i| Complex::from_real(0.5 * (TAU * 4.0 * i as f64 / n as f64).sin()))
        .collect();
    let spectrum = trace(&input).expect("valid").states.last().expect("final").clone();
    let found = findings(&spectrum);
    assert!((found.elsewhere - 0.5).abs() < 1e-9);
    assert!(found.readings.iter().all(|r| r.amplitude < 1e-9));
}

#[test]
fn a_constant_offset_counts_as_elsewhere_without_being_doubled() {
    let spectrum = vec![Complex::from_real(8.0 * 0.25); 1]
        .into_iter()
        .chain(vec![Complex::ZERO; 7])
        .collect::<Vec<_>>();
    assert!((findings(&spectrum).elsewhere - 0.25).abs() < 1e-12);
}

#[test]
fn phases_are_apart_by_the_short_way_round() {
    assert!((apart(350.0, 10.0) - 20.0).abs() < 1e-12);
    assert!((apart(10.0, 350.0) - 20.0).abs() < 1e-12);
    assert_eq!(apart(90.0, 90.0), 0.0);
    assert_eq!(apart(0.0, 180.0), 180.0);
}

#[test]
fn a_hair_below_zero_is_zero_and_not_a_negative_zero() {
    assert!(turn(-1e-12).is_sign_positive());
    assert_eq!(format!("{:.0}", turn(-1e-12)), "0");
}
