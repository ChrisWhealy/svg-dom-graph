//! The grid of modules: two packed bitsets for the colours and one role for each module, and the rules that keep them
//! consistent.

use super::{CellState, DataRole, FunctionRole, GridError, QrModuleKind};

/// The number of bits in one word of a bitset.
const WORD_BITS: usize = u64::BITS as usize;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A grid of modules, stored as two packed bitsets and one role per module.
///
/// A module is unassigned when its `assigned` bit is `0`, light when that is `1` and its `dark` bit is `0`, and dark
/// when both are `1`. The `dark` bit is never set on an unassigned module, and the unused high bits of each bitset's
/// last word stay `0`. `check_invariants`, which only the tests use, checks all of that.
///
/// The grid is a general rectangular matrix and knows nothing about Version 1. It takes any width and height. The role
/// types are QR terms, but no rule here depends on a size or a position. Only [`SIZE`](super::SIZE) and the code in
/// `function.rs` assume a 21 by 21 symbol. If a generic matrix is ever extracted for the library, it must not take
/// those with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BitGrid {
    width: usize,
    height: usize,
    assigned: Vec<u64>,
    dark: Vec<u64>,
    roles: Vec<Option<QrModuleKind>>,
}

impl BitGrid {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// A grid of `width` by `height` modules, none of which has a role or a colour.
    pub(crate) fn new(width: usize, height: usize) -> Self {
        let words = (width * height).div_ceil(WORD_BITS);
        Self {
            width,
            height,
            assigned: vec![0; words],
            dark: vec![0; words],
            roles: vec![None; width * height],
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(crate) const fn width(&self) -> usize {
        self.width
    }

    pub(crate) const fn height(&self) -> usize {
        self.height
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The colour of the module at `row`, `col`, or `None` if that is outside the grid.
    pub(crate) fn get(&self, row: usize, col: usize) -> Option<CellState> {
        let index = self.index(row, col)?;
        Some(match (self.bit(&self.assigned, index), self.bit(&self.dark, index)) {
            (false, _) => CellState::Unassigned,
            (true, false) => CellState::Light,
            (true, true) => CellState::Dark,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The role of the module at `row`, `col`, or `None` if it has none or is outside the grid.
    pub(crate) fn role(&self, row: usize, col: usize) -> Option<QrModuleKind> {
        self.index(row, col).and_then(|index| self.roles[index])
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Reserves the module at `row`, `col` for `role`, leaving it uncoloured.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the module is outside the grid, or already has a role or a colour.
    pub(crate) fn reserve(&mut self, row: usize, col: usize, role: FunctionRole) -> Result<(), GridError> {
        let index = self.index(row, col).ok_or(GridError::OutOfBounds { row, col })?;
        if self.roles[index].is_some() || self.bit(&self.assigned, index) {
            return Err(GridError::Occupied { row, col });
        }
        self.roles[index] = Some(QrModuleKind::Function(role));
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Checks that [`assign`](Self::assign) would accept the module at `row`, `col`, and changes nothing. It lets a
    /// caller check several modules first, so that it writes all of them or none.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the module is outside the grid, or already has a role or a colour.
    pub(crate) fn check_assignable(&self, row: usize, col: usize) -> Result<usize, GridError> {
        let index = self.index(row, col).ok_or(GridError::OutOfBounds { row, col })?;
        if self.roles[index].is_some() || self.bit(&self.assigned, index) {
            return Err(GridError::Occupied { row, col });
        }
        Ok(index)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Places one codeword bit: colours the module at `row`, `col` and gives it `role`, which is `dark` when `value`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the module is outside the grid, or already has a role or a colour. A reserved module has a
    /// role, so it can only be coloured by [`fill_reserved`](Self::fill_reserved).
    pub(crate) fn assign(&mut self, row: usize, col: usize, value: bool, role: DataRole) -> Result<(), GridError> {
        let index = self.check_assignable(row, col)?;
        self.roles[index] = Some(QrModuleKind::Payload(role));
        self.write(index, value);
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Colours a module that is reserved for `expected` and does not have a colour yet.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the module is outside the grid, is not reserved for `expected`, or is already coloured.
    pub(crate) fn fill_reserved(
        &mut self,
        row: usize,
        col: usize,
        value: bool,
        expected: FunctionRole,
    ) -> Result<(), GridError> {
        let index = self.index(row, col).ok_or(GridError::OutOfBounds { row, col })?;
        if self.roles[index] != Some(QrModuleKind::Function(expected)) || self.bit(&self.assigned, index) {
            return Err(GridError::NotReserved { row, col });
        }
        self.write(index, value);
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Inverts the colour of a placed data or error correction module, which is how a mask is applied.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the module is outside the grid, is uncoloured, or is a function module.
    pub(crate) fn flip_data(&mut self, row: usize, col: usize) -> Result<(), GridError> {
        let index = self.index(row, col).ok_or(GridError::OutOfBounds { row, col })?;
        let is_payload = matches!(self.roles[index], Some(QrModuleKind::Payload(_)));
        if !is_payload || !self.bit(&self.assigned, index) {
            return Err(GridError::NotFlippable { row, col });
        }
        let word = &mut self.dark[index / WORD_BITS];
        *word ^= 1 << (index % WORD_BITS);
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Every module as `(row, col)`, a row at a time from the top left.
    pub(crate) fn cells(&self) -> impl Iterator<Item = (usize, usize)> + use<> {
        let width = self.width;
        (0..self.width * self.height).map(move |index| (index / width, index % width))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many modules have colour `state`.
    pub(crate) fn count_state(&self, state: CellState) -> usize {
        self.cells().filter(|&(row, col)| self.get(row, col) == Some(state)).count()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many modules have a role that is a function role.
    #[cfg(test)]
    pub(crate) fn count_function(&self) -> usize {
        self.roles
            .iter()
            .filter(|role| matches!(role, Some(QrModuleKind::Function(_))))
            .count()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Checks every rule the grid keeps, and describes the first one that is broken.
    ///
    /// The rules are: a dark module is assigned, an assigned module has a role, a payload module is assigned, and the
    /// unused bits of each bitset are `0`.
    ///
    /// # Errors
    ///
    /// Returns `Err` with that description.
    #[cfg(test)]
    pub(crate) fn check_invariants(&self) -> Result<(), String> {
        let total = self.width * self.height;
        for index in 0..total {
            let (assigned, dark) = (self.bit(&self.assigned, index), self.bit(&self.dark, index));
            let (row, col) = (index / self.width, index % self.width);
            if dark && !assigned {
                return Err(format!("module ({row}, {col}) is dark but not assigned"));
            }
            if assigned && self.roles[index].is_none() {
                return Err(format!("module ({row}, {col}) is assigned but has no role"));
            }
            if matches!(self.roles[index], Some(QrModuleKind::Payload(_))) && !assigned {
                return Err(format!("module ({row}, {col}) is a payload module that is not assigned"));
            }
        }
        for (name, bits) in [("assigned", &self.assigned), ("dark", &self.dark)] {
            let Some(&last) = bits.last() else { continue };
            let used = total - (bits.len() - 1) * WORD_BITS;
            if used < WORD_BITS && last >> used != 0 {
                return Err(format!("the unused bits of the {name} bitset are not zero"));
            }
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The position of `row`, `col` in a row-major array, or `None` if it is outside the grid.
    fn index(&self, row: usize, col: usize) -> Option<usize> {
        (row < self.height && col < self.width).then_some(row * self.width + col)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Whether bit `index` of `bits` is set.
    fn bit(&self, bits: &[u64], index: usize) -> bool {
        bits[index / WORD_BITS] >> (index % WORD_BITS) & 1 == 1
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Marks module `index` as assigned, and dark or light.
    fn write(&mut self, index: usize, dark: bool) {
        let (word, mask) = (index / WORD_BITS, 1_u64 << (index % WORD_BITS));
        self.assigned[word] |= mask;
        if dark {
            self.dark[word] |= mask;
        } else {
            self.dark[word] &= !mask;
        }
    }
}
