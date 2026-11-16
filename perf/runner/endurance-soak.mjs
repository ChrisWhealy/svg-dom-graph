// Runs an endurance manifest for hours, and writes down what it ran and whether anything grew.
//
//   node perf/runner/endurance-soak.mjs <manifest.toml> --hours H [--layers b] [--segment-cycles N] [--segment-minutes M] [--out-dir DIR]
//
// Nothing here predicts how long a run will take, and nothing is one long call. Three attempts to size a run from the time of short ones were wrong on a
// hosted runner: the start of a run there takes between a minute and two, so the difference between two short runs is mostly noise. A single call of
// a million cycles was still running after 5.4 hours without a word, and a recovery run that was to stop itself at three hours was still running at
// four and a half. A single long call cannot be watched, so a soak is made of runs that can be.
//
// A soak is segments, one after another, each a run of its own with the gates of an endurance run, and the next is started only while the hours
// have not passed. A segment is `--segment-cycles` cycles: 100,000 for W3, whose cycle takes about 3 ms on a runner, and 3,000 for a recovery workload
// (W5), whose cycle takes 0.1 s or more, so that either is about five minutes. The time of every segment is written as it ends, so a slowdown shows. A segment
// that has not ended after `--segment-minutes` (30) is stopped and named, and the first segment that fails ends the soak. Because every segment
// starts a new page, the cost of a cycle that rises with the length of a run (W5's does, for a reason that has not been found) does not accumulate.
//
// The record says `timed_out` if a segment was stopped, and a run that timed out has no gates, so it is a failure and not evidence. A W3 soak of three
// hours in 34 segments of 283 to 368 s each made 3.4 million cycles, which shows that a segment's time does not grow from one to the next.
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
const segmentMinutes = Number(argument('--segment-minutes', 30))
const usage =
  'usage: node perf/runner/endurance-soak.mjs <manifest.toml> --hours H [--layers b] [--segment-cycles N] [--segment-minutes M] [--out-dir DIR]'
if (!fs.existsSync(manifestPath) || !(hours > 0)) throw new Error(usage)
fs.mkdirSync(outDir, { recursive: true })

for (const built of ['target/release/perf-harness', 'perf-harness-wasm/pkg/perf_harness_wasm_bg.wasm'])
  if (!fs.existsSync(path.join(root, built)))
    throw new Error(
      `${built} is not built. Run \`cargo build --release -p perf-harness\` and \`wasm-pack build perf-harness-wasm --target web --release --out-dir pkg\` first`
    )

const text = fs.readFileSync(manifestPath, 'utf8')
const warmup = Number(text.match(/^warmup_iterations\s*=\s*(\d+)/m)?.[1])
/** A recovery workload holds twelve JavaScript objects for each cycle at once, and a cycle of it takes 0.1 s or more, so its segment is short. */
const recovery = /^workload\s*=\s*"recovery"/m.test(text)
const segmentCycles = Number(argument('--segment-cycles', recovery ? 3_000 : 100_000))
const segmentSeconds = segmentMinutes * 60

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
    timedOut: ran.error?.code === 'ETIMEDOUT',
    output,
    gates,
    seconds: (Date.now() - began) / 1000,
    passed: ran.status === 0 && gates.length > 0 && gates.every(g => g.outcome === 'pass')
  }
}

console.log(
  `${path.basename(manifestPath)} runs segments of ${segmentCycles} cycles for ${hours} hours, each stopped if it takes more than ${segmentMinutes} minutes`
)
const started = new Date()
const began = Date.now()
const runs = []

while ((Date.now() - began) / 1000 < hours * 3600) {
  const segment = run(`segment-${String(runs.length + 1).padStart(3, '0')}`, segmentCycles, segmentSeconds)
  runs.push(segment)
  console.log(
    `segment ${runs.length}: ${segment.cycles} cycles in ${segment.seconds.toFixed(0)} s (${((Date.now() - began) / 3600000).toFixed(2)} of ${hours} hours), ${segment.passed ? 'every gate passed' : segment.timedOut ? 'TIMED OUT' : 'FAILED'}`
  )
  if (!segment.passed) break
  // Only the last segment's output is worth keeping once it passed, and the others would fill the artifact for hours.
  if (runs.length > 1) runs[runs.length - 2].output = ''
}

const seconds = (Date.now() - began) / 1000
const failed = runs.find(r => !r.passed)
const last = runs[runs.length - 1]

const record = {
  schema_version: 3,
  manifest: path.relative(root, manifestPath),
  hours_requested: hours,
  hours_run: Number((seconds / 3600).toFixed(3)),
  seconds_run: Math.round(seconds),
  warmup_iterations: warmup,
  segment_cycles: segmentCycles,
  cycles_completed: runs.filter(r => r.passed).reduce((sum, r) => sum + r.cycles, 0),
  segments: runs.map(r => ({
    run: r.name,
    cycles: r.cycles,
    seconds: Math.round(r.seconds),
    passed: r.passed,
    timed_out: r.timedOut
  })),
  layers,
  started_at: started.toISOString(),
  host: { platform: process.platform, cpu: os.cpus()[0]?.model ?? null, cpus: os.cpus().length },
  passed: runs.length > 0 && !failed && seconds >= hours * 3600,
  timed_out: runs.some(r => r.timedOut),
  failure: failed
    ? `${failed.name}${failed.timedOut ? ` was stopped after ${Math.round(failed.seconds)} s, which is its ceiling of ${segmentSeconds} s: it did not end` : ' did not pass'}\n` +
      failed.output
        .split('\n')
        .filter(l => l.trim())
        .slice(-12)
        .join('\n')
    : null,
  gates: (failed ?? last)?.gates ?? []
}
fs.writeFileSync(path.join(outDir, 'endurance-record.json'), JSON.stringify(record, null, 2) + '\n')
fs.writeFileSync(path.join(outDir, 'soak.log'), (failed ?? last)?.output ?? '')
console.log(((failed ?? last)?.output ?? '').split('\n').slice(-25).join('\n'))
console.log(
  `\n${record.passed ? 'no growth: every endurance gate passed' : 'the run stopped before it reported its gates, or a gate FAILED'} after ${record.hours_run} hours and ${record.cycles_completed} cycles; see ${path.relative(root, path.join(outDir, 'endurance-record.json'))}`
)
process.exit(record.passed ? 0 : 1)
