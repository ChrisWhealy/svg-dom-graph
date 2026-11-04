//! Marching squares on a sampled scalar field: the case of each cell, the crossing on each grid edge, the segments of
//! each cell, and the contours that the segments make. Pure: nothing here touches the DOM, and no zoom, pixel or
//! rounding enters any of it.
//!
//! Corners are named TL, TR, BR and BL, and `g = sample - level`. A corner is set when `g > 0`, with TL = 1, TR = 2, BR
//! = 4 and BL = 8. A crossing is calculated once for each grid edge, and both cells that share the edge use it, so the
//! coordinates are the same bits. An ambiguous cell (case 5 or 10) takes the asymptotic decider `Q`, and an exact `Q =
//! 0` is two strands that cross at the saddle, each with a virtual key of its own.

mod stitch;

use super::field::LEVELS;
use std::collections::BTreeMap;

pub(crate) use stitch::{Contour, Totals, stitch, totals};

/// The least and the most samples along a side, and the most samples and cells.
pub(crate) const MIN_SIDE: usize = 2;
pub(crate) const MAX_SIDE: usize = 65;
pub(crate) const MAX_SAMPLES: usize = 4_225;
pub(crate) const MAX_CELLS: usize = 4_096;
/// The most raw segments: four for every cell at every level, since an exact tie has four spokes.
pub(crate) const MAX_SEGMENTS: usize = 4 * MAX_CELLS * LEVELS;
/// A segment has two ends.
pub(crate) const MAX_INCIDENCES: usize = 2 * MAX_SEGMENTS;
/// The most vertices of the contours of one level. Each segment adds at most one vertex to a chain, and a chain adds
/// one more, and a closed one repeats its first, so this is more than any field can need, and it is checked all the
/// same.
pub(crate) const MAX_VERTICES: usize = MAX_SEGMENTS + MAX_SEGMENTS / 2;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Why a field or a level was refused. Every case has the values that name it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SolverError {
    BadDimensions {
        width: usize,
        height: usize,
    },
    WrongSampleCount {
        expected: usize,
        got: usize,
    },
    NonFiniteSample {
        row: usize,
        col: usize,
    },
    /// The levels are not finite and strictly ascending.
    BadLevels,
    /// A sample is exactly at a level, which is a plateau and not a saddle.
    VertexOnLevel {
        row: usize,
        col: usize,
        slot: usize,
    },
    /// More geometry than the demo draws.
    GeometryBudgetExceeded {
        kind: &'static str,
        requested: usize,
        limit: usize,
    },
    /// A topology rule failed, which would be a defect in the solver.
    Topology(String),
}

impl std::fmt::Display for SolverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadDimensions { width, height } => {
                write!(f, "a field of {width} by {height} samples is not between 2 by 2 and 65 by 65")
            },
            Self::WrongSampleCount { expected, got } => write!(f, "the field needs {expected} samples and has {got}"),
            Self::NonFiniteSample { row, col } => write!(f, "the sample at row {row}, column {col} is not finite"),
            Self::BadLevels => write!(f, "the three levels must be finite and strictly ascending"),
            Self::VertexOnLevel { row, col, slot } => write!(
                f,
                "the sample at row {row}, column {col} is exactly at the {} level, which makes a plateau and not a saddle",
                super::field::SLOT_NAMES[*slot]
            ),
            Self::GeometryBudgetExceeded { kind, requested, limit } => {
                write!(f, "{requested} {kind} is more than the {limit} that are drawn")
            },
            Self::Topology(problem) => write!(f, "the contours broke a rule: {problem}"),
        }
    }
}

impl std::error::Error for SolverError {}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The samples of a field, row by row, with row 0 at the top.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Grid {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) values: Vec<f64>,
}

