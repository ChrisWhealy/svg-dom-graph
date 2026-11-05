// Reruns the Phase 0 experiments and keeps their raw output as evidence, in `perf/evidence/phase0/`.
//
// The record holds the output of each experiment, the environment (machine, browser, toolchain), the commit, and a hash
// of every file the fixture was built into, so a later reader can tell what was observed from what is assumed. Run it
// from the repository root:
//
//   node perf/spike/record-evidence.mjs --build
//
// `--build` rebuilds the fixture first with the same command the CDP tests use. Without it the existing `pkg/` is used,
// and the record says when that pkg was built, because `pkg/` is not committed and may not match the checkout.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import crypto from 'node:crypto'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../..')
const out = path.join(root, 'perf/evidence/phase0')
const fixture = path.join(root, 'cdp-test-fixture')
const chrome = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'

const run = (command, args, options = {}) => {
  const started = Date.now()
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, ...options })
  return { ...result, seconds: (Date.now() - started) / 1000 }
}
const first = (command, args) => run(command, args).stdout?.trim().split('\n')[0] ?? null
const sha256 = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex')

fs.mkdirSync(out, { recursive: true })

const build = process.argv.includes('--build')
if (build) {
  const built = run('wasm-pack', ['build', '--target', 'web'], { cwd: fixture, stdio: ['ignore', 'pipe', 'pipe'] })
  if (built.status !== 0) {
    console.error(built.stderr)
    process.exit(1)
  }
}

const pkg = path.join(fixture, 'pkg')
const fixtureFiles = Object.fromEntries(
  fs
    .readdirSync(pkg)
    .filter(name => /\.(js|wasm)$/.test(name))
    .sort()
    .map(name => [name, { sha256: sha256(path.join(pkg, name)), bytes: fs.statSync(path.join(pkg, name)).size }]),
)
const pkgBuilt = fs.statSync(path.join(pkg, 'cdp_test_fixture_bg.wasm')).mtime.toISOString()

const environment = {
  recorded_at: new Date().toISOString(),
  commit: first('git', ['rev-parse', 'HEAD']),
  commit_date: first('git', ['log', '-1', '--format=%cI']),
  uncommitted_changes: run('git', ['status', '--porcelain']).stdout.split('\n').filter(Boolean),
  machine: { os: `${os.type()} ${os.release()}`, macos: first('sw_vers', ['-productVersion']), cpu: os.cpus()[0].model, cores: os.cpus().length, memory_bytes: os.totalmem() },
  browser: first(chrome, ['--version']),
  node: process.version,
  rustc: first('rustc', ['--version']),
  wasm_pack: first('wasm-pack', ['--version']),
  fixture: { rebuilt_by_this_run: build, pkg_built: pkgBuilt, files: fixtureFiles },
}

const experiments = [
  { name: 'timer-and-counters-not-isolated.json', args: ['perf/spike/phase0-spike.mjs', 'cdp-test-fixture'] },
  { name: 'timer-and-counters-isolated.json', args: ['perf/spike/phase0-spike.mjs', 'cdp-test-fixture', 'isolated'] },
  { name: 'listener-ownership.txt', args: ['perf/spike/listener-ownership.mjs', 'cdp-test-fixture'] },
  { name: 'listener-tracker-tests.txt', args: ['perf/page/test-listener-tracker.mjs'] },
]
const commands = []
for (const { name, args } of experiments) {
  const result = run(process.execPath, args)
  // Port numbers and temporary paths differ on every run and say nothing, so they are not kept in the output.
  const text = result.stdout.replace(/127\.0\.0\.1:\d+/g, '127.0.0.1:PORT')
  fs.writeFileSync(path.join(out, name), text)
  commands.push({ file: name, command: `node ${args.join(' ')}`, exit_code: result.status, seconds: result.seconds })
  console.log(`${result.status === 0 ? 'ok  ' : 'FAIL'} ${name}`)
}
environment.commands = commands
fs.writeFileSync(path.join(out, 'environment.json'), JSON.stringify(environment, null, 2) + '\n')
console.log(`written to ${path.relative(root, out)}`)
