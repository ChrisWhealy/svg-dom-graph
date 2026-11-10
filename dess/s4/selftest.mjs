// Proves that the aggregate gate on externref slots can fail, which a run in which every slot is explained cannot show.
//
//   node dess/s4/selftest.mjs
//
// The service recovery run is made as it is, which has to pass, and then with one failed call of the harness's own that no row records.
// That call loses one slot that nothing explains, so the whole run has lost more slots than the rows measured, and the gate that compares
// them has to fail while every contract of every fault point still passes. It exits with 1 if it does not.
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from '../../perf/runner/lib/harness-page.mjs'
import { oracleFor } from '../../perf/runner/service-oracle.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const manifestPath = path.join(root, 'perf/manifests/w5-service-recovery-small.toml')
const canonical = spawnSync(path.join(root, 'target/release/perf-harness'), ['canonical', manifestPath], {
  encoding: 'utf8'
}).stdout.trim()
const oracle = oracleFor(JSON.parse(canonical).recovery.scenario)
const GATE = 'recovery.service.every_row_loses_exactly_the_slots_its_failure_explains'

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

const page = await openHarnessPage({ faults: true })
const run = async testing =>
  JSON.parse(
    await page.evaluate(
      `window.perfHarness.service_recovery_run(${JSON.stringify(canonical)}, ${JSON.stringify(JSON.stringify({ oracle, ...testing }))})`
    )
  )
try {
  const clean = await run({})
  const gate = clean.gates.find(g => g.name === GATE)
  check(
    gate?.status === 'pass' && clean.gates.every(g => g.status === 'pass'),
    `as it is, every gate passes: ${gate?.detail}`
  )

  const leaky = await run({ unexplainedSlotLoss: true })
  const slot = leaky.gates.find(g => g.name === GATE)
  const others = leaky.gates.filter(g => g.name !== GATE && g.status !== 'pass').map(g => g.name)
  check(slot?.status === 'fail', `with one slot lost outside every row, the gate fails: ${slot?.detail}`)
  check(
    others.length === 0 && leaky.checkpoints_failed.length === 0,
    `and nothing else fails: ${others.join(', ') || 'no other gate, no failed checkpoint'}`
  )
} finally {
  await page.close()
}
console.log(
  failures
    ? `\n${failures} failed`
    : '\nthe aggregate slot gate fires when a slot is lost that no row explains, and only then'
)
process.exit(failures ? 1 : 0)
