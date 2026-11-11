// Shows that the injector's self-test (injector.mjs) can fail: the injector is changed in small ways, one at a time, on a copy, and the self-test has
// to notice each.
//
//   node dess/demo-panel/p0/injector-mutation-check.mjs
//
// A self-test that passes proves nothing until it is known to fail when the injector is wrong. Each mutation is one edit to a copy of
// perf/page/fault-injector.js, made in a temporary folder, so the repository's injector is never touched. The self-test is run with the copy
// (`P0_INJECTOR`), skipping the graph recovery self-test, which is not what is being tested here. A mutation is caught if the self-test exits with 1
// and says which check failed. It exits with 1 if any is missed.
//
//   wrong ordinal       the detach fires one call late.
//   never fires         the detach is armed and never fires.
//   fires every call    the detach fires at the nth call and again at every eligible call after it, so that it fires more than once.
//   counting skipped    the counting of calls is skipped while a detach is armed, which changes what recording and an armed exception see.
//   exception ignored   an exception armed beside a detach is ignored.
//   leaves the host     disarming does not forget the detach, so that it is left armed.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../../..')
const original = fs.readFileSync(path.join(root, 'perf/page/fault-injector.js'), 'utf8')

const MUTATIONS = [
  ['wrong ordinal', 'if (detach.seen !== detach.nth) return', 'if (detach.seen !== detach.nth + 1) return'],
  ['never fires', 'if (detach.fired || method !== detach.method', 'if (true || method !== detach.method'],
  [
    'fires every call',
    ['if (detach.fired || method !== detach.method', 'if (method !== detach.method'],
    ['if (detach.seen !== detach.nth) return', 'if (detach.seen < detach.nth) return']
  ],
  [
    'counting skipped',
    '  const count = (method, target, args) => {\n    if (!recording && !armed) return',
    '  const count = (method, target, args) => {\n    if (detach !== null) return\n    if (!recording && !armed) return'
  ],
  [
    'exception ignored',
    '      if (armed.counted >= armed.index && armed.counted < armed.index + armed.times) {',
    '      if (detach === null && armed.counted >= armed.index && armed.counted < armed.index + armed.times) {'
  ],
  ['leaves the host', '      detach = null\n      return report', '      return report']
]

const work = fs.mkdtempSync(path.join(os.tmpdir(), 'injector-mutation-'))
let missed = 0
try {
  for (const [name, ...edits] of MUTATIONS) {
    // A mutation is one edit, or a list of edits that are made together.
    const pairs = Array.isArray(edits[0]) ? edits : [edits]
    const missing = pairs.find(([from]) => !original.includes(from))
    if (missing) {
      console.log(`MISSED  ${name}: the text to change is not in the injector, so the mutation does not apply`)
      missed++
      continue
    }
    const copy = path.join(work, `${name.replaceAll(' ', '-')}.js`)
    fs.writeFileSync(
      copy,
      pairs.reduce((text, [from, to]) => text.replace(from, to), original)
    )
    const ran = spawnSync(
      'node',
      [
        path.join(root, 'dess/demo-panel/p0/injector.mjs'),
        '--skip-recovery',
        '--out',
        path.join(work, 'evidence.json')
      ],
      {
        cwd: root,
        encoding: 'utf8',
        env: { ...process.env, P0_INJECTOR: copy },
        maxBuffer: 1 << 26,
        timeout: 600_000
      }
    )
    const failed = ran.stdout.split('\n').filter(line => line.startsWith('FAIL'))
    const caught = ran.status === 1 && failed.length > 0
    if (!caught) missed++
    console.log(`${caught ? 'caught ' : 'MISSED '} ${name}`)
    console.log(
      caught
        ? `         first failure: ${failed[0].slice(5, 170)}`
        : `         exit ${ran.status}, ${ran.stdout.trim().split('\n').slice(-2).join(' | ').slice(0, 160)}`
    )
  }
} finally {
  fs.rmSync(work, { recursive: true, force: true })
}
console.log(
  missed
    ? `\n${missed} mutation(s) were not caught`
    : `\n${MUTATIONS.length} of ${MUTATIONS.length} mutations were caught, and the repository's injector was never touched`
)
process.exit(missed ? 1 : 0)
