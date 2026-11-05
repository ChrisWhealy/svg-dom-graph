//! Deterministic workloads and a reference model for measuring svg-dom-graph.
//!
//! A [`Manifest`](manifest::Manifest) names a workload, a seed and how many times to run it. [`workload::generate`] turns it
//! into a stream of [`ops::Op`]s. The [`model::Model`] says what a scene should look like after them. [`report::build_report`]
//! ties the three together, so that a runner in a browser can prove that it ran the intended stream and compare what it drew.
//!
//! Nothing here touches a DOM or a clock. The crate builds for wasm as well as natively, so the browser runner can generate
//! the stream itself from the same code. Switch off the `schema` feature to leave out the JSON-schema check on a manifest.

pub mod compare;
pub mod encode;
pub mod manifest;
pub mod metrics;
pub mod model;
pub mod ops;
pub mod outcome;
pub mod prng;
pub mod report;
pub mod stats;
pub mod workload;

#[cfg(feature = "layer-a")]
pub mod layer_a;
