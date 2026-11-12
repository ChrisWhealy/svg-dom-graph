//! The areas under a line: where it crosses the baseline, and what an extreme value does to them.

use super::*;

fn run(points: &[(f64, f64)]) -> Vec<PlotPoint> {
    points.iter().map(|&p| p.into()).collect()
}

/// The area of the polygon through `corners`, by the shoelace formula.
fn polygon_area(corners: &[(f64, f64)]) -> f64 {
    let n = corners.len();
    (0..n)
        .map(|i| {
            let (a, b) = (corners[i], corners[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

#[test]
fn a_line_that_stays_above_the_baseline_is_one_piece_closed_along_it() {
    let found = pieces(&run(&[(0.0, 1.0), (1.0, 2.0)]), 0.0);
    assert_eq!(
        found,
        vec![Piece {
            above: true,
            corners: vec![(0.0, 0.0), (0.0, 1.0), (1.0, 2.0), (1.0, 0.0)],
        }]
    );
}

#[test]
fn a_line_that_stays_below_the_baseline_is_one_piece_that_says_so() {
    let found = pieces(&run(&[(0.0, -1.0), (1.0, -1.0)]), 0.0);
    assert_eq!(found.len(), 1);
    assert!(!found[0].above);
}

#[test]
fn a_line_is_cut_where_it_crosses_the_baseline() {
    let found = pieces(&run(&[(0.0, 1.0), (2.0, -1.0)]), 0.0);
    assert_eq!(found.len(), 2);
    assert!(found[0].above && !found[1].above);
    assert_eq!(found[0].corners, vec![(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (1.0, 0.0)]);
    assert_eq!(found[1].corners, vec![(1.0, 0.0), (1.0, 0.0), (2.0, -1.0), (2.0, 0.0)]);
}

#[test]
fn a_run_that_goes_on_over_several_points_is_one_piece() {
    let found = pieces(&run(&[(0.0, 0.0), (1.0, 1.0), (2.0, 1.0), (3.0, 0.0), (4.0, -1.0)]), 0.0);
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].corners.len(), 6, "the run of four points, and its two feet");
    assert!(found[0].above && !found[1].above);
}

#[test]
fn the_areas_of_the_pieces_are_the_integral_of_a_sine_above_and_below() {
    // One cycle of sin(2πt), in many segments: the area above is 1/π, and so is the area below.
    let n = 2000;
    let wave: Vec<PlotPoint> = (0..=n)
        .map(|i| {
            let t = f64::from(i) / f64::from(n);
            PlotPoint::new(t, (std::f64::consts::TAU * t).sin())
        })
        .collect();
    let found = pieces(&wave, 0.0);
    assert_eq!(found.len(), 2);
    for piece in &found {
        assert!((polygon_area(&piece.corners) - 1.0 / std::f64::consts::PI).abs() < 1e-4);
    }
}

#[test]
fn a_baseline_that_is_not_zero_is_the_line_the_pieces_are_cut_on() {
    let found = pieces(&run(&[(0.0, 1.0), (1.0, 3.0)]), 2.0);
    assert_eq!(found.len(), 2);
    assert!(!found[0].above && found[1].above);
    assert_eq!(found[0].corners[0], (0.0, 2.0));
    assert_eq!(found[1].corners.last(), Some(&(1.0, 2.0)));
}

#[test]
fn a_stretch_on_the_baseline_has_no_area_and_breaks_the_run() {
    let found = pieces(&run(&[(0.0, 1.0), (1.0, 0.0), (2.0, 0.0), (3.0, 1.0)]), 0.0);
    assert_eq!(found.len(), 2);
    assert!(pieces(&run(&[(0.0, 0.0), (1.0, 0.0)]), 0.0).is_empty());
}

#[test]
fn a_point_that_is_not_finite_breaks_the_area_in_two() {
    let found = pieces(&run(&[(0.0, 1.0), (1.0, 1.0), (2.0, f64::NAN), (3.0, 1.0), (4.0, 1.0)]), 0.0);
    assert_eq!(found.len(), 2);
    assert!(pieces(&run(&[(0.0, 1.0)]), 0.0).is_empty(), "one point has no area");
}

#[test]
fn a_line_that_touches_the_baseline_without_crossing_it_stays_on_its_side() {
    // Down to the baseline and up again: one piece above, whose middle corner is on the baseline.
    let found = pieces(&run(&[(0.0, 1.0), (1.0, 0.0), (2.0, 1.0)]), 0.0);
    assert_eq!(found.len(), 1);
    assert!(found[0].above);
    assert_eq!(
        found[0].corners,
        vec![(0.0, 0.0), (0.0, 1.0), (1.0, 0.0), (2.0, 1.0), (2.0, 0.0)]
    );
}

#[test]
fn a_line_that_passes_through_the_baseline_at_a_point_changes_side_there() {
    // The point on the baseline is the end of one piece and the start of the next, and it is not cut twice.
    let found = pieces(&run(&[(0.0, 1.0), (1.0, 0.0), (2.0, -1.0)]), 0.0);
    assert_eq!(found.len(), 2);
    assert!(found[0].above && !found[1].above);
    assert_eq!(found[0].corners.last(), Some(&(1.0, 0.0)));
    assert_eq!(found[1].corners.first(), Some(&(1.0, 0.0)));
    assert_eq!(found[0].corners[..3], [(0.0, 0.0), (0.0, 1.0), (1.0, 0.0)]);
}

#[test]
fn a_crossing_is_where_the_straight_line_between_the_points_meets_the_baseline() {
    // From (1, 3) to (4, -3) the line meets the axis at 2.5, and from (0, 3) to (2, 1) at a baseline of 2, at 1.
    let found = pieces(&run(&[(1.0, 3.0), (4.0, -3.0)]), 0.0);
    assert_eq!(found[0].corners.last(), Some(&(2.5, 0.0)));
    let found = pieces(&run(&[(0.0, 3.0), (2.0, 1.0)]), 2.0);
    assert_eq!(found[0].corners[1..3], [(0.0, 3.0), (1.0, 2.0)]);
    // A crossing that is almost at an end is still between the two points.
    let found = pieces(&run(&[(0.0, 1e-12), (1.0, -1.0)]), 0.0);
    let crossing = found[0].corners.last().map(|c| c.0);
    assert!(crossing.is_some_and(|x| (0.0..=1.0).contains(&x)), "{found:?}");
}

#[test]
fn two_finite_heights_too_far_apart_for_their_difference_to_be_a_number_are_still_cut_in_the_middle() {
    let found = pieces(&run(&[(0.0, 1e308), (1.0, -1e308), (2.0, -1.0)]), 0.0);
    assert_eq!(found.len(), 2);
    assert!(
        found
            .iter()
            .all(|p| p.corners.iter().all(|c| c.0.is_finite() && c.1.is_finite())),
        "{found:?}"
    );
    assert_eq!(
        found[0].corners.last(),
        Some(&(0.5, 0.0)),
        "half way, since the heights are as far either way"
    );
    assert!(found[0].above && !found[1].above);
}

/// Whether a piece is wholly on the side it says, which is what keeps the pieces apart.
fn on_its_side(piece: &Piece, baseline: f64) -> bool {
    piece
        .corners
        .iter()
        .all(|&(_, y)| if piece.above { y >= baseline } else { y <= baseline })
}

#[test]
fn a_crossing_between_heights_whose_product_is_less_than_the_least_number_is_still_found() {
    // 1e-200 times -2e-200 is -0.0, and -0.0 is not less than 0.0, but the segment does cross the baseline.
    let found = pieces(&run(&[(0.0, 1e-200), (1.0, -2e-200)]), 0.0);
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].above && !found[1].above);
    let at = found[0].corners.last().map(|c| c.0).unwrap_or(f64::NAN);
    assert!((at - 1.0 / 3.0).abs() < 1e-12, "{at}");
    assert!(found.iter().all(|p| on_its_side(p, 0.0)));
}

#[test]
fn a_crossing_is_found_at_every_magnitude_down_to_the_subnormal_numbers_and_in_both_directions() {
    let least = 5e-324;
    // Pairs of heights on either side of the baseline, and where between 0 and 1 the line meets it.
    let cases = [
        (1e-200, -2e-200, 1.0 / 3.0),
        (1e-300, -1e-300, 0.5),
        (1e-300, -3e-300, 0.25),
        (least, -3.0 * least, 0.25),
        (3.0 * least, -least, 0.75),
        (least, -least, 0.5),
        (1e-310, -2e-310, 1.0 / 3.0),
        (1e300, -1e300, 0.5),
        (1e300, -3e300, 0.25),
    ];
    for (above, below, meets) in cases {
        // The two directions are the line and its mirror, so the baseline is met in the same place and the pieces swap.
        let down = pieces(&run(&[(0.0, above), (1.0, below)]), 0.0);
        assert_eq!(down.len(), 2, "{above} to {below}: {down:?}");
        assert!(down[0].above && !down[1].above, "{above} to {below}");
        let at = down[0].corners.last().map(|c| c.0).unwrap_or(f64::NAN);
        assert!((at - meets).abs() < 1e-9, "{above} to {below}: crosses at {at}, not {meets}");
        assert!(down.iter().all(|p| on_its_side(p, 0.0)), "{above} to {below}: {down:?}");

        let up = pieces(&run(&[(0.0, below), (1.0, above)]), 0.0);
        assert_eq!(up.len(), 2, "{below} to {above}: {up:?}");
        assert!(!up[0].above && up[1].above, "{below} to {above}");
        let at = up[0].corners.last().map(|c| c.0).unwrap_or(f64::NAN);
        assert!((at - (1.0 - meets)).abs() < 1e-9, "{below} to {above}: crosses at {at}");
        assert!(up.iter().all(|p| on_its_side(p, 0.0)), "{below} to {above}: {up:?}");
    }
}

#[test]
fn a_crossing_is_between_its_two_points_however_far_apart_they_are_in_x() {
    let found = pieces(&run(&[(-1e308, 1.0), (1e308, -1.0)]), 0.0);
    assert_eq!(found.len(), 2);
    let at = found[0].corners.last().map(|c| c.0).unwrap_or(f64::NAN);
    assert!(at.is_finite() && at.abs() < 1e295, "{at}");
    let near_the_end = pieces(&run(&[(0.0, 1e300), (1.0, -5e-324)]), 0.0);
    let at = near_the_end[0].corners.last().map(|c| c.0).unwrap_or(f64::NAN);
    assert!((0.0..=1.0).contains(&at) && (at - 1.0).abs() < 1e-12, "{at}");
}

#[test]
fn a_baseline_that_is_not_zero_is_crossed_by_small_differences_too() {
    // Heights of 1 and 1 + 3e-200 are the same number, so they are on the baseline and do not cross it.
    assert!(pieces(&run(&[(0.0, 1.0), (1.0, 1.0 + 3e-200)]), 1.0).is_empty());
    // Round a baseline of 1e-200, a line from 2e-200 to 0 is above it and one from 0 to -1e-200 is below it.
    let found = pieces(&run(&[(0.0, 2e-200), (1.0, -1e-200)]), 1e-200);
    assert_eq!(found.len(), 2);
    assert!(found.iter().all(|p| on_its_side(p, 1e-200)));
}

fn log_and_linear() -> PlotLayout {
    PlotLayout::new(&real(PlotAxis::linear(0.0, 1.0), PlotAxis::logarithmic(1.0, 100.0))).expect("a valid plot")
}

#[test]
fn a_shaded_area_with_both_ends_of_a_segment_outside_the_plot_is_cut_to_what_crosses_it() {
    let layout = PlotLayout::new(&real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(-1.0, 1.0))).expect("a valid plot");
    // One segment from far left and far above to far right and far above: it never enters the plot, and the shading
    // under it, down to the axis, is a band that does.
    let wide = layout.clip_area(&run(&[(-5.0, 4.0), (6.0, 4.0)]), 0.0);
    assert_eq!(wide.len(), 1);
    assert!(
        (polygon_area(&wide[0].1)
            - (layout.area.right - layout.area.left) * (layout.area.bottom - layout.area.top) / 2.0)
            .abs()
            < 1e-6
    );
    // One that is wholly outside, with the baseline too, is nothing.
    assert!(layout.clip_area(&run(&[(2.0, 4.0), (3.0, 4.0)]), 3.0).is_empty());
    // A steep one, from far below to far above, crosses the whole height of the plot and the shading is cut by its edges.
    let steep = layout.clip_area(&run(&[(0.25, -9.0), (0.75, 9.0)]), 0.0);
    assert_eq!(steep.len(), 2);
    for (_, polygon) in &steep {
        assert!(polygon.iter().all(|&p| layout.area.contains((
            p.0.clamp(layout.area.left, layout.area.right),
            p.1.clamp(layout.area.top, layout.area.bottom)
        ))));
        assert!(
            polygon.iter().all(|&(x, y)| x >= layout.area.left - 1e-9
                && x <= layout.area.right + 1e-9
                && y >= layout.area.top - 1e-9
                && y <= layout.area.bottom + 1e-9),
            "{polygon:?}"
        );
    }
}

#[test]
fn a_point_that_cannot_be_placed_breaks_a_shaded_area_in_two() {
    let layout = log_and_linear();
    // On a logarithmic axis a height of zero or less cannot be placed.
    let found = layout.clip_area(&run(&[(0.0, 10.0), (0.2, 10.0), (0.4, 0.0), (0.6, 10.0), (0.8, 10.0)]), 1.0);
    assert_eq!(found.len(), 2, "one stretch before the point and one after");
    assert!(found.iter().all(|(above, _)| *above));
    let nan = layout.clip_area(
        &run(&[(0.0, 10.0), (0.2, 10.0), (0.4, f64::NAN), (0.6, 10.0), (0.8, 10.0)]),
        1.0,
    );
    assert_eq!(nan.len(), 2);
}

#[test]
fn a_baseline_that_cannot_be_placed_shades_nothing() {
    let layout = log_and_linear();
    let points = run(&[(0.0, 10.0), (1.0, 10.0)]);
    assert!(layout.clip_area(&points, 0.0).is_empty(), "zero on a logarithmic axis");
    assert!(layout.clip_area(&points, -5.0).is_empty(), "below zero on a logarithmic axis");
    assert!(layout.clip_area(&points, f64::NAN).is_empty());
    assert_eq!(layout.clip_area(&points, 1.0).len(), 1, "a baseline above zero is shaded to");
}

#[test]
fn a_shaded_area_on_a_logarithmic_axis_is_cut_where_it_crosses_the_baseline_in_the_values() {
    let layout = log_and_linear();
    // From 2 to 50 across a baseline of 10: one piece below it, and one above.
    let found = layout.clip_area(&run(&[(0.0, 2.0), (1.0, 50.0)]), 10.0);
    assert_eq!(found.len(), 2);
    assert!(!found[0].0 && found[1].0);
}

#[test]
fn a_shaded_area_is_never_given_a_corner_that_is_not_a_number_however_extreme_the_values() {
    let configs = [
        real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(-1.0, 1.0)),
        real(
            PlotAxis::linear(-1e-300, 1e-300).with_major_interval(1e-300),
            PlotAxis::linear(-1e-300, 1e-300).with_major_interval(1e-300),
        ),
        real(
            PlotAxis::linear(-1e300, 1e300).with_major_interval(1e299),
            PlotAxis::linear(-1e300, 1e300).with_major_interval(1e299),
        ),
        real(
            PlotAxis::logarithmic(1e-100, 1e100).with_minor(MinorTicks::None),
            PlotAxis::logarithmic(1e-100, 1e100).with_minor(MinorTicks::None),
        ),
    ];
    let extremes = [1e308, -1e308, f64::MAX, f64::MIN, 1e300, -1e-300, 1e-320, 0.0, 1.0];
    for config in configs {
        let layout = PlotLayout::new(&config).expect("a valid plot");
        for &a in &extremes {
            for &b in &extremes {
                for baseline in [0.0, 1.0, -1.0, 1e300, f64::MAX] {
                    let points = run(&[(0.1, a), (0.5, b), (0.9, a)]);
                    for (_, polygon) in layout.clip_area(&points, baseline) {
                        assert!(polygon.len() >= 3);
                        assert!(
                            polygon.iter().all(|&(x, y)| x.is_finite() && y.is_finite()),
                            "{a} {b} {baseline}: {polygon:?}"
                        );
                        let area = layout.area;
                        assert!(
                            polygon.iter().all(|&(x, y)| {
                                x >= area.left - 1e-6
                                    && x <= area.right + 1e-6
                                    && y >= area.top - 1e-6
                                    && y <= area.bottom + 1e-6
                            }),
                            "{a} {b} {baseline}: {polygon:?} is outside {area:?}"
                        );
                    }
                }
            }
        }
    }
}

