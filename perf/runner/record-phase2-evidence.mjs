// Reruns the Phase 2 checks and keeps their output as evidence, in `perf/evidence/phase2/`.
//
//   node perf/runner/record-phase2-evidence.mjs
//
// It builds both runners, verifies the counting allocator, proves that the checkpoint comparison notices faults and that the
// harness's own work is outside every timing, that the endurance gate catches injected leaks, where the bytes that stay
// allocated come from, runs the unit tests, and runs the realistic manifests on both layers. The result documents and their raw samples are kept, so that the
// figures can be read again and the schemas checked against them. It needs a Rust toolchain with wasm-pack and the
// wasm32-wasip1 target, a local Chrome, Node 22 or later, and `npm ci` in perf/schemas.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../..')
const out = path.join(root, 'perf/evidence/phase2')
fs.rmSync(out, { recursive: true, force: true })
fs.mkdirSync(path.join(out, 'results'), { recursive: true })

const run = (command, args, options = {}) => {
  const started = Date.now()
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 512 * 1024 * 1024, ...options })
  return { ...result, seconds: (Date.now() - started) / 1000 }
}
const first = (command, args) => run(command, args).stdout?.trim().split('\n')[0] ?? null

// Timings, build chatter and run identifiers change on every run and say nothing, so they are not kept in the text.
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
  const result = run(command, args, options)
  fs.writeFileSync(path.join(out, file), tidy(result.stdout + (result.status === 0 ? '' : result.stderr)))
  // The repository is wherever it was checked out, so its path is not kept.
  commands.push({ file, command: `${command} ${args.join(' ')}`.replaceAll(root, '<repo>'), exit_code: result.status, seconds: result.seconds })
  console.log(`${result.status === 0 ? 'ok  ' : 'FAIL'} ${file}`)
  return result
}

// Builds first, so that every later step uses the same binaries.
step('build.txt', 'sh', ['-c', 'cargo build -q --release -p perf-harness && wasm-pack build perf-harness-wasm --target web --release --out-dir pkg'])

const wasiRunner = `node ${path.join(root, 'perf/crosscheck/run-wasi.mjs')}`
step('rust-tests-perf-harness.txt', 'cargo', ['test', '-p', 'perf-harness'], { env: { ...process.env, RUST_BACKTRACE: '0' } })
step('rust-tests-perf-harness-wasm.txt', 'cargo', ['test', '-p', 'perf-harness-wasm'], { env: { ...process.env, RUST_BACKTRACE: '0' } })
step('rust-tests-perf-harness-wasm32-wasi.txt', 'cargo', ['test', '-p', 'perf-harness', '--no-default-features', '--target', 'wasm32-wasip1'], {
  env: { ...process.env, RUST_BACKTRACE: '0', CARGO_TARGET_WASM32_WASIP1_RUNNER: wasiRunner },
})
step('allocator-verification.txt', 'node', ['perf/runner/verify-allocator.mjs', '--out', path.join(out, 'allocator-verification.json')])
step('checker-selftest.txt', 'node', ['perf/runner/selftest.mjs'])
step('timing-selftest.txt', 'node', ['perf/runner/selftest-timing.mjs'])
step('endurance-selftest.txt', 'node', ['perf/runner/selftest-endurance.mjs'])
step('final-state-selftest.txt', 'node', ['perf/runner/selftest-final-state.mjs'])
step('persistent-allocations.txt', 'node', ['perf/runner/attribute-persistent.mjs', '--out', path.join(out, 'persistent-allocations.json')])

// The realistic manifests, on both layers. The small ones are covered by Phase 1 and by the self-test.
const manifests = [
  { name: 'w1-scene-construction-medium', mode: 'performance' },
  { name: 'w2-connector-rerouting-large', mode: 'all' },
  { name: 'w3-scene-lifecycle-endurance', mode: 'endurance', layers: 'b' },
]
for (const { name, mode, layers } of manifests) {
  const outDir = path.join(out, 'results', name)
  step(`run-${name}.txt`, 'node', ['perf/runner/run.mjs', `perf/manifests/${name}.toml`, '--mode', mode, '--no-build', '--out-dir', outDir, ...(layers ? ['--layers', layers] : [])])
  // The results hold paths relative to the repository, and the samples sit beside them, so they are kept where they are.
}

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
  wasm_files: Object.fromEntries(
    fs.readdirSync(path.join(root, 'perf-harness-wasm/pkg')).filter(f => f.endsWith('.wasm')).map(f => [f, fs.statSync(path.join(root, 'perf-harness-wasm/pkg', f)).size]),
  ),
  commands,
}
fs.writeFileSync(path.join(out, 'environment.json'), JSON.stringify(environment, null, 2) + '\n')
console.log(`written to ${path.relative(root, out)}`)
process.exit(commands.every(c => c.exit_code === 0) ? 0 : 1)
