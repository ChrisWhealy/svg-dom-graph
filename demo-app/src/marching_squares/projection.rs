//! What a state looks like: which contours are drawn, which cell is shown, and what the inspector and the status say. A
//! pure function of the trace and the state, so that going forward, going back and jumping all give the same picture,
//! and nothing here knows the view.

use super::{
    field::{FieldId, LEVELS, SLOT_NAMES, SLOT_PATTERNS},
    solver::{CellResult, Decision, Edge, Totals, Trace, VertexKey, stitch, totals},
};

/// Which levels are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Show {
    All,
    One(usize),
}

impl Show {
    pub(crate) fn shows(self, slot: usize) -> bool {
        match self {
            Self::All => true,
            Self::One(only) => only == slot,
        }
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::One(0) => "low",
            Self::One(1) => "middle",
            Self::One(_) => "high",
        }
    }

    pub(crate) fn from_key(key: &str) -> Option<Self> {
        match key {
            "all" => Some(Self::All),
            "low" => Some(Self::One(0)),
            "middle" => Some(Self::One(1)),
            "high" => Some(Self::One(2)),
            _ => None,
        }
    }
}

/// Everything that decides the picture, apart from the trace and the view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct State {
    pub(crate) field: FieldId,
    pub(crate) levels: [f64; LEVELS],
    /// The number of cells visited so far, from 0 to the number of cells.
    pub(crate) step: usize,
    pub(crate) show: Show,
    /// The level that the inspector reads.
    pub(crate) inspected: usize,
    /// The cell a person chose, as row and column.
    pub(crate) selected: Option<(usize, usize)>,
}

impl State {
    pub(crate) fn first(field: FieldId) -> Self {
        Self {
            field,
            levels: field.levels(),
            step: 0,
            show: Show::All,
            inspected: 1,
            selected: None,
        }
    }
}

/// A contour to draw.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Line {
    pub(crate) slot: usize,
    pub(crate) points: Vec<(f64, f64)>,
    pub(crate) closed: bool,
}

/// What the inspector says about a cell.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Inspected {
    /// No cell is selected, and none has been visited.
    NoCell,
    /// The cell has not been visited, so nothing but where it is is said.
    NotYet {
        row: usize,
        col: usize,
    },
    Cell(Box<CellView>),
}

/// A visited cell at the inspected level.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CellView {
    pub(crate) row: usize,
    pub(crate) col: usize,
    pub(crate) slot: usize,
    pub(crate) level: f64,
    /// The four samples, as top left, top right, bottom right and bottom left.
    pub(crate) samples: [f64; 4],
    pub(crate) result: CellResult,
    /// The crossed edges, with the point on each.
    pub(crate) crossings: Vec<(Edge, (f64, f64))>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Projection {
    pub(crate) lines: Vec<Line>,
    /// The cells that have been visited, at a level that is drawn, and are ambiguous.
    pub(crate) ambiguous: Vec<(usize, usize)>,
    /// The cell that is outlined: the chosen one, or the last visited.
    pub(crate) cell: Option<(usize, usize)>,
    pub(crate) inspected: Inspected,
    pub(crate) status: String,
    pub(crate) description: String,
    /// For each level: the counts of its contours so far, and whether they are final.
    pub(crate) totals: [Totals; LEVELS],
    pub(crate) complete: bool,
    pub(crate) segments: usize,
    pub(crate) digest: u64,
}

/// The cell that the inspector reads: the chosen one, or the last that was visited.
pub(crate) fn effective_cell(trace: &Trace, state: &State) -> Option<(usize, usize)> {
    state
        .selected
        .or_else(|| state.step.checked_sub(1).map(|last| trace.grid.cell_at(last)))
}

/// A level for people: with its sign.
pub(crate) fn level_text(level: f64) -> String {
    format!("{level:+.3}")
}

/// Pairs such as "T–R and B–L".
pub(crate) fn pairs_text(pairs: &[(Edge, Edge)]) -> String {
    pairs
        .iter()
        .map(|(a, b)| format!("{}–{}", a.letter(), b.letter()))
        .collect::<Vec<_>>()
        .join(" and ")
}

fn q_text(cell: &CellResult) -> String {
    match cell.decision {
        Some(Decision::Positive) => "Q positive".to_owned(),
        Some(Decision::Negative) => "Q negative".to_owned(),
        Some(Decision::Tie) => "Q exactly zero, so two strands cross at the saddle".to_owned(),
        None => "no decider".to_owned(),
    }
}

