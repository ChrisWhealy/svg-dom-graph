// Runs a workload manifest on Layer A (native), Layer B (wasm in Chrome) and, for the interaction workload, Layer C (real input
// in Chrome), and writes a result document for each run.
//
//   node perf/runner/run.mjs <manifest.toml> [--mode correctness|performance|endurance|all] [--layers a,b,c]
//                            [--repetitions N] [--out-dir DIR] [--no-build]
//                            [--inject-slowdown-us N] [--inject-leak none|bytes|closure|library_watch]
//                            [--play] [--inject-late-pause]
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
import { runRecoveryC } from './layer-c/recovery.mjs'
import { runSimulationC, runSimulationCCorrectness } from './layer-c/service.mjs'
import { oracleFor } from './service-oracle.mjs'
import { buildTrace } from './service-trace.mjs'
import { createHash } from 'node:crypto'

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
    else if (arg === '--layers') {
      options.layers = argv[++i].split(',')
      options.layersAsked = true
    } else if (arg === '--repetitions') options.repetitions = Number(argv[++i])
    else if (arg === '--out-dir') options.outDir = argv[++i]
    else if (arg === '--no-build') options.build = false
    else if (arg === '--inject-slowdown-us') options.slowdownUs = Number(argv[++i])
    else if (arg === '--inject-leak') options.leak = argv[++i]
    else if (arg === '--play') options.play = true
    else if (arg === '--inject-late-pause') options.lateDuringPlay = true
    else if (arg.startsWith('--')) throw new Error(`unknown option ${arg}`)
    else options.manifest = arg
  }
  if (!options.manifest) throw new Error('usage: node perf/runner/run.mjs <manifest.toml> [options]')
  return options
}

const run = (command, args, options = {}) =>
  spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 512 * 1024 * 1024, ...options })
const first = (command, args) => run(command, args).stdout?.trim().split('\n')[0] ?? null
const must = (result, what) => {
  if (result.status !== 0) throw new Error(`${what} failed:\n${result.stderr || result.stdout}`)
  return result
}

// A seed can be larger than 2**53. The reviver keeps its exact text, so that it is written back as it was read.
const readExact = text =>
  JSON.parse(text, (key, value, context) => (key === 'seed' ? JSON.rawJSON(context.source) : value))

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const options = parseArguments(process.argv.slice(2))
const manifestPath = path.resolve(options.manifest)
const manifestId = path.basename(manifestPath).replace(/\.toml$/, '')
const outDir = path.resolve(options.outDir ?? path.join(root, 'target/perf', manifestId))
fs.mkdirSync(outDir, { recursive: true })

if (options.build) must(run('cargo', ['build', '-q', '--release', '-p', 'perf-harness']), 'building perf-harness')
const binary = path.join(root, 'target/release/perf-harness')

const reportPath = path.join(outDir, 'workload-report.json')
const canonicalManifest = must(run(binary, ['canonical', manifestPath]), 'reading the manifest').stdout.trim()
// The recovery workload aimed at the service simulator has no graph and no operation stream to generate. Its stream is the independent
// oracle's hash of the simulator's state at every slice of the scenario, which is what its executor compares the controller with.
const service = JSON.parse(canonicalManifest).recovery?.target === 'service_simulation'
const serviceOracle = service ? oracleFor(JSON.parse(canonicalManifest).recovery.scenario) : null
const sha = text => createHash('sha256').update(text).digest('hex')
// Workload W6 is the simulator run cleanly. Its trace is worked out before anything is timed, by the independent engine, and its operation
// stream is that engine's hash of the state at every slice: the whole chain for a correctness run, and the prefix that the windows consume for
// a performance run, which does not execute the rest.
const simulating = JSON.parse(canonicalManifest).workload === 'service_simulation'
// A seed can be larger than 2**53, and then a number loses it. Its decimal text is kept as it is in the canonical manifest, and is turned into a
// BigInt only at the independent engine's boundary, so the oracle and the native engine run the same seed.
const seedText = /"seed":(\d+)/.exec(canonicalManifest)?.[1]
const simulationTrace = simulating ? buildTrace(JSON.parse(canonicalManifest).simulation, seedText) : null
if (!service && !simulating)
  must(
    run(binary, ['generate', manifestPath, '--id', manifestId, '--out', reportPath]),
    'generating the workload report'
  )
