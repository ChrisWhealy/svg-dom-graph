// Proves that the W6 runners can fail: that each thing they say they guard against is noticed when it is made to happen.
//
//   node dess/s5/selftest.mjs [--skip-play] [--skip-classification]
//
// A clean run in which every gate passes shows nothing until it is known that a gate fails when it should. This makes things wrong on
// purpose, in the pieces that a clean run does not break, and each has to be seen for what it is and for nothing else.
//
//   1. A seed that S0 refused has its timing refused before any layer runs, with every figure that was worked out and no time series, and
//      the run does not pass. The modes that are not timings still run on the trace and are valid, with the qualification recorded.
//   2. A trace that is wrong. One checkpoint of the independent engine is changed, and Layer B correctness, Layer B performance and Layer C must
//      all come out invalid_incorrect and name the slice, so that a timing is never taken from a run that was wrong.
//   3. The controller reads layout to decide whether it may draw, as the product's own page does. The gate on layout in a timed interval has to fail.
//   4. A window that is shifted: a manifest that needs fewer warm-up batches than the fixed window holds ends earlier, and the gate on the fixed
//      windows has to say where each measured batch began and ended.
//   5. The classification of repetitions. The three of the per-commit smoke profile, given to the real comparison as a baseline and as a
//      candidate, have to come out diagnostic with no interval, and the five of the comparison profile have to give a verdict with one, so
//      that a statistical pass is never reported from three.
//   6. A Play cycle whose Pause comes late, past the interval, so that a second tick commits first. The repetition has to be rejected as
//      invalid_protocol_extra_tick, and for nothing else. This is the one that takes about 20 seconds.
//
// It exits with 1 if any is not seen.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from '../../perf/runner/lib/harness-page.mjs'
import { runSimulationC } from '../../perf/runner/layer-c/service.mjs'
import { buildTrace } from '../../perf/runner/service-trace.mjs'
import { METHOD_VERSION, compareCollections, environmentKey, resultFiles } from '../../perf/lib/baselines.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const binary = path.join(root, 'target/release/perf-harness')
const manifestPath = path.join(root, 'perf/manifests/w6-service-simulation-smoke.toml')
const skipPlay = process.argv.includes('--skip-play')
// The classification of repetitions runs the smoke manifest three times and the comparison manifest five, which is most of this script's time, and
// it tests the comparison and not the runners. Per-commit CI skips it, and the nightly profile runs it.
const skipClassification = process.argv.includes('--skip-classification')
const canonical = spawnSync(binary, ['canonical', manifestPath], { encoding: 'utf8' }).stdout.trim()
const manifest = JSON.parse(canonical)
const trace = buildTrace(manifest.simulation, manifest.seed)

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

// 1. A refused seed, through the orchestrator.
{
  const work = fs.mkdtempSync(path.join(os.tmpdir(), 'dess-s5-'))
  const refusedManifest = path.join(work, 'w6-refused.toml')
  fs.writeFileSync(refusedManifest, fs.readFileSync(manifestPath, 'utf8').replace('seed = 20261106', 'seed = 85'))
  const out = path.join(work, 'out')
  const ran = spawnSync(
    'node',
    [
      path.join(root, 'perf/runner/run.mjs'),
      refusedManifest,
      '--out-dir',
      out,
      '--no-build',
      '--layers',
      'a,b',
      '--repetitions',
      '1'
    ],
    {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: 1 << 28
    }
  )
  const results = fs
    .readdirSync(out)
    .filter(f => f.endsWith('.json') && !f.endsWith('.cases.json') && f.startsWith('w6-refused'))
    .map(f => JSON.parse(fs.readFileSync(path.join(out, f), 'utf8')))
  const refused = results.filter(r => r.mode === 'performance')
  const others = results.filter(r => r.mode !== 'performance')
  check(ran.status !== 0, 'a refused trace does not pass: the run exits with a failure')
  check(
    refused.length === 1 &&
      refused.every(
        r => r.status === 'refused' && /halves differ/.test(r.error) && r.metrics.length === 0 && !r.raw_samples
      ),
    `the timing is refused with the reason, and has no figure and no time series: ${refused.length} result`
  )
  check(
    /no layer was run/.test(refused[0]?.error ?? ''),
    'and it says that no layer was run, since the refusal comes before any'
  )
  check(
    results.every(
      r =>
        r.simulation?.qualification.qualified === false &&
        r.simulation.qualification.metrics &&
        r.simulation.counts.pops > 0
    ),
    'every result keeps the generator revision, the seed and every figure that was worked out, the qualification among them'
  )
  // A trace that cannot be timed can still be checked against, so the correctness and endurance of it run, and are valid.
  check(
    others.length > 0 && others.every(r => r.status === 'ok'),
    `the modes that are not timings are still run on it, and are valid: ${others.map(r => `${r.mode} ${r.status}`).join(', ')}`
  )
  fs.rmSync(work, { recursive: true, force: true })
}

