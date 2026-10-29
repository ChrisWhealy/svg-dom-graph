use super::{
    function::{
        FORMAT_BITS, format_copy_a, format_copy_b, function_modules, placement_order, reserve_function_modules,
    },
    grid::{BitGrid, CellState, DataRole, FunctionRole, GridError, QrModuleKind, SIZE},
    paths::{base_paths, highlight_path, matrix_text, role_path},
    spike::{self, CODEWORDS, spike_steps},
    text::wrap,
    {
        CAPTION_LINES, EXPLANATION_LINES, KEY, KEY_LINES, LEFT_COLUMN_CHARS, NO_ROLE, NO_ROLE_EXPLANATION,
        RIGHT_COLUMN_CHARS, STATS_LINES, caption_text, role_action, stats_text,
    },
};
use std::collections::HashSet;

const FINDER: QrModuleKind = QrModuleKind::Function(FunctionRole::Finder);
const FORMAT: QrModuleKind = QrModuleKind::Function(FunctionRole::Format);

fn grid() -> BitGrid {
    BitGrid::new(SIZE, SIZE)
}

fn reserved() -> BitGrid {
    let mut grid = grid();
    reserve_function_modules(&mut grid).expect("the function modules fit an empty grid");
    grid
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The grid
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_new_grid_has_no_colour_and_no_role() {
    let grid = grid();
    assert_eq!(grid.count_state(CellState::Unassigned), 441);
    assert_eq!(grid.role(0, 0), None);
    assert_eq!(grid.check_invariants(), Ok(()));
}

#[test]
fn a_position_outside_the_grid_is_refused() {
    let mut grid = grid();
    assert_eq!(grid.get(21, 0), None);
    assert_eq!(grid.get(0, 21), None);
    assert_eq!(
        grid.reserve(21, 0, FunctionRole::Finder),
        Err(GridError::OutOfBounds { row: 21, col: 0 })
    );
    assert_eq!(
        grid.assign(0, 21, true, DataRole::Data),
        Err(GridError::OutOfBounds { row: 0, col: 21 })
    );
    assert_eq!(grid.flip_data(21, 21), Err(GridError::OutOfBounds { row: 21, col: 21 }));
}

#[test]
fn reserving_gives_a_role_and_no_colour() {
    let mut grid = grid();
    grid.reserve(8, 0, FunctionRole::Format)
        .expect("an empty module can be reserved");
    assert_eq!(grid.role(8, 0), Some(FORMAT));
    assert_eq!(grid.get(8, 0), Some(CellState::Unassigned));
    assert_eq!(grid.check_invariants(), Ok(()));
}

#[test]
fn a_module_cannot_be_reserved_twice() {
    let mut grid = grid();
    grid.reserve(1, 1, FunctionRole::Finder)
        .expect("the first reservation succeeds");
    assert_eq!(
        grid.reserve(1, 1, FunctionRole::Timing),
        Err(GridError::Occupied { row: 1, col: 1 })
    );
    assert_eq!(grid.role(1, 1), Some(FINDER));
}

#[test]
fn assigning_colours_the_module_and_gives_it_a_payload_role() {
    let mut grid = grid();
    grid.assign(3, 4, true, DataRole::Data)
        .expect("an empty module can be assigned");
    grid.assign(3, 5, false, DataRole::ErrorCorrection)
        .expect("an empty module can be assigned");
    assert_eq!(grid.get(3, 4), Some(CellState::Dark));
    assert_eq!(grid.get(3, 5), Some(CellState::Light));
    assert_eq!(grid.role(3, 4), Some(QrModuleKind::Payload(DataRole::Data)));
    assert_eq!(grid.role(3, 5), Some(QrModuleKind::Payload(DataRole::ErrorCorrection)));
    assert_eq!(grid.check_invariants(), Ok(()));
}

#[test]
fn assigning_cannot_overwrite_a_reserved_or_assigned_module() {
    let mut grid = grid();
    grid.reserve(0, 0, FunctionRole::Finder)
        .expect("an empty module can be reserved");
    grid.assign(5, 5, true, DataRole::Data)
        .expect("an empty module can be assigned");
    assert_eq!(
        grid.assign(0, 0, true, DataRole::Data),
        Err(GridError::Occupied { row: 0, col: 0 })
    );
    assert_eq!(
        grid.assign(5, 5, false, DataRole::Data),
        Err(GridError::Occupied { row: 5, col: 5 })
    );
    assert_eq!(grid.get(5, 5), Some(CellState::Dark));
    assert_eq!(grid.get(0, 0), Some(CellState::Unassigned));
}

#[test]
fn a_reserved_module_is_coloured_only_for_its_own_role() {
    let mut grid = grid();
    grid.reserve(8, 0, FunctionRole::Format)
        .expect("an empty module can be reserved");
    assert_eq!(
        grid.fill_reserved(8, 0, true, FunctionRole::Timing),
        Err(GridError::NotReserved { row: 8, col: 0 })
    );
    assert_eq!(grid.get(8, 0), Some(CellState::Unassigned));
    grid.fill_reserved(8, 0, true, FunctionRole::Format).expect("the role matches");
    assert_eq!(grid.get(8, 0), Some(CellState::Dark));
    assert_eq!(grid.role(8, 0), Some(FORMAT));
}

#[test]
fn a_module_that_is_not_reserved_or_is_already_coloured_cannot_be_filled() {
    let mut grid = grid();
    assert_eq!(
        grid.fill_reserved(2, 2, true, FunctionRole::Format),
        Err(GridError::NotReserved { row: 2, col: 2 })
    );
    grid.reserve(2, 2, FunctionRole::Format)
        .expect("an empty module can be reserved");
    grid.fill_reserved(2, 2, true, FunctionRole::Format)
        .expect("the first fill succeeds");
    assert_eq!(
        grid.fill_reserved(2, 2, false, FunctionRole::Format),
        Err(GridError::NotReserved { row: 2, col: 2 })
    );
    assert_eq!(grid.get(2, 2), Some(CellState::Dark));
}

#[test]
fn flipping_inverts_a_payload_module_and_keeps_its_role() {
    let mut grid = grid();
    grid.assign(4, 4, true, DataRole::ErrorCorrection)
        .expect("an empty module can be assigned");
    grid.flip_data(4, 4).expect("a payload module can be flipped");
    assert_eq!(grid.get(4, 4), Some(CellState::Light));
    assert_eq!(grid.role(4, 4), Some(QrModuleKind::Payload(DataRole::ErrorCorrection)));
    grid.flip_data(4, 4).expect("a payload module can be flipped back");
    assert_eq!(grid.get(4, 4), Some(CellState::Dark));
    assert_eq!(grid.check_invariants(), Ok(()));
}

#[test]
fn flipping_refuses_function_modules_and_modules_without_a_colour() {
    let mut grid = reserved();
    assert_eq!(grid.flip_data(0, 0), Err(GridError::NotFlippable { row: 0, col: 0 }));
    assert_eq!(grid.flip_data(8, 0), Err(GridError::NotFlippable { row: 8, col: 0 }));
    assert_eq!(grid.flip_data(20, 20), Err(GridError::NotFlippable { row: 20, col: 20 }));
    assert_eq!(grid.get(0, 0), Some(CellState::Dark));
}

#[test]
fn a_refused_change_leaves_the_grid_as_it_was() {
    let mut grid = reserved();
    let before = grid.clone();
    assert!(grid.assign(0, 0, false, DataRole::Data).is_err());
    assert!(grid.reserve(0, 0, FunctionRole::Timing).is_err());
    assert!(grid.fill_reserved(8, 0, true, FunctionRole::Timing).is_err());
    assert!(grid.flip_data(0, 0).is_err());
    assert_eq!(grid, before);
}

#[test]
fn a_grid_that_does_not_fill_its_last_word_keeps_the_spare_bits_zero() {
    let mut grid = BitGrid::new(3, 3);
    for row in 0..3 {
        for col in 0..3 {
            grid.assign(row, col, true, DataRole::Data)
                .expect("an empty module can be assigned");
            grid.flip_data(row, col).expect("a payload module can be flipped");
            grid.flip_data(row, col).expect("a payload module can be flipped back");
        }
    }
    assert_eq!(grid.check_invariants(), Ok(()));
    assert_eq!(grid.count_state(CellState::Dark), 9);
}

#[test]
fn the_role_keys_are_distinct_and_read_back() {
    let keys: HashSet<_> = QrModuleKind::ALL.iter().map(|kind| kind.key()).collect();
    assert_eq!(keys.len(), 7);
    for kind in QrModuleKind::ALL {
        assert_eq!(QrModuleKind::from_key(kind.key()), Some(kind));
    }
    assert_eq!(QrModuleKind::from_key("none"), None);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The layout of a Version 1 symbol
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn there_are_233_function_modules_and_none_is_listed_twice() {
    let modules = function_modules();
    assert_eq!(modules.len(), 233);
    let positions: HashSet<_> = modules.iter().map(|module| (module.row, module.col)).collect();
    assert_eq!(positions.len(), 233);
}

#[test]
fn the_function_modules_split_into_the_known_patterns() {
    let count = |role| function_modules().iter().filter(|module| module.role == role).count();
    assert_eq!(count(FunctionRole::Finder), 147);
    assert_eq!(count(FunctionRole::Separator), 45);
    assert_eq!(count(FunctionRole::Timing), 10);
    assert_eq!(count(FunctionRole::DarkModule), 1);
    assert_eq!(count(FunctionRole::Format), 30);
}

#[test]
fn reserving_leaves_208_modules_for_codewords_and_only_the_format_modules_uncoloured() {
    let grid = reserved();
    assert_eq!(grid.count_function(), 233);
    assert_eq!(grid.count_state(CellState::Unassigned), 441 - 233 + 30);
    assert_eq!(grid.check_invariants(), Ok(()));
}

#[test]
fn reserving_twice_is_refused_and_changes_nothing() {
    let mut grid = reserved();
    let before = grid.clone();
    assert_eq!(reserve_function_modules(&mut grid), Err(GridError::Occupied { row: 0, col: 0 }));
    assert_eq!(grid, before);
}

#[test]
fn a_finder_pattern_is_a_dark_ring_a_light_ring_and_a_dark_centre() {
    let grid = reserved();
    let ring = |row, col| grid.get(row, col);
    assert_eq!(ring(0, 0), Some(CellState::Dark));
    assert_eq!(ring(1, 1), Some(CellState::Light));
    assert_eq!(ring(2, 2), Some(CellState::Dark));
    assert_eq!(ring(3, 3), Some(CellState::Dark));
    assert_eq!(ring(7, 7), Some(CellState::Light));
    assert_eq!(ring(0, 20), Some(CellState::Dark));
    assert_eq!(ring(20, 0), Some(CellState::Dark));
}

#[test]
fn the_timing_patterns_start_dark_and_alternate() {
    let grid = reserved();
    let row: Vec<_> = (8..13).map(|col| grid.get(6, col)).collect();
    let column: Vec<_> = (8..13).map(|row| grid.get(row, 6)).collect();
    let expected = [
        CellState::Dark,
        CellState::Light,
        CellState::Dark,
        CellState::Light,
        CellState::Dark,
    ]
    .map(Some);
    assert_eq!(row, expected);
    assert_eq!(column, expected);
}

#[test]
fn the_dark_module_is_dark() {
    assert_eq!(reserved().get(13, 8), Some(CellState::Dark));
}

#[test]
fn the_two_format_copies_are_where_the_design_puts_them() {
    let copy_a: Vec<_> = (0..FORMAT_BITS).map(format_copy_a).collect();
    let copy_b: Vec<_> = (0..FORMAT_BITS).map(format_copy_b).collect();
    assert_eq!(
        copy_a,
        [
            (0, 8),
            (1, 8),
            (2, 8),
            (3, 8),
            (4, 8),
            (5, 8),
            (7, 8),
            (8, 8),
            (8, 7),
            (8, 5),
            (8, 4),
            (8, 3),
            (8, 2),
            (8, 1),
            (8, 0)
        ]
    );
    assert_eq!(
        copy_b,
        [
            (8, 20),
            (8, 19),
            (8, 18),
            (8, 17),
            (8, 16),
            (8, 15),
            (8, 14),
            (8, 13),
            (14, 8),
            (15, 8),
            (16, 8),
            (17, 8),
            (18, 8),
            (19, 8),
            (20, 8)
        ]
    );
}

#[test]
fn the_format_modules_are_exactly_the_reserved_format_roles() {
    let grid = reserved();
    let reserved_format: HashSet<_> = grid.cells().filter(|&(row, col)| grid.role(row, col) == Some(FORMAT)).collect();
    let listed: HashSet<_> = (0..FORMAT_BITS)
        .flat_map(|bit| [format_copy_a(bit), format_copy_b(bit)])
        .collect();
    assert_eq!(reserved_format, listed);
    assert_eq!(listed.len(), 30);
}

#[test]
fn the_placement_order_visits_each_of_the_208_modules_once_and_never_a_function_module() {
    let grid = reserved();
    let order = placement_order(&grid);
    assert_eq!(order.len(), 208);
    assert_eq!(order.iter().copied().collect::<HashSet<_>>().len(), 208);
    assert!(order.iter().all(|&(row, col)| grid.role(row, col).is_none()));
    assert!(order.iter().all(|&(_, col)| col != 6));
}

#[test]
fn the_placement_starts_at_the_bottom_right_and_moves_up_in_pairs_of_columns() {
    let order = placement_order(&reserved());
    assert_eq!(order[..6], [(20, 20), (20, 19), (19, 20), (19, 19), (18, 20), (18, 19)]);
    // The finder patterns and format modules block the top of the first pair, so it ends at (9, 19). The next pair then
    // starts at the top of its own column and moves down, which is also blocked until row 9.
    let end = order.iter().position(|&cell| cell == (9, 19)).expect("(9, 19) is listed");
    assert_eq!(order[end + 1..end + 3], [(9, 18), (9, 17)]);
    assert_eq!(order[end + 3..end + 5], [(10, 18), (10, 17)]);
}

#[test]
fn the_vertical_timing_column_is_skipped_by_moving_the_pair_across_it() {
    let order = placement_order(&reserved());
    let last_of_pair_8_7 = order
        .iter()
        .rposition(|&(_, col)| col == 7 || col == 8)
        .expect("column 7 is used");
    assert_eq!(order[last_of_pair_8_7 + 1].1, 5);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The path data
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn an_empty_grid_is_all_unassigned_and_a_row_is_one_rectangle() {
    let paths = base_paths(&grid());
    assert_eq!(paths.light, "");
    assert_eq!(paths.dark, "");
    assert!(paths.unassigned.starts_with("M0 0h21v1h-21z M0 1h21v1h-21z"));
    assert_eq!(paths.unassigned.matches('M').count(), 21);
}

#[test]
fn a_run_of_neighbours_is_one_rectangle_and_a_gap_starts_another() {
    let mut grid = BitGrid::new(6, 2);
    for col in [0, 1, 2, 4] {
        grid.assign(1, col, true, DataRole::Data)
            .expect("an empty module can be assigned");
    }
    assert_eq!(base_paths(&grid).dark, "M0 1h3v1h-3z M4 1h1v1h-1z");
}

#[test]
fn every_module_is_in_exactly_one_base_path() {
    let steps = spike_steps().expect("the spike's steps are valid");
    for step in &steps {
        let paths = base_paths(&step.grid);
        let area = |d: &str| {
            d.split('z')
                .filter(|rectangle| !rectangle.is_empty())
                .map(|rectangle| {
                    let width = rectangle.split('h').nth(1).and_then(|rest| rest.split('v').next());
                    width
                        .and_then(|n| n.parse::<usize>().ok())
                        .expect("every rectangle has a width")
                })
                .sum::<usize>()
        };
        assert_eq!(
            area(&paths.unassigned) + area(&paths.light) + area(&paths.dark),
            441,
            "{}",
            step.label
        );
    }
}

#[test]
fn the_role_outline_holds_one_square_for_each_module_with_that_role() {
    let grid = reserved();
    assert_eq!(role_path(&grid, Some(FINDER)).matches('M').count(), 147);
    assert_eq!(role_path(&grid, Some(FORMAT)).matches('M').count(), 30);
    assert_eq!(role_path(&grid, Some(QrModuleKind::Payload(DataRole::Data))), "");
    assert_eq!(role_path(&grid, None), "");
}

#[test]
fn the_highlight_outline_holds_one_square_for_each_module() {
    assert_eq!(highlight_path(&[]), "");
    assert_eq!(highlight_path(&[(0, 0), (3, 4)]).matches('M').count(), 2);
}

#[test]
fn the_matrix_text_shows_the_three_states() {
    let mut grid = BitGrid::new(3, 2);
    grid.assign(0, 0, true, DataRole::Data)
        .expect("an empty module can be assigned");
    grid.assign(0, 1, false, DataRole::Data)
        .expect("an empty module can be assigned");
    assert_eq!(matrix_text(&grid), "10.\n...");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The scripted steps
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_spike_has_five_steps_and_every_grid_keeps_its_rules() {
    let steps = spike_steps().expect("the spike's steps are valid");
    assert_eq!(steps.len(), 5);
    for step in &steps {
        assert_eq!(step.grid.check_invariants(), Ok(()), "{}", step.label);
    }
}

#[test]
fn the_format_modules_stay_uncoloured_until_the_last_step() {
    let steps = spike_steps().expect("the spike's steps are valid");
    let coloured = |step: &spike::SpikeStep| {
        step.grid
            .cells()
            .filter(|&(row, col)| step.grid.role(row, col) == Some(FORMAT))
            .filter(|&(row, col)| step.grid.get(row, col) != Some(CellState::Unassigned))
            .count()
    };
    assert!(steps[..4].iter().all(|step| coloured(step) == 0));
    assert_eq!(coloured(&steps[4]), 30);
}

#[test]
fn the_codewords_are_placed_with_the_right_roles_and_leave_nothing_empty() {
    let steps = spike_steps().expect("the spike's steps are valid");
    let one = &steps[1].grid;
    let all = &steps[2].grid;
    let payload = |grid: &BitGrid, role| {
        grid.cells()
            .filter(|&(row, col)| grid.role(row, col) == Some(QrModuleKind::Payload(role)))
            .count()
    };
    assert_eq!(payload(one, DataRole::Data), 8);
    assert_eq!(payload(one, DataRole::ErrorCorrection), 0);
    assert_eq!(payload(all, DataRole::Data), 19 * 8);
    assert_eq!(payload(all, DataRole::ErrorCorrection), 7 * 8);
    assert_eq!(CODEWORDS, 26);
    assert_eq!(all.count_state(CellState::Unassigned), 30);
}

#[test]
fn the_mask_step_changes_only_payload_modules() {
    let steps = spike_steps().expect("the spike's steps are valid");
    let (before, after) = (&steps[2].grid, &steps[3].grid);
    let mut flipped = 0;
    for (row, col) in before.cells() {
        let changed = before.get(row, col) != after.get(row, col);
        let is_payload = matches!(before.role(row, col), Some(QrModuleKind::Payload(_)));
        assert_eq!(changed, is_payload && (row + col) % 2 == 0, "({row}, {col})");
        flipped += usize::from(changed);
    }
    assert_eq!(flipped, 104);
}

#[test]
fn the_last_step_writes_the_sample_format_into_both_copies() {
    let steps = spike_steps().expect("the spike's steps are valid");
    let grid = &steps[4].grid;
    for bit in 0..FORMAT_BITS {
        let expected = Some(if (0b111_0111_1100_0100_u16 >> bit) & 1 == 1 {
            CellState::Dark
        } else {
            CellState::Light
        });
        assert_eq!(
            grid.get(format_copy_a(bit).0, format_copy_a(bit).1),
            expected,
            "copy A bit {bit}"
        );
        assert_eq!(
            grid.get(format_copy_b(bit).0, format_copy_b(bit).1),
            expected,
            "copy B bit {bit}"
        );
    }
}

#[test]
fn the_steps_are_the_same_every_time() {
    let first = spike_steps().expect("the spike's steps are valid");
    let second = spike_steps().expect("the spike's steps are valid");
    assert!(
        first
            .iter()
            .zip(&second)
            .all(|(a, b)| a.grid == b.grid && a.highlights == b.highlights)
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Walking the steps
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn next_walks_forward_from_unstarted_and_stops_at_the_last_step() {
    assert_eq!(spike::next(None, 5), Some(0));
    assert_eq!(spike::next(Some(0), 5), Some(1));
    assert_eq!(spike::next(Some(3), 5), Some(4));
    assert_eq!(spike::next(Some(4), 5), Some(4));
    assert_eq!(spike::next(None, 0), None);
}

#[test]
fn prev_walks_back_to_unstarted_and_stops_there() {
    assert_eq!(spike::prev(Some(4)), Some(3));
    assert_eq!(spike::prev(Some(1)), Some(0));
    assert_eq!(spike::prev(Some(0)), None);
    assert_eq!(spike::prev(None), None);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The text
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn text_is_wrapped_between_words_at_the_width() {
    assert_eq!(wrap("one two three four", 9), ["one two", "three", "four"]);
    assert_eq!(wrap("one two three four", 13), ["one two three", "four"]);
    assert_eq!(wrap("  spaced   out  ", 20), ["spaced out"]);
    assert!(wrap("", 10).is_empty());
}

#[test]
fn a_word_longer_than_the_width_is_kept_whole() {
    assert_eq!(wrap("a unbelievable b", 5), ["a", "unbelievable", "b"]);
}

#[test]
fn no_wrapped_line_is_wider_than_the_width() {
    for role in QrModuleKind::ALL {
        assert!(
            wrap(role.explanation(), RIGHT_COLUMN_CHARS)
                .iter()
                .all(|line| line.chars().count() <= RIGHT_COLUMN_CHARS)
        );
    }
}

#[test]
fn every_explanation_fits_the_room_for_it() {
    for role in QrModuleKind::ALL {
        let lines = wrap(role.explanation(), RIGHT_COLUMN_CHARS).len();
        assert!(lines <= EXPLANATION_LINES, "{} needs {lines} lines", role.label());
    }
    assert!(wrap(NO_ROLE_EXPLANATION, RIGHT_COLUMN_CHARS).len() <= EXPLANATION_LINES);
    assert!(wrap(KEY, RIGHT_COLUMN_CHARS).len() <= KEY_LINES);
}

#[test]
fn every_role_has_its_own_explanation_that_names_no_undefined_term() {
    let explanations: HashSet<_> = QrModuleKind::ALL.iter().map(|role| role.explanation()).collect();
    assert_eq!(explanations.len(), 7);
    // Each of these is explained where it is first used, so a reader needs no other page.
    assert!(
        QrModuleKind::Function(FunctionRole::Finder)
            .explanation()
            .contains("A scanner looks for them first")
    );
    assert!(
        QrModuleKind::Function(FunctionRole::Timing)
            .explanation()
            .contains("measure the grid")
    );
    assert!(
        QrModuleKind::Function(FunctionRole::Separator)
            .explanation()
            .contains("light modules")
    );
}

#[test]
fn every_step_caption_and_stats_line_fits_the_room_for_it() {
    let steps = spike_steps().expect("the spike's steps are valid");
    for step in &steps {
        let lines = wrap(&caption_text(step.label, &step.grid), LEFT_COLUMN_CHARS).len();
        assert!(lines <= CAPTION_LINES, "{} needs {lines} lines", step.label);
    }
    let unstarted = caption_text("Not started. Every module is empty.", &grid());
    assert!(wrap(&unstarted, LEFT_COLUMN_CHARS).len() <= CAPTION_LINES);
    let stats = stats_text("Step 5 of 5", 6, 12);
    assert!(wrap(&stats, LEFT_COLUMN_CHARS).len() <= STATS_LINES);
}

#[test]
fn each_role_has_a_button_action_and_none_has_its_own() {
    let actions: HashSet<_> = std::iter::once(None)
        .chain(QrModuleKind::ALL.map(Some))
        .map(role_action)
        .collect();
    assert_eq!(actions.len(), 8);
    assert_eq!(role_action(None), NO_ROLE);
    assert_eq!(role_action(Some(FINDER)), "role-finder");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Checking before assigning
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_module_can_be_checked_for_assigning_without_changing_anything() {
    let mut grid = reserved();
    let before = grid.clone();
    assert_eq!(grid.check_assignable(20, 20), Ok(20 * 21 + 20));
    assert_eq!(grid.check_assignable(0, 0), Err(GridError::Occupied { row: 0, col: 0 }));
    assert_eq!(grid.check_assignable(21, 0), Err(GridError::OutOfBounds { row: 21, col: 0 }));
    assert_eq!(grid, before);
    grid.assign(20, 20, true, DataRole::Data)
        .expect("an empty module can be assigned");
    assert_eq!(grid.check_assignable(20, 20), Err(GridError::Occupied { row: 20, col: 20 }));
}
