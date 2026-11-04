//! The sixteen cases, the saddles, the stitching, the prefixes, the budgets and the presets, checked against the rules
//! of the spec and against a second reading of the same grids.

use super::{oracle::reference_pairs, *};
use crate::marching_squares::field::{FieldId, LabCase, SLOT_NAMES};
use std::collections::BTreeSet;

/// A 2 by 2 grid from its corners, as top left, top right, bottom right and bottom left.
fn cell_grid(tl: f64, tr: f64, br: f64, bl: f64) -> Grid {
    Grid::new(2, 2, vec![tl, tr, bl, br]).expect("valid")
}

fn trace_at_zero(tl: f64, tr: f64, br: f64, bl: f64) -> Trace {
    Trace::build(cell_grid(tl, tr, br, bl), [-0.5, 0.0, 0.5]).expect("valid")
}

/// The pairs of the middle level of a one-cell trace, as the letters of the edges.
fn pairs_of(t: &Trace) -> Vec<String> {
    t.levels[1].by_cell[0]
        .pairs
        .iter()
        .map(|(a, b)| format!("{}{}", a.letter(), b.letter()))
        .collect()
}

fn raw(key: VertexKey) -> (bool, usize, usize) {
    match key {
        VertexKey::H { row, col } => (false, usize::from(row), usize::from(col)),
        VertexKey::V { row, col } => (true, usize::from(row), usize::from(col)),
        VertexKey::SaddleV { .. } | VertexKey::SaddleH { .. } => unreachable!("not a grid edge"),
    }
}

/// A pair of grid edges in a cell, as the oracle writes it.
type CellPair = (usize, usize, (bool, usize, usize), (bool, usize, usize));

