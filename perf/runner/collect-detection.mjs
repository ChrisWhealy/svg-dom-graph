// Keeps the records of the `perf-detection` workflow as evidence.
//
//   node perf/runner/collect-detection.mjs <run id>
//
// Downloads the `perf-detection-*` artifacts of a run with the GitHub CLI, and copies each record that shows something into
// perf/evidence/detection/, named for the manifest, the environment and the commit. A record of a job that could not try (no baseline, an
// unapproved one, or a runner that is not the baseline's environment) shows nothing and is not kept. A record that does not hold is kept, since it
// is the evidence that the baseline or the comparison needs another look. Nothing is committed: review the files, and commit them.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { root } from '../lib/baselines.mjs'

const run = process.argv[2]

if (!run) throw new Error('usage: node perf/runner/collect-detection.mjs <run id>')

const temp = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-detection-'))
const downloaded = spawnSync('gh', ['run', 'download', run, '--pattern', 'perf-detection-*', '--dir', temp], {
  cwd: root,
  encoding: 'utf8'
})

if (downloaded.status !== 0) throw new Error(`downloading the artifacts of run ${run}:\n${downloaded.stderr}`)

const target = path.join(root, 'perf/evidence/detection')
fs.mkdirSync(target, { recursive: true })
const records = fs.readdirSync(temp, { recursive: true }).filter(f => String(f).endsWith('report.json'))
let kept = 0

for (const file of records) {
  const record = JSON.parse(fs.readFileSync(path.join(temp, String(file)), 'utf8'))
  if (record.outcome === 'skipped' || record.outcome === 'failed') {
    console.log(`not kept (${record.outcome}): ${record.summary}`)
    continue
  }

  const name = `${record.manifest}-${record.environment}-${record.commit.slice(0, 8)}.json`
  fs.writeFileSync(path.join(target, name), JSON.stringify(record, null, 2) + '\n')
  console.log(`kept ${path.relative(root, path.join(target, name))}: ${record.outcome}. ${record.summary}`)
  kept++
}

fs.rmSync(temp, { recursive: true, force: true })

console.log(
  `${kept} of ${records.length} records kept. Run node perf/runner/qualify.mjs to see which baselines have proof.`
)
