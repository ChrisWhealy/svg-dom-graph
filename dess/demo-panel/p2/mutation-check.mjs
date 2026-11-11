// Shows that the gallery panel's acceptance tests can fail: the panel is broken in small ways, one at a time, and G01 to G10 have to notice each.
//
//   node dess/demo-panel/p2/mutation-check.mjs [--only 3,5] [--out dess/demo-panel/p2/evidence/mutation-check.json]
//
// A test that passes proves nothing until it is known to fail when the page is wrong. The sources are copied to a tree of their own
// (`target/p2-mutation-tree`), every mutation is made and built there into a gallery of its own, and the real tests are run against that gallery with
// `--stage`, so the working tree is never touched and nothing can be committed mutated. A mutation that does not build is reported as that and is never
// counted as caught. It is caught if the tests then fail. The first run builds the whole demo in the copy, which takes several minutes, and each
// mutation after it rebuilds the demo and runs the ten tests, which takes about a minute and a half.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const out = argument('--out') ? path.resolve(argument('--out')) : null
const only = argument('--only')?.split(',').map(Number) ?? null

const CONTROLS = 'demo-app/src/service_sim/controls.rs'
const MOD = 'demo-app/src/service_sim/mod.rs'
const PROJECTION = 'service-sim/src/projection/mod.rs'

// [what is wrong, [[file, the text, what it becomes], ...]]
const MUTATIONS = [
  [
    'Next is not marked as at the last snapshot when it is',
    [[CONTROLS, 'slice >= end, "Already at last snapshot"', 'slice > end, "Already at last snapshot"']]
  ],
  [
    'a boundary button gets the HTML disabled attribute',
    [[CONTROLS, 'button.set_attribute("aria-disabled", "true")', 'button.set_attribute("disabled", "true")']]
  ],
  [
    'Play at the end asks for playback and not a step',
    [[CONTROLS, '} else if controller.committed_slice() >= s.end.get() {', '} else if false {']]
  ],
  [
    'each seek request is made twice',
    [
      [
        CONTROLS,
        'request(&s, |c, _| c.seek(target), "Already there");',
        'request(\n                &s,\n                |c, _| {\n                    c.seek(target);\n                    c.seek(target)\n                },\n                "Already there",\n            );'
      ]
    ]
  ],
  [
    'the range stops saying Snapshot N of M',
    [[CONTROLS, '    let _ = ui.seek.set_attribute("aria-valuetext", &format!("Snapshot {shown} of {end}"));\n', '']]
  ],
  [
    'the range ends one snapshot too late',
    [[MOD, 'shared.end.set(extent.slices);', 'shared.end.set(extent.slices + 1);']]
  ],
  [
    'a click on Previous at the first snapshot is swallowed',
    [
      [
        CONTROLS,
        'request(&s, |c, _| c.previous(), "Already at the first snapshot");',
        'if s.ui.previous.get_attribute("aria-disabled").is_none() {\n                request(&s, |c, _| c.previous(), "Already at the first snapshot");\n            }'
      ]
    ]
  ],
  ['choosing a job selects none', [[CONTROLS, 'c.select_job(chosen)', 'c.select_job(None)']]],
  [
    'Repair restarts instead',
    [
      [
        CONTROLS,
        '                    shared.repaired.set(true);\n                    c.repair()',
        '                    shared.repaired.set(true);\n                    c.restart()'
      ]
    ]
  ],
  [
    'the chosen speed is not given to the controller',
    [
      [CONTROLS, '                controller.set_speed(ms);\n', ''],
      [CONTROLS, '            controller.set_speed(s.ui.speed.value().parse().unwrap_or(1_000));\n', '']
    ]
  ],
  [
    'the stale and no-op counters change places',
    [[PROJECTION, '        c.stale,\n        c.noops,', '        c.noops,\n        c.stale,']]
  ],
  [
    'a failed draw is not said',
    [
      [
        CONTROLS,
        'Outcome::Failed => announce(ui, "The display could not be updated. Use Repair display."),',
        'Outcome::Failed => {},'
      ]
    ]
  ],
  [
    'a failed write of the Play control is not counted as needing repair',
    [[CONTROLS, 'controller.is_dirty() || controller.with_host(|host| host.controls_stale())', 'controller.is_dirty()']]
  ],
  [
    'a failed write of a boundary attribute is not counted as needing repair',
    [
      [
        CONTROLS,
        ' || controller.with_host(|host| host.controls_stale()) || boundary_failed;',
        ' || controller.with_host(|host| host.controls_stale());'
      ]
    ]
  ],
  [
    'a failure is never cleared from the page',
    [[CONTROLS, 'if !dirty && (committed || shared.repaired.replace(false)) {', 'if false {']]
  ]
]

