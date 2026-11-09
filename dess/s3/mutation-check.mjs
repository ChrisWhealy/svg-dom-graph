// Shows that the controller's tests can fail: the controller and its browser side are broken in small ways, one at a time, and a test has
// to notice each. A test that passes proves nothing until it is known to fail on a fault.
//
//   node dess/s3/mutation-check.mjs [--out FILE]
//
// Each mutation is a single edit to a source file of `service-sim`, made on a copy that is put back afterwards, even if the run is
// interrupted. A native mutation runs the library's tests, with a fake host and a manual clock. A browser mutation runs the controller's
// tests in Chrome, with `wasm-pack test --headless --chrome service-sim --test controller_browser`, and needs Chrome, as they do.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../..')
const out = process.argv.includes('--out') ? process.argv[process.argv.indexOf('--out') + 1] : null

const CONTROLLER = 'service-sim/src/controller/mod.rs'
const HOST = 'service-sim/src/host.rs'

const MUTATIONS = [
  [
    CONTROLLER,
    'the guard after a draw is not asked, so a host that left in the middle of it is committed',
    '        // that went through is no evidence that the page is attached.\n        if !self.live() {',
    '        // that went through is no evidence that the page is attached.\n        if false {',
    'native'
  ],
  [
    CONTROLLER,
    'a draw that failed is committed anyway',
    '            self.full_repair();\n            return Outcome::Failed;\n        }\n        let mut st = self.state.borrow_mut();\n        st.committed_slice',
    '            self.full_repair();\n        }\n        let mut st = self.state.borrow_mut();\n        st.committed_slice',
    'native'
  ],
  [
    CONTROLLER,
    'a draw that failed is not repaired at once',
    '            // One immediate, bounded repair of what was committed, and no retry after it.\n            self.full_repair();',
    '            // One immediate, bounded repair of what was committed, and no retry after it.',
    'native'
  ],
  [
    CONTROLLER,
    'a stop does not advance the playback epoch',
    '            st.epoch += 1;\n            st.seek_token += 1;\n            st.seek = None;',
    '            st.seek_token += 1;\n            st.seek = None;',
    'native'
  ],
  [CONTROLLER, 'a timer of an old epoch is not stale', 'st.epoch != epoch || ', '', 'native'],
  [CONTROLLER, 'a frame of an old seek is not stale', 'st.seek_token != token || ', '', 'native'],
  [
    CONTROLLER,
    'showing a panel again resumes playback',
    '        self.state.borrow_mut().announced = false;\n        if self.state.borrow().dirty {\n            self.full_repair();\n        }\n    }\n}',
    '        self.state.borrow_mut().announced = false;\n        if self.state.borrow().dirty {\n            self.full_repair();\n        }\n        self.play();\n    }\n}',
    'native'
  ],
  [
    CONTROLLER,
    'a request with nowhere to go does not repair a dirty page',
    '        if self.state.borrow().dirty {\n            self.full_repair();\n        }\n        Outcome::AtBoundary',
    '        Outcome::AtBoundary',
    'native'
  ],
  [
    CONTROLLER,
    'a slice that was staged is stepped again after a failed draw',
    'st.staged.take().filter(|s| s.slice == target)',
    'st.staged.take().filter(|_| false)',
    'native'
  ],
  [
    CONTROLLER,
    'the forced one-slice budget is ignored',
    'let budget = st.chunk.unwrap_or(u64::MAX);',
    'let budget = u64::MAX;',
    'native'
  ],
  [
    CONTROLLER,
    'a speed change does not replace the timer',
    '        self.inner.restart_timer()\n    }',
    '        Outcome::Done\n    }',
    'native'
  ],
  [
    CONTROLLER,
    'a failure is reported every time and not once',
    '            !std::mem::replace(&mut st.reported, true)',
    '            true',
    'native'
  ],
  [
    CONTROLLER,
    'the host is announced as unavailable every time and not once',
    '            !std::mem::replace(&mut st.announced, true)',
    '            true',
    'native'
  ],
  [
    CONTROLLER,
    'a pause leaves its timer waiting',
    '            st.epoch += 1;\n            (st.timer.take(), std::mem::replace(&mut st.playing, false))',
    '            st.epoch += 1;\n            (None, std::mem::replace(&mut st.playing, false))',
    'native'
  ],
  [
    CONTROLLER,
    'a step does not cancel playback or a pending seek',
    '    pub fn next(&self) -> Outcome {\n        self.inner.claim_navigation();',
    '    pub fn next(&self) -> Outcome {',
    'native'
  ],
  [
    CONTROLLER,
    'a seek does not cancel what was pending before it looks at its target',
    '    pub fn seek(&self, target: u64) -> Outcome {\n        self.inner.claim_navigation();',
    '    pub fn seek(&self, target: u64) -> Outcome {',
    'native'
  ],
  [
    CONTROLLER,
    'play does not cancel a pending seek',
    '    pub fn play(&self) -> Outcome {\n        self.inner.claim_navigation();',
    '    pub fn play(&self) -> Outcome {',
    'native'
  ],
  [
    CONTROLLER,
    'restart does not cancel what was pending before it reaches a boundary',
    '    pub fn restart(&self) -> Outcome {\n        self.inner.claim_navigation();',
    '    pub fn restart(&self) -> Outcome {',
    'native'
  ],
  [
    CONTROLLER,
    'an engine error during a seek is taken for the end of the run',
    '                    Err(e) => {\n                        failed = Some(e.message());\n                        break;',
    '                    Err(e) => {\n                        let _ = e;\n                        failed = None::<String>;\n                        ended = true;\n                        break;',
    'native'
  ],
  [HOST, 'a hidden panel is taken for a live one', 'd.hidden()) || !self.rendered() {', 'd.hidden()) {', 'browser'],
  [HOST, 'a cancelled task keeps its closure', 'drop(entry.closure);', 'std::mem::forget(entry.closure);', 'browser'],
  [
    HOST,
    'a closure that has run is never released',
    'tasks.borrow_mut().finished.clear();',
    'let _ = &tasks;',
    'browser'
  ]
]

