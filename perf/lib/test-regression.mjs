// Tests the comparison, the baseline promotion and the reports on synthetic runs, with no browser.
//
//   node perf/lib/test-regression.mjs
//
// The runs are written to a temporary directory in the shape of real results, with timings drawn from a fixed seed, and are given to
// the real `perf-harness compare` and to the real promotion command. A clean fixture has to produce no regression. A fixture with
// a slowdown injected has to produce one, and a leak is caught by the deterministic gate of the orchestrator, which
// selftest-regression.mjs shows in a browser. What is tested here is everything that does not need one: the verdicts, the refusals,
// and that a comparison never touches a baseline.
//
// It needs the release build of perf-harness and the schemas' dependencies (`npm ci` in perf/schemas).
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { collection, HEAD, OTHER } from './synthetic-runs.mjs'
import { baselineDirFor, compareCollections, environmentKey, readPromotion, regressions, root, run, summarise } from './baselines.mjs'

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

const work = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-regression-'))

const promote = (from, baselines, ...extra) =>
  spawnSync('node', [path.join(root, 'perf/runner/promote-baseline.mjs'), '--from', from, '--layer', 'B', '--baselines-dir', baselines, ...extra], {
    encoding: 'utf8'
  })
const digest = dir =>
  createHash('sha256')
    .update(fs.readdirSync(dir).sort().map(name => name + fs.readFileSync(path.join(dir, name), 'utf8')).join('\0'))
    .digest('hex')
const verdictOf = report => report.comparisons.find(c => c.metric === 'mutation.move_node' && c.statistic === 'p50')
const compare = (baselineDir, candidate, settings) =>
  compareCollections({ baselineDir, candidateDirs: [candidate], layer: 'B', mode: 'performance', out: path.join(work, `cmp-${Math.random().toString(36).slice(2)}.json`), settings })

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Promotion: the first baseline of a workload.
const baselines = path.join(work, 'baselines')
const first = collection(path.join(work, 'first'), 'first', { seed: 11 })
{
  const refused = promote(first, baselines, '--reason', 'x', '--min-runs', '6')
  check(refused.status !== 0 && /5 runs, and at least 6/.test(refused.stderr), 'a baseline of fewer runs than were asked for is refused')
}
{
  const dirty = collection(path.join(work, 'dirty'), 'dirty', { seed: 11, environment: { git_dirty: true } })
  const refused = promote(dirty, baselines, '--reason', 'x')
  check(refused.status !== 0 && /uncommitted changes/.test(refused.stderr), 'runs from a tree with uncommitted changes are refused')
}
{
  const unknown = collection(path.join(work, 'unknown'), 'unknown', { seed: 11, environment: { git_sha: OTHER } })
  const refused = promote(unknown, baselines, '--reason', 'x')
  check(refused.status !== 0 && /git does not know the commit/.test(refused.stderr), 'runs from a commit that git does not know are refused')
}
{
  const invalid = collection(path.join(work, 'invalid'), 'invalid', { seed: 11, document: { status: 'invalid_incorrect' } })
  const refused = promote(invalid, baselines, '--reason', 'x')
  check(refused.status !== 0 && /only a valid run may be a baseline/.test(refused.stderr), 'a run that is not valid is refused')
}
{
  const mixed = collection(path.join(work, 'mixed'), 'mixed', { seed: 11 })
  const file = path.join(mixed, 'mixed-B-performance-r0.json')
  const doc = JSON.parse(fs.readFileSync(file, 'utf8'))
  doc.manifest.sha256 = 'c'.repeat(64)
  fs.writeFileSync(file, JSON.stringify(doc))
  const refused = promote(mixed, baselines, '--reason', 'x')
  check(refused.status !== 0 && /more than one manifest/.test(refused.stderr), 'runs of two manifests are refused')
}
{
  const injected = collection(path.join(work, 'injected'), 'injected', { seed: 11, document: { injection: { slowdown_us: 10, leak: 'none' } } })
  const refused = promote(injected, baselines, '--reason', 'x')
  check(refused.status !== 0 && /changed on purpose/.test(refused.stderr), 'runs of a candidate changed on purpose are never a baseline')
}
check(!fs.existsSync(baselines), 'none of those refusals wrote anything')

