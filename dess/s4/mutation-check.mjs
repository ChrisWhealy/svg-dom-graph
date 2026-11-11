// Shows that the service recovery executor can fail: the code that it breaks is broken in small ways, one at a time, and the executor has to
// notice each. A recovery run in which every contract holds shows nothing until it is known to fail when one does not.
//
//   node dess/s4/mutation-check.mjs [--out FILE]
//
// Each mutation is a single edit to a source file of the controller, its browser side, the surface, the library's data setter or the
// executor itself, made on a copy that is put back afterwards, even if the run is interrupted by a signal. After each, the executor
// (`perf/runner/service-recovery.mjs`) is run, which builds the page and breaks the controller at every fault point, and has to exit
// with 1, which means that a gate failed. It exits with 2 if the build failed, which is not a detection and is reported as one that did not
// build. A run is a few minutes, so the whole check is slow, and it needs Chrome.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { locating } from '../lib/locate.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const out = process.argv.includes('--out') ? process.argv[process.argv.indexOf('--out') + 1] : null

const CONTROLLER = 'service-sim/src/controller/mod.rs'
const HOST = 'service-sim/src/host.rs'
const SETTER = 'src/scene/node/data/values.rs'
const EXECUTOR = 'perf-harness-wasm/src/service_recovery.rs'

const RAW_MUTATIONS = [
  [
    CONTROLLER,
    'a draw that failed is committed anyway',
    '            self.full_repair();\n            return Outcome::Failed;\n        }\n        let mut st = self.state.borrow_mut();\n        st.committed_slice',
    '            self.full_repair();\n        }\n        let mut st = self.state.borrow_mut();\n        st.committed_slice'
  ],
  [
    CONTROLLER,
    'the guard after a draw is not asked, so a host that left in the middle of it is committed',
    '        // that went through is no evidence that the page is attached.\n        if !self.live() {',
    '        // that went through is no evidence that the page is attached.\n        if false {'
  ],
  [
    CONTROLLER,
    'a draw that failed is not repaired at once',
    '            // One immediate, bounded repair of what was committed, and no retry after it.\n            self.full_repair();',
    '            // One immediate, bounded repair of what was committed, and no retry after it.'
  ],
  [
    CONTROLLER,
    'a change of speed does not advance the playback epoch, so the timer that it took back is not stale',
    '            st.epoch += 1;\n        }\n        self.schedule_tick()',
    '        }\n        self.schedule_tick()'
  ],
  [CONTROLLER, 'a timer of an old epoch is not stale', 'st.epoch != epoch || ', ''],
  [
    CONTROLLER,
    'a timer that cannot be scheduled leaves playback running',
    '                // The last committed display stays, and playback stops.\n                self.stop_playing();',
    '                // The last committed display stays, and playback stops.'
  ],
  [
    CONTROLLER,
    'a frame that cannot be asked for offers no retry',
    '                    st.retry = st.seek.take().map(|s| s.target);',
    '                    st.seek = None;'
  ],
  [
    CONTROLLER,
    'showing a panel again resumes playback',
    '        self.state.borrow_mut().announced = false;\n        if self.state.borrow().dirty {\n            self.full_repair();\n        }\n    }\n}',
    '        self.state.borrow_mut().announced = false;\n        if self.state.borrow().dirty {\n            self.full_repair();\n        }\n        self.play();\n    }\n}'
  ],
  [
    HOST,
    'a full draw does not write the control again',
    '        if full || self.controls_stale {\n            self.sync_controls().map_err(DrawFailure::Failed)?;\n        }',
    '        if false {\n            self.sync_controls().map_err(DrawFailure::Failed)?;\n        }'
  ],
  [
    HOST,
    'a lifecycle listener is not taken off the page when its owner is dropped',
    '        remove(&self.window, "pagehide", &self.left);\n',
    ''
  ],
  [HOST, 'a cancelled task keeps its closure', 'drop(entry.closure);', 'std::mem::forget(entry.closure);'],
  [
    SETTER,
    'a node whose write failed is not marked, so a call with the committed values changes nothing',
    'let force = handles.decor.texts_stale;',
    'let force = false;'
  ],
  [
    EXECUTOR,
    'the model hash is not the engine at the committed slice',
    '    let mut last = engine.checkpoint(0, 0);',
    '    let mut last = engine.checkpoint(1, 1);'
  ],
  [
    CONTROLLER,
    'a draw that failed corrupts the committed model and leaves its slice number alone',
    '            self.full_repair();\n            return Outcome::Failed;\n        }\n        let mut st = self.state.borrow_mut();',
    '            self.full_repair();\n            self.state.borrow_mut().committed_model.counters.completed += 1;\n            return Outcome::Failed;\n        }\n        let mut st = self.state.borrow_mut();'
  ],
  [
    CONTROLLER,
    'a draw that failed leaves the live engine rewound by a slice, with the controller still recording where it was',
    '            self.full_repair();\n            return Outcome::Failed;\n        }\n        let mut st = self.state.borrow_mut();',
    '            self.full_repair();\n            if let Ok(engine) = self.replay(target.slice.saturating_sub(2)) {\n                self.state.borrow_mut().engine = engine;\n            }\n            return Outcome::Failed;\n        }\n        let mut st = self.state.borrow_mut();'
  ],
  [
    CONTROLLER,
    'a draw that failed moves the cursor and still reports failure',
    '            self.full_repair();\n            return Outcome::Failed;\n        }\n        let mut st = self.state.borrow_mut();',
    '            self.full_repair();\n            self.state.borrow_mut().committed_slice = target.slice;\n            return Outcome::Failed;\n        }\n        let mut st = self.state.borrow_mut();'
  ]
]
const MUTATIONS = RAW_MUTATIONS.map(locating(root))

