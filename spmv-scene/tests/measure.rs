//! The measurements of the resource risks that `docs/security.md` documents. They are heavy, so every one is ignored by default.
//!
//!   wasm-pack test --headless --chrome spmv-scene --test measure -- --nocapture --include-ignored
//!
//! Each prints one JSON record on a line that begins `SPMV-REPORT `. A record reports; it asserts nothing about speed, since a timing
//! is never a gate. They use an unlimited budget on purpose: the point is to find where behaviour degrades, which a budget would hide.
#![cfg(target_arch = "wasm32")]

use serde_json::{Value, json};
use spmv_core::Prepared;
use spmv_core::budget::Budget;
use spmv_core::spec::Spec;
use spmv_scene::probe;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};

wasm_bindgen_test_configure!(run_in_browser);

fn emit(record: &Value) {
    web_sys::console::log_1(&format!("SPMV-REPORT {record}").into());
}

fn spec(rows: u64, cols: u64, per_row: u64, hubs: (u64, u64), label_len: u64, cells: u64, motion: (u64, u64)) -> Spec {
    Spec {
        seed: 20_261_109,
        rows,
        cols,
        per_row,
        hub_cols: hubs.0,
        hub_percent: hubs.1,
        label_len,
        cells,
        updates: motion.0,
        steps: motion.1,
    }
}

fn open(s: &Spec) -> Result<Prepared, String> {
    Prepared::new(s, &Budget::unlimited()).map_err(|e| e.to_string())
}

