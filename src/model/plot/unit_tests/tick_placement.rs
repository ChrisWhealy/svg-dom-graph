//! Where the ticks of an axis are, what they say, and how a range of any size gets distinct ones.

use super::*;

#[test]
fn an_automatic_interval_is_a_round_one_that_gives_about_six() {
    assert_eq!(nice_interval(1.0), 0.2);
    assert_eq!(nice_interval(11.0), 2.0);
    assert_eq!(nice_interval(10.0), 2.0);
    assert_eq!(nice_interval(100.0), 20.0);
    assert_eq!(nice_interval(0.03), 0.005);
    for span in [0.7, 3.0, 7.0, 13.0, 250.0, 9_000.0] {
        let step = nice_interval(span);
        let count = span / step;
        assert!((3.0..=9.0).contains(&count), "{span}: {count} intervals of {step}");
    }
}

#[test]
fn a_linear_axis_has_a_major_tick_at_every_multiple_of_its_interval_and_no_others() {
    let axis = PlotAxis::linear(-5.5, 5.5).with_major_interval(1.0);
    let expected: Vec<f64> = (-5..=5).map(f64::from).collect();
    assert_eq!(majors(&axis), expected);
    assert_eq!(values(&axis), expected, "there are no minor ticks by default");
}

#[test]
fn a_tick_is_not_a_hair_off_its_value_after_many_multiplications() {
    let axis = PlotAxis::linear(0.0, 1.0).with_major_interval(0.1);
    let got = majors(&axis);
    assert_eq!(got.len(), 11);
    assert_eq!(got[3], 0.3, "0.1 times 3 is not 0.30000000000000004 here");
    assert_eq!(got[10], 1.0);
}

#[test]
fn a_tick_at_either_end_of_the_axis_is_kept_and_one_past_it_is_not() {
    let axis = PlotAxis::linear(0.0, 1.0).with_major_interval(0.25);
    assert_eq!(majors(&axis), vec![0.0, 0.25, 0.5, 0.75, 1.0]);
    let axis = PlotAxis::linear(0.1, 0.9).with_major_interval(0.25);
    assert_eq!(majors(&axis), vec![0.25, 0.5, 0.75]);
}

#[test]
fn minor_divisions_split_each_major_interval_and_the_part_intervals_at_the_ends() {
    let axis = PlotAxis::linear(0.0, 2.0)
        .with_major_interval(1.0)
        .with_minor(MinorTicks::Divisions(4));
    let all = ticks(&axis);
    assert_eq!(all.iter().filter(|t| t.major).count(), 3);
    assert_eq!(all.iter().filter(|t| !t.major).count(), 6, "three between each pair of majors");
    assert!(all.windows(2).all(|w| w[0].value < w[1].value), "in order");
    assert!(near(all[1].value, 0.25) && !all[1].major);
    // An axis that starts part way through an interval still has the minor ticks of that interval.
    let axis = PlotAxis::linear(0.5, 2.0)
        .with_major_interval(1.0)
        .with_minor(MinorTicks::Divisions(2));
    assert_eq!(values(&axis), vec![0.5, 1.0, 1.5, 2.0]);
}

#[test]
fn automatic_labels_say_the_value_and_carry_the_suffix() {
    let axis = PlotAxis::linear(-2.0, 2.0).with_major_interval(1.0).with_tick_suffix("i");
    let labels: Vec<String> = ticks(&axis).into_iter().filter_map(|t| t.label).collect();
    assert_eq!(labels, ["\u{2212}2i", "\u{2212}1i", "0i", "1i", "2i"]);
}

#[test]
fn labels_can_be_turned_off_and_a_minor_tick_never_has_one() {
    let axis = PlotAxis::linear(0.0, 2.0)
        .with_major_interval(1.0)
        .with_minor(MinorTicks::Divisions(2))
        .with_tick_labels(TickLabels::None);
    assert!(ticks(&axis).iter().all(|t| t.label.is_none()));
    let axis = axis.with_tick_labels(TickLabels::Auto);
    assert!(ticks(&axis).iter().filter(|t| !t.major).all(|t| t.label.is_none()));
    assert!(ticks(&axis).iter().filter(|t| t.major).all(|t| t.label.is_some()));
}

