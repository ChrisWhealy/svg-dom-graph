use super::*;
use svg_dom_graph::scene::PlotPoints;

const TOLERANCE: f64 = 1e-9;

fn lines(items: &[PlotItem]) -> Vec<&PlotLine> {
    items
        .iter()
        .filter_map(|item| if let PlotItem::Line(line) = item { Some(line) } else { None })
        .collect()
}

fn labels(items: &[PlotItem]) -> Vec<&PlotLabel> {
    items
        .iter()
        .filter_map(|item| if let PlotItem::Label(label) = item { Some(label) } else { None })
        .collect()
}

fn dots(items: &[PlotItem]) -> Vec<&PlotPoints> {
    items
        .iter()
        .filter_map(|item| if let PlotItem::Points(dots) = item { Some(dots) } else { None })
        .collect()
}

#[test]
fn the_first_wave_is_a_sine_that_never_moves() {
    let still = lines(&phase_items(0.0))[0].points.clone();

    for shift in [1.0, 45.0, 90.0, 180.0, 359.0] {
        let items = phase_items(shift);
        let first = lines(&items)[0];
        assert_eq!(first.points, still, "at {shift}");
        assert_eq!(first.colour, HEIGHT, "the first wave is blue");
        assert_eq!(first.points.len(), 361);
        assert_eq!(first.points[0], PlotPoint::new(0.0, 0.0));
        assert!((first.points[90].y - 1.0).abs() < TOLERANCE, "it is at its crest at 90");
    }
}

#[test]
fn the_second_wave_is_the_first_with_the_shift_added_to_the_angle() {
    for shift in [0.0, 30.0, 45.0, 90.0, 270.0, 359.0] {
        let items = phase_items(shift);
        let second = lines(&items)[1];
        for (x, p) in second.points.iter().enumerate() {
            let expected = (x as f64 + shift).to_radians().sin();
            assert!((p.y - expected).abs() < TOLERANCE, "at {x} with a shift of {shift}");
            assert_eq!(p.x, x as f64);
        }
        assert_eq!(second.colour, SHIFTED);
    }
}

#[test]
fn the_two_waves_are_not_the_same_colour() {
    assert_ne!(HEIGHT, SHIFTED);
}

#[test]
fn with_no_shift_the_waves_are_one_on_the_other() {
    let items = phase_items(0.0);
    let waves = lines(&items);

    assert_eq!(waves[0].points, waves[1].points);
}

#[test]
fn a_shift_of_90_makes_the_second_wave_a_cosine() {
    let items = phase_items(90.0);
    let second = lines(&items)[1];

    for (x, p) in second.points.iter().enumerate() {
        assert!((p.y - width(x as f64)).abs() < TOLERANCE, "at {x}");
    }
}

#[test]
fn the_second_wave_starts_at_the_height_of_c() {
    for shift in [0.0, 45.0, 135.0, 200.0, 359.0] {
        let items = phase_items(shift);
        let second = lines(&items)[1];

        assert!((second.points[0].y - height(shift)).abs() < TOLERANCE, "at {shift}");
    }
}

#[test]
fn c_is_marked_on_the_second_wave_at_the_left_edge() {
    let items = phase_items(45.0);
    let c = dots(&items).into_iter().find(|dots| dots.colour == POINT).expect("a dot for C");

    assert_eq!(c.points.len(), 1);
    assert_eq!(c.points[0].x, 0.0);
    assert!((c.points[0].y - height(45.0)).abs() < TOLERANCE);
    assert!(labels(&items).iter().any(|label| label.text == "C"));
}

#[test]
fn the_shift_is_the_distance_along_the_axis_between_where_the_waves_rise_through_zero() {
    for shift in [1.0, 45.0, 180.0, 359.0] {
        let items = phase_items(shift);
        let line = lines(&items)
            .into_iter()
            .find(|line| line.dash.is_some())
            .expect("a dotted line");
        let (from, to) = (line.points[0], line.points[1]);

        assert_eq!((from.y, to.y), (0.0, 0.0), "along the axis");
        assert!((to.x - from.x - shift).abs() < TOLERANCE, "{shift}");
        assert!(
            sine(from.x + shift).abs() < 1e-9 && sine(from.x + shift + 1.0) > 0.0,
            "the second wave rises there"
        );
        assert!(labels(&items).iter().any(|label| label.text == "\u{03C6}"));
    }
}

#[test]
fn no_shift_has_no_dotted_line() {
    let items = phase_items(0.0);

    assert!(lines(&items).iter().all(|line| line.dash.is_none()));
    assert!(labels(&items).iter().all(|label| label.text != "\u{03C6}"));
    assert_eq!(rising_zero(0.0), None);
    assert_eq!(rising_zero(360.0), None);
}