// A seed over 2**53 is the same seed in the independent engine and in the native one, and in the page. Layer A holds every checkpoint of the
// Rust engine to the independent engine's, so a seed that one of them rounded would fail it, and Layer B does the same through the wasm.
{
  const work = fs.mkdtempSync(path.join(os.tmpdir(), 'dess-s5-seed-'))
  for (const seed of ['9007199254740993', '18446744073709551557']) {
    const file = path.join(work, `w6-${seed}.toml`)
    fs.writeFileSync(file, fs.readFileSync(manifestPath, 'utf8').replace('seed = 20261106', `seed = ${seed}`))
    const out = path.join(work, `out-${seed}`)
    const ran = spawnSync(
      'node',
      [
        path.join(root, 'perf/runner/run.mjs'),
        file,
        '--mode',
        'correctness',
        '--layers',
        'a,b',
        '--out-dir',
        out,
        '--no-build'
      ],
      {
        cwd: root,
        encoding: 'utf8',
        maxBuffer: 1 << 28
      }
    )
    const files = fs.existsSync(out)
      ? fs.readdirSync(out).filter(f => f.endsWith('.json') && !f.endsWith('.cases.json') && f.startsWith('w6-'))
      : []
    const texts = files.map(f => fs.readFileSync(path.join(out, f), 'utf8'))
    check(
      ran.status === 0 &&
        texts.length === 2 &&
        texts.every(
          t => t.includes(`"seed": ${seed}`) && !t.includes(`"seed": ${Number(seed)}`.replace(/e\+?\d+$/, ''))
        ),
      `seed ${seed}: the Rust engine and the page both agree with the independent engine on every slice, and the result keeps the exact seed`
    )
  }
  fs.rmSync(work, { recursive: true, force: true })
}

// A manifest that names its layers is held to them. The 100,000-pop one is for Layer A alone, so no browser is started for it, and a layer
// that is asked for by name and is not named is refused.
{
  const big = path.join(root, 'perf/manifests/w6-service-simulation-100k.toml')
  const out = fs.mkdtempSync(path.join(os.tmpdir(), 'dess-s5-100k-'))
  const args = [path.join(root, 'perf/runner/run.mjs'), big, '--out-dir', out, '--no-build']
  const ran = spawnSync('node', args, { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28 })
  const layers = fs
    .readdirSync(out)
    .filter(f => f.startsWith('w6-service-simulation-100k') && f.endsWith('.json') && !f.endsWith('.cases.json'))
    .map(f => JSON.parse(fs.readFileSync(path.join(out, f), 'utf8')).run_id.match(/-([ABC])-/)?.[1])
  check(
    ran.status === 0 && layers.length === 1 && layers[0] === 'A' && /skipping layer b, c/.test(ran.stdout),
    `the 100,000-pop manifest, with no layers asked for, runs Layer A only: ${layers.join(', ') || 'none'}`
  )
  const asked = spawnSync('node', [...args, '--layers', 'a,b'], { cwd: root, encoding: 'utf8' })
  check(
    asked.status !== 0 && /for layer a alone, and b was asked for/.test(asked.stderr),
    'and Layer B, asked for by name, is refused'
  )
  fs.rmSync(out, { recursive: true, force: true })
}

// 2, 3 and 4 need a page.
const page = await openHarnessPage()
const run = async (mode, expected, testing = {}, changed = manifest) =>
  JSON.parse(
    await page.evaluate(
      `window.perfHarness.sim_run(${JSON.stringify(mode)}, ${JSON.stringify(JSON.stringify(changed))}, ${JSON.stringify(JSON.stringify(expected))}, ${JSON.stringify(JSON.stringify(testing))})`
    )
  )
try {
  // 2. A checkpoint that is wrong.
  const wrong = structuredClone(trace.expected)
  wrong.checkpoints[300] = '0'.repeat(64)
  // A slice inside a batch and not the last of it: batches end at 300, and 299 is the one before.
  const inside = structuredClone(trace.expected)
  inside.checkpoints[299] = '0'.repeat(64)
  const hidden = await run('performance', inside)
  check(
    hidden.status === 'invalid_incorrect' &&
      hidden.checkpoints_failed.some(f => /after slice 299/.test(f.detail)) &&
      hidden.stream.matches === false,
    `Layer B performance with a wrong checkpoint inside a batch (slice 299) is invalid_incorrect and names it: ${hidden.status}`
  )
  for (const mode of ['correctness', 'performance']) {
    const o = await run(mode, wrong)
    check(
      o.status === 'invalid_incorrect' &&
        o.checkpoints_failed.some(
          f => /slice 300|ending at slice 300|at slice 300/.test(f.detail) || /300/.test(f.detail)
        ),
      `Layer B ${mode} with a wrong checkpoint is invalid_incorrect and names slice 300: ${o.status}, ${o.checkpoints_failed.length} failed`
    )
    check(o.stream && o.stream.matches === false, `and its digest says that it ran another stream (${mode})`)
  }

  // 3. A controller that reads layout.
  const reading = await run('performance', trace.expected, { product: true })
  const gate = reading.gates.find(g => g.name === 'simulation.no_layout_is_read_in_a_timed_interval')
  check(gate?.status === 'fail', `a controller that reads layout fails the gate on it: ${gate?.detail}`)
  const clean = await run('performance', trace.expected)
  check(
    clean.status === 'ok' &&
      clean.gates.find(g => g.name === 'simulation.no_layout_is_read_in_a_timed_interval')?.status === 'pass',
    'and the measurement fixture, which does not, passes it'
  )

  // 4. A shorter run ends earlier, and the gate on the windows says where.
  const shorter = structuredClone(manifest)
  shorter.execution.warmup_iterations = 5
  shorter.execution.measurement_iterations = 30
  const short = await run('performance', trace.expected, {}, shorter)
  const windows = short.gates.find(g => g.name === 'simulation.windows_are_not_wrapped')
  check(
    windows?.status === 'pass' && /began at slice 277 and ended at 396/.test(windows.detail),
    `a smaller manifest begins its measured window after its own warm-up and says so: ${windows?.detail}`
  )
} finally {
  await page.close()
}