const originals = new Map(MUTATIONS.map(([file]) => [file, fs.readFileSync(path.join(root, file), 'utf8')]))
// Every mutation has to apply to the source as it is now, and rustfmt can move the text, so this is checked before anything is changed.
const stale = MUTATIONS.filter(([file, , from]) => !originals.get(file).includes(from)).map(([, name]) => name)
if (stale.length > 0) {
  console.error(`these mutations do not apply to the source as it is: ${stale.join('; ')}`)
  process.exit(2)
}

const COMMANDS = {
  native: ['cargo', 'test', '-p', 'service-sim', '--lib'],
  browser: ['wasm-pack', 'test', '--headless', '--chrome', 'service-sim', '--test', 'controller_browser']
}

const results = []
try {
  for (const [file, name, from, to, kind] of MUTATIONS) {
    fs.writeFileSync(path.join(root, file), originals.get(file).replace(from, to))
    const args = COMMANDS[kind]
    const run = spawnSync(args[0], args.slice(1), { cwd: root, encoding: 'utf8', timeout: 900_000 })
    fs.writeFileSync(path.join(root, file), originals.get(file))
    const text = `${run.stdout}${run.stderr}`
    const failed = text
      .split('\n')
      .filter(line => line.startsWith('test ') && (line.endsWith(' FAILED') || line.endsWith(' FAIL')))
      .map(line => line.slice(5).split(' ... ')[0])
    const built = !text.includes('could not compile') && !text.includes('Compilation of your program failed')
    const caught = failed.length > 0
    results.push({ name, kind, caught, failed, built })
    console.log(`${caught ? 'caught ' : 'MISSED '} ${name}${built ? '' : '  (it did not build)'}`)
    if (caught) console.log(`         first test to fail: ${failed[0]}`)
  }
} finally {
  for (const [file, text] of originals) fs.writeFileSync(path.join(root, file), text)
}

const missed = results.filter(r => !r.caught)
console.log(`${results.length - missed.length} of ${results.length} mutations were caught, and the source is restored`)
if (out) fs.writeFileSync(path.resolve(root, out), `${JSON.stringify({ mutations: results }, null, 2)}\n`)
process.exit(missed.length === 0 ? 0 : 1)