#[test]
fn the_waves_are_named_in_their_own_colours() {
    let items = phase_items(45.0);
    let named: Vec<(&str, &str)> = labels(&items)
        .into_iter()
        .filter(|label| label.text.starts_with("sin"))
        .map(|label| (label.text.as_str(), label.colour.as_str()))
        .collect();

    assert_eq!(named, vec![("sin(x)", HEIGHT), ("sin(x + 45\u{b0})", SHIFTED)]);
}

#[test]
fn the_names_of_the_waves_are_below_where_either_wave_ever_goes() {
    let up = SIDE / (2.0 * SPAN);
    for shift in (0..360).map(f64::from) {
        let items = phase_items(shift);
        let lowest_wave = lines(&items)
            .iter()
            .filter(|line| line.dash.is_none())
            .flat_map(|line| line.points.iter().map(|p| p.y))
            .fold(f64::MAX, f64::min);
        for label in labels(&items).into_iter().filter(|label| label.text.starts_with("sin")) {
            // The text is centred on its position moved down by the offset, and its letters are about 0.4 of the size
            // above and below that.
            let middle = label.at.y - label.offset.1 / up;
            let (top, bottom) = (middle + 0.4 * TITLE_SIZE / up, middle - 0.4 * TITLE_SIZE / up);
            assert!(
                top < lowest_wave - 0.01,
                "{} at {shift}: {top} against {lowest_wave}",
                label.text
            );
            assert!(bottom > -SPAN, "{} at {shift} is inside the plot: {bottom}", label.text);
        }
    }
}

#[test]
fn the_names_are_at_the_left_of_the_plot_and_one_under_the_other() {
    let items = phase_items(45.0);
    let names: Vec<&PlotLabel> = labels(&items)
        .into_iter()
        .filter(|label| label.text.starts_with("sin"))
        .collect();

    assert_eq!(names.len(), 2);
    assert!(
        names
            .iter()
            .all(|label| label.at.x == 0.0 && label.anchor == LabelAnchor::Start)
    );
    assert!(names[1].at.y < names[0].at.y, "the second is lower");
}

#[test]
fn the_shift_is_written_as_a_whole_number_of_degrees() {
    assert_eq!(shifted_name(0.0), "sin(x + 0\u{b0})");
    assert_eq!(shifted_name(359.0), "sin(x + 359\u{b0})");
    assert!(!shifted_name(0.0).contains("-0"));
}

#[test]
fn every_shift_draws_the_same_items_but_the_dotted_line() {
    for shift in (1..360).map(f64::from) {
        assert_eq!(phase_items(shift).len(), phase_items(1.0).len(), "at {shift}");
    }
}

