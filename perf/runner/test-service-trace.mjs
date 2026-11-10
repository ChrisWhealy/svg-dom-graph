// Tests the trace of workload W6 and the test of whether it may be timed, with no browser.
//
//   node perf/runner/test-service-trace.mjs
//
// The manifest seed has to qualify and its figures are the ones that S0 recorded and froze, so a change to the generator, the engine or the
// bounds moves them. A seed that S0 refused has to be refused, with the reason. A trace that cannot meet the manifest's own pop minimum is
// refused whatever its slice count, since the two minimums are independent. A refusal keeps every figure that was worked out.
import fs from 'node:fs'
import path from 'node:path'
import { buildTrace, WINDOWS } from './service-trace.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const frozen = JSON.parse(fs.readFileSync(path.join(root, 'dess/s0/prototype/frozen.json'), 'utf8'))
const simulation = { min_pops: 10000 }
let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

const manifestTrace = buildTrace(simulation, frozen.manifest_seed)
const q = manifestTrace.qualification
check(q.qualified && q.reasons.length === 0, 'the manifest seed qualifies')
check(
  manifestTrace.counts.pops === 11214 &&
    manifestTrace.counts.slices === 5464 &&
    manifestTrace.counts.arrivals === 4000 &&
    manifestTrace.counts.stale_pops === 3002 &&
    manifestTrace.counts.noop_pops === 107,
  `its counts are the recorded 11,214 pops, 5,464 slices, 4,000 arrivals, 3,002 stale and 107 no-ops: ${JSON.stringify(manifestTrace.counts)}`
)
check(
  manifestTrace.hashes.input_stream_sha256 === frozen.input_stream_sha256 &&
    manifestTrace.hashes.event_pop_sha256 === frozen.event_pop_sha256,
  'its input and event-pop hashes are the ones that S0 froze'
)
check(
  new Set(Object.values(manifestTrace.hashes)).size === 3,
  'the three hashes are distinct: the input, the pops and the checkpoints'
)
check(
  manifestTrace.checkpoints.length === manifestTrace.counts.slices + 1,
  'a checkpoint for each slice and one for the start'
)
check(
  Math.abs(q.metrics.occupancyMean - 0.497) < 0.001 &&
    q.metrics.waitingNonemptyFraction === 1 &&
    q.metrics.busyFraction === 1,
  `its measured window has occupancy ${q.metrics.occupancyMean.toFixed(3)}, a nonempty queue and a busy server in every slice`
)
check(
  q.occupancy_histogram.length === 13 && q.occupancy_histogram.reduce((a, b) => a + b, 0) === 240,
  'its occupancy histogram counts the 240 measured slices over the 13 waiting counts'
)
check(
  q.interactive_window.slices === 30 && q.measured_window.slices === 240,
  'the Layer C window has 30 slices and the A and B window 240'
)
check(
  WINDOWS.measured[1] - WINDOWS.measured[0] + 1 === 240 &&
    WINDOWS.interactiveMeasured[1] === WINDOWS.warmup[1] &&
    WINDOWS.interactiveWarmup[0] === WINDOWS.warmup[0],
  'the windows are 297 to 536, and Layer C ends at slice 296 as the warm-up of A and B does'
)

// A seed that S0 refused, on the half-window test.
const refused = buildTrace(simulation, 85)
check(
  !refused.qualification.qualified && refused.qualification.reasons.some(r => /halves differ/.test(r)),
  `seed 85 is refused, as S0 refused it: ${refused.qualification.reasons.join('; ')}`
)
check(
  refused.qualification.metrics !== null &&
    refused.qualification.occupancy_histogram !== null &&
    refused.counts.pops > 0,
  'and the refusal keeps every figure that was worked out'
)

// The pop minimum is the manifest's own, and has nothing to do with how many slices there are.
const tooMany = buildTrace({ min_pops: 12000 }, frozen.manifest_seed)
check(
  !tooMany.qualification.qualified &&
    tooMany.qualification.minimums.slices.met &&
    !tooMany.qualification.minimums.pops.met &&
    tooMany.qualification.reasons.some(r => /12000/.test(r)),
  'a trace with plenty of slices and too few pops is refused for its pops'
)
check(
  buildTrace(simulation, frozen.manifest_seed).hashes.checkpoint_chain_sha256 ===
    manifestTrace.hashes.checkpoint_chain_sha256,
  'the same seed gives the same trace'
)

// A scaled run is the frozen revision with more or fewer arrivals, and the draws of an arrival do not depend on how many there are.
const scaled = buildTrace({ min_pops: 100000, arrivals: 36000 }, frozen.manifest_seed)
check(
  scaled.qualification.qualified && scaled.counts.pops >= 100000 && scaled.counts.arrivals === 36000,
  `a scaled run of about 100,000 pops qualifies: ${scaled.counts.pops} pops, ${scaled.counts.slices} slices`
)
const prefix = buildTrace({ min_pops: 10000, arrivals: 360 }, frozen.manifest_seed)
check(
  !prefix.qualification.qualified &&
    !prefix.qualification.minimums.slices.met &&
    !prefix.qualification.minimums.pops.met &&
    prefix.qualification.reasons.some(r => /only \d+ nonempty slices/.test(r)),
  `a run of about 1,000 pops is refused for timing, for its slices and for its pops: ${prefix.counts.pops} pops, ${prefix.counts.slices} slices`
)
check(
  scaled.hashes.input_stream_sha256 !== manifestTrace.hashes.input_stream_sha256 &&
    scaled.checkpoints.length > manifestTrace.checkpoints.length,
  'and a scaled run is a different input stream, and a longer one'
)

// A seed over 2**53 is not rounded: its decimal text reaches the independent engine as a BigInt, and the seed next to it is another run.
const big = buildTrace(simulation, '9007199254740993')
const rounded = buildTrace(simulation, '9007199254740992')
check(
  big.hashes.input_stream_sha256 !== rounded.hashes.input_stream_sha256 &&
    big.hashes.checkpoint_chain_sha256 !== rounded.hashes.checkpoint_chain_sha256,
  'the seed 9007199254740993 is not run as 9007199254740992, which is what a number would make of it'
)
check(
  buildTrace(simulation, 18446744073709551557n).counts.pops > 0 &&
    buildTrace(simulation, '18446744073709551557').hashes.event_pop_sha256 ===
      buildTrace(simulation, 18446744073709551557n).hashes.event_pop_sha256,
  'and a seed near 2**64 is the same run as a text and as a BigInt'
)

console.log(failures ? `\n${failures} failed` : '\nthe trace of W6 qualifies and refuses as it should')
process.exit(failures ? 1 : 0)
