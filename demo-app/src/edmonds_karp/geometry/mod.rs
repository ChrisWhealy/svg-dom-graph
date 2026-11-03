//! Where the vertices, the edges and the residual lanes are drawn, and where their labels are. Pure: nothing here
//! touches the DOM.
//!
//! Every route is worked out once for a graph, from where its vertices are, and then it never moves. An edge's route
//! depends on its two ends and its ID, and a residual lane's on those and which way it goes, and neither depends on how
//! much flow there is. So a lane that has no room is hidden, and the others stay where they were.
//!
//! The edges between one pair of vertices are spread to either side of the straight line between them, in the order of
//! their IDs. Two edges that go opposite ways, as `e2` and `e3` do, are on opposite sides. The residual lanes of the
//! pair are a family of their own, outside the edges: two for each edge, a forward one and a reverse one.

use super::{
    fixtures::Fixture,
    projection::NODE_RADIUS,
    solver::{Direction, Trace},
};

/// How far apart the original edges of a pair are, and how far the nearest residual lane is outside the farthest of
/// them, and how far apart the lanes are. These are how far a curve's middle is from the straight line.
const EDGE_STEP: f64 = 44.0;
const LANE_FIRST: f64 = 26.0;
const LANE_STEP: f64 = 24.0;

/// How far a label is from its curve.
const LABEL_GAP: f64 = 12.0;

/// Where along its curve a label may be written, as a fraction of its length, in the order that they are tried. The
/// first that does not meet a label that is placed already, or a vertex, is used.
const LABEL_STOPS: [f64; 17] = [
    0.5, 0.44, 0.56, 0.38, 0.62, 0.32, 0.68, 0.26, 0.74, 0.2, 0.8, 0.14, 0.86, 0.1, 0.9, 0.08, 0.92,
];

/// The most that a label can fill: the text of the longest label of each kind at 7 units a character, with room round
/// it, and 14 units high. Labels are placed so that these boxes never meet, so no text can.
const EDGE_LABEL_WIDTH: f64 = 62.0;
const LANE_LABEL_WIDTH: f64 = 48.0;
const LABEL_BOX_HEIGHT: f64 = 14.0;

/// How far a label's box is kept from the middle of a vertex, past the vertex's radius.
const VERTEX_ROOM: f64 = 4.0;

/// The size of the text of a label, in the units of the scene, for the tests of whether labels meet.
#[cfg(test)]
pub(crate) const LABEL_HEIGHT: f64 = 14.0;
#[cfg(test)]
pub(crate) const LABEL_CHARACTER: f64 = 7.0;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where one curve goes: its path, the middle of the curve, and where its label is written.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Route {
    pub(crate) d: String,
    pub(crate) mid: (f64, f64),
    pub(crate) label: (f64, f64),
}

/// Where everything is for one graph.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Geometry {
    pub(crate) vertices: Vec<(f64, f64)>,
    pub(crate) edges: Vec<Route>,
    /// The residual lanes, two for each edge: the forward one at `2 * e`, and the reverse one after it.
    pub(crate) lanes: Vec<Route>,
    /// Where the mark of each vertex's place in the queue is.
    pub(crate) badges: Vec<(f64, f64)>,
}

impl Geometry {
    pub(crate) fn lane(&self, e: usize, direction: Direction) -> &Route {
        &self.lanes[2 * e + usize::from(direction == Direction::Reverse)]
    }
}

type Point = (f64, f64);

