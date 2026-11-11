// The cost of the simulator panel in the demo's wasm, measured and not assumed (design section 14).
//
//   node dess/demo-panel/p4/size-ledger.mjs [--before <git ref>] [--out dess/demo-panel/p4/evidence/size-ledger]
//
// The demo is built twice with the same command, the same toolchain and the same wasm-opt setting, which is whatever `cargo run -p demo-server --
// --build-only` does: once at the commit before the panel was added, in a worktree of its own, and once at the tree as it is now. For each, the byte
// count of the wasm and its size when gzipped at the best level are recorded, with the delta in bytes and in per cent, and no threshold is imposed. The
// dependency tree of the demo for the wasm target is recorded for both, both resolved with the same Cargo.lock, and what the panel added to it is listed, including whether `sha2` came in once
// or twice. The record is a JSON file and a table, and the first build of each tree takes several minutes.
import fs from 'node:fs'
import path from 'node:path'
import zlib from 'node:zlib'
import { spawnSync } from 'node:child_process'
import { sourceDirty } from '../../lib/git-state.mjs'

const root = path.resolve(import.meta.dirname, '../../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const out = path.resolve(argument('--out') ?? path.join(root, 'dess/demo-panel/p4/evidence/size-ledger'))
const run = (command, args, options = {}) =>
  spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28, ...options })
const git = (...args) => run('git', args).stdout.trim()

// The commit before the first one that has the panel's module.
const added = git('log', '--diff-filter=A', '--format=%H', '--', 'demo-app/src/service_sim/mod.rs')
  .split('\n')
  .filter(Boolean)
  .at(-1)
const beforeRef = argument('--before') ?? `${added}^`
const beforeSha = git('rev-parse', beforeRef)
const afterSha = git('rev-parse', 'HEAD')
const dirty = sourceDirty(root)

const tree = dir => {
  const result = run(
    'cargo',
    ['tree', '-p', 'svg-dom-graph-demo', '--target', 'wasm32-unknown-unknown', '--color', 'never', '--charset', 'utf8'],
    { cwd: dir }
  )
  if (result.status !== 0) throw new Error(`cargo tree failed in ${dir}: ${result.stderr}`)
  return result.stdout
}
/** The distinct crates of a `cargo tree`, as `name version`, with each one that is in it more than once counted. */
const crates = text => {
  const found = new Map()
  for (const line of text.split('\n')) {
    const m = line.match(/([A-Za-z0-9_-]+) v(\d[^\s]*)/)
    if (m) found.set(`${m[1]} ${m[2]}`, true)
  }
  return [...found.keys()].sort()
}
const build = dir => {
  const built = run('cargo', ['run', '-p', 'demo-server', '--', '--build-only'], {
    cwd: dir,
    stdio: ['ignore', 'inherit', 'inherit']
  })
  if (built.status !== 0) throw new Error(`the demo did not build in ${dir}`)
  const wasm = fs.readdirSync(path.join(dir, 'target/demo-stage/pkg')).find(f => f.endsWith('_bg.wasm'))
  const bytes = fs.readFileSync(path.join(dir, 'target/demo-stage/pkg', wasm))
  return { file: wasm, bytes: bytes.length, gzip: zlib.gzipSync(bytes, { level: 9 }).length }
}

const worktree = path.join(root, 'target/size-ledger-before')
fs.rmSync(worktree, { recursive: true, force: true })
run('git', ['worktree', 'prune'])
const added_ = run('git', ['worktree', 'add', '--detach', worktree, beforeSha])
if (added_.status !== 0) throw new Error(`could not make a worktree at ${beforeSha}: ${added_.stderr}`)
// Cargo.lock is not in the repository, so a worktree would resolve every dependency again and might take newer versions, and the difference
// between the two builds would be the dependencies' and not the panel's. The lock of the tree as it is now is put in the worktree, so that both builds
// use the same version of every crate that they share.
fs.copyFileSync(path.join(root, 'Cargo.lock'), path.join(worktree, 'Cargo.lock'))
let before, after, beforeTree, afterTree
try {
  console.log(`building the demo at ${beforeSha.slice(0, 10)}, before the panel`)
  before = build(worktree)
  beforeTree = tree(worktree)
  console.log(
    `building the demo at ${afterSha.slice(0, 10)}${dirty ? ' and the changes that are not committed' : ''}, with the panel`
  )
  after = build(root)
  afterTree = tree(root)
} finally {
  run('git', ['worktree', 'remove', '--force', worktree])
}

const percent = (a, b) => Math.round(((b - a) / a) * 10000) / 100
const was = new Set(crates(beforeTree))
const now = crates(afterTree)
const added2 = now.filter(c => !was.has(c))
const sha2 = now.filter(c => c.startsWith('sha2 '))
const ledger = {
  measured_at: new Date().toISOString(),
  toolchain: run('rustc', ['--version']).stdout.trim(),
  command: 'cargo run -p demo-server -- --build-only',
  before: { commit: beforeSha, ...before },
  after: { commit: afterSha, dirty, ...after },
  delta: {
    bytes: after.bytes - before.bytes,
    bytes_percent: percent(before.bytes, after.bytes),
    gzip_bytes: after.gzip - before.gzip,
    gzip_percent: percent(before.gzip, after.gzip)
  },
  crates_added: added2,
  sha2_versions_in_the_demo: sha2,
  note: 'No threshold is imposed. These are measurements of one build of each tree, by the same command.'
}
fs.mkdirSync(path.dirname(out), { recursive: true })
fs.writeFileSync(`${out}.json`, `${JSON.stringify(ledger, null, 2)}\n`)
fs.writeFileSync(`${out}-tree-before.txt`, beforeTree)
fs.writeFileSync(`${out}-tree-after.txt`, afterTree)
const kib = n => `${(n / 1024).toFixed(1)} KiB`
console.log(
  `\n| | wasm | gzip |\n|---|---|---|\n| before | ${before.bytes} (${kib(before.bytes)}) | ${before.gzip} (${kib(before.gzip)}) |\n| after | ${after.bytes} (${kib(after.bytes)}) | ${after.gzip} (${kib(after.gzip)}) |\n| delta | ${ledger.delta.bytes} (${ledger.delta.bytes_percent}%) | ${ledger.delta.gzip_bytes} (${ledger.delta.gzip_percent}%) |`
)
console.log(`\ncrates added to the demo's wasm dependency tree: ${added2.join(', ') || 'none'}`)
console.log(`sha2 in the demo: ${sha2.join(', ') || 'not present'}`)
console.log(`\nrecorded in ${path.relative(root, out)}.json`)
