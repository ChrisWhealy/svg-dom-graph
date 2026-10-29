//! The matrix of a QR symbol: what colour each module is, and what each module is for.
//!
//! Colour and role are separate. A module can hold a role before it holds a colour: the format modules are reserved
//! when the function patterns are drawn, but their colours are not known until a mask is chosen. A data slot holds
//! neither until a codeword is placed in it. Every mutation checks its rules first and then writes, so a rejected call
//! leaves the grid exactly as it was.

use std::fmt;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The width and height of a Version 1 symbol, in modules. [`BitGrid`] does not use it, and only callers that build a
/// symbol do.
pub(crate) const SIZE: usize = 21;

/// The number of bits in one word of a bitset.
const WORD_BITS: usize = u64::BITS as usize;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The colour of one module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellState {
    /// No colour has been chosen yet. This is not the same as light.
    Unassigned,
    Light,
    Dark,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A module that belongs to a fixed pattern, not to the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FunctionRole {
    Finder,
    Separator,
    Timing,
    DarkModule,
    Format,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A module that holds one bit of a codeword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DataRole {
    Data,
    ErrorCorrection,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a module is for. The two halves are separate types so that [`BitGrid::assign`] cannot be given a function role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QrModuleKind {
    Function(FunctionRole),
    Payload(DataRole),
}

impl QrModuleKind {
    /// Every role, in the order the legend lists them.
    pub(crate) const ALL: [Self; 7] = [
        Self::Function(FunctionRole::Finder),
        Self::Function(FunctionRole::Separator),
        Self::Function(FunctionRole::Timing),
        Self::Function(FunctionRole::DarkModule),
        Self::Function(FunctionRole::Format),
        Self::Payload(DataRole::Data),
        Self::Payload(DataRole::ErrorCorrection),
    ];

    /// The name shown in the legend and read by assistive technology.
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Function(FunctionRole::Finder) => "Finder",
            Self::Function(FunctionRole::Separator) => "Separator",
            Self::Function(FunctionRole::Timing) => "Timing",
            Self::Function(FunctionRole::DarkModule) => "Dark module",
            Self::Function(FunctionRole::Format) => "Format",
            Self::Payload(DataRole::Data) => "Data",
            Self::Payload(DataRole::ErrorCorrection) => "Error correction",
        }
    }

    /// What this role is for, in words for someone who has never met a QR code. It is shown when the role is chosen.
    pub(crate) const fn explanation(self) -> &'static str {
        match self {
            Self::Function(FunctionRole::Finder) => {
                "The three large squares in the corners. Each is a dark ring, a light ring and a dark 3 by 3 centre. \
                 A scanner looks for them first, to find the code and to see its size. The fourth corner has none, \
                 so the scanner can tell which way up the code is."
            },
            Self::Function(FunctionRole::Separator) => {
                "A border of light modules, one module wide, along the inner sides of each finder pattern. It keeps \
                 the dark ring of a finder apart from the dark modules of the data beside it, so that the two are \
                 not read as one shape."
            },
            Self::Function(FunctionRole::Timing) => {
                "Two lines of alternating dark and light modules between the finder patterns: one along row 6 and \
                 one down column 6. Both start and end dark. A scanner counts along them to measure the grid, which \
                 corrects for a code that is stretched or photographed at an angle."
            },
            Self::Function(FunctionRole::DarkModule) => {
                "One module that is always dark, at row 13 and column 8, beside the bottom left finder. The standard \
                 reserves it, so it is never part of the message and no mask ever changes it."
            },
            Self::Function(FunctionRole::Format) => {
                "Fifteen bits that say which error correction level and which mask the symbol uses. A scanner reads \
                 them before anything else. They are written twice, once around the top left finder and once split \
                 between the other two, so that damage to one copy can be survived. They have no colour until the \
                 mask is chosen."
            },
            Self::Payload(DataRole::Data) => {
                "The modules that hold the message: the mode, the length, the bytes, the terminator and the \
                 padding. 19 codewords of 8 bits fill 152 modules. They are placed two columns at a time, starting at \
                 the bottom right and zig-zagging up, then down, then up again."
            },
            Self::Payload(DataRole::ErrorCorrection) => {
                "Seven extra codewords, 56 modules, worked out from the data with Reed-Solomon coding. They let a \
                 scanner rebuild the message when part of the symbol is dirty, torn or hidden. Level L is designed to \
                 recover about 7% of the symbol."
            },
        }
    }

    /// The value of the legend's own option for this role, which [`from_key`](Self::from_key) reads back.
    pub(crate) const fn key(self) -> &'static str {
        match self {
            Self::Function(FunctionRole::Finder) => "finder",
            Self::Function(FunctionRole::Separator) => "separator",
            Self::Function(FunctionRole::Timing) => "timing",
            Self::Function(FunctionRole::DarkModule) => "dark-module",
            Self::Function(FunctionRole::Format) => "format",
            Self::Payload(DataRole::Data) => "data",
            Self::Payload(DataRole::ErrorCorrection) => "error-correction",
        }
    }

    /// The role whose [`key`](Self::key) is `key`, or `None` for any other text, such as the legend's "none" option.
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.key() == key)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a [`BitGrid`] refused a change. The grid is unchanged whenever one of these is returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GridError {
    OutOfBounds {
        row: usize,
        col: usize,
    },
    /// The module already has a role or a colour, so it cannot be reserved or assigned.
    Occupied {
        row: usize,
        col: usize,
    },
    /// The module is not reserved for the role the caller expected.
    NotReserved {
        row: usize,
        col: usize,
    },
    /// The module is not a coloured data or error correction module, so it cannot be flipped.
    NotFlippable {
        row: usize,
        col: usize,
    },
}

