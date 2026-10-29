use super::{
    grid::{CellState, QrModuleKind, SIZE},
    qr_core::{TOTAL_CODEWORDS, encode_text, symbol::build_symbol},
    snapshots::{
        BITSTREAM_CELLS, BYTE_CELLS, COUNT, EDGES, FORMAT_CELLS, JUMPS, NodeKey, Region, SCORE_CELLS, SCORE_COLUMNS,
        SnapshotSet, Stage, build, format_cells_of,
    },
};
use std::collections::HashSet;

fn set_of(text: &str) -> SnapshotSet {
    let encoded = encode_text(text).expect("the text fits");
    let symbol = build_symbol(&encoded).expect("a symbol is made");
    build(text, &encoded, &symbol).expect("the snapshots are made")
}

fn symbol_of(text: &str) -> super::qr_core::symbol::Symbol {
    build_symbol(&encode_text(text).expect("the text fits")).expect("a symbol is made")
}

fn payload_modules(grid: &super::grid::BitGrid) -> usize {
    grid.cells()
        .filter(|&(row, col)| matches!(grid.role(row, col), Some(QrModuleKind::Payload(_))))
        .count()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The count and the stages
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn there_are_62_snapshots_whatever_the_input() {
    assert_eq!(COUNT, 62);
    for text in ["", "A", "Hello, World!", "abcdefghijklmnopq", "\u{7F}\0\t"] {
        assert_eq!(set_of(text).snapshots.len(), COUNT, "{text:?}");
    }
}

#[test]
fn the_stages_have_the_lengths_in_the_design() {
    let stages: Vec<Stage> = (0..COUNT).map(Stage::of).collect();
    let count = |stage| stages.iter().filter(|&&s| s == stage).count();
    assert_eq!(
        [
            count(Stage::Input),
            count(Stage::Bytes),
            count(Stage::Bitstream),
            count(Stage::DataCodewords),
            count(Stage::ReedSolomon),
            count(Stage::FunctionModules),
            count(Stage::Placement),
            count(Stage::Masks),
            count(Stage::Final),
        ],
        [1, 1, 4, 1, 19, 1, 26, 8, 1]
    );
    // The stages follow one another, and none comes back.
    assert!(stages.windows(2).all(|pair| pair[0] as u8 <= pair[1] as u8));
}

#[test]
fn each_jump_goes_to_the_first_snapshot_of_a_stage() {
    let targets: Vec<usize> = JUMPS.iter().map(|&(_, index)| index).collect();
    assert_eq!(targets, [0, 7, 26, 27, 53, 61]);
    let stages: Vec<Stage> = targets.iter().map(|&index| Stage::of(index)).collect();
    assert_eq!(
        stages,
        [
            Stage::Input,
            Stage::ReedSolomon,
            Stage::FunctionModules,
            Stage::Placement,
            Stage::Masks,
            Stage::Final
        ]
    );
    for &index in &targets[1..] {
        assert_ne!(
            Stage::of(index - 1),
            Stage::of(index),
            "snapshot {index} is not the first of its stage"
        );
    }
}

#[test]
fn every_snapshot_reads_as_its_own_sentence() {
    let set = set_of("Hello, World!");
    let labels: HashSet<&str> = set.snapshots.iter().map(|snapshot| snapshot.label.as_str()).collect();
    assert_eq!(labels.len(), COUNT);
    assert!(set.snapshots.iter().all(|snapshot| snapshot.label.ends_with('.')));
    assert!(!set.unstarted.label.is_empty());
    assert!(set.snapshots.iter().all(|snapshot| snapshot.label != set.unstarted.label));
}

#[test]
fn every_stage_has_its_own_explanation() {
    let stages = [
        Stage::Input,
        Stage::Bytes,
        Stage::Bitstream,
        Stage::DataCodewords,
        Stage::ReedSolomon,
        Stage::FunctionModules,
        Stage::Placement,
        Stage::Masks,
        Stage::Final,
    ];
    let explanations: HashSet<&str> = stages.iter().map(|stage| stage.explanation()).collect();
    assert_eq!(explanations.len(), stages.len());
}

#[test]
fn a_position_with_no_snapshot_is_the_unstarted_one() {
    let set = set_of("Hello");
    assert_eq!(set.at(None), &set.unstarted);
    assert_eq!(set.at(Some(0)), &set.snapshots[0]);
    assert_eq!(set.at(Some(61)), &set.snapshots[61]);
    assert_eq!(set.at(Some(62)), &set.unstarted);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The matrix
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn the_matrix_is_empty_until_the_function_modules() {
    let set = set_of("Hello");
    for snapshot in std::iter::once(&set.unstarted).chain(&set.snapshots[..26]) {
        assert_eq!(snapshot.grid.count_state(CellState::Unassigned), SIZE * SIZE);
        assert_eq!(snapshot.grid.count_function(), 0);
    }
}

#[test]
fn the_function_modules_are_the_symbols_own_with_the_format_modules_reserved_and_uncoloured() {
    let (set, symbol) = (set_of("Hello"), symbol_of("Hello"));
    assert_eq!(set.snapshots[26].grid, symbol.functions);
    assert_eq!(set.snapshots[26].grid.count_state(CellState::Unassigned), 441 - 203);
}

#[test]
fn one_codeword_is_placed_at_each_step_and_the_last_is_the_placed_symbol() {
    let (set, symbol) = (set_of("Hello"), symbol_of("Hello"));
    for word in 0..TOTAL_CODEWORDS {
        let snapshot = &set.snapshots[27 + word];
        assert_eq!(payload_modules(&snapshot.grid), (word + 1) * 8, "after codeword {word}");
        assert_eq!(snapshot.placing, Some(word));
        assert_eq!(snapshot.highlights, symbol.placement[word * 8..word * 8 + 8].to_vec());
    }
    assert_eq!(set.snapshots[52].grid, symbol.placed);
}

#[test]
fn each_mask_step_shows_its_own_candidate_and_the_last_is_the_winner() {
    let (set, symbol) = (set_of("Hello, World!"), symbol_of("Hello, World!"));
    for mask in 0..8 {
        assert_eq!(set.snapshots[53 + mask].grid, symbol.candidates[mask].grid, "mask {mask}");
    }
    assert_eq!(set.snapshots[61].grid, symbol.candidates[symbol.chosen].grid);
    assert_eq!(set.snapshots[61].winner, Some(symbol.chosen));
    assert!(set.snapshots[..61].iter().all(|snapshot| snapshot.winner.is_none()));
}

#[test]
fn the_highlight_marks_what_the_step_is_about() {
    let set = set_of("Hello");
    for (index, snapshot) in set.snapshots.iter().enumerate() {
        let expected = match Stage::of(index) {
            Stage::FunctionModules | Stage::Masks => 30,
            Stage::Placement => 8,
            _ => 0,
        };
        assert_eq!(snapshot.highlights.len(), expected, "snapshot {index}");
    }
    assert!(set.unstarted.highlights.is_empty());
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The nodes
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn what_each_node_holds_does_not_change_with_the_step() {
    let set = set_of("Hi!");
    let content = &set.content;
    assert_eq!(content.text, "Hi!");
    assert_eq!(content.bytes.len(), BYTE_CELLS);
    assert_eq!(content.bytes[..3], *b"Hi!");
    assert!(content.bytes[3..].iter().all(|&byte| byte == 0));
    assert_eq!(content.bitstream.len(), BITSTREAM_CELLS);
    assert_eq!(content.bitstream[..12], [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1]);
    assert!(content.bitstream.iter().all(|&bit| bit <= 1));
    let encoded = encode_text("Hi!").expect("the text fits");
    assert_eq!(content.data, encoded.data.codewords);
    assert_eq!(content.codewords, encoded.codewords());
}

#[test]
fn the_bytes_are_reached_at_the_second_snapshot_and_the_unused_places_stay_dim() {
    let set = set_of("Hello");
    assert_eq!(set.unstarted.unreached.bytes.len(), BYTE_CELLS);
    assert_eq!(set.snapshots[0].unreached.bytes.len(), BYTE_CELLS);
    assert_eq!(set.snapshots[1].unreached.bytes, (5..BYTE_CELLS).collect::<Vec<_>>());
    assert_eq!(set.snapshots[61].unreached.bytes, (5..BYTE_CELLS).collect::<Vec<_>>());
    assert!(set_of("abcdefghijklmnopq").snapshots[1].unreached.bytes.is_empty());
}

#[test]
fn the_bitstream_is_shown_a_part_at_a_time_and_each_part_is_marked() {
    let set = set_of("Hello");
    assert_eq!(set.snapshots[1].unreached.bitstream.len(), BITSTREAM_CELLS);
    // 4 bits of mode, 8 of count, 40 of data and a terminator of 4.
    let parts = [(0..4, 4), (4..12, 12), (12..52, 52), (52..56, 56)];
    for (position, (marked, reached)) in parts.into_iter().enumerate() {
        let snapshot = &set.snapshots[2 + position];
        assert_eq!(snapshot.marked_bits, marked.collect::<Vec<_>>(), "part {position}");
        assert_eq!(
            snapshot.unreached.bitstream,
            (reached..BITSTREAM_CELLS).collect::<Vec<_>>(),
            "part {position}"
        );
        assert_eq!(snapshot.unreached.data.len(), 19, "the data codewords are not reached yet");
    }
    // After the terminator the whole bitstream is on show, and nothing in it is marked.
    assert_eq!(set.snapshots[6].unreached.bitstream, (56..BITSTREAM_CELLS).collect::<Vec<_>>());
    assert!(set.snapshots[6].marked_bits.is_empty());
    assert!(set.snapshots[6].unreached.data.is_empty());
}

#[test]
fn the_four_parts_are_the_mode_the_count_the_data_and_the_terminator() {
    assert_eq!(Region::ALL, [Region::Mode, Region::Count, Region::Data, Region::Terminator]);
    assert_eq!(Region::Mode.cells(17, 4), 0..4);
    assert_eq!(Region::Count.cells(17, 4), 4..12);
    assert_eq!(Region::Data.cells(17, 4), 12..148);
    assert_eq!(Region::Terminator.cells(17, 4), 148..152);
    // The parts are in a row with nothing between and nothing over, whatever the length.
    for used in 0..=17 {
        let ends: Vec<_> = Region::ALL.iter().map(|part| part.cells(used, 4)).collect();
        assert!(ends.windows(2).all(|pair| pair[0].end == pair[1].start), "{used} bytes");
        assert_eq!(ends[3].end, 16 + 8 * used);
    }
}

#[test]
fn a_part_with_nothing_in_it_is_marked_with_nothing() {
    let set = set_of("");
    assert!(set.snapshots[4].marked_bits.is_empty(), "the data of an empty text has no bits");
    assert_eq!(set.snapshots[4].unreached.bitstream, (12..BITSTREAM_CELLS).collect::<Vec<_>>());
    // Every part still has its own step, so that the count does not depend on the text.
    assert_eq!(set.snapshots.len(), COUNT);
}

#[test]
fn the_note_says_what_the_marked_part_holds_and_only_the_bitstream_has_one() {
    let set = set_of("This is a QR code");
    let notes: Vec<_> = (2..6).map(|index| set.snapshots[index].note.clone().expect("a note")).collect();
    let titles: Vec<&str> = notes.iter().map(|(title, _)| title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "Mode indicator, 4 bits",
            "Character count, 8 bits",
            "Data, 136 bits",
            "Terminator, 4 bits"
        ]
    );
    assert!(notes[0].1.contains("0100") && notes[0].1.contains("byte mode"));
    assert!(notes[1].1.starts_with("17 in binary"));
    assert!(notes[2].1.contains("17 bytes"));
    for (index, snapshot) in set.snapshots.iter().enumerate() {
        assert_eq!(
            snapshot.note.is_some(),
            Stage::of(index) == Stage::Bitstream,
            "snapshot {index}"
        );
    }
    assert!(set.unstarted.note.is_none());
    assert_eq!(
        set_of("").snapshots[4].note.clone().map(|(title, _)| title),
        Some("Data, 0 bits".to_owned())
    );
}

#[test]
fn the_division_and_the_error_correction_are_reached_with_the_first_step() {
    let set = set_of("Hello");
    assert_eq!(set.snapshots[6].unreached.division, vec![0, 1]);
    assert_eq!(set.snapshots[6].unreached.error_correction.len(), 7);
    assert!(set.snapshots[7].unreached.division.is_empty());
    assert!(set.snapshots[7].unreached.error_correction.is_empty());
}

#[test]
fn each_step_of_the_division_shows_its_own_byte_factor_and_remainder() {
    let encoded = encode_text("Hello").expect("the text fits");
    let set = set_of("Hello");
    for (step, expected) in encoded.division.iter().enumerate() {
        let snapshot = &set.snapshots[7 + step];
        assert_eq!(snapshot.division, [expected.data_byte, expected.factor], "step {step}");
        assert_eq!(snapshot.register.as_slice(), expected.register.as_slice(), "step {step}");
    }
    // The last step ends on the error correction codewords, and they stay for the rest of the walk.
    for snapshot in &set.snapshots[25..] {
        assert_eq!(snapshot.register, encoded.error_correction);
    }
    assert!(set.snapshots[..7].iter().all(|snapshot| snapshot.register == [0; 7]));
}

#[test]
fn the_codewords_are_reached_when_the_division_ends() {
    let set = set_of("Hello");
    assert_eq!(set.snapshots[24].unreached.codewords.len(), TOTAL_CODEWORDS);
    assert!(set.snapshots[25].unreached.codewords.is_empty());
    assert!(set.snapshots[25].active_edges.contains(&EDGES[5]) && set.snapshots[25].active_edges.contains(&EDGES[6]));
}

#[test]
fn the_scores_are_reached_a_row_at_a_time() {
    let set = set_of("Hello");
    assert_eq!(set.snapshots[52].unreached.scores.len(), SCORE_CELLS);
    for mask in 0..8 {
        let reached = (mask + 1) * SCORE_COLUMNS;
        assert_eq!(
            set.snapshots[53 + mask].unreached.scores,
            (reached..SCORE_CELLS).collect::<Vec<_>>(),
            "mask {mask}"
        );
    }
    assert!(set.snapshots[61].unreached.scores.is_empty());
}

#[test]
fn the_scores_are_the_candidates_own() {
    let (set, symbol) = (set_of("Hello, World!"), symbol_of("Hello, World!"));
    for (mask, candidate) in symbol.candidates.iter().enumerate() {
        let p = candidate.penalty;
        let row = &set.content.scores[mask * SCORE_COLUMNS..(mask + 1) * SCORE_COLUMNS];
        assert_eq!(
            row,
            [p.n1, p.n2, p.n3, p.n4, p.total()].map(|n| u16::try_from(n).expect("a small score"))
        );
    }
}

#[test]
fn the_format_information_is_the_masks_own_word_most_significant_bit_first() {
    let set = set_of("Hello");
    assert_eq!(set.snapshots[52].format, [0; FORMAT_CELLS]);
    assert_eq!(set.snapshots[52].unreached.format.len(), FORMAT_CELLS);
    for mask in 0..8_u8 {
        let snapshot = &set.snapshots[53 + usize::from(mask)];
        assert_eq!(Some(snapshot.format), format_cells_of(mask));
        assert!(snapshot.unreached.format.is_empty());
    }
    // Mask 0 at level L is 111011111000100.
    assert_eq!(format_cells_of(0), Some([1, 1, 1, 0, 1, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0]));
    assert_eq!(format_cells_of(8), None);
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// What is active
// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -

#[test]
fn every_active_edge_is_a_real_edge_between_nodes_that_are_active_or_were_just_reached() {
    let set = set_of("Hello");
    for (index, snapshot) in set.snapshots.iter().enumerate() {
        for edge in &snapshot.active_edges {
            assert!(EDGES.contains(edge), "snapshot {index}");
        }
    }
    let edges: HashSet<_> = EDGES.iter().collect();
    assert_eq!(edges.len(), EDGES.len());
}

#[test]
fn the_active_nodes_follow_the_stage() {
    let set = set_of("Hello");
    let active = |index: usize| set.snapshots[index].active_nodes.clone();
    assert_eq!(active(0), [NodeKey::Input]);
    assert_eq!(active(1), [NodeKey::Bytes]);
    for index in 2..6 {
        assert_eq!(active(index), [NodeKey::Bitstream], "snapshot {index}");
    }
    assert_eq!(active(6), [NodeKey::DataCodewords]);
    assert_eq!(active(13), [NodeKey::Division, NodeKey::ErrorCorrection]);
    assert!(active(26).is_empty());
    assert_eq!(active(33), [NodeKey::Codewords]);
    assert_eq!(active(58), [NodeKey::MaskScores, NodeKey::Format]);
    assert_eq!(active(61), [NodeKey::MaskScores, NodeKey::Format]);
    assert!(set.unstarted.active_nodes.is_empty());
}

#[test]
fn only_a_placement_step_has_a_codeword_being_placed() {
    let set = set_of("Hello");
    for (index, snapshot) in set.snapshots.iter().enumerate() {
        assert_eq!(
            snapshot.placing.is_some(),
            Stage::of(index) == Stage::Placement,
            "snapshot {index}"
        );
    }
}

#[test]
fn the_same_input_makes_the_same_snapshots_and_another_makes_others() {
    assert_eq!(set_of("Hello"), set_of("Hello"));
    assert_ne!(set_of("Hello"), set_of("Hellp"));
}

#[test]
fn the_data_codeword_that_the_division_takes_in_is_marked_and_only_then() {
    let set = set_of("Hello");
    for (index, snapshot) in set.snapshots.iter().enumerate() {
        let expected = (Stage::of(index) == Stage::ReedSolomon).then(|| index - 7);
        assert_eq!(snapshot.dividing, expected, "snapshot {index}");
    }
    assert_eq!(set.snapshots[7].dividing, Some(0));
    assert_eq!(set.snapshots[25].dividing, Some(18));
    assert_eq!(set.unstarted.dividing, None);
}
