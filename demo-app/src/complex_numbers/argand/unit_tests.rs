//! The arithmetic of the Argand diagram: what the plot node is configured with, what is drawn on it, which way an arc
//! runs, what a tick step is, and how a number is written. None touches the DOM, so all run under a plain `cargo test`.

use super::*;
use std::f64::consts::FRAC_PI_2;
use svg_dom_graph::scene::{PlotPlane, TickLabelPlacement};

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn diagram() -> Argand {
    Argand {
        side: 400.0,
        extent: 5.5,
        unit_circle: false,
        marks: Vec::new(),
        description: "an Argand diagram".to_owned(),
    }
}

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

#[test]
fn the_plot_node_is_a_complex_plane_with_a_square_plotted_area_and_arrowheads() {
    let config = diagram().config();

    assert_eq!(config.plane, PlotPlane::Complex);
    assert_eq!(
        config.size,
        Size::new(428.0, 428.0),
        "the plotted area and a margin of PAD all round"
    );
    assert_eq!((config.x.min, config.x.max), (-5.5, 5.5));
    assert_eq!((config.y.min, config.y.max), (-5.5, 5.5));
    assert!(config.x.arrow && config.y.arrow);
    assert_eq!(config.x.major_interval, Some(1.0));
    assert_eq!(config.description.as_deref(), Some("an Argand diagram"));
    assert_eq!(
        config.label_placement, None,
        "a complex plane puts its labels beside the axes of its own accord"
    );
    let _ = TickLabelPlacement::Axis;
}

#[test]
fn a_unit_of_the_plane_is_as_many_user_units_as_the_side_over_the_span() {
    assert!(near(diagram().scale(), 400.0 / 11.0));
}

#[test]
fn the_tick_step_grows_with_the_extent() {
    assert!(near(tick_step(1.45), 0.5));
    assert!(near(tick_step(2.0), 0.5));
    assert!(near(tick_step(4.5), 1.0));
    assert!(near(tick_step(6.0), 1.0));
    assert!(near(tick_step(10.0), 2.0));
}

#[test]
fn an_arc_starts_and_ends_where_it_is_told_and_stays_on_its_circle() {
    let arc = arc_points(2.0, 0.0, FRAC_PI_2, 8);

    assert_eq!(arc.len(), 9);
    assert!(near(arc[0].re, 2.0) && near(arc[0].im, 0.0));
    assert!(near(arc[8].re, 0.0) && near(arc[8].im, 2.0));
    assert!(arc.iter().all(|z| near(z.norm(), 2.0)));
}

#[test]
fn an_arc_to_a_lesser_angle_runs_clockwise_below_the_real_axis() {
    let arc = arc_points(1.0, 0.0, -FRAC_PI_2, 4);

    assert!(arc[1..].iter().all(|z| z.im < 0.0));
}

#[test]
fn numbers_are_written_without_trailing_zeros_or_a_negative_zero() {
    assert_eq!(trimmed(3.0, 2), "3");
    assert_eq!(trimmed(0.5, 2), "0.5");
    assert_eq!(trimmed(53.130_102, 2), "53.13");
    assert_eq!(trimmed(-0.0001, 2), "0");
    assert_eq!(trimmed(-0.0, 2), "0");
    assert_eq!(trimmed(-2.5, 2), "-2.5");
    assert_eq!(trimmed(100.0, 0), "100");
}

#[test]
fn a_number_is_written_with_a_plus_or_a_minus() {
    assert_eq!(rectangular_text(Complex::new(3.0, 4.0), 2), "3 + 4i");
    assert_eq!(rectangular_text(Complex::new(-3.0, -2.5), 2), "-3 - 2.5i");
    assert_eq!(rectangular_text(Complex::new(0.0, 0.0), 2), "0 + 0i");
    // A part that rounds to zero is not a subtraction.
    assert_eq!(rectangular_text(Complex::new(1.0, -0.0001), 2), "1 + 0i");
}

