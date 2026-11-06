// Reruns the Phase 5 checks and keeps their output as evidence, in `perf/evidence/phase5/`.
//
//   node perf/runner/record-phase5-evidence.mjs [step-file ...]
//
// It builds both runners, runs the unit tests (which include the comparison's statistics) and the cross-check, tests the
// comparison, the reports and the promotion of baselines on synthetic runs, proves in a browser that an injected regression and an
// injected leak are caught and a clean candidate produces neither, measures how often the comparison raises a false alarm and how
// often it finds a regression of a given size, and runs the perf-smoke profile twice: once with no baseline, and once against a
// baseline promoted from the first, to show the report. It needs a Rust toolchain with wasm-pack and the wasm32-wasip1 target, a
// local Chrome, Node 22 or later, and `npm ci` in perf/schemas.
//
// With step files named, only those steps are run again and the rest of the record is kept.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../..')
const out = path.join(root, 'perf/evidence/phase5')
const only = process.argv.slice(2)
if (only.length === 0) fs.rmSync(out, { recursive: true, force: true })
fs.mkdirSync(path.join(out, 'results'), { recursive: true })
const wanted = file => only.length === 0 || only.includes(file)

const run = (command, args, options = {}) => {
  const started = Date.now()
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 512 * 1024 * 1024, ...options })
  return { ...result, seconds: (Date.now() - started) / 1000 }
}
const first = (command, args) => run(command, args).stdout?.trim().split('\n')[0] ?? null

const tidy = text =>
  text
    .split('\n')
    .filter(line => !/^\s*(Compiling|Finished|Running|Blocking|Downloaded|Downloading|Fresh|Doc-tests)\b/.test(line))
    .filter(line => !/ExperimentalWarning|node --trace-warnings|^\(Use `node/.test(line))
    .join('\n')
    .replace(/finished in [0-9.]+s/g, 'finished in <time>')
    .replace(/\n{3,}/g, '\n\n')
    .trim() + '\n'

const commands = []
const step = (file, command, args, options = {}) => {
  if (!wanted(file)) return null
  const result = run(command, args, options)
  fs.writeFileSync(path.join(out, file), tidy(result.stdout + (result.status === 0 ? '' : result.stderr)))
  commands.push({
    file,
    command: `${command} ${args.join(' ')}`.replaceAll(root, '<repo>'),
    exit_code: result.status,
    seconds: result.seconds
  })
  console.log(`${result.status === 0 ? 'ok  ' : 'FAIL'} ${file}`)
  return result
}

step('build.txt', 'sh', [
  '-c',
  'cargo build -q --release -p perf-harness && wasm-pack build perf-harness-wasm --target web --release --out-dir pkg'
])

// The parts that need no browser.
const wasiRunner = `node ${path.join(root, 'perf/crosscheck/run-wasi.mjs')}`
step('rust-tests-native.txt', 'cargo', ['test', '-p', 'perf-harness'], { env: { ...process.env, RUST_BACKTRACE: '0' } })
step(
  'rust-tests-wasm32-wasi.txt',
  'cargo',
  ['test', '-p', 'perf-harness', '--no-default-features', '--target', 'wasm32-wasip1'],
  {
    env: { ...process.env, RUST_BACKTRACE: '0', CARGO_TARGET_WASM32_WASIP1_RUNNER: wasiRunner }
  }
)
step('crosscheck.txt', 'node', ['perf/crosscheck/crosscheck.mjs'])
step('schema-checks.txt', 'node', ['check.mjs'], { cwd: path.join(root, 'perf/schemas') })
step('contract-checks.txt', 'node', ['perf/lib/test-contracts.mjs'])
step('comparison-and-promotion-tests.txt', 'node', ['perf/lib/test-regression.mjs'])

// The proof, in a browser, that a regression and a leak that are put in on purpose are caught, and a clean candidate is not.
step('regression-selftest.txt', 'node', ['perf/runner/selftest-regression.mjs'])

// How the comparison behaves: a false alarm at no change, and how often a regression of a given size is found.
step('calibration.txt', 'node', ['perf/lib/calibrate-comparison.mjs', '--trials', '100', '--out', path.join(out, 'calibration.json')])

// The profile that runs on every commit, first with no baseline, and then against one promoted from it.
const results = name => path.join(out, 'results', name)
const baselines = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-evidence-baselines-'))
const smoke = results('perf-smoke')
step('perf-smoke-no-baseline.txt', 'node', ['perf/runner/perf-smoke.mjs', '--no-build', '--out-dir', smoke], {
  env: { ...process.env, PERF_BASELINES_DIR: baselines }
})
if (wanted('perf-smoke-with-baseline.txt')) {
  for (const layer of ['A', 'B'])
    run('node', ['perf/runner/promote-baseline.mjs', '--from', path.join(smoke, 'w2-connector-rerouting-smoke'), '--layer', layer, '--reason', 'Evidence baseline', '--allow-dirty'], {
      env: { ...process.env, PERF_BASELINES_DIR: baselines }
    })
}
const againstBaseline = results('perf-smoke-with-baseline')
step(
  'perf-smoke-with-baseline.txt',
  'node',
  ['perf/runner/perf-smoke.mjs', '--no-build', '--manifests', 'w2-connector-rerouting-smoke', '--out-dir', againstBaseline],
  { env: { ...process.env, PERF_BASELINES_DIR: baselines } }
)
if (wanted('perf-smoke-with-baseline.txt'))
  fs.copyFileSync(path.join(againstBaseline, 'perf-smoke-summary.md'), path.join(out, 'perf-smoke-summary-with-baseline.md'))
fs.rmSync(baselines, { recursive: true, force: true })

const environment = {
  recorded_at: new Date().toISOString(),
  commit: first('git', ['rev-parse', 'HEAD']),
  uncommitted_changes: run('git', ['status', '--porcelain']).stdout.split('\n').filter(Boolean),
  machine: {
    os: `${os.type()} ${os.release()}`,
    cpu: os.cpus()[0].model,
    cores: os.cpus().length,
    memory_bytes: os.totalmem()
  },
  browser: first(process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', ['--version']),
  rustc: first('rustc', ['--version']),
  wasm_pack: first('wasm-pack', ['--version']),
  node: process.version,
  build_profile: 'release',
  commands:
    only.length === 0
      ? commands
      : [
          ...JSON.parse(fs.readFileSync(path.join(out, 'environment.json'), 'utf8')).commands.filter(
            c => !only.includes(c.file)
          ),
          ...commands
        ]
}
fs.writeFileSync(path.join(out, 'environment.json'), JSON.stringify(environment, null, 2) + '\n')
console.log(`written to ${path.relative(root, out)}`)
process.exit(environment.commands.every(c => c.exit_code === 0) ? 0 : 1)
