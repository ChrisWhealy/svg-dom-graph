// Runs an endurance manifest for hours, and writes down what it ran and whether anything grew.
//
//   node perf/runner/endurance-soak.mjs <manifest.toml> --hours H [--layers b] [--segment-cycles N] [--segment-minutes M] [--out-dir DIR]
//
// Nothing here predicts how long a run will take. Two attempts to do so, from the time of short runs, were wrong on a hosted runner: the start of a
// run there takes between a minute and two, so the difference between two short runs is mostly noise, and a recovery cycle costs more the longer
// the run is. A third gave W3 a million cycles in one call, which was still running after 5.4 hours without a word, and there was nothing to look at.
// A single long call cannot be watched, and cannot be stopped part-way, so a soak is made of calls that can be.
//
//  - A recovery workload (W5) is driven from the page one cycle at a time, so it is one run that is given the hours, and ends at the first cycle that
//    starts after them. `cycles_completed` is how many fitted, and every gate judges those.
//  - Any other workload (W3) runs its operation stream in wasm in a single call, whose hash is compared with the whole of the intended stream. So a
//    soak is segments of `--segment-cycles` (100,000) cycles, one after another, each a run of its own with the gates of an endurance run, and the
//    next is started only while the hours have not passed. The time of every segment is written as it ends, so a slowdown shows, and a segment
//    that has not ended after `--segment-minutes` (30) is stopped and named. The first segment that fails ends the soak.
//
// Every run has a ceiling, so that one that is stuck does not hold a job until the job's own limit: 1.5 times the hours for a recovery soak,
// and the segment's minutes for a segment. The record says `timed_out`, and a run that timed out has no gates, so it is a failure and not evidence.
//
// It runs what is already built (`cargo build --release -p perf-harness`, and wasm-pack for perf-harness-wasm, as perf-endurance.yml does), and
// writes copies of the manifest into the output directory. The manifest in the repository is not changed. The harness keeps about 230 bytes of wasm
// memory for each cycle, which is why a segment is not made longer.
//
// The gates are the ones that every endurance run has: the elements and listeners return to their baseline after every teardown, the live
// bytes do not grow after the warm-up, and every recovery reaches the state that its contract promises. They are exact, so a soak that
// ends with them passing shows no growth, and one that does not names the cycle. `endurance-record.json` in the output directory is what to keep
// as evidence: put it in `perf/evidence/endurance/`, where `qualify.mjs` looks for it.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '../..')
const argument = (name, fallback = null) => {
  const at = process.argv.indexOf(name)
  return at < 0 ? fallback : process.argv[at + 1]
}
const manifestPath = path.resolve(process.argv[2] ?? '')
const hours = Number(argument('--hours'))
const layers = argument('--layers', 'b')
const outDir = path.resolve(argument('--out-dir', path.join(root, 'target/endurance-soak')))
const segmentCycles = Number(argument('--segment-cycles', 100_000))
const segmentSeconds = Number(argument('--segment-minutes', 30)) * 60
const usage =
  'usage: node perf/runner/endurance-soak.mjs <manifest.toml> --hours H [--layers b] [--segment-cycles N] [--segment-minutes M] [--out-dir DIR]'
if (!fs.existsSync(manifestPath) || !(hours > 0)) throw new Error(usage)
fs.mkdirSync(outDir, { recursive: true })

/** A time-bound soak is given more cycles than the time can hold: no recovery cycle has been seen to take less than this, so the time decides. */
const FASTEST_SECONDS_A_CYCLE = 0.02
/** The most measured cycles that one run is given. The harness keeps about 230 bytes of wasm memory for each, and wasm32 has 4 GiB. */
const MAX_CYCLES = 2_000_000
for (const built of ['target/release/perf-harness', 'perf-harness-wasm/pkg/perf_harness_wasm_bg.wasm'])
  if (!fs.existsSync(path.join(root, built)))
    throw new Error(
      `${built} is not built. Run \`cargo build --release -p perf-harness\` and \`wasm-pack build perf-harness-wasm --target web --release --out-dir pkg\` first`
    )

const text = fs.readFileSync(manifestPath, 'utf8')
const setting = key => Number(text.match(new RegExp(`^${key}\\s*=\\s*(\\d+)`, 'm'))?.[1])
const warmup = setting('warmup_iterations')
const timeBound = /^workload\s*=\s*"recovery"/m.test(text)

/** One run of the runner on a copy of the manifest with `cycles` measured cycles, stopped at `ceilingSeconds`. */
const run = (name, cycles, ceilingSeconds) => {
  const file = path.join(outDir, `${name}-${path.basename(manifestPath)}`)
  fs.writeFileSync(
    file,
    text
      .replace(/^measurement_iterations\s*=.*$/m, `measurement_iterations = ${cycles}`)
      .replace(/^repetitions\s*=.*$/m, 'repetitions = 1')
      .replace(/^wall_ms\s*=.*$/m, `wall_ms = ${Math.ceil(ceilingSeconds * 1000)}`)
  )
  const began = Date.now()
  const ran = spawnSync(
    'node',
    [
      path.join(root, 'perf/runner/run.mjs'),
      file,
      '--mode',
      'endurance',
      '--layers',
      layers,
      '--out-dir',
      path.join(outDir, name),
      '--no-build',
      '--repetitions',
      '1'
    ],
    {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: 1 << 30,
      timeout: Math.ceil(ceilingSeconds * 1000),
      killSignal: 'SIGTERM'
    }
  )
  const output = ran.stdout + ran.stderr
  const gates = [...output.matchAll(/^\s+(?:ok|FAIL)\s+(endurance\.[a-z_]+): (pass|FAIL)[^\n]*/gm)].map(m => ({
    gate: m[1],
    outcome: m[2]
  }))
  return {
    name,
    cycles,
    status: ran.status,
    timedOut: ran.error?.code === 'ETIMEDOUT',
    output,
    gates,
    seconds: (Date.now() - began) / 1000,
    passed: ran.status === 0 && gates.length > 0 && gates.every(g => g.outcome === 'pass')
  }
}

