// Proves that the recovery runner can fail, and that it breaks what it says it breaks, in a real browser.
//
//   node perf/runner/selftest-recovery.mjs
//
// A recovery run in which every contract holds shows nothing unless the runner is known to notice a contract that does not. This
// runs the recovery workload as it is, which has to pass, and then with things made wrong on purpose:
//
//   1. The injector. Arming the nth DOM call makes exactly that call fail, once, and a call beyond the last makes nothing fail.
//      Without that, a sweep could be checking calls that never failed.
//   2. A wrong expectation. Each documented point is given the other outcome, and the sweep has to fail it. The undocumented
//      point is given a rollback that nothing promises, and the sweep has to fail that too, which is why it is a finding.
//   3. A library with no rollback. Removing elements is made to do nothing, and the construction sweep has to find what is left.
//   4. A frame that is not cancelled. The scene is dropped with a frame pending and the cancel does nothing, and the Layer C
//      scenario has to see the frame run after its closure was dropped.
//   5. Endurance. A leak of bytes, of a listener and of an element in every cycle has to fail its own gate and no other.
//
// It exits with 1 if any does not.
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from './lib/harness-page.mjs'
import { runRecoveryC } from './layer-c/recovery.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const binary = path.join(root, 'target/release/perf-harness')
const manifestPath = path.join(root, 'perf/manifests/w5-recovery-small.toml')
const sh = (...args) => {
  const result = spawnSync(binary, args, { encoding: 'utf8', maxBuffer: 1 << 28 })
  if (result.status !== 0) throw new Error(result.stderr)
  return result.stdout
}
const manifest = JSON.parse(sh('canonical', manifestPath))
const report = JSON.parse(sh('generate', manifestPath))

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}
const only = (points, change = {}) => ({ ...manifest, ...change, recovery: { points } })
const point = (id, expect) => ({ point: id, expect })
const gate = (outcome, name) => outcome.gates.find(g => g.name === name)

const page = await openHarnessPage({ faults: true })
const run = (m, testing = {}) =>
  page
    .evaluate(
      `window.perfHarness.recovery_run(${JSON.stringify(JSON.stringify(m))}, ${JSON.stringify(JSON.stringify(testing))})`
    )
    .then(JSON.parse)

// 1. The injector.
{
  const arm = await page
    .evaluate(
      `(() => {
    const h = window.perfHarness, f = window.__faults
    h.recovery_build(${JSON.stringify(JSON.stringify(manifest))}, 0, false)
    const op = JSON.stringify({ op: 'add_node', key: 'probe', x: 40, y: 600, w: 80, h: 36 })
    f.record(true); h.recovery_act(0, op); f.record(false)
    const calls = f.take()
    h.recovery_build(${JSON.stringify(JSON.stringify(manifest))}, 0, false)
    const results = []
    for (const index of [1, 7, calls.length, calls.length + 1]) {
      f.arm(index)
      const result = JSON.parse(h.recovery_act(0, op))
      const fired = f.disarm()
      results.push({ index, ok: result.ok, fired: fired.fired !== null, counted: fired.counted })
      h.recovery_build(${JSON.stringify(JSON.stringify(manifest))}, 0, false)
    }
    f.arm(2); const again = JSON.parse(h.recovery_act(0, op)); f.disarm()
    const afterwards = JSON.parse(h.recovery_act(0, op))
    return JSON.stringify({ calls: calls.length, results, afterwards: afterwards.ok })
  })()`
    )
    .then(JSON.parse)
  const [a, b, c, d] = arm.results
  check(
    a.fired && !a.ok && b.fired && !b.ok && c.fired && !c.ok,
    `arming the 1st, 7th and last of the ${arm.calls} DOM calls of add_node makes the operation fail`
  )
  check(
    !d.fired && d.ok && d.counted === arm.calls,
    `arming a call beyond the last fires nothing, and the operation counted ${d.counted} calls`
  )
  check(arm.afterwards, 'a fault fires once, and the next operation works again')
}

