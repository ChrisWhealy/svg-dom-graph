// Tests promote-from-run.mjs on synthetic runs, with no browser and no GitHub: the artifact is a folder that is already unpacked, and every
// promotion is a dry run into a scratch copy of the baselines, so nothing in perf/baselines/ is written.
//
//   node perf/runner/test-promote-from-run.mjs
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { collection, HEAD } from '../lib/synthetic-runs.mjs'
import { survey } from './promote-from-run.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const script = path.join(root, 'perf/runner/promote-from-run.mjs')
let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}
const cli = (...args) => spawnSync('node', [script, ...args], { cwd: root, encoding: 'utf8' })
const before = fs.existsSync(path.join(root, 'perf/baselines'))
  ? fs.readdirSync(path.join(root, 'perf/baselines')).sort().join()
  : ''

const work = fs.mkdtempSync(path.join(os.tmpdir(), 'test-promote-run-'))
// `gh run download --name` may put the files one level down, in a folder of the artifact's name, and the script finds them either way.
const artifact = path.join(work, `perf-smoke-${HEAD}`)
collection(path.join(artifact, 'w2-connector-rerouting-smoke'), 'w2', { runs: 3 })
collection(path.join(artifact, 'w3-dirty-smoke'), 'w3', {
  runs: 3,
  environment: { git_dirty: true },
  manifest: { id: 'w3-dirty-smoke' }
})

const found = survey(artifact, 'performance')
check(
  found.length === 2 && found.every(w => w.layers.join() === 'B' && w.commits.join() === HEAD),
  `the workloads and their layers are read from the results themselves: ${found.map(w => `${w.workload} ${w.layers}`).join(', ')}`
)

const clean = cli('--from-dir', path.join(work, 'only-clean-parent'), '--reason', 'x')
check(clean.status !== 0, 'a folder that does not exist is an error')

const mixed = cli('--from-dir', work, '--reason', 'the first baseline', '--dry-run')
check(
  mixed.status === 1 &&
    /would be promoted\s+w2-connector-rerouting-smoke layer B/.test(mixed.stdout) &&
    /refused\s+w3-dirty-smoke layer B/.test(mixed.stdout) &&
    /uncommitted changes/.test(mixed.stdout) &&
    /skipped\s+w2-connector-rerouting-smoke layer A/.test(mixed.stdout),
  'a clean workload would be promoted, a dirty one is refused with the reason, a layer with no runs is skipped, and the exit is a failure'
)

const only = cli(
  '--from-dir',
  work,
  '--reason',
  'x',
  '--workloads',
  'w2-connector-rerouting-smoke',
  '--layers',
  'B',
  '--dry-run'
)
check(
  only.status === 0 && /would be promoted/.test(only.stdout) && !/w3-dirty/.test(only.stdout),
  'asking for one workload and one layer promotes only that one, and succeeds'
)

const none = cli('--from-dir', work, '--reason', 'x', '--workloads', 'w9-nothing', '--dry-run')
check(
  none.status === 1 && /no performance results of w9-nothing/.test(none.stderr),
  'a workload that the artifact does not hold is an error that names what it has'
)

const noReason = cli('--from-dir', work, '--dry-run')
check(noReason.status === 1 && /--reason is required/.test(noReason.stderr), 'a baseline without a reason is refused')

const badLayer = cli('--from-dir', work, '--reason', 'x', '--layers', 'D', '--dry-run')
check(
  badLayer.status === 1 && /--layers is a list of A, B and C/.test(badLayer.stderr),
  'a layer that does not exist is refused'
)

const noRun = cli('--reason', 'x')
check(noRun.status === 1 && /usage/.test(noRun.stderr), 'with neither a run id nor a folder it says how to be used')

const after = fs.existsSync(path.join(root, 'perf/baselines'))
  ? fs.readdirSync(path.join(root, 'perf/baselines')).sort().join()
  : ''
check(before === after, 'and nothing was written under perf/baselines: every promotion was a dry run')
fs.rmSync(work, { recursive: true, force: true })

console.log(failures ? `\n${failures} failed` : '\nthe promotion from a run fetches, sorts and refuses as it should')
process.exit(failures ? 1 : 0)
