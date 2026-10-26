//! Where a slider goes, and what its thumb does: pure arithmetic, so a plain `cargo test` runs it.

use super::*;

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn near_point(a: Point, b: Point) -> bool {
    near(a.x, b.x) && near(a.y, b.y)
}

/// Where `point` goes when it is turned `degrees` clockwise about `about`, as SVG's `rotate` does, with `y` pointing down.
fn turned(point: Point, about: Point, degrees: f64) -> Point {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (dx, dy) = (point.x - about.x, point.y - about.y);
    Point::new(about.x + dx * cos - dy * sin, about.y + dx * sin + dy * cos)
}

/// Where the centre of the thumb is, along the axis that runs from `start` to `end`, for a slider whose value is `value`,
/// between `min` and `max`: a native slider puts it that share of the way along. An empty range puts it at the start.
fn thumb_centre(start: Point, end: Point, min: f64, max: f64, value: f64) -> Point {
    let fraction = if max > min { ((value - min) / (max - min)).clamp(0.0, 1.0) } else { 0.0 };
    Point::new(start.x + (end.x - start.x) * fraction, start.y + (end.y - start.y) * fraction)
}

#[test]
fn a_horizontal_slider_is_one_thumb_longer_than_its_axis_so_the_thumb_centre_covers_the_axis() {
    let axis = frame(Point::new(300.0, 500.0), 360.0, Orientation::Horizontal);

    assert!(near(axis.size.width, 360.0 + THUMB) && near(axis.size.height, BAND));
    assert!(near(axis.top_left.x, 300.0 - (360.0 + THUMB) / 2.0));
    assert!(near(axis.top_left.y, 500.0 - BAND / 2.0));
    assert_eq!(axis.turn, None);

    // The thumb's centre is half a thumb in from each end of the element, which is the two ends of the axis.
    let (first, last) = (axis.top_left.x + THUMB / 2.0, axis.top_left.x + axis.size.width - THUMB / 2.0);
    assert!(near(first, 300.0 - 180.0) && near(last, 300.0 + 180.0));
}

#[test]
fn a_vertical_slider_is_turned_a_quarter_turn_anticlockwise_about_the_middle_of_its_axis() {
    let centre = Point::new(40.0, 220.0);
    let axis = frame(centre, 360.0, Orientation::Vertical);

    // Before the turn it is the same box as a horizontal one, about the same point.
    let level = frame(centre, 360.0, Orientation::Horizontal);
    assert_eq!((axis.top_left, axis.size), (level.top_left, level.size));
    assert_eq!(axis.turn, Some((-90.0, centre)));
}

#[test]
fn turned_a_quarter_turn_anticlockwise_the_left_end_is_at_the_bottom_and_the_right_end_at_the_top() {
    let centre = Point::new(40.0, 220.0);
    let axis = frame(centre, 360.0, Orientation::Vertical);
    let (degrees, about) = axis.turn.expect("a vertical slider is turned");

    // The thumb's centre at the minimum is the left end of the axis, and at the maximum the right end.
    let left = turned(Point::new(centre.x - 180.0, centre.y), about, degrees);
    let right = turned(Point::new(centre.x + 180.0, centre.y), about, degrees);
    assert!(
        near_point(left, Point::new(40.0, 400.0)),
        "the minimum is {left:?}, below the middle"
    );
    assert!(
        near_point(right, Point::new(40.0, 40.0)),
        "the maximum is {right:?}, above the middle"
    );
}

#[test]
fn turning_by_nothing_or_a_whole_turn_leaves_a_point_where_it_was() {
    let (point, about) = (Point::new(7.0, -3.0), Point::new(1.0, 2.0));

    assert!(near_point(turned(point, about, 0.0), point));
    assert!(near_point(turned(point, about, 360.0), point));
    assert!(near_point(turned(about, about, 123.0), about));
}

#[test]
fn the_thumb_is_where_the_value_is_on_the_axis() {
    let (start, end) = (Point::new(100.0, 50.0), Point::new(460.0, 50.0));

    assert!(near_point(thumb_centre(start, end, -5.0, 5.0, -5.0), start));
    assert!(near_point(thumb_centre(start, end, -5.0, 5.0, 5.0), end));
    assert!(near_point(thumb_centre(start, end, -5.0, 5.0, 0.0), Point::new(280.0, 50.0)));
    assert!(near_point(thumb_centre(start, end, -5.0, 5.0, 2.5), Point::new(370.0, 50.0)));
}

#[test]
fn the_thumb_stays_on_the_axis_whatever_the_value_and_the_range() {
    let (start, end) = (Point::new(0.0, 0.0), Point::new(100.0, 0.0));

    assert!(near_point(thumb_centre(start, end, 0.0, 10.0, -50.0), start));
    assert!(near_point(thumb_centre(start, end, 0.0, 10.0, 50.0), end));
    assert!(near_point(thumb_centre(start, end, 3.0, 3.0, 3.0), start), "an empty range");
    assert!(near_point(thumb_centre(start, end, 5.0, 0.0, 3.0), start), "a backwards range");
}

#[test]
fn a_value_is_snapped_to_the_step_and_never_a_negative_zero() {
    assert!(near(snapped(3.04, 0.1), 3.0));
    assert!(near(snapped(3.06, 0.1), 3.1));
    assert!(near(snapped(-2.96, 0.1), -3.0));
    assert!(near(snapped(1.0 / 3.0, 0.5), 0.5));

    let zero = snapped(-0.04, 0.1);
    assert!(zero == 0.0 && zero.is_sign_positive(), "snapping -0.04 gave {zero:?}");
}

#[test]
fn a_step_or_value_that_cannot_snap_leaves_the_value_alone() {
    assert!(near(snapped(1.234, 0.0), 1.234));
    assert!(near(snapped(1.234, -0.1), 1.234));
    assert!(near(snapped(1.234, f64::NAN), 1.234));
    assert!(snapped(f64::NAN, 0.1).is_nan());
    assert!(snapped(f64::INFINITY, 0.1).is_infinite());
}

#[test]
fn the_style_gives_the_thumb_the_width_the_frame_allows_for() {
    let css = style_sheet();
    let thumb = format!("width: {THUMB}px; height: {THUMB}px");

    // Both browsers' thumbs, and the track that is a thumb shorter than the input.
    assert_eq!(css.matches(&thumb).count(), 2, "{css}");
    assert!(css.contains(&format!("calc(100% - {THUMB}px)")));
}

#[test]
fn the_style_colours_the_thumbs_and_marks_focus_for_both_browsers() {
    let css = style_sheet();

    for selector in [
        ".argand-slider-real::-webkit-slider-thumb",
        ".argand-slider-real::-moz-range-thumb",
        ".argand-slider-imaginary::-webkit-slider-thumb",
        ".argand-slider-imaginary::-moz-range-thumb",
        ".argand-slider:focus-visible::-webkit-slider-thumb",
        ".argand-slider:focus-visible::-moz-range-thumb",
    ] {
        assert!(css.contains(selector), "no rule for {selector}");
    }
}
