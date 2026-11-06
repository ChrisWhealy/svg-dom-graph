//! Checks that the counting allocator tells the truth, before its figures are used.
//!
//! The design says the allocator is not authoritative until it has been shown to count a deliberately retained allocation, a
//! release, and a leaked closure. Each step is repeated many times, so that a counter that drifts or counts twice shows.
//!
//! The steps are:
//!
//! 1. Retain blocks of a known size. Every one must add exactly its size and one block.
//! 2. Release them. The count must return exactly to where it started.
//! 3. Leak closures with `Closure::forget`, as the library does. Each must add at least what it captures, and the bytes must stay.
//! 4. Call the library's own `svg_buttons::watch`, which leaks two closures a call. It must add bytes on every call, the same
//!    on each, and removing the element it watches must not give them back.
//! 5. Grow one vector a byte at a time, which reallocates. The count must equal its capacity, and release exactly.
//! 6. Repeat the retain and release again and again. Each must return exactly to the start.
//! 7. The linear memory is at least as large as the most that was ever live.

use crate::alloc::{AllocStats, stats};
use crate::memory::capacity_bytes;
use serde_json::{Value, json};
use std::hint::black_box;
use wasm_bindgen::prelude::*;

/// How many times each step is repeated.
const ROUNDS: usize = 100;
/// The size of each retained block.
const BLOCK: usize = 64 * 1024;
/// What each leaked closure captures, so that it is a known number of bytes.
const CAPTURE: usize = 4096;
/// The repeats of retain-and-release in step 6.
const REPEATS: usize = 25;
/// The most a leaked closure may add beyond what it captures: its own box and a little bookkeeping.
const CLOSURE_OVERHEAD: usize = 256;

fn step(name: &str, expectation: &str, pass: bool, observed: Value) -> Value {
    json!({"name": name, "expectation": expectation, "pass": pass, "observed": observed})
}

