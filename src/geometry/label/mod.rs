//! Where an edge's label goes. Pure geometry: nothing here touches the DOM, so every rule can be tested without a
//! browser, and a label can be placed on every frame of a drag without measuring anything.
//!
//! A label is a small box with a weight or a name in it. It sits beside its connector, on one of the connector's
//! straight runs, close enough to be read as belonging to it and clear of the line, the arrowhead and every box. This
//! module picks the place from a short, fixed list of candidates, and does the same thing every time it is given the
//! same route.
//!
//! # The candidates
//!
//! A route has two to four points, so one to three straight runs. A run is _usable_ where it is not rounded away at a
//! corner, and not next to the arrowhead or the box it starts at. The corner trim is that of the path itself: the
//! smaller of the corner radius and half of each run meeting there.
//!
//! The longest usable run is preferred, and of runs that are as long, the later one, which is further from the source.
//! Several connectors can leave one box together, and their first runs are the ones that crowd. Four anchors are taken
//! along the runs: the middle, a quarter and three quarters of the longest, and the middle of the next longest.
//! Each anchor gets a candidate on the preferred side of its run and another on the opposite side, so there are at most
//! [`MAX_CANDIDATES`] of them. The preferred side is above a run that is closer to horizontal, and to the right of one
//! that is closer to vertical.
//!
//! # Choosing one
//!
//! Every candidate gets a hard score and a soft one.
//!
//! - The hard score is how many things it overlaps, and by how much. A box is padded a little first. Other labels, the
//!   arrowhead region and the connector's own other runs count too. A run counts for the length of it inside the
//!   candidate, weighted as a stroke is wide. A candidate that overlaps nothing is _clear_.
//! - The soft score is a whole number: `round(abs(dx) + abs(dy))`, plus 16 for the opposite side of a run. `dx` and `dy`
//!   are how far the candidate's anchor is from the first candidate's anchor, in user-space units. It is in scene units,
//!   not screen pixels, so zooming does not change it.
//!
//! Clear candidates come first. Of those, the lowest soft score wins, then the lowest ordinal. If none is clear, the
//! least bad wins: the fewest overlaps, then the least overlap, then the soft score, then the ordinal. The weight is
//! never hidden.
//!
//! # Staying put
//!
//! A label that moves about as its connector is dragged is hard to read. So the candidate a label had last time, by
//! ordinal, stays chosen while it is still clear. It is only left if another clear candidate beats its soft score by at
//! least [`SWITCH_THRESHOLD`] points. Eleven is not enough and twelve is. A candidate that is no longer clear is left at
//! once, whatever the threshold. Its position is always worked out again from the new route: only the ordinal is kept,
//! never the old coordinates.

use std::ops::Deref;
use svg_dom::root::utils::{Point, Rect, Size};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The most candidates a label ever has: up to four anchors, each on both sides of its run.
pub(crate) const MAX_CANDIDATES: usize = 8;

/// How many points of soft score another clear candidate must beat the label's current one by, to be moved to.
pub(crate) const SWITCH_THRESHOLD: u32 = 12;

/// What the opposite side of a run adds to the soft score.
pub(crate) const OPPOSITE_SIDE_PENALTY: u32 = 16;

/// The gap between a connector's line and the nearest edge of its label.
const LABEL_GAP: f64 = 3.0;

/// How much further every other label's box is padded, so that two labels never touch.
const LABEL_MARGIN: f64 = 2.0;

/// How much a box is padded before a label is kept clear of it.
const VERTEX_PADDING: f64 = 4.0;

/// How much of the start of the first run is left bare. It is next to the box the connector leaves.
const START_CLEARANCE: f64 = 6.0;

/// How far from the end of a route the arrowhead is taken to reach, and so how much of the last run is left bare.
pub(crate) const ARROW_REACH: f64 = 14.0;

/// What a connector's stroke is worth in area, per unit of its length that is inside a candidate.
const STROKE_ALLOWANCE: f64 = 3.0;

/// How far along its run each of the longest run's anchors is, and the one anchor of the next longest.
const ANCHOR_FRACTIONS: [f64; 3] = [0.5, 0.25, 0.75];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One place a label could go.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Candidate {
    /// The label's own box, in the scene's user space.
    pub(crate) rect: Rect,
    /// The point on the route that the box is beside.
    pub(crate) anchor: Point,
    /// Whether the box is on the preferred side of its run.
    pub(crate) preferred: bool,
}