const UNIT_BOX: Bounds = Bounds {
    left: 0.0,
    top: 0.0,
    right: 1.0,
    bottom: 1.0,
};

#[test]
fn a_polygon_inside_the_bounds_is_not_changed() {
    let square = [(0.25, 0.25), (0.75, 0.25), (0.75, 0.75), (0.25, 0.75)];
    assert_eq!(clip_polygon(&square, UNIT_BOX), square.to_vec());
}

#[test]
fn a_polygon_that_leaves_the_bounds_is_cut_at_the_edge() {
    let square = [(0.5, 0.5), (1.5, 0.5), (1.5, 1.5), (0.5, 1.5)];
    let cut = clip_polygon(&square, UNIT_BOX);
    assert!((polygon_area(&cut) - 0.25).abs() < 1e-12, "{cut:?}");
    assert!(cut.iter().all(|&p| UNIT_BOX.contains(p)));
}

#[test]
fn a_polygon_wholly_outside_the_bounds_is_nothing() {
    assert!(clip_polygon(&[(2.0, 2.0), (3.0, 2.0), (3.0, 3.0)], UNIT_BOX).is_empty());
}

#[test]
fn a_polygon_that_covers_the_bounds_is_the_bounds() {
    let cut = clip_polygon(&[(-1.0, -1.0), (2.0, -1.0), (2.0, 2.0), (-1.0, 2.0)], UNIT_BOX);
    assert!((polygon_area(&cut) - 1.0).abs() < 1e-12, "{cut:?}");
}

