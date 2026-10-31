use super::*;
use svg_dom_graph::scene::{PlotPlane, TickLabels};

const TOLERANCE: f64 = 1e-9;

fn lines(items: &[PlotItem]) -> Vec<&PlotLine> {
    items
        .iter()
        .filter_map(|item| if let PlotItem::Line(line) = item { Some(line) } else { None })
        .collect()
}

fn labels(items: &[PlotItem]) -> Vec<&str> {
    items
        .iter()
        .filter_map(|item| if let PlotItem::Label(label) = item { Some(label.text.as_str()) } else { None })
        .collect()
}

fn dotted_lines(items: &[PlotItem]) -> Vec<&PlotLine> {
    lines(items).into_iter().filter(|line| line.dash.is_some()).collect()
}

/// The page position of a value, in pixels, along an axis that runs from `low` to `high` across `length` pixels.
fn across(value: f64, low: f64, high: f64, length: f64) -> f64 {
    (value - low) / (high - low) * length
}

#[test]
fn the_circle_is_centred_with_a_margin_of_a_fifth_of_a_unit_all_round() {
    let config = circle_config();

    assert_eq!((config.size.width, config.size.height), (400.0, 400.0));
    for axis in [&config.x, &config.y] {
        assert_eq!((axis.min, axis.max), (-1.2, 1.2));
        assert_eq!(axis.position, AxisPosition::AtZero);
    }
    assert_eq!(config.plane, PlotPlane::Real);
}

fn size_labels(axis: &PlotAxis) -> Vec<(f64, String)> {
    let TickLabels::Custom(ticks) = &axis.tick_labels else {
        panic!("the ticks are given")
    };
    ticks.clone()
}

#[test]
fn the_size_axes_are_ticked_every_fifth_and_labelled_only_as_far_as_1() {
    let config = circle_config();

    for axis in [&config.x, &config.y] {
        let ticks = size_labels(axis);
        let values: Vec<f64> = ticks.iter().map(|(value, _)| *value).collect();
        assert_eq!(ticks.len(), 10, "five each side, and none at the crossing");
        assert!(
            values.iter().all(|v| v.abs() <= 1.0 + TOLERANCE),
            "nothing at the margin round the circle"
        );
        assert!(values.windows(2).all(|pair| pair[1] > pair[0]));
        assert_eq!(ticks[0].1, "\u{2212}1");
        assert_eq!(ticks[1].1, "\u{2212}0.8");
        assert_eq!(ticks[4].1, "\u{2212}0.2");
        assert_eq!(ticks[5].1, "0.2");
        assert_eq!(ticks[9].1, "1");
    }
}

#[test]
fn the_plots_that_share_a_size_axis_label_it_the_same_way() {
    assert_eq!(size_labels(&sine_config().y), size_labels(&circle_config().y));
    assert_eq!(size_labels(&cosine_config().x), size_labels(&circle_config().x));
}

#[test]
fn the_circle_has_no_margin_so_that_its_scales_can_be_shared() {
    assert_eq!(circle_config().insets, Some(Insets::new(0.0, 0.0, 0.0, 0.0)));
}

#[test]
fn the_angle_axes_have_room_at_their_ends_for_the_last_label_and_no_other_margin() {
    assert_eq!(sine_config().insets, Some(Insets::new(0.0, 0.0, END_MARGIN, 0.0)));
    assert_eq!(cosine_config().insets, Some(Insets::new(0.0, 0.0, 0.0, END_MARGIN)));
    for config in [circle_config(), sine_config(), cosine_config()] {
        config.validate().expect("the configuration can be drawn");
    }
}

#[test]
fn the_circle_and_the_sine_share_a_vertical_scale() {
    let (circle, sine) = (circle_config(), sine_config());

    assert_eq!(circle.size.height, sine.size.height);
    assert_eq!((circle.y.min, circle.y.max), (sine.y.min, sine.y.max));
    assert_eq!(
        sine.insets.map(|i| (i.top, i.bottom)),
        Some((0.0, 0.0)),
        "no room is taken from the height"
    );
}

#[test]
fn the_circle_and_the_cosine_share_a_horizontal_scale() {
    let (circle, cosine) = (circle_config(), cosine_config());

    assert_eq!(circle.size.width, cosine.size.width);
    assert_eq!((circle.x.min, circle.x.max), (cosine.x.min, cosine.x.max));
    assert_eq!(
        cosine.insets.map(|i| (i.left, i.right)),
        Some((0.0, 0.0)),
        "no room is taken from the width"
    );
}

