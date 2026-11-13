// Shows how the time of an endurance run grows with its cycles, on the machine that this runs on.
//
//   node perf/runner/endurance-scaling.mjs <manifest.toml> [--longest SECONDS] [--out-dir DIR]
//
// endurance-soak.mjs sizes a soak from two short runs, and on a hosted runner the soaks took half as long again as that said. The start of a run
// varies there by more than a minute, so two short runs cannot say whether a cycle costs the same at the end of a long run as at the start. This
// makes runs of 200 cycles and then four times as many each time, until one takes `--longest` seconds (600 by default), and prints for each the
// time, the time that it added to the one before, and so the time of a cycle between the two. A time of a cycle that stays the same says that the
// sizing was wrong because of the start, and one that rises says that the cycles slow down. The runs are not soaks and say nothing about growth.
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
const longest = Number(argument('--longest', 600))
const outDir = path.resolve(argument('--out-dir', path.join(root, 'target/endurance-scaling')))

if (!fs.existsSync(manifestPath))
  throw new Error('usage: node perf/runner/endurance-scaling.mjs <manifest.toml> [--longest SECONDS]')

fs.mkdirSync(outDir, { recursive: true })
const text = fs.readFileSync(manifestPath, 'utf8')
const warmup = Number(text.match(/^warmup_iterations\s*=\s*(\d+)/m)[1])

const timeOf = measured => {
  const file = path.join(outDir, `scaling-${measured}.toml`)
  fs.writeFileSync(
    file,
    text
      .replace(/^measurement_iterations\s*=.*$/m, `measurement_iterations = ${measured}`)
      .replace(/^repetitions\s*=.*$/m, 'repetitions = 1')
      .replace(/^wall_ms\s*=.*$/m, 'wall_ms = 3600000')
  )
  const started = Date.now()
  const ran = spawnSync(
    'node',
    [
      path.join(root, 'perf/runner/run.mjs'),
      file,
      '--mode',
      'endurance',
      '--layers',
      'b',
      '--no-build',
      '--repetitions',
      '1',
      '--out-dir',
      path.join(outDir, `run-${measured}`)
    ],
    { cwd: root, encoding: 'utf8', maxBuffer: 1 << 30 }
  )
  const seconds = (Date.now() - started) / 1000

  if (ran.status !== 0)
    throw new Error(`${measured} cycles did not pass:\n${(ran.stdout + ran.stderr).split('\n').slice(-15).join('\n')}`)
  fs.rmSync(path.join(outDir, `run-${measured}`), { recursive: true, force: true })

  return seconds
}

console.log(
  `${os.cpus()[0]?.model}, ${os.cpus().length} cpus, ${(os.totalmem() / 2 ** 30).toFixed(1)} GiB, load ${os
    .loadavg()
    .map(n => n.toFixed(2))
    .join(' ')}`
)
console.log('cycles       seconds   added   s a cycle between this and the run before')
let before = null

for (let measured = 200; ; measured *= 4) {
  const cycles = warmup + measured
  const seconds = timeOf(measured)
  const added = before ? seconds - before.seconds : null
  const rate = before ? added / (cycles - before.cycles) : null
  console.log(
    `${String(cycles).padStart(8)} ${seconds.toFixed(1).padStart(10)} ${added === null ? '' : added.toFixed(1).padStart(7)} ${rate === null ? '' : rate.toFixed(5).padStart(10)}`
  )
  before = { cycles, seconds }

  if (seconds >= longest) break
}
