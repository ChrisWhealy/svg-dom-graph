//! Where to put a deliberate error into the executed stream, to prove that a run which ends in the right place says so
//! when it does not.
//!
//! A performance run executes only the last two checkpoints. They must between them vouch for everything that was
//! timed, so a mutation that is applied wrongly just before either of them has to make the run invalid. A runner is
//! given the ordinal of that mutation, which is its position among all the operations of the stream, and applies it
//! wrongly there. It is the operation that is wrong and not what is observed, so the whole chain is tested: the
//! operation, the state it leaves, the model's prediction and the comparison.

use crate::manifest::Manifest;
use crate::ops::Op;
use crate::workload::{WorkloadError, generate};

/// How far a node is put out, in the same units as the stream's coordinates. It is large enough to be unmistakable, and
/// small enough for the node to stay where a reader would expect it.
pub const DISPLACEMENT: i64 = 7;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Which of the two executed checkpoints of a performance run the corrupted mutation comes before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// The last checkpoint. For the cycle workloads it follows the final removal, and for rerouting the final move.
    Final,
    /// The one before it. For the cycle workloads it follows the final build, and for rerouting an earlier move.
    Penultimate,
}

impl Target {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "final" => Some(Self::Final),
            "penultimate" => Some(Self::Penultimate),
            _ => None,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The mutation to apply wrongly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Corruption {
    /// The position of the operation among every operation of the stream, markers included.
    pub ordinal: u64,
    /// The operation, for a message.
    pub operation: Op,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Finds the last mutation before the checkpoint that `target` names, with every checkpoint of the stream counted.
///
/// # Errors
///
/// Returns [`WorkloadError`] if the stream cannot be generated, or if it has no such checkpoint or no mutation before
/// it.
pub fn locate(manifest: &Manifest, target: Target) -> Result<Corruption, WorkloadError> {
    let mut ops: Vec<Op> = Vec::new();
    generate(manifest, &mut |op: &Op| {
        ops.push(op.clone());
        Ok(())
    })?;
    let checkpoints: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter(|(_, op)| matches!(op, Op::Checkpoint { .. }))
        .map(|(position, _)| position)
        .collect();
    let wanted = match target {
        Target::Final => checkpoints.len().checked_sub(1),
        Target::Penultimate => checkpoints.len().checked_sub(2),
    };
    let position = wanted
        .map(|index| checkpoints[index])
        .ok_or_else(|| WorkloadError::Plan(format!("the stream has no {target:?} checkpoint")))?;
    let ordinal = ops[..position]
        .iter()
        .rposition(Op::is_mutation)
        .ok_or_else(|| WorkloadError::Plan(format!("no mutation comes before the {target:?} checkpoint")))?;
    Ok(Corruption {
        ordinal: ordinal as u64,
        operation: ops[ordinal].clone(),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