// The scene as it is, with every contract held.
const clean = await run(manifest)
check(
  clean.status === 'ok' && clean.gates.every(g => g.status === 'pass'),
  `as it is: ${clean.status}, ${clean.gates.length} gates, ${clean.checkpoints_passed} failures recovered from`
)
// Every point that a manifest leaves undocumented, and the glue's behaviour, are findings. A documented one is a gate and is not among them.
const undocumented = [
  ...manifest.recovery.points.filter(p => p.expect === 'undocumented').map(p => p.point),
  'externref_slot_per_failed_call'
]
check(
  clean.findings.length === undocumented.length &&
    clean.findings.every(f => f.contract === 'undocumented') &&
    undocumented.every(point => clean.findings.some(f => f.point === point)),
  `the undocumented points and the glue's behaviour are findings (${clean.findings.map(f => f.point).join(', ')}) and not gates`
)
const slotFinding = clean.findings.find(f => f.point === 'externref_slot_per_failed_call')
check(
  slotFinding?.details.exactly_one_slot_per_failed_object_call === true &&
    slotFinding.details.losing.length > 0 &&
    slotFinding.details.table.some(row => !row.returns_an_object && row.slots_lost === 0),
  `a failed DOM call loses exactly one externref slot if it returns an object (${slotFinding?.details.losing.join(', ')}) and none if it does not`
)

// 2. A wrong expectation, for each documented point.
for (const [id, wrong] of [
  ['node_construction', 'partly_committed'],
  ['dom_transform', 'partly_committed'],
  ['connector_update', 'nothing_committed'],
  ['label_update', 'nothing_committed'],
  ['drag_install', 'partly_committed'],
  ['edge_construction', 'partly_committed']
]) {
  const outcome = await run(only([point(id, wrong)]))
  const verdict = gate(outcome, `recovery.${id}.matches_the_contract`)
  check(
    verdict?.status === 'fail',
    `${id} expected to be ${wrong.replace('_', ' ')}, against its contract: ${verdict?.status}`
  )
}
{
  // A point that is left undocumented is not judged. It is reported as a finding, which is what edge_construction was until the
  // library made adding an edge a transaction. Made undocumented again, it must produce a finding and no gate.
  const outcome = await run({ ...manifest, recovery: { points: [point('edge_construction', 'undocumented')] } })
  const finding = outcome.findings.find(f => f.point === 'edge_construction')
  check(
    finding !== undefined && gate(outcome, 'recovery.edge_construction.matches_the_contract') === undefined,
    `an undocumented edge_construction is a finding and not a gate: ${finding?.summary.slice(0, 110)}`
  )
}

// 3. A library with no rollback.
{
  const outcome = await run(only([point('node_construction', 'nothing_committed')]), { suppressRemoval: true })
  const verdict = gate(outcome, 'recovery.node_construction.matches_the_contract')
  check(
    verdict?.status === 'fail' && outcome.status === 'invalid_incorrect',
    `with removal made to do nothing, node_construction fails: ${verdict?.detail.slice(0, 130)}`
  )
  // The same for adding an edge, whose rollback is the removal of the connector's path. A library with none leaves the stray
  // path that the sweep found in it before it was made a transaction.
  const edge = await run(only([point('edge_construction', 'nothing_committed')]), { suppressRemoval: true })
  const edgeVerdict = gate(edge, 'recovery.edge_construction.matches_the_contract')
  check(
    edgeVerdict?.status === 'fail',
    `and edge_construction, whose rollback is the same removal: ${edgeVerdict?.detail.slice(0, 130)}`
  )
}

// 4. The points that were added after the first four are each shown able to fail, in the same two ways. Their rollbacks are removals as well,
// so a library with none leaves the label, the node or the operator behind, and each sweep has to see it. And each is held to what it
// says: given the other outcome, it fails.
for (const id of ['edge_label_draw', 'data_node_construction', 'operator_construction', 'container_construction']) {
  const outcome = await run(only([point(id, 'nothing_committed')]), { suppressRemoval: true })
  const verdict = gate(outcome, `recovery.${id}.matches_the_contract`)
  check(verdict?.status === 'fail', `with removal made to do nothing, ${id} fails: ${verdict?.detail.slice(0, 120)}`)
}
for (const [id, wrong] of [
  ['edge_label_draw', 'partly_committed'],
  ['edge_label_placement', 'nothing_committed'],
  ['connector_type_redraw', 'partly_committed'],
  ['connector_type_label', 'nothing_committed'],
  ['data_node_construction', 'partly_committed'],
  ['data_values_update', 'nothing_committed'],
  ['operator_construction', 'partly_committed'],
  ['container_construction', 'partly_committed']
]) {
  const verdict = gate(await run(only([point(id, wrong)])), `recovery.${id}.matches_the_contract`)
  check(
    verdict?.status === 'fail',
    `${id} expected to be ${wrong.replace('_', ' ')}, against its contract: ${verdict?.status}`
  )
}
{
  // The failures of a rollback are found, and told apart: a second failure at a call that the rollback makes is injected, and what it
  // leaves is a finding. Adding an edge has a rollback that writes names, so it has some, and adding a node has none.
  const outcome = await run(
    only([point('rollback_edge_construction', 'undocumented'), point('rollback_node_construction', 'undocumented')])
  )
  const edge = outcome.findings.find(f => f.point === 'rollback_edge_construction')
  const node = outcome.findings.find(f => f.point === 'rollback_node_construction')
  check(
    edge?.details.observed.length > 0 && /inside the rollback/.test(edge.details.observed[0].what_was_left),
    `a second failure is put inside the rollback of adding an edge: ${edge?.summary.slice(0, 100)}`
  )
  check(
    node?.details.observed.length === 0,
    `and adding a node has no rollback call to fail: ${node?.summary.slice(0, 90)}`
  )
}

