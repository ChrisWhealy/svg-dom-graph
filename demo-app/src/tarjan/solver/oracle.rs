//! A second way to find the components, which the tests compare Tarjan's partition with. It is Kosaraju's algorithm,
//! which shares no code and no idea with the one under test: two passes of depth-first search, the second on the
//! reverse graph.

use super::Graph;
use std::collections::BTreeSet;

/// The components of `graph`, each as a sorted set, with the sets in a sorted set so that order does not matter.
pub(super) fn partition(graph: &Graph) -> BTreeSet<Vec<usize>> {
    let n = graph.len();
    let mut seen = vec![false; n];
    let mut finished = Vec::with_capacity(n);
    for root in 0..n {
        if seen[root] {
            continue;
        }
        // An explicit stack, so that this is not the algorithm under test written again in recursion.
        let mut stack = vec![(root, 0usize)];
        seen[root] = true;
        while let Some(&mut (v, ref mut next)) = stack.last_mut() {
            if let Some(&w) = graph.targets(v).get(*next) {
                *next += 1;
                if !seen[w] {
                    seen[w] = true;
                    stack.push((w, 0));
                }
            } else {
                finished.push(v);
                stack.pop();
            }
        }
    }
    let mut reverse = vec![Vec::new(); n];
    for (a, b) in graph.edges() {
        reverse[b].push(a);
    }
    let mut assigned = vec![false; n];
    let mut out = BTreeSet::new();
    for &root in finished.iter().rev() {
        if assigned[root] {
            continue;
        }
        let mut component = vec![root];
        assigned[root] = true;
        let mut at = 0;
        while at < component.len() {
            let v = component[at];
            at += 1;
            for &w in &reverse[v] {
                if !assigned[w] {
                    assigned[w] = true;
                    component.push(w);
                }
            }
        }
        component.sort_unstable();
        out.insert(component);
    }
    out
}