impl Grid {
    /// Checks the size and the samples.
    ///
    /// # Errors
    ///
    /// Returns `Err` for a side outside 2 to 65, too many samples, a count that is not width times height, or a sample
    /// that is not finite.
    pub(crate) fn new(width: usize, height: usize, values: Vec<f64>) -> Result<Self, SolverError> {
        if !(MIN_SIDE..=MAX_SIDE).contains(&width) || !(MIN_SIDE..=MAX_SIDE).contains(&height) {
            return Err(SolverError::BadDimensions { width, height });
        }
        let expected = width
            .checked_mul(height)
            .filter(|&n| n <= MAX_SAMPLES)
            .ok_or(SolverError::BadDimensions { width, height })?;
        if values.len() != expected {
            return Err(SolverError::WrongSampleCount { expected, got: values.len() });
        }
        if let Some(at) = values.iter().position(|v| !v.is_finite()) {
            return Err(SolverError::NonFiniteSample {
                row: at / width,
                col: at % width,
            });
        }
        Ok(Self { width, height, values })
    }

    /// The sample at row `r` and column `c`.
    pub(crate) fn at(&self, r: usize, c: usize) -> f64 {
        self.values[r * self.width + c]
    }

    /// The number of cells across and down.
    pub(crate) fn cells_across(&self) -> usize {
        self.width - 1
    }

    pub(crate) fn cell_count(&self) -> usize {
        (self.width - 1) * (self.height - 1)
    }

    /// The row and the column of the cell with row-major index `index`.
    pub(crate) fn cell_at(&self, index: usize) -> (usize, usize) {
        (index / self.cells_across(), index % self.cells_across())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A point in the grid's own coordinates: `x` is the column, and `y` is the height minus one minus the row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Point2 {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

/// Which crossing it is. The order is the order of the declaration, then the row, then the column, so that `H` is
/// before `V`, and before the two virtual keys of a saddle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum VertexKey {
    /// The horizontal grid edge from sample `(row, col)` to `(row, col + 1)`.
    H { row: u16, col: u16 },
    /// The vertical grid edge from sample `(row, col)` to `(row + 1, col)`.
    V { row: u16, col: u16 },
    /// The saddle of an exact tie, where the strand from top to bottom crosses.
    SaddleV { row: u16, col: u16 },
    /// The saddle of an exact tie, where the strand from left to right crosses.
    SaddleH { row: u16, col: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Crossing {
    pub(crate) key: VertexKey,
    pub(crate) point: Point2,
}

/// The identity of a segment: the index of its cell, and its place among the cell's segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SegmentId {
    pub(crate) cell: u32,
    pub(crate) index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Segment {
    pub(crate) id: SegmentId,
    pub(crate) a: Crossing,
    pub(crate) b: Crossing,
}

/// One side of a cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edge {
    T,
    R,
    B,
    L,
}

impl Edge {
    pub(crate) fn letter(self) -> char {
        match self {
            Self::T => 'T',
            Self::R => 'R',
            Self::B => 'B',
            Self::L => 'L',
        }
    }
}

/// How an ambiguous cell was decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    /// `Q > 0`: top with right, and bottom with left.
    Positive,
    /// `Q < 0`: top with left, and right with bottom.
    Negative,
    /// `Q = 0`: two strands cross at the saddle.
    Tie,
}

/// What one cell holds at one level.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CellResult {
    pub(crate) mask: u8,
    /// `Q`, for an ambiguous cell.
    pub(crate) q: Option<f64>,
    /// `|Q| / max(|gTL gBR|, |gTR gBL|)`, for an ambiguous cell. Both products are positive there.
    pub(crate) rho: Option<f64>,
    pub(crate) decision: Option<Decision>,
    /// The pairs of edges that the cell joins, in the order of the table. An exact tie has the two strands, and each is
    /// two spokes through its own saddle key, so it has four segments.
    pub(crate) pairs: Vec<(Edge, Edge)>,
    pub(crate) segments: Vec<Segment>,
}

/// A cell below this `rho` is flagged near-degenerate. This is only a note, and never changes the branch.
pub(crate) const NEAR_DEGENERATE: f64 = 0.10;

impl CellResult {
    pub(crate) fn ambiguous(&self) -> bool {
        self.mask == 5 || self.mask == 10
    }

    pub(crate) fn near_degenerate(&self) -> bool {
        self.rho.is_some_and(|rho| rho > 0.0 && rho < NEAR_DEGENERATE)
    }
}

/// Everything about one level.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LevelTrace {
    pub(crate) level: f64,
    pub(crate) by_cell: Vec<CellResult>,
}

