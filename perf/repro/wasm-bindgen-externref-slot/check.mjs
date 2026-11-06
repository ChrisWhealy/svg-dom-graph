// Says whether the pinned wasm-bindgen, or one named, still loses an externref slot when an import that returns an object throws.
//
//   node check.mjs [--version 0.2.130] [--expect leak|fixed]
//
// It copies this crate to a temporary directory, pins the version there, and runs `wasm-pack test --node`. The three tests give
// four outcomes that it reads:
//   leak     the behaviour is present: one slot a failure for an import that returns an object, and none for the controls;
//   fixed    the behaviour is gone: nothing left live, and the controls still nothing;
//   other    anything else, which is a change to be looked at before it is believed.
// It exits 0 if the outcome is the one expected (`leak` unless told), so a repository that works around the problem is told
// when it can stop, and is told if the problem changes shape.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const here = import.meta.dirname
const argument = name => {
  const at = process.argv.indexOf(name)
  return at > 0 ? process.argv[at + 1] : null
}
const expect = argument('--expect') ?? 'leak'
const pinned = fs.readFileSync(path.join(here, 'Cargo.toml'), 'utf8').match(/wasm-bindgen = "=([^"]+)"/)[1]
const version = argument('--version') ?? pinned

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'externref-repro-'))
for (const entry of ['Cargo.toml', 'helpers.js', 'src', 'tests'])
  fs.cpSync(path.join(here, entry), path.join(dir, entry), { recursive: true })
const manifest = path.join(dir, 'Cargo.toml')
fs.writeFileSync(
  manifest,
  fs.readFileSync(manifest, 'utf8').replace(/wasm-bindgen = "=[^"]+"/, `wasm-bindgen = "=${version}"`)
)

// The test runner has to be the version of the library, and wasm-pack fetches the one that matches the pin.
const run = spawnSync('wasm-pack', ['test', '--node'], { cwd: dir, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 })
const output = run.stdout + run.stderr
const result = name => (output.match(new RegExp(`test ${name} \\.\\.\\. (ok|FAIL)`)) ?? [])[1] ?? 'missing'
const should = result('a_failed_import_that_returns_an_object_should_leave_no_slot')
const today = result('today_a_failed_import_that_returns_an_object_leaves_one_slot_each')
const controls = result('the_controls_leave_nothing')
const left = output.match(/(\d+) slots were left live by (\d+) failures/)

let outcome = 'other'
if (controls === 'ok' && should === 'FAIL' && today === 'ok') outcome = 'leak'
if (controls === 'ok' && should === 'ok' && today === 'FAIL') outcome = 'fixed'

console.log(`wasm-bindgen ${version}: ${outcome}`)
console.log(
  `  should leave no slot: ${should}; leaves one each, as today: ${today}; controls leave nothing: ${controls}`
)
if (left) console.log(`  ${left[1]} slots were left live by ${left[2]} failures of an import that returns an object`)
if (outcome === 'other') console.log(output.split('\n').slice(-30).join('\n'))
fs.rmSync(dir, { recursive: true, force: true })
console.log(outcome === expect ? `as expected (${expect})` : `NOT as expected: expected ${expect}, found ${outcome}`)
process.exit(outcome === expect ? 0 : 1)