#[test]
fn custom_ticks_replace_the_automatic_ones_and_those_off_the_axis_are_dropped() {
    let axis = PlotAxis::linear(0.0, 1.0)
        .with_major_interval(0.1)
        .with_tick_labels(TickLabels::Custom(vec![
            (0.75, "\u{00BE}".to_owned()),
            (0.0, "0".to_owned()),
            (0.5, "\u{00BD}".to_owned()),
            (2.0, "off the end".to_owned()),
        ]));
    let got = ticks(&axis);
    assert_eq!(
        got.iter()
            .map(|t| (t.value, t.label.clone().expect("a label")))
            .collect::<Vec<_>>(),
        vec![
            (0.0, "0".to_owned()),
            (0.5, "\u{00BD}".to_owned()),
            (0.75, "\u{00BE}".to_owned())
        ]
    );
    assert!(got.iter().all(|t| t.major));
}

#[test]
fn numbers_are_written_with_a_true_minus_no_stray_zeros_and_an_exponent_when_extreme() {
    assert_eq!(format_number(0.0), "0");
    assert_eq!(format_number(-0.0), "0");
    assert_eq!(format_number(2.0), "2");
    assert_eq!(format_number(-2.5), "\u{2212}2.5");
    assert_eq!(format_number(0.30000000000000004), "0.3");
    assert_eq!(format_number(1000.0), "1000");
    assert_eq!(format_number(1e6), "1e6");
    assert_eq!(format_number(2.5e-5), "2.5e\u{2212}5");
    assert_eq!(format_number(f64::NAN), "0");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_logarithmic_axis_has_a_major_tick_at_each_power_of_ten_and_minor_ones_between() {
    let axis = PlotAxis::logarithmic(1.0, 1000.0);
    assert_eq!(majors(&axis), vec![1.0, 10.0, 100.0, 1000.0]);
    let minors: Vec<f64> = ticks(&axis).iter().filter(|t| !t.major).map(|t| t.value).collect();
    assert_eq!(minors.len(), 3 * 8, "2 to 9, in each of three decades");
    assert!(minors.contains(&2.0) && minors.contains(&90.0) && !minors.contains(&10.0));
    assert!(minors.iter().all(|m| (1.0..=1000.0).contains(m)));
}

#[test]
fn the_labels_of_a_logarithmic_axis_are_the_powers_of_ten() {
    let axis = PlotAxis::logarithmic(0.01, 100.0);
    let labels: Vec<String> = ticks(&axis).into_iter().filter_map(|t| t.label).collect();
    assert_eq!(labels, ["0.01", "0.1", "1", "10", "100"]);
}

#[test]
fn a_logarithmic_interval_is_in_decades_and_minor_ticks_can_be_turned_off() {
    let axis = PlotAxis::logarithmic(1.0, 1e6)
        .with_major_interval(3.0)
        .with_minor(MinorTicks::None);
    assert_eq!(values(&axis), vec![1.0, 1000.0, 1e6]);
    let part = PlotAxis::logarithmic(3.0, 300.0);
    assert_eq!(majors(&part), vec![10.0, 100.0]);
    let minors: Vec<f64> = ticks(&part).iter().filter(|t| !t.major).map(|t| t.value).collect();
    assert!(minors.contains(&3.0) && minors.contains(&300.0) && !minors.contains(&2.0));
}

#[test]
fn a_very_wide_logarithmic_range_does_not_overflow_the_estimate() {
    let axis = PlotAxis::logarithmic(1e-250, 1e250)
        .with_major_interval(100.0)
        .with_minor(MinorTicks::None);
    assert!(count_estimate(&axis) < 100);
    assert!(count_estimate(&axis) >= ticks(&axis).len());
}

#[test]
fn the_ticks_of_a_range_far_from_zero_stay_distinct() {
    let axis = PlotAxis::linear(1e15, 1e15 + 1.0).with_major_interval(0.25);
    let found = values(&axis);
    assert_eq!(found.len(), 5);
    assert!(found.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn an_estimate_of_the_ticks_never_under_counts() {
    for axis in [
        PlotAxis::linear(0.0, 1.0),
        PlotAxis::linear(-5.5, 5.5)
            .with_major_interval(0.5)
            .with_minor(MinorTicks::Divisions(5)),
        PlotAxis::logarithmic(1.0, 1e9),
        PlotAxis::linear(0.0, 1.0).with_tick_labels(TickLabels::Custom(vec![(0.5, "x".into())])),
    ] {
        assert!(count_estimate(&axis) >= ticks(&axis).len());
    }
}
