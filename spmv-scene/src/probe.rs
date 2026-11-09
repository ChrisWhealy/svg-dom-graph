//! The probes: each builds or drives a scene and reports what it cost, so that the documented resource risks are measured and not
//! assumed. A probe reports; it does not decide what is acceptable. The numbers are for one browser on one machine.

use crate::scene::{BuildOptions, Built, Host, element_count, heap_used, now};
use serde_json::{Value, json};
use spmv_core::stats::Latency;
use spmv_core::{Prepared, Run};

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// What the measurements ran on: the browser, the cores and the memory it reports. Every record carries it, since a figure means
/// nothing without the machine.
#[must_use]
pub fn environment() -> Value {
    let Some(window) = web_sys::window() else { return Value::Null };
    let navigator = window.navigator();
    let memory = js_sys::Reflect::get(&navigator, &"deviceMemory".into())
        .ok()
        .and_then(|v| v.as_f64());
    json!({
        "user_agent": navigator.user_agent().unwrap_or_default(),
        "cores": navigator.hardware_concurrency(),
        "device_memory_gb": memory,
        "timer_step_ms": timer_step(),
        "heap_reported": heap_used().is_some(),
    })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// The smallest nonzero step of the page's clock in milliseconds, found by reading it until it moves. A page that is not
/// cross-origin isolated gets a coarser clock than one that is, so the figure is measured and not assumed.
pub fn timer_step() -> f64 {
    let mut least = f64::MAX;
    for _ in 0..200 {
        let start = now();
        let mut next = now();
        let mut reads = 0;
        while next <= start && reads < 5_000_000 {
            next = now();
            reads += 1;
        }
        if next > start {
            least = least.min(next - start);
        }
    }
    if least == f64::MAX { 0.0 } else { round(least) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn latency_json(samples: &[f64]) -> Value {
    Latency::from_samples(samples).map_or(Value::Null, |l| l.to_json())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn round(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds a scenario in a host of its own and reports the cost: the time of each part, the page's elements, and the heap.
///
/// The host and the scene are given back, so that the caller can drive them and then tear them down.
///
/// # Errors
///
/// Returns `Err` if the host or the scene could not be built.
pub fn build(id: &str, prepared: &Prepared, run: &Run) -> Result<(Host, Built, Value), String> {
    build_with(id, prepared, run, BuildOptions::default())
}

/// As [`build`], with the options that make the scene readable, so that what they cost can be measured.
///
/// # Errors
///
/// Returns `Err` if the host or the scene could not be built.
pub fn build_with(
    id: &str,
    prepared: &Prepared,
    run: &Run,
    options: BuildOptions,
) -> Result<(Host, Built, Value), String> {
    let layout = crate::layout::Layout::new(prepared.spec());
    let heap_before = heap_used();
    let host = Host::create(id, (layout.width, layout.height))?;
    let baseline = element_count(&host.svg()?)?;
    let built = Built::build_with(&host.svg_id, prepared, run, options)?;
    let elements = element_count(&host.svg()?)?;
    let t = built.timings;
    let plan = prepared.plan();
    let report = json!({
        "nodes": plan.nodes, "edges": plan.edges, "cells": plan.cells, "max_degree": plan.max_degree,
        "label_len": prepared.spec().label_len,
        "estimated_dom": plan.est_dom, "dom_elements": elements, "dom_baseline": baseline,
        "build_ms": { "total": round(t.total_ms), "nodes": round(t.nodes_ms), "edges": round(t.edges_ms), "data": round(t.data_ms), "spy": round(t.spy_ms), "style": round(t.style_ms) },
        "heap_before": heap_before, "heap_after": heap_used(),
    });
    Ok((host, built, report))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs `steps` steps of the scenario, writing the data nodes after each, and reports the latency of the pure computation, of the
/// two data-node writes, and of both together. Each step's updates are the scenario's own, and no step is verified, so that the
/// figures are those of the draw and not of a check.
///
/// # Errors
///
/// Returns `Err` if a write failed.
pub fn updates(built: &Built, run: &mut Run, steps: usize) -> Result<Value, String> {
    let (mut compute, mut x_write, mut y_write, mut whole) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for _ in 0..steps {
        let t0 = now();
        run.advance(false).map_err(|e| e.to_string())?;
        let t1 = now();
        let (x_ms, y_ms) = built.apply(run)?;
        compute.push(t1 - t0);
        x_write.push(x_ms);
        y_write.push(y_ms);
        whole.push(now() - t0);
    }
    Ok(json!({
        "steps": steps,
        "compute_ms": latency_json(&compute), "x_write_ms": latency_json(&x_write),
        "y_write_ms": latency_json(&y_write), "step_ms": latency_json(&whole),
    }))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Moves column node `j` back and forth `moves` times, which redraws each of its connectors each time, and reports the latency of a
/// move. This is the per-frame cost of dragging a node of that degree.
///
/// # Errors
///
/// Returns `Err` if a move failed.
pub fn drag(built: &Built, j: usize, moves: usize) -> Result<Value, String> {
    let mut samples = Vec::with_capacity(moves);
    for n in 0..moves {
        let dx = if n % 2 == 0 { 7.0 } else { -7.0 };
        samples.push(built.move_col(j, dx, 3.0 * dx)?);
    }
    Ok(json!({ "moves": moves, "move_ms": latency_json(&samples) }))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Builds and tears down the scenario `cycles` times, and reports the page's elements and the heap after each, so that a rise in the
/// baseline would show. The elements of the host are counted with the container removed, which is what a teardown leaves.
///
/// # Errors
///
/// Returns `Err` if a build failed.
pub fn rebuild(id: &str, prepared: &Prepared, cycles: usize) -> Result<Value, String> {
    let document = web_sys::window().and_then(|w| w.document()).ok_or("there is no document")?;
    let count_body = || -> Result<u32, String> { element_count(&document.body().ok_or("no body")?.into()) };
    let before = count_body()?;
    let heap_start = heap_used();
    let (mut after_elements, mut heap_after, mut build_ms) = (Vec::new(), Vec::new(), Vec::new());
    for n in 0..cycles {
        let run = prepared.run();
        let (host, built, report) = build(&format!("{id}-{n}"), prepared, &run)?;
        build_ms.push(report["build_ms"]["total"].as_f64().unwrap_or(0.0));
        drop(built);
        host.remove();
        after_elements.push(count_body()?);
        heap_after.push(heap_used());
    }
    // The heap is noisy, since it depends on when the collector last ran, so the trend is the median of the last quarter of the
    // cycles against the median of the first quarter.
    let heaps: Vec<f64> = heap_after.iter().flatten().copied().collect();
    let median = |v: &[f64]| -> Option<f64> {
        let mut s = v.to_vec();
        s.sort_by(f64::total_cmp);
        s.get(s.len() / 2).copied()
    };
    let quarter = (heaps.len() / 4).max(1);
    let (first, last) = (
        median(&heaps[..quarter.min(heaps.len())]),
        median(&heaps[heaps.len().saturating_sub(quarter)..]),
    );
    Ok(json!({
        "cycles": cycles, "body_elements_before": before, "body_elements_after": after_elements,
        "elements_stable": after_elements.iter().all(|n| *n == before),
        "heap_start": heap_start, "heap_after": heap_after, "heap_first_quarter_median": first, "heap_last_quarter_median": last,
        "build_ms": latency_json(&build_ms),
    }))
}
