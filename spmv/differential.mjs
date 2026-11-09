// The differential run: the Rust core and the oracle, scenario by scenario, on every field of the outcome and every hash.
//
//   node spmv/differential.mjs [--seeds 600] [--wasi] [--rust PATH]
//
// The scenarios are the cases of `spmv/cases.json` (hand-derived outcomes, refusals and malformed specifications) and the panel: the
// seeds 0 to N - 1, each deriving its own specification, so that the two also agree on the derivation. `--wasi` runs the Rust core as
// a 32-bit wasm32-wasip1 binary under Node's WASI. The first difference of a scenario is printed with its place, and the run exits
// with 1 if there is any. A difference is a discrepancy, to be adjudicated against `spmv/docs/contract.md` before either side changes.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { outcome, panelSpec } from './reference-js/oracle.mjs'

const root = path.resolve(import.meta.dirname, '..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const SEEDS = Number(argument('--seeds') ?? 600)
const WASI = process.argv.includes('--wasi')
const rust =
  argument('--rust') ??
  path.join(root, WASI ? 'target/wasm32-wasip1/release/spmv-trace.wasm' : 'target/release/spmv-trace')
const casesFile = path.join(root, 'spmv/cases.json')

const rustRun = (args, input) => {
  const command = WASI ? ['node', [path.join(root, 'perf/crosscheck/run-wasi.mjs'), rust, ...args]] : [rust, args]
  const result = spawnSync(command[0], command[1], { cwd: root, encoding: 'utf8', maxBuffer: 1 << 30, input })
  if (result.status !== 0)
    throw new Error(`the Rust core failed (${args.join(' ')}): ${result.stderr || result.stdout}`)
  return result.stdout
}
const rustLines = (args, input) =>
  rustRun(args, input)
    .split('\n')
    .filter(Boolean)
    .map(line => JSON.parse(line))

// JSON text with every object's keys sorted, so that two outcomes are the same whatever order a side wrote an object's keys in.
const canonical = v =>
  Array.isArray(v)
    ? `[${v.map(canonical).join(',')}]`
    : v !== null && typeof v === 'object'
      ? `{${Object.keys(v)
          .sort()
          .map(k => `${JSON.stringify(k)}:${canonical(v[k])}`)
          .join(',')}}`
      : JSON.stringify(v)

/** The first place where two values differ, as a path, or null. */
const firstDifference = (a, b, at = '') => {
  if (canonical(a) === canonical(b)) return null
  if (a && b && typeof a === 'object' && typeof b === 'object' && Array.isArray(a) === Array.isArray(b)) {
    for (const key of [...new Set([...Object.keys(a), ...Object.keys(b)])].sort()) {
      const d = firstDifference(a[key], b[key], `${at}/${key}`)
      if (d) return d
    }
  }
  return `${at || '/'}: rust ${JSON.stringify(a)}, oracle ${JSON.stringify(b)}`
}

let scenarios = 0
let steps = 0
let refused = 0
const differences = []
const compare = (name, fromRust, fromOracle) => {
  scenarios++
  steps += fromOracle.steps?.length ?? 0
  if (fromOracle.outcome === 'refused') refused++
  const d = firstDifference(fromRust, fromOracle)
  if (d) differences.push(`${name}: ${d}`)
}

// The cases.
{
  const text = fs.readFileSync(casesFile, 'utf8')
  const cases = JSON.parse(text)
  for (const row of rustLines(['cases', '-'], text)) {
    const { name, ...fromRust } = row
    const c = cases[name]
    compare(`case ${name}`, fromRust, outcome(c.spec, c.budget))
  }
}

// The panel, in chunks of 100 seeds. The oracle uses the Rust core's default budget, so that both refuse alike.
{
  const budget = JSON.parse(rustRun(['budget']))
  for (let from = 0; from < SEEDS; from += 100) {
    const to = Math.min(from + 100, SEEDS)
    for (const row of rustLines(['panel', String(from), String(to)])) {
      const { name, ...fromRust } = row
      const seed = Number(name.split('/')[1])
      compare(`panel seed ${seed}`, fromRust, outcome(panelSpec(seed), budget))
    }
  }
}

console.log(
  `${WASI ? 'wasm32-wasip1' : 'native'} Rust against the oracle: ${scenarios} scenarios, ${steps} steps compared, ${refused} refusals`
)
if (differences.length > 0) {
  for (const d of differences.slice(0, 10)) console.log(`DIFFERENT ${d}`)
  console.log(`${differences.length} scenarios differ`)
  process.exit(1)
}
console.log('every scenario agrees on every field and every hash')
