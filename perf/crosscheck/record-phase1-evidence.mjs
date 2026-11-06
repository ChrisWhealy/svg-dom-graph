// Reruns the Phase 1 checks and keeps their output as evidence, in `perf/evidence/phase1/`.
//
// The record holds the output of each check, the workload report of each reference manifest, and the environment. Run it
// from the repository root:
//
//   node perf/crosscheck/record-phase1-evidence.mjs
//
// It needs a Rust toolchain with the wasm32-wasip1 target, and Node 22 or later.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../..')
const out = path.join(root, 'perf/evidence/phase1')
fs.rmSync(out, { recursive: true, force: true })
fs.mkdirSync(path.join(out, 'reports'), { recursive: true })

const run = (command, args, options = {}) => {
  const started = Date.now()
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024, ...options })
  return { ...result, seconds: (Date.now() - started) / 1000 }
}
const first = (command, args) => run(command, args).stdout?.trim().split('\n')[0] ?? null

// Timings and build chatter change on every run and say nothing, so they are not kept.
const tidy = text =>
  text
    .split('\n')
    .filter(line => !/^\s*(Compiling|Finished|Running|Blocking|Downloaded|Downloading|Fresh|Doc-tests)\b/.test(line))
    .filter(line => !/ExperimentalWarning|node --trace-warnings|^\(Use `node/.test(line))
    .join('\n')
    .replace(/finished in [0-9.]+s/g, 'finished in <time>')
    .replace(/\n{3,}/g, '\n\n')
    .trim() + '\n'

const wasiRunner = `node ${path.join(root, 'perf/crosscheck/run-wasi.mjs')}`
const checks = [
  { file: 'rust-tests-native.txt', command: 'cargo', args: ['test', '-p', 'perf-harness'] },
  {
    file: 'rust-tests-without-schema.txt',
    command: 'cargo',
    args: ['test', '-p', 'perf-harness', '--no-default-features']
  },
  {
    file: 'rust-tests-wasm32-wasi.txt',
    command: 'cargo',
    args: ['test', '-p', 'perf-harness', '--no-default-features', '--target', 'wasm32-wasip1'],
    env: { CARGO_TARGET_WASM32_WASIP1_RUNNER: wasiRunner, RUST_BACKTRACE: '0' }
  },
  { file: 'crosscheck-rust-vs-javascript.txt', command: 'node', args: ['perf/crosscheck/crosscheck.mjs'] },
  { file: 'schema-checks.txt', command: 'npm', args: ['test'], cwd: path.join(root, 'perf/schemas') }
]

const commands = []
for (const check of checks) {
  const result = run(check.command, check.args, {
    cwd: check.cwd ?? root,
    env: { ...process.env, RUST_BACKTRACE: '0', ...check.env }
  })
  fs.writeFileSync(path.join(out, check.file), tidy(result.stdout + (result.status === 0 ? '' : result.stderr)))
  commands.push({
    file: check.file,
    command: `${check.command} ${check.args.join(' ')}`,
    exit_code: result.status,
    seconds: result.seconds
  })
  console.log(`${result.status === 0 ? 'ok  ' : 'FAIL'} ${check.file}`)
}

// The report of each reference manifest, which is what a runner is given to check itself against.
for (const name of fs.readdirSync(path.join(root, 'perf/manifests')).filter(file => file.endsWith('.toml'))) {
  const id = name.replace(/\.toml$/, '')
  const result = run('cargo', [
    'run',
    '-q',
    '-p',
    'perf-harness',
    '--',
    'generate',
    `perf/manifests/${name}`,
    '--id',
    id,
    '--out',
    `perf/evidence/phase1/reports/${id}.json`
  ])
  commands.push({
    file: `reports/${id}.json`,
    command: `perf-harness generate perf/manifests/${name} --id ${id}`,
    exit_code: result.status,
    seconds: result.seconds
  })
  console.log(`${result.status === 0 ? 'ok  ' : 'FAIL'} reports/${id}.json`)
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
  rustc: first('rustc', ['--version']),
  cargo: first('cargo', ['--version']),
  node: process.version,
  targets_run: ['native (this machine)', 'wasm32-wasip1 (32-bit, under Node)'],
  commands
}
fs.writeFileSync(path.join(out, 'environment.json'), JSON.stringify(environment, null, 2) + '\n')
console.log(`written to ${path.relative(root, out)}`)
process.exit(commands.every(c => c.exit_code === 0) ? 0 : 1)
