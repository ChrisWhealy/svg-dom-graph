use super::{
    GRAPH_HEIGHT, GRAPH_WIDTH, NAMES, NO_PREDECESSOR, QUEUE_CELLS, QUEUE_ENTRIES, ROW_GAP, TOOLBAR_ROOM, TOP, Texts,
    UNREACHED, VERTEX_RADIUS, distance_text, place, previous_text, queue_texts, vertex_centre, vertex_origins,
    widest_by,
};
use crate::dijkstra::algorithm::{Graph, QueueEntry, RawEdge, Snapshot, steps};
use demo_common::util::MARGIN;
use svg_dom::root::utils::{Point, Size};

/// The reference graph, in the order the demo draws its edges.
const REFERENCE: [RawEdge; 9] = [
    (0, 1, 4),
    (0, 2, 2),
    (2, 1, 1),
    (1, 3, 5),
    (2, 3, 8),
    (2, 4, 10),
    (3, 4, 2),
    (3, 5, 6),
    (4, 5, 3),
];

fn graph() -> Graph {
    Graph::new(&NAMES, &REFERENCE).expect("the reference graph")
}

/// A box as `(left, top, right, bottom)`.
type Box4 = (f64, f64, f64, f64);

fn vertex_boxes() -> Vec<Box4> {
    vertex_origins()
        .iter()
        .map(|origin| {
            (
                origin.x,
                origin.y,
                origin.x + 2.0 * VERTEX_RADIUS,
                origin.y + 2.0 * VERTEX_RADIUS,
            )
        })
        .collect()
}

fn overlaps(a: Box4, b: Box4) -> bool {
    a.0 < b.2 && a.2 > b.0 && a.1 < b.3 && a.3 > b.1
}

fn node_box(origin: Point, size: Size) -> Box4 {
    (origin.x, origin.y, origin.x + size.width, origin.y + size.height)
}

fn centre(b: Box4) -> (f64, f64) {
    ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_vertex_boxes_do_not_overlap_and_are_inside_the_graphs_part_of_the_scene() {
    let boxes = vertex_boxes();
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            assert!(!overlaps(*a, *b), "{a:?} overlaps {b:?}");
        }
        assert!(
            a.0 >= MARGIN && a.2 <= MARGIN + GRAPH_WIDTH,
            "{a:?} is outside the graph's width"
        );
        assert!(a.1 >= TOP && a.3 <= TOP + GRAPH_HEIGHT, "{a:?} is outside the graph's height");
    }
}

#[test]
fn no_connector_passes_through_a_box_it_does_not_join() {
    let boxes = vertex_boxes();
    for (index, &(from, to, _)) in REFERENCE.iter().enumerate() {
        let (a, b) = (centre(boxes[from]), centre(boxes[to]));
        for (vertex, &other) in boxes.iter().enumerate().filter(|&(vertex, _)| vertex != from && vertex != to) {
            let hits = (1..400).any(|step| {
                let t = f64::from(step) / 400.0;
                let (x, y) = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                x > other.0 && x < other.2 && y > other.1 && y < other.3
            });
            assert!(
                !hits,
                "edge {index} ({} to {}) passes through {}",
                NAMES[from], NAMES[to], NAMES[vertex]
            );
        }
    }
}