const report = simulating
  ? {
      manifest: { sha256: sha(canonicalManifest), canonical: readExact(canonicalManifest) },
      stream: {
        sha256: simulationTrace.hashes.checkpoint_chain_sha256,
        mutations_sha256: sha(simulationTrace.checkpoints.slice(0, 537).join('\n'))
      }
    }
  : service
    ? {
        manifest: { sha256: sha(canonicalManifest), canonical: JSON.parse(canonicalManifest) },
        stream: { sha256: sha(serviceOracle.checkpoints.join('\n')), mutations_sha256: sha('') }
      }
    : readExact(fs.readFileSync(reportPath, 'utf8'))
const expected = JSON.stringify({ sha256: report.stream.sha256, mutations_sha256: report.stream.mutations_sha256 })
const manifest = report.manifest.canonical

// Layer C drives a scene with real input, which only the interaction workload has. It is on by default for that workload and
// refused for any other, and a layer that is asked for by name and cannot run says so and does not quietly do nothing.
const interactive = manifest.workload === 'interaction'
const recovery = manifest.workload === 'recovery'
options.layers ??= service ? ['b'] : interactive || recovery || simulating ? ['a', 'b', 'c'] : ['a', 'b']
// A simulator manifest can say which layers it is for. A scaled streaming check names Layer A alone, and then no browser is ever started for
// it: the layers are cut down to those that it names, and a layer that was asked for by name and is not one of them is refused.
if (simulating && manifest.simulation.layers) {
  const allowed = manifest.simulation.layers
  const refused = options.layers.filter(layer => !allowed.includes(layer))
  if (options.layersAsked && refused.length > 0)
    throw new Error(`this manifest is for layer ${allowed.join(', ')} alone, and ${refused.join(', ')} was asked for`)
  if (refused.length > 0)
    console.log(`skipping layer ${refused.join(', ')}: the manifest is for layer ${allowed.join(', ')} alone`)
  options.layers = options.layers.filter(layer => allowed.includes(layer))
}
if (service && options.layers.some(layer => layer !== 'b')) {
  console.log(
    'skipping layers a and c: the service target is broken in the page, by its own executor, and has neither native work nor real input'
  )
  options.layers = ['b']
}
if (options.layers.includes('c') && !interactive && !recovery && !simulating) {
  console.log(
    `skipping layer C: ${manifest.workload} has no input to drive, and only the interaction and recovery workloads do`
  )
  options.layers = options.layers.filter(layer => layer !== 'c')
}
if (options.build && (options.layers.includes('b') || options.layers.includes('c'))) {
  must(
    run('wasm-pack', ['build', 'perf-harness-wasm', '--target', 'web', '--release', '--out-dir', 'pkg']),
    'building perf-harness-wasm'
  )
}
// The gestures are planned once, by the same engine that planned the scene, and every repetition plays the same plan.
const gesturePlan =
  options.layers.includes('c') && interactive
    ? JSON.parse(must(run(binary, ['gestures', manifestPath]), 'planning the gestures').stdout)
    : null

// A candidate changed on purpose, to prove that the comparison and the gates notice a change. It only reaches Layer B, where
// the library is timed, and the result says so, so that it can never be taken for a measurement of the library.
const injected = options.slowdownUs > 0 || (options.leak && options.leak !== 'none')
const injection = injected ? { slowdown_us: options.slowdownUs ?? 0, leak: options.leak ?? 'none' } : null
if (injected && options.layers.some(layer => layer !== 'b'))
  throw new Error('an injected candidate runs in Layer B only: use --layers b')

// A soak of a recovery workload is given a time, and ends at it with the cycles that fit (see runEndurance), since no count fills a time there.
const stopAfter = process.env.PERF_ENDURANCE_STOP_AFTER_SECONDS
  ? { stopAfterMs: Number(process.env.PERF_ENDURANCE_STOP_AFTER_SECONDS) * 1000 }
  : {}

