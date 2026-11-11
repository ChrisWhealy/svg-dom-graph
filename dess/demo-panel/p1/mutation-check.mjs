// Shows that the session coordinator's tests can fail: the coordinator is broken in small ways, one at a time, and a test has to notice each.
//
//   node dess/demo-panel/p1/mutation-check.mjs [--out dess/demo-panel/p1/evidence/mutation-check.json]
//
// A test that passes proves nothing until it is known to fail when the code is wrong. The sources are copied to a tree of their own
// (`target/p1-mutation-tree`) and every mutation is made there, so the working tree is never touched and nothing can be committed mutated. Each
// mutation is built on its own first, so that one that does not compile is reported as that and is never counted as caught. It is caught if the
// tests of `service_sim::session` then fail. It exits with 1 if any is missed. The first run builds the copy from nothing and takes a minute or two.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../../..')
const FILE = 'service-sim/src/session/mod.rs'

// [what is wrong, the text, what it becomes]
const MUTATIONS = [
  [
    'the new scene is not validated after the handoff',
    'read_back.and_then(|()| Self::validate(&candidate))',
    'read_back'
  ],
  [
    'a candidate that is not clean is accepted',
    '        if c.is_dirty() {\n            return Err(',
    '        if false {\n            return Err('
  ],
  [
    'a rolled back candidate is not removed',
    '            self.host.discard(candidate.token);\n        }\n        // The old picture',
    '        }\n        // The old picture'
  ],
  [
    'the old picture is not repaired after a failed handoff',
    '(handed_off || old.is_dirty()).then(|| old.repair())',
    'None'
  ],
  [
    'a failed handoff is not undone',
    '                undo_failed = self\n                    .host\n                    .undo_publish(self.active.as_ref().map(|a| &a.token), &candidate.token)\n                    .is_err();',
    '                undo_failed = false;'
  ],
  [
    'the old playback is not invalidated at the start of a request',
    '        if let Some(old) = &old {\n            old.suspend();\n        }\n',
    ''
  ],
  [
    'the saved speed is not applied',
    '        if let Some(ms) = speed {\n            candidate.controller.set_speed(ms);\n        }\n',
    ''
  ],
  [
    'the old controller is not disposed on commit',
    '        if let Some(previous) = previous {\n            previous.dispose();\n        }\n',
    '        let _ = previous;\n'
  ],
  [
    'the old page is not removed on commit',
    '        if let Some(replaced) = replaced {\n            self.host.finalize(replaced.token);\n        }\n',
    '        let _ = replaced;\n'
  ],
  [
    'a request that is not valid is built anyway',
    '        if let Err(problem) = spec.preflight() {\n            return SwitchOutcome::Rejected(problem);\n        }\n',
    ''
  ],
  [
    'a scenario of the same size builds a new surface',
    '            if active.spec.servers == spec.servers {',
    '            if false {'
  ],
  [
    'a handoff that reports success is not read back',
    '        let read_back = self\n            .host\n            .verify_published(self.active.as_ref().map(|a| &a.token), &candidate.token);\n        if let Err(reason) = read_back.and_then(|()| Self::validate(&candidate)) {',
    '        if let Err(reason) = Self::validate(&candidate) {'
  ],
  [
    'a hook never reaches the current controller',
    '        self.target.upgrade().and_then(|t| t.borrow().clone())',
    '        None'
  ]
]

const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const out = argument('--out') ? path.resolve(argument('--out')) : null
const results = []
const tree = path.join(root, 'target/p1-mutation-tree')
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
if (!fs.existsSync(path.join(tree, FILE))) {
  console.error(`${FILE} does not exist, so no mutation can reach the coordinator`)
  process.exit(2)
}
const original = fs.readFileSync(path.join(tree, FILE), 'utf8')
const stale = MUTATIONS.filter(([, from]) => !original.includes(from)).map(([name]) => name)
if (stale.length > 0) {
  console.error(`these mutations do not apply to the source as it is: ${stale.join('; ')}`)
  process.exit(2)
}
const env = { ...process.env, CARGO_TARGET_DIR: path.join(tree, 'target') }
const cargo = (...args) =>
  spawnSync('cargo', ['test', '-p', 'service-sim', '--lib', ...args], {
    cwd: tree,
    encoding: 'utf8',
    env,
    maxBuffer: 1 << 26
  })

let missed = 0
try {
  for (const [name, from, to] of MUTATIONS) {
    fs.writeFileSync(path.join(tree, FILE), original.replace(from, to))
    const built = cargo('--no-run')
    if (built.status !== 0) {
      missed++
      results.push({ name, built: false, caught: false })
      console.log(`MISSED  ${name}  (it did not build)`)
      console.log(`         ${built.stderr.trim().split('\n').slice(-3).join(' | ').slice(0, 200)}`)
      continue
    }
    const ran = cargo('--', 'session')
    const failed = `${ran.stdout}${ran.stderr}`.split('\n').filter(line => /FAIL \[|^test .* FAILED/.test(line))
    const caught = ran.status !== 0 && failed.length > 0
    if (!caught) missed++
    results.push({ name, built: true, caught, first_failure: failed[0]?.trim().slice(0, 150) ?? null })
    console.log(`${caught ? 'caught ' : 'MISSED '} ${name}`)
    console.log(caught ? `         first failure: ${failed[0].trim().slice(0, 150)}` : `         exit ${ran.status}`)
  }
} finally {
  fs.writeFileSync(path.join(tree, FILE), original)
}
if (out) {
  const git = args => spawnSync('git', args, { cwd: root, encoding: 'utf8' }).stdout.trim()
  fs.mkdirSync(path.dirname(out), { recursive: true })
  fs.writeFileSync(
    out,
    `${JSON.stringify({ measured_at: new Date().toISOString(), git_sha: git(['rev-parse', 'HEAD']), git_dirty: git(['status', '--porcelain']).length > 0, file: FILE, caught: results.filter(r => r.caught).length, total: MUTATIONS.length, mutations: results }, null, 2)}\n`
  )
}
console.log(
  missed
    ? `\n${missed} mutation(s) were not caught`
    : `\n${MUTATIONS.length} of ${MUTATIONS.length} mutations were caught, and the working tree was never touched`
)
process.exit(missed ? 1 : 0)
