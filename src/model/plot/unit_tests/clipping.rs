//! Cutting a segment, a polyline and a polygon to the plotted area.

use super::*;

const BOX: Bounds = Bounds {
    left: 0.0,
    top: 0.0,
    right: 10.0,
    bottom: 10.0,
};

#[test]
fn a_segment_inside_is_kept_and_one_outside_is_dropped() {
    assert_eq!(clip_segment((1.0, 1.0), (9.0, 9.0), BOX), Some(((1.0, 1.0), (9.0, 9.0))));
    assert_eq!(clip_segment((11.0, 0.0), (20.0, 5.0), BOX), None);
    assert_eq!(
        clip_segment((-5.0, 11.0), (15.0, 11.0), BOX),
        None,
        "parallel to an edge, and outside it"
    );
}

#[test]
fn a_segment_that_leaves_is_cut_at_the_edge() {
    assert_eq!(clip_segment((5.0, 5.0), (15.0, 5.0), BOX), Some(((5.0, 5.0), (10.0, 5.0))));
    let (a, b) = clip_segment((-10.0, 5.0), (20.0, 5.0), BOX).expect("crosses the box");
    assert_eq!((a, b), ((0.0, 5.0), (10.0, 5.0)));
    let (a, b) = clip_segment((-5.0, -5.0), (15.0, 15.0), BOX).expect("crosses the corners");
    assert!(near(a.0, 0.0) && near(a.1, 0.0) && near(b.0, 10.0) && near(b.1, 10.0));
}

#[test]
fn a_polyline_is_split_where_it_leaves_and_where_a_point_is_missing() {
    let points = [
        Some((1.0, 1.0)),
        Some((5.0, 5.0)),
        Some((15.0, 5.0)),
        Some((15.0, 8.0)),
        Some((5.0, 8.0)),
        Some((6.0, 9.0)),
        None,
        Some((2.0, 2.0)),
        Some((3.0, 3.0)),
    ];
    let runs = clip_polyline(&points, BOX);
    assert_eq!(
        runs,
        vec![
            vec![(1.0, 1.0), (5.0, 5.0), (10.0, 5.0)],
            vec![(10.0, 8.0), (5.0, 8.0), (6.0, 9.0)],
            vec![(2.0, 2.0), (3.0, 3.0)],
        ]
    );
}

#[test]
fn a_polyline_wholly_outside_or_of_one_point_has_no_run() {
    assert!(clip_polyline(&[Some((20.0, 20.0)), Some((30.0, 30.0))], BOX).is_empty());
    assert!(clip_polyline(&[Some((5.0, 5.0))], BOX).is_empty());
    assert!(clip_polyline(&[], BOX).is_empty());
}
