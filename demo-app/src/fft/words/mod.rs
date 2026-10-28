//! Every sentence the FFT panel says that is not a node's own label. Pure, so each can be tested without a browser.
//!
//! The sentences go to a live region outside the scene, for a screen reader, and to the visible status line. They
//! describe the selected butterfly and never list all the lanes.

use super::algorithm::PairTrace;
use demo_common::complex::Complex;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A real number to three decimals, with a true minus sign, and no minus sign on a zero.
fn real(x: f64) -> String {
    let text = format!("{:.3}", x.abs());
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    let trimmed = if trimmed.is_empty() { "0" } else { trimmed };
    if x < 0.0 && trimmed != "0" {
        format!("\u{2212}{trimmed}")
    } else {
        trimmed.to_owned()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A complex number to three decimals, in the shortest form that reads correctly: `1`, `−i`, `0.5i`, `0.707 − 0.707i`.
pub(super) fn complex(z: Complex) -> String {
    let (re, im) = (real(z.re), real(z.im));
    let imaginary = |text: &str| match text {
        "1" => "i".to_owned(),
        "\u{2212}1" => "\u{2212}i".to_owned(),
        other => format!("{other}i"),
    };
    match (re.as_str(), im.as_str()) {
        (_, "0") => re,
        ("0", _) => imaginary(&im),
        (_, _) => match im.strip_prefix('\u{2212}') {
            Some(magnitude) => format!("{re} \u{2212} {}", imaginary(magnitude)),
            None => format!("{re} + {}", imaginary(&im)),
        },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `W4^0 = 1`: the twiddle factor of a butterfly.
pub(super) fn twiddle(pair: &PairTrace) -> String {
    format!("W{}^{} = {}", pair.block_width(), pair.j, complex(pair.w))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The title above the inspector: a preview before any stage is complete, and the stage and pair afterwards.
pub(super) fn inspector_title(completed: usize, stages: usize, pair_number: usize, pairs: usize) -> String {
    if completed == 0 {
        format!("Preview: stage 1, pair {pair_number}")
    } else {
        format!("Stage {completed} of {stages}, pair {pair_number} of {pairs}")
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The text of one entry of the pair selector.
pub(super) fn pair_option(pair_number: usize, pair: &PairTrace) -> String {
    format!("Pair {pair_number} \u{00B7} lanes {}, {}", pair.upper, pair.lower)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Said when a size is chosen, a slider is let go, and when the demo restarts: nothing is complete yet.
pub(super) fn ready(n: usize, pair_number: usize, pair: &PairTrace) -> String {
    format!(
        "FFT ready: N={n} samples of the summed waves. No stages completed. Previewing stage 1, pair {pair_number}, lanes {} and {}.",
        pair.upper, pair.lower
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Said after the stepping toolbar moves to `completed` stages.
pub(super) fn stepped(completed: usize, stages: usize, pair_number: usize, pairs: usize, pair: &PairTrace) -> String {
    let mut said = format!(
        "Stage {completed} of {stages} complete; block width {}; pair {pair_number} of {pairs}; lanes {} and {}; twiddle {}.",
        pair.block_width(),
        pair.upper,
        pair.lower,
        twiddle(pair)
    );
    if completed == stages {
        said.push_str(" This is the transform: the lanes now hold X[0] onwards, in order.");
    }
    said
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Said when another butterfly of the same stage is chosen. It gives the numbers of the butterfly and no others.
pub(super) fn pair_chosen(completed: usize, pair_number: usize, pairs: usize, pair: &PairTrace) -> String {
    let stage = if completed == 0 {
        "Preview of stage 1".to_owned()
    } else {
        format!("Stage {completed}")
    };
    format!(
        "{stage}, pair {pair_number} of {pairs}: lanes {} and {}; {}. a = {}, b = {}, w times b = {}, sum = {}, difference = {}.",
        pair.upper,
        pair.lower,
        twiddle(pair),
        complex(pair.a),
        complex(pair.b),
        complex(pair.product),
        complex(pair.sum),
        complex(pair.difference)
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The banner when a repaint failed and the last good display is back.
pub(super) fn rolled_back(error: &str) -> String {
    format!("That step failed, and the previous display is back: {error}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The banner when the display could not be put right. A retry button is offered beside it.
pub(super) fn needs_repair(error: &str) -> String {
    format!("The display needs repair: {error}")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Added when the inspector's butterfly could not stay where it was dragged to.
pub(super) const MOVED_INSIDE: &str = " The butterfly was moved to stay inside the new canvas.";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
