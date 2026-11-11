// Shows that the W6 runners can fail on the product: the code that they measure is broken in small ways, one at a time, and the run has to
// notice each. A clean run proves nothing until it is known to fail when the thing it checks is wrong.
//
//   node dess/s5/mutation-check.mjs [--out FILE] [--prove-build-failure]
//
// The sources are copied to a tree of their own (`target/s5-mutation-tree`) and every mutation is made there, so the working tree is never
// touched and nothing can be committed mutated. Each mutation is a single edit to a file of the copy, put back afterwards. After each, the
// orchestrator is run in the copy on the W6 manifest in the mode that sees the mutation, which rebuilds the page, and it has to exit with 1,
// naming what it saw. A run that cannot build is reported as one that did not build, and is not a detection. The first run builds the copy
// from nothing and takes several minutes, and later ones reuse what it built.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { locating } from '../lib/locate.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const out = process.argv.includes('--out') ? process.argv[process.argv.indexOf('--out') + 1] : null
const MANIFEST = 'perf/manifests/w6-service-simulation-smoke.toml'

const SURFACE = 'service-sim/src/surface.rs'
const CONTROLLER = 'service-sim/src/controller/mod.rs'
const HOST = 'service-sim/src/host.rs'
const EXECUTOR = 'perf-harness-wasm/src/service_perf.rs'
const PAGE = 'perf/runner/page/service-sim.js'

// [file, what is wrong, from, to, the mode that sees it, the layers that see it (b when absent)]
const PROVE_BUILD_FAILURE = process.argv.includes('--prove-build-failure')
const RAW_MUTATIONS = [
  [
    SURFACE,
    "the data node's setter is not called, though a call is counted",
    '        self.scene\n            .set_data_values(self.data_node, NodeValues::U64(projection.data.to_vec()))\n            .map_err(|e| DrawError::Write(e.to_string()))?;\n        self.setter_calls += 1;',
    '        self.setter_calls += 1;',
    'correctness'
  ],
  [
    CONTROLLER,
    'a commit moves the cursor one slice too far',
    '        st.committed_slice = target.slice;\n        st.committed_model = target.model;',
    '        st.committed_slice = target.slice + 1;\n        st.committed_model = target.model;',
    'correctness'
  ],
  [
    CONTROLLER,
    'the slices prepared ahead are one engine step late',
    '            made += 1;\n            st.engine_at = committed + made;',
    '            let _ = st.engine.step();\n            made += 1;\n            st.engine_at = committed + made;',
    'performance'
  ],
  [
    HOST,
    'the controller reads layout whatever it was given',
    '        self.cached_activation.is_none()\n    }',
    '        true\n    }',
    'performance'
  ],
  [
    EXECUTOR,
    'the four slices of a batch are worked out inside the timed interval',
    '        let checkpoints = sim.controller.prepare_ahead_with_checkpoints(u64::from(count))?;',
    '        let checkpoints = vec![String::new(); count as usize];',
    'performance'
  ],
  [
    PAGE,
    'the priming window is one slice short, so every window shifts',
    'const PRIMING = 256',
    'const PRIMING = 255',
    'performance'
  ],
  [
    SURFACE,
    'a slot is never written, though it is counted as drawn',
    'Field::Text => el.text.set_text_content(Some(&slot.text)),',
    'Field::Text => {},',
    'correctness'
  ],
  [
    SURFACE,
    'a slot is never written, and the timing is of the cheaper draw',
    'Field::Text => el.text.set_text_content(Some(&slot.text)),',
    'Field::Text => {},',
    'performance'
  ],
  [
    EXECUTOR,
    'the Seek control has no accessible name',
    '    seek.set_attribute("aria-label", "Seek to slice").map_err(js("seek"))?;\n',
    '',
    'correctness',
    'c'
  ],
  [
    EXECUTOR,
    'a Seek takes the focus away from its control',
    '                        if let Some(target) = value {\n                            c.seek(target);',
    '                        if let Some(target) = value {\n                            if let Some(e) = slider.dyn_ref::<web_sys::HtmlElement>() {\n                                let _ = e.blur();\n                            }\n                            c.seek(target);',
    'correctness',
    'c'
  ]
]
const ALL_MUTATIONS = RAW_MUTATIONS.map(locating(root))

const tree = path.join(root, 'target/s5-mutation-tree')
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
// The schemas' dependencies are the same files, and are linked and not copied.
fs.rmSync(path.join(tree, 'perf/schemas/node_modules'), { recursive: true, force: true })
fs.symlinkSync(path.join(root, 'perf/schemas/node_modules'), path.join(tree, 'perf/schemas/node_modules'))
// With `--prove-build-failure` the only edit is one that cannot compile, and the check passes only if it is reported as not built and not as caught.
const MUTATIONS = PROVE_BUILD_FAILURE
  ? [
      [
        EXECUTOR,
        'a source that does not compile',
        'pub fn state() -> String {',
        'pub fn state() -> String { this does not compile',
        'correctness'
      ]
    ]
  : ALL_MUTATIONS
