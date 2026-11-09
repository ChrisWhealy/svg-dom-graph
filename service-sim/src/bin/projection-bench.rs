//! Times the preparation of a draw, in three parts that are kept apart: extracting the engine's model, building the
//! projection from it, and computing the delta against the previous slice's projection. It also times one engine slice,
//! for scale, and counts the jobs that each extraction copies. It prints one JSON object, and runs natively: the
//! surface needs a browser, and none of this does.
//!
//! The workloads are those of the replay benchmark, `dess-bench`: the frozen revision `mixed_outage/7` at the manifest
//! seed, scaled to about 1,000, 10,000 and 100,000 pops.

use service_sim::projection::{View, extract, project, writes};
use service_sim_engine::{Engine, Limits, Params};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// Counts what is asked of the allocator, so that the copying that a preparation does can be measured and not guessed
/// at.
struct Counting;
static BYTES: AtomicU64 = AtomicU64::new(0);
static CALLS: AtomicU64 = AtomicU64::new(0);

// SAFETY: every call is passed to the system allocator unchanged, and only counted.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        CALLS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        CALLS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// The bytes and the calls that `f` asked the allocator for.
fn allocated(f: impl FnOnce()) -> (u64, u64) {
    let (b, c) = (BYTES.load(Ordering::Relaxed), CALLS.load(Ordering::Relaxed));
    f();
    (BYTES.load(Ordering::Relaxed) - b, CALLS.load(Ordering::Relaxed) - c)
}

const SEED: u64 = 20_261_106;
const TARGET_POPS: [u64; 3] = [1_000, 10_000, 100_000];
/// Measured over the qualification panel: 11,214 pops for 4,000 arrivals.
const POPS_PER_ARRIVAL: f64 = 2.8;
const CURSORS: u64 = 8;
const REPEATS: usize = 25;

fn engine(arrivals: u64) -> Engine {
    let mut params = Params::mixed_outage_7();
    params.arrivals = arrivals;
    let config = params.config().expect("the frozen revision is valid");
    Engine::new("bench", SEED, config, &[], Some(&params), Limits::default())
        .expect("a valid scenario")
        .without_pop_hash()
}

fn micros(f: impl FnOnce()) -> f64 {
    let start = Instant::now();
    f();
    start.elapsed().as_secs_f64() * 1e6
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    sorted[((q * sorted.len() as f64) as usize).min(sorted.len() - 1)]
}

fn summary(mut times: Vec<f64>) -> String {
    times.sort_by(f64::total_cmp);
    format!(
        "{{\"p50\":{:.2},\"p95\":{:.2},\"worst\":{:.2}}}",
        quantile(&times, 0.5),
        quantile(&times, 0.95),
        times[times.len() - 1]
    )
}

fn main() {
    let view = View::default();
    let mut rows = Vec::new();
    for target in TARGET_POPS {
        let arrivals = (target as f64 / POPS_PER_ARRIVAL).ceil() as u64;
        let mut whole = engine(arrivals);
        while whole.advance().expect("the run") {}
        let (pops, slices) = (whole.counters().pops, whole.counters().slices);

        let (mut extract_us, mut build, mut delta, mut step) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        let (mut extract_bytes, mut build_bytes, mut extract_calls) = (Vec::new(), Vec::new(), Vec::new());
        let mut cursors = Vec::new();
        for i in 1..=CURSORS {
            let cursor = (slices * i / CURSORS).max(2);
            let mut e = engine(arrivals);
            for _ in 0..cursor - 1 {
                e.advance().expect("the replay runs");
            }
            // The slice before the cursor is what the page holds, and the cursor is what is about to be drawn.
            let before = project(&extract(&e, &view), &view);
            step.push(micros(|| {
                e.advance().expect("a slice");
            }));
            let jobs = extract(&e, &view).jobs.len();
            cursors.push(jobs);
            for _ in 0..REPEATS {
                let (bytes, calls) = allocated(|| {
                    std::hint::black_box(extract(&e, &view));
                });
                extract_bytes.push(bytes as f64);
                extract_calls.push(calls as f64);
                let m = extract(&e, &view);
                let (bytes, _) = allocated(|| {
                    std::hint::black_box(project(&m, &view));
                });
                build_bytes.push(bytes as f64);
                let mut model = None;
                extract_us.push(micros(|| model = Some(std::hint::black_box(extract(&e, &view)))));
                let model = model.expect("a model");
                let mut after = None;
                build.push(micros(|| after = Some(std::hint::black_box(project(&model, &view)))));
                let after = after.expect("a projection");
                delta.push(micros(|| {
                    std::hint::black_box(writes(Some(&before), &after));
                }));
            }
        }
        rows.push(format!(
            "{{\"actual_pops\":{pops},\"slices\":{slices},\"jobs_copied_at_the_cursors\":{cursors:?},\"engine_one_slice_us\":{},\"model_us\":{},\"projection_us\":{},\"delta_us\":{},\"model_bytes\":{},\"model_allocations\":{},\"projection_bytes\":{}}}",
            summary(step),
            summary(extract_us),
            summary(build),
            summary(delta),
            summary(extract_bytes),
            summary(extract_calls),
            summary(build_bytes)
        ));
    }
    println!(
        "{{\"what\":\"the preparation of one draw, native: times in microseconds, sizes in bytes\",\"pointer_width\":{},\"rows\":[{}]}}",
        usize::BITS,
        rows.join(",")
    );
}
