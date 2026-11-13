// Shows that the comparison finds a regression and leaves a clean candidate alone against a baseline that was committed and approved, on the runner
// that made it, and not against a baseline of five runs made a moment before (which is what `selftest-regression.mjs` does).
//
//   node perf/runner/selftest-regression.mjs --against-baseline [--manifest ID] [--clean N] [--slow N] [--slowdown-us A,B] [--repetitions N]
//                                                                [--out FILE] [--allow-unapproved]
//
// The quiet spread of five runs in a row is the narrowest that a runner has, so a comparison that works against it has not been shown to work
// against the spread of a baseline that took 10 runs over a longer time. This runs the same claim against the real one:
//
//   1. It makes a collection of clean runs, reads the environment that they were measured in, and finds the baseline of that environment, as
//      the committed `perf/baselines` (or `PERF_BASELINES_DIR`) has it. No baseline, an unapproved one, or a runner that is not the baseline's
//      environment is not a failure: it says so and exits 3, since nothing was shown.
//   2. It makes `--clean` more collections of clean runs and compares each with the baseline. None may be a regression in any metric.
//   3. For each size in `--slowdown-us`, it makes `--slow` collections with that many microseconds spent after every move, inside the timed
//      interval, and compares each. Every one has to be a regression of the move.
//   4. It writes a record: the commit, the environment, the baseline and its runs, every collection's verdict, the false-alarm rate and the
//      detection rate of each size. `qualify.mjs` reads the records in perf/evidence/detection/ and says which approved baselines have one.
//
// Exit status: 0 it holds, 1 it does not, 3 it could not be tried. A collection is `--repetitions` runs, 5 by default, since a smaller one gives
// no verdict at all. It needs the release builds that perf-smoke needs, and a local Chrome.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import {
  baselineDirFor,
  baselinesDir,
  compareCollections,
  environmentKey,
  readPromotion,
  regressions,
  resultFiles,
  root
} from '../lib/baselines.mjs'
import { isQualified, spreadOf } from '../lib/qualification.mjs'

const argument = (name, fallback) => {
  const at = process.argv.indexOf(name)
  return at < 0 ? fallback : process.argv[at + 1]
}
const manifestId = argument('--manifest', 'w2-connector-rerouting-smoke')
const cleanCollections = Number(argument('--clean', 6))
const slowCollections = Number(argument('--slow', 3))
const slowdowns = String(argument('--slowdown-us', '5,10')).split(',').map(Number)
const repetitions = Number(argument('--repetitions', 5))
const out = path.resolve(argument('--out', path.join(root, 'target/regression-against-baseline/report.json')))
const allowUnapproved = process.argv.includes('--allow-unapproved')
const SKIPPED = 3
const work = path.join(path.dirname(out), 'runs')
fs.rmSync(work, { recursive: true, force: true })
fs.mkdirSync(work, { recursive: true })

for (const built of ['target/release/perf-harness', 'perf-harness-wasm/pkg/perf_harness_wasm_bg.wasm'])
  if (!fs.existsSync(path.join(root, built))) throw new Error(`${built} is not built: build it as perf-smoke needs it`)

const orchestrate = (name, ...extra) => {
  const outDir = path.join(work, name)
  const ran = spawnSync(
    'node',
    [
      path.join(root, 'perf/runner/run.mjs'),
      path.join(root, `perf/manifests/${manifestId}.toml`),
      '--mode',
      'performance',
      '--layers',
      'b',
      '--no-build',
      '--repetitions',
      String(repetitions),
      '--out-dir',
      outDir,
      ...extra
    ],
    { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28 }
  )
  return { outDir, status: ran.status }
}
const git = (...args) => spawnSync('git', args, { cwd: root, encoding: 'utf8' }).stdout.trim()
const finish = (record, code) => {
  record.finished_at = new Date().toISOString()
  fs.mkdirSync(path.dirname(out), { recursive: true })
  fs.writeFileSync(out, JSON.stringify(record, null, 2) + '\n')
  const lines = [`## Regression detection against the committed baseline: ${record.outcome}`, '', record.summary, '']
  if (process.env.GITHUB_STEP_SUMMARY) fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, lines.join('\n') + '\n')
  console.log(`\n${record.summary}\nrecord: ${path.relative(root, out)}`)
  process.exit(code)
}
const record = {
  schema_version: 1,
  started_at: new Date().toISOString(),
  commit: git('rev-parse', 'HEAD'),
  tree_clean: git('status', '--porcelain') === '',
  manifest: manifestId,
  repetitions,
  baseline: null,
  environment: null
}

