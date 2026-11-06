// Proves that the endurance gate notices a leak, in a real browser.
//
//   node perf/runner/selftest-endurance.mjs
//
// A gate that has never failed has not shown that it can. This runs the lifecycle workload for 70 cycles with a leak of a known
// size put into every cycle on purpose, and checks three things about each. The gate has to fail. The slope it reports has to be
// the size of the leak, which shows that it is measuring the leak and not something else. And with no leak the gate has to
// pass. It also checks that the allowance in the manifest turns the verdict exactly where it should. It exits with 1 if any of
// that is wrong.
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from './lib/harness-page.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const canonical = spawnSync(
  path.join(root, 'target/release/perf-harness'),
  ['canonical', path.join(root, 'perf/manifests/w3-scene-lifecycle-endurance.toml')],
  { encoding: 'utf8' }
)
if (canonical.status !== 0) throw new Error(`could not read the manifest: ${canonical.stderr}`)

const manifestWith = allowance => {
  const manifest = JSON.parse(canonical.stdout)
  manifest.execution.warmup_iterations = 10
  manifest.execution.measurement_iterations = 60
  manifest.measurement.checkpoint_every = 1000
  if (allowance !== undefined) manifest.limits.live_growth_allowance_bytes = allowance
  return JSON.stringify(manifest)
}

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

const run = async (kind, bytes, allowance) => {
  // A page of its own, so that one case's leak is not in the next one's baseline.
  const page = await openHarnessPage()
  const outcome = JSON.parse(
    await page.evaluate(
      `window.perfHarness.selftest_endurance(${JSON.stringify(manifestWith(allowance))}, ${JSON.stringify(kind)}, ${bytes})`
    )
  )
  await page.close()
  if (outcome.status !== 'ok') throw new Error(`${kind} ${bytes}: ${outcome.status} ${outcome.error ?? ''}`)
  const metric = name => outcome.metrics.find(m => m.name === name && m.status === 'measured')?.value
  const gate = outcome.gates.find(g => g.name === 'endurance.live_bytes_do_not_grow')
  return {
    outcome,
    gate,
    slope: metric('wasm.live_bytes_slope_per_cycle'),
    net: metric('wasm.live_bytes_net_change_after_warmup'),
    grows: metric('wasm.memory_grow_events_after_warmup'),
    oneTime: metric('wasm.live_bytes_one_time_after_first_cycle'),
    points: metric('lifecycle.measured_cycles_sampled')
  }
}

const none = await run('none', 0)
check(
  none.gate?.status === 'pass' && none.slope === 0 && none.net === 0,
  `no leak: the gate ${none.gate?.status}, slope ${none.slope}, net change ${none.net} (${none.gate?.detail})`
)
check(
  Number.isFinite(none.oneTime),
  `no leak: the cost of the first cycle is reported apart from the interval (${none.oneTime} bytes)`
)

// Each leak, the slope that it has to give, and what else it has to do.
const cases = [
  { kind: 'bytes', bytes: 64, exactly: 64 },
  { kind: 'bytes', bytes: 1024, exactly: 1024 },
  { kind: 'closure', bytes: 256, atLeast: 256, within: 300 },
  { kind: 'library_watch', bytes: 0, atLeast: 1, within: 2000 },
  { kind: 'bytes', bytes: 65536, exactly: 65536, growsCapacity: true }
]
const results = new Map()
for (const { kind, bytes, exactly, atLeast, within, growsCapacity } of cases) {
  const result = await run(kind, bytes)
  results.set(`${kind}/${bytes}`, result)
  const label = `${kind}${bytes ? ` ${bytes} bytes a cycle` : ''}`
  check(result.gate?.status === 'fail', `${label}: the gate fails (${result.gate?.detail})`)
  if (exactly !== undefined)
    check(result.slope === exactly, `${label}: the slope is ${result.slope} bytes a cycle, and the leak is ${exactly}`)
  else
    check(
      result.slope >= atLeast && result.slope <= atLeast + within,
      `${label}: the slope is ${result.slope?.toFixed(1)} bytes a cycle, in ${atLeast} to ${atLeast + within}`
    )
  if (growsCapacity) check(result.grows > 0, `${label}: it also grows the linear memory, ${result.grows} time(s)`)
}

// The allowance is the growth tolerated over the interval, and the verdict turns exactly on it.
const leaking = results.get('bytes/64')
const net = leaking.net
const atNet = await run('bytes', 64, net)
const belowNet = await run('bytes', 64, net - 1)
check(
  atNet.gate?.status === 'pass',
  `an allowance of ${net} bytes, the growth that was seen, passes (${atNet.gate?.detail})`
)
check(
  belowNet.gate?.status === 'fail',
  `an allowance of ${net - 1} bytes, one byte less, fails (${belowNet.gate?.detail})`
)

console.log(
  failures
    ? `\n${failures} did not behave: the endurance gate cannot be trusted`
    : '\nthe gate fails every leak, sizes each one correctly, passes a clean run, and turns exactly on its allowance'
)
process.exit(failures ? 1 : 0)
