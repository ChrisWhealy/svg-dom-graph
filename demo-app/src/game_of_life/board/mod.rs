//! The rules of Conway's Game of Life, and the patterns that the demo can place on a board. Pure: nothing here touches
//! the DOM.
//!
//! A board is a grid of cells that are alive or dead. Each step, every cell looks at its eight neighbours and decides
//! the next generation at once:
//!
//! - A live cell with two or three live neighbours survives. With fewer it dies of loneliness, and with more it dies of
//!   crowding.
//! - A dead cell with exactly three live neighbours is born.
//!
//! The edges either wrap round, so that the board is the surface of a doughnut, or stop, so that beyond them is dead.

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A pattern of live cells that can be put on a board, drawn as text: `O` is a live cell and `.` is a dead one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Pattern {
    /// What the page calls the pattern, which is the label of its button.
    pub(crate) name: &'static str,
    /// What the pattern is called in the code of the page: the action of its button, without its prefix.
    pub(crate) id: &'static str,
    /// The rows from the top, all as long as the longest.
    pub(crate) rows: &'static [&'static str],
}

impl Pattern {
    /// How many columns the pattern is across.
    pub(crate) fn width(&self) -> usize {
        self.rows.iter().map(|row| row.len()).max().unwrap_or(0)
    }

    /// How many rows the pattern is down.
    pub(crate) fn height(&self) -> usize {
        self.rows.len()
    }