fn difference(after: AllocStats, before: AllocStats) -> (isize, isize) {
    (
        after.live_bytes as isize - before.live_bytes as isize,
        after.live_allocations as isize - before.live_allocations as isize,
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn retained_and_released() -> Vec<Value> {
    // The outer vector is made before the starting reading, so that pushing into it allocates nothing of its own.
    // The vector that records each reading is made before it too, for the same reason.
    let mut held: Vec<Vec<u8>> = Vec::with_capacity(ROUNDS);
    let mut per_block: Vec<(isize, isize)> = Vec::with_capacity(ROUNDS);
    let start = stats();
    for round in 0..ROUNDS {
        let before = stats();
        held.push(vec![(round as u8) | 1; BLOCK]);
        per_block.push(difference(stats(), before));
    }
    black_box(&held);
    let held_total = difference(stats(), start);
    let exact = per_block.iter().all(|d| *d == (BLOCK as isize, 1));

    held.clear();
    let released = difference(stats(), start);
    vec![
        step(
            "retain",
            &format!("each of {ROUNDS} retained blocks of {BLOCK} bytes adds exactly {BLOCK} bytes and one block"),
            exact && held_total == ((ROUNDS * BLOCK) as isize, ROUNDS as isize),
            json!({"per_block_all_exact": exact, "total_bytes": held_total.0, "total_blocks": held_total.1, "expected_bytes": ROUNDS * BLOCK}),
        ),
        step(
            "release",
            "releasing every retained block returns the live bytes and blocks exactly to where they began",
            released == (0, 0),
            json!({"bytes_not_returned": released.0, "blocks_not_returned": released.1}),
        ),
    ]
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn leaked_closures() -> Value {
    let mut deltas: Vec<isize> = Vec::with_capacity(ROUNDS);
    let mut live_after_each: Vec<usize> = Vec::with_capacity(ROUNDS);
    let start = stats();
    for round in 0..ROUNDS {
        let before = stats();
        let payload = vec![round as u8; CAPTURE];
        let closure = Closure::<dyn FnMut()>::new(move || {
            black_box(&payload);
        });
        // This is the call the library makes, and it is the one that gives the closure up to the page for good.
        closure.forget();
        let after = stats();
        deltas.push(difference(after, before).0);
        live_after_each.push(after.live_bytes);
    }
    // Allocating and freeing something else must not give a leaked closure back.
    drop(black_box(vec![0u8; 1 << 20]));
    let total = difference(stats(), start).0;

    // Every closure adds at least what it captures, and the bytes never go down. The most it may add beyond that is a bound,
    // since wasm-bindgen keeps tables of its own that grow from time to time, so the overhead is not the same on every call.
    let each_at_least = deltas.iter().all(|d| *d >= CAPTURE as isize);
    let never_falls = live_after_each.windows(2).all(|pair| pair[1] >= pair[0]);
    let overhead_total = total - (ROUNDS * CAPTURE) as isize;
    let overhead_bounded = overhead_total >= 0 && overhead_total <= (ROUNDS * CLOSURE_OVERHEAD) as isize;
    let smallest = deltas.iter().min().copied().unwrap_or(0);
    let largest = deltas.iter().max().copied().unwrap_or(0);
    step(
        "leaked closure",
        &format!(
            "each of {ROUNDS} closures leaked with forget() adds at least the {CAPTURE} bytes it captures, the live bytes never fall, the extra is at most {CLOSURE_OVERHEAD} bytes a closure on average, and none comes back"
        ),
        each_at_least && never_falls && overhead_bounded && total >= (ROUNDS * CAPTURE) as isize,
        json!({
            "smallest_per_closure": smallest,
            "largest_per_closure": largest,
            "all_at_least_capture": each_at_least,
            "live_bytes_never_fell": never_falls,
            "overhead_total_bytes": overhead_total,
            "overhead_allowed_bytes": ROUNDS * CLOSURE_OVERHEAD,
            "total_bytes_after_other_churn": total,
            "minimum_total": ROUNDS * CAPTURE,
        }),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn library_leak() -> Value {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return step(
            "library leak",
            "the library's own leak is counted",
            false,
            json!({"error": "there is no document"}),
        );
    };
    let Some(body) = document.body() else {
        return step(
            "library leak",
            "the library's own leak is counted",
            false,
            json!({"error": "there is no body"}),
        );
    };
    let Ok(stage) = document.create_element("div") else {
        return step(
            "library leak",
            "the library's own leak is counted",
            false,
            json!({"error": "no stage could be made"}),
        );
    };
    stage.set_id("perf-allocator-stage");
    let _ = body.append_child(&stage);

    let mut deltas: Vec<isize> = Vec::with_capacity(ROUNDS);
    let mut live_after_each: Vec<usize> = Vec::with_capacity(ROUNDS);
    let start = stats();
    for _ in 0..ROUNDS {
        let before = stats();
        if let Err(message) = svg_dom_graph::controls::svg_buttons::watch(&document, "perf-allocator-stage", |_| {}) {
            return step(
                "library leak",
                "the library's own leak is counted",
                false,
                json!({"error": message}),
            );
        }
        let after = stats();
        deltas.push(difference(after, before).0);
        live_after_each.push(after.live_bytes);
    }
    let leaked = difference(stats(), start).0;
    stage.remove();
    let after_removal = difference(stats(), start).0;
    // The library's leak has no size that can be known in advance, so this shows what matters about it: that it is counted,
    // that every call adds, that the bytes never fall, and that removing the element it watches does not give them back.
    let positive = deltas.iter().all(|d| *d > 0);
    let never_falls = live_after_each.windows(2).all(|pair| pair[1] >= pair[0]);
    let smallest = deltas.iter().min().copied().unwrap_or(0);
    let largest = deltas.iter().max().copied().unwrap_or(0);
    step(
        "library leak",
        &format!(
            "{ROUNDS} calls of the library's svg_buttons::watch, which leaks two closures each, add bytes on every call, the live bytes never fall, and removing the element gives none back"
        ),
        positive && never_falls && leaked == deltas.iter().sum::<isize>() && after_removal >= leaked,
        json!({"smallest_per_call": smallest, "largest_per_call": largest, "all_positive": positive, "live_bytes_never_fell": never_falls, "total_bytes": leaked, "bytes_after_removing_the_element": after_removal}),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Growing one byte at a time is the point: it makes the vector reallocate many times. `vec![1; N]` would allocate once.
#[allow(clippy::same_item_push)]
fn reallocation() -> Value {
    let start = stats();
    let mut grown: Vec<u8> = Vec::new();
    for _ in 0..(1usize << 20) {
        grown.push(1);
    }
    let held = difference(stats(), start);
    let capacity = grown.capacity() as isize;
    drop(black_box(grown));
    let released = difference(stats(), start);
    step(
        "reallocation",
        "a vector grown a byte at a time, which reallocates many times, is counted at its capacity as one block, and released exactly",
        held == (capacity, 1) && released == (0, 0),
        json!({"live_bytes": held.0, "vector_capacity": capacity, "blocks": held.1, "bytes_not_returned": released.0}),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn repeated_release() -> Value {
    let mut drift: Vec<(isize, isize)> = Vec::with_capacity(REPEATS);
    let start = stats();
    for round in 0..REPEATS {
        let mut held: Vec<Vec<u8>> = Vec::with_capacity(16);
        for index in 0..16 {
            held.push(vec![(round + index) as u8; 1000 + index * 37]);
        }
        black_box(&held);
        drop(held);
        drift.push(difference(stats(), start));
    }
    step(
        "repeated release",
        &format!(
            "{REPEATS} rounds of retain and release each return exactly to the start, so nothing drifts or counts twice"
        ),
        drift.iter().all(|d| *d == (0, 0)),
        json!({"worst_bytes_drift": drift.iter().map(|d| d.0.abs()).max(), "worst_blocks_drift": drift.iter().map(|d| d.1.abs()).max()}),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
fn capacity_is_a_ceiling() -> Value {
    let peak = stats().peak_live_bytes as f64;
    let capacity = capacity_bytes();
    step(
        "capacity",
        "the linear memory is at least as large as the most that was ever live",
        capacity.is_some_and(|bytes| bytes >= peak),
        json!({"capacity_bytes": capacity, "peak_live_bytes": peak}),
    )
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/// Runs every step and gives the result as JSON, with an overall `pass`.
pub fn verify() -> String {
    let mut steps = retained_and_released();
    steps.push(leaked_closures());
    steps.push(library_leak());
    steps.push(reallocation());
    steps.push(repeated_release());
    steps.push(capacity_is_a_ceiling());
    let pass = steps.iter().all(|s| s["pass"] == true);
    let end = stats();
    json!({
        "pass": pass,
        "rounds": ROUNDS,
        "steps": steps,
        "final": {"live_bytes": end.live_bytes, "live_allocations": end.live_allocations, "total_allocations": end.total_allocations, "peak_live_bytes": end.peak_live_bytes},
    })
    .to_string()
}