const modes = options.mode === 'all' ? manifest.execution.modes : [options.mode]
const repetitionsFor = mode => (mode === 'correctness' ? 1 : (options.repetitions ?? manifest.execution.repetitions))

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const chromePath = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
const chromeVersion = first(chromePath, ['--version'])?.match(/(\d+(?:\.\d+)+)/)?.[1] ?? null
const crateVersion = fs.readFileSync(path.join(root, 'Cargo.toml'), 'utf8').match(/^version = "([^"]+)"/m)?.[1]
const git = {
  sha: first('git', ['rev-parse', 'HEAD']),
  dirty: run('git', ['status', '--porcelain']).stdout.trim().length > 0
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
  build_profile: 'release'
})

const ajv = new Ajv({ strict: true, strictTypes: false, strictRequired: false, allErrors: true })
addFormats(ajv)
for (const name of ['manifest', 'result'])
  ajv.addSchema(JSON.parse(fs.readFileSync(path.join(root, `perf/schemas/${name}.schema.json`), 'utf8')))
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
  console.log(
    allocatorVerified
      ? 'the counting allocator is verified'
      : 'the counting allocator is NOT verified: its figures are withheld'
  )
}

const withholdAllocatorFigures = outcome => {
  // A gate that rests on the allocator's figures cannot be judged without them, and not judging is never a pass.
  outcome.gates = (outcome.gates ?? []).map(gate =>
    gate.name.includes('live_bytes')
      ? { ...gate, status: 'inconclusive', detail: 'the counting allocator did not pass its verification' }
      : gate
  )
  outcome.metrics = outcome.metrics.map(metric =>
    typeof metric.mechanism === 'string' && metric.mechanism.includes('counting allocator')
      ? {
          name: metric.name,
          layer: metric.layer,
          status: 'unavailable',
          reason: 'the counting allocator did not pass its verification'
        }
      : metric
  )
}

const keep = (layer, mode, repetition, outcome, crossOriginIsolated) => {
  // A valid performance run that timed nothing has no figure to keep, and the schema asks a performance result for its samples.
  // This is a layer that has nothing to time in this workload, such as Layer B for the interaction workload, whose timings are
  // taken with real input in Layer C. It is said, and it is not a result.
  if (mode === 'performance' && outcome.status === 'ok' && Object.keys(outcome.samples ?? {}).length === 0) {
    if (repetition === 0)
      console.log(`layer ${layer} timed nothing in performance mode of ${manifest.workload}, so no result is kept`)
    return
  }
  const runId = `${manifestId}-${layer}-${mode}-r${repetition}-${new Date().toISOString().replace(/[-:.]/g, '').slice(0, 15)}`
  const keepsSamples = mode === 'performance' || mode === 'endurance' || mode === 'recovery'
  const samplesFile = path.join(outDir, `${runId}.samples.ndjson`)
  if (keepsSamples) {
    const lines = Object.entries(outcome.samples ?? {}).map(([metric, values]) => JSON.stringify({ metric, values }))
    fs.writeFileSync(samplesFile, lines.map(line => line + '\n').join(''))
  }
  const casesFile = path.join(outDir, `${runId}.cases.json`)
  if (outcome.cases?.length) fs.writeFileSync(casesFile, JSON.stringify(outcome.cases, null, 1) + '\n')
  // The stream a recovery run built is held to the digest of the report, as the other layers' are.
  if (outcome.stream && outcome.stream.expected_sha256 === null) {
    outcome.stream.expected_sha256 = report.stream.sha256
    outcome.stream.matches = outcome.stream.sha256 === report.stream.sha256
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
      canonical: manifest
    },
    mode,
    status: outcome.status,
    ...(outcome.error ? { error: outcome.error } : {}),
    ...(injection ? { injection } : {}),
    metrics: outcome.metrics,
    checkpoints_passed: outcome.checkpoints_passed,
    checkpoints_failed: outcome.checkpoints_failed,
    ...(outcome.gates?.length ? { gates: outcome.gates } : {}),
    ...(keepsSamples && Object.keys(outcome.samples ?? {}).length
      ? { raw_samples: path.relative(root, samplesFile) }
      : {}),
    ...(outcome.findings?.length ? { findings: outcome.findings } : {}),
    ...(outcome.cases?.length ? { cases: path.relative(root, casesFile) } : {}),
    ...(outcome.simulation ? { simulation: outcome.simulation } : {})
  }
  // The seed is validated as the number it is. Only the file keeps its exact text.
  const asData = JSON.parse(JSON.stringify(document))
  if (!validateResult(asData)) {
    throw new Error(
      `the result of ${runId} does not satisfy the result schema: ${ajv.errorsText(validateResult.errors)}`
    )
  }
  fs.writeFileSync(path.join(outDir, `${runId}.json`), JSON.stringify(document, null, 2) + '\n')
  runs.push({ layer, mode, repetition, runId, outcome })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Workload W6. The trace was worked out before anything runs, and a trace that may not be timed is refused here: every layer and mode that