    /// The places of the live cells, as columns and rows from the top left of the pattern.
    pub(crate) fn cells(&self) -> Vec<(usize, usize)> {
        self.rows
            .iter()
            .enumerate()
            .flat_map(|(row, text)| {
                text.bytes()
                    .enumerate()
                    .filter(|&(_, c)| c == b'O')
                    .map(move |(col, _)| (col, row))
            })
            .collect()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The patterns of the page, in the order of their buttons.
pub(crate) const PATTERNS: [Pattern; 5] = [
    Pattern {
        name: "Glider",
        id: "glider",
        rows: &[".O.", "..O", "OOO"],
    },
    Pattern {
        name: "Blinker",
        id: "blinker",
        rows: &["OOO"],
    },
    Pattern {
        name: "R-pentomino",
        id: "r-pentomino",
        rows: &[".OO", "OO.", ".O."],
    },
    Pattern {
        name: "Acorn",
        id: "acorn",
        rows: &[".O.....", "...O...", "OO..OOO"],
    },
    Pattern {
        name: "Glider gun",
        id: "glider-gun",
        rows: &[
            "........................O...........",
            "......................O.O...........",
            "............OO......OO............OO",
            "...........O...O....OO............OO",
            "OO........O.....O...OO..............",
            "OO........O...O.OO....O.O...........",
            "..........O.....O.......O...........",
            "...........O...O....................",
            "............OO......................",
        ],
    },
];

/// The pattern with the id `id`, if there is one.
pub(crate) fn pattern(id: &str) -> Option<&'static Pattern> {
    PATTERNS.iter().find(|pattern| pattern.id == id)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The spaceships that the page can add to a board, in the order of their buttons. Each is a pattern that repeats
/// itself every 4 generations, two cells further to the right, so it flies across the board from the left. They are the
/// light, medium and heavy weight spaceships of Conway, which differ in how many cells they have, and are drawn here
/// the other way round from how they are usually shown, so that they fly to the right.
pub(crate) const SPACESHIPS: [Pattern; 3] = [
    Pattern {
        name: "Light spaceship",
        id: "lwss",
        rows: &["O..O.", "....O", "O...O", ".OOOO"],
    },
    Pattern {
        name: "Medium spaceship",
        id: "mwss",
        rows: &["..O...", "O...O.", ".....O", "O....O", ".OOOOO"],
    },
    Pattern {
        name: "Heavy spaceship",
        id: "hwss",
        rows: &["..OO...", "O....O.", "......O", "O.....O", ".OOOOOO"],
    },
];

/// The spaceship with the id `id`, if there is one.
pub(crate) fn spaceship(id: &str) -> Option<&'static Pattern> {
    SPACESHIPS.iter().find(|ship| ship.id == id)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A small random number generator, so that a random board is the same for the same seed, and can be tested. It is the
/// xorshift generator of Marsaglia, which is fast and good enough to scatter cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rng(u32);

impl Rng {
    /// A generator for `seed`. A seed of 0 is changed, since the generator would never leave it.
    pub(crate) fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x9E37_79B9 } else { seed })
    }

    /// The next number, from 0 up to but not including 1.
    pub(crate) fn next_unit(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        f64::from(x) / (f64::from(u32::MAX) + 1.0)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The generation of cells, on a grid of `cols` by `rows`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Board {
    cols: usize,
    rows: usize,
    /// Row by row from the top left.
    cells: Vec<bool>,
    wrap: bool,
}

impl Board {
    /// An empty board of `cols` by `rows`, whose edges wrap round if `wrap` is true.
    ///
    /// A board is at least one cell across and one down, since where the edges wrap a cell is counted by its place
    /// modulo the size, which a size of nothing has no answer for. A size of 0 is made 1, so that no method can fail on
    /// a board, and [`Board::cols`] and [`Board::rows`] say what the size is. The page uses a board of 40 by 30.
    pub(crate) fn new(cols: usize, rows: usize, wrap: bool) -> Self {
        let (cols, rows) = (cols.max(1), rows.max(1));
        Self {
            cols,
            rows,
            cells: vec![false; cols * rows],
            wrap,
        }
    }

    pub(crate) fn cols(&self) -> usize {
        self.cols
    }

    pub(crate) fn rows(&self) -> usize {
        self.rows
    }

    /// Whether the edges wrap round.
    pub(crate) fn wraps(&self) -> bool {
        self.wrap
    }

    /// Makes the edges wrap round or stop. The cells stay as they are.
    pub(crate) fn set_wrap(&mut self, wrap: bool) {
        self.wrap = wrap;
    }

    /// Whether the cell at `col` and `row` is alive. A place that is not on the board is dead.
    pub(crate) fn get(&self, col: usize, row: usize) -> bool {
        col < self.cols && row < self.rows && self.cells[row * self.cols + col]
    }

    /// Makes the cell at `col` and `row` alive or dead. A place that is not on the board is ignored.
    pub(crate) fn set(&mut self, col: usize, row: usize, alive: bool) {
        if col < self.cols && row < self.rows {
            self.cells[row * self.cols + col] = alive;
        }
    }

    /// Changes the cell at `col` and `row`, and says whether it is now alive.
    pub(crate) fn toggle(&mut self, col: usize, row: usize) -> bool {
        let alive = !self.get(col, row);
        self.set(col, row, alive);
        alive
    }

    /// How many cells are alive.
    pub(crate) fn population(&self) -> usize {
        self.cells.iter().filter(|&&alive| alive).count()
    }

    /// Kills every cell.
    pub(crate) fn clear(&mut self) {
        self.cells.fill(false);
    }

    /// The places of the live cells as columns and rows, row by row from the top left. The page draws them as a path,
    /// so this is for the tests, which compare whole boards.
    #[cfg(test)]
    pub(crate) fn live_cells(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.cells
            .iter()
            .enumerate()
            .filter(|&(_, &alive)| alive)
            .map(|(i, _)| (i % self.cols, i / self.cols))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// How many of the eight neighbours of the cell at `col` and `row` are alive. Where the edges wrap, a cell on one
    /// edge is next to the cells on the opposite edge. Where they stop, a place beyond the edge is dead.
    pub(crate) fn live_neighbours(&self, col: usize, row: usize) -> u8 {
        let mut count = 0;
        for (dc, dr) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
            if let Some((c, r)) = self.neighbour(col, row, dc, dr) {
                count += u8::from(self.cells[r * self.cols + c]);
            }
        }
        count
    }

    /// The place that is `dc` columns and `dr` rows from `col` and `row`, or `None` if that is beyond an edge that
    /// stops. On a board that is only one cell across, a wrapped neighbour can be the cell itself, which counts.
    fn neighbour(&self, col: usize, row: usize, dc: isize, dr: isize) -> Option<(usize, usize)> {
        let place = |at: usize, delta: isize, size: usize| -> Option<usize> {
            let moved = at as isize + delta;
            if self.wrap {
                Some(moved.rem_euclid(size as isize) as usize)
            } else {
                (0..size as isize).contains(&moved).then_some(moved as usize)
            }
        };
        Some((place(col, dc, self.cols)?, place(row, dr, self.rows)?))
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Moves on to the next generation: every cell changes at once, by what the generation before it had.
    pub(crate) fn step(&mut self) {
        let mut next = vec![false; self.cells.len()];
        for row in 0..self.rows {
            for col in 0..self.cols {
                let neighbours = self.live_neighbours(col, row);
                let alive = self.cells[row * self.cols + col];
                next[row * self.cols + col] = matches!((alive, neighbours), (true, 2 | 3) | (false, 3));
            }
        }
        self.cells = next;
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Puts `pattern` on the board with its top left corner at `col` and `row`. Where the edges wrap, a part that goes
    /// past one comes back at the other. Where they stop, it is cut off. Cells that are alive stay so.
    pub(crate) fn place(&mut self, pattern: &Pattern, col: usize, row: usize) {
        for (dc, dr) in pattern.cells() {
            let (c, r) = (col + dc, row + dr);
            if self.wrap {
                self.set(c % self.cols, r % self.rows, true);
            } else {
                self.set(c, r, true);
            }
        }
    }

    /// Puts `pattern` in the middle of the board, as near as a whole number of cells allows.
    pub(crate) fn place_centred(&mut self, pattern: &Pattern) {
        let col = self.cols.saturating_sub(pattern.width()) / 2;
        let row = self.rows.saturating_sub(pattern.height()) / 2;
        self.place(pattern, col, row);
    }

    /// Adds `pattern` at the left edge, level with `row`, so that a spaceship flies into the board from the left. The
    /// pattern is moved up or down as far as it needs to be to be wholly on the board, and the cells that are alive
    /// stay so.
    pub(crate) fn place_level_with(&mut self, pattern: &Pattern, row: usize) {
        let top = row
            .saturating_sub(pattern.height() / 2)
            .min(self.rows.saturating_sub(pattern.height()));
        self.place(pattern, 0, top);
    }

    /// Kills every cell, and then gives each a chance of `density`, from 0 to 1, of being alive, from `seed`.
    pub(crate) fn randomise(&mut self, seed: u32, density: f64) {
        let mut rng = Rng::new(seed);
        for alive in &mut self.cells {
            *alive = rng.next_unit() < density;
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How many generations are due after `elapsed_ms` has passed, and how much of the time that was not used is carried to
/// the next frame. A generation is due every `period_ms`. No more than `most` are due at once, so that a page that has
/// been in the background for a while does not spend a long time catching up, and then the time that is left is let go.
pub(crate) fn steps_due(carried_ms: f64, elapsed_ms: f64, period_ms: f64, most: u32) -> (u32, f64) {
    if period_ms.is_nan() || period_ms <= 0.0 || !carried_ms.is_finite() || !elapsed_ms.is_finite() {
        return (0, 0.0);
    }
    let total = (carried_ms + elapsed_ms.max(0.0)).max(0.0);
    let due = (total / period_ms).floor();
    if due >= f64::from(most) {
        (most, 0.0)
    } else {
        (due as u32, total - due * period_ms)
    }
}

/// How long there is between two generations, in milliseconds, at `speed` generations in each second.
pub(crate) fn period_ms(speed: f64) -> f64 {
    1000.0 / speed.max(f64::MIN_POSITIVE)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
