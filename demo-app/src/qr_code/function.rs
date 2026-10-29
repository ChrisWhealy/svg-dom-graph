//! The fixed layout of a Version 1 symbol: the function patterns, where the format bits go, and the order the
//! codewords fill the rest.
//!
//! Phase 0 uses this to exercise [`BitGrid`] with the real counts, which are 233 function modules and 208 data
//! modules. Everything here is plain arithmetic on coordinates, with no DOM.

use super::grid::{BitGrid, FunctionRole, GridError, QrModuleKind, SIZE};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The row and column of the fixed dark module.
pub(crate) const DARK_MODULE: (usize, usize) = (13, 8);

/// The number of bits in the format information, which is written twice.
pub(crate) const FORMAT_BITS: usize = 15;

/// The top left module of each finder pattern: top left, top right and bottom left.
const FINDER_ORIGINS: [(usize, usize); 3] = [(0, 0), (0, SIZE - 7), (SIZE - 7, 0)];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One function module, with the colour it always has, or `None` for a format module whose colour depends on the mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FunctionModule {
    pub(crate) row: usize,
    pub(crate) col: usize,
    pub(crate) role: FunctionRole,
    pub(crate) dark: Option<bool>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where format bit `bit` goes in the first copy, around the top left finder pattern. Bit 0 is the least significant.
pub(crate) const fn format_copy_a(bit: usize) -> (usize, usize) {
    match bit {
        0..=5 => (bit, 8),
        6 => (7, 8),
        7 => (8, 8),
        8 => (8, 7),
        _ => (8, 14 - bit),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where format bit `bit` goes in the second copy, split between the top right and bottom left finder patterns.
pub(crate) const fn format_copy_b(bit: usize) -> (usize, usize) {
    if bit < 8 { (8, SIZE - 1 - bit) } else { (SIZE - 15 + bit, 8) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every function module of a Version 1 symbol, with no module listed twice: 233 in all.
pub(crate) fn function_modules() -> Vec<FunctionModule> {
    let mut modules = Vec::with_capacity(233);
    let mut add = |row: usize, col: usize, role: FunctionRole, dark: Option<bool>| {
        modules.push(FunctionModule { row, col, role, dark });
    };

    for (row0, col0) in FINDER_ORIGINS {
        // The pattern is a dark ring, a light ring and a dark 3 x 3 centre, so the colour depends only on the ring.
        for row in row0..row0 + 7 {
            for col in col0..col0 + 7 {
                let ring = (row - row0).abs_diff(3).max((col - col0).abs_diff(3));
                add(row, col, FunctionRole::Finder, Some(ring != 2));
            }
        }
        // The separator is the light border between the pattern and the rest of the symbol, where it is in the grid.
        for row in (row0 as isize - 1)..=(row0 as isize + 7) {
            for col in (col0 as isize - 1)..=(col0 as isize + 7) {
                let in_grid = (0..SIZE as isize).contains(&row) && (0..SIZE as isize).contains(&col);
                let in_pattern = (row0 as isize..row0 as isize + 7).contains(&row)
                    && (col0 as isize..col0 as isize + 7).contains(&col);
                if in_grid && !in_pattern {
                    add(row as usize, col as usize, FunctionRole::Separator, Some(false));
                }
            }
        }
    }

    // The timing patterns alternate dark and light, and start dark. They run between the finder patterns.
    for position in 8..SIZE - 8 {
        add(6, position, FunctionRole::Timing, Some(position % 2 == 0));
        add(position, 6, FunctionRole::Timing, Some(position % 2 == 0));
    }

    add(DARK_MODULE.0, DARK_MODULE.1, FunctionRole::DarkModule, Some(true));

    for bit in 0..FORMAT_BITS {
        for (row, col) in [format_copy_a(bit), format_copy_b(bit)] {
            add(row, col, FunctionRole::Format, None);
        }
    }
    modules
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Reserves every function module of `grid`, and colours all of them except the format modules.
///
/// The grid is changed only if every module can be reserved.
///
/// # Errors
///
/// Returns `Err` if any module is already reserved or coloured.
pub(crate) fn reserve_function_modules(grid: &mut BitGrid) -> Result<(), GridError> {
    let mut staged = grid.clone();
    for module in function_modules() {
        staged.reserve(module.row, module.col, module.role)?;
        if let Some(dark) = module.dark {
            staged.fill_reserved(module.row, module.col, dark, module.role)?;
        }
    }
    *grid = staged;
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The modules that hold codeword bits, in the order the bits fill them.
///
/// The symbol is read in pairs of columns from the right, and the first pair is read upwards. Each later pair runs the
/// other way, and the vertical timing pattern in column 6 is skipped. Function modules are left out.
pub(crate) fn placement_order(grid: &BitGrid) -> Vec<(usize, usize)> {
    let mut order = Vec::with_capacity(208);
    let mut right = SIZE - 1;
    loop {
        if right == 6 {
            right = 5;
        }
        let upwards = (right + 1) & 2 == 0;
        for step in 0..SIZE {
            let row = if upwards { SIZE - 1 - step } else { step };
            for col in [right, right - 1] {
                if !matches!(grid.role(row, col), Some(QrModuleKind::Function(_))) {
                    order.push((row, col));
                }
            }
        }
        if right < 3 {
            return order;
        }
        right -= 2;
    }
}
