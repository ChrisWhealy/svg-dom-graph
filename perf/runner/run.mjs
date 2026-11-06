// Runs a workload manifest on Layer A (native), Layer B (wasm in Chrome) and, for the interaction workload, Layer C (real input
// in Chrome), and writes a result document for each run.
//
//   node perf/runner/run.mjs <manifest.toml> [--mode correctness|performance|endurance|all] [--layers a,b,c]
//                            [--repetitions N] [--out-dir DIR] [--no-build]
//
// Each repetition of Layer B is a separate Chrome process with its own profile, so repetitions do not share a page, a heap or
// a JIT. Every result is checked against `perf/schemas/result.schema.json` before it is written, and a result that does not
// satisfy it is an error and is not kept. The deterministic gates are printed at the end. Timing is never a gate here.
//
// It needs the schemas' dependencies (`npm ci` in perf/schemas), a Rust toolchain, wasm-pack, and a local Chrome.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { createRequire } from 'node:module'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from './lib/harness-page.mjs'
import { edgesOf, runInteraction } from './layer-c/interaction.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const requireFromSchemas = createRequire(path.join(root, 'perf/schemas/package.json'))
const Ajv = requireFromSchemas('ajv/dist/2020.js')
const addFormats = requireFromSchemas('ajv-formats')

/** The version of how this harness measures. A change that makes old figures incomparable raises it. */
const PROTOCOL_VERSION = '1'

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const parseArguments = argv => {
  const options = { layers: null, mode: 'all', build: true, outDir: null, repetitions: null, manifest: null }
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]
    if (arg === '--mode') options.mode = argv[++i]
    else if (arg === '--layers') options.layers = argv[++i].split(',')
    else if (arg === '--repetitions') options.repetitions = Number(argv[++i])
    else if (arg === '--out-dir') options.outDir = argv[++i]
    else if (arg === '--no-build') options.build = false
    else if (arg.startsWith('--')) throw new Error(`unknown option ${arg}`)
    else options.manifest = arg
  }
  if (!options.manifest) throw new Error('usage: node perf/runner/run.mjs <manifest.toml> [options]')
  return options
}

const run = (command, args, options = {}) => spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 512 * 1024 * 1024, ...options })
const first = (command, args) => run(command, args).stdout?.trim().split('\n')[0] ?? null
const must = (result, what) => {
  if (result.status !== 0) throw new Error(`${what} failed:\n${result.stderr || result.stdout}`)
  return result
}

// A seed can be larger than 2**53. The reviver keeps its exact text, so that it is written back as it was read.
const readExact = text => JSON.parse(text, (key, value, context) => (key === 'seed' ? JSON.rawJSON(context.source) : value))

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const options = parseArguments(process.argv.slice(2))
const manifestPath = path.resolve(options.manifest)
const manifestId = path.basename(manifestPath).replace(/\.toml$/, '')
const outDir = path.resolve(options.outDir ?? path.join(root, 'target/perf', manifestId))
fs.mkdirSync(outDir, { recursive: true })

if (options.build) must(run('cargo', ['build', '-q', '--release', '-p', 'perf-harness']), 'building perf-harness')
const binary = path.join(root, 'target/release/perf-harness')

const reportPath = path.join(outDir, 'workload-report.json')
must(run(binary, ['generate', manifestPath, '--id', manifestId, '--out', reportPath]), 'generating the workload report')
const report = readExact(fs.readFileSync(reportPath, 'utf8'))
const canonicalManifest = must(run(binary, ['canonical', manifestPath]), 'reading the manifest').stdout.trim()
const expected = JSON.stringify({ sha256: report.stream.sha256, mutations_sha256: report.stream.mutations_sha256 })
const manifest = report.manifest.canonical

// Layer C drives a scene with real input, which only the interaction workload has. It is on by default for that workload and
// refused for any other, and a layer that is asked for by name and cannot run says so and does not quietly do nothing.
const interactive = manifest.workload === 'interaction'
options.layers ??= interactive ? ['a', 'b', 'c'] : ['a', 'b']
if (options.layers.includes('c') && !interactive) {
  console.log(`skipping layer C: ${manifest.workload} has no input to drive, and only the interaction workload does`)
  options.layers = options.layers.filter(layer => layer !== 'c')
}
if (options.build && (options.layers.includes('b') || options.layers.includes('c'))) {
  must(run('wasm-pack', ['build', 'perf-harness-wasm', '--target', 'web', '--release', '--out-dir', 'pkg']), 'building perf-harness-wasm')
}
// The gestures are planned once, by the same engine that planned the scene, and every repetition plays the same plan.
const gesturePlan = options.layers.includes('c') ? JSON.parse(must(run(binary, ['gestures', manifestPath]), 'planning the gestures').stdout) : null

