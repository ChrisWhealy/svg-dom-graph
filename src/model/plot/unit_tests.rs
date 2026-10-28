use super::{
    axis::AxisName,
    clip::{Bounds, clip_segment},
    config::MAX_TICKS,
    layout::{self, Side},
    map::AxisMap,
    ticks::{count_estimate, format_number, nice_interval},
    *,
};
use crate::complex::Complex;
use svg_dom::root::utils::Size;

fn real(x: PlotAxis, y: PlotAxis) -> PlotConfig {
    PlotConfig::new(Size::new(400.0, 300.0), x, y)
}

fn values(axis: &PlotAxis) -> Vec<f64> {
    ticks(axis).iter().map(|t| t.value).collect()
}

fn majors(axis: &PlotAxis) -> Vec<f64> {
    ticks(axis).iter().filter(|t| t.major).map(|t| t.value).collect()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9 * (1.0 + b.abs())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Ticks
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The map
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_linear_axis_maps_its_ends_to_the_ends_of_its_span_and_back() {
    let axis = PlotAxis::linear(-4.0, 4.0);
    let map = AxisMap::new(&axis, 100.0, 300.0);
    assert_eq!(map.page_of(-4.0), Some(100.0));
    assert_eq!(map.page_of(4.0), Some(300.0));
    assert_eq!(map.page_of(0.0), Some(200.0));
    for value in [-3.2, 0.0, 1.5, 3.99] {
        assert!(near(map.value_at(map.page_of(value).expect("placed")), value));
    }
}

#[test]
fn a_vertical_axis_has_the_greater_values_higher_up_the_page() {
    // The bottom, which is the greater coordinate, is where the axis starts.
    let map = AxisMap::new(&PlotAxis::linear(0.0, 10.0), 250.0, 50.0);
    assert!(map.page_of(9.0).expect("placed") < map.page_of(1.0).expect("placed"));
    assert_eq!(map.page_of(0.0), Some(250.0));
    assert_eq!(map.page_of(10.0), Some(50.0));
}

#[test]
fn a_logarithmic_axis_gives_equal_ratios_equal_distances() {
    let map = AxisMap::new(&PlotAxis::logarithmic(1.0, 1000.0), 0.0, 300.0);
    assert!(near(map.page_of(1.0).expect("placed"), 0.0));
    assert!(near(map.page_of(10.0).expect("placed"), 100.0));
    assert!(near(map.page_of(100.0).expect("placed"), 200.0));
    assert!(near(map.page_of(1000.0).expect("placed"), 300.0));
    assert!(near(
        map.page_of(30.0).expect("placed") - map.page_of(3.0).expect("placed"),
        100.0
    ));
    assert!(near(map.value_at(250.0), 10f64.powf(2.5)));
}

#[test]
fn a_value_that_cannot_be_placed_is_none_and_one_off_the_end_is_placed_beyond_it() {
    let linear = AxisMap::new(&PlotAxis::linear(0.0, 1.0), 0.0, 100.0);
    assert_eq!(linear.page_of(f64::NAN), None);
    assert_eq!(linear.page_of(f64::INFINITY), None);
    assert_eq!(linear.page_of(2.0), Some(200.0));
    let log = AxisMap::new(&PlotAxis::logarithmic(1.0, 10.0), 0.0, 100.0);
    assert_eq!(log.page_of(0.0), None);
    assert_eq!(log.page_of(-3.0), None);
}

#[test]
fn an_axis_line_is_kept_on_the_plot() {
    let map = AxisMap::new(&PlotAxis::linear(2.0, 6.0), 0.0, 100.0);
    assert_eq!(
        map.clamped_page(0.0, false),
        0.0,
        "zero is off the end, so the line is at the near edge"
    );
    assert_eq!(map.clamped_page(4.0, false), 50.0);
    assert_eq!(map.clamped_page(f64::INFINITY, true), 100.0);
    let log = AxisMap::new(&PlotAxis::logarithmic(1.0, 100.0), 0.0, 100.0);
    assert_eq!(log.clamped_page(0.0, false), 0.0, "zero cannot be placed on a logarithmic axis");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Clipping
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const BOX: Bounds = Bounds {
    left: 0.0,
    top: 0.0,
    right: 10.0,
    bottom: 10.0,
};

#[test]
fn a_segment_inside_is_kept_and_one_outside_is_dropped() {
    assert_eq!(clip_segment((1.0, 1.0), (9.0, 9.0), BOX), Some(((1.0, 1.0), (9.0, 9.0))));
    assert_eq!(clip_segment((11.0, 0.0), (20.0, 5.0), BOX), None);
    assert_eq!(
        clip_segment((-5.0, 11.0), (15.0, 11.0), BOX),
        None,
        "parallel to an edge, and outside it"
    );
}

#[test]
fn a_segment_that_leaves_is_cut_at_the_edge() {
    assert_eq!(clip_segment((5.0, 5.0), (15.0, 5.0), BOX), Some(((5.0, 5.0), (10.0, 5.0))));
    let (a, b) = clip_segment((-10.0, 5.0), (20.0, 5.0), BOX).expect("crosses the box");
    assert_eq!((a, b), ((0.0, 5.0), (10.0, 5.0)));
    let (a, b) = clip_segment((-5.0, -5.0), (15.0, 15.0), BOX).expect("crosses the corners");
    assert!(near(a.0, 0.0) && near(a.1, 0.0) && near(b.0, 10.0) && near(b.1, 10.0));
}

#[test]
fn a_polyline_is_split_where_it_leaves_and_where_a_point_is_missing() {
    let points = [
        Some((1.0, 1.0)),
        Some((5.0, 5.0)),
        Some((15.0, 5.0)),
        Some((15.0, 8.0)),
        Some((5.0, 8.0)),
        Some((6.0, 9.0)),
        None,
        Some((2.0, 2.0)),
        Some((3.0, 3.0)),
    ];
    let runs = clip_polyline(&points, BOX);
    assert_eq!(
        runs,
        vec![
            vec![(1.0, 1.0), (5.0, 5.0), (10.0, 5.0)],
            vec![(10.0, 8.0), (5.0, 8.0), (6.0, 9.0)],
            vec![(2.0, 2.0), (3.0, 3.0)],
        ]
    );
}

#[test]
fn a_polyline_wholly_outside_or_of_one_point_has_no_run() {
    assert!(clip_polyline(&[Some((20.0, 20.0)), Some((30.0, 30.0))], BOX).is_empty());
    assert!(clip_polyline(&[Some((5.0, 5.0))], BOX).is_empty());
    assert!(clip_polyline(&[], BOX).is_empty());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The layout
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_real_plot_keeps_its_labels_in_the_margins_and_a_complex_one_beside_its_axes() {
    let x = PlotAxis::linear(-5.0, 5.0);
    let chart = PlotLayout::new(&real(x.clone(), x.clone())).expect("valid");
    assert!(chart.area.left > 40.0 && chart.area.bottom < 300.0 - 30.0);
    assert_eq!(chart.placement, TickLabelPlacement::Edge);
    let argand = PlotLayout::new(&real(x.clone(), x).with_plane(PlotPlane::Complex)).expect("valid");
    assert_eq!(argand.placement, TickLabelPlacement::Axis);
    assert!(argand.area.left < chart.area.left, "the margin is only as big as what is in it");
    let forced = PlotLayout::new(
        &real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(0.0, 1.0)).with_label_placement(TickLabelPlacement::Axis),
    )
    .expect("valid");
    assert_eq!(forced.placement, TickLabelPlacement::Axis);
}

#[test]
fn the_margins_grow_for_a_legend_and_for_titles_and_shrink_when_labels_are_off() {
    let plain = PlotLayout::new(&real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(0.0, 1.0))).expect("valid");
    let legend = PlotLayout::new(
        &real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(0.0, 1.0)).with_legend(vec![LegendEntry::line("a", "#f00")]),
    )
    .expect("valid");
    assert!(legend.area.top > plain.area.top);
    let titled = PlotLayout::new(&real(
        PlotAxis::linear(0.0, 1.0).with_title("t"),
        PlotAxis::linear(0.0, 1.0).with_title("v"),
    ))
    .expect("valid");
    assert!(titled.area.bottom < plain.area.bottom && titled.area.top < plain.area.top + 1.0 + 100.0);
    assert!(titled.area.top > plain.area.top);
    let bare = PlotLayout::new(&real(
        PlotAxis::linear(0.0, 1.0).with_tick_labels(TickLabels::None),
        PlotAxis::linear(0.0, 1.0).with_tick_labels(TickLabels::None),
    ))
    .expect("valid");
    assert!(bare.area.left < plain.area.left && bare.area.bottom > plain.area.bottom);
}

