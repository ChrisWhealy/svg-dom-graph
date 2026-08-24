pub mod edge;
pub mod graph;
pub mod node;

use std::sync::atomic::AtomicUsize;

/// Assigns each `Graph` a distinct number, stamped into every id it hands out.
///
/// Keep `NodeId`/`EdgeId` `Graph` specific.  This avoids ids originating in different `Graph`s from being mistaken for
/// one from another. Different `Graph`s can number their own nodes from zero without the risk of accidentally confusing
/// id `1` from `Graph` `a` with id `1` from `Graph` `b`.
static NEXT_GRAPH_ID: AtomicUsize = AtomicUsize::new(0);

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
