// Decides, from the records of `baseline-status.mjs` in a directory, whether the baselines need to be run again, and writes what to tell a person.
//
//   node perf/runner/report-baseline-status.mjs <directory of status.json files> [--out FILE.md]
//
// An environment needs a new baseline if a job ran in it and there is no approved baseline for it (`no_baseline`, or `not_approved`). A job that
// could not measure is reported, and is not a reason for a new baseline. Prints `needed=true` or `needed=false` as its last line, for a workflow to
// read, and writes the text of an issue to the file. It writes nothing for the issue when nothing is needed.
import fs from 'node:fs'
import path from 'node:path'

const directory = process.argv[2]

if (!directory) throw new Error('usage: report-baseline-status.mjs <directory> [--out FILE.md]')

const at = process.argv.indexOf('--out')
const out = at < 0 ? null : path.resolve(process.argv[at + 1])
const records = fs
  .readdirSync(directory, { recursive: true })
  .filter(f => String(f).endsWith('status.json'))
  .map(f => JSON.parse(fs.readFileSync(path.join(directory, String(f)), 'utf8')))
const missing = new Map()

for (const r of records) if (['no_baseline', 'not_approved'].includes(r.status)) missing.set(r.environment, r)

const covered = [...new Set(records.filter(r => r.status === 'covered').map(r => r.environment))]
const unmeasured = records.filter(r => r.status === 'could_not_measure')
const lines = [
  `The weekly check ran ${records.length} job(s). ${covered.length} environment(s) are covered by an approved baseline, and ${missing.size} are not.`,
  ''
]

if (missing.size > 0) {
  lines.push(
    'These environments have no approved baseline, so `perf-smoke` and `perf-detection` can compare nothing on them:',
    ''
  )
  for (const [key, r] of missing)
    lines.push(
      `- \`${key}\` (${r.status === 'not_approved' ? `a baseline of ${r.baseline.runs} runs exists and is not approved` : 'no baseline'})`
    )
  lines.push(
    '',
    'To fix it:',
    '',
    '1. Start the `perf-baseline` workflow from the Actions tab, and wait for it to succeed.',
    '2. In a clean checkout, run `node perf/runner/adopt-baselines.mjs --reason "..." --dry-run`, then again with `--approve`, and review the spreads.',
    '3. Commit and push `perf/baselines`, start `perf-detection`, and run `node perf/runner/collect-detection.mjs <run id>`.',
    '',
    'The runner image has probably moved to another Chrome or kernel, or GitHub gave a job another processor. The environments that are covered are listed in the weekly run.'
  )
}

if (covered.length) lines.push('', 'Covered:', '', ...covered.map(k => `- \`${k}\``))
if (unmeasured.length)
  lines.push('', `${unmeasured.length} job(s) could not measure, so they say nothing about a baseline.`)
if (out && missing.size > 0) fs.writeFileSync(out, lines.join('\n') + '\n')

console.log(lines.join('\n'))
console.log(`needed=${missing.size > 0}`)
