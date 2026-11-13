// Runs an endurance manifest for a long time, and writes down what it ran and whether anything grew.
//
//   node perf/runner/endurance-soak.mjs <manifest.toml> --hours H      for a recovery workload (W5), which is given a time
//   node perf/runner/endurance-soak.mjs <manifest.toml> --cycles N     for any other workload (W3), which is given a count of cycles
//                                       [--layers b] [--max-hours H] [--out-dir DIR]
//
// Nothing here predicts how long a run will take. Two attempts to do so, from the time of short runs, were wrong on a hosted runner: the start of a
// run there takes between a minute and two, so the difference between two short runs is mostly noise, and a recovery cycle costs more the longer
// the run is. So each workload is given what it can honour exactly, and what it cannot be given is written down afterwards.
//
//  - A recovery workload is driven from the page, one cycle at a time, and ends at the first cycle that starts after the time it was given. The
//    hours are an input, and the cycles that fitted are a result (`cycles_completed`). Every gate judges the cycles that were made.
//  - Any other workload runs its operation stream in wasm in a single call, and the stream's hash is compared with the whole of the intended one,
//    so it cannot be stopped part-way. The cycles are the input, and the time that they took is the result (`hours_run`).
//
// Both are stopped at a ceiling, so that a run that is stuck does not hold a job until the job's own limit: 1.5 times the hours for a recovery
// soak, and `--max-hours` (5.4, which is under a hosted job's six) for any other. The record says `timed_out`, and a record that timed out has
// no gates, so it is a failure and not evidence.
//
// It runs what is already built (`cargo build --release -p perf-harness`, and wasm-pack for perf-harness-wasm, as perf-endurance.yml does), and
// writes a copy of the manifest with the cycles and a wall limit into the output directory. The manifest in the repository is not changed. The
// harness keeps about 230 bytes of wasm memory for each cycle, so a run is capped at `MAX_CYCLES`.
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
const layers = argument('--layers', 'b')
const outDir = path.resolve(argument('--out-dir', path.join(root, 'target/endurance-soak')))
const maxHours = Number(argument('--max-hours', 5.4))
const usage =
  'usage: node perf/runner/endurance-soak.mjs <manifest.toml> (--hours H for a recovery workload | --cycles N for any other) [--layers b] [--max-hours H] [--out-dir DIR]'
if (!fs.existsSync(manifestPath)) throw new Error(usage)
fs.mkdirSync(outDir, { recursive: true })

/**
 * The most measured cycles that one run is given. The harness keeps about 230 bytes of its wasm memory for each cycle (22.9 MB at 100,000 cycles of
 * the W3 manifest), and wasm32 has 4 GiB. 36 million cycles, which a floor on a time of a cycle once asked for, stopped the run with
 * `RuntimeError: unreachable` after half an hour. Two million is under a tenth of the memory.
 */
const MAX_CYCLES = 2_000_000
/** A time-bound soak is given more cycles than the time can hold: no recovery cycle has been seen to take less than this, so the time decides. */
const FASTEST_SECONDS_A_CYCLE = 0.02
for (const built of ['target/release/perf-harness', 'perf-harness-wasm/pkg/perf_harness_wasm_bg.wasm'])
  if (!fs.existsSync(path.join(root, built)))
    throw new Error(
      `${built} is not built. Run \`cargo build --release -p perf-harness\` and \`wasm-pack build perf-harness-wasm --target web --release --out-dir pkg\` first`
    )

const text = fs.readFileSync(manifestPath, 'utf8')
const setting = key => Number(text.match(new RegExp(`^${key}\\s*=\\s*(\\d+)`, 'm'))?.[1])
const warmup = setting('warmup_iterations')
const timeBound = /^workload\s*=\s*"recovery"/m.test(text)

const hours = Number(argument('--hours'))
const wantedCycles = Number(argument('--cycles'))
if (timeBound ? !(hours > 0) : !(wantedCycles > 0))
  throw new Error(
    `${path.basename(manifestPath)} is ${timeBound ? 'given a time, with --hours' : 'given a count of cycles, with --cycles'}.\n${usage}`
  )
if (!timeBound && Number.isFinite(hours) && hours > 0)
  throw new Error(
    `${path.basename(manifestPath)} cannot be stopped at a time: give it --cycles, and read the time that they took from the record`
  )

const measured = timeBound
  ? Math.min(MAX_CYCLES, Math.ceil((hours * 3600) / FASTEST_SECONDS_A_CYCLE))
  : Math.min(MAX_CYCLES, wantedCycles)
