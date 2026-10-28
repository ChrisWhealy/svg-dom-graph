use super::{Grid, TOOLBAR_ROOM, TOP, column_gap, dependencies, heading, lane_name, widest_by};
use demo_common::util::MARGIN;
use svg_dom::root::utils::Size;

fn grid(lanes: usize, boundaries: usize) -> Grid {
    Grid::new(lanes, boundaries, Size::new(120.0, 50.0), 30.0)
}

#[test]
fn columns_are_one_node_and_one_gap_apart() {
    let g = grid(8, 4);
    assert_eq!(g.column_x(0), MARGIN);
    assert_eq!(g.column_x(1) - g.column_x(0), 120.0 + column_gap(8));
    assert_eq!(g.column_x(3) - g.column_x(2), 120.0 + column_gap(8));
}

#[test]
fn rows_are_one_node_and_one_gap_apart_and_the_same_in_every_column() {
    let g = grid(8, 4);
    for boundary in 0..4 {
        assert_eq!(g.node_origin(boundary, 3).y, g.node_origin(0, 3).y);
    }
    let step = g.node_origin(0, 1).y - g.node_origin(0, 0).y;
    assert!(step > 50.0);
    assert_eq!(g.node_origin(0, 7).y - g.node_origin(0, 6).y, step);
}

#[test]
fn the_nodes_start_below_the_headings_and_the_headings_below_the_toolbar() {
    let g = grid(8, 4);
    assert_eq!(g.heading_origin(2).y, TOP);
    assert!(g.node_origin(2, 0).y >= TOP + 30.0);
    assert_eq!(g.heading_origin(2).x, g.node_origin(2, 0).x);
}

#[test]
fn the_canvas_holds_every_node_and_the_toolbar() {
    for (lanes, boundaries) in [(8, 4), (16, 5)] {
        let g = grid(lanes, boundaries);
        let canvas = g.canvas();
        let last = g.node_origin(boundaries - 1, lanes - 1);
        assert!(canvas.width >= last.x + 120.0 + MARGIN + TOOLBAR_ROOM);
        assert!(canvas.height >= last.y + 50.0 + MARGIN);
    }
}

#[test]
fn the_sixteen_point_scene_has_wider_gaps() {
    assert!(column_gap(16) > column_gap(8));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_butterfly_has_the_four_connectors_a_to_sum_a_to_difference_b_to_sum_b_to_difference() {
    let d = dependencies(1, 5);
    let ends: Vec<(usize, usize)> = d.iter().map(|d| (d.source, d.target)).collect();
    assert_eq!(ends, [(1, 1), (1, 5), (5, 1), (5, 5)]);
}

#[test]
fn the_four_connectors_use_distinct_points_on_each_side() {
    let d = dependencies(1, 5);
    // Leaving: each lane uses two different points, one for each target.
    assert_ne!(d[0].from_position, d[1].from_position);
    assert_ne!(d[2].from_position, d[3].from_position);
    // Arriving: each lane gets one from `a` and one from `b`, at different points.
    assert_ne!(d[0].to_position, d[2].to_position);
    assert_ne!(d[1].to_position, d[3].to_position);
    for dep in d {
        assert!((0.0..=1.0).contains(&dep.from_position) && (0.0..=1.0).contains(&dep.to_position));
    }
}

#[test]
fn the_connectors_to_the_upper_target_leave_and_arrive_above_those_to_the_lower() {
    let d = dependencies(0, 4);
    // Both connectors that leave the upper lane: the one to the upper target leaves higher.
    assert!(d[0].from_position < d[1].from_position);
    // Both that leave the lower lane.
    assert!(d[2].from_position < d[3].from_position);
    // Into the upper target, `a` arrives higher than `b`; the same into the lower target.
    assert!(d[0].to_position < d[2].to_position);
    assert!(d[1].to_position < d[3].to_position);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_input_column_names_the_sample_each_lane_starts_with() {
    assert_eq!(lane_name(0, 1, 4), "S0[1]=x[4]");
    assert_eq!(lane_name(2, 7, 7), "S2[7]");
}

#[test]
fn headings_name_the_block_width() {
    assert_eq!(heading(0), "S0 \u{00B7} bit-reversed input");
    assert_eq!(heading(1), "S1 \u{00B7} block width 2");
    assert_eq!(heading(3), "S3 \u{00B7} block width 8");
}

#[test]
fn the_widest_item_wins_and_the_first_of_equals() {
    let widest = widest_by(["a", "ccc", "bb", "ddd"], |s| Ok(s.len() as f64));
    assert_eq!(widest, Ok(Some("ccc")));
    assert_eq!(widest_by(Vec::<&str>::new(), |s| Ok(s.len() as f64)), Ok(None));
}

#[test]
fn a_measurement_that_fails_stops_the_search() {
    let result = widest_by([1, 2, 3], |&n| if n == 2 { Err("no".to_owned()) } else { Ok(f64::from(n)) });
    assert_eq!(result, Err("no".to_owned()));
}
