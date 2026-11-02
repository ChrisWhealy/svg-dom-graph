//! What the four plots of the convolution demo hold: their configuration, the writing, and what is drawn for a probe.

use super::*;
use crate::convolution::analysis::{FIRST_PROBE, samples, spectrum};

fn finding(frequency: f64) -> Finding {
    Finding::new(&samples(), frequency)
}

/// The items of one kind, as lines, areas or labels.
fn lines(items: &[PlotItem]) -> Vec<&PlotLine> {
    items
        .iter()
        .filter_map(|i| if let PlotItem::Line(l) = i { Some(l) } else { None })
        .collect()
}

fn areas(items: &[PlotItem]) -> Vec<&PlotArea> {
    items
        .iter()
        .filter_map(|i| if let PlotItem::Area(a) = i { Some(a) } else { None })
        .collect()
}

fn texts(items: &[PlotItem]) -> Vec<String> {
    items
        .iter()
        .filter_map(|i| if let PlotItem::Label(l) = i { Some(l.text.clone()) } else { None })
        .collect()
}

fn dots(items: &[PlotItem]) -> Vec<&PlotPoints> {
    items
        .iter()
        .filter_map(|i| if let PlotItem::Points(p) = i { Some(p) } else { None })
        .collect()
}

#[test]
fn a_number_is_written_with_its_sign_and_a_proper_minus_and_zero_has_none() {
    assert_eq!(signed(0.3214), "+0.32");
    assert_eq!(signed(-0.1149), "\u{2212}0.11");
    assert_eq!(signed(0.0), "0.00");
    assert_eq!(signed(-0.0001), "0.00", "a number that rounds to nothing has no sign");
    assert_eq!(frequency_text(2.0), "2.0 Hz");
}

#[test]
fn the_signal_plot_has_the_probes_behind_the_signal_and_names_all_three() {
    let items = signal_items(&samples(), 2.0);
    let drawn = lines(&items);
    assert_eq!(drawn.len(), 3);
    assert_eq!(
        drawn.iter().map(|l| l.colour.as_str()).collect::<Vec<_>>(),
        [COSINE, SINE, SIGNAL],
        "the signal is last, so it is over the probes"
    );
    assert!(drawn[0].width < drawn[2].width, "the probes are thinner");
    assert!(drawn.iter().all(|l| l.points.len() == SEGMENTS + 1));
    assert_eq!(
        texts(&items),
        [
            "signal s(t)",
            "cos(2\u{03C0} \u{00B7} 2.0 \u{00B7} t)",
            "sin(2\u{03C0} \u{00B7} 2.0 \u{00B7} t)"
        ]
    );
}

#[test]
fn the_probe_in_the_signal_plot_is_the_one_of_the_frequency() {
    let items = signal_items(&samples(), 3.0);
    let cosine = lines(&items)[0];
    assert!((cosine.points[0].y - 1.0).abs() < 1e-12, "a cosine starts at 1");
    // A cosine of 3 Hz is back at 1 after a third of a second.
    let third = cosine.points.iter().find(|p| (p.x - 1.0 / 3.0).abs() < 1e-3).map(|p| p.y);
    assert!(third.is_some_and(|y| y > 0.99), "{third:?}");
}

#[test]
fn a_product_is_shaded_above_and_below_its_axis_and_the_line_is_drawn_over_the_shading() {
    let found = finding(FIRST_PROBE);
    for probe in [Probe::Cosine, Probe::Sine] {
        let items = product_items(probe, &found);
        let shading = areas(&items);
        assert_eq!(shading.len(), 1);
        assert_eq!((shading[0].above.as_str(), shading[0].below.as_str()), (ABOVE, BELOW));
        assert_eq!(shading[0].baseline, 0.0, "the shading is down to the axis");
        assert!((shading[0].opacity - SHADING).abs() < 1e-12);
        assert!(shading[0].opacity > 0.0 && shading[0].opacity < 1.0, "it can be seen through");
        let first = items.iter().position(|i| matches!(i, PlotItem::Area(_)));
        let line = items.iter().position(|i| matches!(i, PlotItem::Line(_)));
        assert!(first < line, "the shading is drawn before the line");
        assert_eq!(shading[0].points.len(), SEGMENTS + 1);
    }
}