#[test]
fn given_margins_are_used_as_they_are() {
    let config =
        real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(0.0, 1.0)).with_insets(Insets::new(10.0, 20.0, 30.0, 40.0));
    let layout = PlotLayout::new(&config).expect("valid");
    assert_eq!(
        (layout.area.left, layout.area.top, layout.area.right, layout.area.bottom),
        (10.0, 20.0, 370.0, 260.0)
    );
}

#[test]
fn margins_that_leave_nothing_are_refused() {
    let config =
        real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(0.0, 1.0)).with_insets(Insets::new(200.0, 0.0, 200.0, 0.0));
    assert_eq!(PlotLayout::new(&config), Err(PlotConfigError::Size));
}

#[test]
fn a_value_is_found_on_the_node_and_a_position_is_found_among_the_values() {
    let config = real(PlotAxis::linear(0.0, 10.0), PlotAxis::linear(0.0, 100.0));
    let layout = PlotLayout::new(&config).expect("valid");
    let (x, y) = layout.point(5.0, 50.0).expect("placed");
    let back = layout.values_at((x, y)).expect("inside");
    assert!(near(back.0, 5.0) && near(back.1, 50.0));
    assert_eq!(layout.point(0.0, 0.0), Some((layout.area.left, layout.area.bottom)));
    assert_eq!(layout.point(10.0, 100.0), Some((layout.area.right, layout.area.top)));
    assert_eq!(
        layout.values_at((layout.area.left - 1.0, y)),
        None,
        "the margin is not the plot"
    );
    assert_eq!(layout.values_at((x, layout.area.bottom + 1.0)), None);
    assert_eq!(layout.point(f64::NAN, 1.0), None);
}