/// Every level of a field, worked out before anything is drawn.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Trace {
    pub(crate) grid: Grid,
    pub(crate) levels: [LevelTrace; LEVELS],
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn bad_levels(levels: &[f64; LEVELS]) -> bool {
    levels.iter().any(|l| !l.is_finite()) || levels.windows(2).any(|w| w[0] >= w[1])
}

/// The key of an edge of cell `(r, c)`.
pub(crate) fn edge_key(edge: Edge, r: usize, c: usize) -> VertexKey {
    let (row, col) = (r as u16, c as u16);
    match edge {
        Edge::T => VertexKey::H { row, col },
        Edge::B => VertexKey::H { row: row + 1, col },
        Edge::L => VertexKey::V { row, col },
        Edge::R => VertexKey::V { row, col: col + 1 },
    }
}

/// The pairs of edges for a mask that is not ambiguous, in the order of the table.
fn table(mask: u8) -> &'static [(Edge, Edge)] {
    use Edge::{B, L, R, T};
    match mask {
        1 => &[(T, L)],
        2 => &[(T, R)],
        3 => &[(L, R)],
        4 => &[(R, B)],
        6 => &[(T, B)],
        7 => &[(L, B)],
        8 => &[(B, L)],
        9 => &[(T, B)],
        11 => &[(R, B)],
        12 => &[(L, R)],
        13 => &[(T, R)],
        14 => &[(T, L)],
        _ => &[],
    }
}

impl Trace {
    /// Works out every cell at every level.
    ///
    /// # Errors
    ///
    /// Returns `Err` for levels that are not finite and strictly ascending, a sample exactly at a level, or more
    /// geometry than is drawn. Nothing is partly done.
    pub(crate) fn build(grid: Grid, levels: [f64; LEVELS]) -> Result<Self, SolverError> {
        if bad_levels(&levels) {
            return Err(SolverError::BadLevels);
        }
        let cells = grid.cell_count();
        let limit = |kind, requested: Option<usize>, limit| match requested {
            Some(n) if n <= limit => Ok(()),
            other => Err(SolverError::GeometryBudgetExceeded {
                kind,
                requested: other.unwrap_or(usize::MAX),
                limit,
            }),
        };
        limit("cells", Some(cells), MAX_CELLS)?;
        limit(
            "segments",
            cells.checked_mul(4).and_then(|n| n.checked_mul(LEVELS)),
            MAX_SEGMENTS,
        )?;
        limit(
            "endpoints",
            cells.checked_mul(8).and_then(|n| n.checked_mul(LEVELS)),
            MAX_INCIDENCES,
        )?;
        for (slot, &level) in levels.iter().enumerate() {
            if let Some(at) = grid.values.iter().position(|&v| v == level) {
                return Err(SolverError::VertexOnLevel {
                    row: at / grid.width,
                    col: at % grid.width,
                    slot,
                });
            }
        }
        let build_level = |level: f64| LevelTrace {
            level,
            by_cell: level_cells(&grid, level),
        };
        let traces = [build_level(levels[0]), build_level(levels[1]), build_level(levels[2])];
        let trace = Self { grid, levels: traces };
        // The rules of a finished field hold for every field the solver accepts, so a failure here is a defect, and it
        // is found before anything is drawn.
        trace.validate()?;
        Ok(trace)
    }

    /// Every level's segments of the first `k` cells, in cell order.
    pub(crate) fn segments(&self, slot: usize, k: usize) -> Vec<Segment> {
        self.levels[slot]
            .by_cell
            .iter()
            .take(k)
            .flat_map(|cell| cell.segments.iter().copied())
            .collect()
    }

    /// The contours that the first `k` cells make at a level.
    pub(crate) fn contours(&self, slot: usize, k: usize) -> Vec<Contour> {
        stitch(&self.segments(slot, k))
    }

