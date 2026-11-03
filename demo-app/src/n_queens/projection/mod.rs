//! What the page shows for one snapshot of the search, worked out from the trace and nothing else. Pure: nothing here
//! touches the DOM.
//!
//! The renderer never keeps its own idea of the search. It asks [`project`] for the [`View`] of an index, compares that
//! with the view it drew last, and writes only what differs. So going backwards is the same call as going forwards.

use super::solver::{CompactSnapshot, Conflict, SearchEvent, Trace};

/// The size of the board in the scene, in pixels, and the margin round it that the frame and the focus ring can use.
pub(crate) const BOARD_SIZE: f64 = 480.0;
pub(crate) const MARGIN: f64 = 12.0;

/// The size of the scene, in pixels, which is the same for every board size.
pub(crate) const SCENE_SIZE: f64 = BOARD_SIZE + 2.0 * MARGIN;

/// What is written where a field has nothing to say.
pub(crate) const NOTHING: &str = "\u{2014}";

/// What is said when a button cannot move the search any further.
pub(crate) const AT_FIRST: &str = "Already at the first step.";
pub(crate) const AT_LAST: &str = "Already at the last step.";
pub(crate) const NO_EARLIER_SOLUTION: &str = "There is no earlier solution.";
pub(crate) const NO_LATER_SOLUTION: &str = "There is no later solution.";

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the event that made a snapshot was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Initial,
    Try,
    Reject,
    Place,
    Solution,
    Remove,
    Exhausted,
}

