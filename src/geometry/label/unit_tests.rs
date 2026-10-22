use super::{
    ARROW_REACH, Conflicts, MAX_CANDIDATES, OPPOSITE_SIDE_PENALTY, PlacerInput, SWITCH_THRESHOLD, candidates, choose,
    place, reach, soft_score,
};
use svg_dom::root::utils::{Point, Rect, Size};

const LABEL: Size = Size { width: 20.0, height: 12.0 };

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        origin: p(x, y),
        size: Size::new(width, height),
    }
}

fn centre(rect: Rect) -> Point {
    p(rect.origin.x + rect.size.width / 2.0, rect.origin.y + rect.size.height / 2.0)
}

fn input<'a>(route: &'a [Point], vertices: &'a [Rect], others: &'a [Rect], previous: Option<usize>) -> PlacerInput<'a> {
    PlacerInput {
        route,
        corner_radius: 0.0,
        label: LABEL,
        vertices,
        others,
        previous,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_horizontal_run_has_its_label_above_the_middle_of_its_usable_part() {
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    let all = candidates(&route, 0.0, LABEL);
    let first = all[0];
    // The usable part is 6 to 186, so its middle is at 96.
    assert_eq!(first.anchor, p(96.0, 100.0));
    assert!(first.preferred);
    assert_eq!(centre(first.rect).x, 96.0);
    assert!(
        first.rect.origin.y + first.rect.size.height < 100.0,
        "above the line, with a gap"
    );
}

#[test]
fn a_vertical_run_has_its_label_to_the_right() {
    let route = [p(50.0, 0.0), p(50.0, 200.0)];
    let first = candidates(&route, 0.0, LABEL)[0];
    assert!(first.rect.origin.x > 50.0, "right of the line");
    assert_eq!(centre(first.rect).y, first.anchor.y);
}

#[test]
fn a_run_that_runs_upwards_still_prefers_the_right() {
    let route = [p(50.0, 200.0), p(50.0, 0.0)];
    assert!(candidates(&route, 0.0, LABEL)[0].rect.origin.x > 50.0);
}

#[test]
fn a_run_that_runs_leftwards_still_prefers_above() {
    let route = [p(200.0, 100.0), p(0.0, 100.0)];
    let first = candidates(&route, 0.0, LABEL)[0];
    assert!(first.rect.origin.y + first.rect.size.height < 100.0);
}

#[test]
fn the_opposite_side_is_the_other_side_of_the_same_anchor() {
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    let all = candidates(&route, 0.0, LABEL);
    let (above, below) = (all[0], all[3]);
    assert_eq!(above.anchor, below.anchor);
    assert!(!below.preferred);
    assert!(below.rect.origin.y > 100.0, "below the line");
}

#[test]
fn a_sloping_run_has_its_label_clear_of_the_line() {
    let route = [p(0.0, 0.0), p(200.0, 120.0)];
    for candidate in candidates(&route, 0.0, LABEL).iter() {
        let c = super::clipped_length(route[0], route[1], candidate.rect);
        assert_eq!(c, 0.0, "{candidate:?} touches the line");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn one_run_has_three_anchors_on_each_side() {
    let all = candidates(&[p(0.0, 0.0), p(200.0, 0.0)], 0.0, LABEL);
    assert_eq!(all.len(), 6);
}

#[test]
fn a_route_with_several_runs_has_at_most_eight_candidates() {
    let route = [p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(200.0, 100.0)];
    let all = candidates(&route, 10.0, LABEL);
    assert_eq!(all.len(), 8);
    assert!(all.len() <= MAX_CANDIDATES);
}

#[test]
fn a_route_with_fewer_than_two_points_has_none_and_no_placement() {
    assert!(candidates(&[], 0.0, LABEL).is_empty());
    assert!(candidates(&[p(1.0, 1.0)], 0.0, LABEL).is_empty());
    assert!(place(&input(&[p(1.0, 1.0)], &[], &[], None)).is_none());
}

#[test]
fn the_candidates_do_not_depend_on_anything_but_the_route_and_the_label() {
    let route = [p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0)];
    let (a, b) = (candidates(&route, 8.0, LABEL), candidates(&route, 8.0, LABEL));
    assert_eq!(&a[..], &b[..]);
}

#[test]
fn the_longest_usable_run_is_the_one_the_label_prefers() {
    // The first run is 40 long and the second 200 long, so the label goes on the second.
    let route = [p(0.0, 0.0), p(40.0, 0.0), p(40.0, 200.0)];
    let first = candidates(&route, 0.0, LABEL)[0];
    assert_eq!(first.anchor.x, 40.0);
    assert!(first.anchor.y > 40.0);
}

#[test]
fn of_runs_as_long_the_later_one_is_preferred() {
    // The first run is usable from 6 to 100 and the second from 0 to 94, so both have 94 units of it.
    let route = [p(0.0, 0.0), p(100.0, 0.0), p(100.0, 108.0)];
    let first = candidates(&route, 0.0, LABEL)[0];
    assert_eq!(
        first.anchor,
        p(100.0, 47.0),
        "on the second run, which is further from the source"
    );
}

#[test]
fn a_rounded_corner_is_trimmed_like_the_path_trims_it() {
    // The corner at (100, 0) is rounded by 30, so 30 of the first run's end and of the second's start are not usable.
    let route = [p(0.0, 0.0), p(100.0, 0.0), p(100.0, 400.0)];
    let all = candidates(&route, 30.0, LABEL);
    // The longest run is the second, usable from 30 to 400 - 14. Its quarter point is 30 + 0.25 * 356 = 119.
    assert!((all[1].anchor.y - 119.0).abs() < 1e-9, "{:?}", all[1].anchor);
}

#[test]
fn a_run_too_short_to_trim_is_usable_at_its_middle_only() {
    let route = [p(0.0, 0.0), p(10.0, 0.0)];
    let all = candidates(&route, 0.0, LABEL);
    assert!(all.iter().all(|c| c.anchor == p(5.0, 0.0)));
}

#[test]
fn the_last_run_leaves_the_arrowhead_bare() {
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    for candidate in candidates(&route, 0.0, LABEL).iter() {
        assert!(candidate.anchor.x <= 200.0 - ARROW_REACH + 1e-9, "{:?}", candidate.anchor);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn with_nothing_in_the_way_the_first_candidate_is_chosen() {
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    let placement = place(&input(&route, &[], &[], None)).expect("a placement");
    assert_eq!(placement.candidate, 0);
    assert_eq!(placement.score, 0);
    assert_eq!(placement.conflicts, Conflicts::CLEAR);
}

#[test]
fn a_box_in_the_way_moves_the_label_along_the_run() {
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    // A box just above the middle of the run, where the first candidate would be.
    let block = rect(80.0, 70.0, 40.0, 25.0);
    let placement = place(&input(&route, &[block], &[], None)).expect("a placement");
    assert_ne!(placement.candidate, 0);
    assert!(placement.conflicts.is_clear());
    assert!(!block.origin.x.is_nan());
}

#[test]
fn a_box_that_is_padded_still_counts_as_in_the_way() {
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    // The first candidate's box ends 3 above the line, so a box starting 2 above it overlaps once padded by 4.
    let first = candidates(&route, 0.0, LABEL)[0];
    let beside = rect(first.rect.origin.x - 8.0, first.rect.origin.y, 6.0, first.rect.size.height);
    let placement = place(&input(&route, &[beside], &[], None)).expect("a placement");
    assert_ne!(placement.candidate, 0, "a box two units from the label is too close");
}

#[test]
fn the_opposite_side_is_used_when_the_preferred_side_is_blocked_all_along() {
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    let wall_above = rect(-10.0, 40.0, 220.0, 55.0);
    let placement = place(&input(&route, &[wall_above], &[], None)).expect("a placement");
    assert!(placement.conflicts.is_clear());
    let chosen = candidates(&route, 0.0, LABEL)[placement.candidate];
    assert!(!chosen.preferred);
    assert!(chosen.rect.origin.y > 100.0);
    assert!(placement.score >= OPPOSITE_SIDE_PENALTY);
}

#[test]
fn another_label_in_the_way_is_avoided() {
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    let first = candidates(&route, 0.0, LABEL)[0];
    let placement = place(&input(&route, &[], &[first.rect], None)).expect("a placement");
    assert_ne!(placement.candidate, 0);
    assert!(placement.conflicts.is_clear());
}

#[test]
fn the_labels_own_other_run_is_avoided() {
    // The label of the first run, near the corner, would sit on the second run, which is a hard conflict.
    let route = [p(0.0, 0.0), p(60.0, 0.0), p(60.0, 300.0)];
    let placement = place(&input(&route, &[], &[], None)).expect("a placement");
    assert!(placement.conflicts.is_clear());
}

#[test]
fn when_every_candidate_is_blocked_the_least_bad_is_chosen_and_the_weight_is_kept() {
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    let everything = rect(-50.0, -50.0, 400.0, 400.0);
    let placement = place(&input(&route, &[everything], &[], None)).expect("a placement");
    assert!(!placement.conflicts.is_clear());
    assert_eq!(placement.conflicts.count, 1, "overlaps the one box and nothing else");
}

#[test]
fn fewer_overlaps_beat_less_overlap() {
    // Every candidate overlaps the big box. Those on the lower side also overlap the small one, so are worse.
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    let big = rect(-50.0, -50.0, 400.0, 400.0);
    let small = rect(0.0, 105.0, 200.0, 30.0);
    let placement = place(&input(&route, &[big, small], &[], None)).expect("a placement");
    let chosen = candidates(&route, 0.0, LABEL)[placement.candidate];
    assert!(chosen.preferred, "the side with fewer overlaps");
    assert_eq!(placement.conflicts.count, 1);
}

/// A set of scored candidates, all clear, with these soft scores.
fn clear(scores: &[u32]) -> Vec<(Conflicts, u32)> {
    scores.iter().map(|&score| (Conflicts::CLEAR, score)).collect()
}

#[test]
fn eleven_points_of_improvement_is_not_enough_to_leave_a_clear_candidate() {
    // The label was at ordinal 1, which scores 40. Ordinal 0 scores 29, eleven better.
    assert_eq!(choose(&clear(&[29, 40]), Some(1)), Some(1));
}

#[test]
fn twelve_points_of_improvement_is_enough() {
    assert_eq!(choose(&clear(&[28, 40]), Some(1)), Some(0));
}

#[test]
fn the_best_alternative_is_the_one_moved_to_and_ties_go_to_the_lower_ordinal() {
    assert_eq!(choose(&clear(&[30, 5, 5, 40]), Some(3)), Some(1));
}

#[test]
fn with_no_previous_candidate_the_threshold_does_not_apply() {
    assert_eq!(choose(&clear(&[1, 0]), None), Some(1));
}

#[test]
fn a_candidate_that_is_no_longer_clear_is_left_at_once_whatever_the_threshold() {
    let mut scored = clear(&[10, 11]);
    scored[1].0 = Conflicts { count: 1, area: 1.0 };
    // Ordinal 0 is only one point better than the blocked ordinal 1, far under the threshold, and is taken anyway.
    assert_eq!(choose(&scored, Some(1)), Some(0));
}

#[test]
fn a_previous_ordinal_that_is_not_clear_never_stays_even_when_it_scores_best() {
    let mut scored = clear(&[0, 50]);
    scored[0].0 = Conflicts { count: 2, area: 9.0 };
    assert_eq!(choose(&scored, Some(0)), Some(1));
}

#[test]
fn with_no_candidate_clear_the_fewest_overlaps_win_then_the_least_overlap_then_the_score() {
    let at = |count, area, score| (Conflicts { count, area }, score);
    assert_eq!(choose(&[at(2, 1.0, 0), at(1, 90.0, 9), at(1, 80.0, 99)], None), Some(2));
    assert_eq!(choose(&[at(1, 5.0, 9), at(1, 5.0, 3)], None), Some(1));
    assert_eq!(choose(&[at(1, 5.0, 3), at(1, 5.0, 3)], None), Some(0));
}

#[test]
fn a_blocked_candidate_is_left_for_another_in_a_real_route() {
    let route = [p(0.0, 100.0), p(69.0, 100.0)];
    let blocked = candidates(&route, 0.0, LABEL)[1].rect;
    let placement = place(&input(&route, &[blocked], &[], Some(1))).expect("a placement");
    assert_ne!(placement.candidate, 1);
}

#[test]
fn a_previous_ordinal_beyond_the_candidates_is_ignored() {
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    let placement = place(&input(&route, &[], &[], Some(7))).expect("a placement");
    assert_eq!(placement.candidate, 0);
}

#[test]
fn only_the_ordinal_is_kept_so_the_label_follows_a_moved_route() {
    let before = [p(0.0, 100.0), p(200.0, 100.0)];
    let after = [p(0.0, 300.0), p(200.0, 300.0)];
    let first = place(&input(&before, &[], &[], None)).expect("a placement");
    let again = place(&input(&after, &[], &[], Some(first.candidate))).expect("a placement");
    assert_eq!(again.candidate, first.candidate);
    assert_eq!(
        again.rect.origin.y - first.rect.origin.y,
        200.0,
        "the old coordinates were not kept"
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_same_input_gives_the_same_placement() {
    let route = [p(0.0, 0.0), p(120.0, 0.0), p(120.0, 80.0)];
    let vertices = [rect(100.0, -40.0, 40.0, 30.0)];
    let a = place(&input(&route, &vertices, &[], Some(2)));
    let b = place(&input(&route, &vertices, &[], Some(2)));
    assert_eq!(a, b);
}

#[test]
fn a_label_never_overlaps_the_arrowhead_region_when_it_has_a_choice() {
    let route = [p(0.0, 100.0), p(200.0, 100.0)];
    let placement = place(&input(&route, &[], &[], None)).expect("a placement");
    let reach = rect(200.0 - ARROW_REACH, 100.0 - ARROW_REACH, 2.0 * ARROW_REACH, 2.0 * ARROW_REACH);
    assert_eq!(super::overlap_area(placement.rect, reach), 0.0);
}

#[test]
fn clipping_measures_the_part_of_a_segment_inside_a_box() {
    let b = rect(10.0, 10.0, 20.0, 20.0);
    assert_eq!(super::clipped_length(p(0.0, 20.0), p(40.0, 20.0), b), 20.0);
    assert_eq!(super::clipped_length(p(0.0, 0.0), p(40.0, 0.0), b), 0.0);
    assert_eq!(super::clipped_length(p(15.0, 15.0), p(25.0, 15.0), b), 10.0);
    assert_eq!(super::clipped_length(p(20.0, 0.0), p(20.0, 5.0), b), 0.0);
}

#[test]
fn two_labels_one_either_end_of_two_close_routes_are_told_apart() {
    // Two routes leave the same box together, as C to D and C to E do. Their labels must not overlap once the first is
    // given to the second as an obstacle.
    let from_c_to_d = [p(0.0, 0.0), p(300.0, 40.0)];
    let from_c_to_e = [p(0.0, 0.0), p(300.0, 70.0)];
    let first = place(&input(&from_c_to_d, &[], &[], None)).expect("a placement");
    let second = place(&input(&from_c_to_e, &[], &[first.rect], None)).expect("a placement");
    assert!(second.conflicts.is_clear());
    assert_eq!(super::overlap_area(super::padded(first.rect, 2.0), second.rect), 0.0);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A small deterministic generator, so that these tests need no dependency and give the same numbers on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: f64) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64 / (1u64 << 31) as f64) * below
    }
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.origin.x < b.origin.x + b.size.width
        && a.origin.x + a.size.width > b.origin.x
        && a.origin.y < b.origin.y + b.size.height
        && a.origin.y + a.size.height > b.origin.y
}

/// Leaving out every node and label that is outside [`reach`] of the route gives the very same placement, for a few
/// hundred random scenes, and for a previous candidate as well as for none.
#[test]
fn leaving_out_what_is_beyond_reach_does_not_change_the_placement() {
    let mut random = Lcg(7);
    for round in 0..300 {
        let route = [
            p(random.next(600.0), random.next(600.0)),
            p(random.next(600.0), random.next(600.0)),
        ];
        let vertices: Vec<Rect> = (0..60)
            .map(|_| {
                rect(
                    random.next(700.0),
                    random.next(700.0),
                    20.0 + random.next(60.0),
                    20.0 + random.next(40.0),
                )
            })
            .collect();
        let others: Vec<Rect> = (0..60)
            .map(|_| {
                rect(
                    random.next(700.0),
                    random.next(700.0),
                    10.0 + random.next(40.0),
                    12.0 + random.next(8.0),
                )
            })
            .collect();
        let region = reach(&route, LABEL).expect("a route");
        let near = |all: &[Rect]| all.iter().copied().filter(|r| overlaps(region, *r)).collect::<Vec<_>>();
        let (near_vertices, near_others) = (near(&vertices), near(&others));
        for previous in [None, Some(0), Some(3)] {
            let everything = place(&input(&route, &vertices, &others, previous));
            let nearby = place(&input(&route, &near_vertices, &near_others, previous));
            assert_eq!(everything, nearby, "round {round}, previous {previous:?}");
        }
    }
}

/// How long placing the labels of a few hundred edges takes, with and without leaving out what is far away. Run with
/// `cargo test --release -- --ignored --nocapture label_placing_time`. It asserts nothing, since a time depends on the
/// machine, and is here to show whether the work grows with the graph.
#[test]
#[ignore = "a timing, not a check"]
fn label_placing_time() {
    let mut random = Lcg(11);
    for count in [50_usize, 200, 400] {
        let vertices: Vec<Rect> = (0..count)
            .map(|_| rect(random.next(4000.0), random.next(4000.0), 56.0, 36.0))
            .collect();
        let routes: Vec<[Point; 2]> = (0..count)
            .map(|_| {
                [
                    p(random.next(4000.0), random.next(4000.0)),
                    p(random.next(4000.0), random.next(4000.0)),
                ]
            })
            .collect();
        let labels: Vec<Rect> = (0..count)
            .map(|_| rect(random.next(4000.0), random.next(4000.0), 20.0, 12.0))
            .collect();
        let time = |filtered: bool| {
            let start = std::time::Instant::now();
            for (index, route) in routes.iter().enumerate() {
                let region = reach(route, LABEL).expect("a route");
                let keep = |all: &[Rect]| -> Vec<Rect> {
                    if filtered {
                        all.iter().copied().filter(|r| overlaps(region, *r)).collect()
                    } else {
                        all.to_vec()
                    }
                };
                let (near_vertices, near_labels) = (keep(&vertices), keep(&labels[..index]));
                std::hint::black_box(place(&input(route, &near_vertices, &near_labels, None)));
            }
            start.elapsed()
        };
        println!(
            "{count} labelled edges: all obstacles {:?}, nearby only {:?}",
            time(false),
            time(true)
        );
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Coordinates far larger than any scene has still give a score, and no sum of scores wraps or panics.
#[test]
fn scores_of_enormous_coordinates_saturate_and_do_not_overflow() {
    let route = [p(0.0, 0.0), p(1.0e12, 0.0)];
    let all = candidates(&route, 0.0, LABEL);
    let first = all[0];
    let far_opposite = all
        .iter()
        .find(|candidate| !candidate.preferred && candidate.anchor != first.anchor)
        .expect("an opposite-side candidate at another anchor");
    // The offset is past u32::MAX, so the points saturate, and the penalty must not wrap them round to a small number.
    assert_eq!(soft_score(far_opposite, &first), u32::MAX);
    assert!(place(&input(&route, &[], &[], None)).is_some());
}

/// The comparison that decides whether to leave a place does not overflow when the scores are near the largest.
#[test]
fn keeping_a_place_does_not_overflow_when_the_scores_are_near_the_largest() {
    let clear = Conflicts::CLEAR;
    // A difference of one is less than the threshold, so the previous place is kept, and nothing may wrap or saturate
    // into a switch. A difference of the threshold is enough to leave it.
    let near = [(clear, u32::MAX), (clear, u32::MAX - 1)];
    assert_eq!(choose(&near, Some(0)), Some(0));
    let enough = [(clear, u32::MAX), (clear, u32::MAX - SWITCH_THRESHOLD)];
    assert_eq!(choose(&enough, Some(0)), Some(1));
}
