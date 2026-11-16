// Keeps the records of the `perf-endurance` workflow as evidence.
//
//   node perf/runner/collect-endurance.mjs <run id>
//
// Downloads the `endurance-*` artifacts of a run with the GitHub CLI, and copies each soak that passed into perf/evidence/endurance/, named for the
// manifest, the day and the hours. The run's id and commit are added, since the record does not say which commit it measured. A soak that failed is
// not kept: it is a fault to look at and not evidence, and `qualify.mjs` would list it as FAILED beside the ones that count. Nothing is committed:
// review the files, and commit them.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { root } from '../lib/baselines.mjs'

const run = process.argv[2]
if (!run) throw new Error('usage: node perf/runner/collect-endurance.mjs <run id>')

const gh = args => {
  const result = spawnSync('gh', args, { cwd: root, encoding: 'utf8' })
  if (result.status !== 0) throw new Error(`gh ${args.slice(0, 2).join(' ')} failed:\n${result.stderr}`)
  return result.stdout
}
const commit = JSON.parse(gh(['run', 'view', run, '--json', 'headSha'])).headSha
const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-endurance-'))
gh(['run', 'download', run, '--pattern', 'endurance-*', '--dir', temp])

const target = path.join(root, 'perf/evidence/endurance')
fs.mkdirSync(target, { recursive: true })
const files = fs.readdirSync(temp, { recursive: true }).filter(f => String(f).endsWith('endurance-record.json'))
let kept = 0
for (const file of files) {
  const record = JSON.parse(fs.readFileSync(path.join(temp, String(file)), 'utf8'))
  const manifest = path.basename(record.manifest, '.toml')
  if (!record.passed) {
    console.log(
      `not kept: ${manifest} did not pass (${record.timed_out ? 'a segment timed out' : 'a gate failed or the run stopped'})`
    )
    continue
  }
  const name = `${manifest}-${record.started_at.slice(0, 10)}-${record.hours_run}h.json`
  fs.writeFileSync(path.join(target, name), JSON.stringify({ ...record, run_id: Number(run), commit }, null, 2) + '\n')
  console.log(
    `kept ${path.relative(root, path.join(target, name))}: ${record.hours_run} hours, ${record.cycles_completed} cycles`
  )
  kept++
}
fs.rmSync(temp, { recursive: true, force: true })
console.log(
  `${kept} of ${files.length} soaks kept. Run node perf/runner/qualify.mjs to see whether endurance is qualified.`
)
