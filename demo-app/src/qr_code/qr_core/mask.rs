//! The eight data mask patterns. Each says, for a row and a column, whether to invert the module there.

/// How many mask patterns there are.
pub(crate) const MASKS: usize = 8;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether the 0 to 7 mask pattern `mask` inverts the module at `row` and `col`, counted from zero at the top left.
///
/// Any other `mask` inverts nothing. It is a wrong index, which no caller passes, and no module is touched.
pub(crate) const fn inverts(mask: u8, row: usize, col: usize) -> bool {
    let product = row * col;
    match mask {
        0 => (row + col) % 2 == 0,
        1 => row % 2 == 0,
        2 => col % 3 == 0,
        3 => (row + col) % 3 == 0,
        4 => (row / 2 + col / 3) % 2 == 0,
        5 => product % 2 + product % 3 == 0,
        6 => (product % 2 + product % 3) % 2 == 0,
        7 => ((row + col) % 2 + product % 3) % 2 == 0,
        _ => false,
    }
}
