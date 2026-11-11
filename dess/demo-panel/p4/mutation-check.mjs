// Shows that the gallery panel's acceptance tests can fail: the panel is broken in small ways, one at a time, and the tests have to notice each.
//
//   node dess/demo-panel/p4/mutation-check.mjs [--only 3,5] [--out dess/demo-panel/p4/evidence/mutation-check.json]
//
// A test that passes proves nothing until it is known to fail when the page is wrong. The sources are copied to a tree of their own
// (`target/p4-mutation-tree`), every mutation is made and built there into a gallery of its own, and the real tests are run against that gallery with
// `--stage`, so the working tree is never touched and nothing can be committed mutated. A mutation that does not build is reported as that and is never
// counted as caught. It is caught if the tests then fail. The first run builds the whole demo in the copy, which takes several minutes, and each
// mutation after it rebuilds the demo and runs the tests, which takes about a minute and a half.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const out = argument('--out') ? path.resolve(argument('--out')) : null
const only = argument('--only')?.split(',').map(Number) ?? null

const MOD = 'demo-app/src/service_sim/mod.rs'
const LIB = 'demo-app/src/lib.rs'
const PROJECTION = 'service-sim/src/projection/mod.rs'

// [what is wrong, [[file, the text, what it becomes], ...]]
const MUTATIONS = [
  [
    'taking the panel down leaves the page lifecycle listeners on',
    [[MOD, '        drop(lifecycle);', '        std::mem::forget(lifecycle);']]
  ],
  [
    'taking the panel down leaves the watch on the gallery stage on',
    [[MOD, '        drop(gallery);', '        std::mem::forget(gallery);']]
  ],
  [
    "taking the panel down leaves the controls' listeners on",
    [[MOD, '        drop(listeners);', '        std::mem::forget(listeners);']]
  ],
  [
    'taking the panel down leaves the controller and the scene',
    [[MOD, '        shared.coordinator.borrow_mut().dispose();', '        let _ = &shared;']]
  ],
  [
    'taking the panel down leaves what the build wrote into the markup',
    [[MOD, 'for id in ["dess-scenario", "dess-job", "dess-counters"] {', 'for id in ["dess-job"] {']]
  ],
  [
    'the panel is not marked as unbuilt when it is taken down',
    [
      [
        LIB,
        '    section\n        .remove_attribute(PANEL_STATE_ATTR)\n        .map_err(|e| JsValue::from_str(&format!("{e:?}")))?;',
        '    let _ = &section;'
      ]
    ]
  ],
  [
    'taking the panel down leaks one JavaScript object a time, so that the table of externrefs does not grow',
    [
      [
        MOD,
        '    let panel = PANEL.with(|panel| panel.borrow_mut().take());',
        '    let panel = PANEL.with(|panel| panel.borrow_mut().take());\n    std::mem::forget(document.clone());'
      ]
    ]
  ],
  [
    'a callback that is kept after the panel is taken down changes a text and adds no element',
    [
      [
        MOD,
        '    let panel = PANEL.with(|panel| panel.borrow_mut().take());',
        '    let panel = PANEL.with(|panel| panel.borrow_mut().take());\n    if let Some(window) = web_sys::window() {\n        let stale = wasm_bindgen::closure::Closure::once_into_js(|| {\n            if let Some(live) = web_sys::window().and_then(|w| w.document()).and_then(|d| d.get_element_by_id("dess-live")) {\n                live.set_text_content(Some("stale"));\n            }\n        });\n        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(stale.unchecked_ref(), 60_000);\n    }'
      ]
    ]
  ],
  [
    'a callback that is kept after the panel is taken down asks for a timer',
    [
      [
        MOD,
        '    let panel = PANEL.with(|panel| panel.borrow_mut().take());',
        '    let panel = PANEL.with(|panel| panel.borrow_mut().take());\n    if let Some(window) = web_sys::window() {\n        let again = wasm_bindgen::closure::Closure::once_into_js(|| {\n            if let Some(window) = web_sys::window() {\n                let nothing = wasm_bindgen::closure::Closure::once_into_js(|| {});\n                let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(nothing.unchecked_ref(), 60_000);\n            }\n        });\n        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(again.unchecked_ref(), 60_000);\n    }'
      ]
    ]
  ],
  [
    'the stale and no-op counters change places',
    [[PROJECTION, '        c.stale,\n        c.noops,', '        c.noops,\n        c.stale,']]
  ]
]

const tree = path.join(root, 'target/p4-mutation-tree')
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
    [
      path.join(root, 'dess/demo-panel/test-gallery.mjs'),
      '--stage',
      path.join(tree, 'target/demo-stage'),
      '--nightly',
      '--cycles',
      '3'
    ],
    { encoding: 'utf8', maxBuffer: 1 << 27 }
  )
  if (control.status !== 0) {
    console.error(`the tests fail on the unmutated copy, so a failure proves nothing:\n${control.stdout.slice(-1500)}`)
    process.exit(2)
  }
  console.log('the unmutated copy builds and passes every test')
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
        [
          path.join(root, 'dess/demo-panel/test-gallery.mjs'),
          '--stage',
          path.join(tree, 'target/demo-stage'),
          '--nightly',
          '--cycles',
          '3'
        ],
        { encoding: 'utf8', maxBuffer: 1 << 27, timeout: 600000, killSignal: 'SIGTERM' }
      )
      const failures = ran.stdout.split('\n').filter(line => line.includes('FAIL'))
      const caught = ran.status !== 0 && failures.length > 0
      if (!caught) missed++
      results.push({ name, built: true, caught, first_failure: failures[0]?.trim().slice(0, 200) ?? null })
      console.log(`${caught ? 'caught ' : 'MISSED '} ${index + 1}. ${name}`)
      console.log(
        caught
          ? `         first failure: ${failures[0].trim().slice(0, 170)}`
          : `         ${ran.error?.code === 'ETIMEDOUT' ? 'the tests did not finish in 10 minutes' : `exit ${ran.status}`}`
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
