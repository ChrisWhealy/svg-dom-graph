use super::*;
use crate::tarjan::fixture::{self, edges, members_of};

/// The sizes of box that the tests try, which stand for a measured one.
const BOXES: [(f64, f64); 3] = [(101.3, 91.8), (96.0, 88.0), (106.0, 96.0)];

fn layouts() -> impl Iterator<Item = Layout> {
    BOXES.into_iter().map(|(w, h)| Layout::build(w, h))
}

#[test]
fn the_slots_are_distinct_places_of_the_grid() {
    let mut seen = SLOT.to_vec();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), VERTICES);
    assert!(SLOT.iter().all(|&s| s < COLUMNS * ROWS));
}

#[test]
fn no_two_vertex_boxes_overlap_or_touch() {
    for layout in layouts() {
        for a in 0..VERTICES {
            for b in a + 1..VERTICES {
                let (p, q) = (layout.rect(a).grown(CLEARANCE), layout.rect(b));
                let apart = p.right() <= q.x || q.right() <= p.x || p.bottom() <= q.y || q.bottom() <= p.y;
                assert!(apart, "{a} and {b} with a box of {:?}", layout.box_size);
            }
        }
    }
}

#[test]
fn everything_is_inside_the_picture_with_room_to_spare() {
    for layout in layouts() {
        for v in 0..VERTICES {
            let r = layout.rect(v);
            assert!(
                r.x >= LEFT_PANEL
                    && r.y >= 30.0
                    && r.right() <= layout.size.0 - 30.0
                    && r.bottom() <= layout.size.1 - 30.0
            );
        }
    }
}

/// How many other vertices lie inside the bounds of the vertices of a region of four or more, and how many it should
/// have for the picture to give nothing away: one and a half for each vertex of the region. The sum of what is short.
fn mixing_deficit(layout: &Layout) -> usize {
    (0..fixture::REGIONS)
        .map(|g| {
            let members: Vec<usize> = members_of(g).collect();
            if members.len() < 4 {
                return 0;
            }
            (members.len() * 3 / 2).saturating_sub(strangers_among(layout, &members))
        })
        .sum()
}

fn strangers_among(layout: &Layout, members: &[usize]) -> usize {
    let bounds = layout.bounds_of(members, 0.0).unwrap();
    (0..VERTICES)
        .filter(|v| !members.contains(v))
        .filter(|&v| {
            let c = layout.centre(v);
            c.0 >= bounds.x && c.0 <= bounds.right() && c.1 >= bounds.y && c.1 <= bounds.bottom()
        })
        .count()
}

#[test]
fn the_components_are_not_next_to_each_other_so_the_picture_gives_nothing_away() {
    let layout = Layout::build(101.3, 91.8);
    for g in 0..fixture::REGIONS {
        let members: Vec<usize> = members_of(g).collect();
        if members.len() >= 4 {
            let strangers = strangers_among(&layout, &members);
            assert!(
                strangers >= members.len(),
                "the {} vertices of region {g} have only {strangers} others among them",
                members.len()
            );
        }
    }
}

#[test]
fn the_pinned_lanes_of_an_antiparallel_pair_run_side_by_side_and_are_at_least_16_units_apart() {
    let edges = edges().unwrap();
    for layout in layouts() {
        for &(a, b) in &[(56usize, 58usize), (60, 62), (61, 62)] {
            let forward = ends(&layout, a, b, edges.contains(&(b, a)));
            let backward = ends(&layout, b, a, edges.contains(&(a, b)));
            let (End::Pinned(s1, f1), End::Pinned(t1, _)) = (forward.from, forward.to) else {
                panic!("pinned");
            };
            let (End::Pinned(s2, f2), End::Pinned(t2, _)) = (backward.from, backward.to) else {
                panic!("pinned");
            };
            assert_eq!((s1, t1), (t2, s2), "the two share the same two sides, the other way round");
            assert_eq!((f1, f2), LANES);
            let along = if matches!(s1, Side::East | Side::West) {
                layout.box_size.1
            } else {
                layout.box_size.0
            };
            assert!((LANES.1 - LANES.0) * along >= 16.0, "{along}");
        }
    }
}