#[test]
fn the_sine_and_the_cosine_are_the_circle_and_a_margin_at_the_end_of_the_angle_axis() {
    assert_eq!((sine_config().size.width, sine_config().size.height), (SIDE + END_MARGIN, SIDE));
    assert_eq!(
        (cosine_config().size.width, cosine_config().size.height),
        (SIDE, SIDE + END_MARGIN)
    );
}

#[test]
fn thirty_degrees_is_as_many_pixels_as_a_fifth_of_a_unit_on_the_circle() {
    let circle = circle_config();
    let fifth_of_a_unit = 0.2 / (circle.x.max - circle.x.min) * circle.size.width;
    let (sine, cosine) = (sine_config(), cosine_config());
    let along_sine = sine.size.width - END_MARGIN;
    let along_cosine = cosine.size.height - END_MARGIN;

    assert!((30.0 / (sine.x.max - sine.x.min) * along_sine - fifth_of_a_unit).abs() < TOLERANCE);
    assert!((30.0 / (cosine.y.max - cosine.y.min) * along_cosine - fifth_of_a_unit).abs() < TOLERANCE);
}

#[test]
fn the_sine_measures_the_angle_along_the_bottom_in_degrees_from_0_to_360() {
    let config = sine_config();

    assert_eq!((config.x.min, config.x.max), (0.0, 360.0));
    assert_eq!(config.x.tick_suffix.as_deref(), Some("\u{00B0}"));
    assert_eq!(config.x.position, AxisPosition::AtZero, "the horizontal axis is at height 0");
    assert_eq!(
        config.y.position,
        AxisPosition::AtMinimum,
        "the vertical axis is at the left edge"
    );
}

#[test]
fn the_cosine_has_its_horizontal_axis_at_the_top_and_its_angle_axis_down_the_middle() {
    let config = cosine_config();

    assert_eq!(config.x.position, AxisPosition::AtMaximum, "the top of the plot");
    assert_eq!(
        config.y.position,
        AxisPosition::AtZero,
        "the middle, since the width is 0 there"
    );
}

#[test]
fn the_angle_runs_down_the_cosine_and_is_labelled_with_positive_degrees() {
    let config = cosine_config();
    let TickLabels::Custom(ticks) = &config.y.tick_labels else {
        panic!("the ticks are given")
    };

    assert_eq!((config.y.min, config.y.max), (-360.0, 0.0));
    assert_eq!(ticks.len(), 13);
    assert_eq!(ticks[0], (-0.0, "0\u{00B0}".to_owned()));
    assert_eq!(ticks[12], (-360.0, "360\u{00B0}".to_owned()));
    assert!(ticks.iter().all(|(value, text)| text == &format!("{:.0}\u{00B0}", -value)));
}