impl Candidate {
    const NONE: Self = Self {
        rect: Rect {
            origin: Point { x: 0.0, y: 0.0 },
            size: Size { width: 0.0, height: 0.0 },
        },
        anchor: Point { x: 0.0, y: 0.0 },
        preferred: true,
    };
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The candidates for one label on one route, held inline, in ordinal order.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Candidates {
    items: [Candidate; MAX_CANDIDATES],
    len: usize,
}

impl Deref for Candidates {
    type Target = [Candidate];

    fn deref(&self) -> &[Candidate] {
        &self.items[..self.len]
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What a candidate overlaps: how many things, and how much in all. Nothing overlapped is [`Conflicts::CLEAR`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Conflicts {
    pub(crate) count: u32,
    pub(crate) area: f64,
}

impl Conflicts {
    pub(crate) const CLEAR: Self = Self { count: 0, area: 0.0 };

    pub(crate) fn is_clear(self) -> bool {
        self.count == 0
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything a label's place depends on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlacerInput<'a> {
    /// The connector's route: two to four points.
    pub(crate) route: &'a [Point],
    /// The radius the connector's corners are rounded by.
    pub(crate) corner_radius: f64,
    /// The size of the label's box, which has already been measured.
    pub(crate) label: Size,
    /// The boxes of the nodes the label must keep clear of.
    pub(crate) vertices: &'a [Rect],
    /// The boxes of the other labels that are already where they are going to stay.
    pub(crate) others: &'a [Rect],
    /// The ordinal of the candidate the label had before, if it had one.
    pub(crate) previous: Option<usize>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The place chosen for a label.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Placement {
    pub(crate) rect: Rect,
    /// The ordinal of the candidate, to give back as [`PlacerInput::previous`] next time.
    pub(crate) candidate: usize,
    pub(crate) conflicts: Conflicts,
    /// The soft score.
    pub(crate) score: u32,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A straight run of the route, with the part of it that can take a label.
#[derive(Clone, Copy)]
struct Run {
    from: Point,
    to: Point,
    length: f64,
    /// Where along the run the usable part starts and ends, as distances from `from`.
    usable: (f64, f64),
}

impl Run {
    fn usable_length(&self) -> f64 {
        self.usable.1 - self.usable.0
    }

    /// The point `fraction` of the way along the usable part.
    fn anchor(&self, fraction: f64) -> Point {
        let along = self.usable.0 + fraction * self.usable_length();
        let t = if self.length > 0.0 { along / self.length } else { 0.0 };
        Point::new(
            self.from.x + (self.to.x - self.from.x) * t,
            self.from.y + (self.to.y - self.from.y) * t,
        )
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn distance(a: Point, b: Point) -> f64 {
    (b.x - a.x).hypot(b.y - a.y)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The runs of `route`, each with the part of it that can take a label. A corner is trimmed by the smaller of `radius` and
/// half of each run meeting there, as the path itself rounds it. The first run leaves [`START_CLEARANCE`] bare, and the
/// last [`ARROW_REACH`]. A run that is too short for that is usable at its middle only.
fn runs(route: &[Point], radius: f64) -> ([Run; 3], usize) {
    let blank = Run {
        from: Point::new(0.0, 0.0),
        to: Point::new(0.0, 0.0),
        length: 0.0,
        usable: (0.0, 0.0),
    };
    let mut out = [blank; 3];
    let count = route.len().saturating_sub(1).min(3);
    let length = |i: usize| distance(route[i], route[i + 1]);
    for i in 0..count {
        let own = length(i);
        let before = if i == 0 {
            START_CLEARANCE
        } else {
            radius.min(own / 2.0).min(length(i - 1) / 2.0)
        };
        let after = if i + 2 == route.len() {
            ARROW_REACH
        } else {
            radius.min(own / 2.0).min(length(i + 1) / 2.0)
        };
        let usable = if own - after < before {
            (own / 2.0, own / 2.0)
        } else {
            (before, own - after)
        };
        out[i] = Run {
            from: route[i],
            to: route[i + 1],
            length: own,
            usable,
        };
    }
    (out, count)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The unit normal of a run that points to the side a label prefers: up for a run that is closer to horizontal, and
/// right for one that is closer to vertical.
fn preferred_normal(run: &Run) -> (f64, f64) {
    let (dx, dy) = (run.to.x - run.from.x, run.to.y - run.from.y);
    let length = dx.hypot(dy);
    if length == 0.0 {
        return (0.0, -1.0);
    }
    let (tx, ty) = (dx / length, dy / length);
    let normal = (-ty, tx);
    let wanted_up = tx.abs() >= ty.abs();
    let points_up = normal.1 < 0.0;
    let points_right = normal.0 > 0.0;
    let keep = if wanted_up { points_up } else { points_right };
    if keep { normal } else { (-normal.0, -normal.1) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The box of size `label` beside `anchor`, on the side `normal` points to, and just clear of the run: far enough along
/// the normal that the box, whatever the run's slope, does not touch the line.
fn beside(anchor: Point, normal: (f64, f64), label: Size) -> Rect {
    let reach = LABEL_GAP + (label.width * normal.0.abs() + label.height * normal.1.abs()) / 2.0;
    let centre = Point::new(anchor.x + normal.0 * reach, anchor.y + normal.1 * reach);
    Rect {
        origin: Point::new(centre.x - label.width / 2.0, centre.y - label.height / 2.0),
        size: label,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Every candidate place for a label of size `label` on `route`, in ordinal order: the preferred side of each anchor,
/// then the opposite side of each. Empty for a route of fewer than two points.
pub(crate) fn candidates(route: &[Point], corner_radius: f64, label: Size) -> Candidates {
    let mut out = Candidates {
        items: [Candidate::NONE; MAX_CANDIDATES],
        len: 0,
    };
    let (runs, count) = runs(route, corner_radius);
    if count == 0 {
        return out;
    }
    let mut order = [0_usize, 1, 2];
    let order = &mut order[..count];
    // Longest first, and of runs as long, the later one: it is further from the source.
    order.sort_by(|&a, &b| runs[b].usable_length().total_cmp(&runs[a].usable_length()).then(b.cmp(&a)));
    let mut anchors: [(Point, usize); 4] = [(Point::new(0.0, 0.0), 0); 4];
    let mut anchor_count = 0;
    for (rank, &run) in order.iter().enumerate().take(2) {
        let fractions: &[f64] = if rank == 0 { &ANCHOR_FRACTIONS } else { &ANCHOR_FRACTIONS[..1] };
        for &fraction in fractions {
            anchors[anchor_count] = (runs[run].anchor(fraction), run);
            anchor_count += 1;
        }
    }
    for preferred in [true, false] {
        for &(anchor, run) in &anchors[..anchor_count] {
            let normal = preferred_normal(&runs[run]);
            let normal = if preferred { normal } else { (-normal.0, -normal.1) };
            out.items[out.len] = Candidate {
                rect: beside(anchor, normal, label),
                anchor,
                preferred,
            };
            out.len += 1;
        }
    }
    out
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn padded(rect: Rect, by: f64) -> Rect {
    Rect {
        origin: Point::new(rect.origin.x - by, rect.origin.y - by),
        size: Size::new(rect.size.width + 2.0 * by, rect.size.height + 2.0 * by),
    }
}

/// The area two rectangles have in common, or `0` if they do not overlap.
fn overlap_area(a: Rect, b: Rect) -> f64 {
    let width = (a.origin.x + a.size.width).min(b.origin.x + b.size.width) - a.origin.x.max(b.origin.x);
    let height = (a.origin.y + a.size.height).min(b.origin.y + b.size.height) - a.origin.y.max(b.origin.y);
    if width > 0.0 && height > 0.0 { width * height } else { 0.0 }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// How much of the segment from `a` to `b` is inside `rect`, by Liang and Barsky's clipping. `0` if none of it is.
fn clipped_length(a: Point, b: Point, rect: Rect) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let (mut enter, mut leave) = (0.0_f64, 1.0_f64);
    let sides = [
        (-dx, a.x - rect.origin.x),
        (dx, rect.origin.x + rect.size.width - a.x),
        (-dy, a.y - rect.origin.y),
        (dy, rect.origin.y + rect.size.height - a.y),
    ];
    for (direction, distance_inside) in sides {
        if direction == 0.0 {
            if distance_inside < 0.0 {
                return 0.0;
            }
        } else {
            let t = distance_inside / direction;
            if direction < 0.0 {
                enter = enter.max(t);
            } else {
                leave = leave.min(t);
            }
            if enter > leave {
                return 0.0;
            }
        }
    }
    (leave - enter).max(0.0) * dx.hypot(dy)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the box `rect` of a candidate overlaps. See "Choosing one" in this module's own doc comment.
fn conflicts_of(rect: Rect, input: &PlacerInput<'_>) -> Conflicts {
    let mut found = Conflicts::CLEAR;
    let mut count = |area: f64| {
        if area > 0.0 {
            found.count += 1;
            found.area += area;
        }
    };
    for &vertex in input.vertices {
        count(overlap_area(rect, padded(vertex, VERTEX_PADDING)));
    }
    let margin = padded(rect, LABEL_MARGIN);
    for &other in input.others {
        count(overlap_area(margin, other));
    }
    if let Some(&end) = input.route.last() {
        let reach = Rect {
            origin: Point::new(end.x - ARROW_REACH, end.y - ARROW_REACH),
            size: Size::new(2.0 * ARROW_REACH, 2.0 * ARROW_REACH),
        };
        count(overlap_area(rect, reach));
    }
    for pair in input.route.windows(2) {
        count(clipped_length(pair[0], pair[1], rect) * STROKE_ALLOWANCE);
    }
    found
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The soft score of a candidate: how far its anchor is from the first candidate's, and a penalty for the far side.
fn soft_score(candidate: &Candidate, first: &Candidate) -> u32 {
    let offset = (candidate.anchor.x - first.anchor.x).abs() + (candidate.anchor.y - first.anchor.y).abs();
    // Not negative, and not past what a label could ever be moved by on a screen, so the conversion cannot lose a sign.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let points = offset.round().min(f64::from(u32::MAX)) as u32;
    points.saturating_add(if candidate.preferred { 0 } else { OPPOSITE_SIDE_PENALTY })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The index of the candidate to take from `scored`, which holds each one's conflicts and soft score in ordinal order.
/// `previous` is the ordinal the label had before. See "Choosing one" and "Staying put" in this module's own doc comment.
fn choose(scored: &[(Conflicts, u32)], previous: Option<usize>) -> Option<usize> {
    let best_clear = scored
        .iter()
        .enumerate()
        .filter(|(_, (conflicts, _))| conflicts.is_clear())
        .min_by_key(|&(index, &(_, score))| (score, index))
        .map(|(index, _)| index);
    let previous = previous.filter(|&p| p < scored.len() && scored[p].0.is_clear());
    match (previous, best_clear) {
        (Some(previous), Some(best)) if scored[previous].1.saturating_sub(scored[best].1) >= SWITCH_THRESHOLD => {
            Some(best)
        },
        (Some(previous), _) => Some(previous),
        (None, Some(best)) => Some(best),
        (None, None) => scored
            .iter()
            .enumerate()
            .min_by(|(i, (a, a_score)), (j, (b, b_score))| {
                a.count
                    .cmp(&b.count)
                    .then(a.area.total_cmp(&b.area))
                    .then(a_score.cmp(b_score))
                    .then(i.cmp(j))
            })
            .map(|(index, _)| index),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The part of the scene that can matter to a label on `route`: no candidate, and none of the clearance around one,
/// reaches outside it. A node or a label that does not overlap this box cannot conflict with any candidate, so the
/// caller can leave it out of [`PlacerInput`] without changing the result.
///
/// Every candidate is a box beside a point of the route, at most the label's own width and height and a small gap away,
/// and the clearances kept around nodes and labels are smaller than the slack added here.
pub(crate) fn reach(route: &[Point], label: Size) -> Option<Rect> {
    let first = route.first()?;
    let (mut low, mut high) = (*first, *first);
    for point in route {
        low = Point::new(low.x.min(point.x), low.y.min(point.y));
        high = Point::new(high.x.max(point.x), high.y.max(point.y));
    }
    let by = LABEL_GAP + label.width + label.height + VERTEX_PADDING + LABEL_MARGIN + STROKE_ALLOWANCE + 8.0;
    Some(Rect {
        origin: Point::new(low.x - by, low.y - by),
        size: Size::new(high.x - low.x + 2.0 * by, high.y - low.y + 2.0 * by),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Chooses where the label of `input` goes: the place it would stay in, if it still can, and otherwise the best one. See
/// this module's own doc comment for the rules. `None` for a route of fewer than two points, which has nowhere to put one.
pub(crate) fn place(input: &PlacerInput<'_>) -> Option<Placement> {
    let all = candidates(input.route, input.corner_radius, input.label);
    let first = all.first()?;
    let mut scored: [(Conflicts, u32); MAX_CANDIDATES] = [(Conflicts::CLEAR, 0); MAX_CANDIDATES];
    for (i, candidate) in all.iter().enumerate() {
        scored[i] = (conflicts_of(candidate.rect, input), soft_score(candidate, first));
    }
    let scored = &scored[..all.len()];

    let chosen = choose(scored, input.previous)?;
    Some(Placement {
        rect: all[chosen].rect,
        candidate: chosen,
        conflicts: scored[chosen].0,
        score: scored[chosen].1,
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
