// Runs an endurance manifest for hours, and writes down what it ran and whether anything grew.
//
//   node perf/runner/endurance-soak.mjs <manifest.toml> --hours H [--layers b] [--out-dir DIR]
//
// The manifests are 200 cycles, which is minutes. A long soak is the same manifest with more cycles, as the manifests say, so this works out how
// many cycles fill the hours: it times the manifest at two lengths, takes the time of one cycle from the difference (which leaves out the start of
// the browser), and writes a copy of the manifest with that many measured cycles and a wall limit that allows for them. The copy goes to the
// output directory, and the manifest in the repository is not changed.
//
// It runs what is already built (`cargo build --release -p perf-harness`, and wasm-pack for perf-harness-wasm, as perf-endurance.yml does), so
// that the time of a cycle is not mixed with the time of a build. A run that cannot be timed, or that would need more cycles than the harness can
// hold, is said to be so: the cycles are capped at `MAX_CYCLES`, and the record says that the cap and not the hours decided the length.
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
if (!fs.existsSync(manifestPath) || !(hours > 0))
  throw new Error('usage: node perf/runner/endurance-soak.mjs <manifest.toml> --hours H [--layers b] [--out-dir DIR]')
fs.mkdirSync(outDir, { recursive: true })

/**
 * The most measured cycles that one run is given. The harness keeps about 230 bytes of its wasm memory for each cycle (22.9 MB at 100,000 cycles of
 * the W3 manifest), and wasm32 has 4 GiB. 36 million cycles, which is what a floor on a time of a cycle once asked for, stopped the run with
 * `RuntimeError: unreachable` after half an hour. Two million is under a tenth of the memory.
 */
const MAX_CYCLES = 2_000_000
/** How much longer the long calibration run has to be than the short one for the difference to be believed, which is well over the noise of a start. */
const CLEAR_SECONDS = 60
/** The most cycles that calibration will try. */
const MAX_CALIBRATION_CYCLES = 1_000_000
/** The least time of a cycle that is believed. A calibration that gives less is noise, and is refused and not used. */
const LEAST_SECONDS_A_CYCLE = 2e-5
for (const built of ['target/release/perf-harness', 'perf-harness-wasm/pkg/perf_harness_wasm_bg.wasm'])
  if (!fs.existsSync(path.join(root, built)))
    throw new Error(
      `${built} is not built. Run \`cargo build --release -p perf-harness\` and \`wasm-pack build perf-harness-wasm --target web --release --out-dir pkg\` first`
    )

const orchestrate = (manifest, directory, extra = []) => {
  const started = Date.now()
  const ran = spawnSync(
    'node',
    [
      path.join(root, 'perf/runner/run.mjs'),
      manifest,
      '--mode',
      'endurance',
      '--layers',
      layers,
      '--out-dir',
      directory,
      ...extra
    ],
    {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: 1 << 30
    }
  )
  return { status: ran.status, output: ran.stdout + ran.stderr, seconds: (Date.now() - started) / 1000 }
}
const setting = (text, key) => Number(text.match(new RegExp(`^${key}\\s*=\\s*(\\d+)`, 'm'))?.[1])

const text = fs.readFileSync(manifestPath, 'utf8')
const warmup = setting(text, 'warmup_iterations')
const withCycles = (source, measured, wallMs) =>
  source
    .replace(/^measurement_iterations\s*=.*$/m, `measurement_iterations = ${measured}`)
    .replace(/^repetitions\s*=.*$/m, 'repetitions = 1')
    .replace(/^wall_ms\s*=.*$/m, `wall_ms = ${wallMs}`)
