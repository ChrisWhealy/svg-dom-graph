// P0-B and P0-C: the page injector's detach mode, tested on the simulator's real surface and controller.
//
//   node dess/demo-panel/p0/injector.mjs [--skip-recovery] [--out dess/demo-panel/p0/evidence/injector.json]
//
// The gallery's draw has to meet a host that leaves in the middle of it, and the gallery is one wasm build with no Rust fault hook, so the page
// injector (perf/page/fault-injector.js) gets a detach mode: `__faults.armDetach({ svgId, method, nth })` makes the nth call of `method` inside the
// `<svg>` natively and then removes the `<svg>`, and `reattach()` puts it back. This holds that mode to what design-specs/dess_demo.pdf asks of it.
//
//   1. Compatibility. With the mode unarmed, the injector counts what it counted before it had the mode. The calls of one slice, and what arming
//      three ordinals does, are compared with the vector that injector-baseline.mjs captured from the injector as it was.
//   2. Firing. Armed at the first, a middle and the last call of a slice, it fires exactly once, at that call, as a native write and then a detach,
//      and the report says which method, which ordinal and which tag and attribute. The controller refuses the commit (Unavailable, the cursor
//      unmoved, the page dirty), and after `reattach()` a repair makes the page the committed slice again and the next slice commits.
//   3. Not firing. An ordinal past the last call reports not fired, a helper that a test uses to demand a firing throws on it, and the slice
//      that it did not interrupt commits normally. A test that is armed and does not fire is a failed test and never a pass.
//   4. Isolation. A detach cannot be armed while an exception is, and a second arm is refused.
//   5. Cleanup. Disarmed, nothing is left: no arm state, no timer, no frame, no new listener, and the counting is the baseline's again, which is
//      checked a second time after all of the above.
//
// With `--skip-recovery` the existing graph recovery self-test (perf/runner/selftest-recovery.mjs) is not run. Without it, it is, since the
// injector is shared and that self-test is its other user. It exits with 1 if anything fails.
import fs from 'node:fs'
import path from 'node:path'
import crypto from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from '../../../perf/runner/lib/harness-page.mjs'
import { vector, SEED } from './injector-baseline.mjs'

const root = path.resolve(import.meta.dirname, '../../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const out = path.resolve(argument('--out') ?? path.join(root, 'dess/demo-panel/p0/evidence/injector.json'))
const baseline = JSON.parse(
  fs.readFileSync(path.join(root, 'dess/demo-panel/p0/evidence/injector-baseline.json'), 'utf8')
)
const injectorHash = crypto
  .createHash('sha256')
  .update(fs.readFileSync(path.join(root, 'perf/page/fault-injector.js')))
  .digest('hex')

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b)

/** What a test does with an armed detach: it has to have fired, exactly once, and a report that says it did not is a failure. */
export const assertFired = report => {
  if (!report || report.fired !== true || report.firings !== 1)
    throw new Error(`the detach was armed and did not fire exactly once: ${JSON.stringify(report)}`)
  return report
}

const page = await openHarnessPage({ faults: true })
const run = async expression => JSON.parse(await page.evaluate(`JSON.stringify(${expression})`))
// The wasm functions answer in JSON text, which `evaluate` returns as it is.
const call = async (fn, ...args) =>
  JSON.parse(await page.evaluate(`window.perfHarness.${fn}(${args.map(a => JSON.stringify(a)).join(', ')})`))
const build = () =>
  page.evaluate(`window.perfHarness.sim_build(JSON.stringify({ seed: ${JSON.stringify(SEED)}, product: true }))`)
const state = () => call('sim_state')
const evidence = { injector_sha256: injectorHash, baseline_injector_sha256: baseline.injector_sha256, cases: [] }

