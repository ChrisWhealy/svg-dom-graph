//! The items a plot holds, their defaults, and the finite place of anything however far off.

use super::*;

fn config_with(x: PlotAxis) -> PlotConfig {
    real(x, PlotAxis::linear(0.0, 1.0))
}

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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Extreme values
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_position_is_always_finite_however_far_off_the_axis_the_value_is() {
    let map = AxisMap::new(&PlotAxis::linear(0.0, 1.0), 0.0, 400.0);
    for value in [1e308, -1e308, f64::MAX, f64::MIN, 1e30] {
        let page = map.page_of(value).expect("a finite value can be placed");
        assert!(page.is_finite(), "{value} gave {page}");
        let fraction = map.finite_fraction(value).expect("placed");
        assert!(fraction.is_finite() && fraction.abs() <= map::FRACTION_LIMIT);
    }
    let log = AxisMap::new(&PlotAxis::logarithmic(1.0, 10.0), 0.0, 400.0);
    assert!(log.page_of(f64::MAX).expect("placed").is_finite());
}

#[test]
fn a_point_inside_a_plot_wider_than_a_billion_units_is_not_moved() {
    let map = AxisMap::new(&PlotAxis::linear(0.0, 1.0), 0.0, 4e9);
    assert_eq!(map.page_of(0.5), Some(2e9));
}

#[test]
fn a_segment_from_inside_the_plot_to_an_extremely_distant_point_is_cut_at_the_edge_with_finite_coordinates() {
    let layout = PlotLayout::new(&real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(0.0, 1.0))).expect("valid");
    let inside = layout.point(0.5, 0.5).expect("placed");
    for far in [
        (1e308, 0.5),
        (1e308, 1e308),
        (-1e308, -1e308),
        (0.5, f64::MAX),
        (f64::MAX, f64::MIN),
    ] {
        let runs = layout.clip_line(&[(0.5, 0.5), far]);
        assert_eq!(runs.len(), 1, "{far:?}");
        for (x, y) in &runs[0] {
            assert!(x.is_finite() && y.is_finite(), "{far:?} gave ({x}, {y})");
            assert!(
                near(x.clamp(layout.area.left, layout.area.right), *x)
                    && near(y.clamp(layout.area.top, layout.area.bottom), *y),
                "{far:?}"
            );
        }
        assert_eq!(runs[0][0], inside);
    }
    // Straight to the right, the cut is at the right edge and keeps the height.
    let runs = layout.clip_line(&[(0.5, 0.5), (1e308, 0.5)]);
    let end = runs[0][1];
    assert!(near(end.0, layout.area.right) && near(end.1, inside.1));
}

#[test]
fn a_line_to_a_point_far_off_in_one_direction_and_less_far_in_the_other_keeps_its_direction() {
    let layout = PlotLayout::new(&real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(0.0, 1.0))).expect("valid");
    let runs = layout.clip_line(&[(0.5, 0.5), (1e308, 1e100)]);
    let end = runs[0][1];
    let middle = layout.point(0.5, 0.5).expect("placed").1;
    assert!(near(end.0, layout.area.right));
    assert!(
        (end.1 - middle).abs() < 1e-6,
        "left the right edge at {} instead of near {middle}",
        end.1
    );
}

#[test]
fn a_segment_with_a_position_that_is_not_finite_is_dropped_and_not_turned_into_nan() {
    let bounds = Bounds {
        left: 0.0,
        top: 0.0,
        right: 10.0,
        bottom: 10.0,
    };
    assert_eq!(clip_segment((1.0, 1.0), (f64::INFINITY, 5.0), bounds), None);
    assert_eq!(clip_segment((f64::NAN, 1.0), (5.0, 5.0), bounds), None);
}

