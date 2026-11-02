//! Where the cells and the buttons are, which cell a point is in, the paths that are drawn, the cursor, and the
//! writing.

use super::*;
use crate::game_of_life::board::{PATTERNS, SPACESHIPS};

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

#[test]
fn the_board_is_40_by_30_cells_of_14_pixels_in_a_scene_that_is_560_wide() {
    assert_eq!((COLS, ROWS), (40, 30));
    assert_eq!((BOARD.width, BOARD.height), (560.0, 420.0));
    assert_eq!((SCENE.width, SCENE.height), (560.0, TOP + 420.0 + BOTTOM));
}

#[test]
fn a_point_is_in_the_cell_that_it_is_over_and_the_edges_belong_to_the_cell_after() {
    assert_eq!(cell_at(0.0, TOP), Some((0, 0)));
    assert_eq!(cell_at(13.99, TOP + 13.99), Some((0, 0)));
    assert_eq!(cell_at(14.0, TOP + 14.0), Some((1, 1)));
    assert_eq!(cell_at(559.99, TOP + 419.99), Some((COLS - 1, ROWS - 1)));
}

#[test]
fn a_point_that_is_not_over_the_board_is_in_no_cell() {
    assert_eq!(cell_at(-0.01, TOP + 10.0), None);
    assert_eq!(cell_at(560.0, TOP + 10.0), None);
    assert_eq!(cell_at(10.0, TOP - 0.01), None);
    assert_eq!(cell_at(10.0, TOP + 420.0), None);
    assert_eq!(cell_at(10.0, 10.0), None, "in the strip of buttons");
    assert_eq!(cell_at(10.0, TOP + 420.0 + 10.0), None, "in the strip of patterns");
    assert_eq!(cell_at(f64::NAN, TOP + 10.0), None);
    assert_eq!(cell_at(10.0, f64::INFINITY), None);
}

#[test]
fn the_origin_of_a_cell_is_in_the_cell_it_is_the_origin_of() {
    for (col, row) in [(0, 0), (1, 0), (0, 1), (39, 29), (17, 11)] {
        let at = cell_origin(col, row);
        assert_eq!(cell_at(at.x, at.y), Some((col, row)));
        assert_eq!(cell_at(at.x + CELL - 0.01, at.y + CELL - 0.01), Some((col, row)));
    }
}

#[test]
fn an_empty_board_has_no_path_and_a_cell_is_a_square_of_the_size_of_a_cell() {
    let mut board = Board::new(COLS, ROWS, true);
    assert_eq!(live_path(&board), "");
    board.set(2, 3, true);
    assert_eq!(live_path(&board), "M28 134h14v14h-14z");
}

#[test]
fn cells_that_are_next_to_each_other_in_a_row_are_one_rectangle_and_a_gap_or_a_new_row_starts_another() {
    let mut board = Board::new(COLS, ROWS, true);
    for col in [1, 2, 3, 6] {
        board.set(col, 0, true);
    }
    board.set(1, 1, true);
    let path = live_path(&board);
    assert_eq!(path.matches('M').count(), 3, "{path}");
    assert!(path.contains("h42v14h-42z"), "the run of three is 42 wide: {path}");
    assert!(path.contains("h14v14h-14z"));
}

#[test]
fn a_run_that_reaches_the_right_edge_is_ended_there() {
    let mut board = Board::new(COLS, ROWS, true);
    for col in (COLS - 3)..COLS {
        board.set(col, 4, true);
    }
    assert_eq!(live_path(&board).matches('M').count(), 1);
    assert!(live_path(&board).contains("h42v14h-42z"));
}

#[test]
fn the_area_of_the_path_is_the_area_of_the_live_cells() {
    // Every rectangle is a whole number of cells wide and one cell high, so the cells add up to the population.
    let mut board = Board::new(COLS, ROWS, true);
    board.randomise(99, 0.4);
    let path = live_path(&board);
    let mut cells = 0.0;
    for piece in path.split('M').filter(|p| !p.is_empty()) {
        let width: f64 = piece
            .split('h')
            .nth(1)
            .and_then(|rest| rest.split('v').next())
            .and_then(|w| w.parse().ok())
            .unwrap_or(f64::NAN);
        cells += width / CELL;
    }
    assert!(near(cells, board.population() as f64), "{cells} against {}", board.population());
}

