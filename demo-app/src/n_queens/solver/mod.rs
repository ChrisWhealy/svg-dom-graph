//! The N-Queens search as a list of events, and a snapshot of the board after each one. Pure: nothing here touches the
//! DOM.
//!
//! The search is depth first. It tries the rows in order, and the columns of a row in order, and it never skips a
//! branch because of a symmetry. Each candidate square is a `Try`, which is followed at once by a `Reject` or a
//! `Place`. Every `Place` is undone by a `Remove` later, and a complete board is a `Place`, a `Solution` and a
//! `Remove`.

/// The most queens that a materialised trace can hold, which is the length of the array in a snapshot.
pub(crate) const MAX_N: usize = 9;

/// The most events that a trace can hold. A search that needs more is an error, and is never cut short.
pub(crate) const EVENT_BUDGET: usize = 250_000;

/// What a row of a snapshot holds when no queen is on it.
pub(crate) const UNPLACED: u8 = 0xFF;

/// The largest board whose events can be counted without being kept.
#[cfg(test)]
pub(crate) const MAX_COUNTED_N: usize = 10;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The line along which an earlier queen attacks a square. When several do, the first of these in this order is the
/// reason that is given.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Conflict {
    Column,
    MainDiagonal,
    AntiDiagonal,
}

impl Conflict {
    /// The words that name the line, as they are written in the status and in the name of a cell.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Column => "column",
            Self::MainDiagonal => "main diagonal",
            Self::AntiDiagonal => "anti-diagonal",
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One step of the search. Rows and columns are counted from 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchEvent {
    /// A square is about to be tried. Nothing is decided yet.
    Try { row: u8, col: u8 },
    /// The square is attacked. `reason` is the first line in [`Conflict`] order, and `directions` is how many of the
    /// three lines have an earlier queen on them.
    Reject { row: u8, col: u8, reason: Conflict, directions: u8 },
    /// The square is free, and a queen is put on it.
    Place { row: u8, col: u8 },
    /// The last row has a queen, so the board is a solution. The queens stay where they are.
    Solution,
    /// The queen on this square is taken off, as the search goes back.
    Remove { row: u8, col: u8 },
    /// Every branch has been searched.
    Exhausted,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The queens on the board after an event: the column of the queen in each row, or [`UNPLACED`], and how many are
/// placed. The rows from `depth` on are always [`UNPLACED`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompactSnapshot {
    pub(crate) positions: [u8; MAX_N],
    pub(crate) depth: u8,
}

impl CompactSnapshot {
    /// The board with no queens.
    pub(crate) const EMPTY: Self = Self {
        positions: [UNPLACED; MAX_N],
        depth: 0,
    };

