//! The rules, the patterns, the edges, the random board and the pace.

use super::*;

/// A board of `cols` by `rows` from rows of text in which `O` is alive.
fn board_of(rows: &[&str], wrap: bool) -> Board {
    let mut board = Board::new(rows.iter().map(|r| r.len()).max().unwrap_or(0), rows.len(), wrap);
    for (row, text) in rows.iter().enumerate() {
        for (col, c) in text.bytes().enumerate() {
            board.set(col, row, c == b'O');
        }
    }
    board
}

/// The live cells, sorted, as a list that can be compared.
fn live(board: &Board) -> Vec<(usize, usize)> {
    let mut cells: Vec<_> = board.live_cells().collect();
    cells.sort_unstable();
    cells
}

fn steps(board: &mut Board, n: usize) {
    for _ in 0..n {
        board.step();
    }
}

// The four rules, one at a time, on a board that is large enough for the edges not to matter.

#[test]
fn a_live_cell_with_fewer_than_two_live_neighbours_dies_of_loneliness() {
    let mut alone = board_of(&[".....", "..O..", "....."], false);
    alone.step();
    assert_eq!(alone.population(), 0);
    let mut one_neighbour = board_of(&[".....", ".OO..", "....."], false);
    one_neighbour.step();
    assert_eq!(
        one_neighbour.population(),
        0,
        "both of two cells next to each other have one neighbour"
    );
}

#[test]
fn a_live_cell_with_two_or_three_live_neighbours_survives() {
    // The middle cell of a row of three has two neighbours, and survives.
    let mut row = board_of(&[".....", ".OOO.", "....."], false);
    assert_eq!(row.live_neighbours(2, 1), 2);
    row.step();
    assert!(row.get(2, 1));
    // Every cell of a block has three neighbours, and survives.
    let mut block = board_of(&["....", ".OO.", ".OO.", "...."], false);
    assert_eq!(block.live_neighbours(1, 1), 3);
    block.step();
    assert_eq!(block.population(), 4);
}

#[test]
fn a_live_cell_with_more_than_three_live_neighbours_dies_of_crowding() {
    let mut plus = board_of(&[".O.", "OOO", ".O."], false);
    assert_eq!(plus.live_neighbours(1, 1), 4);
    plus.step();
    assert!(!plus.get(1, 1), "the middle of a plus has four neighbours");
}

#[test]
fn a_dead_cell_with_exactly_three_live_neighbours_is_born_and_with_any_other_number_is_not() {
    let mut three = board_of(&["OO.", "O..", "..."], false);
    assert_eq!(three.live_neighbours(1, 1), 3);
    three.step();
    assert!(three.get(1, 1), "born");
    let mut two = board_of(&["OO.", "...", "..."], false);
    two.step();
    assert!(!two.get(1, 1), "two neighbours is too few");
    let mut four = board_of(&["OOO", "O..", "..."], false);
    assert_eq!(four.live_neighbours(1, 1), 4);
    four.step();
    assert!(!four.get(1, 1), "four neighbours is too many");
}

#[test]
fn every_cell_changes_at_once_by_what_the_generation_before_had() {
    // If a cell changed as the step reached it, the row of three would not turn into a column of three.
    let mut blinker = board_of(&[".....", ".....", ".OOO.", ".....", "....."], false);
    blinker.step();
    assert_eq!(live(&blinker), vec![(2, 1), (2, 2), (2, 3)]);
}

// Patterns that are known to behave in a particular way.

#[test]
fn a_block_is_a_still_life_that_never_changes() {
    let mut block = board_of(&["....", ".OO.", ".OO.", "...."], false);
    let before = block.clone();
    steps(&mut block, 5);
    assert_eq!(block, before);
}

#[test]
fn a_blinker_has_a_period_of_two() {
    let mut blinker = board_of(&[".....", ".....", ".OOO.", ".....", "....."], false);
    let first = blinker.clone();
    blinker.step();
    assert_ne!(blinker, first);
    blinker.step();
    assert_eq!(blinker, first);
}