#[test]
fn the_grid_has_a_line_between_every_cell_and_one_round_the_board() {
    let path = grid_path();
    assert_eq!(path.matches('v').count(), COLS + 1);
    assert_eq!(path.matches('h').count(), ROWS + 1);
    assert!(path.starts_with(&format!("M0 {TOP}v420")));
}

#[test]
fn the_arrow_keys_move_the_cursor_one_cell_and_go_round_at_an_edge() {
    assert_eq!(move_cursor((5, 5), "ArrowLeft"), Some((4, 5)));
    assert_eq!(move_cursor((5, 5), "ArrowRight"), Some((6, 5)));
    assert_eq!(move_cursor((5, 5), "ArrowUp"), Some((5, 4)));
    assert_eq!(move_cursor((5, 5), "ArrowDown"), Some((5, 6)));
    assert_eq!(move_cursor((0, 7), "ArrowLeft"), Some((COLS - 1, 7)));
    assert_eq!(move_cursor((COLS - 1, 7), "ArrowRight"), Some((0, 7)));
    assert_eq!(move_cursor((7, 0), "ArrowUp"), Some((7, ROWS - 1)));
    assert_eq!(move_cursor((7, ROWS - 1), "ArrowDown"), Some((7, 0)));
}

#[test]
fn a_key_that_is_not_an_arrow_does_not_move_the_cursor() {
    for key in [" ", "Enter", "a", "Home", "Tab", "arrowleft"] {
        assert_eq!(move_cursor((3, 3), key), None, "{key}");
    }
}

#[test]
fn the_buttons_above_the_board_are_side_by_side_inside_the_scene_and_do_not_overlap() {
    let areas: Vec<Rect> = (0..CONTROLS.len()).map(control_area).collect();
    assert!(near(areas[0].origin.x, MARGIN));
    for pair in areas.windows(2) {
        assert!(pair[1].origin.x >= pair[0].origin.x + pair[0].size.width, "{pair:?}");
    }
    let last = areas[areas.len() - 1];
    assert!(last.origin.x + last.size.width <= SCENE.width - MARGIN + 1e-9, "{last:?}");
    assert!(areas.iter().all(|a| a.origin.y + a.size.height < TOP && a.origin.y >= 0.0));
    assert_eq!(
        control_area(CONTROLS.len()).size.width,
        0.0,
        "a button that is not there has no width"
    );
}

#[test]
fn the_buttons_for_the_patterns_are_in_the_middle_of_the_strip_under_the_board() {
    let count = PATTERNS.len();
    let areas: Vec<Rect> = (0..count).map(|i| pattern_area(i, count)).collect();
    let left = areas[0].origin.x;
    let right = areas[count - 1].origin.x + areas[count - 1].size.width;
    assert!(near(left, SCENE.width - right), "{left} and {}", SCENE.width - right);
    for pair in areas.windows(2) {
        assert!(pair[1].origin.x > pair[0].origin.x + pair[0].size.width);
    }
    assert!(
        areas
            .iter()
            .all(|a| a.origin.y >= TOP + BOARD.height && a.origin.y + a.size.height <= SCENE.height)
    );
    assert!(left >= 0.0 && right <= SCENE.width);
}

#[test]
fn the_slider_for_the_speed_is_in_the_strip_and_inside_the_scene() {
    const {
        assert!(SLIDER_LEFT + SLIDER_LENGTH < SCENE.width);
        assert!(SLIDER_MIDDLE < TOP && SLIDER_MIDDLE > BUTTONS_TOP + BUTTON_HEIGHT);
        assert!(FIRST_SPEED >= SPEEDS.0 && FIRST_SPEED <= SPEEDS.1);
    }
    assert!(
        near(((FIRST_SPEED - SPEEDS.0) / SPEED_STEP).fract(), 0.0),
        "a speed that the slider can take"
    );
}

