//! Contrast between the colours that text and outlines are drawn in and the colours behind them, by the formula of WCAG
//! 2.2 (success criterion 1.4.3, and 1.4.11 for the outline of a control). It reads the real palette constants, so a
//! colour that is changed to one that is too faint fails here, with no browser needed. It does not check every pairing
//! of every colour: only those a rule is claimed for.

use super::*;

/// The channels of a `#rrggbb` colour.
fn channels(colour: &str) -> [f64; 3] {
    let hex = colour.strip_prefix('#').expect("a colour that starts with #");
    assert_eq!(hex.len(), 6, "{colour} is not #rrggbb");
    let channel = |at: usize| f64::from(u8::from_str_radix(&hex[at..at + 2], 16).expect("two hex digits"));
    [channel(0), channel(2), channel(4)]
}

/// The relative luminance of `colour`, as WCAG defines it.
fn luminance(colour: &str) -> f64 {
    let [r, g, b] = channels(colour).map(|value| {
        let c = value / 255.0;
        if c <= 0.039_28 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// The contrast ratio of two colours: from 1, for the same colour, to 21, for black on white.
fn contrast(a: &str, b: &str) -> f64 {
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_formula_gives_the_known_ratios() {
    assert!((contrast("#000000", "#ffffff") - 21.0).abs() < 1e-9);
    assert!((contrast("#777777", "#777777") - 1.0).abs() < 1e-9);
    // WCAG's own example: #777 on white is just under 4.5 to 1, and #767676 is just over.
    assert!(contrast("#777777", "#ffffff") < 4.6 && contrast("#777777", "#ffffff") > 4.4);
    assert!(contrast("#767676", "#ffffff") >= 4.5);
}

#[test]
fn an_edge_labels_text_reads_against_its_background() {
    let ratio = contrast(TEXT_FILL, EDGE_LABEL_FILL);
    assert!(
        ratio >= 4.5,
        "label text is {ratio:.2} to 1 against its background, and needs 4.5 to 1"
    );
}

#[test]
fn an_edge_labels_outline_can_be_told_from_the_page_behind_it() {
    let ratio = contrast(EDGE_LABEL_STROKE, EDGE_LABEL_FILL);
    assert!(
        ratio >= 3.0,
        "the outline is {ratio:.2} to 1 against its background, and needs 3 to 1"
    );
}

#[test]
fn a_focused_edge_can_be_told_from_a_plain_one_and_from_the_page() {
    let page = contrast(EDGE_FOCUS_STROKE, EDGE_LABEL_FILL);
    assert!(page >= 3.0, "a focused edge is {page:.2} to 1 against white, and needs 3 to 1");
    assert_ne!(EDGE_FOCUS_STROKE, CONNECTOR_STROKE);
}

#[test]
fn text_cells_read_against_the_text_colour() {
    let ratio = contrast(TEXT_FILL, TYPE_COLOUR_TEXT);
    assert!(ratio >= 4.5, "text on a text cell is {ratio:.2} to 1, and needs 4.5 to 1");
}

#[test]
fn a_boolean_mark_reads_against_its_halo() {
    for ink in [BOOL_TRUE_INK, BOOL_FALSE_INK] {
        let ratio = contrast(ink, BOOL_ICON_HALO);
        assert!(ratio >= 4.5, "{ink} is {ratio:.2} to 1 against its halo, and needs 4.5 to 1");
    }
}
