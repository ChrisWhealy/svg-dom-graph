//! From the 26 codewords to the symbol: the function patterns, the placement, the eight candidates and the choice.
//!
//! Every stage is kept, so that each can be shown. The function modules go in first, with the format modules reserved
//! and not coloured. Then each codeword is placed. Each of the eight masks is then applied to a copy of the placed
//! symbol, its own format information is written, and the copy is scored. The least total wins, and the lowest mask
//! wins a tie.

use super::{
    super::{
        function::{FORMAT_BITS, format_copy_a, format_copy_b, placement_order, reserve_function_modules},
        grid::{BitGrid, DataRole, FunctionRole, GridError, SIZE},
    },
    DATA_CODEWORDS, Encoded, TOTAL_CODEWORDS,
    format::format_word,
    mask::{MASKS, inverts},
    score::{DarkMatrix, Penalty, choose_mask, penalty},
};
use std::fmt;

/// How many modules hold the bits of codewords, in a Version 1 symbol.
const CODEWORD_MODULES: usize = TOTAL_CODEWORDS * 8;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a symbol could not be made. No symbol is made if one of these is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SymbolError {
    /// The grid refused a change, which means the layout and the grid's rules disagree.
    Grid(GridError),
    /// A mask number that is not 0 to 7.
    InvalidMask { mask: u8 },
    /// A module had no colour when it was needed.
    Uncoloured { row: usize, col: usize },
    /// A stage found a count or a state that its own arithmetic rules out.
    Internal { stage: &'static str, detail: String },
}

impl From<GridError> for SymbolError {
    fn from(error: GridError) -> Self {
        Self::Grid(error)
    }
}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grid(error) => write!(f, "internal error in the grid: {error}"),
            Self::InvalidMask { mask } => write!(f, "{mask} is not a mask: they are numbered 0 to 7"),
            Self::Uncoloured { row, col } => write!(f, "module ({row}, {col}) has no colour and cannot be scored"),
            Self::Internal { stage, detail } => write!(f, "internal error in the {stage} stage: {detail}"),
        }
    }
}

impl std::error::Error for SymbolError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One of the eight candidates: the placed symbol with one mask applied and that mask's format information, scored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub(crate) mask: u8,
    pub(crate) format_bits: u16,
    pub(crate) grid: BitGrid,
    pub(crate) penalty: Penalty,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every stage of the symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Symbol {
    /// The function patterns, with the format modules reserved and not coloured, and no codeword placed.
    pub(crate) functions: BitGrid,
    /// The modules the codewords fill, in the order they fill them.
    pub(crate) placement: Vec<(usize, usize)>,
    /// All 26 codewords placed, and no mask applied.
    pub(crate) placed: BitGrid,
    /// The eight candidates, in the order of their masks.
    pub(crate) candidates: Vec<Candidate>,
    /// The index of the winning candidate, which is also its mask.
    pub(crate) chosen: usize,
}

impl Symbol {
    /// The finished symbol, which is the winning candidate.
    pub(crate) fn finished(&self) -> &Candidate {
        &self.candidates[self.chosen]
    }