#[test]
fn a_degree_is_the_same_distance_in_the_sine_and_in_the_cosine() {
    let (sine, cosine) = (sine_config(), cosine_config());
    let along_sine = sine.size.width - END_MARGIN;
    let along_cosine = cosine.size.height - END_MARGIN;

    assert_eq!(along_sine, along_cosine);
    assert_eq!(sine.x.max - sine.x.min, cosine.y.max - cosine.y.min);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_width_is_the_cosine_and_the_height_is_the_sine() {
    for (degrees, w, h) in [
        (0.0, 1.0, 0.0),
        (90.0, 0.0, 1.0),
        (180.0, -1.0, 0.0),
        (270.0, 0.0, -1.0),
        (30.0, 0.75_f64.sqrt(), 0.5),
    ] {
        assert!((width(degrees) - w).abs() < TOLERANCE, "width at {degrees}");
        assert!((height(degrees) - h).abs() < TOLERANCE, "height at {degrees}");
    }
}

#[test]
fn c_goes_round_anti_clockwise() {
    // Anti-clockwise from the right: it rises first, so the height grows and the width shrinks.
    assert!(height(10.0) > height(0.0));
    assert!(width(10.0) < width(0.0));
    assert!(
        width(100.0) < 0.0 && height(100.0) > 0.0,
        "the second quarter is up and to the left"
    );
}

#[test]
fn a_turn_takes_twelve_seconds_and_starts_again_at_zero() {
    assert!((turned(0.0, 1000.0) - 30.0).abs() < TOLERANCE);
    assert!((turned(0.0, 12_000.0)).abs() < TOLERANCE, "one whole turn is back at the start");
    assert!((turned(350.0, 1000.0) - 20.0).abs() < TOLERANCE, "it wraps past 360");
    assert!(turned(10.0, 0.0) == 10.0);
    assert!(turned(10.0, 5_000_000.0) < TURN);
}

#[test]
fn a_negative_zero_is_never_written() {
    for degrees in [-0.0, -0.4, 0.0, 0.4, 359.6, 360.0, -360.0, 720.0] {
        assert!(
            readout(degrees).starts_with("Angle 0\u{b0}\n"),
            "{degrees}: {}",
            readout(degrees)
        );
        assert!(whole(degrees).is_sign_positive(), "{degrees}");
    }
}

#[test]
fn a_direction_just_below_the_right_is_0_and_not_negative_zero() {
    // atan2 of a negative zero or a tiny negative height is a negative angle, which rounds to a negative zero.
    for dy in [-0.0, -0.001, 0.001] {
        let degrees = angle_of(5.0, dy).expect("a direction");
        assert_eq!(degrees, 0.0);
        assert!(degrees.is_sign_positive(), "dy {dy}");
    }
}

#[test]
fn the_keys_never_make_a_negative_zero() {
    for key in ["ArrowLeft", "ArrowDown", "PageDown", "ArrowRight", "Home"] {
        for from in [0.0, -0.0, 1.0, 15.0, 359.0] {
            assert!(key_angle(key, from).is_some_and(f64::is_sign_positive), "{key} from {from}");
        }
    }
}

#[test]
fn the_readout_shows_a_whole_degree_and_never_a_turn() {
    assert!(readout(359.6).starts_with("Angle 0\u{b0}\n"));
    assert!(readout(0.4).starts_with("Angle 0\u{b0}\n"));
    assert!(readout(44.6).starts_with("Angle 45\u{b0}\n"));
}

#[test]
fn the_angle_of_a_direction_counts_anti_clockwise_from_the_right_in_whole_degrees() {
    for (dx, dy, degrees) in [
        (1.0, 0.0, 0.0),
        (0.0, 1.0, 90.0),
        (-1.0, 0.0, 180.0),
        (0.0, -1.0, 270.0),
        (1.0, 1.0, 45.0),
        (1.0, -1.0, 315.0),
    ] {
        assert_eq!(angle_of(dx, dy), Some(degrees), "direction ({dx}, {dy})");
    }
    assert_eq!(angle_of(5.0, 0.001), Some(0.0), "rounded to a whole degree");
    assert_eq!(angle_of(5.0, -0.001), Some(0.0), "and a turn is 0, not 360");
}

#[test]
fn the_middle_of_the_circle_has_no_angle() {
    assert_eq!(angle_of(0.0, 0.0), None);
}

#[test]
fn dragging_round_a_circle_gives_the_angle_of_the_point_c_is_at() {
    for degrees in (0..360).map(f64::from) {
        let (dx, dy) = (width(degrees) * 70.0, height(degrees) * 70.0);
        assert_eq!(angle_of(dx, dy), Some(degrees));
    }
}

#[test]
fn the_arrow_keys_move_c_a_degree_and_the_page_keys_fifteen() {
    assert_eq!(key_angle("ArrowRight", 45.0), Some(46.0));
    assert_eq!(key_angle("ArrowUp", 45.0), Some(46.0));
    assert_eq!(key_angle("ArrowLeft", 45.0), Some(44.0));
    assert_eq!(key_angle("ArrowDown", 45.0), Some(44.0));
    assert_eq!(key_angle("PageUp", 45.0), Some(60.0));
    assert_eq!(key_angle("PageDown", 45.0), Some(30.0));
}

#[test]
fn home_and_end_go_to_the_first_and_the_last_whole_degree() {
    assert_eq!(key_angle("Home", 200.0), Some(0.0));
    assert_eq!(key_angle("End", 200.0), Some(359.0));
}

#[test]
fn the_keys_wrap_round_the_turn_in_both_directions() {
    assert_eq!(key_angle("ArrowRight", 359.0), Some(0.0));
    assert_eq!(key_angle("ArrowLeft", 0.0), Some(359.0));
    assert_eq!(key_angle("PageUp", 350.0), Some(5.0));
    assert_eq!(key_angle("PageDown", 5.0), Some(350.0));
}

#[test]
fn a_key_moves_c_from_the_whole_degree_it_is_shown_at() {
    assert_eq!(key_angle("ArrowRight", 44.6), Some(46.0), "C is shown at 45");
    assert_eq!(key_angle("ArrowLeft", 359.7), Some(359.0), "C is shown at 0 and not 360");
}

#[test]
fn other_keys_do_nothing() {
    for key in ["Tab", "Enter", " ", "a", "Escape", "Shift"] {
        assert_eq!(key_angle(key, 10.0), None, "{key}");
    }
}

#[test]
fn the_readout_names_the_angle_the_width_and_the_height() {
    assert_eq!(readout(60.0), "Angle 60\u{00B0}\nWidth  W = cos = 0.50\nHeight H = sin = 0.87");
    assert_eq!(
        readout(180.0),
        "Angle 180\u{00B0}\nWidth  W = cos = -1.00\nHeight H = sin = 0.00"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_circle_plot_has_the_circle_the_radius_and_two_dotted_lines() {
    let items = circle_items(60.0);
    let all = lines(&items);

    assert_eq!(all.len(), 4);
    assert_eq!(all[0].points.len(), 361, "the circle is a whole degree at a time");
    let (first, last) = (all[0].points[0], all[0].points[360]);
    assert!(
        (first.x - last.x).abs() < TOLERANCE && (first.y - last.y).abs() < TOLERANCE,
        "and it is closed"
    );
    assert_eq!(dotted_lines(&items).len(), 2);
}

#[test]
fn the_radius_runs_from_the_middle_to_c() {
    let items = circle_items(60.0);
    let radius = &lines(&items)[1].points;

    assert_eq!(radius[0], PlotPoint::new(0.0, 0.0));
    assert!((radius[1].x - 0.5).abs() < TOLERANCE && (radius[1].y - 0.75_f64.sqrt()).abs() < TOLERANCE);
}

#[test]
fn the_dotted_lines_of_the_circle_end_on_the_axes_at_w_and_h() {
    let items = circle_items(60.0);
    let dotted = dotted_lines(&items);
    let (w, h) = (width(60.0), height(60.0));

    assert_eq!(
        dotted[0].points,
        vec![PlotPoint::new(w, h), PlotPoint::new(w, 0.0)],
        "straight down to the horizontal axis"
    );
    assert_eq!(
        dotted[1].points,
        vec![PlotPoint::new(w, h), PlotPoint::new(0.0, h)],
        "straight across to the vertical axis"
    );
    let names = labels(&items);
    assert!(names.contains(&"W") && names.contains(&"H") && names.contains(&"C"));
}

#[test]
fn the_sine_trace_has_a_point_for_each_whole_degree_and_ends_at_c() {
    let items = sine_items(90.5);
    let trace = &lines(&items)[0].points;

    assert_eq!(trace.len(), 92, "the points 0 to 90, and C at 90.5");
    assert_eq!(trace[0], PlotPoint::new(0.0, 0.0));
    assert_eq!(trace[90], PlotPoint::new(90.0, 1.0));
    assert_eq!(trace[91], PlotPoint::new(90.5, height(90.5)));
}

#[test]
fn a_whole_angle_does_not_repeat_its_last_point() {
    let items = sine_items(45.0);

    assert_eq!(lines(&items)[0].points.len(), 46);
}

#[test]
fn the_sine_dotted_line_drops_from_c_to_the_axis_where_h_is() {
    let items = sine_items(120.0);
    let dotted = dotted_lines(&items);

    assert_eq!(dotted.len(), 1);
    assert_eq!(
        dotted[0].points,
        vec![PlotPoint::new(120.0, height(120.0)), PlotPoint::new(120.0, 0.0)]
    );
    assert!(labels(&items).contains(&"H"));
}

#[test]
fn the_cosine_trace_runs_down_so_the_angle_is_a_negative_value() {
    let items = cosine_items(90.5);
    let trace = &lines(&items)[0].points;

    assert_eq!(trace.len(), 92);
    assert_eq!(trace[0], PlotPoint::new(1.0, -0.0));
    assert_eq!(trace[90].y, -90.0);
    assert_eq!(trace[91], PlotPoint::new(width(90.5), -90.5));
}

#[test]
fn the_cosine_dotted_line_runs_across_from_c_to_the_vertical_axis() {
    let items = cosine_items(120.0);
    let dotted = dotted_lines(&items);

    assert_eq!(dotted.len(), 1);
    assert_eq!(
        dotted[0].points,
        vec![PlotPoint::new(width(120.0), -120.0), PlotPoint::new(0.0, -120.0)]
    );
    assert!(labels(&items).contains(&"W"));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn c_is_the_same_height_in_the_circle_and_in_the_sine_on_the_page() {
    // The pixel height of a value is the same in both plots, since they share a scale and a height.
    let (circle, sine) = (circle_config(), sine_config());
    for degrees in [0.0, 37.0, 90.0, 215.0, 359.0] {
        let h = height(degrees);
        let in_circle = across(h, circle.y.min, circle.y.max, circle.size.height);
        let in_sine = across(h, sine.y.min, sine.y.max, sine.size.height);
        assert!((in_circle - in_sine).abs() < TOLERANCE, "at {degrees} degrees");
    }
}

#[test]
fn c_is_the_same_distance_across_in_the_circle_and_in_the_cosine_on_the_page() {
    let (circle, cosine) = (circle_config(), cosine_config());
    for degrees in [0.0, 37.0, 90.0, 215.0, 359.0] {
        let w = width(degrees);
        let in_circle = across(w, circle.x.min, circle.x.max, circle.size.width);
        let in_cosine = across(w, cosine.x.min, cosine.x.max, cosine.size.width);
        assert!((in_circle - in_cosine).abs() < TOLERANCE, "at {degrees} degrees");
    }
}

#[test]
fn every_plot_has_its_own_description() {
    for config in [circle_config(), sine_config(), cosine_config()] {
        assert!(
            config
                .description
                .is_some_and(|text| text.contains('C') && !text.trim().is_empty())
        );
    }
}

#[test]
fn a_whole_turn_of_frames_draws_the_same_number_of_items_each_time() {
    // The plots are replaced by their items on every frame, so the items must be a fixed handful and a trace.
    for degrees in (0..360).map(f64::from) {
        assert_eq!(circle_items(degrees).len(), 10);
        assert_eq!(sine_items(degrees).len(), 6 + SINE_TITLE.len());
        assert_eq!(cosine_items(degrees).len(), 6 + COSINE_TITLE.len());
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The widest a letter of the title is, as a share of its size. A guess that is generous for the font that is used.
const LETTER: f64 = 0.65;

/// A box on a plot, in the values of its axes.
#[derive(Debug, Clone, Copy)]
struct Area {
    left: f64,
    right: f64,
    low: f64,
    high: f64,
}

impl Area {
    fn holds(&self, p: PlotPoint) -> bool {
        (self.left..=self.right).contains(&p.x) && (self.low..=self.high).contains(&p.y)
    }
}

/// The titles of `items`, as the box that holds all their lines, on a plot with `across` pixels for each unit of the
/// horizontal axis and `up` pixels for each unit of the vertical one. The titles are left-aligned.
fn title_area(items: &[PlotItem], across: f64, up: f64) -> Area {
    let titles: Vec<&PlotLabel> = items
        .iter()
        .filter_map(|item| match item {
            PlotItem::Label(label) if label.size == TITLE_SIZE => Some(label),
            _ => None,
        })
        .collect();
    let left = titles.iter().map(|l| l.at.x + l.offset.0 / across).fold(f64::MAX, f64::min);
    let right = titles
        .iter()
        .map(|l| l.at.x + (l.offset.0 + l.text.chars().count() as f64 * TITLE_SIZE * LETTER) / across)
        .fold(f64::MIN, f64::max);
    // The page counts down, so a label's baseline is its offset below its position, and the letters are above that.
    let low = titles.iter().map(|l| l.at.y - (l.offset.1 + 4.0) / up).fold(f64::MAX, f64::min);
    let high = titles
        .iter()
        .map(|l| l.at.y - (l.offset.1 - TITLE_SIZE) / up)
        .fold(f64::MIN, f64::max);
    Area { left, right, low, high }
}

/// Every place that the plot's contents can be at, for any angle: the whole curve, C, its dotted line, and the places
/// that the names of C, W and H are at. Not the titles.
fn everywhere(items_at: impl Fn(f64) -> Vec<PlotItem>) -> Vec<PlotPoint> {
    let mut points = Vec::new();
    for degrees in (0..360).map(f64::from) {
        for item in items_at(degrees) {
            match item {
                PlotItem::Line(line) => {
                    for pair in line.points.windows(2).take(if line.dash.is_some() { 1 } else { 400 }) {
                        for step in 0..=10 {
                            let t = f64::from(step) / 10.0;
                            points.push(PlotPoint::new(
                                pair[0].x + (pair[1].x - pair[0].x) * t,
                                pair[0].y + (pair[1].y - pair[0].y) * t,
                            ));
                        }
                    }
                },
                PlotItem::Points(dots) => points.extend(dots.points),
                PlotItem::Label(label) if label.size != TITLE_SIZE => points.push(label.at),
                _ => {},
            }
        }
    }
    points
}

#[test]
fn each_plot_has_a_title_that_says_what_it_shows() {
    let sine = sine_items(10.0);
    let cosine = cosine_items(10.0);

    assert_eq!(labels(&sine).iter().filter(|text| text.starts_with("Sine")).count(), 1);
    assert!(labels(&sine).iter().any(|text| text.contains("height H")));
    assert_eq!(labels(&cosine).iter().filter(|text| text.starts_with("Cosine")).count(), 1);
    assert!(labels(&cosine).iter().any(|text| text.contains("width W")));
}

#[test]
fn the_title_is_the_same_wherever_c_is() {
    let titles = |items: Vec<PlotItem>| -> Vec<PlotLabel> {
        items
            .into_iter()
            .filter_map(|item| match item {
                PlotItem::Label(label) if label.size == TITLE_SIZE => Some(label),
                _ => None,
            })
            .collect()
    };

    assert_eq!(titles(sine_items(5.0)), titles(sine_items(300.0)));
    assert_eq!(titles(cosine_items(5.0)), titles(cosine_items(300.0)));
    assert_eq!(titles(sine_items(5.0)).len(), SINE_TITLE.len());
    assert_eq!(titles(cosine_items(5.0)).len(), COSINE_TITLE.len());
}

#[test]
fn the_title_of_the_sine_is_where_the_curve_never_goes() {
    let (across, up) = ((LENGTH - END_MARGIN) / TURN, SIDE / (2.0 * SPAN));
    let area = title_area(&sine_items(0.0), across, up);

    assert!(
        area.high <= -1.0,
        "the curve is never below -1, and the title ends below {}",
        area.high
    );
    assert!(
        area.low >= -SPAN,
        "and it is inside the plot, which ends at -{SPAN}: {}",
        area.low
    );
    assert!(
        area.left >= 0.0 && area.right <= TURN,
        "and between its sides: {} to {}",
        area.left,
        area.right
    );
    for p in everywhere(sine_items) {
        assert!(!area.holds(p), "{p:?} is in the title {area:?}");
    }
}

#[test]
fn the_title_of_the_cosine_is_where_the_curve_and_c_never_go() {
    let (across, up) = (SIDE / (2.0 * SPAN), (LENGTH - END_MARGIN) / TURN);
    let area = title_area(&cosine_items(0.0), across, up);

    assert!(area.left >= -SPAN, "the title is inside the plot: {}", area.left);
    assert!(area.right < 0.0, "and left of the vertical axis: {}", area.right);
    assert!(area.low >= -TURN, "and above the end of the angle axis: {}", area.low);
    for p in everywhere(cosine_items) {
        assert!(!area.holds(p), "{p:?} is in the title {area:?}");
    }
}

#[test]
fn the_title_of_the_cosine_is_below_the_labels_of_the_horizontal_axis() {
    // The labels are along the top, and are about 22 pixels deep.
    let area = title_area(&cosine_items(0.0), SIDE / (2.0 * SPAN), (LENGTH - END_MARGIN) / TURN);

    assert!(
        -area.high * ((LENGTH - END_MARGIN) / TURN) >= 24.0,
        "the top of the title is {} degrees down",
        -area.high
    );
}

#[test]
fn the_title_of_the_cosine_clears_the_names_of_w_and_c_by_a_margin() {
    // The name W stands about NAME_GAP pixels from the vertical axis and is a letter wide, so the title has to end
    // further from the axis than that.
    let across = SIDE / (2.0 * SPAN);
    let area = title_area(&cosine_items(0.0), across, (LENGTH - END_MARGIN) / TURN);

    assert!(
        -area.right * across > NAME_GAP + NAME_SIZE,
        "ends {} pixels from the axis",
        -area.right * across
    );
}

#[test]
fn the_readout_is_three_lines_of_plain_text() {
    let lines: Vec<String> = readout(60.0).lines().map(str::to_owned).collect();

    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("Angle ") && lines[1].starts_with("Width  W") && lines[2].starts_with("Height H"));
    for degrees in [0.0, 90.0, 215.5, 359.9] {
        assert!(!readout(degrees).contains(['<', '>', '&']), "no markup at {degrees}");
    }
}

#[test]
fn the_names_in_the_readout_line_up_in_a_column() {
    let readout = readout(60.0);
    let columns: Vec<usize> = readout.lines().skip(1).filter_map(|line| line.find(" = ")).collect();

    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0], columns[1], "the equals signs are in the same column: {readout:?}");
}
