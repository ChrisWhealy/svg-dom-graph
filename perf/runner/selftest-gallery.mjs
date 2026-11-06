// Proves that the gallery sweep holds a walk to its expectations, in a real browser.
//
//   node perf/runner/selftest-gallery.mjs
//
// A verdict that cannot come out as a failure would be a diagnostic with a green label. This sweeps three panels with
// expectations that are changed on purpose:
//
//   1. Dijkstra, whose source says that Restart draws new random weights, is declared to restart to its start. That is a defined
//      expectation, it is false, and the walk has to fail and name it.
//   2. The same panel, declared as its source says, is inconclusive for that check and does not fail.
//   3. A selection panel that nothing is declared for. Its Next and Previous changed the panel, and that passes. Whether Previous
//      retraces Next is undefined, so that is inconclusive, and what was seen is written down as an observation and not as a pass.
//
// It exits with 1 if any does not.
import fs from 'node:fs'
import path from 'node:path'
import { sweepGallery } from './layer-c/gallery.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const stageDir = path.join(root, 'target/demo-stage')
const declared = JSON.parse(fs.readFileSync(path.join(root, 'perf/gallery-overrides.json'), 'utf8')).panels

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}
const sweep = (only, overrides) =>
  sweepGallery({ stageDir, overrides, only: [only] }).then(report => report.panels[0].walk)
const outcome = (walk, name) => walk.checks.find(c => c.name === name)

const wrong = await sweep('panel-dijkstra', {
  'panel-dijkstra': { expect: { restart_to_start: true, because: 'declared wrongly, on purpose' } }
})
check(
  wrong.verdict === 'fail' && outcome(wrong, 'restart_returns_to_start').outcome === 'fail',
  `a restart declared to return to the start, where the source says it does not: ${wrong.verdict}`
)
console.log(`      ${outcome(wrong, 'restart_returns_to_start').detail.slice(0, 200)}`)

const right = await sweep('panel-dijkstra', { 'panel-dijkstra': declared['panel-dijkstra'] })
check(
  right.verdict === 'pass' && outcome(right, 'restart_returns_to_start').outcome === 'inconclusive',
  `declared as its source says: ${right.verdict}, and the restart check is ${outcome(right, 'restart_returns_to_start').outcome}`
)

const undefinedPanel = await sweep('panel-selection-1d', {})
const retrace = outcome(undefinedPanel, 'back_retraces_forward')
check(
  undefinedPanel.verdict === 'pass' &&
    retrace.outcome === 'inconclusive' &&
    /observation and not a/.test(retrace.detail),
  `nothing declared: Next and Previous are ${undefinedPanel.verdict}, the retrace is ${retrace.outcome} and says it is an observation`
)
console.log(`      ${retrace.detail.slice(0, 200)}`)

const held = await sweep('panel-selection-1d', { 'panel-selection-1d': declared['panel-selection-1d'] })
check(
  held.verdict === 'pass' && outcome(held, 'back_retraces_forward').outcome === 'pass',
  `declared reversible, as its source says: ${outcome(held, 'back_retraces_forward').detail.slice(0, 110)}`
)

console.log(
  failures
    ? `\n${failures} did not behave: the gallery verdict cannot be trusted`
    : '\na broken declared expectation fails, and what nothing defines is never a pass'
)
process.exit(failures ? 1 : 0)