{
  const done = promote(first, baselines, '--reason', 'The first baseline on this runner class')
  check(done.status === 0, `a clean collection is promoted: ${done.stdout.split('\n')[0]}${done.stderr}`)
}
const key = environmentKey(path.join(first, 'first-B-performance-r0.json'))
const baselineDir = baselineDirFor('w2-connector-rerouting-smoke', 'B', key).replace(path.join(root, 'perf/baselines'), baselines)
const record = readPromotion(baselineDir)
check(
  record?.promotion.commit === HEAD &&
    record.promotion.workload_hash === 'a'.repeat(64) &&
    record.promotion.environment_key === key &&
    record.promotion.method_version === '1' &&
    record.previous === null &&
    record.accepted_regression === null &&
    record.runs.length === 5,
  `the promotion record names the commit, the workload hash, the environment (${key}), the method and the runs`
)
check(
  fs.readdirSync(baselineDir).filter(name => name.endsWith('.samples.ndjson')).length === 5,
  'the baseline keeps the samples beside its results, so that it stands alone'
)

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Comparison.
const before = digest(baselineDir)
{
  const clean = compare(baselineDir, collection(path.join(work, 'clean'), 'clean', { seed: 21 }))
  const v = verdictOf(clean)
  check(v?.verdict === 'no_material_change' && regressions(clean).length === 0, `a clean fixture is no material change (ratio ${v?.ratio.toFixed(3)}, interval ${v?.ratio_interval.low.toFixed(3)} to ${v?.ratio_interval.high.toFixed(3)})`)
  check(clean.compatibility.comparable === true && clean.gating.statistical_gates === 'reporting_only' && clean.gating.baseline_runs === 5, 'a baseline of 5 runs reports and does not block')
}
{
  const slow = compare(baselineDir, collection(path.join(work, 'slow'), 'slow', { seed: 22, centre: 50 * 1.35 }))
  const v = verdictOf(slow)
  check(v?.verdict === 'regression' && v.ratio_interval.low > 1.1, `an injected slowdown of 35% is a regression (ratio ${v?.ratio.toFixed(3)}, interval ${v?.ratio_interval.low.toFixed(3)} to ${v?.ratio_interval.high.toFixed(3)})`)
  check(summarise(slow, 'x').join('\n').includes('**regression**'), 'and the summary a person reads says so')
}
{
  const fast = compare(baselineDir, collection(path.join(work, 'fast'), 'fast', { seed: 23, centre: 50 * 0.7 }))
  check(verdictOf(fast)?.verdict === 'improvement', `an injected speedup of 30% is an improvement, and not a regression`)
}
{
  const few = compare(baselineDir, collection(path.join(work, 'few'), 'few', { seed: 24, runs: 3, centre: 50 * 2 }))
  const v = verdictOf(few)
  check(v?.verdict === 'diagnostic' && v.ratio_interval === undefined && v.ratio > 1.9, `three repetitions, even of a doubling, give a diagnostic and no verdict (ratio ${v?.ratio.toFixed(2)})`)
}
{
  const noisy = compare(baselineDir, collection(path.join(work, 'noisy'), 'noisy', { seed: 25, centre: 50 * 1.12, shift: 0.3 }))
  check(verdictOf(noisy)?.verdict === 'inconclusive', `runs that disagree among themselves give inconclusive, which is never a pass (${verdictOf(noisy)?.verdict})`)
}
for (const [what, overrides, pattern] of [
  ['another browser', { environment: { browser_version: '158.0.1.0' } }, /browser version/],
  ['another runner class', { environment: { runner_class: 'laptop' } }, /runner class/],
  ['another protocol', { environment: { protocol_version: '2' } }, /measurement protocol/],
  ['another manifest', { manifest: { sha256: 'd'.repeat(64) } }, /manifests differ/],
  ['another operation stream', { manifest: { operation_stream_sha256: 'e'.repeat(64) } }, /operation streams differ/],
  ['an invalid candidate run', { document: { status: 'invalid_incorrect' } }, /did not do its work/]
]) {
  const report = compare(baselineDir, collection(path.join(work, `nc-${what.replaceAll(' ', '-')}`), 'nc', { seed: 26, ...overrides }))
  const v = verdictOf(report)
  check(
    report.compatibility.comparable === false && pattern.test(report.compatibility.reason) && v?.verdict === 'not_comparable' && v.ratio === undefined,
    `${what} is not comparable, with no figure compared: ${report.compatibility.reason.slice(0, 90)}`
  )
}
check(digest(baselineDir) === before, 'no comparison changed the baseline')

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Promotion over an existing baseline.
{
  const slow = collection(path.join(work, 'slow'), 'slow', { seed: 22, centre: 50 * 1.35 })
  const refused = promote(slow, baselines, '--reason', 'Newer')
  check(refused.status !== 0 && /regress against the current baseline/.test(refused.stderr), 'a promotion over a regression is refused')
  check(digest(baselineDir) === before, 'and leaves the baseline as it was')
  const accepted = promote(slow, baselines, '--reason', 'Newer', '--accept-regression', 'The move now keeps connectors in step, which costs 35%')
  const after = readPromotion(baselineDir)
  check(
    accepted.status === 0 && after?.accepted_regression?.reason.includes('35%') && after.accepted_regression.metrics.includes('mutation.move_node') && after.previous?.commit === HEAD,
    'with the reason given it is promoted, and the record keeps the reason, the metric and what it replaced'
  )
}
{
  const same = promote(collection(path.join(work, 'again'), 'again', { seed: 31, centre: 50 * 1.35 }), baselines, '--reason', 'Re-measured')
  check(same.status === 0 && readPromotion(baselineDir).accepted_regression === null, 'promoting what does not regress needs no reason, and records none')
}

fs.rmSync(work, { recursive: true, force: true })
console.log(failures ? `\n${failures} did not behave` : '\nthe comparison, the reports and the promotion behave as designed')
process.exit(failures ? 1 : 0)