/// Builds, drives the updates, drags the busiest column and a quiet one, and tears down.
fn full_run(tag: &str, s: &Spec, steps: usize, moves: usize) -> Result<Value, String> {
    let p = open(s)?;
    let mut run = p.run();
    let (host, built, build) = probe::build(&format!("m-{tag}"), &p, &run)?;
    let hub = (0..s.cols as usize).max_by_key(|j| p.matrix().col(*j).count()).unwrap_or(0);
    let quiet = (0..s.cols as usize)
        .filter(|j| p.matrix().col(*j).count() > 0)
        .min_by_key(|j| p.matrix().col(*j).count())
        .unwrap_or(0);
    let hub_degree = p.matrix().col(hub).count();
    let updates = probe::updates(&built, &mut run, steps)?;
    let drag_hub = probe::drag(&built, hub, moves)?;
    let drag_quiet = probe::drag(&built, quiet, moves)?;
    drop(built);
    host.remove();
    Ok(json!({
        "tag": tag, "build": build, "updates": updates,
        "drag_hub": { "degree": hub_degree, "result": drag_hub },
        "drag_quiet": { "degree": p.matrix().col(quiet).count(), "result": drag_quiet },
    }))
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Risk: the total number of nodes and edges. 100, 1,000 and 5,000 graph entities, with a flat and a hub-heavy adjacency.
#[wasm_bindgen_test]
#[ignore = "heavy: a measurement, not a test"]
fn sizes() -> Result<(), String> {
    emit(&json!({ "probe": "environment", "environment": probe::environment() }));
    for entities in [100u64, 1_000, 5_000] {
        let half = entities / 2;
        let cells = (half / 5).clamp(10, 100);
        for (kind, hubs) in [("flat", (0, 0)), ("hub-heavy", (half.min(20), 60))] {
            let s = spec(half, half, 3, hubs, 6, cells, (cells / 2, 40));
            let record = full_run(&format!("{kind}-{entities}"), &s, 40, 30)?;
            emit(&json!({ "probe": "sizes", "entities": entities, "adjacency": kind, "record": record }));
        }
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Risk: node degree. One column is joined to every row, so its degree is the number of rows. Each edge appends a relationship clause
/// to both ends and rewrites the whole description, so the build is expected to grow faster than the degree, and a drag redraws every
/// edge on the node.
#[wasm_bindgen_test]
#[ignore = "heavy: a measurement, not a test"]
fn degree() -> Result<(), String> {
    for degree in [10u64, 50, 100, 250, 500, 1_000, 2_000] {
        let s = spec(degree, 40, 1, (1, 100), 6, 10, (4, 20));
        let p = open(&s)?;
        let mut run = p.run();
        let (host, built, build) = probe::build(&format!("m-deg-{degree}"), &p, &run)?;
        let description = host
            .svg()?
            .query_selector(":scope > g.svg-dom-graph-content > g")
            .map_err(|e| format!("{e:?}"))?
            .and_then(|g| g.get_attribute("aria-label"))
            .map_or(0, |a| a.len());
        let drag = probe::drag(&built, 0, 30)?;
        let updates = probe::updates(&built, &mut run, 20)?;
        emit(&json!({
            "probe": "degree", "degree": degree, "hub_description_bytes": description,
            "build": build, "drag_hub": drag, "updates": updates,
        }));
        drop(built);
        host.remove();
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Risk: string lengths. The same names are copied into the relationship strings of both ends of every edge, and each label is
/// measured by the browser. A hub of 200 edges, with labels of 4 to 1,024 characters.
#[wasm_bindgen_test]
#[ignore = "heavy: a measurement, not a test"]
fn label_lengths() -> Result<(), String> {
    for len in [4u64, 32, 128, 256, 512, 1_024] {
        let s = spec(200, 40, 1, (1, 100), len, 10, (4, 20));
        let p = open(&s)?;
        let run = p.run();
        let (host, built, build) = probe::build(&format!("m-lab-{len}"), &p, &run)?;
        let description = host
            .svg()?
            .query_selector(":scope > g.svg-dom-graph-content > g")
            .map_err(|e| format!("{e:?}"))?
            .and_then(|g| g.get_attribute("aria-label"))
            .map_or(0, |a| a.len());
        let drag = probe::drag(&built, 0, 30)?;
        emit(
            &json!({ "probe": "labels", "label_len": len, "hub_description_bytes": description, "build": build, "drag_hub": drag }),
        );
        drop(built);
        host.remove();
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Risk: the cells of a data node, and the cost of rewriting them. 10, 100 and 1,000 cells, with half of them changed each step.
#[wasm_bindgen_test]
#[ignore = "heavy: a measurement, not a test"]
fn cells() -> Result<(), String> {
    for cells in [10u64, 100, 1_000] {
        let s = spec(1_000, 1_000, 2, (0, 0), 4, cells, (cells / 2, 60));
        let p = open(&s)?;
        let mut run = p.run();
        let (host, built, build) = probe::build(&format!("m-cells-{cells}"), &p, &run)?;
        let updates = probe::updates(&built, &mut run, 60)?;
        emit(&json!({ "probe": "cells", "cells": cells, "build": build, "updates": updates }));
        drop(built);
        host.remove();
    }
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Risk: memory growth when scenes are rebuilt. 80 builds and teardowns of a scene of 1,000 entities.
#[wasm_bindgen_test]
#[ignore = "heavy: a measurement, not a test"]
fn rebuilds() -> Result<(), String> {
    let s = spec(500, 500, 3, (5, 40), 6, 50, (10, 5));
    let p = open(&s)?;
    emit(&json!({ "probe": "rebuilds", "entities": 1_000, "report": probe::rebuild("m-rebuild", &p, 80)? }));
    Ok(())
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Where behaviour degrades. The sweeps above sit comfortably inside what the browser can do, so these go on: more nodes, and a hub of
/// high degree with long labels, where the description a node accumulates is rewritten whole for each edge.
///
/// The test runner's WebDriver gives a synchronous test about 20 seconds, so each sweep is its own test and stops itself when it has
/// used more than `BUDGET_MS`. A sweep that stops early says so in its output, and that is a result: it is where the sweep became too
/// slow to go on.
const BUDGET_MS: f64 = 13_000.0;

#[wasm_bindgen_test]
#[ignore = "heavy: a measurement, not a test"]
fn limits_nodes() -> Result<(), String> {
    let start = spmv_scene::scene::now();
    let mut last: Option<(u64, f64)> = None;
    for entities in [10_000u64, 14_000, 20_000, 28_000, 40_000] {
        let elapsed = spmv_scene::scene::now() - start;
        // The cost of making nodes is found to grow with the square of their number, so the next build is predicted from the last and
        // not started if it would not fit in the time the runner allows.
        if let Some((n, ms)) = last {
            let predicted = ms * (entities as f64 / n as f64).powi(2);
            if elapsed + predicted > BUDGET_MS {
                emit(&json!({
                    "probe": "limits-nodes", "stopped_before": entities, "predicted_build_ms": predicted.round(),
                    "basis": "the last build scaled by the square of the ratio of the sizes",
                    "reason": "the build would not fit in the time the runner allows",
                }));
                break;
            }
        }
        let half = entities / 2;
        let s = spec(half, half, 3, (0, 0), 6, 100, (50, 30));
        let record = full_run(&format!("limit-flat-{entities}"), &s, 30, 20)?;
        last = Some((entities, record["build"]["build_ms"]["total"].as_f64().unwrap_or(0.0)));
        emit(&json!({ "probe": "limits-nodes", "entities": entities, "record": record }));
    }
    Ok(())
}

fn hub_sweep(len: u64) -> Result<(), String> {
    let start = spmv_scene::scene::now();
    let mut last: Option<(u64, f64)> = None;
    for degree in [1_000u64, 2_000, 5_000, 10_000, 20_000] {
        let elapsed = spmv_scene::scene::now() - start;
        // The build of a hub grows with the square of its degree, so the next is predicted from the last.
        if let Some((d, ms)) = last {
            let predicted = ms * (degree as f64 / d as f64).powi(2);
            if elapsed + predicted > BUDGET_MS {
                emit(&json!({
                    "probe": "limits-hub", "label_len": len, "stopped_before": degree, "predicted_build_ms": predicted.round(),
                    "basis": "the last build scaled by the square of the ratio of the degrees",
                    "reason": "the build would not fit in the time the runner allows",
                }));
                break;
            }
        }
        let s = spec(degree, 40, 1, (1, 100), len, 10, (4, 10));
        let p = open(&s)?;
        let mut run = p.run();
        let (host, built, build) = probe::build(&format!("m-lim-{len}-{degree}"), &p, &run)?;
        let description = host
            .svg()?
            .query_selector(":scope > g.svg-dom-graph-content > g")
            .map_err(|e| format!("{e:?}"))?
            .and_then(|g| g.get_attribute("aria-label"))
            .map_or(0, |a| a.len());
        let drag = probe::drag(&built, 0, 20)?;
        let updates = probe::updates(&built, &mut run, 10)?;
        last = Some((degree, build["build_ms"]["total"].as_f64().unwrap_or(0.0)));
        emit(&json!({
            "probe": "limits-hub", "label_len": len, "degree": degree, "hub_description_bytes": description,
            "build": build, "drag_hub": drag, "updates": updates,
        }));
        drop(built);
        host.remove();
    }
    Ok(())
}

#[wasm_bindgen_test]
#[ignore = "heavy: a measurement, not a test"]
fn limits_hub_short_labels() -> Result<(), String> {
    hub_sweep(16)
}

#[wasm_bindgen_test]
#[ignore = "heavy: a measurement, not a test"]
fn limits_hub_medium_labels() -> Result<(), String> {
    hub_sweep(128)
}

#[wasm_bindgen_test]
#[ignore = "heavy: a measurement, not a test"]
fn limits_hub_long_labels() -> Result<(), String> {
    hub_sweep(512)
}
