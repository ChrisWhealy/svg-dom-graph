//! `svg-dom-graph` — dynamically re-routable connectors between SVG boxes, built with [`svg_dom`].
//!
//! The eventual goal is arbitrary graphs of boxes.
//! A graph may be cyclic or acyclic, directed or undirected.
//! As boxes move, their connectors stay correctly routed.
//!
//! [`model::Graph`] holds the topology: nodes, edges, and incidence, with no DOM dependency of its own.
//! [`scene`] renders a `Graph` onto the DOM, and keeps each node's and edge's rendered SVG handles in sync as nodes
//! move.
//!
//! This crate has no opinion about which HTML page hosts it, what graph a caller builds, or when.
//! The sibling `demo-app` crate supplies a small worked example: the `wasm_bindgen(start)` entry point, a specific
//! `<svg>` element id to attach to, and a specific demo graph.
//!
//! See [`geometry::boundary_point`] for the routing math and [`scene::Scene`] for the public rendering API.

pub mod geometry;
pub mod model;
pub mod scene;