    /// The column of the queen in `row`, or `None` if the row has none.
    pub(crate) fn queen_in(&self, row: usize) -> Option<usize> {
        match self.positions.get(row) {
            Some(&col) if col != UNPLACED => Some(usize::from(col)),
            _ => None,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a trace or a count could not be made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TraceError {
    /// The board has no squares.
    InvalidBoardSize,
    /// The board is bigger than this builder takes.
    UnsupportedBoardSize,
    /// The search has more events than the budget allows.
    EventBudgetExceeded,
}

impl std::fmt::Display for TraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidBoardSize => write!(f, "the board must have at least one square"),
            Self::UnsupportedBoardSize => write!(f, "the board is bigger than this demo takes"),
            Self::EventBudgetExceeded => write!(f, "the search needs more than the {EVENT_BUDGET} events allowed"),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The lines through the square `(row, col)` that have a queen of `queens` (the rows above `row`) on them.
///
/// Returns the first of them in [`Conflict`] order, and how many there are, or `None` if the square is free.
pub(crate) fn conflicts(queens: &[u8], row: usize, col: usize) -> Option<(Conflict, u8)> {
    let (mut column, mut main, mut anti) = (false, false, false);
    for (earlier, &placed) in queens.iter().enumerate().take(row) {
        let placed = usize::from(placed);
        column |= placed == col;
        main |= earlier + col == row + placed;
        anti |= earlier + placed == row + col;
    }
    let reason = if column {
        Conflict::Column
    } else if main {
        Conflict::MainDiagonal
    } else if anti {
        Conflict::AntiDiagonal
    } else {
        return None;
    };
    Some((reason, u8::from(column) + u8::from(main) + u8::from(anti)))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The whole search of one board size: the events, and a snapshot after each. Snapshot 0 is the empty board, and event
/// `k` (counted from 1) makes snapshot `k`, so there is one more snapshot than there are events.
pub(crate) struct Trace {
    n: usize,
    events: Vec<SearchEvent>,
    snapshots: Vec<CompactSnapshot>,
    /// The snapshots that are solutions, in order. A snapshot has the number of the event that made it.
    solution_indices: Vec<usize>,
}

impl Trace {
    /// The trace of an `n` by `n` board.
    ///
    /// # Errors
    ///
    /// Returns `Err` if `n` is 0 or more than [`MAX_N`], before anything is allocated, or if the search has more than
    /// [`EVENT_BUDGET`] events.
    pub(crate) fn build(n: usize) -> Result<Self, TraceError> {
        Self::build_within(n, EVENT_BUDGET)
    }

    /// [`Trace::build`] with a budget of `budget` events, so that the limit can be tested on a small board.
    pub(crate) fn build_within(n: usize, budget: usize) -> Result<Self, TraceError> {
        match n {
            0 => return Err(TraceError::InvalidBoardSize),
            n if n > MAX_N => return Err(TraceError::UnsupportedBoardSize),
            _ => {},
        }
        let mut walk = Walk {
            trace: Self {
                n,
                events: Vec::new(),
                snapshots: vec![CompactSnapshot::EMPTY],
                solution_indices: Vec::new(),
            },
            budget,
            now: CompactSnapshot::EMPTY,
        };
        walk.search(0)?;
        walk.push(SearchEvent::Exhausted)?;
        Ok(walk.trace)
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The number of rows and of columns.
    pub(crate) fn size(&self) -> usize {
        self.n
    }

    /// The number of events, which is also the index of the last snapshot.
    pub(crate) fn len(&self) -> usize {
        self.events.len()
    }

    /// The snapshot at `index`, which is 0 for the empty board at the start.
    pub(crate) fn snapshot(&self, index: usize) -> Option<&CompactSnapshot> {
        self.snapshots.get(index)
    }

    /// The event that made the snapshot at `index`, or `None` for snapshot 0.
    pub(crate) fn event(&self, index: usize) -> Option<&SearchEvent> {
        index.checked_sub(1).and_then(|at| self.events.get(at))
    }

    /// How many solutions the search finds in all.
    pub(crate) fn solution_count(&self) -> usize {
        self.solution_indices.len()
    }

    /// The snapshots that are solutions, in the order the search finds them.
    #[cfg(test)]
    pub(crate) fn solution_indices(&self) -> &[usize] {
        &self.solution_indices
    }

    /// How many solutions have been found by the snapshot at `index`, counting one that is at `index`.
    pub(crate) fn solutions_found(&self, index: usize) -> usize {
        self.solution_indices.partition_point(|&at| at <= index)
    }

    /// Which solution the snapshot at `index` is, counted from 1, or `None` if it is not one.
    pub(crate) fn solution_ordinal(&self, index: usize) -> Option<usize> {
        let found = self.solutions_found(index);
        (found > 0 && self.solution_indices[found - 1] == index).then_some(found)
    }

    /// The last solution that is strictly before `index`.
    pub(crate) fn previous_solution(&self, index: usize) -> Option<usize> {
        let before = self.solution_indices.partition_point(|&at| at < index);
        before.checked_sub(1).map(|at| self.solution_indices[at])
    }

    /// The first solution that is strictly after `index`.
    pub(crate) fn next_solution(&self, index: usize) -> Option<usize> {
        let up_to = self.solution_indices.partition_point(|&at| at <= index);
        self.solution_indices.get(up_to).copied()
    }

    /// The bytes that the events and the snapshots hold, by the capacity of their vectors.
    pub(crate) fn allocated_bytes(&self) -> usize {
        self.events.capacity() * size_of::<SearchEvent>()
            + self.snapshots.capacity() * size_of::<CompactSnapshot>()
            + self.solution_indices.capacity() * size_of::<usize>()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A search in progress: the trace so far, how many events it may still add, and the board as it is now.
struct Walk {
    trace: Trace,
    budget: usize,
    now: CompactSnapshot,
}

impl Walk {
    /// Adds an event, and the snapshot it makes. The event that would pass the budget is refused, and nothing is added.
    fn push(&mut self, event: SearchEvent) -> Result<(), TraceError> {
        if self.trace.events.len() >= self.budget {
            return Err(TraceError::EventBudgetExceeded);
        }
        self.trace.events.push(event);
        self.trace.snapshots.push(self.now);
        if event == SearchEvent::Solution {
            self.trace.solution_indices.push(self.trace.events.len());
        }
        Ok(())
    }

    /// Searches `row` and the rows below it. The caller has put queens in the rows above.
    fn search(&mut self, row: usize) -> Result<(), TraceError> {
        for col in 0..self.trace.n {
            let (r, c) = (row as u8, col as u8);
            self.push(SearchEvent::Try { row: r, col: c })?;
            if let Some((reason, directions)) = conflicts(&self.now.positions, row, col) {
                self.push(SearchEvent::Reject {
                    row: r,
                    col: c,
                    reason,
                    directions,
                })?;
                continue;
            }
            self.now.positions[row] = c;
            self.now.depth = r + 1;
            self.push(SearchEvent::Place { row: r, col: c })?;
            if row + 1 == self.trace.n {
                self.push(SearchEvent::Solution)?;
            } else {
                self.search(row + 1)?;
            }
            self.now.positions[row] = UNPLACED;
            self.now.depth = r;
            self.push(SearchEvent::Remove { row: r, col: c })?;
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a search of one board size comes to, counted without keeping it.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Counts {
    pub(crate) events: usize,
    pub(crate) solutions: usize,
}

/// Counts the events and the solutions of an `n` by `n` board, without making a trace, so that a board too big to keep
/// can still be checked.
///
/// This follows the same event rules as [`Trace::build`], but finds the free squares with bit masks and not with
/// [`conflicts`], so it is a check of the builder and not a copy of it. A candidate costs a `Try` and a `Reject` or a
/// `Place`, a `Place` costs a `Remove`, a solution costs one more, and the search ends with one `Exhausted`.
///
/// # Errors
///
/// Returns `Err` if `n` is 0 or more than [`MAX_COUNTED_N`].
#[cfg(test)]
pub(crate) fn count_only(n: usize) -> Result<Counts, TraceError> {
    match n {
        0 => return Err(TraceError::InvalidBoardSize),
        n if n > MAX_COUNTED_N => return Err(TraceError::UnsupportedBoardSize),
        _ => {},
    }
    let mut counts = Counts { events: 1, solutions: 0 };
    count_row(n, 0, 0, 0, 0, &mut counts);
    Ok(counts)
}

/// Counts the row `row` and the rows below it, given the columns and the two kinds of diagonal that have a queen.
#[cfg(test)]
fn count_row(n: usize, row: usize, columns: u32, main: u32, anti: u32, counts: &mut Counts) {
    for col in 0..n {
        counts.events += 2;
        // A main diagonal is the same for the squares where `row + n - col` is the same.
        let (m, a) = (1 << (row + n - col), 1 << (row + col));
        if columns & (1 << col) != 0 || main & m != 0 || anti & a != 0 {
            continue;
        }
        counts.events += 1;
        if row + 1 == n {
            counts.events += 1;
            counts.solutions += 1;
        } else {
            count_row(n, row + 1, columns | (1 << col), main | m, anti | a, counts);
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
