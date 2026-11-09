use super::*;

fn spec(rows: u64, cols: u64, label_len: u64, cells: u64) -> Spec {
    Spec {
        seed: 1,
        rows,
        cols,
        per_row: 2,
        hub_cols: 0,
        hub_percent: 0,
        label_len,
        cells,
        updates: 1,
        steps: 1,
    }
}

#[test]
fn no_two_nodes_overlap_and_the_two_blocks_are_apart() {
    for s in [
        spec(7, 5, 3, 4),
        spec(100, 80, 12, 10),
        spec(1, 1, 1, 1),
        spec(50, 50, 200, 25),
        spec(8, 8, 4, 8),
        spec(64, 40, 6, 10),
    ] {
        let l = Layout::new(&s);
        let mut all: Vec<Rect> = (0..s.cols as usize).map(|j| l.col_node(j)).collect();
        all.extend((0..s.rows as usize).map(|i| l.row_node(i)));
        for (a, ra) in all.iter().enumerate() {
            for rb in &all[a + 1..] {
                let apart = ra.x + ra.w <= rb.x || rb.x + rb.w <= ra.x || ra.y + ra.h <= rb.y || rb.y + rb.h <= ra.y;
                assert!(apart, "{ra:?} overlaps {rb:?}");
            }
        }
    }
}

#[test]
fn everything_is_inside_the_stated_extent_and_below_the_data_nodes() {
    for s in [spec(300, 200, 8, 36), spec(12, 9, 5, 9), spec(64, 64, 6, 20)] {
        let l = Layout::new(&s);
        for j in 0..s.cols as usize {
            let r = l.col_node(j);
            assert!(
                r.x >= 0.0 && r.x + r.w <= l.width && r.y >= l.header_height && r.y + r.h <= l.height,
                "column {j}: {r:?}"
            );
        }
        for i in 0..s.rows as usize {
            let r = l.row_node(i);
            assert!(
                r.x + r.w <= l.width && r.y >= l.header_height && r.y + r.h <= l.height,
                "row {i}: {r:?}"
            );
        }
    }
}

/// A graph that is meant to be read is two single columns with a wide channel between them, with room between the nodes, and the
/// shorter column centred against the taller. A large one is packed into blocks, and is there to be measured.
#[test]
fn a_small_graph_is_two_single_columns_with_a_wide_channel() {
    let l = Layout::new(&spec(12, 8, 4, 8));
    assert!(l.single);
    let xs: Vec<f64> = (0..8).map(|j| l.col_node(j).x).collect();
    assert!(xs.iter().all(|x| *x == xs[0]), "the columns are one column");
    let rows: Vec<f64> = (0..12).map(|i| l.row_node(i).x).collect();
    assert!(rows.iter().all(|x| *x == rows[0]), "the rows are one column");
    assert!(
        l.row_node(0).x - (l.col_node(0).x + l.node_width) >= SINGLE_CHANNEL,
        "the edges have room to slope"
    );
    let gap = l.col_node(1).y - (l.col_node(0).y + l.col_node(0).h);
    assert!(gap >= SINGLE_GAP_Y, "the nodes are {gap} apart");
    // Eight columns are shorter than twelve rows, and are centred against them.
    let col_mid = (l.col_node(0).y + l.col_node(7).y + l.node_height) / 2.0;
    let row_mid = (l.row_node(0).y + l.row_node(11).y + l.node_height) / 2.0;
    assert!(
        (col_mid - row_mid).abs() < 1e-9,
        "the columns are at {col_mid} and the rows at {row_mid}"
    );

    let big = Layout::new(&spec(300, 300, 4, 10));
    assert!(!big.single && big.col_per_line > 1, "a large graph is packed into blocks");
    assert!(Layout::new(&spec(SINGLE_MAX, SINGLE_MAX, 4, 10)).single);
    assert!(!Layout::new(&spec(SINGLE_MAX + 1, 4, 4, 4)).single);
}

#[test]
fn a_shifted_layout_moves_every_place_and_keeps_the_extent() {
    let s = spec(12, 8, 4, 8);
    let (plain, moved) = (Layout::new(&s), Layout::new(&s).shifted(40.0, 25.0));
    assert_eq!((moved.width, moved.height), (plain.width, plain.height));
    for (a, b) in [(plain.col_node(3), moved.col_node(3)), (plain.row_node(11), moved.row_node(11))] {
        assert_eq!((b.x - a.x, b.y - a.y, b.w, b.h), (40.0, 25.0, a.w, a.h));
    }
    assert_eq!(moved.x_node, (plain.x_node.0 + 40.0, plain.x_node.1 + 25.0));
    assert_eq!(moved.spy_origin, (plain.spy_origin.0 + 40.0, plain.spy_origin.1 + 25.0));
}

#[test]
fn a_longer_label_gives_a_wider_node_up_to_a_limit() {
    let w = |len| Layout::new(&spec(10, 10, len, 4)).node_width;
    assert!(w(1) < w(20) && w(20) < w(60));
    assert_eq!(w(100_000), 640.0);
    assert_eq!(w(1), 64.0, "a single column has a minimum width");
    assert_eq!(Layout::new(&spec(300, 300, 1, 4)).node_width, 36.0);
}

#[test]
fn the_data_node_grid_holds_every_cell() {
    for cells in [1u64, 2, 9, 10, 100, 1000] {
        let l = Layout::new(&spec(2000, 2000, 4, cells));
        assert!((l.data_columns as u64) * cells.div_ceil(l.data_columns as u64) >= cells);
    }
}

