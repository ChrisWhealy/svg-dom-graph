//! The two graphs that the page offers, with the places of their vertices. Pure: nothing here touches the DOM.
//!
//! Both have the same six vertices, `s`, `v1`, `v2`, `v3`, `v4` and `t`, counted from 0. The first is the classic
//! network whose maximum flow is 23. The second is a small network that makes the search take a reverse transition, so
//! that it takes flow back from an edge that it had put flow on.

use super::solver::{EdgeSpec, Graph};

/// The size of the picture of a graph, in the units of the scene.
pub(crate) const VIEW_WIDTH: f64 = 740.0;
pub(crate) const VIEW_HEIGHT: f64 = 400.0;

/// The names of the vertices, which are the same for both graphs.
pub(crate) const NAMES: [&str; 6] = ["s", "v1", "v2", "v3", "v4", "t"];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One graph that the page offers.
pub(crate) struct Fixture {
    /// What the page calls it in the root's `data-ek-fixture`, and in the ID of everything in it.
    pub(crate) key: &'static str,
    /// What the selector says.
    pub(crate) label: &'static str,
    /// The centre of each vertex.
    pub(crate) coords: [(f64, f64); 6],
    /// The edges, as `(from, to, capacity)`, in the order of their IDs.
    pub(crate) edges: &'static [(u32, u32, u32)],
}

impl Fixture {
    /// The graph, whose vertices are 0 to 5, whose source is `s` and whose sink is `t`.
    pub(crate) fn graph(&self) -> Graph {
        Graph {
            vertices: (0..6).collect(),
            source: 0,
            sink: 5,
            edges: self
                .edges
                .iter()
                .enumerate()
                .map(|(id, &(from, to, capacity))| EdgeSpec {
                    id: id as u32,
                    from,
                    to,
                    capacity,
                })
                .collect(),
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The classic network. `e0` to `e9` are `s→v1`, `s→v2`, `v1→v2`, `v2→v1`, `v1→v3`, `v3→v2`, `v2→v4`, `v4→v3`, `v3→t`
/// and `v4→t`.
pub(crate) const CANONICAL: Fixture = Fixture {
    key: "canonical",
    label: "Canonical (max flow 23)",
    coords: [
        (80.0, 200.0),
        (260.0, 90.0),
        (260.0, 310.0),
        (470.0, 90.0),
        (470.0, 310.0),
        (660.0, 200.0),
    ],
    edges: &[
        (0, 1, 16),
        (0, 2, 13),
        (1, 2, 10),
        (2, 1, 4),
        (1, 3, 12),
        (3, 2, 9),
        (2, 4, 14),
        (4, 3, 7),
        (3, 5, 20),
        (4, 5, 4),
    ],
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The network that needs a reverse transition. Its second path goes back along `e0`, which is `v1→v3`.
pub(crate) const REVERSE: Fixture = Fixture {
    key: "reverse",
    label: "Reverse cancellation (max flow 3)",
    coords: [
        (80.0, 200.0),
        (230.0, 80.0),
        (490.0, 80.0),
        (490.0, 320.0),
        (230.0, 320.0),
        (650.0, 200.0),
    ],
    edges: &[(1, 3, 2), (3, 5, 2), (4, 3, 1), (1, 2, 2), (0, 1, 2), (0, 4, 1), (2, 5, 2)],
};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Both graphs, in the order that the selector lists them.
pub(crate) const FIXTURES: [&Fixture; 2] = [&CANONICAL, &REVERSE];

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The graph whose key is `key`.
pub(crate) fn by_key(key: &str) -> Option<&'static Fixture> {
    FIXTURES.iter().copied().find(|fixture| fixture.key == key)
}
