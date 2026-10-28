//! The table of what the transform found, as markup. Pure: the page puts it where it belongs.
//!
//! One row for each wave, giving the amplitude and phase that were set beside the amplitude and phase that were read
//! back from the transform, and whether the two agree. Below the table, the strongest amplitude found in any bin that
//! none of the waves uses, which is zero when the transform has found everything.

use super::{
    waves::{FREQUENCIES, Findings, NAMES, Reading, Wave, apart, turn},
    words::complex,
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How far apart an amplitude set and an amplitude found can be and still agree: half the last digit shown.
const AMPLITUDE_TOLERANCE: f64 = 0.005;

/// The same for phases, which are shown in whole degrees.
const PHASE_TOLERANCE: f64 = 0.5;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Does what was found match what was set? A wave with no amplitude has no phase to find, and must not be given one.
pub(super) fn agrees(set: Wave, found: &Reading) -> bool {
    let amplitude = (set.amplitude - found.amplitude).abs() < AMPLITUDE_TOLERANCE;
    let phase = match found.phase {
        Some(phase) => set.amplitude > 0.0 && apart(phase, set.phase) < PHASE_TOLERANCE,
        None => set.amplitude < AMPLITUDE_TOLERANCE,
    };
    amplitude && phase
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The phase in whole degrees, or a dash if there is none.
pub(super) fn degrees(phase: Option<f64>) -> String {
    phase.map_or_else(|| "\u{2014}".to_owned(), |p| format!("{:.0}\u{00B0}", turn(p.round())))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The sentence below the table.
pub(super) fn elsewhere(amplitude: f64) -> String {
    format!(
        "The largest amplitude in any other frequency: {amplitude:.2}. {}",
        if amplitude < AMPLITUDE_TOLERANCE {
            "The transform found nothing that was not put in."
        } else {
            "That is more than the three waves explain."
        }
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The table and the sentence below it, for the `waves` that were set and what was `found` in the transform of them.
pub(super) fn markup(waves: &[Wave; 3], found: &Findings) -> String {
    let mut rows = String::new();
    for index in 0..3 {
        let (set, reading) = (waves[index], &found.readings[index]);
        rows.push_str(&format!(
            "<tr><th scope=\"row\">{}</th><td>{}</td><td>{}</td><td>{:.2}</td><td>{:.2}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            NAMES[index],
            FREQUENCIES[index],
            complex(reading.value),
            set.amplitude,
            reading.amplitude,
            degrees(Some(set.phase)),
            degrees(reading.phase),
            if agrees(set, reading) { "Yes" } else { "No" }
        ));
    }
    format!(
        "<table class=\"fft-findings\"><caption>What the transform found, beside what was set</caption>\
         <thead><tr><th scope=\"col\">Wave</th><th scope=\"col\">Cycles</th><th scope=\"col\">Transform output</th>\
         <th scope=\"col\">Amplitude set</th><th scope=\"col\">Amplitude found</th>\
         <th scope=\"col\">Phase set</th><th scope=\"col\">Phase found</th><th scope=\"col\">Agree?</th></tr></thead>\
         <tbody>{rows}</tbody></table><p>{}</p>",
        elsewhere(found.elsewhere)
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One sentence for a screen reader, said when a slider is let go: what the transform found.
pub(super) fn announcement(waves: &[Wave; 3], found: &Findings) -> String {
    let each: Vec<String> = (0..3)
        .map(|index| {
            let reading = &found.readings[index];
            let phase = reading
                .phase
                .map_or_else(|| "no phase".to_owned(), |p| format!("phase {p:.0} degrees"));
            format!(
                "wave {} at {} cycle{}, amplitude {:.2}, {phase}, {}",
                NAMES[index],
                FREQUENCIES[index],
                if FREQUENCIES[index] == 1 { "" } else { "s" },
                reading.amplitude,
                if agrees(waves[index], reading) { "as set" } else { "not as set" }
            )
        })
        .collect();
    format!("The transform found {}. {}", each.join("; "), elsewhere(found.elsewhere))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
