// Proves that the harness's own work is not inside any timed interval, in a real browser.
//
//   node perf/runner/selftest-timing.mjs
//
// The runner updates a reference model and hashes the stream for every operation, and the specification says that none of that
// may be in a Layer B timing. This runs a workload twice. The second run delays the harness's own work by 100 to 200
// microseconds for every operation, which is far longer than the operations themselves. If any timed figure moves by more than
// a modest allowance, the delay is leaking into it, and the run fails. It exits with 1 if it does.
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from './lib/harness-page.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const binary = path.join(root, 'target/release/perf-harness')
const canonical = name => {
  const result = spawnSync(binary, ['canonical', path.join(root, 'perf/manifests', `${name}.toml`)], { encoding: 'utf8' })
  if (result.status !== 0) throw new Error(`could not read ${name}: ${result.stderr}`)
  return result.stdout.trim()
}

// Each case names a manifest, the delay, the metric to watch, and how much it may move. The allowance is in proportion to the
// figure, with a floor for the timer, and is a small fraction of what the delay would add if it leaked in.
const cases = [
  { manifest: 'w2-connector-rerouting-large', delayUs: 200, metric: 'mutation.move_node', leakWouldAdd: '200 us a move' },
  { manifest: 'w1-scene-construction-medium', delayUs: 100, metric: 'construction.nodes', leakWouldAdd: '100 us a node, 20 ms in all' },
  { manifest: 'w1-scene-construction-medium', delayUs: 100, metric: 'construction.edges', leakWouldAdd: '100 us an edge, 60 ms in all' },
]

const page = await openHarnessPage()
let failures = 0
const median = (outcome, name) => outcome.metrics.find(metric => metric.name === name && metric.status === 'measured')?.summary.p50

const mode = 'performance'
const results = new Map()
for (const { manifest, delayUs } of cases) {
  for (const delay of [0, delayUs]) {
    const key = `${manifest}/${delay}`
    if (results.has(key)) continue
    const outcome = JSON.parse(await page.evaluate(`window.perfHarness.selftest_timing(${JSON.stringify(canonical(manifest))}, ${JSON.stringify(mode)}, ${delay})`))
    if (outcome.status !== 'ok') throw new Error(`${key}: ${outcome.status} ${outcome.error ?? ''}`)
    results.set(key, outcome)
  }
}

for (const { manifest, delayUs, metric, leakWouldAdd } of cases) {
  const plain = median(results.get(`${manifest}/0`), metric)
  const delayed = median(results.get(`${manifest}/${delayUs}`), metric)
  // Run to run the figures differ by several percent, so the allowance is half as much again, and never less than 5 us.
  const allowed = Math.max(plain * 1.5, plain + 5)
  const ok = delayed <= allowed
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${manifest} ${metric}: ${plain.toFixed(1)} us, and ${delayed.toFixed(1)} us with ${delayUs} us of delay in the harness (at most ${allowed.toFixed(1)}; a leak would add ${leakWouldAdd})`)
}
await page.close()
console.log(failures ? `\n${failures} timing(s) include the harness's own work` : "\nno timed figure moved: the harness's own work is outside every timed interval")
process.exit(failures ? 1 : 0)