const tree = path.join(root, 'target/p2-mutation-tree')
fs.mkdirSync(tree, { recursive: true })
const copied = spawnSync(
  'rsync',
  [
    '-a',
    '--delete',
    '--exclude',
    '/target',
    '--exclude',
    '/.git',
    '--exclude',
    'node_modules',
    '--exclude',
    '/perf-harness-wasm/pkg',
    `${root}/`,
    `${tree}/`
  ],
  { encoding: 'utf8' }
)
if (copied.status !== 0) {
  console.error(`copying the sources failed: ${copied.stderr}`)
  process.exit(2)
}
const originals = new Map()
for (const file of new Set(MUTATIONS.flatMap(([, edits]) => edits.map(([f]) => f)))) {
  if (!fs.existsSync(path.join(tree, file))) {
    console.error(`${file} does not exist, so no mutation can reach it`)
    process.exit(2)
  }
  originals.set(file, fs.readFileSync(path.join(tree, file), 'utf8'))
}
const stale = MUTATIONS.filter(([, edits]) => edits.some(([f, from]) => !originals.get(f).includes(from))).map(
  ([name]) => name
)
if (stale.length > 0) {
  console.error(`these mutations do not apply to the source as it is: ${stale.join('; ')}`)
  process.exit(2)
}

const build = () =>
  spawnSync('cargo', ['run', '-p', 'demo-server', '--', '--build-only'], {
    cwd: tree,
    encoding: 'utf8',
    maxBuffer: 1 << 27
  })
const results = []
let missed = 0
try {
  // The copy keeps the times of the files that it was copied from, which can be older than a build of an earlier mutation that is still there, and
  // cargo would then take the mutated build for a fresh one. Writing the files again gives them the time of now.
  for (const [file, text] of originals) fs.writeFileSync(path.join(tree, file), text)
  // The unmutated copy has to pass, or a failure below says nothing.
  const clean = build()
  if (clean.status !== 0) {
    console.error(`the unmutated copy did not build:\n${clean.stderr.slice(-1500)}`)
    process.exit(2)
  }
  const control = spawnSync(
    'node',
    [path.join(root, 'dess/demo-panel/test-gallery.mjs'), '--stage', path.join(tree, 'target/demo-stage')],
    { encoding: 'utf8', maxBuffer: 1 << 27 }
  )
  if (control.status !== 0) {
    console.error(`the tests fail on the unmutated copy, so a failure proves nothing:\n${control.stdout.slice(-1500)}`)
    process.exit(2)
  }
  console.log('the unmutated copy builds and passes all of G01 to G10')
  for (const [index, [name, edits]] of MUTATIONS.entries()) {
    if (only && !only.includes(index + 1)) continue
    for (const [file, from, to] of edits) {
      fs.writeFileSync(path.join(tree, file), fs.readFileSync(path.join(tree, file), 'utf8').replace(from, to))
    }
    const built = build()
    if (built.status !== 0) {
      missed++
      results.push({ name, built: false, caught: false })
      console.log(`MISSED  ${index + 1}. ${name}  (it did not build)`)
      console.log(`         ${built.stderr.trim().split('\n').slice(-3).join(' | ').slice(0, 240)}`)
    } else {
      const ran = spawnSync(
        'node',
        [path.join(root, 'dess/demo-panel/test-gallery.mjs'), '--stage', path.join(tree, 'target/demo-stage')],
        { encoding: 'utf8', maxBuffer: 1 << 27 }
      )
      const failures = ran.stdout.split('\n').filter(line => line.includes('FAIL'))
      const caught = ran.status !== 0 && failures.length > 0
      if (!caught) missed++
      results.push({ name, built: true, caught, first_failure: failures[0]?.trim().slice(0, 200) ?? null })
      console.log(`${caught ? 'caught ' : 'MISSED '} ${index + 1}. ${name}`)
      console.log(
        caught ? `         first failure: ${failures[0].trim().slice(0, 170)}` : `         exit ${ran.status}`
      )
    }
    for (const [file] of edits) fs.writeFileSync(path.join(tree, file), originals.get(file))
  }
} finally {
  for (const [file, text] of originals) fs.writeFileSync(path.join(tree, file), text)
}
if (out) {
  const git = args => spawnSync('git', args, { cwd: root, encoding: 'utf8' }).stdout.trim()
  fs.mkdirSync(path.dirname(out), { recursive: true })
  fs.writeFileSync(
    out,
    `${JSON.stringify({ measured_at: new Date().toISOString(), git_sha: git(['rev-parse', 'HEAD']), git_dirty: git(['status', '--porcelain']).length > 0, caught: results.filter(r => r.caught).length, total: results.length, mutations: results }, null, 2)}\n`
  )
}
console.log(
  missed
    ? `\n${missed} mutation(s) were not caught`
    : `\n${results.length} of ${results.length} mutations were caught, and the working tree was never touched`
)
process.exit(missed ? 1 : 0)
