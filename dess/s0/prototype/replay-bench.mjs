// S0 decision 7: what seeking costs, measured before any checkpoint or chunking machinery is designed.
//
//   node dess/s0/prototype/replay-bench.mjs [--out FILE]
//
// A seek is a replay from the start to a cursor, since a replay is the only thing that gives the state there. This measures it
// at traces of about 1,000, 10,000 and 100,000 pops, to cursors spread over the trace, with the prototype engine in Node. The
// prototype is JavaScript and the S1 engine is Rust, so these figures say how the cost grows and bound it from above. They are
// not the figures of the product, and S1 is to repeat them with the Rust engine under wasm.
//
// A checkpoint every 128 slices would cut a seek to at most 128 slices of replay plus a restore. The replay of 128 slices is
// measured here. The restore is not, since the prototype has no snapshot, and an estimate of it is given from the size of the
// state it would copy.
import fs from 'node:fs'
import { run } from './engine.mjs'
import { MIXED_OUTAGE, configOf, inputsFor } from './generator.mjs'

const out = process.argv.includes('--out') ? process.argv[process.argv.indexOf('--out') + 1] : null
const SEED = 20261106
const TARGET_POPS = [1_000, 10_000, 100_000]
const POPS_PER_ARRIVAL = 2.8 // measured over the qualification panel: 11,255 pops for 4,000 arrivals
const CURSORS = 8
const REPEATS = 15

const quantile = (values, q) => {
  const v = [...values].sort((a, b) => a - b)
  return v[Math.min(v.length - 1, Math.floor(q * v.length))]
}

const rows = []
for (const target of TARGET_POPS) {
  const params = { ...MIXED_OUTAGE, arrivals: Math.ceil(target / POPS_PER_ARRIVAL) }
  const whole = run(configOf(params), inputsFor(SEED, params))
  const { pops, slices } = whole.counters
  const cursors = Array.from({ length: CURSORS }, (_, i) => Math.max(1, Math.round((slices * (i + 1)) / CURSORS)))
  const times = []
  const perCursor = []
  let heapDelta = 0
  for (const cursor of cursors) {
    const samples = []
    for (let r = 0; r < REPEATS; r++) {
      const before = process.memoryUsage().heapUsed
      const start = performance.now()
      run(configOf(params), inputsFor(SEED, params), { stopAfterSlices: cursor })
      samples.push(performance.now() - start)
      heapDelta = Math.max(heapDelta, process.memoryUsage().heapUsed - before)
    }
    times.push(...samples)
    perCursor.push({ cursor, median_ms: +quantile(samples, 0.5).toFixed(3) })
  }
  // The replay that a checkpoint every 128 slices would leave: from slice 0, as the trace is at its start.
  const short = Array.from({ length: REPEATS * 4 }, () => {
    const start = performance.now()
    run(configOf(params), inputsFor(SEED, params), { stopAfterSlices: 128 })
    return performance.now() - start
  })
  rows.push({
    target_pops: target,
    actual_pops: pops,
    slices,
    seek_ms: {
      p50: +quantile(times, 0.5).toFixed(3),
      p95: +quantile(times, 0.95).toFixed(3),
      worst: +Math.max(...times).toFixed(3)
    },
    seek_to_the_end_median_ms: perCursor[perCursor.length - 1].median_ms,
    replay_of_128_slices_ms: { p50: +quantile(short, 0.5).toFixed(3), p95: +quantile(short, 0.95).toFixed(3) },
    peak_heap_growth_bytes: heapDelta,
    per_cursor: perCursor
  })
}

// The state that a checkpoint would copy, at its largest: every live job, every server, and the future heap.
const liveJobs = 40
const heapEvents = 120
const state = {
  jobs: Array.from({ length: liveJobs }, (_, i) => ({
    id: i,
    state: 1,
    attempt: 1,
    token: 3,
    entered: i,
    server: -1,
    priority: 1
  })),
  servers: [0, 1, 2],
  heap: Array.from({ length: heapEvents }, (_, i) => ({ tick: i, rank: 3, seq: i, job: i % liveJobs })),
  counters: Array(12).fill(0)
}
const cloneTimes = Array.from({ length: 200 }, () => {
  const start = performance.now()
  structuredClone(state)
  return performance.now() - start
})
const record = {
  node: process.version,
  seed: SEED,
  engine: 'JavaScript prototype, not the product engine',
  rows,
  checkpoint_restore_estimate: {
    note: 'structuredClone of a state with 40 live jobs and a 120-event heap; the real cost depends on the Rust representation',
    clone_ms_p50: +quantile(cloneTimes, 0.5).toFixed(4),
    clone_ms_p95: +quantile(cloneTimes, 0.95).toFixed(4)
  }
}
if (out) fs.writeFileSync(out, JSON.stringify(record, null, 2) + '\n')
console.log(record.engine, `(Node ${process.version})`)
for (const r of rows)
  console.log(
    `${String(r.actual_pops).padStart(7)} pops, ${String(r.slices).padStart(6)} slices: seek p50 ${r.seek_ms.p50} ms, p95 ${r.seek_ms.p95} ms, worst ${r.seek_ms.worst} ms; ` +
      `to the end ${r.seek_to_the_end_median_ms} ms; 128 slices ${r.replay_of_128_slices_ms.p50} ms (p95 ${r.replay_of_128_slices_ms.p95}); peak heap growth ${(r.peak_heap_growth_bytes / 1e6).toFixed(1)} MB`
  )
console.log(
  `checkpoint copy estimate: p50 ${record.checkpoint_restore_estimate.clone_ms_p50} ms, p95 ${record.checkpoint_restore_estimate.clone_ms_p95} ms`
)
