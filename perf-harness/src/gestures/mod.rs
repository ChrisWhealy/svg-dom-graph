//! Plans the real input that a browser is driven with in workload W4, and what each piece of it must do.
//!
//! The scene is built once, on a grid. For each scale of the view and each gesture there is a case, and a case is a
//! list of gestures drawn from its own stream of the generator, so a case does not depend on the cases before it. Every
//! gesture says where the pointer starts, how far it goes, and what the scene must look like when it has finished. The
//! last is what lets a runner reject a sample whose input did not do what it was meant to, since a latency for a
//! gesture that did nothing is not a latency.
//!
//! Positions are pixels of the host `<svg>`, measured from its top-left corner, with the view's translation at zero
//! when a gesture begins. The host is as many pixels wide as its view box is wide, so a pixel of the pointer is a unit
//! of the scene when the scale is 100%. What a gesture must do is worked out from the library's rules, which are stated
//! beside each one, and a browser run checks each against what the library did.

use crate::manifest::{GestureKind, Manifest, Workload};
use crate::ops::hex;
use crate::prng::SplitMix64;
use crate::workload::{GraphPlan, NODE_H, NODE_W, WorkloadError, graph_plan};
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Case `c` draws from stream `STREAM_CASE_BASE + c`.
const STREAM_CASE_BASE: u64 = 0x2000;
/// A background point or a drag target keeps this far from the edge of the host.
const EDGE: f64 = 4.0;
/// The most draws that finding a point on the background may take.
const BACKGROUND_ATTEMPTS: u32 = 500;
/// One wheel notch is this many pixels of `deltaY`, and zooms by [`ZOOM_STEP`].
pub const NOTCH: i64 = 100;
/// The factor of one zoom step, which the library states and its own tests fix.
pub const ZOOM_STEP: f64 = 1.25;

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A view as the library keeps it: a point of the scene is at its own coordinates times `scale`, plus the translation.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct View {
    pub scale: f64,
    pub tx: f64,
    pub ty: f64,
}

