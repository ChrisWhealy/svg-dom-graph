//! The shaded pieces of a [`PlotArea`](super::PlotArea): where the line crosses the baseline, and the polygons on each
//! side of it. Pure.

use super::items::PlotPoint;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One piece of an area: a run of the line that is all on one side of the baseline.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Piece {
    /// Whether the run is above the baseline, or below it.
    pub(crate) above: bool,
    /// The corners of the polygon, in values: the run, then back along the baseline.
    pub(crate) corners: Vec<(f64, f64)>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Part of one segment of the line: the side of the baseline it is on, and where it starts and ends.
type Part = (bool, (f64, f64), (f64, f64));

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The pieces of the area between `points` and the horizontal line at `baseline`, in order along the line.
///
/// The line is cut where it crosses the baseline, so that each piece is on one side. A stretch that is on the baseline
/// has no area. A point that is not finite breaks the line, as does a stretch that is on the baseline.
pub(crate) fn pieces(points: &[PlotPoint], baseline: f64) -> Vec<Piece> {
    let mut runs: Vec<(bool, Vec<(f64, f64)>)> = Vec::new();
    // The run that the segment before is part of, if the segment before ended where this one starts.
    let mut open = false;
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if ![a.x, a.y, b.x, b.y, baseline].iter().all(|v| v.is_finite()) {
            open = false;
            continue;
        }
        let (da, db) = (a.y - baseline, b.y - baseline);
        // Two finite heights can be further apart than a number can say, and the crossing cannot be found.
        if !da.is_finite() || !db.is_finite() {
            open = false;
            continue;
        }
        // The segment as one or two parts, each with the side it is on.
        let mut parts: Vec<Part> = Vec::new();
        // The signs are compared, and the heights are not multiplied, since the product of two small heights is zero.
        if (da > 0.0 && db < 0.0) || (da < 0.0 && db > 0.0) {
            // Worked out as a ratio, since the difference of two heights can be more than a number can say. The ratio
            // is below zero, so `t` is from 0 up to 1. It is mixed in as `a.x * (1 - t) + b.x * t`, since the distance
            // between the two places can be more than a number can say too, and the mix is kept between them.
            let t = 1.0 / (1.0 - db / da);
            let x = (a.x * (1.0 - t) + b.x * t).clamp(a.x.min(b.x), a.x.max(b.x));
            let cross = (x, baseline);
            parts.push((da > 0.0, (a.x, a.y), cross));
            parts.push((db > 0.0, cross, (b.x, b.y)));
        } else if da + db != 0.0 {
            parts.push((da + db > 0.0, (a.x, a.y), (b.x, b.y)));
        }
        if parts.is_empty() {
            open = false;
            continue;
        }
        for (above, from, to) in parts {
            let continues = open
                && runs
                    .last()
                    .is_some_and(|(side, run)| *side == above && run.last() == Some(&from));
            if continues {
                if let Some((_, run)) = runs.last_mut() {
                    run.push(to);
                }
            } else {
                runs.push((above, vec![from, to]));
            }
            open = true;
        }
    }
    runs.into_iter()
        .map(|(above, run)| {
            let (first, last) = (run[0], run[run.len() - 1]);
            let mut corners = vec![(first.0, baseline)];
            corners.extend(run);
            corners.push((last.0, baseline));
            Piece { above, corners }
        })
        .collect()
}
