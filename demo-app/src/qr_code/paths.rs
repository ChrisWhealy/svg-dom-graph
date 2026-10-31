//! The `d` attribute of each consolidated path the viewport draws. Each is a string worked out from a [`BitGrid`], with
//! no DOM, so that a test can check it.
//!
//! One module is one unit square, so the viewport's `viewBox` is the size of the grid. A run of neighbouring modules
//! in a row is one rectangle, which keeps the strings short and leaves no seam between modules of the same colour.

use super::grid::{BitGrid, CellState, QrModuleKind};

/// How far an outline is drawn inside the edge of its module, so that neighbouring outlines do not merge.
const INSET: f64 = 0.12;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The three base paths: one for the modules without a colour, one for the light modules and one for the dark modules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BasePaths {
    pub(crate) unassigned: String,
    pub(crate) light: String,
    pub(crate) dark: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The three base paths of `grid`.
pub(crate) fn base_paths(grid: &BitGrid) -> BasePaths {
    let filled = |state: CellState| filled_path(grid, |row, col| grid.get(row, col) == Some(state));
    BasePaths {
        unassigned: filled(CellState::Unassigned),
        light: filled(CellState::Light),
        dark: filled(CellState::Dark),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The outline path of every module that has role `role`, or an empty path for `None`.
pub(crate) fn role_path(grid: &BitGrid, role: Option<QrModuleKind>) -> String {
    let Some(role) = role else { return String::new() };
    outline_path(grid.cells().filter(|&(row, col)| grid.role(row, col) == Some(role)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The outline path of the highlighted modules `cells`.
pub(crate) fn highlight_path(cells: &[(usize, usize)]) -> String {
    outline_path(cells.iter().copied())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The path of the dark modules of `grid`, moved right and down by `offset` modules, which is how an export leaves room
/// for the quiet zone.
pub(crate) fn dark_path_offset(grid: &BitGrid, offset: usize) -> String {
    filled_path_offset(grid, |row, col| grid.get(row, col) == Some(CellState::Dark), offset)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One rectangle for each horizontal run of modules that `wanted` accepts.
fn filled_path(grid: &BitGrid, wanted: impl Fn(usize, usize) -> bool) -> String {
    filled_path_offset(grid, wanted, 0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// [`filled_path`], with every rectangle moved right and down by `offset`.
fn filled_path_offset(grid: &BitGrid, wanted: impl Fn(usize, usize) -> bool, offset: usize) -> String {
    let mut d = String::new();
    for row in 0..grid.height() {
        let mut col = 0;
        while col < grid.width() {
            if !wanted(row, col) {
                col += 1;
                continue;
            }
            let start = col;
            while col < grid.width() && wanted(row, col) {
                col += 1;
            }
            let run = col - start;
            if !d.is_empty() {
                d.push(' ');
            }
            d.push_str(&format!("M{} {}h{run}v1h-{run}z", start + offset, row + offset));
        }
    }
    d
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One small square drawn just inside each of `cells`, so that the outlines of neighbouring modules stay apart.
fn outline_path(cells: impl Iterator<Item = (usize, usize)>) -> String {
    let side = 1.0 - 2.0 * INSET;
    cells
        .map(|(row, col)| format!("M{} {}h{side}v{side}h-{side}z", col as f64 + INSET, row as f64 + INSET))
        .collect::<Vec<_>>()
        .join(" ")
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The grid as text for a screen reader, one line a row. `.` has no colour, `0` is light and `1` is dark.
pub(crate) fn matrix_text(grid: &BitGrid) -> String {
    (0..grid.height())
        .map(|row| {
            (0..grid.width())
                .map(|col| match grid.get(row, col) {
                    Some(CellState::Light) => '0',
                    Some(CellState::Dark) => '1',
                    _ => '.',
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}