/// The pairs of a level as the oracle writes them, from the solver's segments.
fn solver_pairs(t: &Trace, slot: usize) -> BTreeSet<CellPair> {
    let across = t.grid.cells_across();
    let mut set = BTreeSet::new();
    for (index, cell) in t.levels[slot].by_cell.iter().enumerate() {
        if cell.decision == Some(Decision::Tie) {
            continue;
        }
        for s in &cell.segments {
            let (x, y) = (raw(s.a.key), raw(s.b.key));
            set.insert((index / across, index % across, x.min(y), x.max(y)));
        }
    }
    set
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn all_sixteen_masks_agree_with_the_independent_reading() {
    for mask in 0..16_u8 {
        // Distinct magnitudes, so that no corner is at the level and no two products are equal.
        let sign = |bit: u8, size: f64| if mask & bit != 0 { size } else { -size };
        let t = trace_at_zero(sign(1, 1.3), sign(2, 2.1), sign(4, 3.4), sign(8, 1.7));
        assert_eq!(t.levels[1].by_cell[0].mask, mask);
        assert_eq!(solver_pairs(&t, 1), reference_pairs(&t.grid, 0.0), "mask {mask}");
    }
}

#[test]
fn the_masks_that_are_not_ambiguous_have_the_pairs_of_the_table() {
    let expected = [
        (0, ""),
        (1, "TL"),
        (2, "TR"),
        (3, "LR"),
        (4, "RB"),
        (6, "TB"),
        (7, "LB"),
        (8, "BL"),
        (9, "TB"),
        (11, "RB"),
        (12, "LR"),
        (13, "TR"),
        (14, "TL"),
        (15, ""),
    ];
    for (mask, pairs) in expected {
        let sign = |bit: u8, size: f64| if mask & bit != 0 { size } else { -size };
        let t = trace_at_zero(sign(1, 1.3), sign(2, 2.1), sign(4, 3.4), sign(8, 1.7));
        assert_eq!(pairs_of(&t).join(""), pairs, "mask {mask}");
    }
}

#[test]
fn the_golden_saddle_fixtures_have_their_mask_q_and_pairs() {
    let golden: [(LabCase, u8, f64, &str); 5] = [
        (LabCase::FivePositive, 5, 1.0, "TR BL"),
        (LabCase::FiveNegative, 5, -4.0, "TL RB"),
        (LabCase::TenPositive, 10, 4.0, "TR BL"),
        (LabCase::TenNegative, 10, -1.0, "TL RB"),
        (LabCase::Tie, 5, 0.0, "TB LR"),
    ];
    for (case, mask, q, pairs) in golden {
        let t = Trace::build(FieldId::Lab(case).grid().expect("grid"), FieldId::Lab(case).levels()).expect("trace");
        let cell = &t.levels[1].by_cell[0];
        assert_eq!((cell.mask, cell.q), (mask, Some(q)), "{case:?}");
        assert_eq!(pairs_of(&t).join(" "), pairs, "{case:?}");
    }
}

#[test]
fn an_exact_tie_is_two_strands_that_cross_at_the_saddle_and_never_a_degree_four_node() {
    let t = Trace::build(FieldId::Lab(LabCase::Tie).grid().expect("grid"), [-0.5, 0.0, 0.5]).expect("trace");
    let cell = &t.levels[1].by_cell[0];
    assert_eq!(cell.decision, Some(Decision::Tie));
    assert_eq!(cell.segments.len(), 4);
    let saddles: Vec<&Crossing> = cell
        .segments
        .iter()
        .flat_map(|s| [&s.a, &s.b])
        .filter(|e| matches!(e.key, VertexKey::SaddleV { .. } | VertexKey::SaddleH { .. }))
        .collect();
    assert!(
        saddles.iter().all(|e| e.point == Point2 { x: 0.5, y: 0.5 }),
        "both at the middle of the cell"
    );
    let mut degree: std::collections::BTreeMap<VertexKey, usize> = std::collections::BTreeMap::new();
    for s in &cell.segments {
        *degree.entry(s.a.key).or_default() += 1;
        *degree.entry(s.b.key).or_default() += 1;
    }
    assert_eq!(degree[&VertexKey::SaddleV { row: 0, col: 0 }], 2);
    assert_eq!(degree[&VertexKey::SaddleH { row: 0, col: 0 }], 2);
    assert!(degree.values().all(|&d| d <= 2));
    let contours = t.contours(1, 1);
    assert_eq!(contours.len(), 2, "two strands");
    assert!(contours.iter().all(|c| !c.closed && c.keys.len() == 3));
    assert_eq!(totals(&contours, &t.segments(1, 1)), Totals { open: 2, closed: 0, ties: 1 });
}

#[test]
fn the_sign_of_q_is_the_branch_and_the_complement_and_rotation_agree() {
    for case in LabCase::ALL {
        let [tl, tr, br, bl] = case.corners();
        let t = trace_at_zero(tl, tr, br, bl);
        let cell = &t.levels[1].by_cell[0];
        // The complement: every sample and the level negated gives the same geometry, to the bit.
        let negated = Trace::build(cell_grid(-tl, -tr, -br, -bl), [-0.5, 0.0, 0.5]).expect("valid");
        let other = &negated.levels[1].by_cell[0];
        assert_eq!(other.mask, 15 - cell.mask, "{case:?}");
        assert_eq!(other.q, cell.q, "Q does not change sign");
        let points = |c: &CellResult| c.segments.iter().flat_map(|s| [s.a.point, s.b.point]).collect::<Vec<_>>();
        assert_eq!(points(cell), points(other), "{case:?}");
        // A quarter turn: the corners move round, case 5 and 10 swap and Q changes sign.
        let turned = trace_at_zero(bl, tl, tr, br);
        let c = &turned.levels[1].by_cell[0];
        assert_eq!(c.mask, if cell.mask == 5 { 10 } else { 5 }, "{case:?}");
        assert_eq!(c.q, cell.q.map(|q| if q == 0.0 { 0.0 } else { -q }), "{case:?}");
        let lengths = |c: &CellResult| {
            let mut v: Vec<u64> = c
                .segments
                .iter()
                .map(|s| ((s.a.point.x - s.b.point.x).hypot(s.a.point.y - s.b.point.y) * 1e9).round() as u64)
                .collect();
            v.sort_unstable();
            v
        };
        assert_eq!(lengths(cell), lengths(c), "{case:?}");
    }
}

#[test]
fn rho_is_the_relative_size_of_q_and_does_not_change_with_scale() {
    let t = trace_at_zero(3.0, -1.0, 1.0, -2.0);
    let rho = t.levels[1].by_cell[0].rho.expect("ambiguous");
    assert!((rho - 1.0 / 3.0).abs() < 1e-12, "{rho}");
    let scaled = Trace::build(cell_grid(30.0, -10.0, 10.0, -20.0), [-5.0, 0.0, 5.0]).expect("valid");
    assert!((scaled.levels[1].by_cell[0].rho.expect("ambiguous") - rho).abs() < 1e-12);
    assert!(!t.levels[1].by_cell[0].near_degenerate());
    let tie = Trace::build(FieldId::Lab(LabCase::Tie).grid().expect("grid"), [-0.5, 0.0, 0.5]).expect("trace");
    assert_eq!(tie.levels[1].by_cell[0].rho, Some(0.0));
    assert!(!tie.levels[1].by_cell[0].near_degenerate(), "a tie is exact and not near");
    let near = trace_at_zero(10.0, -1.0, 1.0, -9.5);
    let cell = &near.levels[1].by_cell[0];
    assert!(cell.near_degenerate(), "{:?}", cell.rho);
    assert_eq!(cell.decision, Some(Decision::Positive), "the flag never changes the branch");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_crossing_is_calculated_once_and_both_cells_use_it_bit_for_bit() {
    let t = Trace::build(FieldId::Ripple.grid().expect("grid"), FieldId::Ripple.levels()).expect("trace");
    for level in &t.levels {
        let mut seen: std::collections::BTreeMap<VertexKey, (u64, u64)> = std::collections::BTreeMap::new();
        for cell in &level.by_cell {
            for s in &cell.segments {
                for end in [s.a, s.b] {
                    let bits = (end.point.x.to_bits(), end.point.y.to_bits());
                    assert_eq!(*seen.entry(end.key).or_insert(bits), bits, "{:?}", end.key);
                }
            }
        }
    }
}

#[test]
fn a_half_plane_has_one_straight_open_contour_at_the_exact_value() {
    let values = (0..4).flat_map(|_| (0..5).map(|c| c as f64)).collect();
    let t = Trace::build(Grid::new(5, 4, values).expect("valid"), [0.5, 1.5, 2.5]).expect("trace");
    for (slot, x) in [0.5, 1.5, 2.5].into_iter().enumerate() {
        let contours = t.contours(slot, t.grid.cell_count());
        assert_eq!(contours.len(), 1);
        assert!(!contours[0].closed);
        assert!(contours[0].vertices.iter().all(|p| p.x == x), "exact, not within a tolerance");
        assert_eq!(contours[0].vertices.len(), 4);
        let ys: Vec<f64> = contours[0].vertices.iter().map(|p| p.y).collect();
        assert!(ys.windows(2).all(|w| w[0] < w[1]) || ys.windows(2).all(|w| w[0] > w[1]));
    }
    t.validate().expect("valid");
}

/// A peak: a bowl turned over, on a 5 by 5 grid.
fn peak() -> Trace {
    let values = (0..5)
        .flat_map(|r| (0..5).map(move |c| -(((c - 2) * (c - 2) + (r - 2) * (r - 2)) as f64)))
        .collect();
    Trace::build(Grid::new(5, 5, values).expect("valid"), [-3.5, -1.5, -0.5]).expect("trace")
}

#[test]
fn a_smooth_peak_has_one_closed_loop_at_each_level_and_no_open_end() {
    let t = peak();
    for slot in 0..LEVELS {
        let contours = t.contours(slot, t.grid.cell_count());
        assert_eq!(contours.len(), 1, "{}", SLOT_NAMES[slot]);
        assert!(contours[0].closed);
    }
    t.validate().expect("valid");
}

#[test]
fn a_loop_is_drawn_with_its_first_point_repeated_and_not_stored_twice() {
    let t = peak();
    let c = &t.contours(2, 16)[0];
    assert_eq!(c.drawn_points().len(), c.vertices.len() + 1);
    assert_eq!(c.drawn_points().first(), c.drawn_points().last());
    assert_eq!(c.segment_count(), c.keys.len());
}

#[test]
fn a_cycle_starts_at_its_lowest_key_and_goes_the_way_with_the_lower_sequence() {
    let t = peak();
    for slot in 0..LEVELS {
        let c = &t.contours(slot, 16)[0];
        assert_eq!(c.keys[0], *c.keys.iter().min().expect("keys"));
        let n = c.keys.len();
        let backward: Vec<VertexKey> = (0..n).map(|i| c.keys[(n - i) % n]).collect();
        assert!(c.keys <= backward);
    }
}

#[test]
fn the_order_of_the_segments_does_not_change_the_contours() {
    let t = Trace::build(FieldId::Ripple.grid().expect("grid"), FieldId::Ripple.levels()).expect("trace");
    for slot in 0..LEVELS {
        let mut segments = t.segments(slot, t.grid.cell_count());
        let before = stitch(&segments);
        segments.reverse();
        assert_eq!(stitch(&segments), before);
        let half = segments.len() / 2;
        segments.rotate_left(half);
        assert_eq!(stitch(&segments), before);
    }
}

#[test]
fn every_open_chain_starts_at_its_lower_end() {
    let t = Trace::build(FieldId::Ripple.grid().expect("grid"), FieldId::Ripple.levels()).expect("trace");
    for slot in 0..LEVELS {
        for c in t.contours(slot, t.grid.cell_count()).iter().filter(|c| !c.closed) {
            assert!(c.keys.first() < c.keys.last(), "{:?}", c.keys);
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn the_first_snapshot_has_no_contours_and_the_last_has_them_all() {
    let t = peak();
    for slot in 0..LEVELS {
        assert!(t.contours(slot, 0).is_empty());
        let all: usize = t.contours(slot, t.grid.cell_count()).iter().map(Contour::segment_count).sum();
        assert_eq!(all, t.segments(slot, t.grid.cell_count()).len());
    }
}

#[test]
fn a_prefix_has_only_the_segments_of_the_cells_so_far_and_every_one_is_used_once() {
    let t = Trace::build(FieldId::Ripple.grid().expect("grid"), FieldId::Ripple.levels()).expect("trace");
    for k in [1, 7, 50, 111, 200, 431] {
        for slot in 0..LEVELS {
            let segments = t.segments(slot, k);
            assert!(segments.iter().all(|s| (s.id.cell as usize) < k));
            let contours = stitch(&segments);
            let used: usize = contours.iter().map(Contour::segment_count).sum();
            assert_eq!(used, segments.len(), "k = {k}");
            // No crossing is in a contour unless a segment of a cell so far has it.
            let keys: BTreeSet<VertexKey> = segments.iter().flat_map(|s| [s.a.key, s.b.key]).collect();
            assert!(contours.iter().all(|c| c.keys.iter().all(|key| keys.contains(key))));
        }
    }
}

#[test]
fn a_loop_is_closed_only_once_all_its_cells_have_been_visited() {
    let t = peak();
    let across = t.grid.cells_across();
    let mut closed_at = None;
    for k in 0..=t.grid.cell_count() {
        let c = t.contours(2, k);
        if c.iter().any(|c| c.closed) {
            closed_at = Some(k);
            break;
        }
        // Before that every chain is open, with its provisional ends on the frontier.
        assert!(c.iter().all(|c| !c.closed));
    }
    let k = closed_at.expect("it closes");
    // The last cell of the innermost loop is at row 2, column 2 (cells run 0..4), so row-major index 2 * 4 + 2 + 1.
    assert_eq!(k, 2 * across + 2 + 1);
    let before = t.contours(2, k - 1);
    assert!(before.iter().all(|c| !c.closed), "one cell fewer leaves two open ends");
    assert_eq!(before.iter().filter(|c| !c.closed).count(), 1);
}

#[test]
fn the_digest_does_not_depend_on_how_it_was_reached() {
    let a = Trace::build(FieldId::Ripple.grid().expect("grid"), FieldId::Ripple.levels()).expect("trace");
    let b = Trace::build(FieldId::Ripple.grid().expect("grid"), FieldId::Ripple.levels()).expect("trace");
    for k in [0, 1, 40, 431] {
        assert_eq!(a.digest(k), b.digest(k));
    }
    assert_ne!(a.digest(0), a.digest(431), "a finished field differs from an empty one");
    assert!(
        (0..431).any(|k| a.digest(k) != a.digest(k + 1)),
        "a cell that adds a segment changes it"
    );
    assert!(
        (0..431).any(|k| a.digest(k) == a.digest(k + 1)),
        "and one that adds none does not"
    );
    let moved = Trace::build(FieldId::Ripple.grid().expect("grid"), [-0.53, 0.102, 0.52]).expect("trace");
    assert_ne!(a.digest(431), moved.digest(431));
    let back = Trace::build(FieldId::Ripple.grid().expect("grid"), [-0.53, 0.101, 0.52]).expect("trace");
    assert_eq!(a.digest(431), back.digest(431), "restoring the level restores the digest");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn every_preset_follows_the_rules_and_agrees_with_the_independent_reading() {
    for field in FieldId::all() {
        let t = Trace::build(field.grid().expect("grid"), field.levels()).expect("trace");
        t.validate().unwrap_or_else(|e| panic!("{field:?}: {e}"));
        for slot in 0..LEVELS {
            assert_eq!(
                solver_pairs(&t, slot),
                reference_pairs(&t.grid, t.levels[slot].level),
                "{field:?} {slot}"
            );
        }
    }
}

/// The ambiguous cells of a level as `(row, column, mask, Q positive)`.
fn ambiguous(t: &Trace, slot: usize) -> Vec<(usize, usize, u8, bool)> {
    t.levels[slot]
        .by_cell
        .iter()
        .enumerate()
        .filter(|(_, c)| c.ambiguous())
        .map(|(i, c)| {
            let (r, col) = t.grid.cell_at(i);
            (r, col, c.mask, c.q.is_some_and(|q| q > 0.0))
        })
        .collect()
}

#[test]
fn the_ripple_levels_each_have_one_ambiguous_cell_with_the_documented_mask_and_q() {
    let t = Trace::build(FieldId::Ripple.grid().expect("grid"), FieldId::Ripple.levels()).expect("trace");
    assert_eq!(ambiguous(&t, 0), vec![(4, 1, 10, false)]);
    assert_eq!(ambiguous(&t, 1), vec![(14, 12, 10, false)]);
    assert_eq!(ambiguous(&t, 2), vec![(13, 22, 5, true)]);
    let q = |slot: usize, index: usize| t.levels[slot].by_cell[index].q.expect("ambiguous");
    let across = t.grid.cells_across();
    assert!((q(0, 4 * across + 1) + 7.889e-4).abs() < 1e-6);
    assert!((q(1, 14 * across + 12) + 4.827e-5).abs() < 1e-7);
    assert!((q(2, 13 * across + 22) - 1.650e-4).abs() < 1e-6);
    let rho = |slot: usize, index: usize| t.levels[slot].by_cell[index].rho.expect("ambiguous");
    assert!((rho(0, 4 * across + 1) - 0.892).abs() < 1e-3);
    assert!((rho(1, 14 * across + 12) - 0.425).abs() < 1e-3);
    assert!((rho(2, 13 * across + 22) - 0.416).abs() < 1e-3);
    assert!(
        t.levels.iter().all(|l| l.by_cell.iter().all(|c| !c.near_degenerate())),
        "none is near-degenerate"
    );
}

/// The shortest distance between the two segments of a cell, in grid units.
fn gap(cell: &CellResult) -> f64 {
    fn to_segment(p: Point2, a: Point2, b: Point2) -> f64 {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        (p.x - (a.x + t * dx)).hypot(p.y - (a.y + t * dy))
    }
    let (s, u) = (cell.segments[0], cell.segments[1]);
    [
        to_segment(s.a.point, u.a.point, u.b.point),
        to_segment(s.b.point, u.a.point, u.b.point),
        to_segment(u.a.point, s.a.point, s.b.point),
        to_segment(u.b.point, s.a.point, s.b.point),
    ]
    .into_iter()
    .fold(f64::INFINITY, f64::min)
}

#[test]
fn the_ripple_saddles_are_drawn_well_apart() {
    let t = Trace::build(FieldId::Ripple.grid().expect("grid"), FieldId::Ripple.levels()).expect("trace");
    let across = t.grid.cells_across();
    let gaps = [
        gap(&t.levels[0].by_cell[4 * across + 1]),
        gap(&t.levels[1].by_cell[14 * across + 12]),
        gap(&t.levels[2].by_cell[13 * across + 22]),
    ];
    for (got, want) in gaps.into_iter().zip([0.848, 0.708, 0.757]) {
        assert!((got - want).abs() < 2e-3, "{got} against {want}");
    }
}

#[test]
fn moving_the_middle_ripple_level_by_a_thousandth_flips_the_pairing_and_back_restores_it() {
    let grid = || FieldId::Ripple.grid().expect("grid");
    let across = grid().cells_across();
    let at = |level: f64| {
        let t = Trace::build(grid(), [-0.53, level, 0.52]).expect("trace");
        let cell = t.levels[1].by_cell[14 * across + 12].clone();
        (cell.mask, cell.q.expect("ambiguous"), cell.pairs)
    };
    let (mask1, q1, pairs1) = at(0.101);
    let (mask2, q2, pairs2) = at(0.102);
    assert_eq!((mask1, mask2), (10, 10));
    assert!(q1 < 0.0 && q2 > 0.0);
    assert!((q2 - 4.312e-5).abs() < 1e-7);
    assert_ne!(pairs1, pairs2);
    assert_eq!(at(0.101), (mask1, q1, pairs1));
}

#[test]
fn the_coarse_preset_has_six_six_and_two_ambiguous_cells_with_nonzero_q() {
    let t = Trace::build(FieldId::Coarse.grid().expect("grid"), FieldId::Coarse.levels()).expect("trace");
    let counts: Vec<usize> = (0..LEVELS).map(|s| ambiguous(&t, s).len()).collect();
    assert_eq!(counts, vec![6, 6, 2]);
    for slot in 0..2 {
        let masks: BTreeSet<u8> = ambiguous(&t, slot).iter().map(|a| a.2).collect();
        assert_eq!(masks, BTreeSet::from([5, 10]), "{slot}");
    }
    assert!(ambiguous(&t, 2).iter().all(|a| a.2 == 10));
    assert!(
        t.levels
            .iter()
            .flat_map(|l| &l.by_cell)
            .filter(|c| c.ambiguous())
            .all(|c| c.q.is_some_and(|q| q != 0.0))
    );
}

#[test]
fn no_preset_sample_is_within_a_ten_thousandth_of_a_level_and_all_four_lab_levels_are_clear() {
    for field in FieldId::all() {
        let grid = field.grid().expect("grid");
        for level in field.levels() {
            let nearest = grid.values.iter().map(|v| (v - level).abs()).fold(f64::INFINITY, f64::min);
            assert!(nearest > 1e-4, "{field:?} {level}: {nearest}");
        }
    }
}

#[test]
fn a_sample_exactly_at_a_level_is_refused_by_name() {
    let grid = FieldId::Ripple.grid().expect("grid");
    assert_eq!(
        Trace::build(grid, [-0.53, 0.1, 0.52]),
        Err(SolverError::VertexOnLevel { row: 14, col: 12, slot: 1 })
    );
}

#[test]
fn the_twin_hills_have_closed_loops_and_follow_the_rules() {
    let t = Trace::build(FieldId::Twin.grid().expect("grid"), FieldId::Twin.levels()).expect("trace");
    assert!(t.contours(2, t.grid.cell_count()).iter().any(|c| c.closed));
    t.validate().expect("valid");
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[test]
fn a_field_that_is_the_wrong_size_or_has_a_bad_sample_is_refused() {
    assert_eq!(
        Grid::new(1, 3, vec![0.0; 3]),
        Err(SolverError::BadDimensions { width: 1, height: 3 })
    );
    assert_eq!(
        Grid::new(66, 2, vec![0.0; 132]),
        Err(SolverError::BadDimensions { width: 66, height: 2 })
    );
    assert_eq!(
        Grid::new(3, 3, vec![0.0; 8]),
        Err(SolverError::WrongSampleCount { expected: 9, got: 8 })
    );
    let mut values = vec![0.5; 9];
    values[4] = f64::NAN;
    assert_eq!(Grid::new(3, 3, values), Err(SolverError::NonFiniteSample { row: 1, col: 1 }));
    let mut values = vec![0.5; 9];
    values[8] = f64::INFINITY;
    assert_eq!(Grid::new(3, 3, values), Err(SolverError::NonFiniteSample { row: 2, col: 2 }));
}

#[test]
fn levels_must_be_finite_and_strictly_ascending() {
    let grid = || Grid::new(2, 2, vec![1.0, 2.0, 3.0, 4.0]).expect("valid");
    for levels in [
        [0.0, 0.0, 1.0],
        [1.0, 0.5, 2.0],
        [0.5, 1.5, 1.5],
        [f64::NAN, 0.0, 1.0],
        [0.0, 1.0, f64::INFINITY],
    ] {
        assert_eq!(Trace::build(grid(), levels), Err(SolverError::BadLevels), "{levels:?}");
    }
}

#[test]
fn the_largest_field_is_accepted_within_its_budget() {
    let values = (0..65 * 65)
        .map(|i| ((i % 65) as f64 * 0.7).sin() * ((i / 65) as f64 * 0.9).cos() + 0.00001 * i as f64)
        .collect();
    let grid = Grid::new(65, 65, values).expect("valid");
    assert_eq!(grid.cell_count(), MAX_CELLS);
    let t = Trace::build(grid, [-0.31, 0.123, 0.57]).expect("within budget");
    t.validate().expect("valid");
    assert_eq!(MAX_SEGMENTS, 49_152);
    assert_eq!(MAX_INCIDENCES, 98_304);
}

#[test]
fn every_error_says_what_it_is_about() {
    for e in [
        SolverError::BadDimensions { width: 1, height: 1 },
        SolverError::WrongSampleCount { expected: 4, got: 3 },
        SolverError::NonFiniteSample { row: 0, col: 0 },
        SolverError::BadLevels,
        SolverError::VertexOnLevel { row: 0, col: 0, slot: 1 },
        SolverError::GeometryBudgetExceeded {
            kind: "segments",
            requested: 9,
            limit: 3,
        },
        SolverError::Topology("x".to_owned()),
    ] {
        assert!(!e.to_string().is_empty());
    }
}

#[test]
fn keys_are_ordered_by_kind_then_row_then_column() {
    let h = VertexKey::H { row: 9, col: 9 };
    let v = VertexKey::V { row: 0, col: 0 };
    let sv = VertexKey::SaddleV { row: 0, col: 0 };
    let sh = VertexKey::SaddleH { row: 0, col: 0 };
    assert!(h < v && v < sv && sv < sh);
    assert!(VertexKey::H { row: 1, col: 5 } < VertexKey::H { row: 2, col: 0 });
    assert!(VertexKey::H { row: 1, col: 4 } < VertexKey::H { row: 1, col: 5 });
}