#[test]
fn a_glider_moves_one_cell_down_and_one_across_every_four_generations() {
    let mut board = Board::new(12, 12, false);
    board.place(&PATTERNS[0], 2, 2);
    let start: Vec<_> = live(&board);
    steps(&mut board, 4);
    let moved: Vec<_> = start.iter().map(|&(c, r)| (c + 1, r + 1)).collect();
    assert_eq!(live(&board), moved);
    steps(&mut board, 4);
    let moved_again: Vec<_> = start.iter().map(|&(c, r)| (c + 2, r + 2)).collect();
    assert_eq!(live(&board), moved_again);
}

#[test]
fn a_glider_that_stops_at_the_edge_ends_as_a_block() {
    // Where the edges stop, a glider that reaches the corner is left as a block, which is still.
    let mut board = Board::new(10, 10, false);
    board.place(&PATTERNS[0], 0, 0);
    steps(&mut board, 60);
    assert_eq!(board.population(), 4);
    let settled = board.clone();
    board.step();
    assert_eq!(board, settled);
}

#[test]
fn a_glider_on_a_board_with_wrapping_edges_comes_back_to_where_it_began() {
    // It goes one cell on the diagonal every four generations, so it goes round a square board of 8 in 32.
    let mut board = Board::new(8, 8, true);
    board.place(&PATTERNS[0], 1, 1);
    let start = board.clone();
    steps(&mut board, 31);
    assert_ne!(board, start);
    board.step();
    assert_eq!(board, start);
}

#[test]
fn the_gosper_gun_makes_a_glider_every_thirty_generations() {
    // Far from every edge, the gun comes back to what it was after 30 generations, with one glider of 5 cells more.
    let mut board = Board::new(90, 60, false);
    board.place(&PATTERNS[4], 5, 5);
    let gun = PATTERNS[4].cells().len();
    assert_eq!(board.population(), gun);
    steps(&mut board, 30);
    assert_eq!(board.population(), gun + 5);
    steps(&mut board, 30);
    assert_eq!(board.population(), gun + 10);
    // The gun itself is where it was, since every one of its cells is alive again.
    assert!(PATTERNS[4].cells().iter().all(|&(c, r)| board.get(5 + c, 5 + r)));
}

#[test]
fn the_r_pentomino_does_not_settle_for_a_long_time() {
    let mut board = Board::new(80, 60, false);
    board.place_centred(&PATTERNS[2]);
    steps(&mut board, 100);
    assert!(board.population() > 20, "{}", board.population());
}

// The edges.

#[test]
fn where_the_edges_wrap_a_cell_in_a_corner_is_next_to_the_cells_in_the_other_three() {
    let mut board = Board::new(5, 4, true);
    for (c, r) in [(4, 0), (0, 3), (4, 3)] {
        board.set(c, r, true);
    }
    assert_eq!(board.live_neighbours(0, 0), 3);
    board.set_wrap(false);
    assert_eq!(board.live_neighbours(0, 0), 0, "where they stop, those places are far away");
}

#[test]
fn where_the_edges_stop_a_place_beyond_one_is_dead() {
    let board = board_of(&["OOO", "OOO", "OOO"], false);
    assert_eq!(board.live_neighbours(0, 0), 3);
    assert_eq!(board.live_neighbours(1, 0), 5);
    assert_eq!(board.live_neighbours(1, 1), 8);
}

#[test]
fn a_pattern_that_goes_past_an_edge_comes_back_at_the_other_where_they_wrap_and_is_cut_where_they_stop() {
    let mut wrapped = Board::new(6, 6, true);
    wrapped.place(&PATTERNS[1], 4, 0);
    assert_eq!(live(&wrapped), vec![(0, 0), (4, 0), (5, 0)]);
    let mut stopped = Board::new(6, 6, false);
    stopped.place(&PATTERNS[1], 4, 0);
    assert_eq!(live(&stopped), vec![(4, 0), (5, 0)]);
}