// 5. Three repetitions are diagnostic, and five are not.
if (skipClassification) console.log('skip the classification of repetitions: --skip-classification')
else {
  const classify = (manifestFile, repetitions, layer) => {
    const work = fs.mkdtempSync(path.join(os.tmpdir(), 'dess-s5-classify-'))
    const out = path.join(work, 'out')
    const ran = spawnSync(
      'node',
      [
        path.join(root, 'perf/runner/run.mjs'),
        manifestFile,
        '--mode',
        'performance',
        '--layers',
        'a,b',
        '--repetitions',
        String(repetitions),
        '--out-dir',
        out,
        '--no-build'
      ],
      { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28 }
    )
    if (ran.status !== 0) return { error: `the runs failed: ${ran.stdout.slice(-300)}` }
    const files = resultFiles(out).filter(f => JSON.parse(fs.readFileSync(f, 'utf8')).run_id.includes(`-${layer}-`))
    const first = JSON.parse(fs.readFileSync(files[0], 'utf8'))
    const baseline = path.join(work, 'baseline')
    fs.cpSync(out, baseline, { recursive: true })
    fs.writeFileSync(
      path.join(baseline, 'promotion.json'),
      JSON.stringify({
        promotion: {
          promoted_at: new Date().toISOString(),
          commit: first.environment.git_sha,
          workload_hash: first.manifest.sha256,
          environment_key: environmentKey(files[0]),
          method_version: METHOD_VERSION
        }
      })
    )
    const report = compareCollections({
      baselineDir: baseline,
      candidateDirs: [out],
      layer,
      mode: 'performance',
      out: path.join(work, 'comparison.json')
    })
    fs.rmSync(work, { recursive: true, force: true })
    return { report }
  }
  for (const [file, repetitions, diagnostic] of [
    ['perf/manifests/w6-service-simulation-smoke.toml', 3, true],
    ['perf/manifests/w6-service-simulation-comparison.toml', 5, false]
  ]) {
    const { report, error } = classify(path.join(root, file), repetitions, 'A')
    const verdicts = report?.comparisons.map(c => c.verdict) ?? []
    const intervals = report?.comparisons.some(c => c.ratio_interval !== undefined) ?? false
    check(
      !error &&
        verdicts.length > 0 &&
        (diagnostic
          ? verdicts.every(v => v === 'diagnostic') && !intervals
          : verdicts.every(v => v !== 'diagnostic') && intervals),
      error ??
        `${repetitions} repetitions come out ${diagnostic ? 'diagnostic, with no interval' : 'with a verdict and an interval'}: ${[...new Set(verdicts)].join(', ')}`
    )
  }
}

// 6. A late Pause.
if (skipPlay) console.log('skip the late Pause: --skip-play')
else {
  const o = await runSimulationC({
    binary,
    manifest,
    seed: String(manifest.seed),
    trace: trace.expected,
    series: ['play'],
    injectLatePause: true
  })
  check(
    o.status === 'invalid_incorrect' &&
      o.checkpoints_failed.length === 1 &&
      o.checkpoints_failed[0].detail.startsWith('invalid_protocol_extra_tick'),
    `a late Pause rejects the repetition as invalid_protocol_extra_tick and for nothing else: ${o.checkpoints_failed.map(f => f.detail).join('; ')}`
  )
  const play = o.metrics.find(m => m.name === 'simulation.play.timer_to_presentation')
  check(
    play?.status === 'unavailable' && !o.samples['simulation.play.timer_to_presentation'],
    'and it leaves no Play latency, which is never a pass'
  )
}

console.log(failures ? `\n${failures} failed` : '\nthe W6 runners refuse, reject and fail where they should')
process.exit(failures ? 1 : 0)