#[test]
fn the_readout_says_the_generation_and_the_population_and_counts_one_cell_in_the_singular() {
    assert_eq!(readout(0, 5), "Generation 0: 5 live cells");
    assert_eq!(readout(12, 1), "Generation 12: 1 live cell");
    assert_eq!(readout(100_000, 0), "Generation 100000: 0 live cells");
}

#[test]
fn the_cursor_is_described_with_its_column_and_row_from_one_and_whether_the_cell_is_alive() {
    assert_eq!(cursor_text((0, 0), false), "Column 1, row 1, dead");
    assert_eq!(cursor_text((4, 2), true), "Column 5, row 3, alive");
}

#[test]
fn the_speed_is_written_as_a_whole_number_in_each_second() {
    assert_eq!(speed_text(8.0), "8 per second");
    assert_eq!(speed_text(30.0), "30 per second");
}

#[test]
fn the_board_and_the_scene_are_described_for_assistive_technology() {
    let board = board_label();
    assert!(board.contains("40 columns by 30 rows") && board.contains("arrow keys") && board.contains("Space"));
    assert!(scene_label(true).contains("wrap round") && scene_label(false).contains("stop"));
    assert!(scene_label(true).contains("Game of Life"));
}

#[test]
fn the_strip_under_the_board_has_room_for_two_rows_of_buttons() {
    assert!(near(BOTTOM, 80.0));
    let first = pattern_area(0, PATTERNS.len());
    let second = spaceship_area(0, SPACESHIPS.len());
    assert!(
        second.origin.y >= first.origin.y + first.size.height,
        "the second row is below the first"
    );
    assert!(near(first.origin.y, TOP + BOARD.height + 8.0));
    assert!(
        near(SCENE.height - (second.origin.y + second.size.height), 8.0),
        "the same room under the second row"
    );
}

#[test]
fn the_buttons_for_the_spaceships_are_in_the_middle_of_the_second_row_and_inside_the_scene() {
    let count = SPACESHIPS.len();
    let areas: Vec<Rect> = (0..count).map(|i| spaceship_area(i, count)).collect();
    let left = areas[0].origin.x;
    let right = areas[count - 1].origin.x + areas[count - 1].size.width;
    assert!(near(left, SCENE.width - right), "{left} and {}", SCENE.width - right);
    assert!(left >= 0.0 && right <= SCENE.width);
    for pair in areas.windows(2) {
        assert!(pair[1].origin.x > pair[0].origin.x + pair[0].size.width);
    }
    assert!(areas.iter().all(|a| near(a.origin.y, areas[0].origin.y)), "they are in one row");
}

#[test]
fn no_button_in_the_strip_under_the_board_overlaps_another() {
    let mut areas: Vec<Rect> = (0..PATTERNS.len()).map(|i| pattern_area(i, PATTERNS.len())).collect();
    areas.extend((0..SPACESHIPS.len()).map(|i| spaceship_area(i, SPACESHIPS.len())));
    for (i, a) in areas.iter().enumerate() {
        for b in &areas[i + 1..] {
            let apart = a.origin.x + a.size.width <= b.origin.x
                || b.origin.x + b.size.width <= a.origin.x
                || a.origin.y + a.size.height <= b.origin.y
                || b.origin.y + b.size.height <= a.origin.y;
            assert!(apart, "{a:?} and {b:?}");
        }
    }
}

#[test]
fn the_labels_of_the_spaceships_fit_in_their_buttons() {
    // The letters are about 9 pixels wide, so a label has to be that many times its length, and room at the sides.
    for ship in SPACESHIPS {
        assert!(ship.name.len() as f64 * 9.0 + 16.0 <= SPACESHIP_WIDTH, "{}", ship.name);
    }
    assert!(scene_label(true).contains("spaceship"));
}
