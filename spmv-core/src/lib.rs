//! The pure core of the sparse-matrix scale demonstrator.
//!
//! It generates a seeded sparse matrix and a vector, computes the exact integer product and keeps it up to date as entries of the
//! vector change, checks a specification against a budget before anything is built, and hashes what it computes. It has no DOM and
//! no `web-sys`, so it builds natively and for `wasm32-wasip1`. The rules are those of `spmv/docs/contract.md`, which a second
//! implementation in JavaScript, in `spmv/reference-js/`, was written from.

pub mod budget;
pub mod explain;
pub mod hash;
pub mod matrix;
pub mod prng;
pub mod run;
pub mod spec;
pub mod stats;
pub mod trace;

pub use budget::{Budget, Plan, Refusal};
pub use matrix::{Matrix, Stats, label};
pub use run::{Run, UpdateOrder};
pub use spec::{Error, Spec};
pub use trace::Prepared;