#[test]
fn a_board_one_cell_across_does_not_fail() {
    let mut board = Board::new(1, 1, true);
    board.set(0, 0, true);
    board.step();
    assert_eq!(
        board.population(),
        0,
        "the cell is its own eight neighbours, and that is too many"
    );
    let mut thin = Board::new(3, 1, true);
    thin.place(&PATTERNS[1], 0, 0);
    thin.step();
    assert!(thin.population() <= 3);
}

// The cells.

#[test]
fn a_place_that_is_not_on_the_board_is_dead_and_cannot_be_made_alive() {
    let mut board = Board::new(3, 2, false);
    board.set(3, 0, true);
    board.set(0, 2, true);
    assert_eq!(board.population(), 0);
    assert!(!board.get(3, 0) && !board.get(0, 2) && !board.get(99, 99));
}

#[test]
fn a_toggle_says_what_the_cell_is_now_and_a_second_puts_it_back() {
    let mut board = Board::new(4, 4, false);
    assert!(board.toggle(2, 1));
    assert!(board.get(2, 1));
    assert!(!board.toggle(2, 1));
    assert_eq!(board.population(), 0);
}

#[test]
fn the_live_cells_are_listed_row_by_row_from_the_top_left() {
    let board = board_of(&[".O.", "O..", "..O"], false);
    assert_eq!(board.live_cells().collect::<Vec<_>>(), vec![(1, 0), (0, 1), (2, 2)]);
    let mut cleared = board.clone();
    cleared.clear();
    assert_eq!(cleared.population(), 0);
}

#[test]
fn changing_the_edges_leaves_the_cells_as_they_were() {
    let mut board = board_of(&["O.O", "...", "O.O"], true);
    let cells = live(&board);
    board.set_wrap(false);
    assert!(!board.wraps());
    assert_eq!(live(&board), cells);
}

// The patterns.

#[test]
fn the_patterns_are_rectangles_of_live_and_dead_cells_with_a_name_that_can_be_found() {
    for pattern in PATTERNS {
        assert!(pattern.rows.iter().all(|row| row.len() == pattern.width()), "{}", pattern.name);
        assert!(pattern.rows.iter().all(|row| row.bytes().all(|c| c == b'O' || c == b'.')));
        assert!(!pattern.cells().is_empty());
        assert_eq!(super::pattern(pattern.id), Some(&pattern));
    }
    assert!(super::pattern("no such pattern").is_none());
    let ids: Vec<_> = PATTERNS.iter().map(|p| p.id).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len(), "the ids are all different");
}

#[test]
fn the_sizes_and_cells_of_the_patterns_are_as_they_are_drawn() {
    assert_eq!(
        (PATTERNS[0].width(), PATTERNS[0].height(), PATTERNS[0].cells().len()),
        (3, 3, 5)
    );
    assert_eq!(
        (PATTERNS[1].width(), PATTERNS[1].height(), PATTERNS[1].cells().len()),
        (3, 1, 3)
    );
    assert_eq!(
        (PATTERNS[2].width(), PATTERNS[2].height(), PATTERNS[2].cells().len()),
        (3, 3, 5)
    );
    assert_eq!(
        (PATTERNS[3].width(), PATTERNS[3].height(), PATTERNS[3].cells().len()),
        (7, 3, 7)
    );
    assert_eq!(
        (PATTERNS[4].width(), PATTERNS[4].height(), PATTERNS[4].cells().len()),
        (36, 9, 36)
    );
    assert_eq!(PATTERNS[0].cells(), vec![(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)]);
}

#[test]
fn every_pattern_fits_on_the_board_of_the_page() {
    for pattern in PATTERNS {
        let mut board = Board::new(40, 30, true);
        board.place_centred(&pattern);
        assert_eq!(board.population(), pattern.cells().len(), "{}", pattern.name);
    }
}

