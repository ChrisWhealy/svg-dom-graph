// The differential run: the Rust engine and the oracle, scenario by scenario, on every pop, every slice and every hash.
//
//   node dess/s1/differential.mjs [--seeds 600] [--wasi] [--rust PATH]
//
// For each scenario both engines produce a trace (design: `dess/s1/docs/contract.md`, section 6), and the traces have to be equal in
// every field. The scenarios are
//
//   * the five known-answer fixtures of `dess/s1/fixtures.json`, which also have to give the design's counts;
//   * the panel: the seeds 0 to N - 1, each deriving its own configuration, so that the two engines also agree on the derivation;
//   * the disturbed panel: the same seeds with a script added, which reaches the rules that generated outages and cancels cannot;
//   * the frozen revision `mixed_outage/7` for the manifest seed, whose parameter, input and event-pop hashes are the S0 pins.
//
// `--wasi` runs the Rust engine as a 32-bit wasm32-wasip1 binary under Node's WASI, and so compares the oracle with the 32-bit build.
// The first difference of a scenario is printed with its place, and the run exits with 1 if there is any. A difference is a
// discrepancy, to be adjudicated against the design's text and recorded in `dess/s1/ledger.md`.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { run } from '../reference-js/oracle.mjs'
import { MIXED_OUTAGE_7, disturbances, panelParams } from '../reference-js/generator.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const SEEDS = Number(argument('--seeds') ?? 600)
const WASI = process.argv.includes('--wasi')
const rust =
  argument('--rust') ??
  path.join(root, WASI ? 'target/wasm32-wasip1/release/dess-trace.wasm' : 'target/release/dess-trace')
const fixturesFile = path.join(root, 'dess/s1/fixtures.json')

const rustLines = (...args) => {
  const command = WASI ? ['node', [path.join(root, 'perf/crosscheck/run-wasi.mjs'), rust, ...args]] : [rust, args]
  // The fixtures are given on standard input, since the WASI runner preopens no directory.
  const input = args[0] === 'fixtures' ? fs.readFileSync(fixturesFile, 'utf8') : undefined
  const result = spawnSync(command[0], command[1], { cwd: root, encoding: 'utf8', maxBuffer: 1 << 30, input })
  if (result.status !== 0)
    throw new Error(`the Rust engine failed (${args.join(' ')}): ${result.stderr || result.stdout}`)
  return result.stdout
    .split('\n')
    .filter(Boolean)
    .map(line => JSON.parse(line))
}

/** The first difference between two traces, with where it is, or null if they are equal. */
// JSON text with every object's keys sorted, so that two traces are the same whatever order an engine wrote an object's keys in.
const canonical = v =>
  Array.isArray(v)
    ? `[${v.map(canonical).join(',')}]`
    : v !== null && typeof v === 'object'
      ? `{${Object.keys(v)
          .sort()
          .map(k => `${JSON.stringify(k)}:${canonical(v[k])}`)
          .join(',')}}`
      : JSON.stringify(v)

const firstDifference = (a, b) => {
  const same = (x, y) => canonical(x) === canonical(y)
  for (const key of ['config_id', 'params_sha256', 'input_stream_sha256', 'event_pop_sha256', 'initial_checkpoint'])
    if (!same(a[key], b[key])) return `${key}: rust ${JSON.stringify(a[key])}, oracle ${JSON.stringify(b[key])}`
  const n = Math.min(a.pops.length, b.pops.length)
  for (let i = 0; i < n; i++)
    if (!same(a.pops[i], b.pops[i]))
      return `pop ${i} [tick, kind, sequence, outcome, subject]: rust ${JSON.stringify(a.pops[i])}, oracle ${JSON.stringify(b.pops[i])}`
  if (a.pops.length !== b.pops.length) return `the number of pops: rust ${a.pops.length}, oracle ${b.pops.length}`
  for (let i = 0; i < Math.min(a.slices.length, b.slices.length); i++)
    for (const key of Object.keys(a.slices[i]))
      if (!same(a.slices[i][key], b.slices[i][key]))
        return `slice ${i + 1}, ${key}: rust ${JSON.stringify(a.slices[i][key])}, oracle ${JSON.stringify(b.slices[i][key])}`
  if (a.slices.length !== b.slices.length)
    return `the number of slices: rust ${a.slices.length}, oracle ${b.slices.length}`
  for (const key of ['counters', 'jobs', 'servers'])
    if (!same(a[key], b[key]))
      return `${key}: rust ${JSON.stringify(a[key]).slice(0, 200)}, oracle ${JSON.stringify(b[key]).slice(0, 200)}`
  return null
}

