use super::{GAP_X, GAP_Y, Item, NOTE_GAP, best_columns, link, plan};
use svg_dom::root::utils::Size;
use svg_dom_graph::scene::Side;

fn items(count: usize) -> Vec<Item> {
    vec![
        Item {
            node: Size::new(200.0, 40.0),
            note_height: 150.0,
        };
        count
    ]
}

#[test]
fn a_path_goes_along_a_row_then_back_along_the_next() {
    let laid = plan(&items(7), 3, 300.0, 0.0, 0.0);
    let cells: Vec<(usize, usize)> = laid.placements.iter().map(|p| (p.row, p.col)).collect();
    assert_eq!(cells, vec![(0, 0), (0, 1), (0, 2), (1, 2), (1, 1), (1, 0), (2, 0)]);
}

#[test]
fn every_cell_is_one_width_and_one_gap_from_the_next() {
    let laid = plan(&items(3), 3, 300.0, 100.0, 0.0);
    assert_eq!(laid.placements[1].note.x - laid.placements[0].note.x, 300.0 + GAP_X);
    assert_eq!(laid.placements[0].note.x, 100.0);
    assert_eq!(laid.size.width, 3.0 * 300.0 + 2.0 * GAP_X);
}

#[test]
fn the_boxes_of_a_row_are_level_and_their_notes_hang_under_them() {
    let mixed = vec![
        Item {
            node: Size::new(100.0, 30.0),
            note_height: 50.0,
        },
        Item {
            node: Size::new(100.0, 70.0),
            note_height: 90.0,
        },
    ];
    let laid = plan(&mixed, 2, 300.0, 0.0, 10.0);
    let middle = |i: usize| laid.placements[i].node.y + mixed[i].node.height / 2.0;
    assert_eq!(middle(0), middle(1));
    assert_eq!(laid.placements[0].note.y, 10.0 + 70.0 + NOTE_GAP);
    assert_eq!(laid.size.height, 70.0 + NOTE_GAP + 90.0);
}

#[test]
fn rows_are_a_gap_apart() {
    let laid = plan(&items(4), 2, 300.0, 0.0, 0.0);
    let row_height = 40.0 + NOTE_GAP + 150.0;
    assert_eq!(laid.placements[2].note.y, row_height + GAP_Y + 40.0 + NOTE_GAP);
}

#[test]
fn the_boxes_at_the_ends_of_a_row_are_pushed_to_the_outside_of_their_cell() {
    let laid = plan(&items(3), 3, 300.0, 0.0, 0.0);
    // The first box starts at the left of its cell, the last ends at the right of its cell, the middle is centred.
    assert_eq!(laid.placements[0].node.x, 0.0);
    assert_eq!(laid.placements[2].node.x + 200.0, 2.0 * (300.0 + GAP_X) + 300.0);
    assert_eq!(laid.placements[1].node.x, 300.0 + GAP_X + 50.0);
}

#[test]
fn a_connector_along_a_row_faces_the_way_the_row_runs() {
    let laid = plan(&items(6), 3, 300.0, 0.0, 0.0);
    let p = &laid.placements;
    assert_eq!((link(&p[0], &p[1]).from, link(&p[0], &p[1]).to), (Side::East, Side::West));
    assert_eq!((link(&p[3], &p[4]).from, link(&p[3], &p[4]).to), (Side::West, Side::East));
    assert!(!link(&p[0], &p[1]).turns);
}

#[test]
fn a_connector_between_rows_loops_round_the_side_the_row_ended_on() {
    let laid = plan(&items(7), 3, 300.0, 0.0, 0.0);
    let p = &laid.placements;
    let right = link(&p[2], &p[3]);
    assert_eq!((right.from, right.to, right.turns), (Side::East, Side::East, true));
    let left = link(&p[5], &p[6]);
    assert_eq!((left.from, left.to, left.turns), (Side::West, Side::West, true));
}

#[test]
fn nine_equal_steps_make_a_block_of_three_by_three() {
    assert_eq!(best_columns(&items(9), 300.0), 3);
}

#[test]
fn short_cells_need_fewer_columns_than_tall_ones_to_make_a_block_of_the_same_shape() {
    let short = vec![
        Item {
            node: Size::new(200.0, 40.0),
            note_height: 20.0,
        };
        12
    ];
    let tall = items(12);
    assert!(best_columns(&short, 300.0) <= best_columns(&tall, 300.0));
}

#[test]
fn a_path_of_one_step_has_one_column_and_a_longer_path_never_fewer_than_two() {
    assert_eq!(best_columns(&items(1), 300.0), 1);
    assert_eq!(best_columns(&[], 300.0), 1);
    for steps in 2..30 {
        assert!(best_columns(&items(steps), 300.0) >= 2);
        assert!(best_columns(&items(steps), 300.0) <= steps);
    }
}

#[test]
fn the_chosen_block_takes_little_more_room_than_the_smallest_there_is() {
    for steps in 2..20 {
        let area = |columns| {
            let size = plan(&items(steps), columns, 300.0, 0.0, 0.0).size;
            size.width * size.height
        };
        let smallest = (2..=steps).map(area).fold(f64::INFINITY, f64::min);
        assert!(area(best_columns(&items(steps), 300.0)) <= smallest * 1.6, "{steps} steps");
    }
}
