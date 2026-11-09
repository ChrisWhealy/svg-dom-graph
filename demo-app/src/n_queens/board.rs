//! The layers of the board for one size, and the writing of one snapshot into them.

use super::{
    chrome::INFO,
    dom::{add, make, set},
    projection::{BOARD_SIZE, CellState, MARGIN, OutlineKind, View, cell_origin, cell_size, ray_path},
    solver::Conflict,
};

/// The class names that the style sheet draws.
pub(super) const OFF: &str = "nq-off";

/// The three lines through a square, in the order that their rays are made.
const LINES: [Conflict; 3] = [Conflict::Column, Conflict::MainDiagonal, Conflict::AntiDiagonal];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that is drawn for one size of board. The layers are separate, so that the rays can pass under the queens
/// and the outline over the rays: the fills, the rays, the queens, the outline, the frame, the grid that holds the
/// cells a person can focus, and last the focus ring.
pub(super) struct BoardParts {
    pub(super) size: usize,
    pub(super) root: web_sys::Element,
    pub(super) glyphs: Vec<web_sys::Element>,
    pub(super) hits: Vec<web_sys::Element>,
    pub(super) rays: [web_sys::Element; 3],
    pub(super) outline: web_sys::Element,
    pub(super) frame: web_sys::Element,
    pub(super) ring: web_sys::Element,
}

