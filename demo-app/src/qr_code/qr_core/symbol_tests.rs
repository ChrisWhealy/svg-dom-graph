use super::{
    TOTAL_CODEWORDS, encode,
    format::format_word,
    mask::{MASKS, inverts},
    score::{DarkMatrix, Penalty, choose_mask, n1, n2, n3, n4, penalty},
    symbol::{SymbolError, build_symbol, make_candidate, place_codeword, rows_of},
    symbol_golden::SYMBOL_GOLDEN,
};
use crate::qr_code::{
    function::{FORMAT_BITS, format_copy_a, format_copy_b, placement_order, reserve_function_modules},
    grid::{BitGrid, CellState, DataRole, FunctionRole, GridError, QrModuleKind, SIZE},
};

fn reserved() -> BitGrid {
    let mut grid = BitGrid::new(SIZE, SIZE);
    reserve_function_modules(&mut grid).expect("the function modules fit an empty grid");
    grid
}

/// A square of `size` where every module is `value`, as rows of text.
fn flat(size: usize, value: char) -> Vec<String> {
    vec![value.to_string().repeat(size); size]
}

fn matrix(rows: &[String]) -> DarkMatrix {
    DarkMatrix::from_rows(&rows.iter().map(String::as_str).collect::<Vec<_>>())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The format information
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// The format bits for level L, for each mask in turn, as the standard's table of them gives them.
const FORMAT_TABLE: [u16; 8] = [
    0b111011111000100,
    0b111001011110011,
    0b111110110101010,
    0b111100010011101,
    0b110011000101111,
    0b110001100011000,
    0b110110001000001,
    0b110100101110110,
];

#[test]
fn the_format_word_of_each_mask_is_the_one_in_the_standard() {
    for mask in 0..8 {
        assert_eq!(format_word(mask), Some(FORMAT_TABLE[usize::from(mask)]), "mask {mask}");
    }
}

#[test]
fn a_format_word_is_15_bits_and_a_codeword_of_the_bch_code() {
    for mask in 0..8 {
        let word = u32::from(format_word(mask).expect("the mask is 0 to 7"));
        assert!(word < 1 << 15);
        // Without the XOR mask the word is a multiple of the generator, so dividing it by 0x537 leaves nothing.
        let mut remainder = word ^ 0x5412;
        for shift in (10..15).rev() {
            if remainder >> shift & 1 == 1 {
                remainder ^= 0x537 << (shift - 10);
            }
        }
        assert_eq!(remainder, 0, "mask {mask}");
    }
}

#[test]
fn the_format_words_differ_in_at_least_seven_bits() {
    for a in 0..8_u8 {
        for b in a + 1..8 {
            let (a_word, b_word) = (format_word(a).expect("0 to 7"), format_word(b).expect("0 to 7"));
            assert!((a_word ^ b_word).count_ones() >= 7, "{a} and {b}");
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The masks
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

/// The first six rows and columns of each mask, with `x` where the module is inverted. They are worked out from the
/// formulas in the design by a separate script, and pasted here.
const MASK_CORNERS: [[&str; 6]; 8] = [
    ["x.x.x.", ".x.x.x", "x.x.x.", ".x.x.x", "x.x.x.", ".x.x.x"],
    ["xxxxxx", "......", "xxxxxx", "......", "xxxxxx", "......"],
    ["x..x..", "x..x..", "x..x..", "x..x..", "x..x..", "x..x.."],
    ["x..x..", "..x..x", ".x..x.", "x..x..", "..x..x", ".x..x."],
    ["xxx...", "xxx...", "...xxx", "...xxx", "xxx...", "xxx..."],
    ["xxxxxx", "x.....", "x..x..", "x.x.x.", "x..x..", "x....."],
    ["xxxxxx", "xxx...", "xx.xx.", "x.x.x.", "x.xx.x", "x...xx"],
    ["x.x.x.", "...xxx", "x...xx", ".x.x.x", "xxx...", ".xxx.."],
];

#[test]
fn each_mask_inverts_the_modules_the_design_says() {
    for (mask, corner) in MASK_CORNERS.iter().enumerate() {
        for (row, text) in corner.iter().enumerate() {
            let got: String = (0..6)
                .map(|col| if inverts(mask as u8, row, col) { 'x' } else { '.' })
                .collect();
            assert_eq!(&got, text, "mask {mask}, row {row}");
        }
    }
}

#[test]
fn a_mask_that_does_not_exist_inverts_nothing() {
    assert!((0..SIZE).all(|row| (0..SIZE).all(|col| !inverts(8, row, col) && !inverts(255, row, col))));
}

#[test]
fn every_mask_inverts_about_half_of_a_symbol() {
    for mask in 0..MASKS as u8 {
        let count = (0..SIZE)
            .flat_map(|row| (0..SIZE).map(move |col| (row, col)))
            .filter(|&(r, c)| inverts(mask, r, c))
            .count();
        assert!((130..=300).contains(&count), "mask {mask} inverts {count}");
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The scores
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn n1_scores_three_for_five_in_a_row_and_one_more_for_each_past_five() {
    // Five in a row, in a square of 5, in each row and each column.
    assert_eq!(n1(&matrix(&flat(5, '1'))), 10 * 3);
    // Six: 3 + 1 for each of 12 lines.
    assert_eq!(n1(&matrix(&flat(6, '0'))), 12 * 4);
    // Four is not enough.
    assert_eq!(n1(&matrix(&flat(4, '1'))), 0);
    // One row of seven dark modules in an otherwise light square of 7. The row scores 3 + 2, and each of the other six
    // rows and all seven columns are runs of seven light modules or of six light and one dark.
    let mut rows = flat(7, '0');
    rows[3] = "1111111".to_owned();
    let alone = n1(&matrix(&rows));
    // Rows: the dark row scores 5, and six light rows of 7 score 5 each. Columns: six light modules then one dark in
    // the middle, so 3 light above and 3 below, which is too short, and score nothing.
    assert_eq!(alone, 5 + 6 * 5);
}

#[test]
fn n1_counts_a_run_that_ends_at_the_edge() {
    let mut rows = flat(6, '0');
    rows[0] = "011111".to_owned();
    // The run of five at the end of the first row scores 3, and so does the run of five light modules in the first
    // column, and the light run of 6 in the other five rows, 4 each. The columns are checked by the next assertion.
    let score = n1(&matrix(&rows));
    assert!(score > 0);
    assert_eq!(
        n1(&matrix(&["11111", "01010", "10101", "01010", "10101"].map(str::to_owned))),
        3
    );
}

#[test]
fn n2_scores_three_for_each_block_of_four_including_blocks_that_overlap() {
    // Every one of the 25 blocks of a flat 6 by 6 square.
    assert_eq!(n2(&matrix(&flat(6, '1'))), 25 * 3);
    // Two blocks of dark modules and two of light ones, one in each corner.
    assert_eq!(n2(&matrix(&["1100", "1100", "0011", "0011"].map(str::to_owned))), 3 * 4);
    // A chessboard has none.
    assert_eq!(n2(&matrix(&["1010", "0101", "1010", "0101"].map(str::to_owned))), 0);
    // A block of 2 by 3 holds two blocks that overlap, and so does the one of light modules under it.
    assert_eq!(n2(&matrix(&["1110", "1110", "0001", "0001"].map(str::to_owned))), 3 * 4);
}

/// An 11 by 11 square that is light, apart from `row`, which is the first row.
fn with_first_row(row: &str) -> DarkMatrix {
    let mut rows = flat(11, '0');
    rows[0] = row.to_owned();
    matrix(&rows)
}

#[test]
fn n3_scores_40_for_a_pattern_with_light_on_one_side_only_by_the_edge() {
    // The pattern is at the left edge, so the four light modules before it are the border's, and it is followed by a
    // dark module, so no light follows it. One pattern, found by one window.
    assert_eq!(n3(&with_first_row("10111010111")), 40);
}

#[test]
fn n3_scores_a_pattern_with_light_on_both_sides_twice() {
    // 1011101 with at least four light modules on each side is found by the window that has them before and by the one
    // that has them after, so it scores 80. segno scores it 40, once.
    assert_eq!(n3(&with_first_row("01011101000")), 80);
    assert_eq!(n3(&with_first_row("10111010000")), 80);
}

/// A square of `size` that is light, apart from its first row, which is `row`.
fn with_row(size: usize, row: &str) -> DarkMatrix {
    assert_eq!(row.len(), size);
    let mut rows = flat(size, '0');
    rows[0] = row.to_owned();
    matrix(&rows)
}

#[test]
fn n3_needs_the_whole_pattern_and_four_light_modules() {
    // The inner modules are wrong.
    assert_eq!(n3(&with_first_row("10110010000")), 0);
    // Away from the edge, three light modules on each side are not four. The dark modules at each end stop the border
    // from being what makes up the four.
    assert_eq!(n3(&with_row(15, "100010111010001")), 0);
    // With four on the left and a dark module after the pattern, the pattern is found once, by the window that has them
    // before it.
    assert_eq!(n3(&with_row(15, "100001011101111")), 40);
    // With four light modules after it and a dark module before it, it is found once, by the other window.
    assert_eq!(n3(&with_row(15, "111101110100001")), 40);
    // A light square has no pattern anywhere.
    assert_eq!(n3(&matrix(&flat(11, '0'))), 0);
}

#[test]
fn the_border_is_light_so_a_pattern_at_the_edge_needs_no_real_light_modules() {
    // Only one light module lies before the pattern, but the border adds four, and that is enough.
    assert_eq!(n3(&with_first_row("01011101111")), 40);
}

#[test]
fn n3_finds_patterns_in_columns_as_it_does_in_rows() {
    let mut rows = flat(11, '0');
    for (row, bit) in "10111010000".chars().enumerate() {
        rows[row].replace_range(0..1, &bit.to_string());
    }
    assert_eq!(n3(&matrix(&rows)), 80);
}

#[test]
fn n3_counts_every_start_position_when_patterns_overlap() {
    // Two patterns with four light modules between them. Each has light on both sides, the left one by the border, and
    // the right one by the border too. So each is found by both windows, which overlap each other, and the four score
    // 160.
    assert_eq!(n3(&with_row(18, "101110100001011101")), 160);
}

#[test]
fn n4_scores_ten_for_each_whole_five_percent_from_half() {
    // 121 modules. 60 or 61 dark is within 5% of half, since the share is 49.6% or 50.4%.
    let dark = |count: usize| {
        let mut rows = flat(11, '0');
        let mut left = count;
        for row in &mut rows {
            let take = left.min(11);
            *row = "1".repeat(take) + &"0".repeat(11 - take);
            left -= take;
        }
        n4(&matrix(&rows))
    };
    assert_eq!(dark(60), 0);
    assert_eq!(dark(61), 0);
    // 5% of 121 is 6.05, so a share of 55% or more is 67 dark, and 45% or less is 54.
    assert_eq!(dark(66), 0);
    assert_eq!(dark(67), 10);
    assert_eq!(dark(55), 0);
    assert_eq!(dark(54), 10);
    assert_eq!(dark(0), 100);
    assert_eq!(dark(121), 100);
}

#[test]
fn the_total_is_the_four_scores_added() {
    let scores = Penalty { n1: 1, n2: 2, n3: 40, n4: 10 };
    assert_eq!(scores.total(), 53);
    let m = matrix(&flat(6, '1'));
    let all = penalty(&m);
    assert_eq!((all.n1, all.n2, all.n3, all.n4), (n1(&m), n2(&m), n3(&m), n4(&m)));
}

#[test]
fn the_least_total_wins_and_the_lowest_mask_wins_a_tie() {
    assert_eq!(choose_mask(&[5, 3, 9, 3, 4, 8, 7, 6]), Some(1));
    assert_eq!(choose_mask(&[7, 7, 7, 7, 7, 7, 7, 7]), Some(0));
    assert_eq!(choose_mask(&[9, 9, 2, 9, 9, 2, 9, 9]), Some(2));
    assert_eq!(choose_mask(&[9, 9, 9, 9, 9, 9, 9, 1]), Some(7));
    assert_eq!(choose_mask(&[]), None);
}

#[test]
fn a_grid_with_a_module_without_a_colour_cannot_be_scored() {
    let grid = reserved();
    let error = DarkMatrix::from_grid(&grid).expect_err("the format modules have no colour");
    assert!(matches!(error, SymbolError::Uncoloured { .. }), "{error:?}");
    let wide = BitGrid::new(3, 2);
    assert!(matches!(
        DarkMatrix::from_grid(&wide),
        Err(SymbolError::Internal { stage: "scoring", .. })
    ));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Placing the codewords
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn a_codeword_goes_into_its_eight_modules_highest_bit_first() {
    let mut grid = reserved();
    let order = placement_order(&grid);
    place_codeword(&mut grid, &order, 0, 0b1011_0001).expect("the first codeword fits");
    let colours: Vec<_> = order[..8].iter().map(|&(row, col)| grid.get(row, col)).collect();
    let (dark, light) = (Some(CellState::Dark), Some(CellState::Light));
    assert_eq!(colours, [dark, light, dark, dark, light, light, light, dark]);
    assert!(
        order[..8]
            .iter()
            .all(|&(row, col)| grid.role(row, col) == Some(QrModuleKind::Payload(DataRole::Data)))
    );
    assert_eq!(grid.get(order[8].0, order[8].1), Some(CellState::Unassigned));
}

#[test]
fn the_last_seven_codewords_are_error_correction() {
    let mut grid = reserved();
    let order = placement_order(&grid);
    place_codeword(&mut grid, &order, 18, 0).expect("it fits");
    place_codeword(&mut grid, &order, 19, 0).expect("it fits");
    let role = |index: usize| grid.role(order[index * 8].0, order[index * 8].1);
    assert_eq!(role(18), Some(QrModuleKind::Payload(DataRole::Data)));
    assert_eq!(role(19), Some(QrModuleKind::Payload(DataRole::ErrorCorrection)));
}

#[test]
fn a_codeword_cannot_be_placed_twice_or_past_the_end() {
    let mut grid = reserved();
    let order = placement_order(&grid);
    place_codeword(&mut grid, &order, 3, 0xFF).expect("it fits");
    let before = grid.clone();
    assert!(matches!(
        place_codeword(&mut grid, &order, 3, 0x00),
        Err(SymbolError::Grid(GridError::Occupied { .. }))
    ));
    assert!(matches!(
        place_codeword(&mut grid, &order, 26, 0),
        Err(SymbolError::Internal { stage: "placement", .. })
    ));
    assert_eq!(grid.get(order[24].0, order[24].1), before.get(order[24].0, order[24].1));
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The symbol
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

fn symbol_of(input: &[u8]) -> super::symbol::Symbol {
    build_symbol(&encode(input).expect("the input fits")).expect("a symbol is made")
}

#[test]
fn a_symbol_has_every_stage_and_eight_candidates() {
    let symbol = symbol_of(b"Hello");
    assert_eq!(symbol.placement.len(), TOTAL_CODEWORDS * 8);
    assert_eq!(symbol.candidates.len(), 8);
    assert!(
        symbol
            .candidates
            .iter()
            .enumerate()
            .all(|(index, candidate)| usize::from(candidate.mask) == index)
    );
    assert!(symbol.chosen < 8);
    assert_eq!(symbol.finished().mask as usize, symbol.chosen);
    assert_eq!(symbol.functions.count_state(CellState::Unassigned), 441 - 203);
    assert_eq!(symbol.placed.count_state(CellState::Unassigned), 30);
}

#[test]
fn every_candidate_is_wholly_coloured_and_keeps_the_rules_of_the_grid() {
    for candidate in &symbol_of(b"Hello").candidates {
        assert_eq!(candidate.grid.count_state(CellState::Unassigned), 0);
        assert_eq!(candidate.grid.check_invariants(), Ok(()));
    }
}

#[test]
fn the_winner_has_the_least_total_and_the_lowest_mask_among_equals() {
    for input in [&b""[..], b"A", b"Hello, World!", b"abcdefghijklmnopq"] {
        let symbol = symbol_of(input);
        let totals: Vec<u32> = symbol.candidates.iter().map(|c| c.penalty.total()).collect();
        let least = *totals.iter().min().expect("there are totals");
        assert_eq!(totals[symbol.chosen], least);
        assert!(
            totals[..symbol.chosen].iter().all(|&total| total > least),
            "a lower mask has the same total"
        );
    }
}

#[test]
fn the_mask_changes_only_data_and_error_correction_modules() {
    let symbol = symbol_of(b"Hello");
    for candidate in &symbol.candidates {
        for (row, col) in symbol.placed.cells() {
            let is_payload = matches!(symbol.placed.role(row, col), Some(QrModuleKind::Payload(_)));
            let changed = symbol.placed.get(row, col) != candidate.grid.get(row, col);
            if is_payload {
                assert_eq!(
                    changed,
                    inverts(candidate.mask, row, col),
                    "mask {} at ({row}, {col})",
                    candidate.mask
                );
            } else if symbol.placed.role(row, col) != Some(QrModuleKind::Function(FunctionRole::Format)) {
                assert!(!changed, "a function module changed at ({row}, {col})");
            }
        }
    }
}

#[test]
fn both_copies_of_the_format_information_are_the_masks_own_word() {
    let symbol = symbol_of(b"Hello, World!");
    for candidate in &symbol.candidates {
        for (copy, place) in [("A", format_copy_a as fn(usize) -> (usize, usize)), ("B", format_copy_b)] {
            let word: u16 = (0..FORMAT_BITS)
                .map(|bit| {
                    let (row, col) = place(bit);
                    u16::from(candidate.grid.get(row, col) == Some(CellState::Dark)) << bit
                })
                .sum();
            assert_eq!(
                Some(word),
                format_word(candidate.mask),
                "copy {copy} of mask {}",
                candidate.mask
            );
            assert_eq!(Some(candidate.format_bits), format_word(candidate.mask));
        }
    }
}

#[test]
fn the_finished_symbol_reads_back_to_the_codewords_it_was_made_from() {
    for input in [&b""[..], b"A", b"Hello, World!", b"abcdefghijklmnopq", &[0xFF, 0x00, 0x7F]] {
        let encoded = encode(input).expect("the input fits");
        let symbol = build_symbol(&encoded).expect("a symbol is made");
        let finished = &symbol.finished().grid;
        // Undo the mask, then read the modules in the order they were placed.
        let bits: Vec<bool> = symbol
            .placement
            .iter()
            .map(|&(row, col)| {
                (finished.get(row, col) == Some(CellState::Dark)) != inverts(symbol.finished().mask, row, col)
            })
            .collect();
        let words: Vec<u8> = bits
            .chunks(8)
            .map(|byte| byte.iter().fold(0, |word, &bit| word << 1 | u8::from(bit)))
            .collect();
        assert_eq!(words, encoded.codewords(), "{input:?}");
    }
}

#[test]
fn the_function_patterns_are_the_same_in_every_candidate() {
    let symbol = symbol_of(b"Hello");
    let rows: Vec<Vec<String>> = symbol.candidates.iter().map(|c| rows_of(&c.grid)).collect();
    // The finder patterns, the separators, the timing patterns and the dark module are the same in all eight.
    for (row, col) in [
        (0, 0),
        (6, 6),
        (7, 7),
        (0, 14),
        (14, 0),
        (6, 8),
        (6, 10),
        (8, 6),
        (10, 6),
        (13, 8),
    ] {
        let first = rows[0][row].as_bytes()[col];
        assert!(
            rows.iter().all(|candidate| candidate[row].as_bytes()[col] == first),
            "({row}, {col})"
        );
    }
    assert_eq!(rows[0][13].as_bytes()[8], b'1');
}

#[test]
fn rows_of_shows_the_three_states() {
    let mut grid = BitGrid::new(3, 1);
    grid.assign(0, 0, true, DataRole::Data)
        .expect("an empty module can be assigned");
    grid.assign(0, 1, false, DataRole::Data)
        .expect("an empty module can be assigned");
    assert_eq!(rows_of(&grid), [".".repeat(0) + "10."]);
}

#[test]
fn every_length_makes_a_symbol_and_no_input_fails() {
    for len in 0..=17 {
        let input: Vec<u8> = (0..len).map(|i| (i * 53 + len * 11) as u8).collect();
        let symbol = symbol_of(&input);
        assert_eq!(symbol.rows().len(), 21);
        assert!(symbol.rows().iter().all(|row| row.len() == 21 && !row.contains('.')));
    }
}

#[test]
fn the_same_input_makes_the_same_symbol() {
    assert_eq!(symbol_of(b"Hello"), symbol_of(b"Hello"));
    assert_ne!(symbol_of(b"Hello").rows(), symbol_of(b"Hellp").rows());
}

#[test]
fn a_candidate_cannot_be_made_from_a_grid_that_is_not_placed() {
    let grid = reserved();
    let order = placement_order(&grid);
    // The data modules have no colour or role, so there is nothing for the mask to invert.
    let error = make_candidate(&grid, &order, 0).expect_err("nothing is placed");
    assert!(matches!(error, SymbolError::Grid(GridError::NotFlippable { .. })), "{error:?}");
}

#[test]
fn the_errors_say_what_is_wrong_in_words() {
    let uncoloured = SymbolError::Uncoloured { row: 3, col: 4 }.to_string();
    assert!(
        uncoloured.contains("(3, 4)") && uncoloured.contains("no colour"),
        "{uncoloured}"
    );
    let internal = SymbolError::Internal {
        stage: "placement",
        detail: "oops".to_owned(),
    }
    .to_string();
    assert!(internal.contains("placement") && internal.contains("oops"), "{internal}");
    let grid = SymbolError::Grid(GridError::OutOfBounds { row: 1, col: 2 }).to_string();
    assert!(grid.contains("grid") && grid.contains("(1, 2)"), "{grid}");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The golden vectors
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn every_candidate_matrix_is_the_one_python_qrcode_makes_with_that_mask() {
    assert!(SYMBOL_GOLDEN.len() >= 8);
    for golden in SYMBOL_GOLDEN {
        let symbol = symbol_of(golden.input);
        for (candidate, expected) in symbol.candidates.iter().zip(&golden.candidates) {
            assert_eq!(candidate.mask, expected.mask);
            assert_eq!(
                candidate.format_bits, expected.format_bits,
                "format bits of {:?}, mask {}",
                golden.input, expected.mask
            );
            assert_eq!(
                rows_of(&candidate.grid),
                expected.rows,
                "matrix of {:?}, mask {}",
                golden.input,
                expected.mask
            );
        }
    }
}

#[test]
fn n1_n2_and_n4_are_what_python_qrcode_and_segno_both_give() {
    for golden in SYMBOL_GOLDEN {
        let symbol = symbol_of(golden.input);
        for (candidate, expected) in symbol.candidates.iter().zip(&golden.candidates) {
            let at = format!("{:?}, mask {}", golden.input, expected.mask);
            assert_eq!(candidate.penalty.n1, expected.n1, "N1 of {at}");
            assert_eq!(candidate.penalty.n2, expected.n2, "N2 of {at}");
            assert_eq!(candidate.penalty.n4, expected.n4, "N4 of {at}");
        }
    }
}

#[test]
fn n3_is_the_designs_own_and_matches_the_plain_python_reading_of_it() {
    for golden in SYMBOL_GOLDEN {
        let symbol = symbol_of(golden.input);
        for (candidate, expected) in symbol.candidates.iter().zip(&golden.candidates) {
            assert_eq!(
                candidate.penalty.n3, expected.n3,
                "N3 of {:?}, mask {}",
                golden.input, expected.mask
            );
        }
    }
}

#[test]
fn the_mask_is_the_one_the_designs_scores_choose() {
    for golden in SYMBOL_GOLDEN {
        assert_eq!(symbol_of(golden.input).chosen, usize::from(golden.mask), "{:?}", golden.input);
    }
}

#[test]
fn the_finished_matrix_is_the_chosen_candidate_and_is_compared_with_the_library_only_when_they_chose_alike() {
    let mut compared = 0;
    for golden in SYMBOL_GOLDEN {
        let symbol = symbol_of(golden.input);
        // Whichever mask was chosen, the finished symbol is that candidate, which has been compared above.
        assert_eq!(symbol.rows(), golden.candidates[usize::from(golden.mask)].rows);
        if golden.mask == golden.mask_qrcode_chosen {
            // python-qrcode chose this mask too, so its own finished symbol is this one.
            compared += 1;
        }
    }
    assert!(compared >= 1, "no input has a finished matrix that python-qrcode also chose");
}

#[test]
fn the_conventions_differ_and_the_difference_is_on_record() {
    let n3_differs_from_qrcode = SYMBOL_GOLDEN.iter().flat_map(|g| &g.candidates).any(|c| c.n3 != c.n3_qrcode);
    let n3_differs_from_segno = SYMBOL_GOLDEN.iter().flat_map(|g| &g.candidates).any(|c| c.n3 != c.n3_segno);
    assert!(n3_differs_from_qrcode, "the design's N3 never differs from python-qrcode's");
    assert!(n3_differs_from_segno, "the design's N3 never differs from segno's");
    // The design counts every window, with a border, so it is never less than python-qrcode's, which has no border and
    // never counts a window that the design does not.
    assert!(SYMBOL_GOLDEN.iter().flat_map(|g| &g.candidates).all(|c| c.n3 >= c.n3_qrcode));
    // Where the choices differ, each library's own choice is on record and is a valid mask.
    let differing: Vec<_> = SYMBOL_GOLDEN
        .iter()
        .filter(|g| g.mask != g.mask_qrcode_chosen || g.mask != g.mask_segno || g.mask != g.mask_qrcode)
        .collect();
    assert!(
        !differing.is_empty(),
        "no input is a case where the conventions choose differently"
    );
    assert!(
        differing
            .iter()
            .all(|g| g.mask_qrcode < 8 && g.mask_qrcode_chosen < 8 && g.mask_segno < 8)
    );
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Mistakes by a caller
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn only_masks_0_to_7_have_a_format_word() {
    assert!((0..8).all(|mask| format_word(mask).is_some()));
    assert_eq!(format_word(8), None);
    assert_eq!(format_word(15), None);
    assert_eq!(format_word(255), None);
}

#[test]
fn a_mask_that_does_not_exist_is_refused_before_anything_is_done() {
    let symbol = symbol_of(b"Hello");
    let (placed, order) = (symbol.placed.clone(), symbol.placement.clone());
    for mask in [8, 9, 16, 255] {
        assert_eq!(make_candidate(&placed, &order, mask), Err(SymbolError::InvalidMask { mask }));
    }
    // The grid it was given is as it was, and a valid mask still works on it.
    assert_eq!(placed, symbol.placed);
    assert_eq!(make_candidate(&placed, &order, 3), Ok(symbol.candidates[3].clone()));
    assert!(SymbolError::InvalidMask { mask: 8 }.to_string().contains("0 to 7"));
}

#[test]
fn a_refused_codeword_leaves_no_bit_of_it_placed() {
    let mut grid = reserved();
    let mut order = placement_order(&grid);
    // Put a function module, which cannot be assigned, as the sixth of the eight modules of codeword 2.
    order[2 * 8 + 5] = (0, 0);
    let before = grid.clone();
    let error = place_codeword(&mut grid, &order, 2, 0xFF).expect_err("a function module cannot be assigned");
    assert_eq!(error, SymbolError::Grid(GridError::Occupied { row: 0, col: 0 }));
    assert_eq!(grid, before, "the five modules before the bad one were left written");
}

#[test]
fn a_codeword_with_a_module_outside_the_grid_or_listed_twice_is_refused_whole() {
    let mut grid = reserved();
    let order = placement_order(&grid);
    let before = grid.clone();

    let mut outside = order.clone();
    outside[8 + 3] = (SIZE, 0);
    assert_eq!(
        place_codeword(&mut grid, &outside, 1, 0xAA),
        Err(SymbolError::Grid(GridError::OutOfBounds { row: SIZE, col: 0 }))
    );
    assert_eq!(grid, before);

    let mut twice = order.clone();
    twice[8 + 6] = twice[8 + 1];
    let repeated = twice[8 + 1];
    assert_eq!(
        place_codeword(&mut grid, &twice, 1, 0xAA),
        Err(SymbolError::Grid(GridError::Occupied {
            row: repeated.0,
            col: repeated.1
        }))
    );
    assert_eq!(grid, before);
}

#[test]
fn a_codeword_that_is_accepted_is_placed_whole() {
    let mut grid = reserved();
    let order = placement_order(&grid);
    place_codeword(&mut grid, &order, 5, 0b0110_1001).expect("it fits");
    let placed = order[40..48]
        .iter()
        .filter(|&&(row, col)| grid.get(row, col) != Some(CellState::Unassigned))
        .count();
    assert_eq!(placed, 8);
}
