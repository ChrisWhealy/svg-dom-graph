//! The graphs that the tests check the solver against, and the answer that does not use the solver. A test build only.
//!
//! The answer is the minimum cut, found by trying every way of splitting the vertices. The solver and the cut agree if
//! the solver is right, and nothing in the cut shares code with the solver.

use super::{EdgeSpec, Graph};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The generator of the numbers: SplitMix64, with one state for as many graphs as are asked for.
pub(super) struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub(super) fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub(super) fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number below `k`, which takes one draw.
    fn below(&mut self, k: u64) -> u64 {
        self.next() % k
    }
}

/// A graph as a list of `(from, to, capacity)`, and how many vertices it has. The source is 0 and the sink is the last.
pub(super) type Raw = (usize, Vec<(usize, usize, u32)>);

/// The graph that `raw` says, with its vertices 0 to n−1.
pub(super) fn graph_of(raw: &Raw) -> Graph {
    let (n, edges) = raw;
    Graph {
        vertices: (0..*n as u32).collect(),
        source: 0,
        sink: *n as u32 - 1,
        edges: edges
            .iter()
            .enumerate()
            .map(|(id, &(from, to, capacity))| EdgeSpec {
                id: id as u32,
                from: from as u32,
                to: to as u32,
                capacity,
            })
            .collect(),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The smallest capacity of a cut: the least, over every set `S` with the source in it and the sink out of it, of the
/// capacity of the edges that go from `S` to the rest. It tries every set of the vertices between the two.
pub(super) fn minimum_cut(raw: &Raw) -> u64 {
    let (n, edges) = raw;
    let between = n - 2;
    (0..1u32 << between)
        .map(|mask| {
            let in_s = |v: usize| v == 0 || (v != n - 1 && mask >> (v - 1) & 1 == 1);
            edges
                .iter()
                .filter(|&&(from, to, _)| in_s(from) && !in_s(to))
                .map(|&(_, _, capacity)| u64::from(capacity))
                .sum::<u64>()
        })
        .min()
        .unwrap_or(0)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The first suite: graphs of 2 to 8 vertices and up to 16 edges, with capacities 0 to 7, which may have parallel
/// edges, antiparallel edges and edges of no capacity, and never a loop. They are drawn from one stream, in this order:
/// how many vertices, how many edges, then for each edge its two ends (drawn again if they are the same vertex) and its
/// capacity.
///
/// Almost none of them need the search to take a reverse transition, so this suite shows that the value is right where
/// no flow has to be taken back. The second suite is for that.
pub(super) fn broad_graphs() -> Vec<Raw> {
    let mut rng = SplitMix64::new(0xEDCA_2026_1103_0001);
    (0..500)
        .map(|_| {
            let n = 2 + rng.below(7) as usize;
            let m = rng.below(17) as usize;
            let mut edges = Vec::new();
            while edges.len() < m {
                let (from, to) = (rng.below(n as u64) as usize, rng.below(n as u64) as usize);
                if from == to {
                    continue;
                }
                edges.push((from, to, rng.below(8) as u32));
            }
            (n, edges)
        })
        .collect()
}

/// The second suite: 500 layered graphs, built so that some of them need a reverse transition. There is a source, a
/// layer `L` of two or three vertices, a layer `R` of two or three, and the sink. This is the generator of the
/// specification's Appendix A, and it is checked by the first graph and the totals that the tests assert.
///
/// A loop that is drawn is thrown away whole: the next attempt draws a new kind, then new ends, and no capacity is
/// drawn for the one that was thrown away. The parts of a tuple are drawn from left to right.
pub(super) fn layered_graphs() -> Vec<Raw> {
    let mut rng = SplitMix64::new(0xEDCA_2026_1103_0002);
    let mut graphs = Vec::with_capacity(500);
    for index in 0..500 {
        let a = 2 + rng.below(2) as usize;
        let b = 2 + rng.below(2) as usize;
        let n = 2 + a + b;
        let left: Vec<usize> = (1..=a).collect();
        let right: Vec<usize> = (a + 1..=a + b).collect();
        let sink = n - 1;
        let cap = |rng: &mut SplitMix64| 1 + rng.below(3) as u32;
        let mut edges: Vec<(usize, usize, u32)> = Vec::new();
        for &x in &left {
            let c = cap(&mut rng);
            edges.push((0, x, c));
        }
        for &y in &right {
            let c = cap(&mut rng);
            edges.push((y, sink, c));
        }
        for &x in &left {
            let y = right[rng.below(b as u64) as usize];
            let c = cap(&mut rng);
            edges.push((x, y, c));
        }
        if index % 4 == 0 {
            let c = cap(&mut rng);
            edges.push((left[0], left[1], c));
            let c = cap(&mut rng);
            edges.push((left[1], left[0], c));
        }
        let target = (8 + rng.below(9) as usize).max(edges.len()).min(16);
        while edges.len() < target {
            let kind = rng.below(10);
            let (u, v) = match kind {
                0 | 1 => (0, left[rng.below(a as u64) as usize]),
                2..=5 => {
                    let u = left[rng.below(a as u64) as usize];
                    (u, right[rng.below(b as u64) as usize])
                },
                6 | 7 => (right[rng.below(b as u64) as usize], sink),
                8 => {
                    let u = left[rng.below(a as u64) as usize];
                    (u, left[rng.below(a as u64) as usize])
                },
                _ => {
                    let u = right[rng.below(b as u64) as usize];
                    (u, right[rng.below(b as u64) as usize])
                },
            };
            if u == v {
                continue;
            }
            let c = cap(&mut rng);
            edges.push((u, v, c));
        }
        for i in (1..edges.len()).rev() {
            let j = rng.below(i as u64 + 1) as usize;
            edges.swap(i, j);
        }
        graphs.push((n, edges));
    }
    graphs
}