#[test]
fn the_shading_is_of_the_product_and_not_of_the_signal() {
    let found = finding(2.0);
    let items = product_items(Probe::Cosine, &found);
    let shading = areas(&items)[0];
    assert!((shading.points[100].y - found.cosine.heights[100]).abs() < 1e-12);
    assert!((shading.points[100].x - seconds(100)).abs() < 1e-12);
}

#[test]
fn a_product_says_its_areas_and_the_integral_is_the_one_less_the_other() {
    let found = finding(2.0);
    let items = product_items(Probe::Cosine, &found);
    let written = texts(&items);
    assert_eq!(written.len(), 4);
    assert_eq!(written[0], "s(t) \u{00B7} cos(2\u{03C0} \u{00B7} 2.0 \u{00B7} t)");
    let areas = found.cosine.areas;
    assert_eq!(written[1], format!("{} above", signed(areas.above)));
    assert_eq!(written[2], format!("{} below", signed(areas.below)));
    assert_eq!(written[3], format!("integral {}", signed(areas.net())));
    assert!(areas.above > 0.0 && areas.below < 0.0);
}

#[test]
fn at_a_frequency_that_is_not_in_the_signal_the_two_areas_cancel() {
    let written = texts(&product_items(Probe::Sine, &finding(3.0)));
    assert_eq!(written[3], "integral 0.00");
    // And each area on its own is not small, so it is a cancelling and not an absence of signal.
    assert_ne!(written[1], "0.00 above");
}

#[test]
fn the_spectrum_has_its_curve_a_dotted_line_and_a_dot_at_the_probe() {
    let found = finding(5.0);
    let items = spectrum_items(&spectrum(&samples()), &found);
    assert_eq!(lines(&items).len(), 2, "the curve, and the dotted line");
    assert_eq!(lines(&items)[0].points.len(), 101);
    let dot = dots(&items);
    assert_eq!(dot.len(), 1);
    assert_eq!(dot[0].points, vec![PlotPoint::new(5.0, found.amplitude())]);
    assert!((found.amplitude() - 0.6).abs() < 1e-3);
    let dotted_line = lines(&items)[1];
    assert_eq!(dotted_line.points[0], PlotPoint::new(5.0, 0.0), "it goes down to the axis");
    assert!(dotted_line.dash.is_some());
}

#[test]
fn the_spectrum_says_that_a_and_b_are_twice_the_integrals_and_leaves_out_the_phase_where_there_is_nothing() {
    let [first, second] = result_lines(&finding(2.0));
    assert_eq!(
        first, "2.0 Hz:   a = 2 \u{00D7} 0.32 = +0.64   b = 2 \u{00D7} 0.38 = +0.77",
        "the integrals are the ones that are written in the products"
    );
    assert_eq!(
        second,
        "amplitude = \u{221A}(a\u{00B2} + b\u{00B2}) = 1.00   phase = 40\u{00B0}"
    );
    let [_, at_three] = result_lines(&finding(3.0));
    assert!(
        at_three.contains("amplitude") && at_three.ends_with("phase = \u{2014}"),
        "{at_three}"
    );
    let written = texts(&spectrum_items(&spectrum(&samples()), &finding(2.0)));
    assert_eq!(written.len(), 3, "the title, and the two lines of the result");
    assert!(written[1].starts_with("2.0 Hz:   a = 2"), "{}", written[1]);
}

#[test]
fn a_negative_integral_is_written_with_a_minus_and_a_positive_one_without_a_plus() {
    assert_eq!(plain(0.3214), "0.32");
    assert_eq!(plain(-0.1149), "\u{2212}0.11");
    assert_eq!(plain(0.0), "0.00");
}

