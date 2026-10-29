//! The scripted states the Phase 0 spike steps through. They are there to exercise the grid and the viewport, and the
//! bits placed are a fixed pseudo-random sequence, not the encoding of any text. Phase 2 replaces them with the real
//! encoder.

use super::{
    function::{FORMAT_BITS, format_copy_a, format_copy_b, placement_order, reserve_function_modules},
    grid::{BitGrid, DataRole, FunctionRole, GridError, SIZE},
};

/// The number of codewords in a Version 1-L symbol, and the first of the seven error correction ones.
pub(crate) const CODEWORDS: usize = 26;
const FIRST_ERROR_CORRECTION: usize = 19;

/// The format word for level L with mask 0, which the last step writes. It is a sample here, not worked out.
const SAMPLE_FORMAT: u16 = 0b111_0111_1100_0100;

/// Seeds the pseudo-random bits.
const SEED: u32 = 0x2545_F491;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One state of the spike: the whole grid, and the modules to highlight.
#[derive(Debug, Clone)]
pub(crate) struct SpikeStep {
    pub(crate) label: &'static str,
    pub(crate) grid: BitGrid,
    pub(crate) highlights: Vec<(usize, usize)>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The 208 sample bits, one for each module that holds a codeword bit.
fn sample_bits() -> Vec<bool> {
    let mut state = SEED;
    (0..CODEWORDS * 8)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 17;
            state ^= state << 5;
            (state >> 16) & 1 == 1
        })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every module that holds a bit of format information, in both copies.
fn format_cells() -> Vec<(usize, usize)> {
    (0..FORMAT_BITS)
        .flat_map(|bit| [format_copy_a(bit), format_copy_b(bit)])
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The five steps of the spike, each a complete grid.
///
/// # Errors
///
/// Returns `Err` if the grid refuses a change, which would mean the layout and the grid's rules disagree.
pub(crate) fn spike_steps() -> Result<Vec<SpikeStep>, GridError> {
    let mut grid = BitGrid::new(SIZE, SIZE);
    reserve_function_modules(&mut grid)?;
    let mut steps = vec![SpikeStep {
        label: "Function modules reserved. The format modules have no colour yet.",
        grid: grid.clone(),
        highlights: format_cells(),
    }];

    let order = placement_order(&grid);
    let bits = sample_bits();
    let place = |grid: &mut BitGrid, codeword: usize| -> Result<Vec<(usize, usize)>, GridError> {
        let role = if codeword < FIRST_ERROR_CORRECTION {
            DataRole::Data
        } else {
            DataRole::ErrorCorrection
        };
        let cells = &order[codeword * 8..codeword * 8 + 8];
        for (&(row, col), &bit) in cells.iter().zip(&bits[codeword * 8..]) {
            grid.assign(row, col, bit, role)?;
        }
        Ok(cells.to_vec())
    };

    let highlights = place(&mut grid, 0)?;
    steps.push(SpikeStep {
        label: "Codeword 1 placed, starting at the bottom right and moving up.",
        grid: grid.clone(),
        highlights,
    });

    let mut highlights = Vec::new();
    for codeword in 1..CODEWORDS {
        highlights = place(&mut grid, codeword)?;
    }
    steps.push(SpikeStep {
        label: "All 26 codewords placed. The last one is highlighted.",
        grid: grid.clone(),
        highlights,
    });

    for (row, col) in order.iter().copied().filter(|&(row, col)| (row + col) % 2 == 0) {
        grid.flip_data(row, col)?;
    }
    steps.push(SpikeStep {
        label: "Mask 0 applied to the data and error correction modules only.",
        grid: grid.clone(),
        highlights: Vec::new(),
    });

    for bit in 0..FORMAT_BITS {
        for (row, col) in [format_copy_a(bit), format_copy_b(bit)] {
            grid.fill_reserved(row, col, (SAMPLE_FORMAT >> bit) & 1 == 1, FunctionRole::Format)?;
        }
    }
    steps.push(SpikeStep {
        label: "Both copies of the format information coloured.",
        grid,
        highlights: format_cells(),
    });
    Ok(steps)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The step after `at` in a walk of `len` steps, where `None` is the unstarted state. The last step has no next.
pub(crate) fn next(at: Option<usize>, len: usize) -> Option<usize> {
    match at {
        None if len > 0 => Some(0),
        Some(index) if index + 1 < len => Some(index + 1),
        other => other,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The step before `at`, where the one before the first step is the unstarted state, which has no previous.
pub(crate) fn prev(at: Option<usize>) -> Option<usize> {
    at.and_then(|index| index.checked_sub(1))
}
