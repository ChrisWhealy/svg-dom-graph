// Runs every check that gathers the evidence for the simulator's stages and the gallery panel's phases, in order, and says what passed.
//
//   node dess/gather-evidence.mjs [--all] [--only a,b] [--skip a,b] [--list] [--allow-dirty] [--stop-on-failure]
//
// The evidence files that it writes carry the commit that they were measured at and whether the tree was dirty, and only a run from a clean,
// committed tree is acceptance evidence, so it refuses to start on a dirty tree unless `--allow-dirty` says that this is a trial. It builds
// the gallery first, since the browser tests serve the staged build. A step that fails does not stop the others unless `--stop-on-failure` is
// given, and the exit status is 1 if any step failed. Nothing is committed: look at `git status` afterwards and commit the evidence files.
//
// Without `--all` the steps that take most of an hour are left out. They are marked slow in `--list`.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '..')
const flag = name => process.argv.includes(name)
const list = name =>
  process.argv.includes(name) ? (process.argv[process.argv.indexOf(name) + 1] ?? '').split(',').filter(Boolean) : []

const WASI_RUNNER = `node ${path.join(root, 'perf/crosscheck/run-wasi.mjs')}`
const P = 'dess/demo-panel'

/** [name, what it shows, command, arguments, { slow, env }] */
const STEPS = [
  ['format', 'rustfmt finds nothing to change', 'cargo', ['fmt', '--all', '--check']],
  ['lint', 'clippy is clean for every crate, native and wasm', './clippy.sh', []],
  [
    'unit-engine',
    'the engine: known answers, golden hashes, invariants, error paths',
    'cargo',
    ['test', '-p', 'service-sim-engine']
  ],
  [
    'unit-service-sim',
    'the projection, controller, coordinator and catalogue',
    'cargo',
    ['test', '-p', 'service-sim', '--lib']
  ],
  [
    'unit-demo',
    'the demo, including that every panel source can be shown',
    'cargo',
    ['test', '-p', 'svg-dom-graph-demo']
  ],
  ['oracle', 'the independent engine against the known answers', 'node', ['dess/reference-js/test-oracle.mjs']],
  [
    'engine-release',
    'the release builds that the differential run needs',
    'cargo',
    ['build', '--release', '-p', 'service-sim-engine']
  ],
  ['differential', 'the two engines agree on every pop, slice and hash (native)', 'node', ['dess/s1/differential.mjs']],
  [
    'engine-wasi',
    'the engine on 32-bit wasm gives the same answers',
    'cargo',
    ['test', '-p', 'service-sim-engine', '--target', 'wasm32-wasip1'],
    { env: { CARGO_TARGET_WASM32_WASIP1_RUNNER: WASI_RUNNER } }
  ],
  [
    'engine-wasi-build',
    'the 32-bit trace binary that the differential run needs',
    'cargo',
    ['build', '--release', '-p', 'service-sim-engine', '--target', 'wasm32-wasip1', '--bin', 'dess-trace']
  ],
  ['differential-wasi', 'the two engines agree on 32-bit wasm', 'node', ['dess/s1/differential.mjs', '--wasi']],
  ['coverage', 'the S1 coverage report of the panels', 'node', ['dess/s1/coverage.mjs']],
  [
    's1-mutation',
    'S1: the engine is broken in small ways and the tests notice',
    'node',
    ['dess/s1/mutation-check.mjs'],
    { slow: true }
  ],
  [
    'browser-service-sim',
    'the surface and controller tests, in Chrome',
    'wasm-pack',
    ['test', '--headless', '--chrome', 'service-sim']
  ],
  ['p0', 'P0: staging, the injector and the feature gate, with their controls', 'node', [`${P}/p0/run-all.mjs`]],
  [
    'p1-mutation',
    'P1: the coordinator is broken in small ways and the tests notice',
    'node',
    [`${P}/p1/mutation-check.mjs`, '--out', `${P}/p1/evidence/mutation-check.json`]
  ],
  [
    'demo-build',
    'the gallery is built and staged for the browser tests',
    'cargo',
    ['run', '-p', 'demo-server', '--', '--build-only']
  ],
  [
    'feature-gate',
    "the demo build does not hold the simulator's test-only fault hook",
    'node',
    [`${P}/p0/features.mjs`, '--require-service-sim']
  ],
  [
    'gallery',
    'P2 and P3: G01 to G17, G05W6, G16F, G16P, L01 and L02 in Chrome',
    'node',
    [`${P}/test-gallery.mjs`, '--out', `${P}/p3/evidence/gallery.json`],
    { copyTo: `${P}/p2/evidence/gallery.json` }
  ],
  [
    'gallery-sweep',
    'the whole-gallery sweep still passes for the panel',
    'node',
    ['perf/runner/gallery-sweep.mjs', '--only', 'panel-service-simulator']
  ],
  ['firefox-service-sim', 'an observation: the surface and controller tests in Firefox (not a gate)', 'wasm-pack', ['test', '--headless', '--firefox', 'service-sim']],
  ['firefox-visit', 'an observation: a short visit to the panel in Firefox through geckodriver (not a gate)', 'node', [`${P}/p4/firefox-observe.mjs`]],
  ['gallery-nightly', 'P4: the long paths G18 and G19 and the cycles of G20, at the nightly size (about a minute)', 'node', [`${P}/test-gallery.mjs`, '--nightly', '--out', `${P}/p4/evidence/gallery-nightly.json`]],
  [
    'p2-mutation',
    'P2: the panel is broken in small ways and the tests notice',
    'node',
    [`${P}/p2/mutation-check.mjs`, '--out', `${P}/p2/evidence/mutation-check.json`],
    { slow: true }
  ],
  ['p4-mutation', 'P4: the taking down of the panel and the long paths are broken in small ways and the tests notice', 'node', [`${P}/p4/mutation-check.mjs`, '--out', `${P}/p4/evidence/mutation-check.json`], { slow: true }],
  ['size-ledger', 'P4: the demo wasm before and after the panel, in bytes and gzipped, with the dependency tree (two full builds)', 'node', [`${P}/p4/size-ledger.mjs`], { slow: true }],
  [
    'p3-mutation',
    'P3: the panel, the handoff and the layout are broken in small ways and the tests notice',
    'node',
    [`${P}/p3/mutation-check.mjs`, '--out', `${P}/p3/evidence/mutation-check.json`],
    { slow: true }
  ],
  [
    'perf-smoke',
    'the performance workloads still run and compare with their baselines',
    'node',
    ['perf/runner/perf-smoke.mjs'],
    { slow: true }
  ]
]