// was asked for gets a result that says so, with every figure that was worked out, and no time series. A refusal is not a pass.
const simulationField = simulating
  ? {
      generator_revision: simulationTrace.qualification.generator_revision,
      // The exact text of the seed, which a number would round. The file keeps it as it is, and the schema reads it as the number it is.
      seed: JSON.rawJSON(seedText),
      qualification: { ...simulationTrace.qualification, seed: JSON.rawJSON(seedText) },
      counts: simulationTrace.counts,
      hashes: simulationTrace.hashes
    }
  : null
const simulationTracePath = path.join(outDir, 'service-trace.json')
if (simulating) fs.writeFileSync(simulationTracePath, JSON.stringify(simulationTrace.expected))
/** The smallest step that this process's clock makes, for a result that timed nothing and still has to say what its clock was. */
const nodeResolutionUs = () => {
  let smallest = Infinity
  let last = performance.now()
  for (let probe = 0; probe < 200000; probe++) {
    const at = performance.now()
    const step = (at - last) * 1000
    if (step > 0 && step < smallest) smallest = step
    last = at
  }
  return Number.isFinite(smallest) ? smallest : 1
}

const runSimulation = async (mode, repetition) => {
  if (options.slowdownUs > 0 || (options.leak && options.leak !== 'none'))
    throw new Error('an injected candidate is for the graph workloads: the simulator has none')
  const attach = outcome => ({ ...outcome, simulation: simulationField })
  // Only a timing needs a trace that qualifies. A trace that is too short or too uneven for a measurement is still a trace that the engine and
  // the controller can be checked against, so correctness and endurance run on it, with the qualification recorded in their results.
  if (!simulationTrace.qualification.qualified && mode === 'performance') {
    // One result for each mode, and no layer ran: the refusal is decided here, from the independent engine's trace, before any browser or
    // native runner is started, so it is a native result and says so. A mode that was asked for cannot be taken for having passed.
    keep(
      'A',
      mode,
      repetition,
      attach({
        status: 'refused',
        error: `the trace of seed ${seedText} is refused before any timing, and no layer was run: ${simulationTrace.qualification.reasons.join('; ')}`,
        metrics: [],
        gates: [],
        samples: {},
        checkpoints_passed: 0,
        checkpoints_failed: [],
        stream: null,
        timer_resolution_us: nodeResolutionUs()
      }),
      null
    )
    return
  }
  if (options.layers.includes('a') && (mode === 'correctness' || mode === 'performance')) {
    const file = path.join(outDir, `layer-a-${mode}-${repetition}.outcome.json`)
    must(
      run(binary, ['native', manifestPath, '--mode', mode, '--trace', simulationTracePath, '--out', file]),
      `the native ${mode} run`
    )
    keep('A', mode, repetition, attach(JSON.parse(fs.readFileSync(file, 'utf8'))), null)
    fs.rmSync(file)
  }
  if (options.layers.includes('b')) {
    const page = await openHarnessPage()
    try {
      const outcome = JSON.parse(
        await page.evaluate(
          `window.perfHarness.sim_run(${JSON.stringify(mode)}, ${JSON.stringify(canonicalManifest)}, ${JSON.stringify(JSON.stringify(simulationTrace.expected))})`
        )
      )
      if (allocatorVerified === false) withholdAllocatorFigures(outcome)
      keep('B', mode, repetition, attach(outcome), page.crossOriginIsolated)
    } finally {
      await page.close()
    }
  }
  if (options.layers.includes('c') && mode === 'correctness') {
    const outcome = await runSimulationCCorrectness({ manifest, seed: seedText, trace: simulationTrace.expected })
    keep('C', mode, repetition, attach(outcome), outcome.crossOriginIsolated ?? null)
  }
  if (options.layers.includes('c') && mode === 'performance') {
    const outcome = await runSimulationC({
      binary,
      manifest,
      seed: seedText,
      trace: simulationTrace.expected,
      series: options.play ? ['next', 'play'] : ['next'],
      injectLatePause: options.lateDuringPlay === true
    })
    keep('C', mode, repetition, attach(outcome), outcome.crossOriginIsolated ?? null)
  }
}