#[test]
fn a_pattern_is_put_in_the_middle_as_near_as_the_cells_allow() {
    let mut board = Board::new(10, 10, false);
    board.place_centred(&PATTERNS[1]);
    assert_eq!(live(&board), vec![(3, 4), (4, 4), (5, 4)]);
    let mut too_big = Board::new(2, 2, false);
    too_big.place_centred(&PATTERNS[0]);
    assert!(
        too_big.population() <= 4,
        "a pattern bigger than the board is cut off and does not fail"
    );
}

// The random board.

#[test]
fn a_random_board_is_the_same_for_the_same_seed_and_not_for_another() {
    let (mut a, mut b, mut c) = (Board::new(20, 20, true), Board::new(20, 20, true), Board::new(20, 20, true));
    a.randomise(7, 0.3);
    b.randomise(7, 0.3);
    c.randomise(8, 0.3);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn the_density_of_a_random_board_is_near_what_was_asked_and_the_ends_are_all_or_none() {
    let mut board = Board::new(60, 60, true);
    board.randomise(12345, 0.25);
    let fraction = board.population() as f64 / 3600.0;
    assert!((fraction - 0.25).abs() < 0.03, "{fraction}");
    board.randomise(1, 0.0);
    assert_eq!(board.population(), 0);
    board.randomise(1, 1.0);
    assert_eq!(board.population(), 3600);
}

#[test]
fn a_random_board_kills_the_cells_that_were_there() {
    let mut board = Board::new(10, 10, true);
    board.place_centred(&PATTERNS[0]);
    board.randomise(3, 0.0);
    assert_eq!(board.population(), 0);
}

#[test]
fn the_generator_gives_numbers_from_0_to_less_than_1_and_a_seed_of_0_still_works() {
    let mut rng = Rng::new(0);
    let numbers: Vec<f64> = (0..1000).map(|_| rng.next_unit()).collect();
    assert!(numbers.iter().all(|n| (0.0..1.0).contains(n)));
    assert!(numbers.windows(2).any(|w| w[0] != w[1]), "it does not stay still");
    assert_eq!(Rng::new(5), Rng::new(5));
}

// The pace.

#[test]
fn a_generation_is_due_each_period_and_the_time_that_is_left_is_carried_on() {
    assert_eq!(steps_due(0.0, 50.0, 100.0, 5), (0, 50.0));
    let (n, carried) = steps_due(50.0, 60.0, 100.0, 5);
    assert_eq!(n, 1);
    assert!((carried - 10.0).abs() < 1e-9);
    let (n, carried) = steps_due(0.0, 250.0, 100.0, 5);
    assert_eq!(n, 2);
    assert!((carried - 50.0).abs() < 1e-9);
}

#[test]
fn no_more_than_the_most_are_due_at_once_and_the_rest_is_let_go() {
    assert_eq!(steps_due(0.0, 10_000.0, 100.0, 5), (5, 0.0));
    assert_eq!(steps_due(0.0, 500.0, 100.0, 5), (5, 0.0));
}

#[test]
fn time_that_is_not_a_number_or_goes_backwards_gives_nothing() {
    assert_eq!(steps_due(0.0, f64::NAN, 100.0, 5), (0, 0.0));
    assert_eq!(steps_due(f64::INFINITY, 10.0, 100.0, 5), (0, 0.0));
    assert_eq!(steps_due(0.0, 10.0, 0.0, 5), (0, 0.0));
    assert_eq!(steps_due(0.0, 10.0, f64::NAN, 5), (0, 0.0));
    assert_eq!(steps_due(0.0, -40.0, 100.0, 5), (0, 0.0));
}

#[test]
fn the_period_is_a_thousand_milliseconds_over_the_speed() {
    assert!((period_ms(8.0) - 125.0).abs() < 1e-9);
    assert!((period_ms(1.0) - 1000.0).abs() < 1e-9);
    assert!(period_ms(0.0).is_finite() || period_ms(0.0).is_infinite());
    assert!(
        period_ms(0.0) > 1e300,
        "a speed of nothing is a very long wait and not a division by zero"
    );
}

// The spaceships.

/// The cells of `board` as a set of places, moved so that the top left of the smallest rectangle that holds them is at
/// 0 and 0, with how far that was moved. It is how to tell that a pattern has moved without having changed shape.
fn shape_and_place(board: &Board) -> (Vec<(usize, usize)>, (usize, usize)) {
    let cells = live(board);
    let left = cells.iter().map(|&(c, _)| c).min().unwrap_or(0);
    let top = cells.iter().map(|&(_, r)| r).min().unwrap_or(0);
    let mut shape: Vec<_> = cells.iter().map(|&(c, r)| (c - left, r - top)).collect();
    shape.sort_unstable();
    (shape, (left, top))
}

#[test]
fn the_spaceships_have_the_cells_that_their_weights_say_and_names_that_can_be_found() {
    let sizes: Vec<_> = SPACESHIPS.iter().map(|ship| ship.cells().len()).collect();
    assert_eq!(sizes, vec![9, 11, 13], "light, medium and heavy");
    assert_eq!(SPACESHIPS.map(|s| (s.width(), s.height())), [(5, 4), (6, 5), (7, 5)]);
    for ship in SPACESHIPS {
        assert!(ship.rows.iter().all(|row| row.len() == ship.width()), "{}", ship.name);
        assert_eq!(spaceship(ship.id), Some(&ship));
        assert!(
            pattern(ship.id).is_none(),
            "a spaceship is not one of the patterns that replace the board"
        );
    }
    assert!(spaceship("glider").is_none());
    let mut ids: Vec<_> = SPACESHIPS.iter().chain(PATTERNS.iter()).map(|p| p.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), SPACESHIPS.len() + PATTERNS.len(), "no id is used twice");
}

