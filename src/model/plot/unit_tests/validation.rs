//! A configuration that is refused, and the message that says why.

use super::*;

fn refusal(config: &PlotConfig) -> Option<PlotConfigError> {
    config.validate().err()
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
