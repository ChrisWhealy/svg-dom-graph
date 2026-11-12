//! Mapping a value to a place on an axis and a place back to a value.

use super::*;

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