#[test]
fn a_click_on_a_log_axis_is_found_in_values() {
    let config = real(PlotAxis::logarithmic(1.0, 1000.0), PlotAxis::linear(0.0, 1.0));
    let layout = PlotLayout::new(&config).expect("valid");
    let middle = (layout.area.left + layout.area.right) / 2.0;
    let (x, _) = layout.values_at((middle, layout.area.top + 1.0)).expect("inside");
    assert!(near(x, 10f64.powf(1.5)));
}

#[test]
fn an_axis_line_is_at_zero_unless_it_is_put_somewhere_else() {
    let config = real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(-2.0, 2.0));
    let layout = PlotLayout::new(&config).expect("valid");
    let flat = PlotAxis::linear(0.0, 1.0);
    // The horizontal axis is at y = 0, which is the middle of the vertical axis. The vertical one is at x = 0, the
    // edge.
    assert!(near(layout.x_axis_height(&flat), (layout.area.top + layout.area.bottom) / 2.0));
    assert_eq!(layout.y_axis_position(&flat), layout.area.left);
    let at_min = flat.clone().with_position(AxisPosition::AtMinimum);
    assert_eq!(layout.x_axis_height(&at_min), layout.area.bottom);
    let at_max = flat.clone().with_position(AxisPosition::AtMaximum);
    assert_eq!(layout.x_axis_height(&at_max), layout.area.top);
    let at = flat.with_position(AxisPosition::At(1.0));
    assert!(near(
        layout.x_axis_height(&at),
        (layout.area.top + layout.area.bottom) / 2.0 - (layout.area.bottom - layout.area.top) / 4.0
    ));
}

