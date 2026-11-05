// Proves that the checkpoint comparison can fail, in a real browser.
//
//   node perf/runner/selftest.mjs [manifest.toml]
//
// A run in which every checkpoint passes shows nothing unless the check is known to notice a fault. This runs the same
// workload once with no fault, which has to pass, and then once for each deliberate fault in what is observed. Each of those
// has to come out invalid and has to name the section that the fault damaged. It exits with 1 if any does not.
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from './lib/harness-page.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const manifest = path.resolve(process.argv[2] ?? path.join(root, 'perf/manifests/w2-connector-rerouting-small.toml'))
const binary = path.join(root, 'target/release/perf-harness')
const canonical = spawnSync(binary, ['canonical', manifest], { encoding: 'utf8' })
if (canonical.status !== 0) throw new Error(`could not read the manifest: ${canonical.stderr}`)

// Each fault, and the section that a check which works must say it damaged.
const faults = [
  ['connector_offset', 'geometry'],
  ['node_moved', 'geometry'],
  ['node_removed', 'topology'],
  ['text_changed', 'content'],
  ['paint_order_swapped', 'paint_order'],
  ['edge_dropped', 'topology'],
]

const page = await openHarnessPage()
let failures = 0
const report = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}
const run = async fault => JSON.parse(await page.evaluate(`window.perfHarness.selftest_checker(${JSON.stringify(canonical.stdout.trim())}, ${JSON.stringify(fault)})`))

const clean = await run('none')
report(clean.status === 'ok' && clean.checkpoints_failed.length === 0, `no fault: the run is valid, with ${clean.checkpoints_passed} checkpoints passed`)

for (const [fault, section] of faults) {
  const outcome = await run(fault)
  const named = outcome.checkpoints_failed.some(failure => failure.section === section)
  report(outcome.status === 'invalid_incorrect' && named, `${fault}: ${outcome.status}, and ${named ? `names ${section}` : `does not name ${section} (it names ${[...new Set(outcome.checkpoints_failed.map(f => f.section))].join(', ') || 'nothing'})`}`)
  if (outcome.checkpoints_failed[0]) console.log(`      ${outcome.checkpoints_failed[0].detail}`)
}
await page.close()
console.log(failures ? `\n${failures} did not behave: the check cannot be trusted` : '\nthe check notices every fault, and passes a clean run')
process.exit(failures ? 1 : 0)