// 1. The environment, and the baseline that belongs to it.
const first = orchestrate('clean-0')
if (first.status !== 0) {
  record.outcome = 'failed'
  record.summary = 'the first clean collection did not pass its deterministic gates, so nothing was compared'
  finish(record, 1)
}
const key = environmentKey(resultFiles(first.outDir)[0])
record.environment = key
const baselineDir = baselineDirFor(manifestId, 'B', key)
const promotion = fs.existsSync(baselineDir) ? readPromotion(baselineDir) : null
if (!promotion) {
  record.outcome = 'skipped'
  record.summary = `there is no baseline of ${manifestId}, layer B, for the environment ${key}, so nothing was shown. Promote one from a run on this runner class.`
  finish(record, SKIPPED)
}
const approved = isQualified(baselineDir)
record.baseline = { dir: path.relative(baselinesDir, baselineDir), runs: promotion.runs, approved }
if (!approved && !allowUnapproved) {
  record.outcome = 'skipped'
  record.summary = `the baseline ${record.baseline.dir} has ${promotion.runs.length} runs and is not approved (node perf/runner/qualify.mjs), so nothing was shown. --allow-unapproved tries it anyway.`
  finish(record, SKIPPED)
}
const moveMedian = spreadOf(baselineDir).find(m => m.name === 'mutation.move_node')?.median ?? null
const compare = (outDir, name) =>
  compareCollections({
    baselineDir,
    candidateDirs: [outDir],
    layer: 'B',
    mode: 'performance',
    out: path.join(work, `${name}.comparison.json`)
  })
const move = report => report.comparisons.find(c => c.metric === 'mutation.move_node' && c.statistic === 'p50')
const judge = (collection, name) => {
  const report = compare(collection.outDir, name)
  const v = move(report)
  return {
    name,
    gates_passed: collection.status === 0,
    comparable: report.compatibility.comparable,
    verdict: v?.verdict ?? null,
    ratio: v ? Number(v.ratio.toFixed(3)) : null,
    interval: v ? [Number(v.ratio_interval.low.toFixed(3)), Number(v.ratio_interval.high.toFixed(3))] : null,
    regressions: regressions(report).map(c => `${c.metric} ${c.statistic}`),
    runs: resultFiles(collection.outDir).map(f => path.basename(f, '.json'))
  }
}

// 2. Candidates that are not changed.
const clean = [judge(first, 'clean-0')]
for (let i = 1; i < cleanCollections; i++) clean.push(judge(orchestrate(`clean-${i}`), `clean-${i}`))
const falseAlarms = clean.filter(c => c.regressions.length > 0)

// 3. Candidates that were made slower.
const slow = slowdowns.map(us => {
  const collections = Array.from({ length: slowCollections }, (_, i) =>
    judge(orchestrate(`slow-${us}-${i}`, '--inject-slowdown-us', String(us)), `slow-${us}-${i}`)
  )
  const detected = collections.filter(c => c.verdict === 'regression').length
  return {
    slowdown_us: us,
    implied_ratio: moveMedian ? Number((1 + us / moveMedian).toFixed(2)) : null,
    collections,
    detected,
    rate: detected / collections.length
  }
})

// 4. The verdict.
const everything = [...clean, ...slow.flatMap(s => s.collections)]
const gatesFailed = everything.filter(c => !c.gates_passed || !c.comparable)
record.baseline.move_node_median_us = moveMedian
record.clean = {
  collections: clean,
  false_alarms: falseAlarms.length,
  false_alarm_rate: falseAlarms.length / clean.length
}
record.slowdowns = slow
const holds =
  falseAlarms.length === 0 && gatesFailed.length === 0 && slow.every(s => s.detected === s.collections.length)
record.outcome = holds ? 'holds' : 'does_not_hold'
record.summary =
  `${clean.length} clean collections of ${repetitions} runs: ${falseAlarms.length} flagged as a regression. ` +
  slow
    .map(
      s =>
        `A slowdown of ${s.slowdown_us} µs a move (about ${s.implied_ratio}×) was found in ${s.detected} of ${s.collections.length}.`
    )
    .join(' ') +
  (gatesFailed.length ? ` ${gatesFailed.length} collections did not pass their gates or were not comparable.` : '') +
  ` Against ${record.baseline.dir}, ${promotion.runs.length} runs, ${approved ? 'approved' : 'NOT approved'}.`
for (const c of clean)
  console.log(
    `clean ${c.name}: ${c.verdict}, ratio ${c.ratio} (${c.interval?.join(' to ')})${c.regressions.length ? `, REGRESSIONS: ${c.regressions.join(', ')}` : ''}`
  )
for (const s of slow)
  for (const c of s.collections)
    console.log(`slow ${c.name}: ${c.verdict}, ratio ${c.ratio} (${c.interval?.join(' to ')})`)
finish(record, holds ? 0 : 1)
