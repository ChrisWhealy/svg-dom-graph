// Tests the prototype against the design's known answers, and against the Rust PRNG's published vectors.
//
//   node dess/s0/prototype/test-prototype.mjs
import crypto from 'node:crypto'
import { SplitMix64, derive } from './splitmix.mjs'
import { run } from './engine.mjs'
import { FIXTURES } from './fixtures.mjs'
import { MIXED_OUTAGE, configOf, inputsFor } from './generator.mjs'

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

// The vectors of `perf-harness/src/prng/unit_tests.rs`.
{
  const prng = new SplitMix64(1234567)
  const want = [
    6457827717110365317n,
    3203168211198807973n,
    9817491932198370423n,
    4593380528125082431n,
    16408922859458223821n
  ]
  check(
    want.every(w => prng.next() === w),
    'SplitMix64 gives the published reference sequence for the seed 1234567'
  )
  check(
    [
      [42, 0, 6332618229526065668n],
      [42, 1, 7575206361173662766n],
      [0, 0x1000, 18400474137010943219n]
    ].every(([s, t, w]) => derive(s, t).next() === w),
    'derive(seed, stream) gives the first outputs that perf-harness fixes'
  )
}

for (const [name, fixture] of Object.entries(FIXTURES)) {
  const result = run(fixture.config, fixture.inputs, { hash: crypto.createHash('sha256') })
  const c = result.counters
  const want = fixture.expect
  const same =
    c.pops === want.pops &&
    c.slices === want.slices &&
    c.slices + 1 === want.snapshots &&
    c.stale === want.stale &&
    JSON.stringify(result.states) === JSON.stringify(want.states) &&
    (want.servers ? JSON.stringify(result.servers) === JSON.stringify(want.servers) : true) &&
    (want.arrivals === undefined ||
      (c.arrivals === want.arrivals && c.admitted === want.admitted && c.rejected === want.rejected))
  check(
    same,
    `${name}: ${c.pops} pops, ${c.slices} slices, ${c.slices + 1} snapshots, ${c.stale} stale, ${JSON.stringify(result.states)}`
  )
}

{
  const a = run(configOf(MIXED_OUTAGE), inputsFor(7), { hash: crypto.createHash('sha256') })
  const b = run(configOf(MIXED_OUTAGE), inputsFor(7), { hash: crypto.createHash('sha256') })
  const c = run(configOf(MIXED_OUTAGE), inputsFor(8), { hash: crypto.createHash('sha256') })
  check(
    a.hash === b.hash && JSON.stringify(a.counters) === JSON.stringify(b.counters),
    'the same seed gives the same pop sequence and counters'
  )
  check(a.hash !== c.hash, 'another seed gives another pop sequence')
  const k = a.counters
  check(
    k.arrivals === k.admitted + k.rejected &&
      k.admitted === k.completed + k.timedOut + k.cancelled + k.failed + countOpen(a.states),
    `conservation holds at the end of a run: ${k.arrivals} arrivals = ${k.admitted} admitted + ${k.rejected} rejected, and admitted jobs are all in one class`
  )
}

function countOpen(states) {
  return Object.values(states).filter(s => ['queued', 'running', 'retrywait'].includes(s)).length
}

console.log(
  failures ? `\n${failures} did not behave` : "\nthe prototype gives the design's known answers, and is deterministic"
)
process.exit(failures ? 1 : 0)
