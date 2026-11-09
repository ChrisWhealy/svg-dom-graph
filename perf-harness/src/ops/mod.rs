//! The operations of a workload.
//!
//! An operation is something a runner does to a scene, or a marker that tells it where it is. Coordinates are whole
//! scene units, which keeps every encoded value an integer and makes the reference model's arithmetic exact.

use crate::encode::{ENCODING_VERSION, Encoder};
use serde::Serialize;
use sha2::{Digest, Sha256};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which part of a run the following operations belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Warmup,
    Measure,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Op {
    /// Starts a scene on a fresh host `<svg>`.
    CreateScene,
    /// Adds a plain node. `x` and `y` are its top-left corner.
    AddNode {
        key: String,
        x: i64,
        y: i64,
        w: i64,
        h: i64,
        label: String,
    },
    /// Adds a straight, directed connector between two nodes, named by their keys.
    AddEdge { from: String, to: String },
    /// Moves a node so that its top-left corner is at `x`, `y`.
    MoveNode { key: String, x: i64, y: i64 },
    /// Drops the last handle on the scene. Its DOM stays where it is.
    DropScene,
    /// Removes the host `<svg>` from the document.
    RemoveHost,
    /// Marks the start of the warm-up or the measured part. Control only: it changes no scene.
    Phase { phase: Phase },
    /// Marks a point at which the scene is compared with the model. Control only: it changes no scene.
    Checkpoint { index: u64 },
}

impl Op {
    /// Whether the operation changes the scene, as opposed to marking a place in the stream.
    ///
    /// A performance run skips checkpoints and so proves it ran the same mutations, not the same markers.
    pub fn is_mutation(&self) -> bool {
        !matches!(self, Self::Phase { .. } | Self::Checkpoint { .. })
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// Writes the canonical encoding of the operation into `encoder`, which is cleared first.
    pub fn encode(&self, encoder: &mut Encoder) {
        encoder.clear();
        match self {
            Self::CreateScene => encoder.code(1),
            Self::DropScene => encoder.code(2),
            Self::RemoveHost => encoder.code(3),
            Self::AddNode { key, x, y, w, h, label } => {
                encoder.code(4);
                encoder.put_str(key);
                for value in [x, y, w, h] {
                    encoder.put_i64(*value);
                }
                encoder.put_str(label);
            },
            Self::AddEdge { from, to } => {
                encoder.code(5);
                encoder.put_str(from);
                encoder.put_str(to);
            },
            Self::MoveNode { key, x, y } => {
                encoder.code(6);
                encoder.put_str(key);
                encoder.put_i64(*x);
                encoder.put_i64(*y);
            },
            Self::Phase { phase } => {
                encoder.code(7);
                encoder.put_u64(match phase {
                    Phase::Warmup => 0,
                    Phase::Measure => 1,
                });
            },
            Self::Checkpoint { index } => {
                encoder.code(8);
                encoder.put_u64(*index);
            },
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Hashes a stream as it is generated, once over every operation and once over the mutations alone.
#[derive(Debug)]
pub struct StreamHasher {
    full: Sha256,
    mutations: Sha256,
    encoder: Encoder,
    operations: u64,
    mutation_count: u64,
}

impl Default for StreamHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamHasher {
    pub fn new() -> Self {
        let mut full = Sha256::new();
        let mut mutations = Sha256::new();
        full.update([ENCODING_VERSION]);
        mutations.update([ENCODING_VERSION]);
        Self {
            full,
            mutations,
            encoder: Encoder::new(),
            operations: 0,
            mutation_count: 0,
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn push(&mut self, op: &Op) {
        op.encode(&mut self.encoder);
        self.full.update(self.encoder.as_slice());
        self.operations += 1;
        if op.is_mutation() {
            self.mutations.update(self.encoder.as_slice());
            self.mutation_count += 1;
        }
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn operations(&self) -> u64 {
        self.operations
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    pub fn mutation_count(&self) -> u64 {
        self.mutation_count
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The two digests, as lower-case hex: every operation, then the mutations alone.
    pub fn finish(self) -> (String, String) {
        (hex(&self.full.finalize()), hex(&self.mutations.finalize()))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Lower-case hex of `bytes`.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