    /// A digest of everything about the first `k` cells at every level: the level, the keys of each contour in order,
    /// and the bits of every coordinate. It does not depend on a view.
    pub(crate) fn digest(&self, k: usize) -> u64 {
        let mut hash = Fnv::new();
        for slot in 0..LEVELS {
            hash.u64(slot as u64);
            hash.u64(self.levels[slot].level.to_bits());
            let contours = self.contours(slot, k);
            hash.u64(contours.len() as u64);
            for contour in &contours {
                hash.u64(u64::from(contour.closed));
                hash.u64(contour.keys.len() as u64);
                for (key, point) in contour.keys.iter().zip(&contour.vertices) {
                    hash.key(*key);
                    hash.u64(point.x.to_bits());
                    hash.u64(point.y.to_bits());
                }
            }
        }
        hash.finish()
    }

    /// Checks the rules of a finished field: every segment is used once, an interior crossing has two segments, and a
    /// crossing with one is on the outer boundary.
    ///
    /// # Errors
    ///
    /// Returns `Err` with the rule that failed.
    pub(crate) fn validate(&self) -> Result<(), SolverError> {
        let (w, h) = (self.grid.width, self.grid.height);
        for slot in 0..LEVELS {
            let segments = self.segments(slot, self.grid.cell_count());
            let mut degree: BTreeMap<VertexKey, usize> = BTreeMap::new();
            let mut seen = std::collections::BTreeSet::new();
            for s in &segments {
                if !seen.insert(s.id) {
                    return Err(SolverError::Topology(format!("segment {:?} appears twice", s.id)));
                }
                for end in [s.a, s.b] {
                    if !end.point.x.is_finite() || !end.point.y.is_finite() {
                        return Err(SolverError::Topology(format!("{:?} is not finite", end.key)));
                    }
                    *degree.entry(end.key).or_default() += 1;
                }
            }
            for (key, d) in &degree {
                let boundary = match *key {
                    VertexKey::H { row, .. } => row == 0 || usize::from(row) == h - 1,
                    VertexKey::V { col, .. } => col == 0 || usize::from(col) == w - 1,
                    VertexKey::SaddleV { .. } | VertexKey::SaddleH { .. } => false,
                };
                match (*d, boundary) {
                    (2, _) | (1, true) => {},
                    (d, _) => {
                        return Err(SolverError::Topology(format!(
                            "{key:?} has degree {d} at the {} level",
                            super::field::SLOT_NAMES[slot]
                        )));
                    },
                }
            }
            let contours = stitch(&segments);
            let used: usize = contours.iter().map(|c| c.segment_count()).sum();
            if used != segments.len() {
                return Err(SolverError::Topology(format!(
                    "{used} of {} segments are in a contour",
                    segments.len()
                )));
            }
            // What is drawn: every vertex, and the first again at the end of a loop.
            let vertices: usize = contours.iter().map(|c| c.drawn_points().len()).sum();
            if vertices > MAX_VERTICES {
                return Err(SolverError::GeometryBudgetExceeded {
                    kind: "vertices",
                    requested: vertices,
                    limit: MAX_VERTICES,
                });
            }
        }
        Ok(())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A 64-bit FNV-1a hash.
struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    fn u64(&mut self, value: u64) {
        for byte in value.to_le_bytes() {
            self.0 ^= u64::from(byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }

    fn key(&mut self, key: VertexKey) {
        let (kind, row, col) = match key {
            VertexKey::H { row, col } => (0, row, col),
            VertexKey::V { row, col } => (1, row, col),
            VertexKey::SaddleV { row, col } => (2, row, col),
            VertexKey::SaddleH { row, col } => (3, row, col),
        };
        self.u64(kind);
        self.u64(u64::from(row));
        self.u64(u64::from(col));
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The crossing on a grid edge, calculated once. Horizontal edges are interpolated from left to right, and vertical
/// ones from top to bottom.
fn crossing(grid: &Grid, level: f64, key: VertexKey) -> Crossing {
    let height = grid.height;
    let (a, b, pa, pb) = match key {
        VertexKey::H { row, col } => {
            let (r, c) = (usize::from(row), usize::from(col));
            let y = (height - 1 - r) as f64;
            (
                grid.at(r, c),
                grid.at(r, c + 1),
                Point2 { x: c as f64, y },
                Point2 { x: (c + 1) as f64, y },
            )
        },
        VertexKey::V { row, col } => {
            let (r, c) = (usize::from(row), usize::from(col));
            let x = c as f64;
            (
                grid.at(r, c),
                grid.at(r + 1, c),
                Point2 { x, y: (height - 1 - r) as f64 },
                Point2 { x, y: (height - 2 - r) as f64 },
            )
        },
        VertexKey::SaddleV { .. } | VertexKey::SaddleH { .. } => unreachable!("a saddle is not on a grid edge"),
    };
    let (ga, gb) = (a - level, b - level);
    let t = ga / (ga - gb);
    Crossing {
        key,
        point: Point2 {
            x: pa.x + t * (pb.x - pa.x),
            y: pa.y + t * (pb.y - pa.y),
        },
    }
}

/// The cell results of a whole level. The crossings are calculated once for each grid edge, in a cache.
fn level_cells(grid: &Grid, level: f64) -> Vec<CellResult> {
    let mut cache: BTreeMap<VertexKey, Crossing> = BTreeMap::new();
    let mut get = |key: VertexKey| *cache.entry(key).or_insert_with(|| crossing(grid, level, key));
    let (across, down) = (grid.cells_across(), grid.height - 1);
    let mut cells = Vec::with_capacity(across * down);
    for r in 0..down {
        for c in 0..across {
            let g = [
                grid.at(r, c) - level,
                grid.at(r, c + 1) - level,
                grid.at(r + 1, c + 1) - level,
                grid.at(r + 1, c) - level,
            ];
            let mask = u8::from(g[0] > 0.0)
                | u8::from(g[1] > 0.0) << 1
                | u8::from(g[2] > 0.0) << 2
                | u8::from(g[3] > 0.0) << 3;
            let index = (r * across + c) as u32;
            let (q, rho, decision, pairs) = if mask == 5 || mask == 10 {
                let (p1, p2) = (g[0] * g[2], g[1] * g[3]);
                let q = p1 - p2;
                let rho = q.abs() / p1.abs().max(p2.abs());
                use Edge::{B, L, R, T};
                let (decision, pairs) = if q > 0.0 {
                    (Decision::Positive, vec![(T, R), (B, L)])
                } else if q < 0.0 {
                    (Decision::Negative, vec![(T, L), (R, B)])
                } else {
                    (Decision::Tie, vec![(T, B), (L, R)])
                };
                (Some(q), Some(rho), Some(decision), pairs)
            } else {
                (None, None, None, table(mask).to_vec())
            };
            let mut segments = Vec::new();
            let end = |edge: Edge, get: &mut dyn FnMut(VertexKey) -> Crossing| get(edge_key(edge, r, c));
            if decision == Some(Decision::Tie) {
                // Where the strands cross, measured right and down in the cell.
                let d = g[0] - g[1] + g[2] - g[3];
                let (sx, sy) = ((g[0] - g[3]) / d, (g[0] - g[1]) / d);
                let point = Point2 {
                    x: c as f64 + sx,
                    y: (grid.height - 1 - r) as f64 - sy,
                };
                let (row, col) = (r as u16, c as u16);
                let sv = Crossing {
                    key: VertexKey::SaddleV { row, col },
                    point,
                };
                let sh = Crossing {
                    key: VertexKey::SaddleH { row, col },
                    point,
                };
                let spokes = [
                    (end(Edge::T, &mut get), sv),
                    (sv, end(Edge::B, &mut get)),
                    (end(Edge::L, &mut get), sh),
                    (sh, end(Edge::R, &mut get)),
                ];
                for (index_in_cell, (a, b)) in spokes.into_iter().enumerate() {
                    segments.push(Segment {
                        id: SegmentId {
                            cell: index,
                            index: index_in_cell as u8,
                        },
                        a,
                        b,
                    });
                }
                // The pairs of a tie are the two strands, each of which is two spokes.
            } else {
                for (index_in_cell, &(e1, e2)) in pairs.iter().enumerate() {
                    let (a, b) = (end(e1, &mut get), end(e2, &mut get));
                    segments.push(Segment {
                        id: SegmentId {
                            cell: index,
                            index: index_in_cell as u8,
                        },
                        a,
                        b,
                    });
                }
            }
            cells.push(CellResult {
                mask,
                q,
                rho,
                decision,
                pairs,
                segments,
            });
        }
    }
    cells
}

#[cfg(test)]
mod oracle;
#[cfg(test)]
mod unit_tests;
