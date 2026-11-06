// Says where the bytes that stay allocated after a teardown come from.
//
//   node perf/runner/attribute-persistent.mjs [--cycles N] [--out FILE]
//
// Each stage of the lifecycle is run in a page of its own, so that its first use is measured on its own: nothing, the host
// element, a scene, a scene with 12 nodes, and a scene with 12 nodes and 24 connectors. What a stage leaves after its first
// cycle, less what the stage before it left, is what that stage's first use costs. What every later cycle adds is the growth,
// and for a lifecycle that does not leak it is zero. The checks are in `perf-harness-wasm/src/persistent`.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from './lib/harness-page.mjs'

const root = path.resolve(import.meta.dirname, '../..')

const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const cycles = Number(argument('--cycles') ?? 30)
const out = argument('--out')

const stages = [
  ['nothing', 'the loop and its recording: the harness'],
  ['host', 'the host <svg> made and removed: the page and wasm-bindgen'],
  ['scene', 'a scene made on the host and dropped: the library, with no content'],
  ['nodes', 'the same with 12 nodes: first use of nodes'],
  ['edges', 'the same with 12 nodes and 24 connectors: first use of connectors']
]

const results = []
const preludes = []
for (const [stage] of stages) {
  const page = await openHarnessPage()
  const result = JSON.parse(
    await page.evaluate(`window.perfHarness.attribute_persistent(${JSON.stringify(stage)}, ${cycles}, 'none', 0)`)
  )
  await page.close()
  if (result.error) throw new Error(`${stage}: ${result.error}`)
  results.push({ stage, ...result })
}

let previous = { bytes_over_baseline: 0, blocks_over_baseline: 0 }
let leaks = 0
console.log(`${cycles} cycles of each stage, each stage in a fresh page\n`)
console.log('stage    left after cycle 1          first use of this stage      growth in cycles 2 to ' + cycles)
for (const [index, result] of results.entries()) {
  const now = result.after_first_cycle
  const added = {
    bytes: now.bytes_over_baseline - previous.bytes_over_baseline,
    blocks: now.blocks_over_baseline - previous.blocks_over_baseline
  }
  const growth = result.growth_after_first_cycle
  if (growth.bytes !== 0 || growth.blocks !== 0) leaks++
  console.log(
    `${result.stage.padEnd(8)} ${String(now.bytes_over_baseline).padStart(6)} bytes ${String(now.blocks_over_baseline).padStart(3)} blocks   ${String(added.bytes).padStart(6)} bytes ${String(added.blocks).padStart(3)} blocks   ${String(growth.bytes).padStart(6)} bytes ${String(growth.blocks).padStart(3)} blocks   ${stages[index][1]}`
  )
  previous = now
}
console.log(
  leaks
    ? `\n${leaks} stage(s) kept growing after the first cycle`
    : '\nno stage grows after its first cycle: the page, the library and wasm-bindgen leave nothing behind'
)

// The executor does some things before it reads its baseline: it reads the clock a great many times, and counts listeners once.
// If either of those leaves a block that the library's first calls then grow, it would be counted as left over, and it would
// not be the library's. The full lifecycle is run again after each, in a fresh page.
console.log('\nThe full lifecycle after what the executor does before its baseline:')
for (const prelude of ['clock', 'tracker', 'both']) {
  const page = await openHarnessPage()
  const result = JSON.parse(
    await page.evaluate(`window.perfHarness.attribute_persistent('edges', ${cycles}, ${JSON.stringify(prelude)}, 0)`)
  )
  await page.close()
  if (result.error) throw new Error(`${prelude}: ${result.error}`)
  const left = result.after_first_cycle
  console.log(
    `  after ${prelude.padEnd(7)}: left after cycle 1 ${left.bytes_over_baseline} bytes ${left.blocks_over_baseline} blocks, growth after that ${result.growth_after_first_cycle.bytes} bytes`
  )
  preludes.push({ prelude, ...result })
}

// What is left after the first cycle grew with the size of the scene in the endurance run, and it grows in steps. A table that
// doubles when it is full, and is never made smaller, would do exactly that: it is as large as the most that was ever alive at
// once. So the same lifecycle is run with scenes of increasing size, each in a fresh page.
console.log('\nWhat is left after the first cycle, by the size of the scene (nodes, and twice as many connectors):')
const sizes = []
for (const nodes of [4, 12, 24, 40, 80, 160, 320]) {
  const page = await openHarnessPage()
  const result = JSON.parse(
    await page.evaluate(`window.perfHarness.attribute_persistent('edges', ${cycles}, 'none', ${nodes})`)
  )
  await page.close()
  if (result.error) throw new Error(`${nodes} nodes: ${result.error}`)
  const left = result.after_first_cycle
  console.log(
    `  ${String(nodes).padStart(4)} nodes: left after cycle 1 ${String(left.bytes_over_baseline).padStart(6)} bytes ${String(left.blocks_over_baseline).padStart(2)} blocks, growth after that ${result.growth_after_first_cycle.bytes} bytes`
  )
  if (result.growth_after_first_cycle.bytes !== 0) leaks++
  sizes.push({ nodes, ...result })
}

// The same question, asked inside a real endurance run: what is still allocated after each teardown, by the phase of the
// harness that allocated it. The allocator follows every block allocated after the run's baseline was read.
const manifestPath = path.join(root, 'perf/manifests/w3-scene-lifecycle-endurance.toml')
const canonical = spawnSync(path.join(root, 'target/release/perf-harness'), ['canonical', manifestPath], {
  encoding: 'utf8'
})
if (canonical.status !== 0) throw new Error(`could not read the manifest: ${canonical.stderr}`)
const manifest = JSON.parse(canonical.stdout)
manifest.execution.warmup_iterations = 1
manifest.execution.measurement_iterations = 5
const page = await openHarnessPage()
const run = JSON.parse(
  await page.evaluate(`window.perfHarness.attribute_persistent_run(${JSON.stringify(JSON.stringify(manifest))}, 4)`)
)
await page.close()
if (run.error) throw new Error(run.error)

console.log(
  '\nInside a real endurance run (W3, 40 nodes, 80 edges), what stays allocated after each teardown, by phase:'
)
for (const entry of run.cycles) {
  console.log(
    `  after cycle ${entry.cycle}: ${entry.live_bytes_over_baseline} bytes and ${entry.live_blocks_over_baseline} blocks over the baseline, ${entry.tracked_blocks} blocks (${entry.tracked_bytes} bytes) followed`
  )
  for (const phase of entry.by_phase)
    console.log(
      `      ${String(phase.blocks).padStart(3)} blocks ${String(phase.bytes).padStart(6)} bytes  ${phase.phase}  sizes ${phase.sizes.slice(0, 12).join(', ')}${phase.sizes.length > 12 ? ', ...' : ''}`
    )
}
for (const growth of run.growths_of_blocks_from_before_tracking)
  console.log(
    `  a block from before the baseline grew from ${growth.from} to ${growth.to} bytes during: ${growth.during}`
  )
console.log(
  `  bytes allocated after the baseline and before tracking began: ${run.bytes_allocated_before_tracking_began}`
)
if (run.blocks_the_tracker_could_not_follow)
  console.log(`  ${run.blocks_the_tracker_could_not_follow} blocks could not be followed, so the lists are incomplete`)
if (out)
  fs.writeFileSync(
    out,
    JSON.stringify({ cycles, stages: results, preludes, sizes, endurance_run: run }, null, 2) + '\n'
  )
process.exit(leaks ? 1 : 0)
