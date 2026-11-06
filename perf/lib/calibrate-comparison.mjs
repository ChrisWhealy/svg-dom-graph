// Measures how the comparison behaves, on synthetic runs with a spread like the real ones: how often it calls a regression when
// nothing changed, and how often it finds one of a given size.
//
//   node perf/lib/calibrate-comparison.mjs [--trials 100] [--shift 0.03] [--noise 0.1] [--resamples 2000] [--out FILE]
//
// Every trial draws a baseline and a candidate from fixed seeds and compares them with the real `perf-harness compare`. The
// candidate's typical time is the baseline's times a factor, and a factor of 1 is no change at all. The result is the share of
// trials that came out as each verdict. A false alarm is a regression where the factor is 1, and a miss is no regression where
// the factor is well above the tolerance. The figures belong to the spread that was given, which the real run-to-run spread of a
// runner class replaces when there is one; the statistical gates stay reporting-only until then (design section 8.1).
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { collection } from './synthetic-runs.mjs'
import { baselineDirFor, compareCollections, environmentKey } from './baselines.mjs'
import { spawnSync } from 'node:child_process'
import { root } from './baselines.mjs'

const argument = (name, fallback) => {
  const at = process.argv.indexOf(name)
  return at > 0 ? process.argv[at + 1] : fallback
}
const trials = Number(argument('--trials', 100))
const shift = Number(argument('--shift', 0.03))
const noise = Number(argument('--noise', 0.1))
const resamples = Number(argument('--resamples', 2000))
const out = argument('--out', null)
const factors = [1, 1.05, 1.15, 1.35]

const work = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-calibrate-'))
const rows = []
for (const factor of factors) {
  const tally = {}
  for (let trial = 0; trial < trials; trial++) {
    const seed = 1000 * factor * 7 + trial
    const baselines = path.join(work, 'baselines')
    fs.rmSync(baselines, { recursive: true, force: true })
    const base = collection(path.join(work, 'base'), 'base', { seed: trial + 1, shift, noise })
    const promoted = spawnSync(
      'node',
      [path.join(root, 'perf/runner/promote-baseline.mjs'), '--from', base, '--layer', 'B', '--baselines-dir', baselines, '--reason', 'calibration'],
      { encoding: 'utf8' }
    )
    if (promoted.status !== 0) throw new Error(promoted.stderr)
    const dir = baselineDirFor('w2-connector-rerouting-smoke', 'B', environmentKey(path.join(base, 'base-B-performance-r0.json'))).replace(
      path.join(root, 'perf/baselines'),
      baselines
    )
    const cand = collection(path.join(work, 'cand'), 'cand', { seed: seed + 500000, shift, noise, centre: 50 * factor })
    const report = compareCollections({
      baselineDir: dir,
      candidateDirs: [cand],
      layer: 'B',
      mode: 'performance',
      out: path.join(work, 'c.json'),
      settings: { resamples }
    })
    const verdict = report.comparisons.find(c => c.statistic === 'p50').verdict
    tally[verdict] = (tally[verdict] ?? 0) + 1
    fs.rmSync(path.join(work, 'base'), { recursive: true, force: true })
    fs.rmSync(path.join(work, 'cand'), { recursive: true, force: true })
  }
  rows.push({ factor, trials, verdicts: tally })
  const share = verdict => `${(((tally[verdict] ?? 0) / trials) * 100).toFixed(0)}%`
  console.log(
    `candidate ${((factor - 1) * 100).toFixed(0).padStart(3)}% slower: regression ${share('regression')}, inconclusive ${share('inconclusive')}, no material change ${share('no_material_change')}, improvement ${share('improvement')}`
  )
}
fs.rmSync(work, { recursive: true, force: true })
const record = { trials, run_to_run_shift: shift, within_run_noise: noise, resamples, tolerance: 0.1, repetitions_per_side: 5, rows }
if (out) fs.writeFileSync(out, JSON.stringify(record, null, 2) + '\n')
