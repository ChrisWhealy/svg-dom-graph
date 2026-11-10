// Promotes the results of one CI run to be the baselines of the workloads that it measured.
//
//   node perf/runner/promote-from-run.mjs <run-id> --reason TEXT
//        [--workloads a,b] [--layers A,B,C] [--mode performance] [--accept-regression TEXT]
//        [--artifact NAME] [--from-dir DIR] [--allow-dirty] [--dry-run]
//
// It downloads the `perf-smoke-<sha>` artifact of the run with the GitHub CLI (`gh`, signed in), finds the workloads and the layers in it, and
// gives each workload and layer to `promote-baseline.mjs`, which is still the only thing that writes under perf/baselines/ and still makes every
// refusal: a run that is not valid, runs from more than one commit or from a tree with uncommitted changes, fewer than three runs, runs from
// more than one environment, and a promotion over a regression without a reason. This command decides nothing about that. It only fetches, sorts
// and calls, and it fails if any promotion was refused, so that a partial baseline is never mistaken for a whole one.
//
// It never commits. The new baselines are in the working tree for a person to review, which is what makes a baseline a decision: if a job
// promoted its own results, a slow commit could become its own reference. `--dry-run` promotes into a scratch copy of perf/baselines/ and
// says what would have happened, including the comparison with the baseline that would be replaced. `--from-dir` uses an artifact that has
// already been downloaded and unpacked, and needs no `gh`.
//
// The run has to have finished and succeeded. The commit that it measured has to be known to git, and is fetched from origin if it is not.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { resultFiles, root } from '../lib/baselines.mjs'

const parse = argv => {
  const options = {
    mode: 'performance',
    layers: ['A', 'B', 'C'],
    workloads: null,
    dryRun: false,
    allowDirty: false,
    fromDir: null,
    artifact: null
  }
  const positional = []
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]
    if (arg === '--reason') options.reason = argv[++i]
    else if (arg === '--workloads') options.workloads = argv[++i].split(',')
    else if (arg === '--layers') options.layers = argv[++i].toUpperCase().split(',')
    else if (arg === '--mode') options.mode = argv[++i]
    else if (arg === '--accept-regression') options.acceptRegression = argv[++i]
    else if (arg === '--artifact') options.artifact = argv[++i]
    else if (arg === '--from-dir') options.fromDir = argv[++i]
    else if (arg === '--allow-dirty') options.allowDirty = true
    else if (arg === '--dry-run') options.dryRun = true
    else if (arg.startsWith('--')) throw new Error(`unknown option ${arg}`)
    else positional.push(arg)
  }
  options.runId = positional[0] ?? null
  if (!options.reason) throw new Error('--reason is required: a baseline records why it was promoted')
  if (!options.fromDir && !options.runId)
    throw new Error('usage: promote-from-run.mjs <run-id> --reason TEXT, or --from-dir DIR --reason TEXT')
  if (options.layers.some(layer => !['A', 'B', 'C'].includes(layer)))
    throw new Error('--layers is a list of A, B and C')
  return options
}

const run = (command, args, options = {}) =>
  spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28, ...options })
const must = (result, what) => {
  if (result.status !== 0) throw new Error(`${what} failed:\n${(result.stderr || result.stdout || '').trim()}`)
  return result
}

/** The folder that holds the workloads' folders, which `gh` may have put one level down. */
const workloadRoot = dir => {
  const inside = fs.readdirSync(dir, { withFileTypes: true }).filter(entry => entry.isDirectory())
  const nested = inside.find(entry => entry.name.startsWith('perf-smoke-'))
  return nested ? path.join(dir, nested.name) : dir
}

/**
 * What is in an unpacked artifact: for each workload folder, the layers that have runs of `mode`. A layer is read from the run's own metrics
 * and id, as promote-baseline.mjs reads it, so that the two agree on what a layer is.
 */
export const survey = (dir, mode) => {
  const found = []
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue
    const folder = path.join(dir, entry.name)
    let docs
    try {
      docs = resultFiles(folder).map(file => JSON.parse(fs.readFileSync(file, 'utf8')))
    } catch {
      continue
    }
    const layers = ['A', 'B', 'C'].filter(layer =>
      docs.some(
        doc => doc.mode === mode && (doc.metrics.some(m => m.layer === layer) || doc.run_id.includes(`-${layer}-`))
      )
    )
    if (docs.length > 0)
      found.push({ workload: entry.name, folder, layers, commits: [...new Set(docs.map(d => d.environment.git_sha))] })
  }
  return found.sort((a, b) => a.workload.localeCompare(b.workload))
}