#[test]
fn the_two_connectors_that_leave_c_go_off_in_different_directions() {
    let boxes = vertex_boxes();
    let c = centre(boxes[2]);
    let angle = |to: usize| {
        let d = centre(boxes[to]);
        (d.1 - c.1).atan2(d.0 - c.0).to_degrees()
    };
    // C to D and C to E are the two whose labels the demo has to keep apart.
    assert!((angle(3) - angle(4)).abs() > 15.0, "{} and {}", angle(3), angle(4));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn array() -> Size {
    Size::new(120.0, 250.0)
}

#[test]
fn the_arrays_sit_side_by_side_to_the_right_of_the_graph_with_the_queue_beside_them() {
    let placement = place(
        [array(), Size::new(100.0, 250.0), Size::new(90.0, 250.0)],
        Size::new(150.0, 280.0),
        150.0,
    );
    let boxes = [
        node_box(placement.distance, array()),
        node_box(placement.previous, Size::new(100.0, 250.0)),
        node_box(placement.settled, Size::new(90.0, 250.0)),
        node_box(placement.queue, Size::new(150.0, 280.0)),
    ];
    for (i, a) in boxes.iter().enumerate() {
        for b in &boxes[i + 1..] {
            assert!(!overlaps(*a, *b), "{a:?} overlaps {b:?}");
        }
        assert!(a.0 >= MARGIN + GRAPH_WIDTH, "{a:?} is not right of the graph");
    }
    assert!(
        placement.queue.x >= placement.settled.x + 90.0,
        "the queue is right of the last array"
    );
    assert_eq!(
        placement.queue.y, placement.distance.y,
        "the queue is level with the arrays' tops"
    );
}

#[test]
fn putting_the_queue_beside_the_arrays_makes_the_scene_wider_and_shorter_than_putting_it_below() {
    let (arrays, queue) = ([array(); 3], Size::new(150.0, 280.0));
    let placement = place(arrays, queue, 150.0);
    let below_height = TOP + array().height + ROW_GAP + queue.height + MARGIN;
    assert!(
        placement.canvas.height < below_height,
        "{} is not under {below_height}",
        placement.canvas.height
    );
    let below_width = placement.settled.x + array().width + MARGIN + TOOLBAR_ROOM;
    assert!(placement.canvas.width > below_width);
}

#[test]
fn the_note_is_below_the_graph_and_as_wide_as_it() {
    let placement = place([array(); 3], Size::new(150.0, 280.0), 150.0);
    assert!(placement.note.y >= TOP + GRAPH_HEIGHT);
    assert_eq!(placement.note.x, MARGIN);
    assert_eq!(placement.note_width, GRAPH_WIDTH);
}

#[test]
fn the_scene_holds_everything_and_has_room_for_the_zoom_toolbar() {
    let queue = Size::new(150.0, 280.0);
    let placement = place([array(); 3], queue, 150.0);
    let right = placement.queue.x + queue.width;
    assert_eq!(placement.canvas.width, right + MARGIN + TOOLBAR_ROOM);
    assert!(placement.canvas.height >= placement.queue.y + queue.height + MARGIN);
    assert!(placement.canvas.height >= placement.note.y + 150.0 + MARGIN);
}

#[test]
fn a_taller_note_makes_the_scene_taller_and_moves_nothing() {
    let (arrays, queue) = ([array(); 3], Size::new(150.0, 280.0));
    let short = place(arrays, queue, 100.0);
    let tall = place(arrays, queue, 900.0);
    assert!(tall.canvas.height > short.canvas.height);
    assert_eq!(
        (tall.distance, tall.previous, tall.settled, tall.queue, tall.note),
        (short.distance, short.previous, short.settled, short.queue, short.note)
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_distance_is_a_number_or_the_unreached_mark() {
    assert_eq!(distance_text(Some(0)), "0");
    assert_eq!(distance_text(Some(1250)), "1250");
    assert_eq!(distance_text(None), UNREACHED);
    assert_ne!(UNREACHED, u64::MAX.to_string(), "no number stands in for it");
}

#[test]
fn a_predecessor_is_the_name_of_the_vertex_or_a_dash() {
    let graph = graph();
    assert_eq!(previous_text(&graph, Some(2)), "C");
    assert_eq!(previous_text(&graph, None), NO_PREDECESSOR);
}

fn snapshot_with_queue(entries: &[(u64, usize, bool)]) -> Snapshot {
    Snapshot {
        queue: entries
            .iter()
            .map(|&(distance, vertex, stale)| QueueEntry { distance, vertex, stale })
            .collect(),
        ..Snapshot::default()
    }
}

#[test]
fn the_queue_always_has_the_same_number_of_cells_with_empty_ones_after_the_entries() {
    let graph = graph();
    for count in 0..=QUEUE_ENTRIES {
        let entries: Vec<(u64, usize, bool)> = (0..count).map(|i| (i as u64, i % 6, false)).collect();
        let cells = queue_texts(&graph, &snapshot_with_queue(&entries));
        assert_eq!(cells.len(), QUEUE_CELLS);
        assert!(cells[..count].iter().all(|cell| !cell.is_empty()));
        assert!(cells[count..].iter().all(String::is_empty), "{cells:?}");
    }
}

#[test]
fn an_entry_is_its_distance_and_vertex_and_says_so_if_it_is_stale() {
    let cells = queue_texts(&graph(), &snapshot_with_queue(&[(3, 1, false), (4, 1, true)]));
    assert_eq!(cells[0], "(3, B)");
    assert_eq!(cells[1], "(4, B) stale");
}

#[test]
fn more_entries_than_there_are_rows_are_counted_in_the_last_cell() {
    let entries: Vec<(u64, usize, bool)> = (0..11).map(|i| (i as u64, 0, false)).collect();
    let cells = queue_texts(&graph(), &snapshot_with_queue(&entries));
    assert_eq!(cells[QUEUE_ENTRIES], "+3 more");
    assert_eq!(cells[QUEUE_ENTRIES - 1], "(7, A)");
    assert_eq!(queue_texts(&graph(), &snapshot_with_queue(&entries[..8]))[QUEUE_ENTRIES], "");
}

#[test]
fn every_step_of_the_reference_run_has_a_cell_for_each_vertex_and_a_full_queue() {
    let graph = graph();
    for snapshot in steps(&graph, 0, Some(5)).expect("a run") {
        let texts = Texts::of(&graph, &snapshot);
        assert_eq!(texts.distance.len(), NAMES.len());
        assert_eq!(texts.previous.len(), NAMES.len());
        assert_eq!(texts.queue.len(), QUEUE_CELLS);
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A stand-in for measuring in the browser: a character is 10 wide, except that the infinity mark is 25.
fn measure(text: &str) -> Result<f64, ()> {
    Ok(text.chars().map(|c| if c == '\u{221E}' { 25.0 } else { 10.0 }).sum())
}

#[test]
fn the_widest_string_is_the_one_measured_widest_and_not_the_one_with_the_most_characters() {
    let strings = ["12", "\u{221E}", "9", "1250"].map(String::from);
    // `1250` has four characters, but the infinity mark is wider than two of them and narrower than four.
    assert_eq!(widest_by(strings, measure), Ok(Some("1250".to_owned())));
    let strings = ["12", "\u{221E}", "9"].map(String::from);
    assert_eq!(
        widest_by(strings, measure),
        Ok(Some("\u{221E}".to_owned())),
        "wider than two digits"
    );
}

#[test]
fn of_equally_wide_strings_the_first_is_taken_and_no_strings_gives_none() {
    assert_eq!(widest_by(["ab", "cd"].map(String::from), measure), Ok(Some("ab".to_owned())));
    assert_eq!(widest_by(Vec::<String>::new(), measure), Ok(None));
}

#[test]
fn an_error_while_measuring_is_returned() {
    let outcome = widest_by(["a", "b"].map(String::from), |_| Err::<f64, &str>("no font"));
    assert_eq!(outcome, Err("no font"));
}

#[test]
fn the_widest_of_every_string_a_column_shows_is_as_wide_as_each_of_them() {
    let graph = graph();
    let all: Vec<Texts> = steps(&graph, 0, Some(5))
        .expect("a run")
        .iter()
        .map(|s| Texts::of(&graph, s))
        .collect();
    for column in [
        all.iter().flat_map(|t| t.distance.clone()).collect::<Vec<_>>(),
        all.iter().flat_map(|t| t.previous.clone()).collect::<Vec<_>>(),
        all.iter().flat_map(|t| t.queue.clone()).collect::<Vec<_>>(),
    ] {
        let widest = widest_by(column.clone(), measure).expect("measured").expect("some strings");
        let widest_width = measure(&widest).expect("measured");
        assert!(column.iter().all(|text| measure(text).expect("measured") <= widest_width));
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_centre_of_a_vertex_is_half_a_square_in_from_its_corner() {
    let centre = vertex_centre(Point::new(100.0, 50.0));
    assert_eq!((centre.x, centre.y), (100.0 + VERTEX_RADIUS, 50.0 + VERTEX_RADIUS));
}

#[test]
fn the_vertices_are_where_they_were_when_they_were_boxes() {
    // The centres of the 56 by 36 boxes that the vertices were before they were circles.
    let box_centres = [
        (0.0, 124.0),
        (166.0, 14.0),
        (150.0, 234.0),
        (336.0, 124.0),
        (346.0, 264.0),
        (500.0, 180.0),
    ];
    for (origin, (x, y)) in vertex_origins().iter().zip(box_centres) {
        let centre = vertex_centre(*origin);
        assert_eq!((centre.x, centre.y), (MARGIN + x + 28.0, TOP + y + 18.0));
    }
}
