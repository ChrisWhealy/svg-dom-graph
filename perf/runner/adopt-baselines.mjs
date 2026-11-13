// Turns the results of a `perf-baseline` run into committed, approved baselines, in one command.
//
//   node perf/runner/adopt-baselines.mjs [<run id>] --reason TEXT [--approve] [--accept-noise TEXT] [--accept-regression TEXT]
//                                         [--commit] [--dry-run] [--allow-dirty]
//
// With no run id it takes the latest successful run of perf-baseline.yml. For that run it
//
//   1. downloads each job's artifact (`perf-baseline-<sha>-<sample>`) with the GitHub CLI, and reads the environment that it was measured in,
//      which includes the processor that GitHub gave the job. Two jobs of one environment are one baseline, so only the first is promoted:
//      the second would overwrite it, and the two are never pooled;
//   2. promotes each environment's results with promote-from-run.mjs, which makes every refusal that it ever made (a run that is not valid,
//      runs of more than one commit, a tree with changes, fewer than three runs, a promotion over a regression without --accept-regression);
//   3. says, for every baseline now in perf/baselines, how many runs it has, how steady they are, and whether it is ready to approve;
//   4. with --approve, approves the baselines that are ready, as `qualify.mjs --approve` does. One whose runs vary by more than the limit is
//      approved only with --accept-noise TEXT, which is written into its record. Without --approve it only reports.
//   5. with --commit, commits perf/baselines with a message that names the run. Without it nothing is committed, and the files are left for a
//      person to review: a baseline is a decision, and a job that promoted its own results could make a slow commit its own reference.
//
// --dry-run promotes into a scratch copy and approves nothing. The next step is `perf-detection`, which shows the comparison working against
// what this adopted: push the commit, start the workflow, and run collect-detection.mjs on its run.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { RUNS_TO_BLOCK, baselinesDir, environmentKey, readPromotion, resultFiles, root } from '../lib/baselines.mjs'
import { NOISE_LIMIT, isQualified, shortfalls, spreadOf } from '../lib/qualification.mjs'

const argument = name => {
  const at = process.argv.indexOf(name)
  return at < 0 ? null : process.argv[at + 1]
}
const flag = name => process.argv.includes(name)
const WITH_VALUE = ['--reason', '--accept-noise', '--accept-regression']
const positional = process.argv.slice(2).filter((a, i, all) => !a.startsWith('--') && !WITH_VALUE.includes(all[i - 1]))
const reason = argument('--reason')
if (!reason) throw new Error('--reason is required: a baseline records why it was promoted')
const dryRun = flag('--dry-run')
const noise = argument('--accept-noise')
const regression = argument('--accept-regression')

const run = (command, args, options = {}) =>
  spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28, ...options })
const must = (result, what) => {
  if (result.status !== 0) throw new Error(`${what} failed:\n${(result.stderr || result.stdout || '').trim()}`)
  return result.stdout
}
const step = text => console.log(`\n=== ${text}`)

// 1. The run, and its artifacts, grouped by the environment each was measured in.
step('the run')
const runId =
  positional[0] ??
  must(
    run('gh', [
      'run',
      'list',
      '--workflow',
      'perf-baseline.yml',
      '--status',
      'success',
      '--limit',
      '1',
      '--json',
      'databaseId',
      '-q',
      '.[0].databaseId'
    ]),
    'finding the latest run of perf-baseline.yml'
  ).trim()
if (!runId) throw new Error('there is no successful run of perf-baseline.yml: start it from the Actions tab')
const names = JSON.parse(
  must(
    run('gh', [
      'api',
      `repos/{owner}/{repo}/actions/runs/${runId}/artifacts`,
      '--jq',
      '[.artifacts[].name | select(startswith("perf-baseline-"))] | sort'
    ]),
    'listing the artifacts'
  )
).filter(Boolean)
if (names.length === 0) throw new Error(`run ${runId} has no perf-baseline-* artifacts`)
console.log(`run ${runId}: ${names.length} artifacts`)