impl View {
    /// The view after zooming by `factor` about the point `(x, y)` of the host, which stays where it is on the screen.
    pub fn zoomed_about(self, factor: f64, x: f64, y: f64) -> Self {
        Self {
            scale: self.scale * factor,
            tx: x - (x - self.tx) * factor,
            ty: y - (y - self.ty) * factor,
        }
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// One gesture, and the scene it must leave.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "gesture", rename_all = "snake_case")]
pub enum Item {
    /// The pointer goes down on the centre of `node`, moves by `dx`, `dy` pixels in `steps` equal moves, and comes up.
    /// The node ends up `dx / scale`, `dy / scale` units from where its corner `start` was, which is how the library
    /// divides a pointer movement by the scale of the view.
    Drag {
        node: String,
        from: [i64; 2],
        dx: i64,
        dy: i64,
        steps: u32,
        start: [i64; 2],
        expected: [f64; 2],
    },
    /// The pointer goes down on the background, moves, and comes up. The view is translated by the movement and not
    /// scaled.
    Pan {
        from: [i64; 2],
        dx: i64,
        dy: i64,
        steps: u32,
        expected: View,
    },
    /// One wheel event with the control key held, at `at`, of `delta_y`. The scale is multiplied by [`ZOOM_STEP`] to
    /// the power of minus `delta_y` over [`NOTCH`], about the pointer.
    WheelZoom { at: [i64; 2], delta_y: i64, expected: View },
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Case {
    pub gesture: GestureKind,
    pub scale_percent: u32,
    pub warmup: Vec<Item>,
    pub measured: Vec<Item>,
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Plan {
    /// The width and height of the host in pixels.
    pub host: [i64; 2],
    pub cases: Vec<Case>,
}

impl Plan {
    /// The digest of the plan as JSON, which pins what a browser is driven with.
    pub fn sha256(&self) -> String {
        hex(&Sha256::digest(
            serde_json::to_vec(self).expect("a plan is always serialisable"),
        ))
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The screen box of a node at `scale`, as `[left, top, right, bottom]`, with the view's translation at zero.
fn screen_box(x: i64, y: i64, scale: f64) -> [f64; 4] {
    [
        x as f64 * scale,
        y as f64 * scale,
        (x + NODE_W) as f64 * scale,
        (y + NODE_H) as f64 * scale,
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// A random whole number from `low` to `high` inclusive, or `low` if the range is empty. One draw is made either way.
fn between(prng: &mut SplitMix64, low: i64, high: i64) -> i64 {
    let span = if high >= low { (high - low + 1) as u64 } else { 1 };
    low + prng.below(span) as i64
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn drag(
    prng: &mut SplitMix64,
    graph: &GraphPlan,
    host: [i64; 2],
    scale: f64,
    reach: i64,
    steps: u32,
) -> Result<Item, WorkloadError> {
    // Only a node that is wholly in view can be pressed on and moved with its whole box staying in view.
    let in_view: Vec<usize> = (0..graph.nodes.len())
        .filter(|&index| {
            let [left, top, right, bottom] = screen_box(graph.nodes[index].x, graph.nodes[index].y, scale);
            left >= EDGE && top >= EDGE && right <= host[0] as f64 - EDGE && bottom <= host[1] as f64 - EDGE
        })
        .collect();
    let pick = prng.below(in_view.len().max(1) as u64) as usize;
    let Some(&index) = in_view.get(pick) else {
        return Err(WorkloadError::Plan(format!("no node is wholly in view at a scale of {scale}")));
    };
    let node = &graph.nodes[index];
    let [left, top, right, bottom] = screen_box(node.x, node.y, scale);
    // The farthest the box can move and stay in view, which the library's confinement would otherwise change.
    let dx = between(
        prng,
        -reach.min((left - EDGE).floor() as i64),
        reach.min((host[0] as f64 - EDGE - right).floor() as i64),
    );
    let dy = between(
        prng,
        -reach.min((top - EDGE).floor() as i64),
        reach.min((host[1] as f64 - EDGE - bottom).floor() as i64),
    );
    Ok(Item::Drag {
        node: node.key.clone(),
        from: [((left + right) / 2.0).round() as i64, ((top + bottom) / 2.0).round() as i64],
        dx,
        dy,
        steps,
        start: [node.x, node.y],
        expected: [node.x as f64 + dx as f64 / scale, node.y as f64 + dy as f64 / scale],
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn pan(
    prng: &mut SplitMix64,
    graph: &GraphPlan,
    host: [i64; 2],
    scale: f64,
    reach: i64,
    steps: u32,
) -> Result<Item, WorkloadError> {
    let boxes: Vec<[f64; 4]> = graph.nodes.iter().map(|node| screen_box(node.x, node.y, scale)).collect();
    // A point on the background is one that no node covers, with a margin so that the press cannot fall on an edge of
    // one.
    let mut found = None;
    for _ in 0..BACKGROUND_ATTEMPTS {
        let x = between(prng, EDGE as i64 + 6, host[0] - EDGE as i64 - 6);
        let y = between(prng, EDGE as i64 + 6, host[1] - EDGE as i64 - 6);
        let covered = boxes.iter().any(|[l, t, r, b]| {
            (x as f64) >= l - 6.0 && (x as f64) <= r + 6.0 && (y as f64) >= t - 6.0 && (y as f64) <= b + 6.0
        });
        if found.is_none() && !covered {
            found = Some([x, y]);
        }
    }
    let from = found.ok_or_else(|| WorkloadError::Plan("no point of the background could be found".to_string()))?;
    let dx = between(prng, -reach, reach);
    let dy = between(prng, -reach, reach);
    Ok(Item::Pan {
        from,
        dx,
        dy,
        steps,
        expected: View {
            scale,
            tx: dx as f64,
            ty: dy as f64,
        },
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn wheel(prng: &mut SplitMix64, host: [i64; 2], scale: f64) -> Item {
    let x = between(prng, EDGE as i64, host[0] - EDGE as i64);
    let y = between(prng, EDGE as i64, host[1] - EDGE as i64);
    // Scrolling up zooms in, and a notch is one step, so the sign is the direction.
    let delta_y = if prng.below(2) == 0 { -NOTCH } else { NOTCH };
    // One notch either way, so the factor is the step or its reciprocal, each of which is a single correctly rounded
    // operation and so is the same on every platform. A power would not be promised that.
    let factor = if delta_y < 0 { ZOOM_STEP } else { 1.0 / ZOOM_STEP };
    Item::WheelZoom {
        at: [x, y],
        delta_y,
        expected: View { scale, tx: 0.0, ty: 0.0 }.zoomed_about(factor, x as f64, y as f64),
    }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Plans every case of an interaction manifest.
///
/// # Errors
///
/// Returns [`WorkloadError::Plan`] if the manifest is not an interaction manifest, or a gesture cannot be placed at a
/// scale, such as a drag when no node is wholly in view.
pub fn plan(manifest: &Manifest) -> Result<Plan, WorkloadError> {
    let interaction = match (&manifest.interaction, manifest.workload) {
        (Some(interaction), Workload::Interaction) => interaction,
        _ => return Err(WorkloadError::Plan("only an interaction manifest has gestures".to_string())),
    };
    let graph = graph_plan(manifest)?;
    let host = [graph.field_w, graph.field_h];
    let reach = i64::from(interaction.steps) * i64::from(interaction.step_pixels);
    let (warmup, measured) = (manifest.execution.warmup_iterations, manifest.execution.measurement_iterations);

    let mut cases = Vec::new();
    for &scale_percent in &interaction.scales_percent {
        for &gesture in &interaction.gestures {
            let mut prng = SplitMix64::derive(manifest.seed, STREAM_CASE_BASE + cases.len() as u64);
            let scale = f64::from(scale_percent) / 100.0;
            let item = |prng: &mut SplitMix64| match gesture {
                GestureKind::Drag => drag(prng, &graph, host, scale, reach, interaction.steps),
                GestureKind::Pan => pan(prng, &graph, host, scale, reach, interaction.steps),
                GestureKind::WheelZoom => Ok(wheel(prng, host, scale)),
            };
            let warmup = (0..warmup).map(|_| item(&mut prng)).collect::<Result<Vec<_>, _>>()?;
            let measured = (0..measured).map(|_| item(&mut prng)).collect::<Result<Vec<_>, _>>()?;
            cases.push(Case {
                gesture,
                scale_percent,
                warmup,
                measured,
            });
        }
    }
    Ok(Plan { host, cases })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
#[cfg(test)]
mod unit_tests;
