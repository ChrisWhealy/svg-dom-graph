//! The workload report: everything a runner needs to know about what a manifest should do, and what the scene should
//! look like at each checkpoint, without holding the stream itself.
//!
//! Building it generates the whole stream once, hashes it, and drives the reference model through it. A runner then
//! regenerates the same stream, checks its own hash against the report's, and compares its checkpoints with the
//! model's.

use crate::encode::ENCODING_VERSION;
use crate::manifest::{Manifest, ViewPolicy};
use crate::model::Model;
use crate::ops::{Op, StreamHasher};
use crate::workload::{WorkloadError, generate};
use serde::Serialize;
use std::collections::BTreeMap;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Serialize)]
pub struct ReportManifest {
    pub id: String,
    pub sha256: String,
    pub canonical: Manifest,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Serialize)]
pub struct StreamInfo {
    pub encoding_version: u8,
    pub operations: u64,
    pub mutations: u64,
    /// Over every operation, markers included. A correctness run executes all of them.
    pub sha256: String,
    /// Over the operations that change the scene. A performance run skips the checkpoints and proves it ran these.
    pub mutations_sha256: String,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Serialize)]
pub struct CheckpointDigest {
    pub index: u64,
    /// The position of the checkpoint operation in the stream, counting from zero.
    pub op_index: u64,
    pub model_sha256: String,
    pub section_sha256: BTreeMap<String, String>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Serialize)]
pub struct Sections {
    /// Predicted from the operations alone, so a difference is a real difference.
    pub modelled: Vec<&'static str>,
    /// Predicted by a rule that has still to be confirmed against the rendered DOM. None are, for the operations there
    /// are.
    pub provisional: Vec<&'static str>,
    /// Left empty by the model, and not to be compared.
    pub not_modelled: Vec<&'static str>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub schema_version: u32,
    pub manifest: ReportManifest,
    pub stream: StreamInfo,
    pub checkpoints: Vec<CheckpointDigest>,
    pub sections: Sections,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds the report of `manifest`, naming it `id`.
///
/// # Errors
///
/// Returns [`WorkloadError`] if a graph cannot be planned, or if the model refuses an operation, which means that the
/// generator produced a stream that makes no sense.
pub fn build_report(manifest: &Manifest, id: &str) -> Result<Report, WorkloadError> {
    let view = manifest.checkpoint.as_ref().map_or(ViewPolicy::Unchanged, |policy| policy.view);
    let mut model = Model::new();
    let mut hasher = StreamHasher::new();
    let mut checkpoints = Vec::new();

    generate(manifest, &mut |op: &Op| {
        model.apply(op)?;
        hasher.push(op);
        if let Op::Checkpoint { index } = op {
            let checkpoint = model.checkpoint(view)?;
            checkpoints.push(CheckpointDigest {
                index: *index,
                op_index: hasher.operations() - 1,
                model_sha256: checkpoint.model_sha256(),
                section_sha256: checkpoint
                    .section_hashes()
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect(),
            });
        }
        Ok(())
    })?;

    let (operations, mutations) = (hasher.operations(), hasher.mutation_count());
    let (sha256, mutations_sha256) = hasher.finish();
    Ok(Report {
        schema_version: 1,
        manifest: ReportManifest {
            id: id.to_string(),
            sha256: manifest.canonical_sha256(),
            canonical: manifest.clone(),
        },
        stream: StreamInfo {
            encoding_version: ENCODING_VERSION,
            operations,
            mutations,
            sha256,
            mutations_sha256,
        },
        checkpoints,
        sections: Sections {
            modelled: vec!["topology", "content", "interaction", "geometry", "paint_order"],
            provisional: vec![],
            not_modelled: vec!["dom_identity.ids", "accessibility"],
        },
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
