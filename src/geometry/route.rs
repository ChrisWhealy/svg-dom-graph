use svg_dom::root::utils::Point;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The most points a route can have.
pub(crate) const MAX_POINTS: usize = 6;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Up to [`MAX_POINTS`] connector route points, stored inline rather than on the heap.
///
/// Every route this crate computes has at most six points: a straight connector always has two, and an elbow has two to
/// six — see [`straight_route`] and [`elbow_route`]. Six is the most an elbow needs when its two ends face away from
/// each other, which takes a short stub out of each end and a channel between them. A fixed-size buffer avoids a heap
/// allocation on every redraw, which matters here since a drag redraws every incident edge on every pointer-move.
///
/// Derefs to `&[Point]`, so it can be used almost anywhere a point slice is expected.
#[derive(Clone, Copy)]
pub(crate) struct Route {
    points: [Point; MAX_POINTS],
    len: usize,
}

impl Route {
    pub(crate) fn new() -> Self {
        Self {
            points: [Point::new(0.0, 0.0); MAX_POINTS],
            len: 0,
        }
    }

    /// Appends `point`, unless it exactly repeats the route's own last point.
    ///
    /// Mirrors `Vec::dedup`'s consecutive-only rule. A bend that collapses onto an anchor still leaves a straight
    /// route, not a zero-length segment.
    ///
    /// # Panics
    ///
    /// Panics if the route already holds [`MAX_POINTS`] points. Every caller in this module pushes at most that many, so this can only
    /// fire from a bug in this module itself, not from anything external.
    pub(crate) fn push(&mut self, point: Point) {
        if self.len > 0 && self.points[self.len - 1] == point {
            return;
        }
        assert!(
            self.len < self.points.len(),
            "Route cannot hold more than {} points",
            self.points.len()
        );
        self.points[self.len] = point;
        self.len += 1;
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl std::ops::Deref for Route {
    type Target = [Point];

    fn deref(&self) -> &[Point] {
        &self.points[..self.len]
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl PartialEq for Route {
    /// Compares the two routes' own points, in order. Unused capacity past each route's own length is never compared.
    fn eq(&self, other: &Self) -> bool {
        self.points[..self.len] == other.points[..other.len]
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl std::fmt::Debug for Route {
    /// Shows only the route's own points, not its unused capacity.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Route").field(&&self.points[..self.len]).finish()
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A two-point route between two already-anchored endpoints.
///
/// Takes `start`/`end` as plain arguments, rather than computing them itself, so the same two-point route works
/// whichever rule chose the anchors — [`boundary_point`]'s own continuous crossing, or [`snapped_anchor`]'s
/// evenly-spaced candidates.
pub(crate) fn straight_route(start: Point, end: Point) -> Route {
    let mut route = Route::new();
    route.push(start);
    route.push(end);
    route
}
