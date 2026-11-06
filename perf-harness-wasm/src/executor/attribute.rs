//! The attribution run: what is still live after each teardown, by the phase of the harness that allocated it.

use super::Run;
use super::phase;
use crate::alloc;
use perf_harness::manifest::{Manifest, Mode};
use std::collections::BTreeMap;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `manifest` as an endurance run with the allocator tracking every block, and says what is still live after the teardown
/// of each of the first `cycles` cycles, by the phase of the harness that allocated it.
///
/// The blocks counted are those allocated after the baseline was read, so they add up to the bytes that the run reports as
/// left over the baseline. Nothing here is a measurement of time, and the run is not valid as one.
pub fn attribute(manifest: &Manifest, cycles: u64) -> String {
    let mut run = match Run::new(manifest, Mode::Endurance) {
        Ok(run) => run,
        Err(message) => return serde_json::json!({"error": message}).to_string(),
    };
    run.track_cycles = cycles;
    // What was allocated between the baseline and the moment tracking begins is not followed, so it is measured, to be accounted for.
    let before_tracking = alloc::stats().live_bytes as i64 - run.baseline.live_bytes.unwrap_or(0.0) as i64;
    alloc::start_tracking();
    let result = run.execute();
    alloc::stop_tracking();
    let untracked = alloc::untracked();
    if let Err(message) = result {
        return serde_json::json!({"error": message}).to_string();
    }
    let log: Vec<serde_json::Value> = run
        .tracking_log
        .iter()
        .map(|(cycle, over_baseline, blocks_over, blocks)| {
            let mut by_phase: BTreeMap<usize, (usize, usize, Vec<usize>)> = BTreeMap::new();
            for block in blocks {
                let entry = by_phase.entry(block.tag).or_default();
                entry.0 += 1;
                entry.1 += block.size;
                entry.2.push(block.size);
            }
            serde_json::json!({
                "cycle": cycle,
                "live_bytes_over_baseline": over_baseline,
                "live_blocks_over_baseline": blocks_over,
                "tracked_blocks": blocks.len(),
                "tracked_bytes": blocks.iter().map(|b| b.size).sum::<usize>(),
                "by_phase": by_phase.into_iter().map(|(tag, (count, bytes, mut sizes))| {
                    sizes.sort_unstable();
                    serde_json::json!({"phase": phase::name(tag), "blocks": count, "bytes": bytes, "sizes": sizes})
                }).collect::<Vec<_>>(),
            })
        })
        .collect();
    serde_json::json!({
        "cycles": log,
        "blocks_the_tracker_could_not_follow": untracked,
        "bytes_allocated_before_tracking_began": before_tracking,
        "growths_of_blocks_from_before_tracking": alloc::growths().into_iter().map(|(old, new, tag)| serde_json::json!({"from": old, "to": new, "during": phase::name(tag)})).collect::<Vec<_>>(),
    })
    .to_string()
}