#[test]
fn the_plane_supplies_the_titles_and_the_suffix_that_an_axis_does_not() {
    let axes = || (PlotAxis::linear(-1.0, 1.0), PlotAxis::linear(-1.0, 1.0));
    let (x, y) = axes();
    let chart = real(x, y);
    assert_eq!(layout::title(&chart, AxisName::X), None);
    assert_eq!(layout::suffix(&chart, AxisName::Y), "");
    let argand = chart.clone().with_plane(PlotPlane::Complex);
    assert_eq!(layout::title(&argand, AxisName::X), Some("Re"));
    assert_eq!(layout::title(&argand, AxisName::Y), Some("Im"));
    assert_eq!(layout::suffix(&argand, AxisName::Y), "i");
    assert_eq!(layout::suffix(&argand, AxisName::X), "");
    let (x, y) = axes();
    let named = real(x.with_title("Time"), y.with_tick_suffix("V")).with_plane(PlotPlane::Complex);
    assert_eq!(layout::title(&named, AxisName::X), Some("Time"));
    assert_eq!(layout::suffix(&named, AxisName::Y), "V");
}

#[test]
fn the_labels_go_outside_the_plot_at_the_edge_and_beside_the_axes_otherwise() {
    let (x, y) = (PlotAxis::linear(-1.0, 1.0), PlotAxis::linear(-1.0, 1.0));
    let chart = PlotLayout::new(&real(x.clone(), y.clone())).expect("valid");
    let beside = PlotLayout::new(&real(x, y).with_plane(PlotPlane::Complex)).expect("valid");
    assert_eq!(chart.x_label(100.0, 150.0).at, (100.0, chart.area.bottom + 14.0));
    assert_eq!(chart.y_label(100.0, 50.0).at.0, chart.area.left - 8.0);
    assert_eq!(chart.y_label(100.0, 50.0).anchor, Side::End);
    assert_eq!(beside.x_label(100.0, 150.0).at, (100.0, 161.0));
    assert_eq!(beside.y_label(100.0, 50.0).at, (43.0, 100.0));
}

#[test]
fn an_arrowhead_has_its_tip_where_it_is_told_and_its_strokes_behind_it() {
    let path = layout::arrow_head((100.0, 50.0), (1.0, 0.0));
    let numbers: Vec<f64> = path.split_whitespace().filter_map(|word| word.parse().ok()).collect();
    assert_eq!(numbers.len(), 6);
    assert_eq!((numbers[2], numbers[3]), (100.0, 50.0), "the tip is the middle point");
    assert!(numbers[0] < 100.0 && numbers[4] < 100.0, "both strokes go back");
    assert!(
        numbers[1] > 50.0 && numbers[5] < 50.0 || numbers[1] < 50.0 && numbers[5] > 50.0,
        "one each side"
    );
}

#[test]
fn the_default_description_names_the_plane_and_both_ranges() {
    let config = real(PlotAxis::linear(0.0, 1.0), PlotAxis::logarithmic(1.0, 1000.0));
    assert_eq!(
        layout::default_description(&config),
        "Real plane plot: horizontal axis linear from 0 to 1, vertical axis logarithmic from 1 to 1000"
    );
    let complex = config.with_plane(PlotPlane::Complex);
    assert!(layout::default_description(&complex).starts_with("Complex plane plot: real axis"));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// What is refused
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn refusal(config: &PlotConfig) -> Option<PlotConfigError> {
    config.validate().err()
}

fn unit() -> PlotConfig {
    real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(0.0, 1.0))
}

#[test]
fn a_sound_configuration_is_accepted() {
    assert_eq!(refusal(&unit()), None);
    assert_eq!(
        refusal(&real(PlotAxis::logarithmic(1.0, 1e6), PlotAxis::linear(-1.0, 1.0))),
        None
    );
}