fn unit(from: Point, to: Point) -> Point {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    let length = dx.hypot(dy).max(f64::EPSILON);
    (dx / length, dy / length)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A curve, as it is drawn: where it starts, where it ends, the point that bends it, and which side of it a label goes.
struct Curve {
    start: Point,
    control: Point,
    end: Point,
    normal: Point,
    side: f64,
    straight: bool,
}

impl Curve {
    /// The curve from `a` to `b` whose middle is `offset` to the side of the straight line, where the side is `normal`.
    /// Its ends are on the edges of the vertices, and it leaves each one towards its control point.
    fn new(a: Point, b: Point, normal: Point, offset: f64) -> Self {
        let mid = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let control = (mid.0 + normal.0 * 2.0 * offset, mid.1 + normal.1 * 2.0 * offset);
        let (towards_a, towards_b) = (unit(a, control), unit(b, control));
        Self {
            start: (a.0 + towards_a.0 * NODE_RADIUS, a.1 + towards_a.1 * NODE_RADIUS),
            end: (b.0 + towards_b.0 * NODE_RADIUS, b.1 + towards_b.1 * NODE_RADIUS),
            control,
            normal,
            side: if offset < 0.0 { -1.0 } else { 1.0 },
            straight: offset == 0.0,
        }
    }

    /// The point a fraction `t` of the way along the curve.
    fn at(&self, t: f64) -> Point {
        let (u, v) = (1.0 - t, t);
        if self.straight {
            (self.start.0 * u + self.end.0 * v, self.start.1 * u + self.end.1 * v)
        } else {
            (
                u * u * self.start.0 + 2.0 * u * v * self.control.0 + v * v * self.end.0,
                u * u * self.start.1 + 2.0 * u * v * self.control.1 + v * v * self.end.1,
            )
        }
    }

    /// Where a label a fraction `t` of the way along is written: beside the curve, on the outer side.
    fn label_at(&self, t: f64) -> Point {
        let on = self.at(t);
        (
            on.0 + self.normal.0 * self.side * LABEL_GAP,
            on.1 + self.normal.1 * self.side * LABEL_GAP,
        )
    }

    fn path(&self) -> String {
        if self.straight {
            format!("M{:.1} {:.1}L{:.1} {:.1}", self.start.0, self.start.1, self.end.0, self.end.1)
        } else {
            format!(
                "M{:.1} {:.1}Q{:.1} {:.1} {:.1} {:.1}",
                self.start.0, self.start.1, self.control.0, self.control.1, self.end.0, self.end.1
            )
        }
    }
}

type Box4 = (f64, f64, f64, f64);

fn label_box(at: Point, width: f64) -> Box4 {
    (
        at.0 - width / 2.0,
        at.1 - LABEL_BOX_HEIGHT / 2.0,
        at.0 + width / 2.0,
        at.1 + LABEL_BOX_HEIGHT / 2.0,
    )
}

fn boxes_meet(a: Box4, b: Box4) -> bool {
    a.0 < b.2 && b.0 < a.2 && a.1 < b.3 && b.1 < a.3
}

/// Whether the box is within [`VERTEX_ROOM`] of a vertex's edge.
fn box_on_vertex(b: Box4, vertex: Point) -> bool {
    let nearest = (vertex.0.clamp(b.0, b.2), vertex.1.clamp(b.1, b.3));
    (nearest.0 - vertex.0).hypot(nearest.1 - vertex.1) < NODE_RADIUS + VERTEX_ROOM
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Places the label of each curve, in the order given, at the first of the stops along it where its box meets no label
/// that is placed already and no vertex. A curve with no free stop has its label at the middle.
fn place_labels(vertices: &[Point], curves: &[(&Curve, f64)]) -> Vec<Point> {
    let mut placed: Vec<Box4> = Vec::new();
    let mut at = Vec::with_capacity(curves.len());
    for (curve, width) in curves {
        let free = LABEL_STOPS
            .iter()
            .map(|&t| curve.label_at(t))
            .find(|&p| {
                let b = label_box(p, *width);
                !placed.iter().any(|&other| boxes_meet(b, other)) && !vertices.iter().any(|&v| box_on_vertex(b, v))
            })
            .unwrap_or_else(|| curve.label_at(0.5));
        placed.push(label_box(free, *width));
        at.push(free);
    }
    at
}

/// The geometry of `trace` drawn with the vertices where `fixture` puts them.
pub(crate) fn geometry(fixture: &Fixture, trace: &Trace) -> Geometry {
    let vertices: Vec<Point> = fixture.coords.iter().take(trace.vertex_count()).copied().collect();
    let edges = trace.edges();
    let mut originals: Vec<Option<Curve>> = (0..edges.len()).map(|_| None).collect();
    let mut lanes: Vec<Option<Curve>> = (0..edges.len() * 2).map(|_| None).collect();
    for (at, edge) in edges.iter().enumerate() {
        if originals[at].is_some() {
            continue;
        }
        // Every edge between this pair, by ID, the way that they were declared.
        let (low, high) = (edge.from.min(edge.to), edge.from.max(edge.to));
        let mut pair: Vec<usize> = (0..edges.len())
            .filter(|&other| {
                edges[other].from.min(edges[other].to) == low && edges[other].from.max(edges[other].to) == high
            })
            .collect();
        pair.sort_by_key(|&other| edges[other].id);
        let (pa, pb) = (vertices[low], vertices[high]);
        let along = unit(pa, pb);
        let normal = (-along.1, along.0);
        let count = pair.len();
        let edge_offset = |k: usize| if count == 1 { 0.0 } else { (k as f64 - (count - 1) as f64 / 2.0) * EDGE_STEP };
        let widest = (0..count).map(|k| edge_offset(k).abs()).fold(0.0, f64::max);
        for (k, &e) in pair.iter().enumerate() {
            let (from, to) = (vertices[edges[e].from], vertices[edges[e].to]);
            originals[e] = Some(Curve::new(from, to, normal, edge_offset(k)));
            for (d, direction) in [Direction::Forward, Direction::Reverse].into_iter().enumerate() {
                let j = 2 * k + d;
                let magnitude = widest + LANE_FIRST + (j / 2) as f64 * LANE_STEP;
                let offset = if j % 2 == 0 { magnitude } else { -magnitude };
                let (a, b) = if direction == Direction::Forward { (from, to) } else { (to, from) };
                lanes[2 * e + d] = Some(Curve::new(a, b, normal, offset));
            }
        }
    }
    let originals: Vec<Curve> = originals.into_iter().flatten().collect();
    let lanes: Vec<Curve> = lanes.into_iter().flatten().collect();
    // The labels are placed one after another, the edges first and then the lanes, each at the first stop that is free.
    let order: Vec<(&Curve, f64)> = originals
        .iter()
        .map(|c| (c, EDGE_LABEL_WIDTH))
        .chain(lanes.iter().map(|c| (c, LANE_LABEL_WIDTH)))
        .collect();
    let labels = place_labels(&vertices, &order);
    let route = |curve: &Curve, label: Point| Route {
        d: curve.path(),
        mid: curve.at(0.5),
        label,
    };
    let badges = vertices
        .iter()
        .map(|&(x, y)| (x + NODE_RADIUS * 0.85, y - NODE_RADIUS * 0.85))
        .collect();
    Geometry {
        edges: originals.iter().zip(&labels).map(|(c, &l)| route(c, l)).collect(),
        lanes: lanes
            .iter()
            .zip(&labels[originals.len()..])
            .map(|(c, &l)| route(c, l))
            .collect(),
        vertices,
        badges,
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
