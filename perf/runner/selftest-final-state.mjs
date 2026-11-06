// Proves that a performance run cannot end in a wrong state without saying so, in a real browser.
//
//   node perf/runner/selftest-final-state.mjs
//
// A performance run executes only the last two checkpoints, to keep checks out of the timing. Those two have to vouch for
// everything that was timed. For every workload and both modes this runs the stream once as it is, which has to be valid,
// and then twice with the last mutation before a checkpoint made wrongly: once before the second to last checkpoint, which
// for the cycle workloads is a completed scene, and once before the last, which is the empty page after the final removal
// or, for rerouting, the final positions after every move. The operation is the thing that is wrong, and the reference model
// is given it right. Each of those has to come out invalid, and has to do so through a checkpoint, since the stream is
// still the one that was generated. It exits with 1 if any does not.
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from './lib/harness-page.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const binary = path.join(root, 'target/release/perf-harness')
const workloads = ['w1-scene-construction-small', 'w2-connector-rerouting-small', 'w3-scene-lifecycle-small']

const page = await openHarnessPage()
let failures = 0
const report = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

for (const name of workloads) {
  const manifest = path.join(root, 'perf/manifests', `${name}.toml`)
  const canonical = spawnSync(binary, ['canonical', manifest], { encoding: 'utf8' })
  if (canonical.status !== 0) throw new Error(`could not read ${name}: ${canonical.stderr}`)
  const json = JSON.stringify(canonical.stdout.trim())
  const run = async (mode, target) =>
    JSON.parse(
      await page.evaluate(
        `window.perfHarness.selftest_corruption(${json}, ${JSON.stringify(mode)}, ${JSON.stringify(target)})`
      )
    )

  for (const mode of ['performance', 'correctness']) {
    const clean = await run(mode, 'none')
    report(
      clean.status === 'ok' && clean.checkpoints_failed.length === 0,
      `${name} ${mode}, as generated: ${clean.status}, ${clean.checkpoints_passed} checkpoints passed`
    )
    if (mode === 'performance')
      report(
        clean.checkpoints_passed === 2,
        `${name} performance executes exactly the last two checkpoints (${clean.checkpoints_passed})`
      )

    for (const target of ['penultimate', 'final']) {
      const outcome = await run(mode, target)
      const sections = [...new Set(outcome.checkpoints_failed.map(failure => failure.section))].join(', ')
      const caught =
        outcome.status === 'invalid_incorrect' &&
        outcome.checkpoints_failed.length > 0 &&
        outcome.stream?.matches === true
      report(
        caught,
        `${name} ${mode}, wrong last mutation before the ${target} checkpoint: ${outcome.status}${caught ? `, caught in ${sections}` : outcome.error ? `, ${outcome.error}` : ', not caught'}`
      )
      if (caught && mode === 'performance') console.log(`      ${outcome.checkpoints_failed[0].detail}`)
    }
  }
}
await page.close()
console.log(
  failures
    ? `\n${failures} did not behave: a performance run can end in a wrong state unnoticed`
    : '\nevery wrong final mutation is caught, and every run as generated is valid'
)
process.exit(failures ? 1 : 0)
