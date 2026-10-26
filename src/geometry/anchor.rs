//! Where a connector meets the edge of a box.

use super::{plane::centre, side::Side};
use svg_dom::root::utils::{Point, Rect};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// True for `East`/`West`.
///
/// These are the sides a horizontal connector segment leaves from or arrives at.
pub(crate) fn is_horizontal(side: Side) -> bool {
    matches!(side, Side::East | Side::West)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The midpoint of the side of `rect` first intersected by a ray from its centre towards `towards` (i.e. the centre of
/// the target node), and which side that is.
///
/// Picks the side the same way [`boundary_point`] picks its crossing point: whichever axis's offset from `rect`'s
/// centre reaches that axis's half-extent first.
///
/// The result sits at the exact midpoint of the chosen side, not at the ray's own crossing point. This lets an elbowed
/// connector leave a box travelling exactly horizontally or exactly vertically.
///
/// Since direction is undefined at zero distance, this function arbitrarily returns `rect`'s centre and `Side::East`
/// when `towards` is exactly the centre.
pub(crate) fn edge_anchor(rect: Rect, towards: Point) -> (Point, Side) {
    let centre = centre(rect);
    let dx = towards.x - centre.x;
    let dy = towards.y - centre.y;

    if dx == 0.0 && dy == 0.0 {
        return (centre, Side::East);
    }

    let half_w = rect.size.width / 2.0;
    let half_h = rect.size.height / 2.0;

    let scale_x = if dx == 0.0 { f64::INFINITY } else { half_w / dx.abs() };
    let scale_y = if dy == 0.0 { f64::INFINITY } else { half_h / dy.abs() };

    if scale_x <= scale_y {
        let side = if dx >= 0.0 { Side::East } else { Side::West };
        (Point::new(centre.x + half_w.copysign(dx), centre.y), side)
    } else {
        let side = if dy >= 0.0 { Side::South } else { Side::North };
        (Point::new(centre.x, centre.y + half_h.copysign(dy)), side)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The side of `rect` first intersected by a ray from its centre towards `towards`. The returned point snaps to
/// whichever of `fixing_points` evenly-spaced candidates on that side sits nearest the ray's own crossing point.
///
/// Picks the side the same way [`edge_anchor`] does. Divides that side into `fixing_points + 1` equal segments, and
/// returns whichever of the `fixing_points` internal division points is closest to the continuous crossing position
/// [`boundary_point`] would have returned.
///
/// `fixing_points == 1` always lands on the side's own midpoint — the same point [`edge_anchor`] always returns,
/// regardless of `towards`.
///
/// Returns `rect`'s centre and `Side::East` when `towards` is exactly the centre. Direction is undefined at zero
/// distance. `fixing_points` is treated as at least `1`; the caller validates `>= 1` before this is ever reached.
pub(crate) fn snapped_anchor(rect: Rect, towards: Point, fixing_points: u8) -> (Point, Side) {
    let centre = centre(rect);
    let dx = towards.x - centre.x;
    let dy = towards.y - centre.y;

    if dx == 0.0 && dy == 0.0 {
        return (centre, Side::East);
    }

    let half_w = rect.size.width / 2.0;
    let half_h = rect.size.height / 2.0;

    let scale_x = if dx == 0.0 { f64::INFINITY } else { half_w / dx.abs() };
    let scale_y = if dy == 0.0 { f64::INFINITY } else { half_h / dy.abs() };

    // At least 1, so `divisions` below is always `>= 2` and the clamp a few lines down always has `min <= max`.
    let candidates = f64::from(fixing_points.max(1));
    let divisions = candidates + 1.0;

    if scale_x <= scale_y {
        let side = if dx >= 0.0 { Side::East } else { Side::West };
        let x = centre.x + half_w.copysign(dx);
        let crossing_y = centre.y + dy * scale_x;
        let top = rect.origin.y;
        let index = ((crossing_y - top) / rect.size.height * divisions)
            .round()
            .clamp(1.0, candidates);
        let y = top + rect.size.height * index / divisions;
        (Point::new(x, y), side)
    } else {
        let side = if dy >= 0.0 { Side::South } else { Side::North };
        let y = centre.y + half_h.copysign(dy);
        let crossing_x = centre.x + dx * scale_y;
        let left = rect.origin.x;
        let index = ((crossing_x - left) / rect.size.width * divisions)
            .round()
            .clamp(1.0, candidates);
        let x = left + rect.size.width * index / divisions;
        (Point::new(x, y), side)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The side of `rect` a ray from its own already-known `centre` toward `towards` first crosses, together with the raw
/// coordinate the ray actually crosses at along that side. That coordinate is an x for a north/south side and a y for
/// an east/west one.
///
/// Picks the side the same way [`edge_anchor`] does. Returns the crossing coordinate itself, not a point snapped to any
/// candidate. [`binary_operator_anchors`] needs to compare two crossings before deciding where either one finally
/// lands. Once decided, it passes the winning crossing straight to [`anchor_from_crossing`] rather than recomputing it
/// there.
///
/// Takes `centre` rather than `rect` alone since [`binary_operator_anchors`] calls this twice, against the same rect,
/// and computes `centre` itself only once for both calls.
///
/// Returns `rect`'s own centre y and `Side::East` when `towards` is exactly the centre, the same degenerate case
/// [`edge_anchor`] returns its own centre for.
pub(super) fn side_and_crossing(rect: Rect, centre: Point, towards: Point) -> (Side, f64) {
    let dx = towards.x - centre.x;
    let dy = towards.y - centre.y;

    if dx == 0.0 && dy == 0.0 {
        return (Side::East, centre.y);
    }

    let half_w = rect.size.width / 2.0;
    let half_h = rect.size.height / 2.0;

    let scale_x = if dx == 0.0 { f64::INFINITY } else { half_w / dx.abs() };
    let scale_y = if dy == 0.0 { f64::INFINITY } else { half_h / dy.abs() };

    if scale_x <= scale_y {
        let side = if dx >= 0.0 { Side::East } else { Side::West };
        (side, centre.y + dy * scale_x)
    } else {
        let side = if dy >= 0.0 { Side::South } else { Side::North };
        (side, centre.x + dx * scale_y)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The anchor point on `rect`'s own `side`, given the ray's own already-known `crossing` coordinate along it — see
/// [`side_and_crossing`]'s own doc comment for what `crossing` means for a given `side`.
///
/// [`binary_operator_anchors`]'s own different-side branch is the only caller. It already has `side` and `crossing` in
/// hand for both operands, from its own initial [`side_and_crossing`] calls. So it reaches here instead of calling
/// [`edge_anchor`]/[`snapped_anchor`]. Either of those would redo the same ray/rectangle intersection just to re-derive
/// the side and crossing this already has.
///
/// `fixing_points: None` returns `side`'s own plain midpoint, ignoring `crossing` entirely — the same point
/// [`edge_anchor`] itself would return. `Some(n)` quantises `crossing` to the nearest of `n` evenly spaced candidates
/// on `side` instead — the same division [`snapped_anchor`] itself would use.
pub(super) fn anchor_from_crossing(
    rect: Rect,
    centre: Point,
    side: Side,
    crossing: f64,
    fixing_points: Option<u8>,
) -> Point {
    let Some(fixing_points) = fixing_points else {
        return match side {
            Side::East => Point::new(rect.origin.x + rect.size.width, centre.y),
            Side::West => Point::new(rect.origin.x, centre.y),
            Side::South => Point::new(centre.x, rect.origin.y + rect.size.height),
            Side::North => Point::new(centre.x, rect.origin.y),
        };
    };

    // At least 1, so `divisions` below is always `>= 2`, mirroring `snapped_anchor`'s own defensive minimum.
    let candidates = f64::from(fixing_points.max(1));
    let divisions = candidates + 1.0;

    if is_horizontal(side) {
        let x = if side == Side::East { rect.origin.x + rect.size.width } else { rect.origin.x };
        let top = rect.origin.y;
        let index = ((crossing - top) / rect.size.height * divisions).round().clamp(1.0, candidates);
        Point::new(x, top + rect.size.height * index / divisions)
    } else {
        let y = if side == Side::South {
            rect.origin.y + rect.size.height
        } else {
            rect.origin.y
        };
        let left = rect.origin.x;
        let index = ((crossing - left) / rect.size.width * divisions).round().clamp(1.0, candidates);
        Point::new(left + rect.size.width * index / divisions, y)
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The `crossing` [`anchor_from_crossing`] needs for `rect`'s own explicitly forced `side`. It is the ray from `centre`
/// toward `towards`, projected onto the infinite line `side` itself lies on. That is the same coordinate
/// [`side_and_crossing`] would have returned, had it picked `side` itself, rather than whichever side its own ray-cast
/// actually prefers.
///
/// Falls back to `centre`'s own coordinate on that axis when the ray runs parallel to `side`. That happens for an
/// East/West side with `towards` at exactly `centre`'s own y. It also happens for a North/South side with `towards` at
/// exactly `centre`'s own x. Direction along that axis is undefined at zero distance, the same degenerate case
/// [`side_and_crossing`] and [`snapped_anchor`] both already fall back for.
pub(super) fn forced_crossing(centre: Point, towards: Point, rect: Rect, side: Side) -> f64 {
    let dx = towards.x - centre.x;
    let dy = towards.y - centre.y;

    if is_horizontal(side) {
        let fixed_x = if side == Side::East { rect.origin.x + rect.size.width } else { rect.origin.x };
        if dx == 0.0 { centre.y } else { centre.y + dy * (fixed_x - centre.x) / dx }
    } else {
        let fixed_y = if side == Side::South {
            rect.origin.y + rect.size.height
        } else {
            rect.origin.y
        };
        if dy == 0.0 { centre.x } else { centre.x + dx * (fixed_y - centre.y) / dy }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The anchor point on `rect`'s own `side`, forced by the caller rather than picked by [`edge_anchor`]/
/// [`snapped_anchor`]'s own ray-cast. See [`Scene::add_edge_with`](crate::scene::Scene::add_edge_with)'s own doc
/// comment for when a caller needs this instead of leaving the side to be chosen automatically.
///
/// `fixing_points: None` returns `side`'s own plain midpoint, ignoring `towards` entirely — the same point
/// [`edge_anchor`] itself would return for whichever side its own ray-cast happened to prefer. `Some(n)` snaps to the
/// nearest of `n` evenly spaced candidates on `side` instead, positioned via the ray toward `towards` — the same
/// division [`snapped_anchor`] would use.
pub(crate) fn forced_anchor(rect: Rect, towards: Point, side: Side, fixing_points: Option<u8>) -> Point {
    let centre = centre(rect);
    let crossing = forced_crossing(centre, towards, rect, side);
    anchor_from_crossing(rect, centre, side, crossing, fixing_points)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The anchor point `fraction` of the way along `rect`'s own `side`, from its west end (North/South) or north end
/// (East/West). `0.0` is that end and `1.0` the other. Independent of any other node's position, so a connector
/// pinned this way leaves at a fixed place however the far endpoint moves.
pub(crate) fn position_anchor(rect: Rect, side: Side, fraction: f64) -> Point {
    match side {
        Side::North => Point::new(rect.origin.x + rect.size.width * fraction, rect.origin.y),
        Side::South => Point::new(rect.origin.x + rect.size.width * fraction, rect.origin.y + rect.size.height),
        Side::West => Point::new(rect.origin.x, rect.origin.y + rect.size.height * fraction),
        Side::East => Point::new(rect.origin.x + rect.size.width, rect.origin.y + rect.size.height * fraction),
    }
}
