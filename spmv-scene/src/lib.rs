//! The scene of the sparse-matrix scale demonstrator.
//!
//! A sparse matrix is drawn as a bipartite dependency graph, with a node for each column and each row and an edge for each nonzero,
//! and two data nodes that show windows of the vector x and the product y. The [`layout`] is pure and is tested natively. The
//! [`scene`] builds it on a `svg-dom-graph` `Scene`, writes the data nodes as the vector changes, and measures what that costs.
//! Nothing here decides what is allowed: the budget of `spmv-core` is checked before the scene is touched.

pub mod layout;
pub mod probe;
pub mod scene;
pub mod worked;