const modes = options.mode === 'all' ? manifest.execution.modes : [options.mode]
const repetitionsFor = mode => (mode === 'correctness' ? 1 : (options.repetitions ?? manifest.execution.repetitions))

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const chromePath = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const chromeVersion = first(chromePath, ['--version'])?.match(/(\d+(?:\.\d+)+)/)?.[1] ?? null
const crateVersion = fs.readFileSync(path.join(root, 'Cargo.toml'), 'utf8').match(/^version = "([^"]+)"/m)?.[1]
const git = {
  sha: first('git', ['rev-parse', 'HEAD']),
  dirty: run('git', ['status', '--porcelain']).stdout.trim().length > 0,
}
const environment = (kind, outcome, crossOriginIsolated) => ({
  kind,
  ...(kind === 'browser' ? { browser: 'Chrome', browser_version: chromeVersion } : {}),
  os: `${os.type()} ${os.release()}`,
  cpu: os.cpus()[0].model,
  memory_bytes: os.totalmem(),
  runner_class: process.env.PERF_RUNNER_CLASS ?? 'local',
  timer_resolution_us: outcome.timer_resolution_us,
  ...(kind === 'browser' ? { cross_origin_isolated: crossOriginIsolated } : {}),
  protocol_version: PROTOCOL_VERSION,
  rustc: first('rustc', ['--version']),
  wasm_pack: first('wasm-pack', ['--version']),
  crate_version: crateVersion,
  git_sha: git.sha,
  git_dirty: git.dirty,
  build_profile: 'release',
})

const ajv = new Ajv({ strict: true, strictTypes: false, strictRequired: false, allErrors: true })
addFormats(ajv)
for (const name of ['manifest', 'result']) ajv.addSchema(JSON.parse(fs.readFileSync(path.join(root, `perf/schemas/${name}.schema.json`), 'utf8')))
const validateResult = ajv.getSchema('urn:svg-dom-graph:perf:result:1')

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const runs = []
let position = 0

// The counting allocator's figures are not used until it has been shown to count. It is checked once, in a page of its own,
// and if it does not pass, every figure that comes from it is withheld and says why.
let allocatorVerified = null
if (options.layers.includes('b') && manifest.measurement.capture_wasm_live_allocation) {
  const page = await openHarnessPage()
  const verification = JSON.parse(await page.evaluate('window.perfHarness.verify_allocator()'))
  await page.close()
  fs.writeFileSync(path.join(outDir, 'allocator-verification.json'), JSON.stringify(verification, null, 2) + '\n')
  allocatorVerified = verification.pass
  console.log(allocatorVerified ? 'the counting allocator is verified' : 'the counting allocator is NOT verified: its figures are withheld')
}

const withholdAllocatorFigures = outcome => {
  // A gate that rests on the allocator's figures cannot be judged without them, and not judging is never a pass.
  outcome.gates = (outcome.gates ?? []).map(gate =>
    gate.name.includes('live_bytes') ? { ...gate, status: 'inconclusive', detail: 'the counting allocator did not pass its verification' } : gate,
  )
  outcome.metrics = outcome.metrics.map(metric =>
    typeof metric.mechanism === 'string' && metric.mechanism.includes('counting allocator')
      ? { name: metric.name, layer: metric.layer, status: 'unavailable', reason: 'the counting allocator did not pass its verification' }
      : metric,
  )
}

const keep = (layer, mode, repetition, outcome, crossOriginIsolated) => {
  const runId = `${manifestId}-${layer}-${mode}-r${repetition}-${new Date().toISOString().replace(/[-:.]/g, '').slice(0, 15)}`
  const keepsSamples = mode === 'performance' || mode === 'endurance'
  const samplesFile = path.join(outDir, `${runId}.samples.ndjson`)
  if (keepsSamples) {
    const lines = Object.entries(outcome.samples ?? {}).map(([metric, values]) => JSON.stringify({ metric, values }))
    fs.writeFileSync(samplesFile, lines.map(line => line + '\n').join(''))
  }
  const document = {
    schema_version: 1,
    run_id: runId,
    started_at: new Date().toISOString(),
    repetition,
    sequence_position: position++,
    environment: environment(layer === 'A' ? 'native' : 'browser', outcome, crossOriginIsolated),
    manifest: {
      id: manifestId,
      sha256: report.manifest.sha256,
      operation_stream_scope: outcome.stream?.scope ?? 'mutations',
      operation_stream_sha256: outcome.stream?.sha256 ?? '0'.repeat(64),
      canonical: manifest,
    },
    mode,
    status: outcome.status,
    ...(outcome.error ? { error: outcome.error } : {}),
    metrics: outcome.metrics,
    checkpoints_passed: outcome.checkpoints_passed,
    checkpoints_failed: outcome.checkpoints_failed,
    ...(outcome.gates?.length ? { gates: outcome.gates } : {}),
    ...(keepsSamples ? { raw_samples: path.relative(root, samplesFile) } : {}),
  }
  // The seed is validated as the number it is. Only the file keeps its exact text.
  const asData = JSON.parse(JSON.stringify(document))
  if (!validateResult(asData)) {
    throw new Error(`the result of ${runId} does not satisfy the result schema: ${ajv.errorsText(validateResult.errors)}`)
  }
  fs.writeFileSync(path.join(outDir, `${runId}.json`), JSON.stringify(document, null, 2) + '\n')
  runs.push({ layer, mode, repetition, runId, outcome })
}