// The start of the browser is most of a short run, so the time of a cycle is the difference between two runs of different lengths.
const timed = (name, measured) => {
  const file = path.join(outDir, name)
  fs.writeFileSync(file, withCycles(text, measured, 3_600_000))
  const result = orchestrate(file, path.join(outDir, name.replace('.toml', '')), ['--no-build', '--repetitions', '1'])
  if (result.status !== 0) {
    console.error(result.output.split('\n').slice(-30).join('\n'))
    throw new Error(`${name} does not pass, so it is not soaked`)
  }
  return { cycles: warmup + measured, seconds: result.seconds }
}
// The extra cycles of the long run have to be more than the noise of starting a browser, which on a shared runner is seconds, and has been over
// a hundred seconds for the start itself. So the long run is made four times longer until it is longer than the short one by `CLEAR_SECONDS`.
const short = timed('calibration-short.toml', setting(text, 'measurement_iterations'))
let long = timed('calibration-long.toml', Math.max(setting(text, 'measurement_iterations') * 20, 20_000))

while (long.seconds - short.seconds < CLEAR_SECONDS && long.cycles * 4 <= MAX_CALIBRATION_CYCLES) {
  console.log(
    `${long.cycles} cycles took ${(long.seconds - short.seconds).toFixed(1)} s more than ${short.cycles}: too little to tell from noise, so it is tried longer`
  )
  long = timed('calibration-long.toml', long.cycles * 4 - warmup)
}

const perCycle = (long.seconds - short.seconds) / (long.cycles - short.cycles)
console.log(
  `${perCycle.toFixed(5)} s a cycle (${short.seconds.toFixed(1)} s for ${short.cycles}, ${long.seconds.toFixed(1)} s for ${long.cycles})`
)
if (!(perCycle >= LEAST_SECONDS_A_CYCLE))
  throw new Error(
    `the time of a cycle could not be worked out (${perCycle} s): the longer run was not longer than the shorter by more than the noise of starting a browser`
  )
const wanted = Math.max(setting(text, 'measurement_iterations'), Math.floor((hours * 3600) / perCycle) - warmup)
const measured = Math.min(wanted, MAX_CYCLES)
const capped = measured < wanted
const wallMs = Math.ceil(hours * 3600 * 1000 * 1.5)
const soak = path.join(outDir, `soak-${path.basename(manifestPath)}`)
fs.writeFileSync(soak, withCycles(text, measured, wallMs))
console.log(
  `${measured} measured cycles for ${hours} hours${capped ? `, and ${wanted} were wanted: the run is capped at ${MAX_CYCLES} cycles by the memory that the harness keeps for each, so it will end sooner than the hours that were asked for` : ''}`
)

const started = new Date()
const soaked = orchestrate(soak, path.join(outDir, 'soak'), ['--no-build', '--repetitions', '1'])
const gates = [...soaked.output.matchAll(/^\s+(?:ok|FAIL)\s+(endurance\.[a-z_]+): (pass|FAIL)[^\n]*/gm)].map(m => ({
  gate: m[1],
  outcome: m[2]
}))
const record = {
  schema_version: 1,
  manifest: path.relative(root, manifestPath),
  hours_requested: hours,
  hours_run: Number((soaked.seconds / 3600).toFixed(3)),
  seconds_run: Math.round(soaked.seconds),
  warmup_iterations: warmup,
  measurement_iterations: measured,
  cycles_wanted: wanted,
  capped_by_memory: capped,
  seconds_a_cycle: Number(perCycle.toFixed(6)),
  layers,
  started_at: started.toISOString(),
  host: { platform: process.platform, cpu: os.cpus()[0]?.model ?? null, cpus: os.cpus().length },
  exit_status: soaked.status,
  passed: soaked.status === 0 && gates.length > 0 && gates.every(g => g.outcome === 'pass'),
  failure:
    soaked.status === 0
      ? null
      : soaked.output
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
  `\n${record.passed ? 'no growth: every endurance gate passed' : 'the run stopped before it reported its gates, or a gate FAILED'} after ${record.hours_run} hours; see ${path.relative(root, path.join(outDir, 'endurance-record.json'))}`
)
process.exit(record.passed ? 0 : 1)