/// Each of the three boxes has room under it for its caption, and the captions are above the graph and apart from each other.
#[test]
fn every_box_has_room_for_its_caption_above_the_graph() {
    for s in [
        spec(8, 8, 1, 8),
        spec(300, 300, 4, 100),
        spec(40, 40, 6, 20),
        spec(1000, 1000, 4, 1000),
    ] {
        let l = Layout::new(&s);
        let rows = s.cells.div_ceil(l.data_columns as u64) as f64;
        let data_bottom = rows * CELL_PITCH.1 + DATA_PAD;
        for (name, caption, box_bottom) in [
            ("x", l.x_caption, data_bottom),
            ("y", l.y_caption, data_bottom),
            ("pattern", l.spy_caption, l.aside_size.1),
        ] {
            assert!(
                caption.1 > box_bottom,
                "the {name} caption begins at {} and its box ends at {box_bottom}",
                caption.1
            );
            let last_line = caption.1 + 3.0 * CAPTION_LINE;
            assert!(
                last_line < l.header_height,
                "the {name} caption ends at {last_line} and the graph begins at {}",
                l.header_height
            );
        }
        assert!(
            l.x_caption.0 < l.y_caption.0 && l.y_caption.0 < l.spy_caption.0,
            "the captions are not in the order of their boxes"
        );
    }
}

#[test]
fn a_shifted_layout_moves_the_captions_with_the_boxes() {
    let s = spec(12, 8, 4, 8);
    let (plain, moved) = (Layout::new(&s), Layout::new(&s).shifted(40.0, 25.0));
    for (a, b) in [
        (plain.x_caption, moved.x_caption),
        (plain.y_caption, moved.y_caption),
        (plain.spy_caption, moved.spy_caption),
    ] {
        assert_eq!((b.0 - a.0, b.1 - a.1), (40.0, 25.0));
    }
}

/// A graph that is small enough is drawn with its matrix in full, in cells that fit the box and are in rows and columns; a larger one is
/// shown as the square of its pattern.
#[test]
fn a_small_matrix_is_drawn_in_full_and_a_large_one_as_a_pattern() {
    let l = Layout::new(&spec(8, 8, 1, 8));
    let m = l.matrix.expect("a matrix of 8 by 8 is drawn in full");
    let (w, h) = m.size();
    assert_eq!((w, h), l.aside_size);
    assert_eq!((m.rows, m.cols), (8, 8));
    for i in 0..8 {
        for j in 0..8 {
            let c = m.cell(i, j);
            assert!(
                c.x >= m.origin.0 + MATRIX_LABELS.0 && c.x + c.w <= m.origin.0 + w,
                "cell {i},{j}: {c:?}"
            );
            assert!(
                c.y >= m.origin.1 + MATRIX_LABELS.1 && c.y + c.h <= m.origin.1 + h,
                "cell {i},{j}: {c:?}"
            );
            if j > 0 {
                assert_eq!(c.x - m.cell(i, j - 1).x, MATRIX_CELL.0, "a row is evenly spaced");
            }
            if i > 0 {
                assert_eq!(c.y - m.cell(i - 1, j).y, MATRIX_CELL.1, "a column is evenly spaced");
            }
        }
    }
    assert!(m.origin.0 >= l.width - w - 1.0, "the matrix is at the right of everything");
    assert_eq!(
        Layout::new(&spec(24, 24, 1, 10)).aside_size,
        MatrixGeom {
            origin: (0.0, 0.0),
            rows: 24,
            cols: 24
        }
        .size()
    );
    assert!(Layout::new(&spec(24, 24, 1, 10)).matrix.is_some());
    let big = Layout::new(&spec(25, 24, 1, 10));
    assert!(big.matrix.is_none(), "25 rows are not drawn in full");
    assert_eq!(big.aside_size, (SPY_SIDE, SPY_SIDE));
    assert!(Layout::new(&spec(300, 300, 4, 10)).matrix.is_none());
}

#[test]
fn a_shifted_layout_moves_the_matrix_with_everything_else() {
    let s = spec(8, 8, 1, 8);
    let (plain, moved) = (Layout::new(&s), Layout::new(&s).shifted(40.0, 25.0));
    let (a, b) = (plain.matrix.expect("a matrix"), moved.matrix.expect("a matrix"));
    assert_eq!((b.origin.0 - a.origin.0, b.origin.1 - a.origin.1), (40.0, 25.0));
    assert_eq!(moved.aside_size, plain.aside_size);
}

/// The vector of x is under the matrix, each entry under its own column, and the vector of y beside it, each entry level with its row.
#[test]
fn the_vectors_line_up_with_the_matrix() {
    let l = Layout::new(&spec(8, 6, 1, 8));
    let m = l.matrix.expect("a matrix");
    let (w, h) = m.size();
    for j in 0..m.cols {
        let (c, e) = (m.cell(0, j), m.x_entry(j));
        assert_eq!((e.x, e.w), (c.x, c.w), "x{j} is under its column");
        assert!(e.y >= m.cell(m.rows - 1, j).y + MATRIX_CELL.1, "x{j} is below the last row");
        assert!(e.y + e.h <= m.origin.1 + h + 0.001);
    }
    for i in 0..m.rows {
        let (c, e) = (m.cell(i, 0), m.y_entry(i));
        assert_eq!((e.y, e.h), (c.y, c.h), "y{i} is level with its row");
        assert!(
            e.x >= m.cell(i, m.cols - 1).x + MATRIX_CELL.0,
            "y{i} is right of the last column"
        );
        assert!(e.x + e.w <= m.origin.0 + w + 0.001);
    }
}
