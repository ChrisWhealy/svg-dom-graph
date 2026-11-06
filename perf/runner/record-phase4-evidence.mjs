// Reruns the Phase 4 checks and keeps their output as evidence, in `perf/evidence/phase4/`.
//
//   node perf/runner/record-phase4-evidence.mjs [step-file ...]
//
// It builds both runners, runs the unit tests and the cross-check, checks the fault contracts against the code they cite, proves
// that the recovery runner fires where it is armed and fails what is wrong, breaks every fault point on Layers B and C, and runs
// the endurance cycles under deliberate failures. The findings are collected into one file. It needs a Rust toolchain with
// wasm-pack and the wasm32-wasip1 target, a local Chrome, Node 22 or later, and `npm ci` in perf/schemas.
//
// With step files named, only those steps are run again and the rest of the record is kept.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../..')
const out = path.join(root, 'perf/evidence/phase4')
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

// The proof that the runner can fail, and then the runs.
step('recovery-selftest.txt', 'node', ['perf/runner/selftest-recovery.mjs'])
const results = name => path.join(out, 'results', name)
step('run-w5-recovery.txt', 'node', [
  'perf/runner/run.mjs',
  'perf/manifests/w5-recovery-small.toml',
  '--mode',
  'recovery',
  '--no-build',
  '--out-dir',
  results('w5-recovery-small-recovery')
])
step('run-w5-endurance.txt', 'node', [
  'perf/runner/run.mjs',
  'perf/manifests/w5-recovery-small.toml',
  '--mode',
  'endurance',
  '--repetitions',
  '2',
  '--no-build',
  '--out-dir',
  results('w5-recovery-small-endurance')
])
step('run-w5-endurance-long.txt', 'node', [
  'perf/runner/run.mjs',
  'perf/manifests/w5-recovery-long.toml',
  '--mode',
  'endurance',
  '--layers',
  'b',
  '--no-build',
  '--out-dir',
  results('w5-recovery-long-endurance')
])

// The findings, from the result documents of the recovery runs, in one place.
if (wanted('findings.json')) {
  const dir = results('w5-recovery-small-recovery')
  const found = {}
  for (const file of fs.existsSync(dir)
    ? fs.readdirSync(dir).filter(f => /-recovery-r0-.*\.json$/.test(f) && !f.endsWith('.cases.json'))
    : []) {
    const result = JSON.parse(fs.readFileSync(path.join(dir, file), 'utf8'))
    for (const finding of result.findings ?? [])
      found[`${result.environment.kind === 'browser' ? file.match(/-(B|C)-/)[1] : '?'}:${finding.point}`] = finding
  }
  fs.writeFileSync(path.join(out, 'findings.json'), JSON.stringify(found, null, 2) + '\n')
  console.log(`ok   findings.json (${Object.keys(found).length})`)
}

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
