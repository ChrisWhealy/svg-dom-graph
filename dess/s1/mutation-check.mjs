// Shows that the differential run can fail: the Rust engine is broken in thirteen small ways, one at a time, and the differential run has
// to find each one. A comparison that agrees proves nothing until it is known to notice a disagreement.
//
//   node dess/s1/mutation-check.mjs [--out FILE]
//
// Each mutation is a single edit to `service-sim-engine/src/engine.rs`, made on a copy that is put back afterwards, even if the
// run is interrupted. The engine is rebuilt for each, so a run takes a few minutes.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../..')
const file = path.join(root, 'service-sim-engine/src/engine.rs')
const original = fs.readFileSync(file, 'utf8')
const out = process.argv.includes('--out') ? process.argv[process.argv.indexOf('--out') + 1] : null

const MUTATIONS = [
  [
    'the deferral cap is one too few',
    'if self.jobs[id].defers > self.config.deferral_cap {',
    'if self.jobs[id].defers >= self.config.deferral_cap {'
  ],
  ['dispatch ignores the queue entry tick', '                .then(a.entered.cmp(&b.entered))\n', ''],
  [
    'the backoff is half what it should be',
    'let backoff = 2u64 << (self.jobs[id].failures - 1);',
    'let backoff = 1u64 << (self.jobs[id].failures - 1);'
  ],
  [
    'a cancel with nothing to act on is counted as stale',
    'Some(job) => (Outcome::NoOp, job as i64),',
    'Some(job) => (Outcome::Stale, job as i64),'
  ],
  [
    'an aborted attempt keeps its token',
    '                let id = job as usize;\n                self.jobs[id].token += 1;\n                self.jobs[id].failures += 1;',
    '                let id = job as usize;\n                self.jobs[id].failures += 1;'
  ],
  [
    'a duplicate scripted key is not refused before the run',
    'if !seen.insert(key.as_str()) {',
    'if !seen.insert(key.as_str()) && false {'
  ],
  // Unobservable through the engine's interface: the validation before the run refuses a duplicate key first, so the guard in the
  // arrival pop, which is there in case a scenario reaches it some other way, is never reached.
  [
    'the guard against a rebound key at the arrival pop is off (equivalent)',
    'if self.by_key.contains_key(&key) {',
    'if false {',
    true
  ],
  // Unobservable: a terminal job fails the state check of every late event, and is not in the checkpoint. It is kept to show that
  // the check reports an equivalent mutant as one, and does not pretend that it was caught.
  [
    'a terminal job keeps its token (equivalent)',
    '        job.state = state;\n        job.token += 1;',
    '        job.state = state;',
    true
  ],
  ['a cancel runs after a deadline', 'Cancel = 2,\n    Deadline = 3,', 'Cancel = 3,\n    Deadline = 2,'],
  [
    'a failure of a server that is down counts as effective',
    'Server::Down => Ok((Outcome::NoOp, server as i64)),',
    'Server::Down => Ok((Outcome::Effective, server as i64)),'
  ],
  [
    'the checkpoint leaves out the deferral count',
    'w.u64(j.token).u32(j.defers as usize).u64(j.entered);',
    'w.u64(j.token).u64(j.entered);'
  ],
  ['a retry that is queued keeps its deferral count', '                self.jobs[id].defers = 0;\n', ''],
  [
    'the queue holds one job too many',
    'if self.waiting.len() < self.config.queue_capacity {\n            self.jobs[id].state = State::Queued;\n            self.jobs[id].entered = tick;\n            self.waiting.push(id as u64);',
    'if self.waiting.len() <= self.config.queue_capacity {\n            self.jobs[id].state = State::Queued;\n            self.jobs[id].entered = tick;\n            self.waiting.push(id as u64);'
  ]
]

const build = () =>
  spawnSync('cargo', ['build', '--release', '-p', 'service-sim-engine'], { cwd: root, encoding: 'utf8' })
const differential = () =>
  spawnSync('node', [path.join(root, 'dess/s1/differential.mjs'), '--seeds', '150'], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 1 << 28
  })

// Every mutation has to apply to the source as it is now, and rustfmt can move the text, so this is checked before anything is changed.
const stale = MUTATIONS.filter(([, from]) => !original.includes(from)).map(([name]) => name)
if (stale.length > 0) {
  console.error(`these mutations do not apply to engine.rs as it is: ${stale.join('; ')}`)
  process.exit(2)
}

const results = []
try {
  for (const [name, from, to, equivalent = false] of MUTATIONS) {
    if (!original.includes(from)) throw new Error(`the mutation "${name}" does not apply: its text is not in engine.rs`)
    fs.writeFileSync(file, original.replace(from, to))
    const built = build()
    if (built.status !== 0) {
      results.push({
        name,
        caught: false,
        equivalent,
        detail: `did not build: ${built.stderr.split('\n').find(l => l.startsWith('error'))}`
      })
      continue
    }
    const ran = differential()
    const first = ran.stdout
      .split('\n')
      .find(l => l.startsWith('  - '))
      ?.trim()
      .slice(2, 190)
    results.push({
      name,
      caught: ran.status === 1,
      equivalent,
      detail: first ?? ran.stdout.split('\n').slice(-3, -1).join(' ')
    })
    console.log(
      `${ran.status === 1 ? 'caught ' : equivalent ? 'equivalent: not observable, ' : 'MISSED '} ${name}${first ? `\n         first difference: ${first}` : ''}`
    )
  }
} finally {
  fs.writeFileSync(file, original)
  build()
}
const missed = results.filter(r => !r.caught && !r.equivalent)
console.log(
  missed.length
    ? `\n${missed.length} mutation(s) were not caught: the differential run cannot be trusted`
    : `\n${results.filter(r => r.caught).length} of ${results.length} mutations were caught, ${results.filter(r => r.equivalent && !r.caught).length} is equivalent (unobservable), and the engine is restored`
)
if (out) fs.writeFileSync(out, JSON.stringify({ mutations: results }, null, 2) + '\n')
process.exit(missed.length ? 1 : 0)
