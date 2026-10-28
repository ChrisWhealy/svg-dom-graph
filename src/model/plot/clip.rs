//! Clipping what is plotted to the plotted area, so that a line that leaves it is cut at its edge and a point beyond it
//! is not drawn. Pure.

/// A rectangle of the page: left, top, right and bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Bounds {
    pub(crate) left: f64,
    pub(crate) top: f64,
    pub(crate) right: f64,
    pub(crate) bottom: f64,
}

impl Bounds {
    pub(crate) fn contains(&self, (x, y): (f64, f64)) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The part of the segment `from`–`to` that is inside `bounds`, by the method of Liang and Barsky. `None` if none is.
pub(crate) fn clip_segment(from: (f64, f64), to: (f64, f64), bounds: Bounds) -> Option<((f64, f64), (f64, f64))> {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
    for (p, q) in [
        (-dx, from.0 - bounds.left),
        (dx, bounds.right - from.0),
        (-dy, from.1 - bounds.top),
        (dy, bounds.bottom - from.1),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
        } else {
            let ratio = q / p;
            if p < 0.0 {
                enter = enter.max(ratio);
            } else {
                leave = leave.min(ratio);
            }
        }
    }
    (enter <= leave).then_some((
        (from.0 + enter * dx, from.1 + enter * dy),
        (from.0 + leave * dx, from.1 + leave * dy),
    ))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The runs of a polyline that are inside `bounds`. A point that is `None` could not be placed, and breaks the line, as
/// does a stretch outside. A run has at least two points.
pub(crate) fn clip_polyline(points: &[Option<(f64, f64)>], bounds: Bounds) -> Vec<Vec<(f64, f64)>> {
    let mut runs: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut continues = false;
    for pair in points.windows(2) {
        let clipped = match (pair[0], pair[1]) {
            (Some(a), Some(b)) => clip_segment(a, b, bounds),
            _ => None,
        };
        let Some((a, b)) = clipped else {
            continues = false;
            continue;
        };
        // The segment carries on from the run before it if the last point of that run is where this one starts.
        let joins = continues && runs.last().and_then(|run| run.last()) == Some(&a);
        if joins {
            if let Some(run) = runs.last_mut() {
                run.push(b);
            }
        } else {
            runs.push(vec![a, b]);
        }
        continues = pair[1] == Some(b);
    }
    runs
}