    /// The finished symbol as 21 rows of text, `1` for dark and `0` for light, without a quiet zone.
    pub(crate) fn rows(&self) -> Vec<String> {
        rows_of(&self.finished().grid)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// `grid` as rows of text: `1` for a dark module, `0` for a light one and `.` for one with no colour.
pub(crate) fn rows_of(grid: &BitGrid) -> Vec<String> {
    use super::super::grid::CellState;
    (0..grid.height())
        .map(|row| {
            (0..grid.width())
                .map(|col| match grid.get(row, col) {
                    Some(CellState::Dark) => '1',
                    Some(CellState::Light) => '0',
                    _ => '.',
                })
                .collect()
        })
        .collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Places the bits of `word`, most significant first, in the modules of codeword `index` of `order`. A data codeword
/// gives its modules the role of data, and one of the last seven gives them the role of error correction.
///
/// All eight modules are checked before any is written, so a codeword is placed whole or not at all, and a refusal
/// leaves `grid` as it was. Nothing is copied to do that.
///
/// # Errors
///
/// Returns `Err` if `index` is past the end of `order`, if a module is outside the grid, already placed or a function
/// module, or if `order` lists a module twice within the codeword.
pub(crate) fn place_codeword(
    grid: &mut BitGrid,
    order: &[(usize, usize)],
    index: usize,
    word: u8,
) -> Result<(), SymbolError> {
    let modules = order.get(index * 8..index * 8 + 8).ok_or_else(|| SymbolError::Internal {
        stage: "placement",
        detail: format!("there is no codeword {index} in {} modules", order.len()),
    })?;
    for (position, &(row, col)) in modules.iter().enumerate() {
        grid.check_assignable(row, col)?;
        // Each is free now, but a module listed twice would not be free for the second of its two bits.
        if modules[..position].contains(&(row, col)) {
            return Err(GridError::Occupied { row, col }.into());
        }
    }
    let role = if index < DATA_CODEWORDS { DataRole::Data } else { DataRole::ErrorCorrection };
    for (position, &(row, col)) in modules.iter().enumerate() {
        grid.assign(row, col, word >> (7 - position) & 1 == 1, role)?;
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One candidate: `placed` with mask `mask` applied to the data and error correction modules, that mask's own format
/// information written, and the result scored.
///
/// # Errors
///
/// Returns `Err` if `mask` is not 0 to 7, which is checked before anything is done, if the grid refuses a change, or if
/// a module is still without a colour.
pub(crate) fn make_candidate(placed: &BitGrid, order: &[(usize, usize)], mask: u8) -> Result<Candidate, SymbolError> {
    let format_bits = format_word(mask).ok_or(SymbolError::InvalidMask { mask })?;
    let mut grid = placed.clone();
    for &(row, col) in order {
        if inverts(mask, row, col) {
            grid.flip_data(row, col)?;
        }
    }
    for bit in 0..FORMAT_BITS {
        let value = format_bits >> bit & 1 == 1;
        for (row, col) in [format_copy_a(bit), format_copy_b(bit)] {
            grid.fill_reserved(row, col, value, FunctionRole::Format)?;
        }
    }
    let penalty = penalty(&DarkMatrix::from_grid(&grid)?);
    Ok(Candidate {
        mask,
        format_bits,
        grid,
        penalty,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Makes the symbol of `encoded`.
///
/// # Errors
///
/// Returns `Err` if a count is not what a Version 1 symbol has, or the grid refuses a change. Either means the layout
/// is wrong, and no symbol is made.
pub(crate) fn build_symbol(encoded: &Encoded) -> Result<Symbol, SymbolError> {
    let mut functions = BitGrid::new(SIZE, SIZE);
    reserve_function_modules(&mut functions)?;
    let placement = placement_order(&functions);
    if placement.len() != CODEWORD_MODULES {
        return Err(SymbolError::Internal {
            stage: "placement",
            detail: format!("{} modules for codewords, not {CODEWORD_MODULES}", placement.len()),
        });
    }

    let mut placed = functions.clone();
    for (index, &word) in encoded.codewords().iter().enumerate() {
        place_codeword(&mut placed, &placement, index, word)?;
    }

    let candidates = (0..MASKS as u8)
        .map(|mask| make_candidate(&placed, &placement, mask))
        .collect::<Result<Vec<_>, _>>()?;
    let totals: Vec<u32> = candidates.iter().map(|candidate| candidate.penalty.total()).collect();
    let chosen = choose_mask(&totals).ok_or_else(|| SymbolError::Internal {
        stage: "selection",
        detail: "there are no candidates".to_owned(),
    })?;
    Ok(Symbol {
        functions,
        placement,
        placed,
        candidates,
        chosen,
    })
}
