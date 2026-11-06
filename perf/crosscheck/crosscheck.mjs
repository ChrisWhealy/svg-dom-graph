// Runs the Rust generator and the independent JavaScript one over a set of manifests, and checks that they agree.
//
//   node perf/crosscheck/crosscheck.mjs
//
// The manifests cover each workload at several sizes and seeds, with and without a degree limit, with and without a
// warm-up, and with different checkpoint intervals. It needs a Rust toolchain, since it builds `perf-harness` first.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { compare, readReport } from './reference-workload.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const built = spawnSync('cargo', ['build', '-q', '-p', 'perf-harness'], { cwd: root, stdio: 'inherit' })
if (built.status !== 0) process.exit(1)
const binary = path.join(root, 'target/debug/perf-harness')

const manifest = ({ workload, seed, nodes, edges, degree, warmup, iterations, every }) => `
schema_version = 1
workload = "${workload}"
seed = ${seed}
prng = "splitmix64"
[graph]
nodes = ${nodes}
edges = ${edges}
${degree === null ? '' : `maximum_degree = ${degree}`}
[execution]
modes = ${every === null ? '["performance"]' : '["correctness", "performance"]'}
warmup_iterations = ${warmup}
measurement_iterations = ${iterations}
repetitions = 1
[measurement]
capture_dom_counts = true
capture_listener_tally = true
capture_heap = false
capture_wasm_capacity = true
capture_wasm_live_allocation = true
capture_frame_timing = false
${every === null ? '' : `checkpoint_every = ${every}`}
[checkpoint]
view = "unchanged"
${workload === 'recovery' ? '[recovery]\npoints = [{ point = "node_construction", expect = "nothing_committed" }, { point = "removed_host", expect = "undocumented" }]' : ''}
${workload === 'interaction' ? '[interaction]\nscales_percent = [50, 100, 200]\ngestures = ["drag", "pan", "wheel_zoom"]\nsteps = 4\nstep_pixels = 5' : ''}
[limits]
wall_ms = 1000
`

const cases = []
for (const workload of ['scene_construction', 'connector_rerouting', 'scene_lifecycle', 'interaction', 'recovery']) {
  // The largest seed that TOML can write is the largest signed 64-bit integer. BigInt keeps the larger ones exact.
  for (const seed of [0n, 1n, 2026n, 9007199254740991n, 9007199254740993n, 9223372036854775807n]) {
    cases.push({ workload, seed, nodes: 12, edges: 20, degree: 8, warmup: 2, iterations: 6, every: 3 })
  }
  // A scene with no connector has none to break, which the recovery workload refuses.
  if (workload !== 'recovery')
    cases.push({ workload, seed: 5n, nodes: 1, edges: 0, degree: null, warmup: 0, iterations: 1, every: null })
  cases.push({ workload, seed: 5n, nodes: 2, edges: 2, degree: 2, warmup: 0, iterations: 3, every: 1 })
  cases.push({ workload, seed: 77n, nodes: 50, edges: 120, degree: null, warmup: 5, iterations: 20, every: 7 })
  cases.push({ workload, seed: 12345n, nodes: 200, edges: 600, degree: 16, warmup: 3, iterations: 9, every: null })
}

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-crosscheck-'))
let failures = 0
for (const [index, c] of cases.entries()) {
  const file = path.join(dir, `case-${index}.toml`)
  fs.writeFileSync(file, manifest(c))
  const run = spawnSync(binary, ['generate', file], { encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 })
  const label = `${c.workload} seed ${c.seed} ${c.nodes}n/${c.edges}e`
  if (run.status !== 0) {
    failures++
    console.log(`FAIL ${label}: the Rust side refused it: ${run.stderr.trim()}`)
    continue
  }
  const { mine, differences } = compare(readReport(run.stdout))
  if (differences.length) failures++
  console.log(
    `${differences.length ? 'FAIL' : 'ok  '} ${label}: ${mine.operations} operations${differences.length ? `, differs in ${differences.join(', ')}` : ''}`
  )
}
fs.rmSync(dir, { recursive: true, force: true })
console.log(failures ? `\n${failures} of ${cases.length} differ` : `\nall ${cases.length} agree`)
process.exit(failures ? 1 : 0)
