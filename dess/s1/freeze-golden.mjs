// Freezes the agreed hashes of the differential panels into `dess/s1/golden.json`.
//
//   node dess/s1/freeze-golden.mjs
//
// Nothing is frozen that the two engines did not agree on. This runs the differential run first, on 600 seeds of each panel, and
// refuses to write the file unless every scenario agrees. The golden file holds, for each seed of each panel, the number of pops and
// slices, the event-pop hash and the checkpoint hash of the end of the run. The Rust tests check all of it, on every platform they
// run on, which makes the 32-bit WASI build agree with the oracle without a second comparison.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { run } from '../reference-js/oracle.mjs'
import { disturbances, panelParams } from '../reference-js/generator.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const SEEDS = 600
const agreed = spawnSync('node', [path.join(root, 'dess/s1/differential.mjs'), '--seeds', String(SEEDS)], {
  cwd: root,
  encoding: 'utf8',
  maxBuffer: 1 << 28
})
if (agreed.status !== 0) {
  console.error('the engines do not agree, so nothing is frozen:\n' + agreed.stdout)
  process.exit(1)
}
console.log(agreed.stdout.trim().split('\n')[0])

const entry = t => [
  t.counters.pops,
  t.counters.slices,
  t.event_pop_sha256,
  (t.slices.at(-1) ?? { checkpoint: t.initial_checkpoint }).checkpoint
]
const panel = []
const panel2 = []
for (let seed = 0; seed < SEEDS; seed++) {
  const p = panelParams(seed)
  panel.push(entry(run({ configId: `panel/1:${seed}`, seed, config: p, params: p })))
  const q = { ...p, revision: 'panel/2' }
  panel2.push(entry(run({ configId: `panel/2:${seed}`, seed, config: q, params: q, script: disturbances(seed, q) })))
}
const golden = {
  note: 'Hashes on which the Rust engine and the oracle agree. Each entry is [pops, slices, event-pop hash, final checkpoint hash]. Written by freeze-golden.mjs, and never by hand.',
  panel,
  panel2
}
fs.writeFileSync(path.join(root, 'dess/s1/golden.json'), JSON.stringify(golden) + '\n')
console.log(`frozen ${panel.length + panel2.length} scenarios into dess/s1/golden.json`)