#[test]
fn every_plot_is_configured_with_its_size_and_a_description_that_can_be_read() {
    let found = finding(2.0);
    let configs = [
        (signal_config(2.0), SIGNAL_SIZE),
        (product_config(Probe::Cosine, &found), PRODUCT_SIZE),
        (product_config(Probe::Sine, &found), PRODUCT_SIZE),
        (spectrum_config(&found), SPECTRUM_SIZE),
    ];
    for (config, size) in configs {
        assert_eq!((config.size.width, config.size.height), size);
        assert!(config.validate().is_ok());
        assert!(config.description.as_deref().is_some_and(|d| d.len() > 30));
    }
    assert!(
        (SIGNAL_SIZE.0 - 2.0 * PRODUCT_SIZE.0).abs() < 1e-12,
        "the two products are as wide together as the other scenes"
    );
}

#[test]
fn the_descriptions_say_the_frequency_and_the_integral() {
    let found = finding(2.0);
    assert!(signal_description(2.0).contains("2.0 Hz"));
    let cosine = product_description(Probe::Cosine, &found);
    assert!(
        cosine.contains("cosine") && cosine.contains("2.0 Hz") && cosine.contains("integral is +0.32"),
        "{cosine}"
    );
    assert!(product_description(Probe::Sine, &found).contains("sine of 2.0 Hz"));
    assert!(spectrum_description(&found).contains("amplitude is 1.00"));
}

#[test]
fn the_three_time_plots_have_the_same_scale_of_height() {
    // The same range of heights, so that a height is the same distance up in all three.
    let found = finding(2.0);
    let heights = [
        signal_config(2.0).y,
        product_config(Probe::Cosine, &found).y,
        product_config(Probe::Sine, &found).y,
    ];
    assert!(heights.iter().all(|y| (y.min, y.max) == (-SPAN, SPAN)));
    assert!(SIGNAL_SIZE.1 - STRIP > PRODUCT_SIZE.1 - 1.0 && SIGNAL_SIZE.1 - STRIP < PRODUCT_SIZE.1 + 20.0);
}

#[test]
fn the_ticks_of_a_height_axis_leave_out_zero_and_use_a_proper_minus() {
    let TickLabels::Custom(ticks) = value_ticks(1.8, 0.5, false) else {
        panic!("custom ticks");
    };
    let values: Vec<f64> = ticks.iter().map(|t| t.0).collect();
    assert_eq!(values, [-1.5, -1.0, -0.5, 0.5, 1.0, 1.5]);
    assert_eq!(ticks[0].1, "\u{2212}1.5");
    let TickLabels::Custom(up) = value_ticks(1.2, 0.2, true) else {
        panic!("custom ticks");
    };
    assert_eq!(up.len(), 5, "0.2 to 1.0, since the end of the axis at 1.2 is only the margin");
    assert_eq!(up[0].1, "0.2");
}

#[test]
fn the_ticks_of_the_time_axes_are_in_seconds_with_none_at_zero() {
    let TickLabels::Custom(fine) = time_ticks(0.1) else { panic!("custom ticks") };
    assert_eq!((fine.len(), fine[0].1.as_str(), fine[9].1.as_str()), (10, "0.1 s", "1.0 s"));
    let TickLabels::Custom(coarse) = time_ticks(0.25) else {
        panic!("custom ticks")
    };
    let labels: Vec<&str> = coarse.iter().map(|t| t.1.as_str()).collect();
    assert_eq!(labels, ["0.25 s", "0.50 s", "0.75 s", "1 s"]);
}

#[test]
fn the_spectrum_says_a_constant_is_found_once_and_has_no_phase() {
    let samples: Vec<f64> = samples().into_iter().map(|s| s + 0.5).collect();
    let [first, second] = result_lines(&Finding::new(&samples, 0.0));
    assert_eq!(first, "0.0 Hz:   a = 1 \u{00D7} 0.50 = +0.50   b = 1 \u{00D7} 0.00 = 0.00");
    assert_eq!(second, "amplitude = \u{221A}(a\u{00B2} + b\u{00B2}) = 0.50   phase = \u{2014}");
    // A wave is still twice, and has its phase.
    let [wave, _] = result_lines(&Finding::new(&samples, 2.0));
    assert!(wave.contains("a = 2 \u{00D7} 0.32 = +0.64"), "{wave}");
}
