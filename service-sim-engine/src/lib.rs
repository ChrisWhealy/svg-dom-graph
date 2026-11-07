//! The pure engine of the discrete-event service simulator.
//!
//! It owns an integer virtual clock and a deterministic future-event heap, and nothing else: no DOM, no browser, no wall clock and
//! no `web-sys`, so it builds natively and for `wasm32-wasip1`. A seed gives the same events, slices, states and hashes on every
//! platform. The rules are those of design revision 6 section 3, and the ones it leaves open are in `dess/s1/docs/contract.md`.
//!
//! A second engine, in JavaScript, is in `dess/reference-js/`. It was written from the same text and shares no code with this one,
//! and `dess/s1/differential.mjs` compares the two on every pop, slice and hash.

pub mod engine;
pub mod generator;
pub mod hash;
pub mod model;
pub mod params;
pub mod prng;
pub mod trace;

pub use engine::{Counters, Engine, Final, Kind, Outcome, PopRecord, ScriptEvent, SliceRecord};
pub use model::{JobState, JobView, Model, ServerView};
pub use params::{Config, Error, Limits, Params};