#[test]
fn a_label_is_pushed_away_from_the_origin_on_the_side_it_faces() {
    let ((dx, _), anchor) = outward_offset(0.0, 10.0);
    assert!(near(dx, 10.0));
    assert_eq!(anchor, LabelAnchor::Start);

    let ((dx, _), anchor) = outward_offset(PI, 10.0);
    assert!(near(dx, -10.0));
    assert_eq!(anchor, LabelAnchor::End);

    // Straight up is above the point, so a negative move down the page, and centred.
    let ((dx, dy), anchor) = outward_offset(FRAC_PI_2, 10.0);
    assert!(dy < -9.0 && near(dx, 0.0));
    assert_eq!(anchor, LabelAnchor::Middle);

    let ((_, dy), _) = outward_offset(-FRAC_PI_2, 10.0);
    assert!(dy > 9.0);
}

#[test]
fn the_guides_say_which_marks_they_draw() {
    assert!(!Guides::None.rectangular() && !Guides::None.polar());
    assert!(Guides::Both.rectangular() && Guides::Both.polar());
}

#[test]
fn a_line_label_is_moved_to_the_side_that_was_asked_for_and_turned_to_run_along_the_line() {
    // Along the real axis the anticlockwise side is up the page: a negative move. No turn.
    let (above, turn) = line_label(0.0, 10.0, 1.0);
    assert!(near(above.0, 0.0) && near(above.1, -10.0) && near(turn, 0.0));
    let (below, _) = line_label(0.0, 10.0, -1.0);
    assert!(near(below.1, 10.0));

    // Along the imaginary axis the anticlockwise side is to its left, and the text is turned a quarter turn anticlockwise,
    // which on the page is -90.
    let (left, turn) = line_label(FRAC_PI_2, 10.0, 1.0);
    assert!(near(left.0, -10.0) && near(left.1, 0.0) && near(turn, -90.0));
}

#[test]
fn a_line_label_is_never_upside_down() {
    for degrees in (-179..=180).step_by(7) {
        let (_, turn) = line_label(f64::from(degrees).to_radians(), 0.0, 1.0);
        // Whole turns do not matter, so bring the turn into -180 to 180. Upright text is within a quarter turn of level.
        let turn = (turn + 180.0).rem_euclid(360.0) - 180.0;
        assert!(turn.abs() <= 90.0 + 1e-9, "{degrees} degrees turns the text {turn}");
    }
}

#[test]
fn the_adjacent_label_is_below_the_axis_for_a_number_above_it_and_above_for_one_below() {
    assert!(adjacent_offset(Complex::new(2.0, 1.0)) > 0.0);
    assert!(adjacent_offset(Complex::new(2.0, -1.0)) < 0.0);
    assert!(adjacent_offset(Complex::new(2.0, 0.0)) > 0.0);
}

