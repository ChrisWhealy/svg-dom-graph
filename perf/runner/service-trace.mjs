// The trace that workload W6 times, and the test of whether it may be timed (design revision 6, section 8.3).
//
// The whole deterministic trace is worked out before anything is timed, by the independent JavaScript engine (dess/reference-js, which
// shares no code with the Rust one). Its slices fix the windows that every layer consumes: slices 1 to 256 are priming, 257 to 296 are the
// warm-up of Layers A and B, and 297 to 536 are their measured window. Layer C starts again from the cursor after slice 256, and takes 10
// warm-up commits and 30 measured ones, which end at slice 296.
//
// A trace that cannot fill the windows, or whose measured window is not busy and steady enough to mean anything, is refused here, before
// any timing, and the refusal is a result of its own with every figure that was worked out, so that a refused seed is a record and not
// a gap. The bounds are the design's, written before any trace was measured and frozen in dess/s0/prototype/preflight.mjs: they are
// imported from there, so that they have one definition, and they are never tuned to keep a seed.
import { createHash } from 'node:crypto'
import { run } from '../../dess/reference-js/oracle.mjs'
import { MIXED_OUTAGE_7 } from '../../dess/reference-js/generator.mjs'
import { BOUNDS, MIN_POPS, MIN_SLICES, WINDOW, evaluate } from '../../dess/s0/prototype/preflight.mjs'

const sha = text => createHash('sha256').update(text).digest('hex')
const mean = values => values.reduce((a, b) => a + b, 0) / values.length

/** The slices of each window, as inclusive ranges of slice numbers. */
export const WINDOWS = {
  priming: [1, WINDOW.priming],
  warmup: [WINDOW.priming + 1, WINDOW.priming + WINDOW.warmup],
  measured: [WINDOW.priming + WINDOW.warmup + 1, MIN_SLICES],
  // Layer C: 10 warm-up commits and 30 measured ones from the cursor after the priming slices, which is slices 257 to 266 and 267 to 296.
  interactiveWarmup: [WINDOW.priming + 1, WINDOW.priming + 10],
  interactiveMeasured: [WINDOW.priming + 11, WINDOW.priming + WINDOW.warmup]
}

const within = (slices, [from, to]) => slices.filter(s => s.index >= from && s.index <= to)

/** The activity figures of a window, which are the same figures at either length and are all that the short window records. */
const activity = (slices, capacity) => {
  if (slices.length === 0) return null
  return {
    slices: slices.length,
    occupancy_mean: mean(slices.map(s => s.waiting / capacity)),
    waiting_nonempty_fraction: mean(slices.map(s => (s.waiting > 0 ? 1 : 0))),
    pops_per_slice: mean(slices.map(s => s.pops)),
    busy_fraction: mean(slices.map(s => (s.busy >= 1 ? 1 : 0)))
  }
}

/** How many slices ended with each waiting count, from 0 to the capacity: the occupancy histogram of a window, exactly. */
const histogram = (slices, capacity) => {
  const counts = new Array(capacity + 1).fill(0)
  for (const s of slices) counts[Math.min(s.waiting, capacity)] += 1
  return counts
}

/**
 * The trace of the manifest's simulation, and whether it may be timed.
 *
 * `simulation` is the manifest's `[simulation]` table and `seed` its seed. The generator that it names has to be the frozen revision, which
 * the manifest reader has already checked, so the figures that it gives are not consulted again here and only `min_pops` is.
 */
export const buildTrace = (simulation, seed) => {
  // A scaled run is the frozen revision with more or fewer arrivals. The draws of an arrival do not depend on how many there are.
  const params = simulation.arrivals ? { ...MIXED_OUTAGE_7, arrivals: simulation.arrivals } : MIXED_OUTAGE_7
  const trace = run({ configId: params.revision, seed, config: params, params })
  const capacity = params.queueCapacity
  const slices = trace.slices
  const counters = trace.counters
  const bare = slices.map(s => ({ index: s.index, pops: s.pops, waiting: s.waiting, busy: s.busy }))
  const verdict = evaluate(bare, capacity, counters.pops)
  const reasons = [...verdict.reasons]
  // The pop minimum is the manifest's own, and the design's 10,000 is the least that it may ask. It is independent of the slice count.
  if (simulation.min_pops > MIN_POPS && counters.pops < simulation.min_pops)
    reasons.push(`only ${counters.pops} pops, and the manifest asks for ${simulation.min_pops}`)
  const measured = within(bare, WINDOWS.measured)
  const report = {
    generator_revision: params.revision,
    seed,
    qualified: reasons.length === 0,
    reasons,
    // The half-window figures and the activity of the measured window, which are worked out whenever the window exists, so that a
    // refusal says how far from the bounds it was.
    metrics: verdict.metrics,
    occupancy_histogram: measured.length === 0 ? null : histogram(measured, capacity),
    measured_window: activity(measured, capacity),
    // The shorter window of Layer C records the same activity counters, and has no half-window criterion.
    interactive_window: activity(within(bare, WINDOWS.interactiveMeasured), capacity),
    minimums: {
      slices: { required: MIN_SLICES, actual: slices.length, met: slices.length >= MIN_SLICES },
      pops: {
        required: Math.max(MIN_POPS, simulation.min_pops),
        actual: counters.pops,
        met: counters.pops >= Math.max(MIN_POPS, simulation.min_pops)
      }
    },
    bounds: BOUNDS
  }
  const pops = counters.pops
  const chain = [trace.initial_checkpoint, ...slices.map(s => s.checkpoint)]
  return {
    qualification: report,
    // What the trace holds, with the stale and no-op pops counted as pops, since an event is a popped heap event whatever it did.
    counts: {
      arrivals: counters.arrivals,
      pops,
      stale_pops: counters.stale,
      noop_pops: counters.noops,
      slices: slices.length,
      stale_fraction: pops === 0 ? 0 : counters.stale / pops,
      pops_per_slice: slices.length === 0 ? 0 : pops / slices.length
    },
    // The three hashes are separate because each can move alone: the input that was generated, the order in which the heap popped it, and
    // the state after each slice.
    hashes: {
      input_stream_sha256: trace.input_stream_sha256,
      event_pop_sha256: trace.event_pop_sha256,
      checkpoint_chain_sha256: sha(chain.join('\n'))
    },
    checkpoints: chain,
    slices,
    windows: WINDOWS,
    // What the independent engine says each slice is, for the runners to hold themselves to. The counts of a slice's pops and of its stale
    // ones are worked out from the engine's pop list, which is in the order of the heap, so that a slice's pops are the next ones.
    expected: expectedOf(trace, chain)
  }
}

/** The part of a trace that a runner is given: the hashes, the counters, and for each slice how many events it popped and how many were stale. */
const expectedOf = (trace, chain) => {
  const slicePops = []
  const sliceStale = []
  let at = 0
  for (const slice of trace.slices) {
    let stale = 0
    for (let i = 0; i < slice.pops; i++) if (trace.pops[at + i][3] === 1) stale += 1
    slicePops.push(slice.pops)
    sliceStale.push(stale)
    at += slice.pops
  }
  return {
    // The windows are the design's and come from here, so that a runner that has its own idea of where they are is held to this one.
    windows: WINDOWS,
    checkpoints: chain,
    counters: { pops: trace.counters.pops, stale: trace.counters.stale, slices: trace.slices.length },
    slice_pops: slicePops,
    slice_stale: sliceStale
  }
}
