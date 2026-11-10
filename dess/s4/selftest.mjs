// Proves two things about the service recovery run that a run in which everything is right cannot show: the aggregate gate on externref
// slots can fail, and the recovery times belong to the fault points that caused them.
//
//   node dess/s4/selftest.mjs
//
// Each time sample is counted against the rows of its own point: a point has one sample for each recovery that its failures needed, and the
// repair that follows a failed repair is timed under a name of its own.
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

  // The samples of a point are those of the recoveries after the failures that were classified as that point, and no others.
  // A recovery is timed after a failed draw, which records which base it was made from, and after a failed write of the control, which records
  // the operation. A failure while the surface was built is repaired by a rebuild that is not timed, and a repair that failed is timed apart.
  const timed = point =>
    clean.cases.filter(r => r.point === point && ('double' in r.observed || 'op' in r.observed)).length
  const samples = name => clean.samples[name]?.length ?? 0
  for (const point of ['service_overlay_write', 'service_data_setter', 'service_control_aria']) {
    check(
      timed(point) > 0 && samples(`recovery.${point}.time`) === timed(point),
      `${point} has ${samples(`recovery.${point}.time`)} time samples, one for each of the ${timed(point)} recoveries that its own failures needed`
    )
  }
  const repairs = clean.cases.filter(r => r.observed.repair).length
  check(
    repairs > 0 && samples('recovery.service.full_repair.time') === repairs,
    `the repairs that follow a failed repair are timed under their own name: ${samples('recovery.service.full_repair.time')} of ${repairs}`
  )
  check(
    new Set(['service_overlay_write', 'service_data_setter']).size === 2 &&
      JSON.stringify(clean.samples['recovery.service_overlay_write.time']) !==
        JSON.stringify(clean.samples['recovery.service_data_setter.time']),
    'the overlay and the setter do not share one set of samples'
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
    : '\nthe aggregate slot gate fires when a slot is lost that no row explains, and every recovery time is counted under its own fault point'
)
process.exit(failures ? 1 : 0)
