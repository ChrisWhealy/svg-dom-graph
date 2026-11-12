//! The margins, the labels and the arrowheads, and what the plane supplies.

use super::*;

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