impl Kind {
    /// The word that the status shows for it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Initial => "Initial",
            Self::Try => "Try",
            Self::Reject => "Reject",
            Self::Place => "Place",
            Self::Solution => "Solution",
            Self::Remove => "Remove",
            Self::Exhausted => "Exhausted",
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the outline round one square says: a square being tried, one that was refused, or one a queen has just left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutlineKind {
    Candidate,
    Rejected,
    Removed,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The one square that has an outline in a snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Outline {
    pub(crate) kind: OutlineKind,
    pub(crate) row: usize,
    pub(crate) col: usize,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The three rays through a square being tried or refused. When a square is refused, `chosen` is the line that is the
/// reason, and that ray is drawn heavier than the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rays {
    pub(crate) row: usize,
    pub(crate) col: usize,
    pub(crate) chosen: Option<Conflict>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What one square of the board is in a snapshot, which is also what its accessible name says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellState {
    Empty,
    Queen,
    Candidate,
    Rejected(Conflict),
    Removed,
}

impl CellState {
    /// The accessible name of a square. The grid says the row and the column, so the name does not.
    pub(crate) fn name(self) -> String {
        match self {
            Self::Empty => "empty".to_owned(),
            Self::Queen => "queen".to_owned(),
            Self::Candidate => "candidate".to_owned(),
            Self::Rejected(reason) => format!("rejected: {}", reason.name()),
            Self::Removed => "removed".to_owned(),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything that the page shows for the snapshot at `index`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct View {
    pub(crate) index: usize,
    /// The index of the last snapshot, which is also the number of events.
    pub(crate) last: usize,
    pub(crate) size: usize,
    pub(crate) kind: Kind,
    pub(crate) snapshot: CompactSnapshot,
    /// The square that the event is about, if it has one.
    pub(crate) cell: Option<(usize, usize)>,
    pub(crate) reason: Option<Conflict>,
    pub(crate) directions: u8,
    /// How many solutions the search has found by this snapshot, how many it finds in all, and which this one is.
    pub(crate) found: usize,
    pub(crate) total: usize,
    pub(crate) ordinal: Option<usize>,
    pub(crate) has_earlier_solution: bool,
    pub(crate) has_later_solution: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The view of the snapshot at `index`, or `None` if the trace has no such snapshot.
pub(crate) fn project(trace: &Trace, index: usize) -> Option<View> {
    let snapshot = *trace.snapshot(index)?;
    let (kind, cell, reason, directions) = match trace.event(index) {
        None => (Kind::Initial, None, None, 0),
        Some(&SearchEvent::Try { row, col }) => (Kind::Try, Some((row.into(), col.into())), None, 0),
        Some(&SearchEvent::Reject { row, col, reason, directions }) => {
            (Kind::Reject, Some((row.into(), col.into())), Some(reason), directions)
        },
        Some(&SearchEvent::Place { row, col }) => (Kind::Place, Some((row.into(), col.into())), None, 0),
        Some(&SearchEvent::Solution) => (Kind::Solution, None, None, 0),
        Some(&SearchEvent::Remove { row, col }) => (Kind::Remove, Some((row.into(), col.into())), None, 0),
        Some(&SearchEvent::Exhausted) => (Kind::Exhausted, None, None, 0),
    };
    Some(View {
        index,
        last: trace.len(),
        size: trace.size(),
        kind,
        snapshot,
        cell,
        reason,
        directions,
        found: trace.solutions_found(index),
        total: trace.solution_count(),
        ordinal: trace.solution_ordinal(index),
        has_earlier_solution: trace.previous_solution(index).is_some(),
        has_later_solution: trace.next_solution(index).is_some(),
    })
}

impl View {
    /// The outline that this snapshot draws, if any. A snapshot has at most one, so the three kinds never meet.
    pub(crate) fn outline(&self) -> Option<Outline> {
        let kind = match self.kind {
            Kind::Try => OutlineKind::Candidate,
            Kind::Reject => OutlineKind::Rejected,
            Kind::Remove => OutlineKind::Removed,
            _ => return None,
        };
        self.cell.map(|(row, col)| Outline { kind, row, col })
    }

    /// The rays that this snapshot draws, if any. They are drawn for a Try and a Reject and for nothing else, and they
    /// do not depend on which queens are on the board.
    pub(crate) fn rays(&self) -> Option<Rays> {
        match (self.kind, self.cell) {
            (Kind::Try, Some((row, col))) => Some(Rays { row, col, chosen: None }),
            (Kind::Reject, Some((row, col))) => Some(Rays { row, col, chosen: self.reason }),
            _ => None,
        }
    }

    /// Whether the frame round the whole board is drawn, which it is for a solution only.
    pub(crate) fn frame(&self) -> bool {
        self.kind == Kind::Solution
    }

    /// What the square at `row` and `col` is in this snapshot. A queen and an outline are never on the same square.
    pub(crate) fn cell_state(&self, row: usize, col: usize) -> CellState {
        if self.snapshot.queen_in(row) == Some(col) {
            return CellState::Queen;
        }
        match self.outline() {
            Some(Outline { kind, row: r, col: c }) if (r, c) == (row, col) => match (kind, self.reason) {
                (OutlineKind::Candidate, _) => CellState::Candidate,
                (OutlineKind::Rejected, Some(reason)) => CellState::Rejected(reason),
                (OutlineKind::Rejected, None) => CellState::Candidate,
                (OutlineKind::Removed, _) => CellState::Removed,
            },
            _ => CellState::Empty,
        }
    }

    /// The squares that are not empty, which are all that a change between two views can touch.
    pub(crate) fn busy_cells(&self) -> Vec<(usize, usize)> {
        let mut cells: Vec<(usize, usize)> = (0..self.size)
            .filter_map(|row| self.snapshot.queen_in(row).map(|col| (row, col)))
            .collect();
        if let Some(outline) = self.outline() {
            cells.push((outline.row, outline.col));
        }
        cells
    }

    /// The columns of the queens, from the first row, counted from 1 as the page writes them.
    pub(crate) fn queens_one_based(&self) -> Vec<usize> {
        (0..self.size)
            .filter_map(|row| self.snapshot.queen_in(row))
            .map(|col| col + 1)
            .collect()
    }

    /// Which buttons cannot move the search, because it is at an end. A button that cannot is marked as disabled but
    /// stays in the tab order.
    pub(crate) fn boundaries(&self) -> Boundaries {
        Boundaries {
            restart: self.index == 0,
            previous: self.index == 0,
            next: self.index == self.last,
            previous_solution: !self.has_earlier_solution,
            next_solution: !self.has_later_solution,
            play: self.index == self.last,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether each button is at the end of what it can do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Boundaries {
    pub(crate) restart: bool,
    pub(crate) previous: bool,
    pub(crate) next: bool,
    pub(crate) previous_solution: bool,
    pub(crate) next_solution: bool,
    pub(crate) play: bool,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The reason for a rejection as the status writes it. The count of directions is left out when there is only one.
pub(crate) fn reason_text(reason: Conflict, directions: u8) -> String {
    if directions > 1 {
        format!("{} (first of {directions} conflicting directions)", reason.name())
    } else {
        reason.name().to_owned()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The fields of the status for a view, as text. Rows, columns and the solution are counted from 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Status {
    pub(crate) position: String,
    pub(crate) kind: String,
    pub(crate) cell: String,
    pub(crate) depth: String,
    pub(crate) reason: String,
    pub(crate) solutions: String,
    pub(crate) vector: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The status for `view`.
pub(crate) fn status(view: &View) -> Status {
    let solutions = match view.ordinal {
        Some(ordinal) => format!("{} / {} (solution {ordinal})", view.found, view.total),
        None => format!("{} / {}", view.found, view.total),
    };
    Status {
        position: format!("{}/{}", view.index, view.last),
        kind: view.kind.name().to_owned(),
        cell: view.cell.map_or_else(
            || NOTHING.to_owned(),
            |(row, col)| format!("row {}, column {}", row + 1, col + 1),
        ),
        depth: format!("{} / {}", view.snapshot.depth, view.size),
        reason: match (view.kind, view.reason) {
            (Kind::Reject, Some(reason)) => reason_text(reason, view.directions),
            _ => NOTHING.to_owned(),
        },
        solutions,
        vector: if view.kind == Kind::Solution { vector_text(view) } else { NOTHING.to_owned() },
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The columns of the queens of a solution, counted from 1, in the order of the rows.
fn vector_text(view: &View) -> String {
    let columns: Vec<String> = view.queens_one_based().iter().map(usize::to_string).collect();
    format!("[{}]", columns.join(", "))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The one sentence that a person who steps by hand hears for `view`.
pub(crate) fn summary(view: &View) -> String {
    let at = format!("Step {} of {}", view.index, view.last);
    let square = |(row, col): (usize, usize)| format!("row {}, column {}", row + 1, col + 1);
    let cell = view.cell.map(square).unwrap_or_default();
    match view.kind {
        Kind::Initial => format!("{at}: the board is empty."),
        Kind::Try => format!("{at}: trying {cell}."),
        Kind::Reject => format!(
            "{at}: {cell} is rejected, {}.",
            view.reason
                .map_or_else(String::new, |reason| reason_text(reason, view.directions))
        ),
        Kind::Place => format!("{at}: queen placed at {cell}, {} of {} queens.", view.snapshot.depth, view.size),
        Kind::Solution => format!(
            "{at}: solution {} of {}, columns by row {}.",
            view.ordinal.unwrap_or(view.found),
            view.total,
            vector_text(view)
        ),
        Kind::Remove => format!("{at}: queen removed from {cell}."),
        Kind::Exhausted => format!("{at}: the search is finished, with {} solutions.", view.total),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The one sentence that a person hears when the generations stop.
pub(crate) fn pause_summary(view: &View) -> String {
    format!(
        "Paused at step {} of {}, {} of {} queens placed, {} of {} solutions found.",
        view.index, view.last, view.snapshot.depth, view.size, view.found, view.total
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Whether a person who is listening is told of a step while the search is playing. Only the two steps that matter are.
pub(crate) fn announced_while_playing(kind: Kind) -> bool {
    matches!(kind, Kind::Solution | Kind::Exhausted)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The value text of the range that seeks, which assistive technology reads in place of the bare number.
pub(crate) fn seek_text(view: &View) -> String {
    format!("Step {} of {}, {}", view.index, view.last, view.kind.name())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the page says about the board that is drawn.
pub(crate) fn board_text(size: usize, events: usize, solutions: usize, bytes: usize) -> String {
    format!(
        "A board of {size} by {size}. Rows are counted from 1 at the top, and columns from 1 at the left. The search \
         has {events} events and finds {solutions} solutions. The trace takes about {} KiB.",
        bytes.div_ceil(1024)
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The size of a square for a board of `size`, in pixels.
pub(crate) fn cell_size(size: usize) -> f64 {
    BOARD_SIZE / size.max(1) as f64
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The top left corner of the square at `row` and `col`, in pixels of the scene.
pub(crate) fn cell_origin(size: usize, row: usize, col: usize) -> (f64, f64) {
    let cell = cell_size(size);
    (MARGIN + col as f64 * cell, MARGIN + row as f64 * cell)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `d` of the path for the ray along `line` through the square at `row` and `col`, from one edge of the board to
/// the other. The ray does not depend on which squares are free.
pub(crate) fn ray_path(size: usize, row: usize, col: usize, line: Conflict) -> String {
    let (x, y) = (col as f64 + 0.5, row as f64 + 0.5);
    let n = size as f64;
    // In squares. A step `t` along a diagonal moves one square across and one square down or up.
    let ((x0, y0), (x1, y1)) = match line {
        Conflict::Column => ((x, 0.0), (x, n)),
        Conflict::MainDiagonal => {
            let (lo, hi) = ((-x).max(-y), (n - x).min(n - y));
            ((x + lo, y + lo), (x + hi, y + hi))
        },
        Conflict::AntiDiagonal => {
            let (lo, hi) = ((-x).max(y - n), (n - x).min(y));
            ((x + lo, y - lo), (x + hi, y - hi))
        },
    };
    let cell = cell_size(size);
    let at = |u: f64| MARGIN + u * cell;
    format!("M{:.2} {:.2}L{:.2} {:.2}", at(x0), at(y0), at(x1), at(y1))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where the active square goes when `key` is pressed with it at `from`, on a board of `size`. The arrow keys stop at
/// an edge, Home and End go to the ends of the row, and with Ctrl to the corners of the board. Every other key,
/// including Page Up and Page Down, is left alone and gives `None`.
pub(crate) fn move_active(key: &str, ctrl: bool, from: (usize, usize), size: usize) -> Option<(usize, usize)> {
    let (row, col) = from;
    let last = size.checked_sub(1)?;
    match (key, ctrl) {
        ("ArrowLeft", _) => Some((row, col.saturating_sub(1))),
        ("ArrowRight", _) => Some((row, (col + 1).min(last))),
        ("ArrowUp", _) => Some((row.saturating_sub(1), col)),
        ("ArrowDown", _) => Some(((row + 1).min(last), col)),
        ("Home", false) => Some((row, 0)),
        ("End", false) => Some((row, last)),
        ("Home", true) => Some((0, 0)),
        ("End", true) => Some((last, last)),
        _ => None,
    }
}

/// `cell` moved onto a board of `size`, to the nearest square if it is off the board.
pub(crate) fn clamp_cell(cell: (usize, usize), size: usize) -> (usize, usize) {
    let last = size.saturating_sub(1);
    (cell.0.min(last), cell.1.min(last))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