for (const mode of modes) {
  if (mode === 'recovery') {
    console.log('skipping recovery: that runner arrives in Phase 4')
    continue
  }
  for (let repetition = 0; repetition < repetitionsFor(mode); repetition++) {
    if (options.layers.includes('a') && (mode === 'correctness' || mode === 'performance')) {
      const file = path.join(outDir, `layer-a-${mode}-${repetition}.outcome.json`)
      must(run(binary, ['native', manifestPath, '--mode', mode, '--out', file]), `the native ${mode} run`)
      keep('A', mode, repetition, JSON.parse(fs.readFileSync(file, 'utf8')), null)
      fs.rmSync(file)
    }
    if (options.layers.includes('c') && (mode === 'correctness' || mode === 'performance')) {
      const outcome = await runInteraction({ binary, mode, canonical: canonicalManifest, plan: gesturePlan, report, edges: edgesOf(binary, manifestPath) })
      keep('C', mode, repetition, outcome, outcome.crossOriginIsolated ?? null)
    }
    if (options.layers.includes('b')) {
      const page = await openHarnessPage()
      try {
        const outcome = JSON.parse(await page.evaluate(`window.perfHarness.run_workload(${JSON.stringify(canonicalManifest)}, ${JSON.stringify(mode)}, ${JSON.stringify(expected)})`))
        if (allocatorVerified === false) withholdAllocatorFigures(outcome)
        keep('B', mode, repetition, outcome, page.crossOriginIsolated)
      } finally {
        await page.close()
      }
    }
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const metricValue = (outcome, name) => outcome.metrics.find(m => m.name === name && m.status === 'measured')?.value
// How much the typical figure moves from one repetition to the next. This is a description of this machine and this run, and
// not a comparison. A verdict needs the comparison report, which takes the repetitions as its unit.
const spread = []
for (const group of Object.values(Object.groupBy(runs, run => `${run.layer} ${run.mode}`))) {
  if (group.length < 2) continue
  const names = new Set(group.flatMap(run => run.outcome.metrics.filter(m => m.status === 'measured' && m.summary).map(m => m.name)))
  for (const name of names) {
    const medians = group.map(run => run.outcome.metrics.find(m => m.name === name && m.summary)?.summary.p50).filter(v => v !== undefined)
    if (medians.length < 2) continue
    const low = Math.min(...medians)
    const high = Math.max(...medians)
    const coarse = group.some(run => run.outcome.metrics.find(m => m.name === name)?.resolution_limited === true)
    const unit = group[0].outcome.metrics.find(m => m.name === name)?.unit ?? 'us'
    spread.push({ group: `${group[0].layer} ${group[0].mode}`, name, unit, repetitions: medians.length, low, high, coarse, percent: low > 0 ? ((high - low) / low) * 100 : null })
  }
}

let gatesFailed = 0
const gate = (ok, text) => {
  if (!ok) gatesFailed++
  console.log(`    ${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

console.log(`\nresults in ${path.relative(root, outDir)}\n`)
for (const { layer, mode, repetition, runId, outcome } of runs) {
  const timing = outcome.metrics.filter(m => m.status === 'measured' && m.summary).map(m => `${m.name} p50 ${m.summary.p50.toFixed(2)} ${m.unit}`)
  console.log(`${layer} ${mode} #${repetition}: ${outcome.status}, ${outcome.checkpoints_passed} checkpoints passed, ${outcome.checkpoints_failed.length} failed${timing.length ? `\n    ${timing.join('\n    ')}` : ''}`)
  gate(outcome.status === 'ok', `the run is valid (${outcome.status}${outcome.error ? `: ${outcome.error}` : ''})`)
  gate(outcome.stream?.matches === true, `it executed the intended ${outcome.stream?.scope} stream (${outcome.stream?.operations} operations)`)
  for (const failure of outcome.checkpoints_failed.slice(0, 5)) console.log(`      ${failure.section}: ${failure.detail}`)
  for (const gateResult of outcome.gates ?? []) gate(gateResult.status === 'pass', `${gateResult.name}: ${gateResult.status}, ${gateResult.detail}`)
  if (layer === 'B') {
    for (const name of ['dom_elements', 'listeners']) {
      const delta = metricValue(outcome, `resource.${name}.delta_after_teardown`)
      if (delta !== undefined) gate(delta === 0, `${name} after teardown equal those before the build (delta ${delta})`)
    }
  }
}
if (spread.length) {
  console.log('\nthe median of each timing in every repetition (the spread is highest over lowest):')
  const show = value => (value >= 100 ? value.toFixed(0) : value.toPrecision(3))
  for (const row of spread) {
    const coarse = row.coarse ? ' (under 50 timer steps, so coarse)' : ''
    console.log(`  ${row.group} ${row.name}: ${show(row.low)} to ${show(row.high)} ${row.unit} over ${row.repetitions} repetitions, ${row.percent === null ? 'n/a' : `${row.percent.toFixed(0)}%`} apart${coarse}`)
  }
}
console.log(gatesFailed ? `\n${gatesFailed} deterministic gate(s) failed` : '\nevery deterministic gate passed')
process.exit(gatesFailed ? 1 : 0)
