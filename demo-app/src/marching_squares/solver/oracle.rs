//! A second, independent reading of the same grids, for the tests. It does not use the case table, the decider as the
//! solver writes it, or the stitcher. For each cell it lists the grid edges that the contour crosses, and pairs them by
//! looking at the corners: two crossed edges are one pair, and four are paired by the sign of the saddle value of the
//! bilinear surface, which isolates the corners whose sign is not the saddle's.

use super::Grid;
use std::collections::BTreeSet;

/// A grid edge as `(vertical, row, column)`, with the same meaning as the solver's keys but a form of its own.
pub(super) type RawEdge = (bool, usize, usize);

/// Every pair of grid edges joined in a cell, as `(cell row, cell column, lower edge, higher edge)`. Cells that are an
/// exact tie are left out, since they have strands and not pairs.
pub(super) fn reference_pairs(grid: &Grid, level: f64) -> BTreeSet<(usize, usize, RawEdge, RawEdge)> {
    let mut found = BTreeSet::new();
    for r in 0..grid.height - 1 {
        for c in 0..grid.width - 1 {
            // Corners in the order top left, top right, bottom right, bottom left.
            let g = [
                grid.at(r, c) - level,
                grid.at(r, c + 1) - level,
                grid.at(r + 1, c + 1) - level,
                grid.at(r + 1, c) - level,
            ];
            let above = g.map(|v| v > 0.0);
            // The four edges, as the corners they join and the grid edge they are.
            let edges: [(usize, usize, RawEdge); 4] = [
                (0, 1, (false, r, c)),
                (1, 2, (true, r, c + 1)),
                (3, 2, (false, r + 1, c)),
                (0, 3, (true, r, c)),
            ];
            let crossed: Vec<usize> = (0..4).filter(|&e| above[edges[e].0] != above[edges[e].1]).collect();
            let mut add = |a: usize, b: usize| {
                let (x, y) = (edges[a].2, edges[b].2);
                found.insert((r, c, x.min(y), x.max(y)));
            };
            match crossed.len() {
                2 => add(crossed[0], crossed[1]),
                4 => {
                    let saddle = (g[0] * g[2] - g[1] * g[3]) / (g[0] - g[1] + g[2] - g[3]);
                    if saddle == 0.0 {
                        continue;
                    }
                    // A corner whose sign is not the saddle's is cut off from the others, by the two edges at it.
                    for corner in (0..4).filter(|&k| (g[k] > 0.0) != (saddle > 0.0)) {
                        let adjacent: Vec<usize> =
                            (0..4).filter(|&e| edges[e].0 == corner || edges[e].1 == corner).collect();
                        add(adjacent[0], adjacent[1]);
                    }
                },
                _ => {},
            }
        }
    }
    found
}