try {
  const unarmedTrace = await run('typeof window.__faults.armDetach')
  check(unarmedTrace === 'function', 'the detach mode is in the page, and nothing has armed it')

  // 1. Compatibility, with the mode unarmed.
  const before = await vector(page)
  check(
    before.calls === baseline.calls && same(before.trace, baseline.trace),
    `unarmed, one slice makes the ${baseline.calls} counted calls that it made before, in the same order`
  )
  check(
    same(before.arms, baseline.arms),
    'and arming three ordinals counts and fails what it did before: ' +
      before.arms.map(a => `${a.n} counted ${a.counted}`).join(', ')
  )

  // How many calls the slice that the detach will interrupt makes, which is the slice after the first.
  await build()
  await page.evaluate('window.perfHarness.sim_step(1)')
  const sliceCalls = await run(
    `(() => { const f = window.__faults; f.record(true); window.perfHarness.sim_step(1); f.record(false); return f.take() })()`
  )
  const eligible = sliceCalls.filter(c => c.method === 'setAttribute').length
  evidence.eligible_calls_in_the_interrupted_slice = eligible
  check(
    eligible >= 3,
    `the slice that is interrupted makes ${eligible} counted setAttribute calls inside the scene, which gives ordinals to arm`
  )

  // 2. Firing, at the first, a middle and the last call.
  for (const nth of [...new Set([1, Math.ceil(eligible / 2), eligible])]) {
    await build()
    await page.evaluate('window.perfHarness.sim_step(1)')
    const atRest = await state()
    await page.evaluate(`window.__faults.armDetach({ svgId: 'perf-sim-svg', method: 'setAttribute', nth: ${nth} })`)
    const stepped = await call('sim_step', 1)
    const report = await run('window.__faults.disarmDetach()')
    const interrupted = await state()
    let fired = false
    try {
      assertFired(report)
      fired =
        report.site.ordinal === nth &&
        report.eligibleSeen === nth &&
        report.site.method === 'setAttribute' &&
        report.detached
    } catch {
      fired = false
    }
    check(
      fired,
      `armed at call ${nth} of ${eligible}: it fires once, at call ${nth}, as ${report?.site?.method} on a <${report?.site?.tag}> for ${report?.site?.name}, and the host has left`
    )
    check(
      stepped.last === 'unavailable' &&
        interrupted.committed_slice === atRest.committed_slice &&
        interrupted.dirty === true &&
        interrupted.presence === 'Detached',
      `at call ${nth} the controller refuses the commit: ${stepped.last}, still slice ${interrupted.committed_slice}, dirty ${interrupted.dirty}, the host ${interrupted.presence}`
    )
    const back = await run('window.__faults.reattach()')
    const repaired = await call('sim_repair')
    const healed = await state()
    const next = await call('sim_step', 1)
    const after = await state()
    check(
      back === true &&
        repaired.outcome === 'done' &&
        !healed.dirty &&
        healed.page_is_projection &&
        healed.committed_slice === atRest.committed_slice,
      `after the host is put back, a repair makes the page slice ${healed.committed_slice} again, clean and equal to its projection`
    )
    check(
      next.last === 'done' && after.committed_slice === atRest.committed_slice + 1 && after.page_is_projection,
      'and the next slice then commits'
    )
    evidence.cases.push({
      nth,
      report,
      stepped: stepped.last,
      interrupted: { slice: interrupted.committed_slice, dirty: interrupted.dirty, presence: interrupted.presence },
      repaired: repaired.outcome,
      next: next.last
    })
  }

  // 3. Not firing.
  await build()
  await page.evaluate('window.perfHarness.sim_step(1)')
  await page.evaluate(
    `window.__faults.armDetach({ svgId: 'perf-sim-svg', method: 'setAttribute', nth: ${eligible + 5} })`
  )
  const quiet = await call('sim_step', 1)
  const none = await run('window.__faults.disarmDetach()')
  let refused = false
  try {
    assertFired(none)
  } catch {
    refused = true
  }
  check(
    none.fired === false && none.eligibleSeen === eligible && none.detached === false,
    `an ordinal past the last (${eligible + 5}) reports not fired, after seeing ${none.eligibleSeen} calls, and nothing left`
  )
  check(
    refused,
    'and the helper that a test uses to demand a firing throws on it, so that a detach that never happened cannot pass'
  )
  check(quiet.last === 'done', 'the slice that it did not interrupt committed normally')
  evidence.cases.push({ nth: eligible + 5, report: none, stepped: quiet.last })

  // The counting while a detach is armed. A detach that cannot fire (an ordinal that is never reached) must not change what is recorded or what an
  // exception does, since a test that arms both modes at different calls would otherwise be measuring the injector.
  await build()
  await page.evaluate('window.perfHarness.sim_step(1)')
  const recordedWhileArmed = await run(`(() => {
    const f = window.__faults
    f.armDetach({ svgId: 'perf-sim-svg', method: 'setAttribute', nth: 1000 })
    f.record(true)
    window.perfHarness.sim_step(1)
    f.record(false)
    const calls = f.take()
    f.disarmDetach()
    return calls
  })()`)
  check(
    recordedWhileArmed.length > 0 && same(recordedWhileArmed, sliceCalls),
    `with a detach armed that cannot fire, recording still sees the ${sliceCalls.length} calls of the slice, the same ones in the same order`
  )
  await build()
  await page.evaluate('window.perfHarness.sim_step(1)')
  const failedWhileArmed = await run(`(() => {
    const f = window.__faults
    f.armDetach({ svgId: 'perf-sim-svg', method: 'setAttribute', nth: 1000 })
    f.arm(1)
    const step = JSON.parse(window.perfHarness.sim_step(1))
    const report = f.disarm()
    f.disarmDetach()
    return { fired: report.fired, outcome: step.last }
  })()`)
  check(
    failedWhileArmed.fired !== null &&
      failedWhileArmed.outcome === 'failed' &&
      failedWhileArmed.fired.name === baseline.arms[0].fired.name &&
      failedWhileArmed.fired.tag === baseline.arms[0].fired.tag,
    'and an exception armed beside it still fails the call that it did without the detach'
  )

  // 4. Isolation from the exception mode, and from itself.
  await build()
  const both = await run(
    `(() => { const f = window.__faults; f.arm(1); let refused = null; try { f.armDetach({ svgId: 'perf-sim-svg', nth: 1 }) } catch (e) { refused = String(e.message) } f.disarm(); return refused })()`
  )
  check(/exception is armed/.test(both ?? ''), `a detach is refused while an exception is armed: ${both}`)
  const twice = await run(
    `(() => { const f = window.__faults; f.armDetach({ svgId: 'perf-sim-svg', nth: 1 }); let refused = null; try { f.armDetach({ svgId: 'perf-sim-svg', nth: 2 }) } catch (e) { refused = String(e.message) } f.disarmDetach(); return refused })()`
  )
  check(/already armed/.test(twice ?? ''), `and a second arm is refused: ${twice}`)

  // 5. Cleanup, and the counting once more, after everything above.
  const leftover = await run(`({
    armed: window.__faults.disarmDetach(),
    timers: window.__timers.pending().length,
    frames: window.__frames.pending(),
    listeners: window.__listeners?.snapshot().total ?? null
  })`)
  check(
    leftover.armed === null && leftover.timers === 0 && leftover.frames === 0,
    'disarmed, no detach state, no held timer and no held frame is left'
  )
  await page.evaluate('window.perfHarness.sim_teardown()')
  const tidy = await run('window.__listeners ? window.__listeners.snapshot().total : null')
  check(tidy === 0 || tidy === null, `and no listener is left after the fixture is torn down (${tidy})`)
  const after = await vector(page)
  check(
    after.calls === baseline.calls && same(after.trace, baseline.trace) && same(after.arms, baseline.arms),
    'the counting after all of it is the prior vector again, call for call'
  )
} finally {
  await page.close()
}

// The injector's other user.
if (process.argv.includes('--skip-recovery')) console.log('skip the graph recovery self-test: --skip-recovery')
else {
  const ran = spawnSync('node', [path.join(root, 'perf/runner/selftest-recovery.mjs')], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 1 << 28
  })
  check(
    ran.status === 0 && /the recovery runner fires where it is armed/.test(ran.stdout),
    'the existing graph recovery self-test, which uses the same injector, still passes'
  )
  evidence.recovery_selftest = ran.status === 0 ? 'pass' : 'fail'
}

evidence.pass = failures === 0
fs.writeFileSync(out, JSON.stringify(evidence, null, 2) + '\n')
console.log(
  failures
    ? `\n${failures} failed`
    : `\nthe detach mode fires where it is armed, reports when it does not, and leaves the injector as it was\nevidence: ${path.relative(root, out)}`
)
process.exit(failures ? 1 : 0)
