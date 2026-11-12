// Checks the rules for approving a baseline on baselines written by hand, so that every refusal has a case. It needs no browser.
//
//   node perf/runner/test-qualification.mjs
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-qualification-'))
process.env.PERF_BASELINES_DIR = scratch
const { RUNS_TO_BLOCK } = await import('../lib/baselines.mjs')
const { NOISE_LIMIT, isQualified, shortfalls, spreadOf } = await import('../lib/qualification.mjs')

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

/** A baseline of `medians.length` runs, each with one latency whose median is the next of `medians`. */
const baseline = (name, medians) => {
  const dir = path.join(scratch, 'w-example', 'B', name)
  fs.mkdirSync(dir, { recursive: true })
  const runs = medians.map((_, i) => `w-example-B-performance-r${i}-20261112T000000`)
  medians.forEach((median, i) =>
    fs.writeFileSync(
      path.join(dir, `${runs[i]}.json`),
      JSON.stringify({
        run_id: runs[i],
        mode: 'performance',
        metrics: [
          {
            name: 'construction.nodes',
            layer: 'B',
            status: 'measured',
            kind: 'latency',
            unit: 'ms',
            summary: { min: median, p50: median, max: median }
          }
        ]
      })
    )
  )
  fs.writeFileSync(path.join(dir, 'promotion.json'), JSON.stringify({ runs }))
  return dir
}
const approve = (dir, ...extra) =>
  spawnSync('node', [path.join(import.meta.dirname, 'qualify.mjs'), '--approve', dir, '--reason', 'a test', ...extra], {
    encoding: 'utf8',
    env: { ...process.env, PERF_BASELINES_DIR: scratch }
  })

const steady = baseline(
  'steady',
  Array.from({ length: RUNS_TO_BLOCK }, (_, i) => 10 + (i % 3) * 0.1)
)
check(spreadOf(steady)[0].spread < 1.1, 'the spread is the highest median over the lowest')
check(shortfalls(steady).length === 0, `${RUNS_TO_BLOCK} steady runs lack nothing`)
check(!isQualified(steady), 'a baseline that nobody approved does not block')
check(approve(steady).status === 0, 'a baseline with enough steady runs is approved')
check(isQualified(steady), 'and then it blocks')
const promotion = JSON.parse(fs.readFileSync(path.join(steady, 'promotion.json'), 'utf8'))
fs.writeFileSync(
  path.join(steady, 'promotion.json'),
  JSON.stringify({ runs: [...promotion.runs.slice(1), 'another-run'] })
)
check(!isQualified(steady), 'promoting other runs over it voids the approval')

const few = baseline('few', [10, 10, 10])
check(approve(few).status === 1 && !isQualified(few), 'three runs are not enough to approve')

const noisy = baseline(
  'noisy',
  Array.from({ length: RUNS_TO_BLOCK }, (_, i) => (i % 2 ? 10 : 10 * (NOISE_LIMIT + 0.5)))
)
check(approve(noisy).status === 1 && !isQualified(noisy), 'runs that vary by more than the limit are refused')
check(
  approve(noisy, '--accept-noise', 'a shared runner').status === 0 && isQualified(noisy),
  'unless a reason for accepting the noise is given'
)
check(
  JSON.parse(fs.readFileSync(path.join(noisy, 'qualification.json'), 'utf8')).accepted_noise === 'a shared runner',
  'and the reason is on record'
)

fs.rmSync(scratch, { recursive: true, force: true })
process.exit(failures ? 1 : 0)