const download = options => {
  must(
    run('gh', ['--version']),
    'the GitHub CLI (gh) is needed to download a run, and is not installed. Download the artifact by hand and use --from-dir'
  )
  const view = JSON.parse(
    must(
      run('gh', ['run', 'view', options.runId, '--json', 'headSha,status,conclusion,workflowName,url']),
      `reading run ${options.runId}`
    ).stdout
  )
  console.log(
    `run ${options.runId}: ${view.workflowName}, ${view.status} (${view.conclusion ?? 'no conclusion'}), commit ${view.headSha.slice(0, 10)}\n  ${view.url}`
  )
  if (view.status !== 'completed' || view.conclusion !== 'success')
    throw new Error(
      `the run has to have finished and succeeded, and it is ${view.status} (${view.conclusion}): a baseline from a failing run measures a broken commit`
    )
  const known = sha => run('git', ['cat-file', '-e', `${sha}^{commit}`]).status === 0
  if (!known(view.headSha)) {
    console.log(`git does not know ${view.headSha.slice(0, 10)}, so it is fetched from origin`)
    run('git', ['fetch', '--quiet', 'origin', view.headSha])
    if (!known(view.headSha))
      throw new Error(`git still does not know the commit ${view.headSha}: fetch it, and promote again`)
  }
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), `promote-run-${options.runId}-`))
  const name = options.artifact ?? `perf-smoke-${view.headSha}`
  must(run('gh', ['run', 'download', options.runId, '--name', name, '--dir', dir]), `downloading the artifact ${name}`)
  return { dir: workloadRoot(dir), cleanup: dir, sha: view.headSha }
}

const main = () => {
  const options = parse(process.argv.slice(2))
  let source
  if (options.fromDir) source = { dir: workloadRoot(path.resolve(options.fromDir)), cleanup: null, sha: null }
  else source = download(options)

  const workloads = survey(source.dir, options.mode).filter(
    w => !options.workloads || options.workloads.includes(w.workload)
  )
  if (options.workloads) {
    const missing = options.workloads.filter(name => !workloads.some(w => w.workload === name))
    if (missing.length > 0)
      throw new Error(
        `no ${options.mode} results of ${missing.join(', ')} in the artifact. It has: ${
          survey(source.dir, options.mode)
            .map(w => w.workload)
            .join(', ') || 'nothing'
        }`
      )
  }
  if (workloads.length === 0) throw new Error(`there is no ${options.mode} result in ${source.dir}`)
  if (source.sha)
    for (const w of workloads)
      if (w.commits.some(sha => sha !== source.sha))
        throw new Error(`${w.workload} holds results from another commit than the run's ${source.sha.slice(0, 10)}`)

  // A dry run promotes into a copy of the baselines, so that the comparison with the baseline that would be replaced is the real one.
  let baselinesDir = null
  if (options.dryRun) {
    baselinesDir = fs.mkdtempSync(path.join(os.tmpdir(), 'promote-dry-run-'))
    if (fs.existsSync(path.join(root, 'perf/baselines')))
      fs.cpSync(path.join(root, 'perf/baselines'), baselinesDir, { recursive: true })
  }

  const outcomes = []
  for (const workload of workloads) {
    for (const layer of options.layers) {
      if (!workload.layers.includes(layer)) {
        outcomes.push({
          workload: workload.workload,
          layer,
          status: 'skipped',
          detail: `no ${options.mode} run of layer ${layer}`
        })
        continue
      }
      const args = [
        path.join(root, 'perf/runner/promote-baseline.mjs'),
        '--from',
        workload.folder,
        '--layer',
        layer,
        '--mode',
        options.mode,
        '--reason',
        options.reason
      ]
      if (options.acceptRegression) args.push('--accept-regression', options.acceptRegression)
      if (options.allowDirty) args.push('--allow-dirty')
      if (baselinesDir) args.push('--baselines-dir', baselinesDir)
      const ran = run('node', args)
      const text = `${ran.stdout}${ran.stderr}`.trim()
      outcomes.push(
        ran.status === 0
          ? {
              workload: workload.workload,
              layer,
              status: options.dryRun ? 'would be promoted' : 'promoted',
              detail: text.split('\n').find(l => l.startsWith('promoted')) ?? text
            }
          : { workload: workload.workload, layer, status: 'refused', detail: text }
      )
    }
  }

  console.log(
    `\n${options.dryRun ? 'dry run, nothing was written: ' : ''}${workloads.length} workload(s), mode ${options.mode}`
  )
  for (const o of outcomes) {
    console.log(`  ${o.status.padEnd(18)} ${o.workload} layer ${o.layer}`)
    if (o.status === 'refused')
      console.log(
        o.detail
          .split('\n')
          .map(line => `      ${line}`)
          .join('\n')
      )
    else if (o.status === 'skipped') console.log(`      ${o.detail}`)
  }
  const refused = outcomes.filter(o => o.status === 'refused')
  const done = outcomes.filter(o => o.status === 'promoted')
  if (done.length > 0) console.log('\nreview what was written, then commit it:\n  git add perf/baselines\n  git status')
  if (refused.length > 0)
    console.log(
      `\n${refused.length} promotion(s) were refused, so the baselines are incomplete. Nothing was committed.`
    )
  for (const dir of [source.cleanup, baselinesDir]) if (dir) fs.rmSync(dir, { recursive: true, force: true })
  process.exit(refused.length > 0 ? 1 : 0)
}

if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    main()
  } catch (error) {
    console.error(`promote-from-run: ${error.message}`)
    process.exit(1)
  }
}