const capped = !timeBound && measured < wantedCycles
const ceilingSeconds = (timeBound ? hours * 1.5 : maxHours) * 3600
const soak = path.join(outDir, `soak-${path.basename(manifestPath)}`)
fs.writeFileSync(
  soak,
  text
    .replace(/^measurement_iterations\s*=.*$/m, `measurement_iterations = ${measured}`)
    .replace(/^repetitions\s*=.*$/m, 'repetitions = 1')
    .replace(/^wall_ms\s*=.*$/m, `wall_ms = ${Math.ceil(ceilingSeconds * 1000)}`)
)
console.log(
  timeBound
    ? `${path.basename(manifestPath)} runs for ${hours} hours, and ends at the first cycle after that (of at most ${measured})`
    : `${path.basename(manifestPath)} runs ${measured} measured cycles${capped ? `, and ${wantedCycles} were asked for: the run is capped at ${MAX_CYCLES} cycles by the memory that the harness keeps for each` : ''}, and takes as long as they take`
)

// A time-bound soak is told its time, in the environment that the runner reads (see `stopAfter` in run.mjs).
if (timeBound) process.env.PERF_ENDURANCE_STOP_AFTER_SECONDS = String(Math.round(hours * 3600))
const started = new Date()
const began = Date.now()
const ran = spawnSync(
  'node',
  [
    path.join(root, 'perf/runner/run.mjs'),
    soak,
    '--mode',
    'endurance',
    '--layers',
    layers,
    '--out-dir',
    path.join(outDir, 'soak'),
    '--no-build',
    '--repetitions',
    '1'
  ],
  { cwd: root, encoding: 'utf8', maxBuffer: 1 << 30, timeout: Math.ceil(ceilingSeconds * 1000), killSignal: 'SIGTERM' }
)
const soaked = {
  status: ran.status,
  timedOut: ran.error?.code === 'ETIMEDOUT',
  output: ran.stdout + ran.stderr,
  seconds: (Date.now() - began) / 1000
}

const gates = [...soaked.output.matchAll(/^\s+(?:ok|FAIL)\s+(endurance\.[a-z_]+): (pass|FAIL)[^\n]*/gm)].map(m => ({
  gate: m[1],
  outcome: m[2]
}))
/** The cycles that a time-bound soak made, which are fewer than it was given. Null if it left no result. A count-bound soak made all that it was given. */
const completedCycles = () => {
  if (!timeBound) return soaked.status === 0 ? measured : null
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
  schema_version: 2,
  manifest: path.relative(root, manifestPath),
  time_bound: timeBound,
  hours_requested: timeBound ? hours : null,
  cycles_requested: timeBound ? null : measured,
  hours_run: Number((soaked.seconds / 3600).toFixed(3)),
  seconds_run: Math.round(soaked.seconds),
  warmup_iterations: warmup,
  measurement_iterations: measured,
  cycles_completed: completedCycles(),
  capped_by_memory: capped,
  layers,
  started_at: started.toISOString(),
  host: { platform: process.platform, cpu: os.cpus()[0]?.model ?? null, cpus: os.cpus().length },
  exit_status: soaked.status,
  passed: soaked.status === 0 && gates.length > 0 && gates.every(g => g.outcome === 'pass'),
  timed_out: soaked.timedOut,
  failure:
    soaked.status === 0
      ? null
      : (soaked.timedOut
          ? `stopped after ${Math.round(soaked.seconds)} s, which is the ceiling of ${(ceilingSeconds / 3600).toFixed(2)} hours: the run did not end\n`
          : '') +
        soaked.output
          .split('\n')
          .filter(l => l.trim())
          .slice(-12)
          .join('\n'),
  gates
}
fs.writeFileSync(path.join(outDir, 'endurance-record.json'), JSON.stringify(record, null, 2) + '\n')
fs.writeFileSync(path.join(outDir, 'soak.log'), soaked.output)
console.log(soaked.output.split('\n').slice(-25).join('\n'))
console.log(
  `\n${record.passed ? 'no growth: every endurance gate passed' : 'the run stopped before it reported its gates, or a gate FAILED'} after ${record.hours_run} hours${record.cycles_completed === null ? '' : ` and ${record.cycles_completed} cycles`}; see ${path.relative(root, path.join(outDir, 'endurance-record.json'))}`
)
process.exit(record.passed ? 0 : 1)
