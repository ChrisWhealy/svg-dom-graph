// Says whether the committed baselines cover the runner that this runs on.
//
//   node perf/runner/baseline-status.mjs [--out FILE]
//
// A baseline is for one environment: the runner class, the operating system and kernel, the processor, the full version of Chrome and the
// measurement protocol. When the runner image moves to another Chrome or kernel, or GitHub gives a job another processor, no baseline matches, and
// perf-smoke can only say so. This asks the question on purpose and cheaply: it makes three runs of the smallest workload, reads the environment
// that they were measured in, and looks for an approved baseline of it. The runs are not compared with anything and are not kept.
//
// It writes a record, `target/baseline-status/status.json` by default, and always exits 0: whether a baseline is missing is for
// `report-baseline-status.mjs` to decide, over the records of every job. It needs the release builds that perf-smoke needs, and a local Chrome.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { baselineDirFor, baselinesDir, environmentKey, readPromotion, resultFiles, root } from '../lib/baselines.mjs'
import { isQualified } from '../lib/qualification.mjs'

const MANIFEST = 'w2-connector-rerouting-smoke'
const at = process.argv.indexOf('--out')
const out = path.resolve(at < 0 ? path.join(root, 'target/baseline-status/status.json') : process.argv[at + 1])
const work = path.join(path.dirname(out), 'runs')

fs.rmSync(work, { recursive: true, force: true })
fs.mkdirSync(work, { recursive: true })

const ran = spawnSync(
  'node',
  [
    path.join(root, 'perf/runner/run.mjs'),
    path.join(root, `perf/manifests/${MANIFEST}.toml`),
    '--mode',
    'performance',
    '--layers',
    'b',
    '--no-build',
    '--repetitions',
    '3',
    '--out-dir',
    work
  ],
  { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28 }
)
const record = {
  checked_at: new Date().toISOString(),
  manifest: MANIFEST,
  layer: 'B',
  environment: null,
  baseline: null,
  status: null
}
const files = ran.status === 0 ? resultFiles(work) : []

if (files.length === 0) {
  record.status = 'could_not_measure'
  record.detail = (ran.stderr || ran.stdout || '').split('\n').filter(Boolean).slice(-5).join('\n')
} else {
  record.environment = environmentKey(files[0])
  const dir = baselineDirFor(MANIFEST, 'B', record.environment)
  const promotion = fs.existsSync(dir) ? readPromotion(dir) : null
  if (!promotion) record.status = 'no_baseline'
  else {
    const approved = isQualified(dir)
    record.baseline = { dir: path.relative(baselinesDir, dir), runs: promotion.runs.length, approved }
    record.status = approved ? 'covered' : 'not_approved'
  }
}

fs.mkdirSync(path.dirname(out), { recursive: true })
fs.writeFileSync(out, JSON.stringify(record, null, 2) + '\n')
console.log(`${record.status}: ${record.environment ?? record.detail}`)

if (process.env.GITHUB_STEP_SUMMARY)
  fs.appendFileSync(
    process.env.GITHUB_STEP_SUMMARY,
    `- baseline status \`${record.status}\`: \`${record.environment ?? 'not measured'}\`\n`
  )