#[test]
fn every_spaceship_goes_two_cells_to_the_right_every_four_generations_and_does_not_change_shape() {
    for ship in SPACESHIPS {
        let mut board = Board::new(60, 20, false);
        board.place(&ship, 5, 5);
        let (shape, (left, top)) = shape_and_place(&board);
        for lap in 1..=6 {
            steps(&mut board, 4);
            let (now, (moved_left, moved_top)) = shape_and_place(&board);
            assert_eq!(now, shape, "{} after {} generations", ship.name, lap * 4);
            assert_eq!((moved_left, moved_top), (left + 2 * lap, top), "{}", ship.name);
        }
    }
}

#[test]
fn between_laps_a_spaceship_is_in_other_shapes_but_has_the_same_number_of_cells_every_four() {
    // The shape that repeats is the one every four generations, and the ones between are other shapes.
    let mut board = Board::new(60, 20, false);
    board.place(&SPACESHIPS[0], 5, 5);
    let (shape, _) = shape_and_place(&board);
    board.step();
    assert_ne!(shape_and_place(&board).0, shape);
}

#[test]
fn a_spaceship_on_a_board_with_wrapping_edges_comes_back_to_where_it_began() {
    // 2 cells every 4 generations on a board 20 across is round in 40.
    let mut board = Board::new(20, 12, true);
    board.place(&SPACESHIPS[0], 0, 4);
    let start = board.clone();
    steps(&mut board, 36);
    assert_ne!(board, start);
    steps(&mut board, 4);
    assert_eq!(board, start);
}

#[test]
fn a_spaceship_that_reaches_a_stopping_edge_does_not_come_back() {
    let mut board = Board::new(30, 12, false);
    board.place(&SPACESHIPS[2], 0, 3);
    steps(&mut board, 4 * 40);
    assert!(
        board.population() < 13,
        "the ship has gone, or been broken up, at the edge: {}",
        board.population()
    );
}