const originals = new Map(MUTATIONS.map(([file]) => [file, fs.readFileSync(path.join(root, file), 'utf8')]))
// Every mutation has to apply to the source as it is now, and rustfmt can move the text, so this is checked before anything is changed.
const stale = MUTATIONS.filter(([file, , from]) => !originals.get(file).includes(from)).map(([, name]) => name)
if (stale.length > 0) {
  console.error(`these mutations do not apply to the source as it is: ${stale.join('; ')}`)
  process.exit(2)
}

const restore = () => {
  for (const [file, text] of originals) fs.writeFileSync(path.join(root, file), text)
}
// A signal ends this process without running a `finally`, and a source left mutated is a source that is wrong, so it is put back first.
for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) {
  process.on(signal, () => {
    restore()
    process.exit(130)
  })
}

const results = []
try {
  for (const [file, name, from, to] of MUTATIONS) {
    fs.writeFileSync(path.join(root, file), originals.get(file).replace(from, to))
    const report = path.join(os.tmpdir(), `service-mutation-${process.pid}.json`)
    const ran = spawnSync('node', ['perf/runner/service-recovery.mjs', '--out', report], {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: 1 << 28,
      timeout: 1_800_000
    })
    fs.writeFileSync(path.join(root, file), originals.get(file))
    const text = `${ran.stdout}${ran.stderr}`
    const built = ran.status !== 2
    const failed = text
      .split('\n')
      .filter(line => line.startsWith('FAIL'))
      .map(line => line.slice(5, 200))
    const caught = ran.status === 1 && failed.length > 0
    results.push({ name, file, caught, built, status: ran.status, first: failed[0] ?? null })
    console.log(`${caught ? 'caught ' : 'MISSED '} ${name}${built ? '' : '  (it did not build)'}`)
    if (caught) console.log(`         first failure: ${failed[0]}`)
    else
      console.log(
        `         exit status ${ran.status}, ${failed.length} failing lines; last output: ${text.trim().split('\n').slice(-2).join(' | ').slice(0, 200)}`
      )
    fs.rmSync(report, { force: true })
  }
} finally {
  restore()
}
// The page was built from a mutated source in the last run, so it is built again from the sources as they are.
spawnSync('wasm-pack', ['build', 'perf-harness-wasm', '--target', 'web', '--release', '--out-dir', 'pkg'], {
  cwd: root,
  encoding: 'utf8'
})

const missed = results.filter(r => !r.caught)
console.log(
  missed.length
    ? `\n${missed.length} mutation(s) were not caught`
    : `\n${results.length} of ${results.length} mutations were caught, and the sources are restored`
)
if (out) fs.writeFileSync(path.resolve(root, out), `${JSON.stringify({ mutations: results }, null, 2)}\n`)
process.exit(missed.length ? 1 : 0)
