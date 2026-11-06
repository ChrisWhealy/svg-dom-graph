// The profile that runs on every commit (design section 8): W1 to W3, and the interaction workload for Layer C, at small size.
//
//   node perf/runner/perf-smoke.mjs [--out-dir DIR] [--manifests a,b] [--repetitions N] [--no-build] [--enforce-statistical]
//
// It runs each workload with the orchestrator, which checks the deterministic gates: every executed checkpoint, the intended
// operation stream, the DOM and listener counts after a teardown, and the validity of the result against its schema. A failure
// of any of those fails this command.
//
// Then, for each layer that has a baseline for this workload in this environment, it compares the runs with the baseline and
// writes a comparison report. The comparison only reports. A timing that is uncertain, or even a clear regression, does not fail
// this command, since the baseline needs 10 runs on one runner class before its thresholds mean anything (section 8.1). With
// the three repetitions of a smoke run the report is labelled diagnostic and gives no verdict at all (section 6).
// `--enforce-statistical` is for when that point has been reached: it then also fails on a regression, but only from a baseline
// that has the runs to block with.
//
// A layer with no baseline in this environment is not an error. It says so, and says how to make one.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import {
  RUNS_TO_BLOCK,
  baselineDirFor,
  compareCollections,
  environmentKey,
  regressions,
  resultFiles,
  root,
  summarise
} from '../lib/baselines.mjs'

const SMOKE = [
  'w1-scene-construction-smoke',
  'w2-connector-rerouting-smoke',
  'w3-scene-lifecycle-smoke',
  'w4-interaction-small'
]

const parse = argv => {
  const options = { outDir: path.join(root, 'target/perf-smoke'), manifests: SMOKE, repetitions: null, build: true, enforce: false }
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]
    if (arg === '--out-dir') options.outDir = path.resolve(argv[++i])
    else if (arg === '--manifests') options.manifests = argv[++i].split(',')
    else if (arg === '--repetitions') options.repetitions = argv[++i]
    else if (arg === '--no-build') options.build = false
    else if (arg === '--enforce-statistical') options.enforce = true
    else throw new Error(`unknown option ${arg}`)
  }
  return options
}

/** The layers that have performance results in a directory. */
const layersIn = dir => {
  const layers = new Set()
  for (const file of resultFiles(dir)) {
    const doc = JSON.parse(fs.readFileSync(file, 'utf8'))
    if (doc.mode !== 'performance') continue
    const metric = doc.metrics.find(m => m.layer)
    if (metric) layers.add(metric.layer)
  }
  return [...layers].sort()
}

const options = parse(process.argv.slice(2))
fs.mkdirSync(options.outDir, { recursive: true })
const summary = []
let deterministicFailures = 0
let statisticalFailures = 0
const compared = []

for (const [index, id] of options.manifests.entries()) {
  const manifest = path.join(root, 'perf/manifests', `${id}.toml`)
  const outDir = path.join(options.outDir, id)
  fs.rmSync(outDir, { recursive: true, force: true })
  const args = [path.join(root, 'perf/runner/run.mjs'), manifest, '--out-dir', outDir]
  if (options.repetitions) args.push('--repetitions', options.repetitions)
  if (!options.build || index > 0) args.push('--no-build')
  console.log(`\n=== ${id}`)
  const ran = spawnSync('node', args, { cwd: root, stdio: 'inherit' })
  if (ran.status !== 0) {
    deterministicFailures++
    summary.push(`### ${id}`, '', `**A deterministic gate failed** (exit ${ran.status}). The log has the gate.`, '')
    continue
  }

  summary.push(`### ${id}`, '')
  for (const layer of layersIn(outDir)) {
    const files = resultFiles(outDir).filter(f => {
      const doc = JSON.parse(fs.readFileSync(f, 'utf8'))
      return doc.mode === 'performance' && (doc.metrics.some(m => m.layer === layer) || doc.run_id.includes(`-${layer}-`))
    })
    const key = environmentKey(files[0])
    const baseline = baselineDirFor(id, layer, key)
    if (!fs.existsSync(path.join(baseline, 'promotion.json'))) {
      const note = `Layer ${layer}: no baseline for ${id} in the environment \`${key}\`, so nothing was compared. To make one: \`node perf/runner/promote-baseline.mjs --from ${path.relative(root, outDir)} --layer ${layer} --reason "..."\``
      console.log(note.replaceAll('`', ''))
      summary.push(note, '')
      continue
    }
    const out = path.join(outDir, `comparison-${layer}.json`)
    try {
      const report = compareCollections({ baselineDir: baseline, candidateDirs: [outDir], layer, mode: 'performance', out })
      const lines = summarise(report, `${id}, layer ${layer}`)
      console.log(lines.join('\n').replaceAll('**', '').replaceAll('`', ''))
      summary.push(...lines)
      compared.push({ id, layer, out: path.relative(root, out), gating: report.gating.statistical_gates })
      if (options.enforce && report.gating.statistical_gates === 'blocking' && regressions(report).length > 0) statisticalFailures++
    } catch (error) {
      // A comparison that cannot be made is not a slow commit. It is said, and it does not fail the run.
      const note = `Layer ${layer}: the comparison could not be made: ${error.message.split('\n')[0]}`
      console.log(note)
      summary.push(note, '')
    }
  }
}

const verdict = deterministicFailures
  ? `${deterministicFailures} workload(s) failed a deterministic gate`
  : statisticalFailures
    ? `${statisticalFailures} comparison(s) regressed against a baseline that blocks`
    : `every deterministic gate passed${compared.length ? ` and ${compared.length} comparison(s) were reported` : ''}`
summary.unshift(`## perf-smoke: ${verdict}`, '')
fs.writeFileSync(path.join(options.outDir, 'perf-smoke-summary.md'), summary.join('\n') + '\n')
fs.writeFileSync(path.join(options.outDir, 'perf-smoke-comparisons.json'), JSON.stringify(compared, null, 2) + '\n')
if (process.env.GITHUB_STEP_SUMMARY) fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary.join('\n') + '\n')
console.log(`\n${verdict}`)
if (!options.enforce && statisticalFailures === 0) console.log(`(statistical verdicts report only until a baseline has ${RUNS_TO_BLOCK} runs)`)
process.exit(deterministicFailures || statisticalFailures ? 1 : 0)