#[test]
fn a_node_that_has_no_size_is_refused() {
    for size in [
        Size::new(0.0, 100.0),
        Size::new(100.0, -1.0),
        Size::new(f64::NAN, 100.0),
        Size::new(100.0, f64::INFINITY),
    ] {
        let config = PlotConfig::new(size, PlotAxis::linear(0.0, 1.0), PlotAxis::linear(0.0, 1.0));
        assert_eq!(refusal(&config), Some(PlotConfigError::Size), "{size:?}");
    }
}

#[test]
fn an_axis_that_does_not_run_from_less_to_more_is_refused_and_the_message_says_which() {
    for (min, max) in [(1.0, 1.0), (2.0, 1.0), (f64::NAN, 1.0), (0.0, f64::INFINITY)] {
        let config = real(PlotAxis::linear(min, max), PlotAxis::linear(0.0, 1.0));
        assert_eq!(refusal(&config), Some(PlotConfigError::Range(AxisName::X)));
        let config = real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(min, max));
        assert_eq!(refusal(&config), Some(PlotConfigError::Range(AxisName::Y)));
    }
    let message = PlotConfigError::Range(AxisName::Y).to_string();
    assert!(message.contains("vertical"), "{message}");
}

#[test]
fn a_logarithmic_axis_must_start_above_zero() {
    for min in [0.0, -1.0] {
        let config = real(PlotAxis::linear(0.0, 1.0), PlotAxis::logarithmic(min, 10.0));
        assert_eq!(refusal(&config), Some(PlotConfigError::LogRange(AxisName::Y)));
    }
}

#[test]
fn an_interval_must_be_finite_and_above_zero_and_must_not_make_a_solid_band() {
    for interval in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let config = real(
            PlotAxis::linear(0.0, 1.0).with_major_interval(interval),
            PlotAxis::linear(0.0, 1.0),
        );
        assert_eq!(refusal(&config), Some(PlotConfigError::Interval(AxisName::X)));
    }
    let config = real(PlotAxis::linear(0.0, 1.0).with_major_interval(1e-6), PlotAxis::linear(0.0, 1.0));
    assert_eq!(refusal(&config), Some(PlotConfigError::TooManyTicks(AxisName::X)));
    let config = real(
        PlotAxis::linear(0.0, 1.0)
            .with_major_interval(0.01)
            .with_minor(MinorTicks::Divisions(20)),
        PlotAxis::linear(0.0, 1.0),
    );
    assert_eq!(refusal(&config), Some(PlotConfigError::TooManyTicks(AxisName::X)));
    let many = (0..=MAX_TICKS).map(|i| (i as f64 / 1000.0, String::new())).collect();
    let config = real(
        PlotAxis::linear(0.0, 1.0).with_tick_labels(TickLabels::Custom(many)),
        PlotAxis::linear(0.0, 1.0),
    );
    assert_eq!(refusal(&config), Some(PlotConfigError::TooManyTicks(AxisName::X)));
}

#[test]
fn the_minor_ticks_must_suit_the_scale() {
    for minor in [
        MinorTicks::Divisions(0),
        MinorTicks::Divisions(MAX_MINOR_DIVISIONS + 1),
        MinorTicks::Decade,
    ] {
        let config = real(PlotAxis::linear(0.0, 1.0).with_minor(minor), PlotAxis::linear(0.0, 1.0));
        assert_eq!(refusal(&config), Some(PlotConfigError::Minor(AxisName::X)), "{minor:?}");
    }
    let config = real(
        PlotAxis::logarithmic(1.0, 10.0).with_minor(MinorTicks::Divisions(5)),
        PlotAxis::linear(0.0, 1.0),
    );
    assert_eq!(refusal(&config), Some(PlotConfigError::Minor(AxisName::X)));
    let config = real(
        PlotAxis::linear(0.0, 1.0).with_minor(MinorTicks::Divisions(MAX_MINOR_DIVISIONS)),
        PlotAxis::linear(0.0, 1.0),
    );
    assert_eq!(refusal(&config), None);
}