#[test]
fn a_polygon_with_a_point_that_is_not_finite_is_nothing() {
    assert!(clip_polygon(&[(0.0, 0.0), (f64::NAN, 0.5), (1.0, 1.0)], UNIT_BOX).is_empty());
    assert!(
        clip_polygon(&[(0.0, 0.0), (1.0, 1.0)], UNIT_BOX).is_empty(),
        "two points have no area"
    );
}

#[test]
fn the_layout_cuts_a_shaded_area_to_the_plotted_area() {
    let config = real(PlotAxis::linear(0.0, 1.0), PlotAxis::linear(-1.0, 1.0));
    let layout = PlotLayout::new(&config).expect("a valid plot");
    // A tall rectangle that goes far above and below the plot, between x of 0.25 and 0.75.
    let cut = layout.clip_polygon(&[(0.25, -9.0), (0.75, -9.0), (0.75, 9.0), (0.25, 9.0)]);
    assert!(!cut.is_empty());
    let (top, bottom) = (layout.area.top, layout.area.bottom);
    assert!(cut.iter().all(|&(_, y)| y >= top - 1e-9 && y <= bottom + 1e-9), "{cut:?}");
    let width = layout.area.right - layout.area.left;
    assert!((polygon_area(&cut) - 0.5 * width * (bottom - top)).abs() < 1e-6);
    assert!(layout.clip_polygon(&[(0.0, 0.0), (f64::NAN, 0.0), (1.0, 1.0)]).is_empty());
}
