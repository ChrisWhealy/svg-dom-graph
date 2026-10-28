use super::*;
use crate::fft::{
    algorithm::trace,
    waves::{findings, initial, samples},
};

fn found(waves: &[Wave; 3], n: usize) -> Findings {
    let spectrum = trace(&samples(waves, n)).expect("valid").states.last().expect("final").clone();
    findings(&spectrum)
}

#[test]
fn every_row_agrees_for_the_initial_waves() {
    for n in [8, 16] {
        let waves = initial();
        let findings = found(&waves, n);
        assert!((0..3).all(|i| agrees(waves[i], &findings.readings[i])), "n = {n}");
        assert!(findings.elsewhere < 1e-9);
    }
}

#[test]
fn the_table_has_a_row_for_each_wave_and_a_sentence_about_the_rest() {
    let waves = initial();
    let text = markup(&waves, &found(&waves, 8));
    assert_eq!(text.matches("<tr>").count(), 4, "a header row and three rows");
    assert_eq!(text.matches("<th scope=\"row\">").count(), 3);
    assert!(text.contains("<caption>"));
    assert!(text.contains("The transform found nothing that was not put in."), "{text}");
    assert_eq!(text.matches("Yes").count(), 3);
}

#[test]
fn a_silent_wave_shows_a_dash_and_still_agrees() {
    let waves = [Wave::new(0.5, 30.0), Wave::new(0.0, 120.0), Wave::new(0.25, 300.0)];
    let findings = found(&waves, 16);
    assert_eq!(findings.readings[1].phase, None);
    assert!(agrees(waves[1], &findings.readings[1]));
    assert!(markup(&waves, &findings).contains('\u{2014}'));
}

#[test]
fn a_wrong_answer_is_reported_as_one() {
    let waves = initial();
    let mut findings = found(&waves, 8);
    findings.readings[0].amplitude += 0.1;
    assert!(!agrees(waves[0], &findings.readings[0]));
    findings.readings[1].phase = Some(waves[1].phase + 20.0);
    assert!(!agrees(waves[1], &findings.readings[1]));
    findings.elsewhere = 0.3;
    assert!(elsewhere(findings.elsewhere).contains("more than"));
    assert!(markup(&waves, &findings).contains("No"));
}

#[test]
fn a_phase_either_side_of_the_turn_agrees() {
    let set = Wave::new(0.5, 0.0);
    let reading = Reading {
        bin: 1,
        value: demo_common::complex::Complex::ZERO,
        amplitude: 0.5,
        phase: Some(359.8),
    };
    assert!(agrees(set, &reading));
}

#[test]
fn degrees_are_whole_and_a_missing_phase_is_a_dash() {
    assert_eq!(degrees(Some(59.999)), "60\u{00B0}");
    assert_eq!(degrees(Some(359.6)), "0\u{00B0}", "a turn is not shown as 360");
    assert_eq!(degrees(None), "\u{2014}");
}

#[test]
fn the_announcement_names_each_wave_and_what_it_found() {
    let waves = initial();
    let said = announcement(&waves, &found(&waves, 8));
    assert!(
        said.starts_with("The transform found wave A at 1 cycle, amplitude 1.00, phase 0 degrees, as set;"),
        "{said}"
    );
    assert!(
        said.contains("wave B at 2 cycles, amplitude 0.60, phase 90 degrees, as set"),
        "{said}"
    );
    assert!(said.ends_with("not put in."), "{said}");
}