impl fmt::Display for GridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::OutOfBounds { row, col } => write!(f, "module ({row}, {col}) is outside the grid"),
            Self::Occupied { row, col } => write!(f, "module ({row}, {col}) already has a role or a colour"),
            Self::NotReserved { row, col } => write!(f, "module ({row}, {col}) is not reserved for that role"),
            Self::NotFlippable { row, col } => write!(f, "module ({row}, {col}) holds no coloured payload bit"),
        }
    }
}

impl std::error::Error for GridError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A grid of modules, stored as two packed bitsets and one role per module.
///
/// A module is unassigned when its `assigned` bit is `0`, light when that is `1` and its `dark` bit is `0`, and dark
/// when both are `1`. The `dark` bit is never set on an unassigned module, and the unused high bits of each bitset's
/// last word stay `0`. `check_invariants`, which only the tests use, checks all of that.
///
/// The grid is a general rectangular matrix and knows nothing about Version 1. It takes any width and height. The role
/// types are QR terms, but no rule here depends on a size or a position. Only [`SIZE`] and the code in `function.rs`
/// assume a 21 by 21 symbol. If a generic matrix is ever extracted for the library, it must not take those with it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BitGrid {
    width: usize,
    height: usize,
    assigned: Vec<u64>,
    dark: Vec<u64>,
    roles: Vec<Option<QrModuleKind>>,
}

impl BitGrid {
    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub(crate) const fn width(&self) -> usize {
        self.width
    }

    pub(crate) const fn height(&self) -> usize {
        self.height
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The colour of the module at `row`, `col`, or `None` if that is outside the grid.
    pub(crate) fn get(&self, row: usize, col: usize) -> Option<CellState> {
        let index = self.index(row, col)?;
        Some(match (self.bit(&self.assigned, index), self.bit(&self.dark, index)) {
            (false, _) => CellState::Unassigned,
            (true, false) => CellState::Light,
            (true, true) => CellState::Dark,
        })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The role of the module at `row`, `col`, or `None` if it has none or is outside the grid.
    pub(crate) fn role(&self, row: usize, col: usize) -> Option<QrModuleKind> {
        self.index(row, col).and_then(|index| self.roles[index])
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Places one codeword bit: colours the module at `row`, `col` and gives it `role`, which is `dark` when `value`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the module is outside the grid, or already has a role or a colour. A reserved module has a
    /// role, so it can only be coloured by [`fill_reserved`](Self::fill_reserved).
    pub(crate) fn assign(&mut self, row: usize, col: usize, value: bool, role: DataRole) -> Result<(), GridError> {
        let index = self.index(row, col).ok_or(GridError::OutOfBounds { row, col })?;
        if self.roles[index].is_some() || self.bit(&self.assigned, index) {
            return Err(GridError::Occupied { row, col });
        }
        self.roles[index] = Some(QrModuleKind::Payload(role));
        self.write(index, value);
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Every module as `(row, col)`, a row at a time from the top left.
    pub(crate) fn cells(&self) -> impl Iterator<Item = (usize, usize)> + use<> {
        let width = self.width;
        (0..self.width * self.height).map(move |index| (index / width, index % width))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many modules have colour `state`.
    pub(crate) fn count_state(&self, state: CellState) -> usize {
        self.cells().filter(|&(row, col)| self.get(row, col) == Some(state)).count()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many modules have a role that is a function role.
    #[cfg(test)]
    pub(crate) fn count_function(&self) -> usize {
        self.roles
            .iter()
            .filter(|role| matches!(role, Some(QrModuleKind::Function(_))))
            .count()
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
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

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The position of `row`, `col` in a row-major array, or `None` if it is outside the grid.
    fn index(&self, row: usize, col: usize) -> Option<usize> {
        (row < self.height && col < self.width).then_some(row * self.width + col)
    }

    /// Whether bit `index` of `bits` is set.
    fn bit(&self, bits: &[u64], index: usize) -> bool {
        bits[index / WORD_BITS] >> (index % WORD_BITS) & 1 == 1
    }

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
