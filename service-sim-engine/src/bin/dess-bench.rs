//! Times a seek: a replay from the start to a cursor, with the engine's fast path that records nothing and hashes nothing.
//!
//! It repeats the measurement that stage S0 made with the JavaScript prototype, now with the Rust engine. It prints one JSON object.
//! It runs natively and under WASI, so that the same figure is had for the 32-bit wasm build that the browser will run.

use service_sim_engine::{Engine, Limits, Params};
use std::time::Instant;

const SEED: u64 = 20_261_106;
const TARGET_POPS: [u64; 3] = [1_000, 10_000, 100_000];
/// Measured over the qualification panel: 11,214 pops for 4,000 arrivals.
const POPS_PER_ARRIVAL: f64 = 2.8;
const CURSORS: u64 = 8;
const REPEATS: usize = 15;

fn engine(arrivals: u64) -> Engine {
    let mut params = Params::mixed_outage_7();
    params.arrivals = arrivals;
    let config = params.config().expect("the frozen revision is valid");
    Engine::new("bench", SEED, config, &[], Some(&params), Limits::default())
        .expect("a valid scenario")
        .without_pop_hash()
}

fn seek(arrivals: u64, cursor: u64) -> f64 {
    let start = Instant::now();
    let mut e = engine(arrivals);
    for _ in 0..cursor {
        if !e.advance().expect("the replay runs") {
            break;
        }
    }
    start.elapsed().as_secs_f64() * 1000.0
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    sorted[((q * sorted.len() as f64) as usize).min(sorted.len() - 1)]
}

fn main() {
    let mut rows = Vec::new();
    for target in TARGET_POPS {
        let arrivals = (target as f64 / POPS_PER_ARRIVAL).ceil() as u64;
        let mut whole = engine(arrivals);
        while whole.advance().expect("the run") {}
        let (pops, slices) = (whole.counters().pops, whole.counters().slices);
        let mut times = Vec::new();
        for i in 1..=CURSORS {
            let cursor = (slices * i / CURSORS).max(1);
            for _ in 0..REPEATS {
                times.push(seek(arrivals, cursor));
            }
        }
        times.sort_by(f64::total_cmp);
        let mut short: Vec<f64> = (0..REPEATS * 4).map(|_| seek(arrivals, 128)).collect();
        short.sort_by(f64::total_cmp);
        rows.push(format!(
            "{{\"actual_pops\":{pops},\"slices\":{slices},\"seek_ms\":{{\"p50\":{:.4},\"p95\":{:.4},\"worst\":{:.4}}},\"replay_of_128_slices_ms\":{{\"p50\":{:.4},\"p95\":{:.4}}}}}",
            quantile(&times, 0.5),
            quantile(&times, 0.95),
            times[times.len() - 1],
            quantile(&short, 0.5),
            quantile(&short, 0.95)
        ));
    }
    // What hashing the state costs, which is what a checkpoint cache would pay to make a checkpoint.
    let mut e = engine(4000);
    for _ in 0..2000 {
        e.advance().expect("the replay runs");
    }
    let mut hashing: Vec<f64> = (0..200)
        .map(|_| {
            let start = Instant::now();
            std::hint::black_box(e.checkpoint(0, 2000));
            start.elapsed().as_secs_f64() * 1000.0
        })
        .collect();
    hashing.sort_by(f64::total_cmp);
    println!(
        "{{\"engine\":\"Rust, fast path (no hash, no record)\",\"pointer_width\":{},\"rows\":[{}],\"checkpoint_hash_ms\":{{\"p50\":{:.4},\"p95\":{:.4}}}}}",
        usize::BITS,
        rows.join(","),
        quantile(&hashing, 0.5),
        quantile(&hashing, 0.95)
    );
}