#[test]
fn the_sine_plot_is_the_one_of_the_sine_and_cosine_demo_and_the_circle_too() {
    let (circle, sine) = (phase_circle_config(), phase_sine_config());

    assert_eq!((circle.size.width, circle.size.height), (SIDE, SIDE));
    assert_eq!(
        sine.size.height, circle.size.height,
        "the height is the circle's, so the scales are shared"
    );
    assert_eq!((sine.y.min, sine.y.max), (circle.y.min, circle.y.max));
    assert!(circle.description.is_some_and(|text| text.contains("phase shift")));
    assert!(sine.description.is_some_and(|text| text.contains("phase shift")));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The arc on the circle: the line in the colour of the second wave, which is not dotted and not the wave.
fn arc_of(items: &[PlotItem]) -> Option<&PlotLine> {
    lines(items)
        .into_iter()
        .find(|line| line.colour == SHIFTED && line.dash.is_none() && line.width == ARC_WIDTH)
}

/// The angle of `p` from the middle of the circle, in degrees from 0 up to but not including 360.
fn angle_of_point(p: PlotPoint) -> f64 {
    p.y.atan2(p.x).to_degrees().rem_euclid(360.0)
}

#[test]
fn the_circle_has_everything_that_the_sine_and_cosine_circle_has_and_an_arc() {
    for shift in [0.0, 45.0, 180.0, 270.0] {
        let plain = circle_items(shift);
        let with_arc = phase_circle_items(shift);

        assert_eq!(&with_arc[..plain.len()], &plain[..], "at {shift}");
        assert_eq!(with_arc.len(), plain.len() + 2, "an arc and its text");
    }
}

#[test]
fn the_arc_is_near_the_middle_of_the_circle_and_at_one_radius() {
    let items = phase_circle_items(100.0);
    let arc = arc_of(&items).expect("an arc");

    for p in &arc.points {
        assert!((p.x.hypot(p.y) - ARC_RADIUS).abs() < TOLERANCE, "{p:?}");
    }
}

#[test]
fn the_arc_runs_from_the_positive_horizontal_axis_to_c() {
    for shift in [1.0, 45.0, 90.0, 179.0, 181.0, 270.0, 359.0] {
        let items = phase_circle_items(shift);
        let arc = &arc_of(&items).expect("an arc").points;
        let (first, last) = (arc[0], arc[arc.len() - 1]);

        assert!(
            (first.x - ARC_RADIUS).abs() < TOLERANCE && first.y.abs() < TOLERANCE,
            "starts on the axis at {shift}"
        );
        assert!((angle_of_point(last) - shift).abs() < 1e-6, "ends at the angle of C: {shift}");
    }
}

#[test]
fn an_angle_of_more_than_180_degrees_is_drawn_the_long_way_round() {
    let items = phase_circle_items(270.0);
    let arc = &arc_of(&items).expect("an arc").points;

    assert_eq!(arc.len(), 271, "a point for every degree, and not for the 90 that are left");
    assert!(arc.iter().any(|p| p.y > 0.1), "it goes over the top");
    assert!(
        arc.iter().any(|p| p.x < -0.1 && p.y < -0.1),
        "and down the other side, past the left"
    );
    let lowest_left = arc.iter().map(|p| p.x).fold(f64::MAX, f64::min);
    assert!(
        (lowest_left + ARC_RADIUS).abs() < 1e-3,
        "it passes through the far left, at 180 degrees"
    );
    // The angles never go down: it is one sweep and not the short angle on the other side.
    let angles: Vec<f64> = arc.iter().skip(1).map(|p| angle_of_point(*p)).collect();
    assert!(angles.windows(2).all(|pair| pair[1] > pair[0]));
}

#[test]
fn an_angle_of_359_degrees_has_a_point_for_every_degree() {
    let items = phase_circle_items(359.0);

    assert_eq!(arc_of(&items).expect("an arc").points.len(), 360);
}

#[test]
fn with_no_angle_the_arc_has_a_single_point_so_nothing_is_drawn() {
    let items = phase_circle_items(0.0);

    assert_eq!(arc_of(&items).expect("an arc").points.len(), 1);
}

#[test]
fn the_angle_is_written_beside_the_arc_in_the_colour_of_the_arc() {
    for (shift, text) in [
        (0.0, "\u{3c6}=0\u{b0}"),
        (45.0, "\u{3c6}=45\u{b0}"),
        (270.0, "\u{3c6}=270\u{b0}"),
    ] {
        let items = phase_circle_items(shift);
        let label = labels(&items)
            .into_iter()
            .find(|label| label.text.starts_with('\u{3c6}'))
            .expect("the angle");

        assert_eq!(label.text, text);
        assert_eq!(label.colour, SHIFTED);
        assert_eq!(angle_text(shift), text);
    }
}

#[test]
fn the_text_of_the_angle_is_outside_the_arc_and_inside_the_circle_on_the_line_that_splits_the_angle() {
    for shift in [10.0, 45.0, 90.0, 179.0, 181.0, 270.0, 350.0] {
        let items = phase_circle_items(shift);
        let label = labels(&items)
            .into_iter()
            .find(|label| label.text.starts_with('\u{3c6}'))
            .expect("the angle");
        let radius = label.at.x.hypot(label.at.y);

        assert!(radius > ARC_RADIUS && radius < 1.0, "{radius} at {shift}");
        assert!(
            (angle_of_point(label.at) - shift / 2.0).abs() < 1e-6,
            "on the half way line at {shift}"
        );
    }
}

#[test]
fn for_an_angle_of_more_than_180_degrees_the_text_is_on_the_far_side_of_the_middle_from_c() {
    let items = phase_circle_items(270.0);
    let label = labels(&items)
        .into_iter()
        .find(|label| label.text.starts_with('\u{3c6}'))
        .expect("the angle");

    assert!(
        label.at.y > 0.0 && label.at.x < 0.0,
        "up and to the left, where C is down: {:?}",
        label.at
    );
}

#[test]
fn the_text_is_clear_of_c_and_of_the_names_of_w_and_h_for_a_shift_of_45_degrees() {
    // C is at 45 degrees, at radius 1, and the text is half way between 0 and 45 degrees, at a radius of about a half.
    let items = phase_circle_items(45.0);
    let label = labels(&items)
        .into_iter()
        .find(|label| label.text.starts_with('\u{3c6}'))
        .expect("the angle");
    let c = PlotPoint::new(width(45.0), height(45.0));

    assert!((label.at.x - c.x).hypot(label.at.y - c.y) > 0.4);
}