// 5. Endurance: a clean run, and a leak of each kind. Each is in a page of its own, since a run that leaves something in the
// document would move the baseline of the next.
await page.close()
const short = {
  ...manifest,
  execution: { ...manifest.execution, warmup_iterations: 3, measurement_iterations: 9 },
  recovery: {
    points: manifest.recovery.points.filter(
      p => !['scheduled_frame', 'teardown', 'mid_drag_teardown'].includes(p.point)
    )
  }
}
const fresh = async work => {
  const own = await openHarnessPage({ faults: true })
  try {
    return await work(testing =>
      own
        .evaluate(
          `window.perfHarness.recovery_endurance(${JSON.stringify(JSON.stringify(short))}, ${JSON.stringify(JSON.stringify(testing))})`
        )
        .then(JSON.parse)
    )
  } finally {
    await own.close()
  }
}
const steady = await fresh(endureIn => endureIn({}))
check(
  steady.status === 'ok' && steady.gates.every(g => g.status === 'pass'),
  `endurance as it is: ${steady.status}, ${steady.gates.map(g => g.name.replace('endurance.', '')).join(', ')} all pass`
)
for (const [testing, failing, others] of [
  [
    { leakBytes: 256 },
    'endurance.live_bytes_do_not_grow',
    ['endurance.listeners_return_to_baseline', 'endurance.dom_returns_to_baseline']
  ],
  [
    { leakListener: true },
    'endurance.listeners_return_to_baseline',
    ['endurance.live_bytes_do_not_grow', 'endurance.dom_returns_to_baseline']
  ],
  [
    { leakElement: true },
    'endurance.dom_returns_to_baseline',
    ['endurance.live_bytes_do_not_grow', 'endurance.listeners_return_to_baseline']
  ],
  [
    { unexplainedSlotLoss: true },
    'endurance.lost_externref_slots_are_explained',
    ['endurance.listeners_return_to_baseline', 'endurance.dom_returns_to_baseline']
  ]
]) {
  const outcome = await fresh(endureIn => endureIn(testing))
  const bad = outcome.gates.filter(g => g.status !== 'pass').map(g => g.name)
  check(
    bad.includes(failing) && others.every(name => gate(outcome, name)?.status === 'pass'),
    `a ${Object.keys(testing)[0].replace('leak', 'leak of ').replace('unexplainedSlotLoss', 'slot lost that no failure explains,').toLowerCase()} in every cycle fails ${failing.replace('endurance.', '')} and only that: failing ${bad.map(n => n.replace('endurance.', '')).join(', ')}`
  )
}

// 4. A frame that is not cancelled, in Layer C.
{
  const outcome = await runRecoveryC({
    binary,
    canonical: sh('canonical', manifestPath).trim(),
    manifest: only([point('teardown', 'nothing_committed')]),
    report,
    repetitions: 2,
    faults: { suppressCancel: true }
  })
  const verdict = gate(outcome, 'recovery.teardown.matches_the_contract')
  check(
    verdict?.status === 'fail',
    `with the cancel of a pending frame made to do nothing, teardown fails: ${verdict?.detail.slice(0, 150)}`
  )
  const clean = await runRecoveryC({
    binary,
    canonical: sh('canonical', manifestPath).trim(),
    manifest: only([point('teardown', 'nothing_committed')]),
    report,
    repetitions: 2
  })
  check(
    gate(clean, 'recovery.teardown.matches_the_contract')?.status === 'pass',
    'and with the cancel working, the same scenario passes'
  )
}

console.log(
  failures
    ? `\n${failures} did not behave: the recovery runner cannot be trusted`
    : '\nthe recovery runner fires where it is armed, fails what is wrong, and passes what is right'
)
process.exit(failures ? 1 : 0)
