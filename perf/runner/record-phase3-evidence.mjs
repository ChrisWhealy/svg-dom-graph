// Reruns the Phase 3 checks and keeps their output as evidence, in `perf/evidence/phase3/`.
//
//   node perf/runner/record-phase3-evidence.mjs [step-file ...]
//
// With step files named, such as `gallery-sweep.txt`, only those steps are run again and the rest of the record is kept, with its
// entry in `environment.json` replaced for each step that was run again.
//
// It builds both runners, runs the unit tests of the gesture planner and the frame attribution, the independent cross-check, and
// the schema checks, reruns the frame-correlation spike, proves that Layer C moves by what it measures and by nothing else, runs
// the interaction workload on every layer, and sweeps the gallery. It needs a Rust toolchain with wasm-pack and the wasm32-wasip1
// target, a local Chrome, Node 22 or later, `npm ci` in perf/schemas, and the gallery built by
// `cargo run -p demo-server -- --build-only`.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../..')
const out = path.join(root, 'perf/evidence/phase3')
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
  commands.push({ file, command: `${command} ${args.join(' ')}`.replaceAll(root, '<repo>'), exit_code: result.status, seconds: result.seconds })
  console.log(`${result.status === 0 ? 'ok  ' : 'FAIL'} ${file}`)
  return result
}

step('build.txt', 'sh', ['-c', 'cargo build -q --release -p perf-harness && wasm-pack build perf-harness-wasm --target web --release --out-dir pkg'])

// The deterministic parts, which need no browser.
const wasiRunner = `node ${path.join(root, 'perf/crosscheck/run-wasi.mjs')}`
step('rust-tests-native.txt', 'cargo', ['test', '-p', 'perf-harness'], { env: { ...process.env, RUST_BACKTRACE: '0' } })
step('rust-tests-wasm32-wasi.txt', 'cargo', ['test', '-p', 'perf-harness', '--no-default-features', '--target', 'wasm32-wasip1'], {
  env: { ...process.env, RUST_BACKTRACE: '0', CARGO_TARGET_WASM32_WASIP1_RUNNER: wasiRunner },
})
step('crosscheck.txt', 'node', ['perf/crosscheck/crosscheck.mjs'])
step('schema-checks.txt', 'node', ['check.mjs'], { cwd: path.join(root, 'perf/schemas') })
step('frame-attribution-tests.txt', 'node', ['perf/lib/test-frames.mjs'])
step('walk-verdict-tests.txt', 'node', ['perf/runner/test-walk-verdict.mjs'])

// The spike that Phase 3 is conditional on, and the self-test that Layer C measures what it says.
step('frame-correlation-spike.txt', 'node', ['perf/spike/frame-correlation.mjs', '--out', path.join(out, 'frame-correlation.json')])
fs.rmSync(path.join(out, 'frame-correlation.txt'), { force: true })
step('layer-c-selftest.txt', 'node', ['perf/runner/selftest-layer-c.mjs'])

// The interaction workload on every layer, and the larger one on Layer C alone.
step('run-w4-interaction-small.txt', 'node', ['perf/runner/run.mjs', 'perf/manifests/w4-interaction-small.toml', '--no-build', '--out-dir', path.join(out, 'results/w4-interaction-small')])
step('run-w4-interaction-medium.txt', 'node', ['perf/runner/run.mjs', 'perf/manifests/w4-interaction-medium.toml', '--layers', 'c', '--mode', 'performance', '--no-build', '--out-dir', path.join(out, 'results/w4-interaction-medium')])

// The gallery, and the proof that its verdicts can fail.
step('gallery-selftest.txt', 'node', ['perf/runner/selftest-gallery.mjs'])
step('gallery-sweep.txt', 'node', ['perf/runner/gallery-sweep.mjs', '--out', path.join(out, 'results/gallery')])

const previous = only.length > 0 ? JSON.parse(fs.readFileSync(path.join(out, 'environment.json'), 'utf8')) : null
const environment = {
  recorded_at: new Date().toISOString(),
  commit: first('git', ['rev-parse', 'HEAD']),
  uncommitted_changes: run('git', ['status', '--porcelain']).stdout.split('\n').filter(Boolean),
  machine: { os: `${os.type()} ${os.release()}`, cpu: os.cpus()[0].model, cores: os.cpus().length, memory_bytes: os.totalmem() },
  browser: first(process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', ['--version']),
  rustc: first('rustc', ['--version']),
  wasm_pack: first('wasm-pack', ['--version']),
  node: process.version,
  build_profile: 'release',
  commands: only.length === 0 ? commands : [...previous.commands.filter(c => !only.includes(c.file)), ...commands],
}
fs.writeFileSync(path.join(out, 'environment.json'), JSON.stringify(environment, null, 2) + '\n')
console.log(`written to ${path.relative(root, out)}`)
process.exit(environment.commands.every(c => c.exit_code === 0) ? 0 : 1)