#[test]
fn an_axis_whose_span_overflows_is_refused() {
    let config = real(PlotAxis::linear(-1e308, 1e308), PlotAxis::linear(0.0, 1.0));
    assert_eq!(config.validate(), Err(PlotConfigError::Range(AxisName::X)));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
//   Very small ranges
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Checks that the major ticks of `axis` are `count` distinct values in order, each the multiple of `step` it should be.
fn assert_majors_are_multiples(axis: &PlotAxis, step: f64, count: usize) {
    let got = majors(axis);
    assert_eq!(got.len(), count, "{got:?}");
    assert!(got.windows(2).all(|w| w[0] < w[1]), "distinct and in order: {got:?}");
    for (k, value) in got.iter().enumerate() {
        let wanted = k as f64 * step;
        assert!((value - wanted).abs() <= 1e-12 * step, "tick {k}: {value:e} against {wanted:e}");
    }
}

#[test]
fn the_ticks_of_a_range_near_a_femto_unit_are_distinct_and_where_they_should_be() {
    let axis = PlotAxis::linear(0.0, 1e-15).with_major_interval(2e-16);
    assert_eq!(config_with(axis.clone()).validate(), Ok(()));
    assert_majors_are_multiples(&axis, 2e-16, 6);
}

#[test]
fn the_ticks_of_a_range_near_an_atto_unit_are_distinct_and_where_they_should_be() {
    let axis = PlotAxis::linear(0.0, 1e-18).with_major_interval(2e-19);
    assert_majors_are_multiples(&axis, 2e-19, 6);
    let split = PlotAxis::linear(0.0, 1e-18)
        .with_major_interval(5e-19)
        .with_minor(MinorTicks::Divisions(5));
    let all = values(&split);
    assert_eq!(all.len(), 11, "{all:?}");
    assert!(all.windows(2).all(|w| w[0] < w[1]), "no two minor ticks collapse: {all:?}");
}

#[test]
fn a_range_whose_every_value_is_subnormal_still_has_distinct_ticks() {
    // Subnormals have fewer digits, so the ticks are only as exact as they can be, and still apart and in order.
    let axis = PlotAxis::linear(0.0, 1e-320).with_major_interval(2e-321);
    let got = majors(&axis);
    // A subnormal has a step of about 5e-324, so 1e-320 is not exactly itself, and the last tick may or may not be on the axis.
    assert!((5..=6).contains(&got.len()), "{got:?}");
    assert!(got.windows(2).all(|w| w[0] < w[1]), "{got:?}");
    assert!(got.iter().all(|v| (0.0..=1.001e-320).contains(v)));
    assert_eq!(config_with(axis).validate(), Ok(()));
}

#[test]
fn an_automatic_interval_is_found_for_a_range_that_is_subnormal() {
    let axis = PlotAxis::linear(0.0, 1e-320);
    let got = majors(&axis);
    assert!((3..=12).contains(&got.len()), "{got:?}");
    assert!(got.windows(2).all(|w| w[0] < w[1]), "{got:?}");
    // A range of the smallest subnormal has no interval a double can hold, and is refused and never drawn.
    let smallest = config_with(PlotAxis::linear(0.0, 5e-324));
    assert!(matches!(smallest.validate(), Err(PlotConfigError::TooManyTicks(AxisName::X))));
    assert!(nice_interval(1e-300) > 0.0 && nice_interval(1e-320) > 0.0);
}

#[test]
fn a_small_range_far_from_zero_keeps_its_ticks_apart() {
    let axis = PlotAxis::linear(1e-15, 2e-15).with_major_interval(2.5e-16);
    let got = majors(&axis);
    assert_eq!(got.len(), 5, "{got:?}");
    assert!(got.windows(2).all(|w| w[0] < w[1]), "{got:?}");
}

#[test]
fn the_cleanup_removes_a_multiplication_artefact_and_nothing_a_double_can_tell_apart() {
    assert_eq!(crate::model::plot::ticks::clean_for_test(3.0, 0.1), 0.3);
    assert_eq!(crate::model::plot::ticks::clean_for_test(0.0, 0.1), 0.0);
    assert!(
        crate::model::plot::ticks::clean_for_test(0.0, -0.1).is_sign_positive(),
        "no negative zero"
    );
    let big = crate::model::plot::ticks::clean_for_test(3.0, 1e300);
    assert!(big.is_finite() && near(big, 3e300));
    // Neighbouring multiples of a tiny step stay neighbours.
    assert!(
        crate::model::plot::ticks::clean_for_test(2.0, 1e-300) < crate::model::plot::ticks::clean_for_test(3.0, 1e-300)
    );
}
