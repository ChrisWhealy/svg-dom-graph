// Shows that the gallery panel's acceptance tests can fail: the panel is broken in small ways, one at a time, and the tests have to notice each.
//
//   node dess/demo-panel/p3/mutation-check.mjs [--only 3,5] [--out dess/demo-panel/p3/evidence/mutation-check.json]
//
// A test that passes proves nothing until it is known to fail when the page is wrong. The sources are copied to a tree of their own
// (`target/p3-mutation-tree`), every mutation is made and built there into a gallery of its own, and the real tests are run against that gallery with
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

const CONTROLS = 'demo-app/src/service_sim/controls.rs'
const SWITCHING = 'demo-app/src/service_sim/switching.rs'
const DOM = 'demo-app/src/service_sim/dom.rs'
const LAYOUT = 'service-sim/src/layout.rs'
const SURFACE = 'service-sim/src/surface.rs'

// [what is wrong, [[file, the text, what it becomes], ...]]
const MUTATIONS = [
  [
    'a stop by hiding is not said',
    [[CONTROLS, 'let stopped = playing && shared.hook.controller().is_some_and(|c| !c.is_playing());', 'let stopped = false;']]
  ],
  ['a refused scenario leaves the list on the scenario that was asked for', [[SWITCHING, '                ui.scenario.set_value(&active);\n', '']]],
  [
    'the range keeps the maximum of the scenario before',
    [[SWITCHING, '            ui.seek.set_max(&extent.slices.to_string());\n            shared.interrupted.set(false);', '            shared.interrupted.set(false);']]
  ],
  [
    'the zoom is not carried to the new scene',
    [[DOM, '            next.with_host(|host| host.surface().scene().set_view(view))\n                .map_err(|e| format!("could not carry the view: {e}"))?;', '            let _ = view;']]
  ],
  [
    'a focus that was in the old scene is left there',
    [[DOM, '                    .focus()\n                    .map_err(|e| format!("could not move the focus: {e:?}"))?;', '                    .hidden();']]
  ],
  ['the old scene is not staged when the new one is shown', [[DOM, '            stage(&self.host_element(*from)?)?;\n', '']]],
  ['a deliberate Play does not clear the reason for the pause', [[CONTROLS, '                s.interrupted.set(false);\n                let ms: u32', '                let ms: u32']]],
  ['a candidate that was refused is left in the page', [[DOM, '        self.controllers.remove(&token);\n        self.remove(token);\n    }\n\n    fn finalize', '        self.controllers.remove(&token);\n    }\n\n    fn finalize']]],
  ['the scene that was replaced is left in the page', [[DOM, '    fn finalize(&mut self, token: u32) {\n        self.controllers.remove(&token);\n        self.remove(token);', '    fn finalize(&mut self, token: u32) {\n        self.controllers.remove(&token);']]],
  ['the dispatch node is not at the common midline of the servers', [[LAYOUT, 'let centre = SERVER_TOP + (last as f64 * SERVER_PITCH + SERVER_SIZE.1) / 2.0;', 'let centre = SERVER_TOP + SERVER_SIZE.1 / 2.0 + (last.min(1) as f64) * SERVER_PITCH;']]],
  ['a connector is not forced to enter the west side', [[SURFACE, '.with_to_side(Some(Side::West))', '.with_to_side(None)']]],
  ['the counter values are too close to the longest name', [[LAYOUT, 'pub const COUNTER_VALUE_DX: f64 = 176.0;', 'pub const COUNTER_VALUE_DX: f64 = 120.0;']]],
  ['the data cells are made for one digit', [[LAYOUT, 'pub const DATA_CELL_SAMPLE: u64 = 999_999;', 'pub const DATA_CELL_SAMPLE: u64 = 0;']]],
  ['a refusal is not shown in the error area', [[SWITCHING, '            let _ = ui.error.remove_attribute("hidden");\n            ui.live.set_text_content(Some(&message));', '            ui.live.set_text_content(Some(&message));']]]
]

const tree = path.join(root, 'target/p3-mutation-tree')
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