let scenarios = 0
let slicesCompared = 0
let popsCompared = 0
const differences = []
const compare = (label, a, b) => {
  scenarios++
  slicesCompared += b.slices.length
  popsCompared += b.pops.length
  const d = firstDifference(a, b)
  if (d) differences.push(`${label}: ${d}`)
}

// The fixtures, which also have to give the design's counts.
const fixtures = JSON.parse(fs.readFileSync(fixturesFile, 'utf8'))
for (const { name, trace } of rustLines('fixtures', '-')) {
  const f = fixtures[name]
  const oracle = run({ configId: `fixture/${name}`, seed: 0, config: f.config, script: f.script })
  compare(`fixture ${name}`, trace, oracle)
  const want = f.expect
  const c = oracle.counters
  const states = Object.fromEntries(oracle.jobs.map(([, key, state]) => [key, state]))
  if (
    c.pops !== want.pops ||
    c.slices !== want.slices ||
    c.stale !== want.stale ||
    c.noops !== want.noops ||
    JSON.stringify(states) !== JSON.stringify(want.states)
  )
    differences.push(
      `fixture ${name}: the design's counts are ${JSON.stringify(want)}, and the engines gave ${c.pops} pops, ${c.slices} slices, ${c.stale} stale, ${c.noops} no-ops`
    )
}

// The panel.
const panel = rustLines('panel', '0', String(SEEDS))
for (const trace of panel) {
  const p = panelParams(trace.seed)
  compare(
    `panel seed ${trace.seed}`,
    trace,
    run({ configId: `panel/1:${trace.seed}`, seed: trace.seed, config: p, params: p })
  )
}

// The disturbed panel: the same scenarios with a script of scripted arrivals, duplicate outages and cancels of random keys.
for (const trace of rustLines('panel2', '0', String(SEEDS))) {
  const p = { ...panelParams(trace.seed), revision: 'panel/2' }
  compare(
    `disturbed panel seed ${trace.seed}`,
    trace,
    run({
      configId: `panel/2:${trace.seed}`,
      seed: trace.seed,
      config: p,
      params: p,
      script: disturbances(trace.seed, p)
    })
  )
}

// The frozen revision, against the S0 pins as well.
{
  const [trace] = rustLines('manifest')
  const seed = trace.seed
  compare(
    'mixed_outage/7',
    trace,
    run({ configId: 'mixed_outage/7', seed, config: MIXED_OUTAGE_7, params: MIXED_OUTAGE_7 })
  )
  const frozen = JSON.parse(fs.readFileSync(path.join(root, 'dess/s0/prototype/frozen.json'), 'utf8'))
  for (const [key, pin] of [
    ['params_sha256', 'parameters_sha256'],
    ['input_stream_sha256', 'input_stream_sha256'],
    ['event_pop_sha256', 'event_pop_sha256']
  ])
    if (trace[key] !== frozen[pin]) differences.push(`mixed_outage/7: ${key} is not the S0 pin ${pin}`)
}

console.log(
  `${WASI ? 'wasm32-wasip1' : 'native'} Rust against the oracle: ${scenarios} scenarios, ${popsCompared} pops and ${slicesCompared} slices compared`
)
if (differences.length === 0) console.log('every scenario agrees on every pop, every slice checkpoint and every hash')
else {
  console.log(`${differences.length} scenario(s) differ. The first difference of each:`)
  for (const d of differences.slice(0, 20)) console.log(`  - ${d}`)
}
process.exit(differences.length ? 1 : 0)