const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'adopt-baselines-'))
const byEnvironment = new Map()
for (const name of names) {
  const dir = path.join(scratch, name)
  must(run('gh', ['run', 'download', runId, '--name', name, '--dir', dir]), `downloading ${name}`)
  const first = fs
    .readdirSync(dir, { withFileTypes: true })
    .filter(e => e.isDirectory())
    .map(e => resultFiles(path.join(dir, e.name)))
    .find(files => files.length > 0)
  if (!first) {
    console.log(`${name}: no results in it, skipped`)
    continue
  }
  const key = environmentKey(first[0])
  const list = byEnvironment.get(key) ?? []
  list.push({ name, dir })
  byEnvironment.set(key, list)
  console.log(`${name}: ${key}`)
}
if (byEnvironment.size === 0) throw new Error('none of the artifacts has results')

// 2. Promote one artifact for each environment.
step(dryRun ? 'what would be promoted (a dry run writes nothing)' : 'promoting')
let refused = 0
for (const [key, list] of byEnvironment) {
  const chosen = list[0]
  if (list.length > 1)
    console.log(`${key}: ${list.map(l => l.name).join(', ')} are one environment, so only ${chosen.name} is promoted`)
  const args = [path.join(root, 'perf/runner/promote-from-run.mjs'), '--from-dir', chosen.dir, '--reason', reason]
  if (regression) args.push('--accept-regression', regression)
  if (flag('--allow-dirty')) args.push('--allow-dirty')
  if (dryRun) args.push('--dry-run')
  const promoted = run('node', args, { env: process.env })
  process.stdout.write(promoted.stdout)
  if (promoted.status !== 0) {
    refused++
    process.stderr.write(promoted.stderr)
  }
}
fs.rmSync(scratch, { recursive: true, force: true })
if (refused > 0) {
  console.error(
    `\n${refused} promotion(s) were refused: what was promoted is in the working tree, and nothing was approved or committed`
  )
  process.exit(1)
}
if (dryRun) process.exit(0)

// 3. Where every baseline stands.
const baselineDirs = []
const walk = dir => {
  if (!fs.existsSync(dir)) return
  if (fs.existsSync(path.join(dir, 'promotion.json'))) return baselineDirs.push(dir)
  for (const entry of fs.readdirSync(dir, { withFileTypes: true }))
    if (entry.isDirectory()) walk(path.join(dir, entry.name))
}
walk(baselinesDir)
step('the baselines')
const approved = []
const waiting = []
for (const dir of baselineDirs.sort()) {
  const promotion = readPromotion(dir)
  const lacking = shortfalls(dir, spreadOf(dir))
  const name = path.relative(baselinesDir, dir)
  if (isQualified(dir)) {
    console.log(`${name}: ${promotion.runs.length} runs, already approved`)
    continue
  }
  const onlyNoise = lacking.length > 0 && lacking.every(text => text.includes('vary by more than'))
  const state =
    lacking.length === 0
      ? 'ready to approve'
      : onlyNoise
        ? `needs --accept-noise (${lacking[0]})`
        : `cannot be approved yet: ${lacking.join('; ')}`
  console.log(`${name}: ${promotion.runs.length} runs of ${RUNS_TO_BLOCK} needed, ${state}`)
  if (lacking.length === 0 || (onlyNoise && noise)) {
    if (flag('--approve')) {
      const args = [path.join(root, 'perf/runner/qualify.mjs'), '--approve', dir, '--reason', reason]
      if (onlyNoise) args.push('--accept-noise', noise)
      const result = run('node', args)
      if (result.status === 0) approved.push(name)
      else console.error(result.stderr.trim())
    } else waiting.push(name)
  } else waiting.push(name)
}

// 4. The change, and the commit if it was asked for.
step('the change')
const status = run('git', ['status', '--short', '--', 'perf/baselines']).stdout.trim()
console.log(status || 'no change to perf/baselines')
if (approved.length) console.log(`approved: ${approved.join(', ')}`)
if (waiting.length)
  console.log(
    `not approved: ${waiting.length}${flag('--approve') ? '' : ' (add --approve to approve those that are ready)'}`
  )
if (flag('--commit') && status) {
  must(run('git', ['add', 'perf/baselines']), 'staging the baselines')
  must(
    run('git', [
      'commit',
      '-m',
      `Promote${approved.length ? ' and approve' : ''} baselines from perf-baseline run ${runId}\n\n${reason}`
    ]),
    'committing'
  )
  console.log('committed. Push it, start `perf-detection`, and run collect-detection.mjs on its run.')
} else if (status)
  console.log('\nReview it, then: git add perf/baselines && git commit    (or run this again with --commit)')
