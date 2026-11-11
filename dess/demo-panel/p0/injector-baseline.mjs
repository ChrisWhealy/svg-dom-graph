// Captures what the page injector counts for one operation of the simulator, from the injector as it is, so that a later change to the injector can be
// held to it. Run it once, before the injector is changed, and keep the result: it is the prior vector that `injector.mjs` compares against.
//
//   node dess/demo-panel/p0/injector-baseline.mjs
//
// The operation is the construction of the simulator's product fixture and then one committed slice, which is a real `Surface` draw: overlay writes
// and the data node's setter. For each of a few ordinals, `arm(n)` is made to fail that call of the slice, and what it counted and which call it
// failed is kept. It writes dess/demo-panel/p0/evidence/injector-baseline.json, with the injector's own hash, so that the file says which injector it
// describes.
import fs from 'node:fs'
import path from 'node:path'
import crypto from 'node:crypto'
import { openHarnessPage } from '../../../perf/runner/lib/harness-page.mjs'

const root = path.resolve(import.meta.dirname, '../../..')
const injector = fs.readFileSync(path.join(root, 'perf/page/fault-injector.js'), 'utf8')
const out = path.join(root, 'dess/demo-panel/p0/evidence/injector-baseline.json')

export const SEED = '20261106'
export const ARMED = [1, 4, 12]

/** The calls that one slice makes, and what arming each of a few ordinals does, as the injector in the page reports them. */
export const vector = async page => {
  const run = async expression => JSON.parse(await page.evaluate(`JSON.stringify(${expression})`))
  const build = () =>
    page.evaluate(`window.perfHarness.sim_build(JSON.stringify({ seed: ${JSON.stringify(SEED)}, product: true }))`)
  await build()
  const trace = await run(`(() => {
    const f = window.__faults
    f.record(true)
    window.perfHarness.sim_step(1)
    f.record(false)
    return f.take()
  })()`)
  const arms = []
  for (const n of ARMED) {
    await build()
    arms.push({
      n,
      ...(await run(`(() => {
        const f = window.__faults
        f.arm(${n})
        const step = JSON.parse(window.perfHarness.sim_step(1))
        const report = f.disarm()
        return { counted: report.counted, fired: report.fired, outcome: step.last }
      })()`))
    })
  }
  await page.evaluate('window.perfHarness.sim_teardown()')
  return { calls: trace.length, trace, arms }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const page = await openHarnessPage({ faults: true })
  try {
    const recorded = await vector(page)
    const record = {
      injector_sha256: crypto.createHash('sha256').update(injector).digest('hex'),
      operation: 'sim_build (product) then sim_step(1)',
      ...recorded
    }
    fs.mkdirSync(path.dirname(out), { recursive: true })
    fs.writeFileSync(out, JSON.stringify(record, null, 2) + '\n')
    console.log(
      `${recorded.calls} counted calls in one slice, and ${recorded.arms.length} armed ordinals, for the injector ${record.injector_sha256.slice(0, 12)}`
    )
    console.log(`written to ${path.relative(root, out)}`)
  } finally {
    await page.close()
  }
}
