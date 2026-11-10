// Breaks the discrete-event service simulator's controller on purpose, and judges what is left (the service target of workload W5).
//
//   node perf/runner/service-recovery.mjs [--manifest FILE] [--out FILE] [--no-build]
//
// The manifest is read by the Rust manifest reader, so that this runner and the harness cannot disagree about what a manifest means. The
// scenario that it names is run by the independent JavaScript engine, and its hash of the state at every slice goes to the page, where the
// controller's committed slice is compared with it. The sweeps themselves are in perf/runner/page/service-recovery.js, in the page, next
// to the controller that they break.
//
// The exit status is 0 only if every gate passed and no checkpoint failed.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from './lib/harness-page.mjs'
import { oracleFor } from './service-oracle.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const manifestPath = path.resolve(
  argument('--manifest') ?? path.join(root, 'perf/manifests/w5-service-recovery-small.toml')
)
const out = argument('--out')

const run = (command, args) => spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28 })
const must = (result, what) => {
  if (result.status !== 0) {
    console.error(`${what} failed:\n${result.stderr || result.stdout}`)
    process.exit(2)
  }
  return result
}

if (!process.argv.includes('--no-build')) {
  must(run('cargo', ['build', '-q', '--release', '-p', 'perf-harness']), 'building perf-harness')
  must(
    run('wasm-pack', ['build', 'perf-harness-wasm', '--target', 'web', '--release', '--out-dir', 'pkg']),
    'building perf-harness-wasm'
  )
}
const canonical = must(
  run(path.join(root, 'target/release/perf-harness'), ['canonical', manifestPath]),
  'reading the manifest'
).stdout.trim()
const manifest = JSON.parse(canonical)
if (manifest.recovery?.target !== 'service_simulation') {
  console.error('this runner breaks the service simulator, and the manifest is not aimed at it')
  process.exit(2)
}
const oracle = oracleFor(manifest.recovery.scenario)
console.log(
  `scenario ${oracle.scenario}: the independent oracle hashed ${oracle.slices + 1} states${oracle.agreesWithRust ? ', and the Rust engine agrees on every one' : ''}`
)

const page = await openHarnessPage({ faults: true })
let outcome
try {
  outcome = JSON.parse(
    await page.evaluate(
      `window.perfHarness.service_recovery_run(${JSON.stringify(canonical)}, ${JSON.stringify(JSON.stringify({ oracle }))})`
    )
  )
} finally {
  await page.close()
}

let failures = 0
for (const gate of outcome.gates) {
  if (gate.status !== 'pass') failures += 1
  console.log(
    `${gate.status === 'pass' ? 'ok  ' : gate.status === 'fail' ? 'FAIL' : 'NOTE'} ${gate.name}: ${gate.detail}`
  )
}
for (const note of outcome.notes) console.log(`     ${note}`)
for (const failed of outcome.checkpoints_failed) console.log(`FAIL ${failed.section}: ${failed.detail}`)
if (outcome.status !== 'ok') console.log(`status: ${outcome.status}${outcome.error ? ` (${outcome.error})` : ''}`)
console.log(
  `${outcome.checkpoints_passed} injected failures were recovered as the contract says, and ${outcome.checkpoints_failed.length} were not`
)
if (out)
  fs.writeFileSync(
    path.resolve(root, out),
    `${JSON.stringify({ ...outcome, oracle: { scenario: oracle.scenario, slices: oracle.slices, agrees_with_rust: oracle.agreesWithRust } }, null, 2)}\n`
  )
process.exit(failures === 0 && outcome.status === 'ok' && outcome.checkpoints_failed.length === 0 ? 0 : 1)