#[test]
fn an_edge_with_no_reverse_is_left_to_the_library() {
    let layout = Layout::build(101.3, 91.8);
    assert_eq!(ends(&layout, 0, 1, false), Ends { from: End::Auto, to: End::Auto });
}

#[test]
fn rectangles_find_a_segment_that_crosses_them_and_not_one_that_misses() {
    let r = Rect {
        x: 0.0,
        y: 0.0,
        width: 10.0,
        height: 10.0,
    };
    assert!(r.meets((-5.0, 5.0), (15.0, 5.0)));
    assert!(r.meets((5.0, -5.0), (5.0, 15.0)));
    assert!(r.meets((2.0, 2.0), (3.0, 3.0)), "a segment wholly inside");
    assert!(!r.meets((-5.0, -5.0), (-1.0, 20.0)));
    assert!(!r.meets((11.0, -5.0), (11.0, 15.0)));
    assert!(r.meets((-5.0, -5.0), (15.0, 15.0)));
}

#[test]
fn the_layout_is_the_same_each_time_and_depends_only_on_the_box() {
    assert_eq!(Layout::build(101.3, 91.8), Layout::build(101.3, 91.8));
    assert_ne!(Layout::build(101.3, 91.8), Layout::build(71.0, 58.0));
}

#[test]
fn the_bounds_of_a_set_of_vertices_cover_all_their_boxes() {
    let layout = Layout::build(101.3, 91.8);
    let outline = layout.bounds_of(&[0, 5, 9], 10.0).unwrap();
    for v in [0, 5, 9] {
        let r = layout.rect(v);
        assert!(outline.x <= r.x && outline.y <= r.y && outline.right() >= r.right() && outline.bottom() >= r.bottom());
    }
    assert_eq!(layout.bounds_of(&[], 10.0), None);
}

#[test]
fn at_most_the_measured_number_of_the_114_edges_pass_through_a_vertex_for_every_box_tried() {
    let edges = edges().unwrap();
    for layout in layouts() {
        let passing = layout.occlusions(&edges).len();
        assert!(
            passing <= MOST_PASSING,
            "{passing} edges pass through a vertex with a box of {:?}",
            layout.box_size
        );
    }
}

/// How many connectors may pass through a vertex that is not theirs. The search found a layout with fewer, and this has
/// some room for the other sizes of box.
const MOST_PASSING: usize = 40;