#[test]
fn the_opposite_label_is_beside_its_leg_and_away_from_the_origin() {
    let (right, anchor) = opposite_offset(Complex::new(2.0, 1.0));
    assert!(right > 0.0 && anchor == LabelAnchor::Start);
    let (left, anchor) = opposite_offset(Complex::new(-2.0, -1.0));
    assert!(left < 0.0 && anchor == LabelAnchor::End);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_number_with_both_guides_has_the_legs_the_modulus_the_arc_and_the_dot() {
    let mut argand = diagram();
    argand.marks = vec![Mark {
        z: Complex::new(3.0, 4.0),
        label: "3 + 4i".to_owned(),
        guides: Guides::Both,
    }];
    let items = argand.items();

    // The adjacent leg, the opposite leg, the modulus and the angle arc.
    let drawn = lines(&items);
    assert_eq!(drawn.len(), 4);
    assert_eq!(drawn[0].colour, REAL_STROKE);
    assert_eq!(drawn[1].colour, IMAGINARY_STROKE);
    assert_eq!(drawn[2].colour, MODULUS_STROKE);
    assert_eq!(drawn[3].colour, ANGLE_STROKE);
    // The legs make a right angle at the point on the real axis below the number.
    assert_eq!(drawn[0].points.last(), drawn[1].points.first());
    assert_eq!(drawn[1].points.last().map(|p| (p.x, p.y)), Some((3.0, 4.0)));

    let texts: Vec<&str> = labels(&items).iter().map(|l| l.text.as_str()).collect();
    assert_eq!(texts, ["a = 3", "b = 4", "r = 5", "\u{3b8} = 53.13\u{b0}", "3 + 4i"]);
    assert!(
        matches!(items.last(), Some(PlotItem::Label(_))),
        "the label of the number is last"
    );
    assert!(items.iter().any(|item| matches!(item, PlotItem::Points(_))), "the dot");
}

#[test]
fn a_number_with_no_guides_is_a_dot_and_its_label() {
    let mut argand = diagram();
    argand.marks = vec![Mark {
        z: Complex::new(1.0, 1.0),
        label: "w1".to_owned(),
        guides: Guides::None,
    }];
    let items = argand.items();

    assert!(lines(&items).is_empty());
    assert_eq!(items.len(), 2);
}

#[test]
fn the_unit_circle_is_a_dashed_line_that_closes_on_itself_with_a_spoke_to_each_number() {
    let mut argand = diagram();
    argand.unit_circle = true;
    argand.marks = vec![Mark {
        z: Complex::new(0.0, 1.0),
        label: "w2".to_owned(),
        guides: Guides::None,
    }];
    let items = argand.items();
    let drawn = lines(&items);

    assert_eq!(drawn.len(), 2);
    assert_eq!(drawn[0].dash.as_deref(), Some("5 4"));
    let (first, last) = (drawn[0].points[0], drawn[0].points[CIRCLE_SEGMENTS]);
    assert!(
        near(first.x, last.x) && near(first.y, last.y),
        "a whole turn ends where it began"
    );
    assert!(drawn[0].points.iter().all(|p| near(p.x.hypot(p.y), 1.0)));
    assert_eq!(drawn[1].points.len(), 2, "the spoke");
    let dots: Vec<_> = items.iter().filter(|item| matches!(item, PlotItem::Points(_))).collect();
    assert_eq!(dots.len(), 1);
}

#[test]
fn the_angle_arc_is_no_bigger_than_its_cap_and_follows_a_short_modulus() {
    let mut argand = diagram();
    argand.marks = vec![Mark {
        z: Complex::new(5.0, 5.0),
        label: String::new(),
        guides: Guides::Both,
    }];
    let long = lines(&argand.items())[3].points.clone();
    let radius_in_user_units = long[0].x.hypot(long[0].y) * argand.scale();
    assert!(
        near(radius_in_user_units, MAX_ARC_RADIUS),
        "capped at {MAX_ARC_RADIUS}, got {radius_in_user_units}"
    );

    argand.marks[0].z = Complex::new(0.5, 0.5);
    let short = lines(&argand.items())[3].points.clone();
    assert!(short[0].x.hypot(short[0].y) * argand.scale() < MAX_ARC_RADIUS);
}

#[test]
fn a_number_at_the_origin_has_no_modulus_and_no_angle_to_draw() {
    let mut argand = diagram();
    argand.marks = vec![Mark {
        z: Complex::ZERO,
        label: "0".to_owned(),
        guides: Guides::Both,
    }];
    let items = argand.items();

    // Only the two legs, which are as long as nothing.
    assert_eq!(lines(&items).len(), 2);
    assert!(
        !labels(&items)
            .iter()
            .any(|l| l.text.starts_with("r =") || l.text.starts_with('\u{3b8}'))
    );
}

#[test]
fn the_modulus_label_runs_along_the_line_and_is_on_the_far_side_from_the_arc() {
    let mut argand = diagram();
    argand.marks = vec![Mark {
        z: Complex::new(3.0, -4.0),
        label: String::new(),
        guides: Guides::Both,
    }];
    let items = argand.items();
    let modulus = labels(&items)
        .into_iter()
        .find(|l| l.text.starts_with("r ="))
        .expect("a modulus label");

    assert_eq!(modulus.anchor, LabelAnchor::Middle);
    assert!(modulus.rotation.abs() <= 90.0);
    // A number below the axis has its arc below the axis too, so the label goes to the other side of the line.
    let (aside, _) = line_label((-4.0f64).atan2(3.0), 12.0, -1.0);
    assert_eq!(modulus.offset, aside);
}