const originals = new Map(
  [...new Set(MUTATIONS.map(([file]) => file))].map(file => [file, fs.readFileSync(path.join(tree, file), 'utf8')])
)
// Every mutation has to apply to the source as it is now, and rustfmt can move text, so this is checked before anything is changed.
const stale = MUTATIONS.filter(([file, , from]) => !originals.get(file).includes(from)).map(([, name]) => name)
if (stale.length > 0) {
  console.error(`these mutations do not apply to the source as it is: ${stale.join('; ')}`)
  process.exit(2)
}
const restore = () => {
  for (const [file, text] of originals) fs.writeFileSync(path.join(tree, file), text)
}
for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP']) {
  process.on(signal, () => {
    restore()
    process.exit(130)
  })
}

const results = []
try {
  for (const [file, name, from, to, mode, layers = 'b'] of MUTATIONS) {
    fs.writeFileSync(path.join(tree, file), originals.get(file).replace(from, to))
    // The build is its own step, so that whether it completed is read from its own exit status and not from how a later run ended. A mutation
    // that does not build is not a detection, and the orchestrator is not run on it.
    const env = { ...process.env, CARGO_TARGET_DIR: path.join(tree, 'target') }
    const compiled = spawnSync('cargo', ['build', '-q', '--release', '-p', 'perf-harness'], {
      cwd: tree,
      encoding: 'utf8',
      env
    })
    const packaged =
      compiled.status === 0
        ? spawnSync('wasm-pack', ['build', 'perf-harness-wasm', '--target', 'web', '--release', '--out-dir', 'pkg'], {
            cwd: tree,
            encoding: 'utf8',
            env
          })
        : compiled
    const built = compiled.status === 0 && packaged.status === 0
    if (!built) {
      fs.writeFileSync(path.join(tree, file), originals.get(file))
      const why = `${packaged.stderr || packaged.stdout}`.trim().split('\n').slice(-3).join(' | ').slice(0, 240)
      results.push({ name, file, mode, caught: false, built: false, status: packaged.status, first: null })
      console.log(`MISSED  ${name}  (it did not build)`)
      console.log(`         ${why}`)
      continue
    }
    const ran = spawnSync(
      'node',
      [
        'perf/runner/run.mjs',
        MANIFEST,
        '--mode',
        mode,
        '--layers',
        layers,
        '--repetitions',
        '1',
        '--no-build',
        '--out-dir',
        path.join(tree, 'target/perf-mutation')
      ],
      {
        cwd: tree,
        encoding: 'utf8',
        maxBuffer: 1 << 28,
        timeout: 3_600_000,
        env
      }
    )
    fs.writeFileSync(path.join(tree, file), originals.get(file))
    const text = `${ran.stdout}${ran.stderr}`
    const failed = text
      .split('\n')
      .filter(line => /^\s+(FAIL|content:|interaction:|dom_identity:)/.test(line))
      .map(line => line.trim().slice(0, 200))
    // Detected means that it built, and that the orchestrator then failed on a gate or a checkpoint, and said which.
    const caught = built && ran.status === 1 && failed.length > 0
    results.push({ name, file, mode, caught, built, status: ran.status, first: failed[0] ?? null })
    console.log(`${caught ? 'caught ' : 'MISSED '} ${name}`)
    console.log(
      caught
        ? `         first failure: ${failed[0]}`
        : `         exit status ${ran.status}, last output: ${text.trim().split('\n').slice(-2).join(' | ').slice(0, 200)}`
    )
  }
} finally {
  restore()
}
// The working tree was never touched, so there is nothing to build again or to put back.

if (PROVE_BUILD_FAILURE) {
  const only = results[0]
  const ok = results.length === 1 && only.built === false && only.caught === false
  console.log(
    ok
      ? '\na mutation that does not build is reported as not built, and is not a detection'
      : '\na mutation that does not build was taken for a detection'
  )
  process.exit(ok ? 0 : 1)
}
const missed = results.filter(r => !r.caught)
console.log(
  missed.length
    ? `\n${missed.length} mutation(s) were not caught`
    : `\n${results.length} of ${results.length} mutations were caught, and the sources are restored`
)
if (out) fs.writeFileSync(path.resolve(root, out), `${JSON.stringify({ mutations: results }, null, 2)}\n`)
process.exit(missed.length ? 1 : 0)
