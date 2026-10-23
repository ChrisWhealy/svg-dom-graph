//! The graph the weighted springs demo moves: the Dijkstra demo's six circles and nine edges, taken from that demo's own
//! definitions. Nothing here repeats a position, an edge or a rectangle. If the Dijkstra layout changes, this changes
//! with it, and the tests below say whether the physics still copes.

use crate::dijkstra::{EDGES, NAMES, VERTEX_RADIUS, graph_region, vertex_origins};
use demo_common::force::solver::{Bounds, Params, SolverError, SpringSystem, Vec2};
use svg_dom::root::utils::Rect;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Everything needed to build the physics for the Dijkstra graph.
#[derive(Clone, Debug)]
pub(super) struct Fixture {
    /// The centre of each vertex, in the order of [`NAMES`].
    pub(super) centres: Vec<Vec2>,
    /// The radius of each vertex.
    pub(super) radii: Vec<f64>,
    /// Each edge as `(from, to, weight)`, in the order the Dijkstra demo draws them.
    pub(super) edges: Vec<(usize, usize, f64)>,
    /// The rectangle that both a dragged circle and a free one must stay inside.
    pub(super) region: Rect,
}

impl Fixture {
    /// The fixture of the Dijkstra demo as it is now.
    pub(super) fn dijkstra() -> Self {
        Self::from_parts(vertex_origins().iter().map(|o| Vec2::new(o.x, o.y)).collect(), graph_region())
    }

    /// A fixture with the given top-left corners, for the Dijkstra edges and radius, inside `region`.
    pub(super) fn from_parts(origins: Vec<Vec2>, region: Rect) -> Self {
        Self {
            centres: origins
                .iter()
                .map(|origin| Vec2::new(origin.x + VERTEX_RADIUS, origin.y + VERTEX_RADIUS))
                .collect(),
            radii: vec![VERTEX_RADIUS; NAMES.len()],
            edges: EDGES.iter().map(|&(from, to, weight)| (from, to, weight as f64)).collect(),
            region,
        }
    }

    /// The region as the solver's bounds.
    pub(super) fn bounds(&self) -> Bounds {
        Bounds::from_origin_and_size(
            self.region.origin.x,
            self.region.origin.y,
            self.region.size.width,
            self.region.size.height,
        )
    }

    /// The physics for this fixture, with its default parameters.
    ///
    /// # Errors
    ///
    /// Returns a [`SolverError`] if the fixture is not one the solver accepts.
    pub(super) fn system(&self) -> Result<SpringSystem, SolverError> {
        SpringSystem::new(&self.centres, &self.radii, &self.edges, self.bounds(), Params::default())
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
