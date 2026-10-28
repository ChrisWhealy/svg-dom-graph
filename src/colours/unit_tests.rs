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

#[test]
fn complex_cells_and_their_brackets_read_against_the_text_colour() {
    for fill in [TYPE_COLOUR_COMPLEX32, TYPE_COLOUR_COMPLEX64] {
        let ratio = contrast(TEXT_FILL, fill);
        assert!(ratio >= 4.5, "text on {fill} is {ratio:.2} to 1, and needs 4.5 to 1");

        let bracket = contrast(MATRIX_BRACKET_STROKE, fill);
        assert!(bracket >= 3.0, "a bracket on {fill} is {bracket:.2} to 1, and needs 3 to 1");
    }
}

#[test]
fn the_strokes_of_a_complex_picture_can_be_told_from_the_cell_behind_them() {
    for fill in [TYPE_COLOUR_COMPLEX32, TYPE_COLOUR_COMPLEX64] {
        for stroke in [PICTURE_VECTOR_STROKE, PICTURE_REAL_STROKE, PICTURE_IMAGINARY_STROKE] {
            let ratio = contrast(stroke, fill);
            assert!(ratio >= 3.0, "{stroke} is {ratio:.2} to 1 against {fill}, and needs 3 to 1");
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_hex_colour_is_read_in_both_lengths_and_nothing_else_is() {
    assert_eq!(parse_hex("#eef4ff"), Some([0xee, 0xf4, 0xff]));
    assert_eq!(parse_hex("#fa0"), Some([0xff, 0xaa, 0x00]));
    for bad in ["eef4ff", "#eef4f", "#eef4ffaa", "#gggggg", "lightsteelblue", "", "#", "#ééé"] {
        assert_eq!(parse_hex(bad), None, "{bad:?}");
    }
}

#[test]
fn darkening_takes_a_fraction_off_every_channel() {
    assert_eq!(darken("#ffffff", 0.0).as_deref(), Some("#ffffff"));
    assert_eq!(darken("#ffffff", 1.0).as_deref(), Some("#000000"));
    assert_eq!(darken("#ffffff", 0.5).as_deref(), Some("#808080"));
    assert_eq!(darken("#fff", 0.5).as_deref(), Some("#808080"));
    assert_eq!(darken("red", 0.5), None);
    assert_eq!(darken("#ffffff", -0.1), None);
    assert_eq!(darken("#ffffff", 1.1), None);
    assert_eq!(darken("#ffffff", f64::NAN), None);
}

#[test]
fn the_grid_of_the_default_plot_is_a_little_darker_than_its_background_and_the_major_is_the_darker() {
    let background = luminance(PLOT_BACKGROUND);
    let major = luminance(&darken(PLOT_BACKGROUND, MAJOR_GRID_DARKENING).expect("a hex background"));
    let minor = luminance(&darken(PLOT_BACKGROUND, MINOR_GRID_DARKENING).expect("a hex background"));
    assert!(major < minor && minor < background);
    // Slightly: a grid line is seen, but is far from the ink of the plotted data.
    assert!(contrast(PLOT_BACKGROUND, &darken(PLOT_BACKGROUND, MAJOR_GRID_DARKENING).expect("hex")) < 1.5);
}

#[test]
fn the_axis_and_tick_ink_read_on_the_default_background() {
    assert!(contrast(PLOT_AXIS, PLOT_BACKGROUND) >= 4.5);
    assert!(contrast(PLOT_TICK_INK, PLOT_BACKGROUND) >= 4.5);
}
