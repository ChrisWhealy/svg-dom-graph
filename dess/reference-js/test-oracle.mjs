// Tests the oracle against the design's known answers, against the PRNG's published vectors, and against the S0 pins, which another
// implementation (the S0 prototype) produced.
//
//   node dess/reference-js/test-oracle.mjs
import fs from 'node:fs'
import { Stream, stream } from './prng.mjs'
import { run } from './oracle.mjs'
import { MIXED_OUTAGE_7, inputStreamHash, panelParams } from './generator.mjs'
import { sha256, canonicalJson } from './canonical.mjs'

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

{
  const prng = new Stream(1234567)
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
    ].every(([seed, n, w]) => stream(seed, n).next() === w),
    'a derived stream gives the first outputs that perf-harness fixes'
  )
}

const fixtures = JSON.parse(fs.readFileSync(new URL('../s1/fixtures.json', import.meta.url), 'utf8'))
for (const [name, f] of Object.entries(fixtures)) {
  const t = run({ configId: `fixture/${name}`, seed: 0, config: f.config, script: f.script })
  const want = f.expect
  const states = Object.fromEntries(t.jobs.map(([, key, state]) => [key, state]))
  const same =
    t.counters.pops === want.pops &&
    t.counters.slices === want.slices &&
    t.slices.length + 1 === want.snapshots &&
    t.counters.stale === want.stale &&
    t.counters.noops === want.noops &&
    JSON.stringify(states) === JSON.stringify(want.states) &&
    (!want.servers || JSON.stringify(t.servers) === JSON.stringify(want.servers)) &&
    (want.arrivals === undefined ||
      (t.counters.arrivals === want.arrivals &&
        t.counters.admitted === want.admitted &&
        t.counters.rejected === want.rejected))
  check(
    same,
    `${name}: ${t.counters.pops} pops, ${t.counters.slices} slices, ${t.slices.length + 1} snapshots, ${t.counters.stale} stale, ${t.counters.noops} no-ops`
  )
}

// The S0 pins of the manifest seed, which the S0 prototype produced. A third reading has to agree with them.
{
  const frozen = JSON.parse(fs.readFileSync(new URL('../s0/prototype/frozen.json', import.meta.url), 'utf8'))
  const t = run({
    configId: 'mixed_outage/7',
    seed: frozen.manifest_seed,
    config: MIXED_OUTAGE_7,
    params: MIXED_OUTAGE_7
  })
  check(
    sha256(canonicalJson(MIXED_OUTAGE_7)) === frozen.parameters_sha256,
    'the parameter hash of mixed_outage/7 is the S0 pin'
  )
  check(
    inputStreamHash(frozen.manifest_seed, MIXED_OUTAGE_7) === frozen.input_stream_sha256,
    `the input stream of seed ${frozen.manifest_seed} is the S0 pin`
  )
  check(t.event_pop_sha256 === frozen.event_pop_sha256, `the event pops of seed ${frozen.manifest_seed} are the S0 pin`)
  const r = frozen.manifest_seed_run
  check(
    t.counters.pops === r.pops &&
      t.counters.slices === r.slices &&
      t.counters.arrivals === r.arrivals &&
      t.counters.stale === r.stale &&
      t.counters.noops === r.noops,
    `and so are its counts: ${t.counters.pops} pops, ${t.counters.slices} slices, ${t.counters.stale} stale, ${t.counters.noops} no-ops`
  )
}

{
  const p = panelParams(5)
  const a = run({ configId: 'panel/1:5', seed: 5, config: p, params: p })
  const b = run({ configId: 'panel/1:5', seed: 5, config: p, params: p })
  check(JSON.stringify(a) === JSON.stringify(b), 'the same seed gives the same trace')
  const zero = run({
    configId: 'zero',
    seed: 0,
    config: { servers: 2, queueCapacity: 4, maxAttempts: 2, deferralCap: 4 },
    params: { ...panelParams(1), arrivals: 0 }
  })
  check(zero.counters.pops === 0 && zero.slices.length === 0, 'a scenario with no arrivals has no pops and no slices')
}

console.log(failures ? `\n${failures} did not behave` : "\nthe oracle gives the design's known answers and the S0 pins")
process.exit(failures ? 1 : 0)
