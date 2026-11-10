// The independent oracle's hash of the simulator's state at every slice of a scenario, for the service target of the recovery workload.
//
// The Rust engine in the page gives its own hash of its state at the slice that a controller committed. This gives the hash that the
// separately coded JavaScript engine (dess/reference-js, written from the design and sharing no code with the Rust one) makes for the same
// slice, so that the page can say whether the controller committed the right slice of the right history. The two engines agree on every
// scenario that the S1 differential run covers, and this checks the one that is used here against the Rust trace as well, before the page
// is asked anything: an oracle that disagrees with the engine is a discrepancy, and is refused and not worked around.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { run } from '../../dess/reference-js/oracle.mjs'
import { MIXED_OUTAGE_7, disturbances, panelParams } from '../../dess/reference-js/generator.mjs'

const root = path.resolve(import.meta.dirname, '../..')

/** The scenario that the manifest names, as the oracle runs it: `E0`, `panel:7`, `panel2:7` or `mixed_outage_7:20261106`. */
const scenarioOf = name => {
  const [kind, seedText] = name.split(':')
  const seed = Number(seedText)
  if (seedText === undefined) {
    const fixtures = JSON.parse(fs.readFileSync(path.join(root, 'dess/s1/fixtures.json'), 'utf8'))
    const f = fixtures[kind]
    if (!f) throw new Error(`there is no scenario called ${name}`)
    return { configId: `fixture/${kind}`, seed: 0, config: f.config, script: f.script }
  }
  if (kind === 'panel') {
    const p = panelParams(seed)
    return { configId: `panel/1:${seed}`, seed, config: p, params: p }
  }
  if (kind === 'panel2') {
    const p = { ...panelParams(seed), revision: 'panel/2' }
    return { configId: `panel/2:${seed}`, seed, config: p, params: p, script: disturbances(seed, p) }
  }
  if (kind === 'mixed_outage_7')
    return { configId: 'mixed_outage/7', seed, config: MIXED_OUTAGE_7, params: MIXED_OUTAGE_7 }
  throw new Error(`there is no scenario called ${name}`)
}

/** The Rust engine's trace of the same scenario, from the binary that the differential run uses. Null if it is not built. */
const rustTrace = name => {
  const binary = path.join(root, 'target/release/dess-trace')
  if (!fs.existsSync(binary)) return null
  const [kind, seed] = name.split(':')
  const fixtures = fs.readFileSync(path.join(root, 'dess/s1/fixtures.json'), 'utf8')
  const args =
    seed === undefined
      ? ['fixtures', '-']
      : kind === 'panel'
        ? ['panel', seed, String(Number(seed) + 1)]
        : kind === 'panel2'
          ? ['panel2', seed, String(Number(seed) + 1)]
          : ['manifest']
  const result = spawnSync(binary, args, {
    cwd: root,
    encoding: 'utf8',
    input: seed === undefined ? fixtures : undefined,
    maxBuffer: 1 << 28
  })
  if (result.status !== 0) return null
  const lines = result.stdout
    .split('\n')
    .filter(Boolean)
    .map(line => JSON.parse(line))
  return seed === undefined ? lines.find(l => l.name === kind)?.trace : lines[0]
}

/**
 * `{ checkpoints: [hash at slice 0, hash at slice 1, ...], slices, agreesWithRust }`. Throws if the two engines disagree on any slice.
 */
export const oracleFor = name => {
  const trace = run(scenarioOf(name))
  const checkpoints = [trace.initial_checkpoint, ...trace.slices.map(s => s.checkpoint)]
  const rust = rustTrace(name)
  let agreesWithRust = null
  if (rust) {
    const theirs = [rust.initial_checkpoint, ...rust.slices.map(s => s.checkpoint)]
    agreesWithRust = theirs.length === checkpoints.length && theirs.every((hash, i) => hash === checkpoints[i])
    if (!agreesWithRust) throw new Error(`the Rust engine and the independent oracle disagree on a slice of ${name}`)
  }
  return { scenario: name, checkpoints, slices: trace.slices.length, agreesWithRust }
}