fn crosses(a: ((f64, f64), (f64, f64)), b: ((f64, f64), (f64, f64))) -> bool {
    let side = |p: (f64, f64), q: (f64, f64), r: (f64, f64)| (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
    let (d1, d2) = (side(b.0, b.1, a.0), side(b.0, b.1, a.1));
    let (d3, d4) = (side(a.0, a.1, b.0), side(a.0, a.1, b.1));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

/// The search that chose `SLOT`, kept so that it can be run again with `--ignored --nocapture`. It moves a vertex to
/// another place, or swaps two, and keeps the change if it passes fewer connectors through a vertex, and then if it
/// crosses fewer, with a falling chance of keeping a worse one.
fn search(seed: u64) -> ([usize; VERTICES], usize, usize, usize) {
    let edges = edges().unwrap();
    let (w, h) = BOXES[0];
    let cost = |slots: &[usize; VERTICES]| -> (usize, usize, usize) {
        let layout = Layout::build_with(w, h, slots);
        let passing = layout.occlusions(&edges).len();
        let segments: Vec<_> = edges.iter().map(|&(a, b)| (layout.centre(a), layout.centre(b))).collect();
        let mut crossing = 0;
        for i in 0..edges.len() {
            for j in i + 1..edges.len() {
                let shared = edges[i].0 == edges[j].0
                    || edges[i].0 == edges[j].1
                    || edges[i].1 == edges[j].0
                    || edges[i].1 == edges[j].1;
                if !shared && crosses(segments[i], segments[j]) {
                    crossing += 1;
                }
            }
        }
        (passing, crossing, mixing_deficit(&layout))
    };
    let mut state = seed;
    let mut next = move |n: usize| -> usize {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        (state % n as u64) as usize
    };
    let mut slots: [usize; VERTICES] = std::array::from_fn(|v| v);
    for i in (1..VERTICES).rev() {
        slots.swap(i, next(i + 1));
    }
    let weigh = |c: (usize, usize, usize)| (c.0 * 40 + c.1 + c.2 * 30) as f64;
    let mut current = cost(&slots);
    let mut best = (slots, current);
    let steps = 250_000;
    for step in 0..steps {
        let temperature = 30.0 * (1.0 - step as f64 / steps as f64) + 0.1;
        let mut trial = slots;
        let v = next(VERTICES);
        let place = next(COLUMNS * ROWS);
        match trial.iter().position(|&s| s == place) {
            Some(other) => trial.swap(v, other),
            None => trial[v] = place,
        }
        let c = cost(&trial);
        let delta = weigh(c) - weigh(current);
        let chance = (next(10_000) as f64) / 10_000.0;
        if delta <= 0.0 || chance < (-delta / temperature).exp() {
            slots = trial;
            current = c;
            if weigh(current) < weigh(best.1) {
                best = (slots, current);
            }
        }
    }
    (best.0, (best.1).0, (best.1).1, (best.1).2)
}

#[test]
#[ignore = "this is the search that chose the places, and takes a while"]
fn print_the_places_that_the_search_finds() {
    let mut best: Option<([usize; VERTICES], usize, usize, usize)> = None;
    for seed in [
        0x9e37_79b9_7f4a_7c15u64,
        0x1234_5678_9abc_def1,
        0xdead_beef_cafe_f00d,
        0x0123_4567_89ab_cdef,
    ] {
        let found = search(seed);
        eprintln!(
            "seed {seed:x}: {} passing, {} crossing, {} short of mixed",
            found.1, found.2, found.3
        );
        if best.is_none_or(|b| found.1 * 40 + found.2 + found.3 * 30 < b.1 * 40 + b.2 + b.3 * 30) {
            best = Some(found);
        }
    }
    eprintln!("{:?}", best.unwrap());
}

#[test]
fn the_left_panel_holds_the_stack_the_roster_and_the_condensation_clear_of_the_graph_and_of_each_other() {
    use crate::tarjan::condensation::{BOX, SLOTS};
    let layout = Layout::build(101.3, 91.8);
    let roster_right = ROSTER_AT.0 + ROSTER_ROW.0;
    let roster_bottom = Layout::roster_row(9).bottom();
    assert!(roster_right + 40.0 <= LEFT_PANEL, "the roster ends at {roster_right}");
    let graph_left = (0..VERTICES).map(|v| layout.rect(v).x).fold(f64::MAX, f64::min);
    assert!(graph_left >= LEFT_PANEL, "the graph starts at {graph_left}");
    assert!(ROSTER_AT.0 >= STACK_AT.0 + 185.0, "room for a stack node that is 175 wide");
    assert!(roster_bottom + 40.0 < layout.dag_origin.1, "the roster ends at {roster_bottom}");
    for column in SLOTS {
        for &slot in column {
            let (x, y) = layout.dag_centre(slot);
            assert!(x - BOX.0 / 2.0 >= ROSTER_AT.0 - 1.0);
            assert!(x + BOX.0 / 2.0 <= LEFT_PANEL && y + BOX.1 / 2.0 <= layout.size.1);
        }
    }
}

#[test]
fn the_roster_has_a_row_for_each_of_the_ten_components_that_do_not_touch() {
    for i in 0..9 {
        assert!(Layout::roster_row(i).bottom() < Layout::roster_row(i + 1).y);
    }
}
