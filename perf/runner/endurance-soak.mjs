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
/** The share of the hours that the soak is sized for, which leaves room for what calibration cannot see. The soak is stopped at 1.5 times the hours. */
const SOAK_SHARE = 0.85
/** Calibration goes on until a run takes this fraction of the soak's time (not counting its start), so that the soak is not far beyond what was measured. */
const ENOUGH = 1 / 6
/** The cycles of the first run after the short one: a few seconds for a quick workload, and minutes for a slow one. */
const FIRST_LONG_CYCLES = 2_000
/** The most cycles that one calibration run will try. */
const MAX_CALIBRATION_CYCLES = 1_000_000
/** The calibration runs may take at most this share of the hours between them. */
const CALIBRATION_BUDGET = 0.5
for (const built of ['target/release/perf-harness', 'perf-harness-wasm/pkg/perf_harness_wasm_bg.wasm'])
  if (!fs.existsSync(path.join(root, built)))
    throw new Error(
      `${built} is not built. Run \`cargo build --release -p perf-harness\` and \`wasm-pack build perf-harness-wasm --target web --release --out-dir pkg\` first`
    )

const orchestrate = (manifest, directory, extra = [], timeoutMs = undefined) => {
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
      maxBuffer: 1 << 30,
      timeout: timeoutMs,
      killSignal: 'SIGTERM'
    }
  )
  return {
    status: ran.status,
    timedOut: ran.error?.code === 'ETIMEDOUT',
    output: ran.stdout + ran.stderr,
    seconds: (Date.now() - started) / 1000
  }
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
const timed = (name, measured, timeoutMs = undefined) => {
  const file = path.join(outDir, name)
  fs.writeFileSync(file, withCycles(text, measured, 3_600_000))
  const result = orchestrate(
    file,
    path.join(outDir, name.replace('.toml', '')),
    ['--no-build', '--repetitions', '1'],
    timeoutMs
  )
  if (result.status !== 0) {
    console.error(result.output.split('\n').slice(-30).join('\n'))
    throw new Error(
      `${name} ${result.timedOut ? 'did not finish in the time that calibration is given' : 'does not pass'}, so it is not soaked`
    )
  }
  return { cycles: warmup + measured, seconds: result.seconds }
}
// The start of a run on a hosted runner varies by a minute or more, and a cycle of one workload takes a thousandth of a second where one of another
// takes a tenth, so a count of cycles cannot be chosen beforehand, and two runs cannot give the time of a cycle. The runs grow, each sized from what
// the one before took, until one takes `ENOUGH` of the soak's time beyond the start. The start is taken as the shortest time that any run took.
//
// The time of a cycle is not always the same. A recovery cycle of W5 takes more the longer the run is (0.095 s at 800 cycles and 0.24 s at 12,810
// on a hosted runner), so the soak is sized with a power law fitted to the last two runs, t - start = c * cycles ^ p, with p between 1 and 2.
const target = hours * 3600 * SOAK_SHARE
const points = []
const measure = cycles => {
  // A run that is stuck, and not slow, would otherwise hold the job until its own limit, so a run is given the whole of the calibration's share.
  const point = timed(`calibration-${points.length}.toml`, cycles - warmup, hours * 3600 * CALIBRATION_BUDGET * 1000)
  points.push(point)
  console.log(`${point.cycles} cycles took ${point.seconds.toFixed(1)} s`)
  return point
}
const startOf = () => Math.min(...points.map(point => point.seconds))
measure(setting(text, 'measurement_iterations') + warmup)
measure(FIRST_LONG_CYCLES)
let spent = points.reduce((sum, point) => sum + point.seconds, 0)
for (;;) {
  const last = points[points.length - 1]
  const extra = last.seconds - startOf()
  if (extra >= target * ENOUGH || last.cycles * 2 > MAX_CALIBRATION_CYCLES) break
  // A run that was no longer than the start says nothing of the time of a cycle, and is made much longer. Otherwise the cycles are scaled by the time
  // that is wanted over the time that it took, which is more than is needed if the time grows faster than the cycles do, and is not much more.
  const scale = extra > 1 ? Math.min(50, Math.max(2, (target * ENOUGH) / extra)) : 50
  const cycles = Math.min(MAX_CALIBRATION_CYCLES, Math.ceil(last.cycles * scale))
  if (spent > hours * 3600 * CALIBRATION_BUDGET) break
  spent += measure(cycles).seconds
}
const start = startOf()
const last = points[points.length - 1]
const before = points.filter(point => point.cycles < last.cycles && point.seconds - start > 5).pop()
const extra = last.seconds - start
if (!(extra > 1))
  throw new Error(
    `the time of a cycle could not be worked out: the longest run took ${last.seconds.toFixed(1)} s, which is not more than the ${start.toFixed(1)} s that starting a browser can take`
  )
const growth = before
  ? Math.min(2, Math.max(1, Math.log(extra / (before.seconds - start)) / Math.log(last.cycles / before.cycles)))
  : 1
const perCycle = extra / last.cycles
console.log(
  `a start of ${start.toFixed(1)} s, ${perCycle.toFixed(5)} s a cycle at ${last.cycles} cycles, and a time that grows as cycles to the power ${growth.toFixed(2)}`
)
const sized = Math.floor(last.cycles * ((target - start) / extra) ** (1 / growth))
const wanted = Math.max(setting(text, 'measurement_iterations'), sized - warmup)
const measured = Math.min(wanted, MAX_CYCLES)
const capped = measured < wanted
const wallMs = Math.ceil(hours * 3600 * 1000 * 1.5)
const soak = path.join(outDir, `soak-${path.basename(manifestPath)}`)
fs.writeFileSync(soak, withCycles(text, measured, wallMs))
console.log(
  `${measured} measured cycles for ${hours} hours${capped ? `, and ${wanted} were wanted: the run is capped at ${MAX_CYCLES} cycles by the memory that the harness keeps for each, so it will end sooner than the hours that were asked for` : ''}`
)

const started = new Date()
// `wall_ms` in a manifest is only checked to be above zero and is never enforced, so the limit is enforced here: a soak that runs over its hours by half
// again was mis-sized, or is stuck, and is stopped and said to be so.
const soaked = orchestrate(soak, path.join(outDir, 'soak'), ['--no-build', '--repetitions', '1'], wallMs)
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
  growth_exponent: Number(growth.toFixed(3)),
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
          ? `stopped after ${Math.round(soaked.seconds)} s, which is the limit of 1.5 times the ${hours} hours: the cycles took longer than calibration showed\n`
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
  `\n${record.passed ? 'no growth: every endurance gate passed' : 'the run stopped before it reported its gates, or a gate FAILED'} after ${record.hours_run} hours; see ${path.relative(root, path.join(outDir, 'endurance-record.json'))}`
)
process.exit(record.passed ? 0 : 1)