for (const mode of modes) {
  if (simulating) {
    for (let repetition = 0; repetition < repetitionsFor(mode); repetition++) await runSimulation(mode, repetition)
    continue
  }
  if (mode === 'recovery' && !recovery) {
    console.log(`skipping recovery: ${manifest.workload} has no fault points, and only the recovery workload does`)
    continue
  }
  for (let repetition = 0; repetition < repetitionsFor(mode); repetition++) {
    // Layer A times the library's own DOM-free geometry, which only the rerouting workload has. In another workload's performance
    // mode it has nothing to time, and a run with no figure is not a result.
    const nativeTimesNothing = mode === 'performance' && manifest.workload !== 'connector_rerouting'
    if (nativeTimesNothing && repetition === 0 && options.layers.includes('a'))
      console.log(`skipping layer A in performance mode: ${manifest.workload} has no DOM-free work for it to time`)
    if (options.layers.includes('a') && !nativeTimesNothing && (mode === 'correctness' || mode === 'performance')) {
      const file = path.join(outDir, `layer-a-${mode}-${repetition}.outcome.json`)
      must(run(binary, ['native', manifestPath, '--mode', mode, '--out', file]), `the native ${mode} run`)
      keep('A', mode, repetition, JSON.parse(fs.readFileSync(file, 'utf8')), null)
      fs.rmSync(file)
    }
    if (service) {
      // Correctness and performance of the simulator are workload W6's, and this target is only broken on purpose: its correctness is
      // the gate that its recovery run makes, that the controller's committed slice is the oracle's.
      if (mode !== 'recovery' && mode !== 'endurance') {
        if (repetition === 0)
          console.log(`skipping ${mode}: the service target is broken on purpose, in recovery and endurance`)
        continue
      }
      const page = await openHarnessPage({ faults: true })
      try {
        const call = mode === 'recovery' ? 'service_recovery_run' : 'service_recovery_endurance'
        const outcome = JSON.parse(
          await page.evaluate(
            `window.perfHarness.${call}(${JSON.stringify(canonicalManifest)}, ${JSON.stringify(JSON.stringify({ oracle: serviceOracle }))})`
          )
        )
        outcome.stream = {
          scope: 'full',
          operations: serviceOracle.slices,
          sha256: report.stream.sha256,
          expected_sha256: null,
          matches: null
        }
        if (allocatorVerified === false) withholdAllocatorFigures(outcome)
        keep('B', mode, repetition, outcome, page.crossOriginIsolated)
      } finally {
        await page.close()
      }
      continue
    }
    if (mode === 'endurance' && recovery) {
      if (options.layers.includes('b')) {
        const page = await openHarnessPage({ faults: true })
        try {
          const outcome = JSON.parse(
            await page.evaluate(
              `window.perfHarness.recovery_endurance(${JSON.stringify(canonicalManifest)}, ${JSON.stringify(JSON.stringify(stopAfter))})`
            )
          )
          if (allocatorVerified === false) withholdAllocatorFigures(outcome)
          keep('B', 'endurance', repetition, outcome, page.crossOriginIsolated)
        } finally {
          await page.close()
        }
      }
      if (options.layers.includes('c')) {
        const outcome = await runRecoveryC({
          binary,
          canonical: canonicalManifest,
          manifest,
          report,
          endurance: {
            warmup: manifest.execution.warmup_iterations,
            measured: manifest.execution.measurement_iterations
          }
        })
        keep('C', 'endurance', repetition, outcome, outcome.crossOriginIsolated ?? null)
      }
      continue
    }
    if (options.layers.includes('c') && (mode === 'correctness' || mode === 'performance')) {
      const outcome = await runInteraction({
        binary,
        mode,
        canonical: canonicalManifest,
        plan: gesturePlan,
        report,
        edges: edgesOf(binary, manifestPath)
      })
      keep('C', mode, repetition, outcome, outcome.crossOriginIsolated ?? null)
    }
    if (mode === 'recovery') {
      if (options.layers.includes('b')) {
        const page = await openHarnessPage({ faults: true })
        try {
          const outcome = JSON.parse(
            await page.evaluate(`window.perfHarness.recovery_run(${JSON.stringify(canonicalManifest)})`)
          )
          keep('B', 'recovery', repetition, outcome, page.crossOriginIsolated)
        } finally {
          await page.close()
        }
      }
      if (options.layers.includes('c')) {
        const outcome = await runRecoveryC({ binary, canonical: canonicalManifest, manifest, report })
        keep('C', 'recovery', repetition, outcome, outcome.crossOriginIsolated ?? null)
      }
      continue
    }
    if (options.layers.includes('b')) {
      const page = await openHarnessPage()
      try {
        const outcome = JSON.parse(
          await page.evaluate(
            injection
              ? `window.perfHarness.run_candidate(${JSON.stringify(canonicalManifest)}, ${JSON.stringify(mode)}, ${JSON.stringify(expected)}, ${injection.slowdown_us}, ${JSON.stringify(injection.leak)})`
              : `window.perfHarness.run_workload(${JSON.stringify(canonicalManifest)}, ${JSON.stringify(mode)}, ${JSON.stringify(expected)})`
          )
        )
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
  const names = new Set(
    group.flatMap(run => run.outcome.metrics.filter(m => m.status === 'measured' && m.summary).map(m => m.name))
  )
  for (const name of names) {
    const medians = group
      .map(run => run.outcome.metrics.find(m => m.name === name && m.summary)?.summary.p50)
      .filter(v => v !== undefined)
    if (medians.length < 2) continue
    const low = Math.min(...medians)
    const high = Math.max(...medians)
    const coarse = group.some(run => run.outcome.metrics.find(m => m.name === name)?.resolution_limited === true)
    const unit = group[0].outcome.metrics.find(m => m.name === name)?.unit ?? 'us'
    spread.push({
      group: `${group[0].layer} ${group[0].mode}`,
      name,
      unit,
      repetitions: medians.length,
      low,
      high,
      coarse,
      percent: low > 0 ? ((high - low) / low) * 100 : null
    })
  }
}

let gatesFailed = 0
const gate = (ok, text) => {
  if (!ok) gatesFailed++
  console.log(`    ${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

console.log(`\nresults in ${path.relative(root, outDir)}\n`)
for (const { layer, mode, repetition, runId, outcome } of runs) {
  const timing = outcome.metrics
    .filter(m => m.status === 'measured' && m.summary)
    .map(m => `${m.name} p50 ${m.summary.p50.toFixed(2)} ${m.unit}`)
  console.log(
    `${layer} ${mode} #${repetition}: ${outcome.status}, ${outcome.checkpoints_passed} checkpoints passed, ${outcome.checkpoints_failed.length} failed${timing.length ? `\n    ${timing.join('\n    ')}` : ''}`
  )
  gate(outcome.status === 'ok', `the run is valid (${outcome.status}${outcome.error ? `: ${outcome.error}` : ''})`)
  gate(
    outcome.stream?.matches === true,
    `it executed the intended ${outcome.stream?.scope} stream (${outcome.stream?.operations} operations)`
  )
  for (const failure of outcome.checkpoints_failed.slice(0, 5))
    console.log(`      ${failure.section}: ${failure.detail}`)
  for (const finding of outcome.findings ?? [])
    console.log(`    FINDING ${finding.point} (${finding.contract}): ${finding.summary.slice(0, 400)}`)
  for (const gateResult of outcome.gates ?? [])
    gate(gateResult.status === 'pass', `${gateResult.name}: ${gateResult.status}, ${gateResult.detail}`)
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
    console.log(
      `  ${row.group} ${row.name}: ${show(row.low)} to ${show(row.high)} ${row.unit} over ${row.repetitions} repetitions, ${row.percent === null ? 'n/a' : `${row.percent.toFixed(0)}%`} apart${coarse}`
    )
  }
}
console.log(gatesFailed ? `\n${gatesFailed} deterministic gate(s) failed` : '\nevery deterministic gate passed')
process.exit(gatesFailed ? 1 : 0)
