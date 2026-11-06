// Proves that Layer C measures what it says it measures, in a real browser.
//
//   node perf/runner/selftest-layer-c.mjs
//
// A figure from a run is worth something only if it moves by what it should when the thing it measures is changed, and does not
// move when something else is. This runs a small interaction workload five times:
//
//   1. as it is, which has to be valid, with every gesture doing what the plan said;
//   2. with a listener that costs a known time on every pointer move, after the library's own. The interaction time has to go
//      up by about that time, and the run has to stay valid, since slow input is still correct input;
//   3. with the browser's input stopped before the library sees it. The gestures do nothing, so the run has to be invalid,
//      since a time for a gesture that did nothing is not a measurement;
//   4. with every CDP command made to take 30 ms longer to answer. The CDP round trip has to go up by that, and the interaction
//      time must not move, since the start of an interaction is the page's own stamp and never CDP's clock;
//   5. with a wheel in a tab of its own, which the first run already includes, and has to have changed the view.
//
// It exits with 1 if any does not.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { runInteraction } from './layer-c/interaction.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const binary = path.join(root, 'target/release/perf-harness')
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-selftest-c-'))
const manifest = path.join(dir, 'selftest.toml')
fs.writeFileSync(
  manifest,
  fs.readFileSync(path.join(root, 'perf/manifests/w4-interaction-small.toml'), 'utf8')
    .replace('scales_percent = [50, 100, 200]', 'scales_percent = [100]')
    .replace('warmup_iterations = 3', 'warmup_iterations = 1')
    .replace('measurement_iterations = 12', 'measurement_iterations = 6')
    .replace('steps = 8', 'steps = 5'),
)
const sh = (...args) => {
  const result = spawnSync(binary, args, { encoding: 'utf8', maxBuffer: 1 << 28 })
  if (result.status !== 0) throw new Error(result.stderr)
  return result.stdout
}
const plan = JSON.parse(sh('gestures', manifest))
const report = JSON.parse(sh('generate', manifest))
const canonical = sh('canonical', manifest).trim()
const play = faults => runInteraction({ binary, mode: 'performance', canonical, plan, report, faults })

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}
const median = (outcome, name) => outcome.metrics.find(metric => metric.name === name && metric.status === 'measured')?.summary.p50

const clean = await play({})
check(clean.status === 'ok' && clean.checkpoints_failed.length === 0, `as it is: ${clean.status}, ${clean.checkpoints_passed} checks passed, every gesture did what the plan said`)
const view = clean.metrics.find(metric => metric.name === 'interaction.wheel_zoom.scale_100.latency')
check(view?.status === 'measured' && view.samples === 6, 'the wheel, in a tab of its own, was placed in a frame every time')
const base = median(clean, 'interaction.drag.scale_100.latency')
const baseTrip = median(clean, 'cdp.drag.scale_100.round_trip')
console.log(`      drag ${base.toFixed(2)} ms, CDP round trip ${baseTrip.toFixed(2)} ms`)

const slow = await play({ slowdownMs: 20 })
const slowed = median(slow, 'interaction.drag.scale_100.latency')
// The listener costs 20 ms after the library has written, so the frame cannot be presented before it returns, whatever the
// move cost before it. A frame that is missed adds up to one more, at 60 Hz.
check(slow.status === 'ok' && slowed >= 20 && slowed <= 20 + 17 + 4, `a 20 ms listener after every move: the drag time goes from ${base.toFixed(2)} to ${slowed.toFixed(2)} ms, and the run stays ${slow.status}`)

const blocked = await play({ blockInput: true })
const named = blocked.checkpoints_failed.some(failure => failure.section === 'interaction')
check(blocked.status === 'invalid_incorrect' && named, `input that never reaches the library: ${blocked.status}, and ${named ? 'names interaction' : 'does not name interaction'}`)
if (blocked.checkpoints_failed[0]) console.log(`      ${blocked.checkpoints_failed[0].detail}`)

const laggy = await play({ cdpExtraMs: 30 })
const trip = median(laggy, 'cdp.drag.scale_100.round_trip')
const lagged = median(laggy, 'interaction.drag.scale_100.latency')
check(trip - baseTrip >= 25, `CDP made 30 ms slower: the round trip goes from ${baseTrip.toFixed(2)} to ${trip.toFixed(2)} ms`)
check(Math.abs(lagged - base) <= 3, `and the interaction time does not move, ${base.toFixed(2)} to ${lagged.toFixed(2)} ms, since CDP's clock is never its start`)

fs.rmSync(dir, { recursive: true, force: true })
console.log(failures ? `\n${failures} did not behave: a Layer C figure cannot be trusted` : '\nLayer C moves by what it measures, and by nothing else')
process.exit(failures ? 1 : 0)