/// What a visited cell says in one sentence.
pub(crate) fn cell_text(view: &CellView, levels_visible: &str) -> String {
    let CellView {
        row, col, slot, level, result, ..
    } = view;
    let name = SLOT_NAMES[*slot];
    let shape = if result.pairs.is_empty() {
        "no contour here".to_owned()
    } else if result.decision == Some(Decision::Tie) {
        "T–B and L–R cross".to_owned()
    } else {
        pairs_text(&result.pairs)
    };
    let rho = result.rho.map_or_else(String::new, |rho| {
        format!(
            "; rho {rho:.3}{}",
            if result.near_degenerate() { ", near-degenerate" } else { "" }
        )
    });
    format!(
        "Cell ({row}, {col}), {name} level {}: mask {}; {}{rho}; {shape}; {levels_visible}.",
        level_text(*level),
        result.mask,
        q_text(result),
    )
}

fn crossed_edges(result: &CellResult, r: usize, c: usize) -> Vec<(Edge, (f64, f64))> {
    let mut found: Vec<(Edge, (f64, f64))> = Vec::new();
    for s in &result.segments {
        for end in [s.a, s.b] {
            if matches!(end.key, VertexKey::SaddleV { .. } | VertexKey::SaddleH { .. }) {
                continue;
            }
            let edge = [Edge::T, Edge::R, Edge::B, Edge::L]
                .into_iter()
                .find(|&e| super::solver::edge_key(e, r, c) == end.key);
            if let Some(edge) = edge {
                if !found.iter().any(|(e, _)| *e == edge) {
                    found.push((edge, (end.point.x, end.point.y)));
                }
            }
        }
    }
    found
}

/// The projection of `state`.
pub(crate) fn project(trace: &Trace, state: &State) -> Projection {
    let cells = trace.grid.cell_count();
    let step = state.step.min(cells);
    // A step beyond the end is the end, here and wherever the last visited cell is worked out.
    let state = &State { step, ..*state };
    let mut lines = Vec::new();
    let mut ambiguous = Vec::new();
    let mut counts = [Totals::default(); LEVELS];
    let mut segments = 0;
    for (slot, count) in counts.iter_mut().enumerate() {
        let segs = trace.segments(slot, step);
        let contours = stitch(&segs);
        *count = totals(&contours, &segs);
        segments += segs.len();
        if !state.show.shows(slot) {
            continue;
        }
        for c in &contours {
            lines.push(Line {
                slot,
                points: c.drawn_points().into_iter().map(|p| (p.x, p.y)).collect(),
                closed: c.closed,
            });
        }
        for (i, cell) in trace.levels[slot].by_cell.iter().take(step).enumerate() {
            if cell.ambiguous() {
                let at = trace.grid.cell_at(i);
                if !ambiguous.contains(&at) {
                    ambiguous.push(at);
                }
            }
        }
    }
    ambiguous.sort_unstable();
    let cell = effective_cell(trace, state);
    let visible = match state.show {
        Show::All => "3 levels visible".to_owned(),
        Show::One(slot) => format!("the {} level only", SLOT_NAMES[slot]),
    };
    let across = trace.grid.cells_across();
    let inspected = match cell {
        None => Inspected::NoCell,
        Some((r, c)) if r * across + c >= step => Inspected::NotYet { row: r, col: c },
        Some((r, c)) => {
            let slot = state.inspected;
            let result = trace.levels[slot].by_cell[r * across + c].clone();
            Inspected::Cell(Box::new(CellView {
                row: r,
                col: c,
                slot,
                level: state.levels[slot],
                samples: [
                    trace.grid.at(r, c),
                    trace.grid.at(r, c + 1),
                    trace.grid.at(r + 1, c + 1),
                    trace.grid.at(r + 1, c),
                ],
                crossings: crossed_edges(&result, r, c),
                result,
            }))
        },
    };
    let complete = step == cells;
    let progress = format!("{step} of {cells} cells visited");
    let status = match &inspected {
        Inspected::NoCell => "No cell selected.".to_owned(),
        Inspected::NotYet { row, col } => format!("Cell ({row}, {col}): Not yet evaluated."),
        Inspected::Cell(view) => cell_text(view, &visible),
    };
    let status = if complete {
        let per_level = (0..LEVELS)
            .map(|s| {
                format!(
                    "{}: {} open, {} closed, {} saddle ties",
                    SLOT_NAMES[s], counts[s].open, counts[s].closed, counts[s].ties
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        format!("{status} Complete. {per_level}.")
    } else {
        format!("{status} {progress}.")
    };
    let levels_text = (0..LEVELS)
        .map(|s| format!("{} {} {}", SLOT_NAMES[s], level_text(state.levels[s]), SLOT_PATTERNS[s]))
        .collect::<Vec<_>>()
        .join(", ");
    let description = format!(
        "Marching squares contours of {}: {levels_text}. {progress}, so the contours are {}.",
        state.field.label(),
        if complete { "complete" } else { "partial, with provisional open ends" }
    );
    Projection {
        lines,
        ambiguous,
        cell,
        inspected,
        status,
        description,
        totals: counts,
        complete,
        segments,
        digest: trace.digest(step),
    }
}

#[cfg(test)]
mod unit_tests;