impl BoardParts {
    pub(super) fn at(&self, row: usize, col: usize) -> usize {
        row * self.size + col
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the layers for a board of `size`, which are not yet in the page. The squares are all empty, and nothing is
/// drawn but the board; [`apply`] draws a snapshot.
///
/// # Errors
///
/// Returns `Err` if an element could not be made or added.
pub(super) fn build_board(document: &web_sys::Document, size: usize) -> Result<BoardParts, String> {
    let cell = cell_size(size);
    let root = make(document, "g", &[("class", "nq-board")])?;
    let fills = make(document, "g", &[("class", "nq-fills"), ("aria-hidden", "true")])?;
    let rays = make(document, "g", &[("class", "nq-rays"), ("aria-hidden", "true")])?;
    let glyphs_layer = make(document, "g", &[("class", "nq-glyphs"), ("aria-hidden", "true")])?;
    let grid = make(
        document,
        "g",
        &[
            ("class", "nq-grid"),
            ("role", "grid"),
            ("aria-label", &grid_label(size)),
            ("aria-describedby", INFO),
        ],
    )?;
    let (mut glyphs, mut hits) = (Vec::with_capacity(size * size), Vec::with_capacity(size * size));
    for row in 0..size {
        let row_group = make(document, "g", &[("role", "row")])?;
        for col in 0..size {
            let (x, y) = cell_origin(size, row, col);
            let (x, y, w) = (x.to_string(), y.to_string(), cell.to_string());
            let shade = if (row + col) % 2 == 0 { "nq-light" } else { "nq-dark" };
            add(
                &fills,
                &make(
                    document,
                    "rect",
                    &[("x", &x), ("y", &y), ("width", &w), ("height", &w), ("class", shade)],
                )?,
            )?;
            let glyph = make(
                document,
                "text",
                &[
                    ("class", "nq-glyph"),
                    ("x", &(x.parse::<f64>().unwrap_or(0.0) + cell / 2.0).to_string()),
                    ("y", &(y.parse::<f64>().unwrap_or(0.0) + cell / 2.0).to_string()),
                    ("font-size", &(cell * 0.72).to_string()),
                ],
            )?;
            glyph.set_text_content(Some("\u{265B}"));
            add(&glyphs_layer, &glyph)?;
            glyphs.push(glyph);
            let hit = make(
                document,
                "rect",
                &[
                    ("id", &format!("nq-r{row}-c{col}")),
                    ("class", "nq-cell"),
                    ("role", "gridcell"),
                    ("tabindex", "-1"),
                    ("aria-label", &CellState::Empty.name()),
                    ("data-row", &row.to_string()),
                    ("data-col", &col.to_string()),
                    ("x", &x),
                    ("y", &y),
                    ("width", &w),
                    ("height", &w),
                ],
            )?;
            add(&row_group, &hit)?;
            hits.push(hit);
        }
        add(&grid, &row_group)?;
    }
    let ray_paths = [(); 3].map(|()| make(document, "path", &[("class", &format!("nq-ray {OFF}")), ("d", "")]));
    let [a, b, c] = ray_paths;
    let ray_elements = [a?, b?, c?];
    for ray in &ray_elements {
        add(&rays, ray)?;
    }
    let outline = make(
        document,
        "g",
        &[("class", &format!("nq-outline {OFF}")), ("aria-hidden", "true")],
    )?;
    let (inset, far) = (9.0, cell - 9.0);
    let (cross_in, cross_far) = (inset + 7.0, cell - inset - 7.0);
    add(
        &outline,
        &make(
            document,
            "rect",
            &[
                ("class", "nq-o-box"),
                ("x", &inset.to_string()),
                ("y", &inset.to_string()),
                ("width", &(far - inset).to_string()),
                ("height", &(far - inset).to_string()),
            ],
        )?,
    )?;
    add(
        &outline,
        &make(
            document,
            "path",
            &[
                ("class", "nq-o-cross"),
                (
                    "d",
                    &format!(
                        "M{cross_in} {cross_in}L{cross_far} {cross_far}M{cross_far} {cross_in}L{cross_in} {cross_far}"
                    ),
                ),
            ],
        )?,
    )?;
    let frame = make(
        document,
        "rect",
        &[
            ("class", &format!("nq-frame {OFF}")),
            ("aria-hidden", "true"),
            ("x", &(MARGIN - 6.0).to_string()),
            ("y", &(MARGIN - 6.0).to_string()),
            ("width", &(BOARD_SIZE + 12.0).to_string()),
            ("height", &(BOARD_SIZE + 12.0).to_string()),
        ],
    )?;
    let ring = make(document, "g", &[("class", &format!("nq-ring {OFF}")), ("aria-hidden", "true")])?;
    for (class, inset) in [("nq-ring-outer", 1.5), ("nq-ring-inner", 5.0)] {
        add(
            &ring,
            &make(
                document,
                "rect",
                &[
                    ("class", class),
                    ("x", &inset.to_string()),
                    ("y", &inset.to_string()),
                    ("width", &(cell - 2.0 * inset).to_string()),
                    ("height", &(cell - 2.0 * inset).to_string()),
                ],
            )?,
        )?;
    }
    for layer in [&fills, &rays, &glyphs_layer, &outline, &frame, &grid, &ring] {
        add(&root, layer)?;
    }
    Ok(BoardParts {
        size,
        root,
        glyphs,
        hits,
        rays: ray_elements,
        outline,
        frame,
        ring,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What assistive technology reads for the grid of squares.
pub(super) fn grid_label(size: usize) -> String {
    format!(
        "N-Queens board, {size} rows by {size} columns. The arrow keys move between squares, Home and End go to the \
         start and end of a row. By using Home and End with Control, you can navigate to the corners of the board. \
         The board only shows the search."
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes `active` the one square of `board` that has `tabindex=0`, and puts the focus ring on it, shown or hidden. It
/// is for a board that is not yet in the page, so that a failure leaves the page as it was.
///
/// # Errors
///
/// Returns `Err` if an attribute could not be written.
pub(super) fn arrange(board: &BoardParts, active: (usize, usize), ring_shown: bool) -> Result<(), String> {
    set(&board.hits[board.at(active.0, active.1)], "tabindex", "0")?;
    let (x, y) = cell_origin(board.size, active.0, active.1);
    set(&board.ring, "transform", &format!("translate({x} {y})"))?;
    set(&board.ring, "class", if ring_shown { "nq-ring" } else { "nq-ring nq-off" })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Writes the squares, the rays, the outline and the frame for `view` into `board`. When `previous` is the view that is
/// already drawn, only what differs is written. With `None`, every square is written.
///
/// # Errors
///
/// Returns `Err` if an attribute could not be written, part way through, which leaves the board partly written.
pub(super) fn apply(board: &BoardParts, previous: Option<&View>, view: &View) -> Result<(), String> {
    let size = board.size;
    let touched: Vec<(usize, usize)> = match previous {
        None => (0..size).flat_map(|row| (0..size).map(move |col| (row, col))).collect(),
        Some(before) => {
            let mut cells = before.busy_cells();
            for cell in view.busy_cells() {
                if !cells.contains(&cell) {
                    cells.push(cell);
                }
            }
            cells
        },
    };
    for (row, col) in touched {
        let now = view.cell_state(row, col);
        let before = previous.map(|p| p.cell_state(row, col));
        if before == Some(now) {
            continue;
        }
        let at = board.at(row, col);
        if before.is_none_or(|b| (b == CellState::Queen) != (now == CellState::Queen)) {
            set(
                &board.glyphs[at],
                "class",
                if now == CellState::Queen { "nq-glyph nq-on" } else { "nq-glyph" },
            )?;
        }
        set(&board.hits[at], "aria-label", &now.name())?;
    }
    apply_outline(board, previous, view)?;
    apply_rays(board, previous, view)?;
    if previous.is_none_or(|p| p.frame() != view.frame()) {
        set(&board.frame, "class", if view.frame() { "nq-frame" } else { "nq-frame nq-off" })?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(super) fn apply_outline(board: &BoardParts, previous: Option<&View>, view: &View) -> Result<(), String> {
    let (before, now) = (previous.and_then(View::outline), view.outline());
    if previous.is_some() && before == now {
        return Ok(());
    }
    match now {
        Some(outline) => {
            let (x, y) = cell_origin(board.size, outline.row, outline.col);
            set(&board.outline, "transform", &format!("translate({x} {y})"))?;
            let kind = match outline.kind {
                OutlineKind::Candidate => "nq-candidate",
                OutlineKind::Rejected => "nq-rejected",
                OutlineKind::Removed => "nq-removed",
            };
            set(&board.outline, "class", &format!("nq-outline {kind}"))
        },
        None => set(&board.outline, "class", &format!("nq-outline {OFF}")),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
pub(super) fn apply_rays(board: &BoardParts, previous: Option<&View>, view: &View) -> Result<(), String> {
    let (before, now) = (previous.and_then(View::rays), view.rays());
    if previous.is_some() && before == now {
        return Ok(());
    }
    let Some(rays) = now else {
        for ray in &board.rays {
            set(ray, "class", &format!("nq-ray {OFF}"))?;
        }
        return Ok(());
    };
    let moved = before.is_none_or(|b| (b.row, b.col) != (rays.row, rays.col));
    for (ray, line) in board.rays.iter().zip(LINES) {
        if moved {
            set(ray, "d", &ray_path(board.size, rays.row, rays.col, line))?;
        }
        let class = match rays.chosen {
            None => "nq-ray",
            Some(chosen) if chosen == line => "nq-ray nq-chosen",
            Some(_) => "nq-ray nq-muted",
        };
        set(ray, "class", class)?;
    }
    Ok(())
}