const started = new Date()
const began = Date.now()
const runs = []
let ceilingSeconds
if (timeBound) {
  // A time-bound soak is told its time, in the environment that the runner reads (see `stopAfter` in run.mjs).
  process.env.PERF_ENDURANCE_STOP_AFTER_SECONDS = String(Math.round(hours * 3600))
  ceilingSeconds = hours * 1.5 * 3600
  const cycles = Math.min(MAX_CYCLES, Math.ceil((hours * 3600) / FASTEST_SECONDS_A_CYCLE))
  console.log(
    `${path.basename(manifestPath)} runs for ${hours} hours, and ends at the first cycle after that (of at most ${cycles})`
  )
  runs.push(run('soak', cycles, ceilingSeconds))
} else {
  ceilingSeconds = segmentSeconds
  console.log(
    `${path.basename(manifestPath)} runs segments of ${segmentCycles} cycles for ${hours} hours, each stopped if it takes more than ${segmentSeconds / 60} minutes`
  )
  while ((Date.now() - began) / 1000 < hours * 3600) {
    const segment = run(
      `segment-${String(runs.length + 1).padStart(3, '0')}`,
      Math.min(MAX_CYCLES, segmentCycles),
      segmentSeconds
    )
    runs.push(segment)
    console.log(
      `segment ${runs.length}: ${segment.cycles} cycles in ${segment.seconds.toFixed(0)} s (${((Date.now() - began) / 3600000).toFixed(2)} of ${hours} hours), ${segment.passed ? 'every gate passed' : segment.timedOut ? 'TIMED OUT' : 'FAILED'}`
    )
    if (!segment.passed) break
  }
}
const seconds = (Date.now() - began) / 1000
const failed = runs.find(r => !r.passed)

/** The cycles that a time-bound soak made, which are fewer than it was given. Null if it left no result. */
const completedCycles = () => {
  if (!timeBound) return runs.filter(r => r.passed).reduce((sum, r) => sum + r.cycles, 0)
  try {
    const dir = path.join(outDir, 'soak')
    const file = fs
      .readdirSync(dir)
      .find(name => name.endsWith('.json') && !name.startsWith('workload') && !name.startsWith('allocator'))
    const result = JSON.parse(fs.readFileSync(path.join(dir, file), 'utf8'))
    return result.metrics.find(metric => metric.name === 'recovery.cycles')?.value ?? null
  } catch {
    return null
  }
}
const record = {
  schema_version: 3,
  manifest: path.relative(root, manifestPath),
  time_bound: timeBound,
  hours_requested: hours,
  hours_run: Number((seconds / 3600).toFixed(3)),
  seconds_run: Math.round(seconds),
  warmup_iterations: warmup,
  measurement_iterations: runs[0]?.cycles ?? null,
  cycles_completed: completedCycles(),
  segments: timeBound
    ? null
    : runs.map(r => ({
        run: r.name,
        cycles: r.cycles,
        seconds: Math.round(r.seconds),
        passed: r.passed,
        timed_out: r.timedOut
      })),
  layers,
  started_at: started.toISOString(),
  host: { platform: process.platform, cpu: os.cpus()[0]?.model ?? null, cpus: os.cpus().length },
  passed: runs.length > 0 && !failed && (timeBound || seconds >= hours * 3600),
  timed_out: runs.some(r => r.timedOut),
  failure: failed
    ? `${failed.name}${failed.timedOut ? ` was stopped after ${Math.round(failed.seconds)} s, which is its ceiling of ${Math.round(ceilingSeconds)} s: it did not end` : ' did not pass'}\n` +
      failed.output
        .split('\n')
        .filter(l => l.trim())
        .slice(-12)
        .join('\n')
    : null,
  gates: (failed ?? runs[runs.length - 1])?.gates ?? []
}
fs.writeFileSync(path.join(outDir, 'endurance-record.json'), JSON.stringify(record, null, 2) + '\n')
fs.writeFileSync(path.join(outDir, 'soak.log'), runs.map(r => `=== ${r.name}\n${r.output}`).join('\n'))
console.log((failed ?? runs[runs.length - 1]).output.split('\n').slice(-25).join('\n'))
console.log(
  `\n${record.passed ? 'no growth: every endurance gate passed' : 'the run stopped before it reported its gates, or a gate FAILED'} after ${record.hours_run} hours and ${record.cycles_completed ?? 'an unknown number of'} cycles; see ${path.relative(root, path.join(outDir, 'endurance-record.json'))}`
)
process.exit(record.passed ? 0 : 1)
