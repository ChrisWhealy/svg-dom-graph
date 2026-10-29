//! The four penalty scores that decide which mask a symbol gets. A lower score is a symbol that is easier to read.
//!
//! The scores are of a finished candidate: every module has a colour, and the format information is in it. Each is
//! plain counting, in integers. N3 has a convention of its own, which is described there, and is the design's.

use super::{
    super::grid::{BitGrid, CellState},
    symbol::SymbolError,
};

/// What each kind of penalty is worth.
const N1_RUN: u32 = 3;
const N2_BLOCK: u32 = 3;
const N3_PATTERN: u32 = 40;
const N4_STEP: u32 = 10;

/// The shortest run of one colour that scores for N1.
const N1_MIN_RUN: usize = 5;

/// The two patterns of N3, each 11 modules, with 1 for dark. Each is the pattern of dark, light, dark, dark, dark,
/// light, dark, which is 1:1:3:1:1, beside four light modules, on one side or the other.
const N3_PATTERNS: [[bool; 11]; 2] = [
    [false, false, false, false, true, false, true, true, true, false, true],
    [true, false, true, true, true, false, true, false, false, false, false],
];

/// How many light modules every row and column has beyond each end for N3.
const N3_BORDER: usize = 4;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A square of modules that are dark or light, which is what the scores read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DarkMatrix {
    size: usize,
    dark: Vec<bool>,
}

impl DarkMatrix {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The matrix of `grid`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the grid is not square, or if a module has no colour yet, since it could not be scored.
    pub(crate) fn from_grid(grid: &BitGrid) -> Result<Self, SymbolError> {
        if grid.width() != grid.height() {
            return Err(SymbolError::Internal {
                stage: "scoring",
                detail: format!("a grid of {} by {} is not square", grid.width(), grid.height()),
            });
        }
        let mut dark = Vec::with_capacity(grid.width() * grid.height());
        for (row, col) in grid.cells() {
            match grid.get(row, col) {
                Some(CellState::Dark) => dark.push(true),
                Some(CellState::Light) => dark.push(false),
                _ => return Err(SymbolError::Uncoloured { row, col }),
            }
        }
        Ok(Self { size: grid.width(), dark })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Builds a matrix of `size` by `size` from `rows`, where `1` is dark. For tests that need a particular pattern.
    #[cfg(test)]
    pub(crate) fn from_rows(rows: &[&str]) -> Self {
        let size = rows.len();
        let dark = rows.iter().flat_map(|row| row.chars().map(|c| c == '1')).collect();
        assert!(rows.iter().all(|row| row.len() == size), "the matrix is not square");
        Self { size, dark }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    fn at(&self, row: usize, col: usize) -> bool {
        self.dark[row * self.size + col]
    }

    /// Every row, then every column, as a line of modules.
    fn lines(&self) -> impl Iterator<Item = Vec<bool>> + '_ {
        let rows = (0..self.size).map(|row| (0..self.size).map(|col| self.at(row, col)).collect());
        let cols = (0..self.size).map(|col| (0..self.size).map(|row| self.at(row, col)).collect());
        rows.chain(cols)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The four scores of one candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Penalty {
    pub(crate) n1: u32,
    pub(crate) n2: u32,
    pub(crate) n3: u32,
    pub(crate) n4: u32,
}

impl Penalty {
    pub(crate) const fn total(self) -> u32 {
        self.n1 + self.n2 + self.n3 + self.n4
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// All four scores of `matrix`.
pub(crate) fn penalty(matrix: &DarkMatrix) -> Penalty {
    Penalty {
        n1: n1(matrix),
        n2: n2(matrix),
        n3: n3(matrix),
        n4: n4(matrix),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// N1: each run of five or more modules of one colour in a row or column scores 3, and 1 more for each module past
/// five.
pub(crate) fn n1(matrix: &DarkMatrix) -> u32 {
    let mut score = 0;
    for line in matrix.lines() {
        let mut run = 0;
        let mut previous = None;
        for module in line.into_iter().map(Some).chain([None]) {
            if module == previous {
                run += 1;
                continue;
            }
            if run >= N1_MIN_RUN {
                score += N1_RUN + u32::try_from(run - N1_MIN_RUN).unwrap_or(0);
            }
            run = 1;
            previous = module;
        }
    }
    score
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// N2: each 2 by 2 block of one colour scores 3. Blocks that overlap each score.
pub(crate) fn n2(matrix: &DarkMatrix) -> u32 {
    let mut score = 0;
    for row in 0..matrix.size.saturating_sub(1) {
        for col in 0..matrix.size.saturating_sub(1) {
            let colour = matrix.at(row, col);
            if matrix.at(row, col + 1) == colour
                && matrix.at(row + 1, col) == colour
                && matrix.at(row + 1, col + 1) == colour
            {
                score += N2_BLOCK;
            }
        }
    }
    score
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// N3: each place in a row or column where one of the two 11 module patterns starts scores 40.
///
/// Every row and column has four light modules added at each end first, so that a pattern at the edge of the symbol has
/// the light modules it needs. Each start position counts, so patterns that overlap each score. A pattern with light
/// modules on both sides is found once by each of the two patterns, and scores 80.
///
/// This is the design's own convention. python-qrcode adds no light modules at the ends, and segno scores a pattern
/// with light modules on both sides once. So neither always gives this score, and the golden vectors record all three.
pub(crate) fn n3(matrix: &DarkMatrix) -> u32 {
    let mut score = 0;
    for line in matrix.lines() {
        let mut padded = vec![false; N3_BORDER];
        padded.extend(line);
        padded.extend([false; N3_BORDER]);
        for window in padded.windows(N3_PATTERNS[0].len()) {
            if N3_PATTERNS.iter().any(|pattern| window == pattern) {
                score += N3_PATTERN;
            }
        }
    }
    score
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// N4: 10 for every whole 5% that the share of dark modules is away from half.
///
/// With `d` dark modules of `t`, the share away from half, in percent, is `|2d - t| * 50 / t`. Whole steps of 5 are
/// `|2d - t| * 10 / t`, which is worked out in integers, so that rounding never decides it.
pub(crate) fn n4(matrix: &DarkMatrix) -> u32 {
    let total = matrix.dark.len();
    let dark = matrix.dark.iter().filter(|&&module| module).count();
    let steps = (2 * dark).abs_diff(total) * 10 / total.max(1);
    N4_STEP * u32::try_from(steps).unwrap_or(0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The mask with the least total, and the lowest of them if several tie. `totals` is indexed by mask.
///
/// Returns `None` if there are no totals.
pub(crate) fn choose_mask(totals: &[u32]) -> Option<usize> {
    totals
        .iter()
        .enumerate()
        .min_by_key(|&(mask, &total)| (total, mask))
        .map(|(mask, _)| mask)
}
