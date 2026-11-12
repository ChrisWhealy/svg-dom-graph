// Runs an endurance manifest for hours, and writes down what it ran and whether anything grew.
//
//   node perf/runner/endurance-soak.mjs <manifest.toml> --hours H [--layers b] [--out-dir DIR]
//
// The manifests are 200 cycles, which is minutes. A long soak is the same manifest with more cycles, as the manifests say, so this works out how
// many cycles fill the hours: it times the manifest at two lengths, takes the time of one cycle from the difference (which leaves out the start of
// the browser), and writes a copy of the manifest with that many measured cycles and a wall limit that
// allows for them. The copy goes to the output directory, and the manifest in the repository is not changed.
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
  const result = orchestrate(file, path.join(outDir, name.replace('.toml', '')), ['--repetitions', '1'])
  if (result.status !== 0) {
    console.error(result.output.split('\n').slice(-30).join('\n'))
    throw new Error(`${name} does not pass, so it is not soaked`)
  }
  return { cycles: warmup + measured, seconds: result.seconds }
}
const short = timed('calibration-short.toml', setting(text, 'measurement_iterations'))
const long = timed('calibration-long.toml', setting(text, 'measurement_iterations') * 8)
const perCycle = Math.max((long.seconds - short.seconds) / (long.cycles - short.cycles), 1e-4)
console.log(
  `${perCycle.toFixed(4)} s a cycle (${short.seconds.toFixed(1)} s for ${short.cycles}, ${long.seconds.toFixed(1)} s for ${long.cycles})`
)
const measured = Math.max(setting(text, 'measurement_iterations'), Math.floor((hours * 3600) / perCycle) - warmup)
const wallMs = Math.ceil(hours * 3600 * 1000 * 1.5)
const soak = path.join(outDir, `soak-${path.basename(manifestPath)}`)
fs.writeFileSync(soak, withCycles(text, measured, wallMs))
console.log(`${measured} measured cycles for ${hours} hours`)

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
  layers,
  started_at: started.toISOString(),
  host: { platform: process.platform, cpu: os.cpus()[0]?.model ?? null, cpus: os.cpus().length },
  exit_status: soaked.status,
  passed: soaked.status === 0 && gates.length > 0 && gates.every(g => g.outcome === 'pass'),
  gates
}
fs.writeFileSync(path.join(outDir, 'endurance-record.json'), JSON.stringify(record, null, 2) + '\n')
fs.writeFileSync(path.join(outDir, 'soak.log'), soaked.output)
console.log(soaked.output.split('\n').slice(-25).join('\n'))
console.log(
  `\n${record.passed ? 'no growth: every endurance gate passed' : 'an endurance gate FAILED, or none was reported'} after ${record.hours_run} hours; see ${path.relative(root, path.join(outDir, 'endurance-record.json'))}`
)
process.exit(record.passed ? 0 : 1)