const MANUAL = [
  'A screen reader (VoiceOver or NVDA) reads the Snapshot range as "Snapshot N of M": Chrome\'s accessibility tree does not show aria-valuetext.',
  'Safari, with its version: the panel loads, the controls work, and the server colours show (they need :has()). Firefox has a script and Safari has no driver here. These are observations, not pass claims: see dess/demo-panel/p4/browser-observations.md.',
  'A phone-width window, by eye: the controls, the scene and the inspector stay together and are readable.',
  'One pass with a real keyboard: Tab through the controls, change the scenario, and see where the focus goes when a switch is refused.',
  'After a push: the dess-gallery-nightly job in perf-nightly.yml runs on its schedule or by hand, and keeps its results for 30 days.'
]

const run = (command, args, env = {}) =>
  spawnSync(command, args, { cwd: root, stdio: 'inherit', env: { ...process.env, ...env } })

if (flag('--list')) {
  for (const [name, what, , , options] of STEPS)
    console.log(`${name.padEnd(20)} ${options?.slow ? 'slow ' : '     '} ${what}`)
  process.exit(0)
}

const only = list('--only')
const skip = list('--skip')
const known = new Set(STEPS.map(([name]) => name))
for (const name of [...only, ...skip]) {
  if (!known.has(name)) {
    console.error(`there is no step called ${name}; --list shows them`)
    process.exit(2)
  }
}
const git = args => spawnSync('git', args, { cwd: root, encoding: 'utf8' }).stdout.trim()
const dirty = git(['status', '--porcelain'])
if (dirty && !flag('--allow-dirty')) {
  console.error(
    `The tree has uncommitted changes, so what this writes would not be acceptance evidence:\n${dirty}\n\nCommit them first, or pass --allow-dirty for a trial run.`
  )
  process.exit(2)
}

const chosen = STEPS.filter(
  ([name, , , , options]) =>
    (only.length ? only.includes(name) : flag('--all') || !options?.slow) && !skip.includes(name)
)

const results = []
for (const [name, what, command, args, options = {}] of chosen) {
  console.log(`\n=== ${name}: ${what}\n    ${[command, ...args].join(' ')}\n`)
  const started = Date.now()
  const ran = run(command, args, options.env)
  const passed = ran.status === 0
  if (passed && options.copyTo)
    fs.copyFileSync(path.join(root, args[args.indexOf('--out') + 1]), path.join(root, options.copyTo))
  results.push({ name, passed, seconds: Math.round((Date.now() - started) / 1000) })
  if (!passed && flag('--stop-on-failure')) break
}

console.log('\n=== summary')
for (const r of results) console.log(`${r.passed ? 'ok  ' : 'FAIL'} ${r.name.padEnd(20)} ${r.seconds} s`)
const skipped = STEPS.filter(([name]) => !chosen.some(([n]) => n === name)).map(([name]) => name)
if (skipped.length)
  console.log(`\nnot run: ${skipped.join(', ')}${flag('--all') || only.length ? '' : '  (--all runs the slow ones)'}`)

const summary = {
  measured_at: new Date().toISOString(),
  git_sha: git(['rev-parse', 'HEAD']),
  git_dirty: dirty.length > 0,
  steps: results
}
fs.mkdirSync(path.join(root, 'target/dess-evidence'), { recursive: true })
fs.writeFileSync(path.join(root, 'target/dess-evidence/summary.json'), `${JSON.stringify(summary, null, 2)}\n`)

console.log('\nTo do by hand, since no script can:')
for (const line of MANUAL) console.log(`  - ${line}`)
console.log(
  '\nNothing was committed. `git status` shows the evidence files that changed, and they are for you to commit.'
)
process.exit(results.every(r => r.passed) ? 0 : 1)
