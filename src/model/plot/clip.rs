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
    // A position that is not finite has no segment to cut. Callers place everything with a finite limit, so this is a guard.
    if ![from.0, from.1, to.0, to.1].iter().all(|v| v.is_finite()) {
        return None;
    }
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

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The part of the polygon through `points` that is inside `bounds`, by the method of Sutherland and Hodgman. Empty if
/// none is, or if a point is not finite. A polygon is closed, so the last point joins the first.
///
/// A polygon that is cut into two pieces comes out as one, joined along the edge of `bounds` by a line with no area
/// between its two sides. That is no matter for a fill.
pub(crate) fn clip_polygon(points: &[(f64, f64)], bounds: Bounds) -> Vec<(f64, f64)> {
    if points.len() < 3 || !points.iter().all(|p| p.0.is_finite() && p.1.is_finite()) {
        return Vec::new();
    }
    // The point where the segment `from`–`to` crosses the vertical line at `x`, or the horizontal one at `y`.
    let at_x =
        |from: (f64, f64), to: (f64, f64), x: f64| (x, from.1 + (x - from.0) / (to.0 - from.0) * (to.1 - from.1));
    let at_y =
        |from: (f64, f64), to: (f64, f64), y: f64| (from.0 + (y - from.1) / (to.1 - from.1) * (to.0 - from.0), y);

    let mut polygon = points.to_vec();
    polygon = clip_to_edge(&polygon, |p| p.0 >= bounds.left, |a, b| at_x(a, b, bounds.left));
    polygon = clip_to_edge(&polygon, |p| p.0 <= bounds.right, |a, b| at_x(a, b, bounds.right));
    polygon = clip_to_edge(&polygon, |p| p.1 >= bounds.top, |a, b| at_y(a, b, bounds.top));
    clip_to_edge(&polygon, |p| p.1 <= bounds.bottom, |a, b| at_y(a, b, bounds.bottom))
}

/// What is left of the polygon `input` on the inside of one edge, which `inside` says which side of, and `cross` finds
/// where a segment that goes from one side to the other meets.
fn clip_to_edge(
    input: &[(f64, f64)],
    inside: impl Fn((f64, f64)) -> bool,
    cross: impl Fn((f64, f64), (f64, f64)) -> (f64, f64),
) -> Vec<(f64, f64)> {
    let mut output = Vec::with_capacity(input.len() + 4);
    for (i, &current) in input.iter().enumerate() {
        let previous = input[(i + input.len() - 1) % input.len()];
        match (inside(previous), inside(current)) {
            (true, true) => output.push(current),
            (true, false) => output.push(cross(previous, current)),
            (false, true) => {
                output.push(cross(previous, current));
                output.push(current);
            },
            (false, false) => {},
        }
    }
    output
}