#[test]
fn a_spaceship_is_added_at_the_left_edge_level_with_the_row_and_what_was_there_stays() {
    let mut board = Board::new(40, 30, true);
    board.set(30, 10, true);
    board.place_level_with(&SPACESHIPS[0], 15);
    assert_eq!(board.population(), 1 + 9);
    assert!(board.get(30, 10), "the cell that was there is alive");
    let (_, (left, top)) = shape_and_place(&{
        let mut only = Board::new(40, 30, true);
        only.place_level_with(&SPACESHIPS[0], 15);
        only
    });
    assert_eq!(left, 0, "at the left edge");
    assert_eq!(top, 13, "the height of 4 is level with row 15 about its middle");
}

#[test]
fn a_spaceship_level_with_a_row_near_an_edge_is_moved_to_be_wholly_on_the_board() {
    for (row, wanted_top) in [(0, 0), (1, 0), (29, 25), (28, 25), (27, 25), (26, 24)] {
        let mut board = Board::new(40, 30, false);
        board.place_level_with(&SPACESHIPS[1], row);
        let (_, (_, top)) = shape_and_place(&board);
        assert_eq!(board.population(), 11, "none is cut off at row {row}");
        assert_eq!(top, wanted_top, "row {row}");
    }
}

#[test]
fn adding_the_same_spaceship_twice_in_the_same_place_adds_nothing_more() {
    let mut board = Board::new(40, 30, true);
    board.place_level_with(&SPACESHIPS[2], 15);
    let once = board.clone();
    board.place_level_with(&SPACESHIPS[2], 15);
    assert_eq!(board, once);
}

#[test]
fn a_spaceship_too_big_for_the_board_is_cut_off_and_does_not_fail() {
    let mut board = Board::new(3, 2, false);
    board.place_level_with(&SPACESHIPS[2], 1);
    assert!(board.population() <= 6);
}

// The size of a board.

#[test]
fn a_board_with_no_cells_across_or_down_is_made_one_cell_across_or_down() {
    for (cols, rows, wanted) in [(0, 0, (1, 1)), (0, 5, (1, 5)), (7, 0, (7, 1)), (3, 2, (3, 2))] {
        let board = Board::new(cols, rows, true);
        assert_eq!((board.cols(), board.rows()), wanted, "{cols} by {rows}");
    }
}

#[test]
fn no_method_fails_on_a_board_that_was_asked_to_be_nothing_across_or_down() {
    for wrap in [true, false] {
        let mut board = Board::new(0, 0, wrap);
        board.place(&PATTERNS[4], 3, 3);
        board.place_centred(&PATTERNS[0]);
        board.place_level_with(&SPACESHIPS[2], 9);
        board.randomise(5, 0.5);
        assert_eq!(
            board.live_neighbours(0, 0),
            if wrap { u8::from(board.get(0, 0)) * 8 } else { 0 }
        );
        board.step();
        board.set(0, 0, true);
        assert!(board.toggle(5, 5) || !board.get(5, 5));
        assert!(board.population() <= 1);
        board.clear();
        assert_eq!(board.population(), 0);
        assert_eq!(board.live_cells().count(), 0);
    }
}

#[test]
fn a_pattern_put_on_a_board_of_one_cell_wraps_onto_it_and_is_cut_off_where_the_edges_stop() {
    let mut wrapped = Board::new(0, 0, true);
    wrapped.place(&PATTERNS[1], 0, 0);
    assert_eq!(wrapped.population(), 1, "every cell of the blinker is on the one place");
    let mut stopped = Board::new(0, 0, false);
    stopped.place(&PATTERNS[1], 0, 0);
    assert_eq!(stopped.population(), 1, "only the first cell is on the board");
}

#[test]
fn the_one_cell_that_a_board_of_nothing_has_obeys_the_rules() {
    let mut wrapped = Board::new(0, 0, true);
    wrapped.set(0, 0, true);
    wrapped.step();
    assert!(
        !wrapped.get(0, 0),
        "where the edges wrap it is its own eight neighbours, which are too many"
    );
    let mut stopped = Board::new(0, 0, false);
    stopped.set(0, 0, true);
    stopped.step();
    assert!(!stopped.get(0, 0), "where they stop it has none, which is too few");
}