#[test]
fn a_custom_tick_must_be_a_usable_value() {
    let labels = |v: f64| TickLabels::Custom(vec![(v, "x".to_owned())]);
    let config = real(
        PlotAxis::linear(0.0, 1.0).with_tick_labels(labels(f64::NAN)),
        PlotAxis::linear(0.0, 1.0),
    );
    assert_eq!(refusal(&config), Some(PlotConfigError::CustomTick(AxisName::X)));
    let config = real(
        PlotAxis::logarithmic(1.0, 10.0).with_tick_labels(labels(0.0)),
        PlotAxis::linear(0.0, 1.0),
    );
    assert_eq!(refusal(&config), Some(PlotConfigError::CustomTick(AxisName::X)));
}

#[test]
fn an_axis_line_position_must_be_finite() {
    let config = real(
        PlotAxis::linear(0.0, 1.0).with_position(AxisPosition::At(f64::NAN)),
        PlotAxis::linear(0.0, 1.0),
    );
    assert_eq!(refusal(&config), Some(PlotConfigError::Position(AxisName::X)));
}

#[test]
fn the_background_must_be_a_hex_colour_because_the_grid_is_worked_out_from_it() {
    for background in ["lightsteelblue", "#12", "rgb(1,2,3)", ""] {
        assert_eq!(
            refusal(&unit().with_background(background)),
            Some(PlotConfigError::Background),
            "{background:?}"
        );
    }
    assert_eq!(refusal(&unit().with_background("#abc")), None);
}

#[test]
fn a_width_must_be_finite_and_above_zero() {
    let grid = PlotGrid {
        major_width: 0.0,
        ..PlotGrid::default()
    };
    assert_eq!(refusal(&unit().with_grid(grid)), Some(PlotConfigError::Width));
    let legend = vec![LegendEntry::line("a", "#000").with_width(f64::NAN)];
    assert_eq!(refusal(&unit().with_legend(legend)), Some(PlotConfigError::Width));
}

#[test]
fn margins_must_be_finite_and_not_negative() {
    for insets in [Insets::new(-1.0, 0.0, 0.0, 0.0), Insets::new(0.0, f64::NAN, 0.0, 0.0)] {
        assert_eq!(refusal(&unit().with_insets(insets)), Some(PlotConfigError::Insets));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Items and defaults
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_complex_number_is_a_point_and_so_is_a_pair() {
    assert_eq!(PlotPoint::from(Complex::new(3.0, -4.0)), PlotPoint::new(3.0, -4.0));
    assert_eq!(PlotPoint::from((1.0, 2.0)), PlotPoint::new(1.0, 2.0));
    let line = PlotLine::new([(0.0, 0.0), (1.0, 1.0)], "#f00").with_dash("4 2").with_width(3.0);
    assert_eq!(line.points.len(), 2);
    assert_eq!((line.width, line.dash.as_deref()), (3.0, Some("4 2")));
    let from_complex = PlotPoints::new([Complex::new(1.0, 1.0)], "#00f");
    assert_eq!(from_complex.points, vec![PlotPoint::new(1.0, 1.0)]);
}

#[test]
fn the_defaults_are_a_real_plot_with_linear_axes_and_the_pale_blue() {
    let config = unit();
    assert_eq!(config.plane, PlotPlane::Real);
    assert_eq!(config.x.scale, AxisScale::Linear);
    assert_eq!(config.background, "#eef4ff");
    assert!(config.grid.major && config.grid.minor);
    assert_eq!(PlotAxis::linear(0.0, 1.0).minor, MinorTicks::None);
    assert_eq!(PlotAxis::logarithmic(1.0, 10.0).minor, MinorTicks::Decade);
    assert_eq!(PlotPlane::default(), PlotPlane::Real);
    assert_eq!(AxisScale::default(), AxisScale::Linear);
}

#[test]
fn a_label_defaults_to_a_halo_and_to_the_start_of_its_text() {
    let label = PlotLabel::new((1.0, 2.0), "hi");
    assert!(label.halo);
    assert_eq!(label.anchor, LabelAnchor::Start);
    assert_eq!(label.offset, (0.0, 0.0));
}
