// Proves, in a real browser, that the harness catches a regression and a leak that are put into a candidate on purpose, and that a
// candidate which is not changed produces neither.
//
//   node perf/runner/selftest-regression.mjs
//
// This is the exit condition of Phase 5. It needs the release build of perf-harness and of perf-harness-wasm, and a local Chrome.
//
//   1. A baseline of five clean runs of the rerouting workload is promoted.
//   2. Five more clean runs are compared with it. They must not regress, and their deterministic gates must pass.
//   3. Five runs with a slowdown injected inside the timed interval, after every move, are compared with it. The comparison must
//      say regression for the move, with an interval wholly above the tolerance. The orchestrator's own exit status must still be
//      success, since a timing is never a deterministic gate.
//   4. A clean run of the lifecycle workload must pass its deterministic gates, and the same workload with listeners leaked after
//      every teardown must fail them, and name the listeners.
//   5. A run with anything injected carries it in its result, and cannot be promoted.
//
// The browser-free half of the same claim, with every verdict and every refusal, is perf/lib/test-regression.mjs.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { baselineDirFor, compareCollections, environmentKey, resultFiles, root, regressions } from '../lib/baselines.mjs'

const work = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-selftest-regression-'))
let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

const orchestrate = (manifest, name, ...extra) => {
  const outDir = path.join(work, name)
  const result = spawnSync(
    'node',
    [path.join(root, 'perf/runner/run.mjs'), path.join(root, `perf/manifests/${manifest}.toml`), '--mode', 'performance', '--layers', 'b', '--no-build', '--out-dir', outDir, ...extra],
    { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28 }
  )
  return { outDir, status: result.status, output: result.stdout + result.stderr }
}
const promote = (from, baselines, ...extra) =>
  spawnSync('node', [path.join(root, 'perf/runner/promote-baseline.mjs'), '--from', from, '--layer', 'B', '--baselines-dir', baselines, ...extra], {
    encoding: 'utf8'
  })

const W2 = 'w2-connector-rerouting-smoke'
const W3 = 'w3-scene-lifecycle-smoke'
const baselines = path.join(work, 'baselines')

// 1. The baseline.
const base = orchestrate(W2, 'base', '--repetitions', '5')
check(base.status === 0, `five clean runs of ${W2} pass their deterministic gates`)
// A working tree with changes is the normal state of a developer, and the promotion command says so and refuses it. The
// self-test is about the comparison, so it promotes with the flag that is for trying the command out.
const promoted = promote(base.outDir, baselines, '--reason', 'Self-test baseline', '--allow-dirty')
check(promoted.status === 0, `they are promoted: ${promoted.stdout.split('\n')[0]}${promoted.stderr}`)
const key = environmentKey(resultFiles(base.outDir)[0])
const baselineDir = baselineDirFor(W2, 'B', key).replace(path.join(root, 'perf/baselines'), baselines)
const compare = (outDir, name) =>
  compareCollections({ baselineDir, candidateDirs: [outDir], layer: 'B', mode: 'performance', out: path.join(work, `${name}.comparison.json`) })
const move = report => report.comparisons.find(c => c.metric === 'mutation.move_node' && c.statistic === 'p50')

// 2. A candidate that is not changed.
const clean = orchestrate(W2, 'clean', '--repetitions', '5')
check(clean.status === 0, 'five more clean runs pass their deterministic gates')
{
  const report = compare(clean.outDir, 'clean')
  const v = move(report)
  check(
    report.compatibility.comparable && regressions(report).length === 0 && v && v.verdict !== 'regression',
    `and are not a regression: ${v?.verdict}, ratio ${v?.ratio.toFixed(3)}, interval ${v?.ratio_interval.low.toFixed(3)} to ${v?.ratio_interval.high.toFixed(3)}`
  )
}

// 3. A candidate that was made slower.
const slow = orchestrate(W2, 'slow', '--repetitions', '5', '--inject-slowdown-us', '20')
check(slow.status === 0, 'five runs with a slowdown injected still pass their deterministic gates, since a timing is not one')
{
  const report = compare(slow.outDir, 'slow')
  const v = move(report)
  check(
    v?.verdict === 'regression' && v.ratio_interval.low > 1.1,
    `and are a regression against the baseline: ratio ${v?.ratio.toFixed(2)}, interval ${v?.ratio_interval.low.toFixed(2)} to ${v?.ratio_interval.high.toFixed(2)}, against a tolerance of 10%`
  )
}
check(
  resultFiles(slow.outDir).every(file => JSON.parse(fs.readFileSync(file, 'utf8')).injection?.slowdown_us === 20),
  'each of their results says that a slowdown was injected'
)

// 4. A leak.
const tidy = orchestrate(W3, 'lifecycle', '--repetitions', '3')
check(tidy.status === 0 && !/FAIL/.test(tidy.output), `a clean ${W3} passes its deterministic gates`)
const leaky = orchestrate(W3, 'leak', '--repetitions', '3', '--inject-leak', 'library_watch')
check(
  leaky.status !== 0 && /FAIL listeners after teardown equal those before the build \(delta [1-9]/.test(leaky.output),
  `the same workload with listeners leaked after every teardown fails them: ${(leaky.output.match(/FAIL listeners[^\n]*/) ?? [''])[0].trim()}`
)

// 5. What was injected is never a baseline.
{
  const refused = promote(slow.outDir, path.join(work, 'other'), '--reason', 'x', '--allow-dirty')
  check(refused.status !== 0 && /changed on purpose/.test(refused.stderr), 'a run with a slowdown injected is refused as a baseline')
}

fs.rmSync(work, { recursive: true, force: true })
console.log(failures ? `\n${failures} did not behave: the regression check cannot be trusted` : '\nthe harness catches an injected regression and an injected leak, and a clean candidate produces neither')
process.exit(failures ? 1 : 0)
