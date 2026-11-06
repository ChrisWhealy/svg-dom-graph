//! Comparing what is in the document at a checkpoint with what the reference model predicted.

use super::Run;
use super::phase;
use crate::alloc;
use crate::observe::observe;
use perf_harness::compare::compare;
use perf_harness::model::{Checkpoint, Model};
use perf_harness::outcome::FailedCheckpoint;

/// A checkpoint that goes wrong reports this many differences, since the rest repeat the same fault.
const MAX_REPORTED_PER_CHECKPOINT: usize = 20;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
impl Run<'_> {
    /// Compares what is in the document with what the model predicted.
    pub(super) fn check(&mut self, index: u64) -> Result<(), String> {
        alloc::set_tag(phase::CHECK);
        self.checkpoint_this_cycle = true;
        let expected = self.model.checkpoint(self.view)?;
        let mut actual = match (&self.svg, &self.scene) {
            (Some(svg), Some(scene)) => observe(svg, scene, &self.edge_keys, self.view)?,
            _ => self.observe_nothing()?,
        };
        if let Some(fault) = self.fault {
            fault.apply(&mut actual);
        }
        let differences = compare(&expected, &actual);
        if differences.is_empty() {
            self.outcome.checkpoints_passed += 1;
        } else {
            let total = differences.len();
            for difference in differences.iter().take(MAX_REPORTED_PER_CHECKPOINT) {
                self.outcome.checkpoints_failed.push(FailedCheckpoint {
                    section: difference.section.to_string(),
                    detail: format!("checkpoint {index}: {}", difference.summary()),
                });
            }
            if total > MAX_REPORTED_PER_CHECKPOINT {
                self.outcome.notes.push(format!("checkpoint {index} differed in {total} places, and the first {MAX_REPORTED_PER_CHECKPOINT} are reported"));
            }
        }
        Ok(())
    }

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    /// The checkpoint of a page with no scene on it, which is an empty one unless a host was left behind.
    fn observe_nothing(&self) -> Result<Checkpoint, String> {
        let leftover = self
            .document
            .query_selector("svg[id^='perf-svg-']")
            .map_err(|e| format!("{e:?}"))?;
        let mut empty = Model::new().checkpoint(self.view)?;
        if let Some(element) = leftover {
            empty
                .topology
                .nodes
                .push(format!("(a host svg, {}, is still in the document)", element.id()));
        }
        Ok(empty)
    }
}
