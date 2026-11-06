// Promotes a collection of runs to be the baseline of a workload. It is the only thing that writes under perf/baselines/.
//
//   node perf/runner/promote-baseline.mjs --from DIR --layer A|B|C --reason TEXT
//                                         [--mode performance] [--accept-regression TEXT] [--min-runs 3] [--allow-dirty]
//
// A baseline is never updated by a run. This copies the results of the runs in DIR, and the samples beside them, to
// perf/baselines/<manifest>/<layer>/<environment>/, and writes promotion.json there with the commit, the hash of the workload,
// the environment, the method and the reason. It does not commit anything. The files are for a person to review and commit.
//
// It refuses to promote
//   * a run that is not valid, since a baseline of a workload that did not do its work compares nothing, or a run of a candidate
//     that was changed on purpose for a self-test;
//   * runs from a tree with uncommitted changes, or from more than one commit, or from a commit that git does not know,
//     since a baseline has to say which code it measured (--allow-dirty is for trying the command out, and it says so);
//   * runs that were not measured alike, in one environment, of one manifest and one operation stream;
//   * fewer runs than --min-runs;
//   * runs that regress against the baseline they would replace, unless --accept-regression gives the reason, which is
//     written into the record together with the metrics that regressed.
import fs from 'node:fs'
import path from 'node:path'
import {
  METHOD_VERSION,
  MIN_PROMOTION_RUNS,
  baselineDirFor,
  baselinesDir,
  compareCollections,
  environmentKey,
  readPromotion,
  regressions,
  resultFiles,
  root,
  run,
  summarise,
  validate
} from '../lib/baselines.mjs'

const parse = argv => {
  const options = { mode: 'performance', minRuns: MIN_PROMOTION_RUNS, allowDirty: false }
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i]
    if (arg === '--from') options.from = argv[++i]
    else if (arg === '--layer') options.layer = argv[++i]
    else if (arg === '--mode') options.mode = argv[++i]
    else if (arg === '--reason') options.reason = argv[++i]
    else if (arg === '--accept-regression') options.acceptRegression = argv[++i]
    else if (arg === '--min-runs') options.minRuns = Number(argv[++i])
    else if (arg === '--allow-dirty') options.allowDirty = true
    else if (arg === '--baselines-dir') options.baselinesDir = argv[++i]
    else throw new Error(`unknown option ${arg}`)
  }
  for (const needed of ['from', 'layer', 'reason'])
    if (!options[needed]) throw new Error(`--${needed} is required`)
  if (!['A', 'B', 'C'].includes(options.layer)) throw new Error('--layer is A, B or C')
  return options
}

/** Gives the reasons a collection may not be promoted, or an empty list. */
export const refusals = (docs, options, gitKnows) => {
  const reasons = []
  if (docs.length < options.minRuns) reasons.push(`${docs.length} runs, and at least ${options.minRuns} are needed`)
  for (const doc of docs)
    if (doc.status !== 'ok') reasons.push(`run ${doc.run_id} is ${doc.status}, and only a valid run may be a baseline`)
  for (const doc of docs)
    if (doc.injection) reasons.push(`run ${doc.run_id} is of a candidate that was changed on purpose, and that is never a baseline`)
  const shas = new Set(docs.map(d => d.environment.git_sha))
  if (shas.size > 1) reasons.push(`the runs are from ${shas.size} commits (${[...shas].map(s => s.slice(0, 10)).join(', ')})`)
  if (!options.allowDirty && docs.some(d => d.environment.git_dirty))
    reasons.push('the runs were made from a tree with uncommitted changes, so the commit does not say what was measured')
  for (const sha of shas) if (!gitKnows(sha)) reasons.push(`git does not know the commit ${sha.slice(0, 10)}`)
  if (new Set(docs.map(d => d.manifest.sha256)).size > 1) reasons.push('the runs are of more than one manifest')
  if (new Set(docs.map(d => d.manifest.operation_stream_sha256)).size > 1)
    reasons.push('the runs executed more than one operation stream')
  if (new Set(docs.map(d => d.manifest.id)).size > 1) reasons.push('the runs are of more than one workload')
  return reasons
}

const main = () => {
  const options = parse(process.argv.slice(2))
  const from = path.resolve(options.from)
  const base = options.baselinesDir ? path.resolve(options.baselinesDir) : baselinesDir
  const files = resultFiles(from)
    .map(file => ({ file, doc: JSON.parse(fs.readFileSync(file, 'utf8')) }))
    .filter(({ doc }) => doc.mode === options.mode)
    .filter(({ doc }) => doc.metrics.some(m => m.layer === options.layer) || doc.run_id.includes(`-${options.layer}-`))
  if (files.length === 0) throw new Error(`no ${options.mode} run of layer ${options.layer} in ${from}`)
  const docs = files.map(f => f.doc)

  const known = sha => run('git', ['cat-file', '-e', `${sha}^{commit}`]).status === 0
  const reasons = refusals(docs, options, known)
  const keys = new Set(files.map(f => environmentKey(f.file)))
  if (keys.size > 1) reasons.push(`the runs were measured in ${keys.size} environments (${[...keys].join(', ')})`)
  if (reasons.length > 0) {
    console.error('not promoted:')
    for (const reason of reasons) console.error(`  - ${reason}`)
    process.exit(1)
  }

  const manifestId = docs[0].manifest.id
  const key = [...keys][0]
  const destination = path.join(base, manifestId, options.layer, key)
  const previous = readPromotion(destination)

  // A promotion over a regression is possible, and only when someone says why.
  let accepted = null
  if (previous) {
    const out = path.join(from, `promotion-check-${options.layer}.comparison.json`)
    const report = compareCollections({ baselineDir: destination, candidateDirs: [from], layer: options.layer, mode: options.mode, out })
    const bad = regressions(report)
    console.log(summarise(report, `The new runs against the baseline they would replace`).join('\n'))
    if (bad.length > 0) {
      if (!options.acceptRegression) {
        console.error(
          `not promoted: ${bad.length} metric(s) regress against the current baseline (${bad.map(c => `${c.metric} ${c.statistic}`).slice(0, 4).join(', ')}).\n` +
            'Fix the regression, or promote it knowingly with --accept-regression "<why this is acceptable>".'
        )
        process.exit(1)
      }
      accepted = { reason: options.acceptRegression, metrics: [...new Set(bad.map(c => c.metric))] }
    } else if (options.acceptRegression) {
      console.log('--accept-regression was given and nothing regressed, so no regression is recorded')
    }
  }

  const record = {
    schema_version: 1,
    manifest_id: manifestId,
    layer: options.layer,
    mode: options.mode,
    promotion: {
      promoted_at: new Date().toISOString(),
      commit: docs[0].environment.git_sha,
      workload_hash: docs[0].manifest.sha256,
      environment_key: key,
      method_version: METHOD_VERSION
    },
    operation_stream_sha256: docs[0].manifest.operation_stream_sha256,
    reason: options.reason,
    runs: docs.map(d => d.run_id),
    comparison_method: {
      statistic: 'ratio of repetition medians',
      bootstrap: 'hierarchical: repetitions, then observations within each',
      ratio_tolerance: 0.1,
      confidence: 0.95
    },
    previous: previous ? { commit: previous.promotion.commit, promoted_at: previous.promotion.promoted_at } : null,
    accepted_regression: accepted
  }
  validate('promotion', record, 'the promotion record')

  // The old baseline's files are replaced, and git keeps their history. The new ones are written in full before the record is.
  fs.rmSync(destination, { recursive: true, force: true })
  fs.mkdirSync(destination, { recursive: true })
  for (const { file, doc } of files) {
    fs.copyFileSync(file, path.join(destination, path.basename(file)))
    if (doc.raw_samples) {
      const samples = fs.existsSync(path.join(path.dirname(file), path.basename(doc.raw_samples)))
        ? path.join(path.dirname(file), path.basename(doc.raw_samples))
        : path.join(root, doc.raw_samples)
      fs.copyFileSync(samples, path.join(destination, path.basename(doc.raw_samples)))
    }
  }
  fs.writeFileSync(path.join(destination, 'promotion.json'), JSON.stringify(record, null, 2) + '\n')

  console.log(`promoted ${docs.length} runs of ${manifestId} (layer ${options.layer}) to ${path.relative(root, destination)}`)
  if (accepted) console.log(`a regression in ${accepted.metrics.join(', ')} was accepted: ${accepted.reason}`)
  if (options.allowDirty) console.log('--allow-dirty was given: this baseline is not tied to a clean commit, and is not for committing')
  console.log(`review it, then: git add ${path.relative(root, destination)}`)
}

if (import.meta.url === `file://${process.argv[1]}`) {
  try {
    main()
  } catch (error) {
    console.error(`promote-baseline: ${error.message}`)
    process.exit(1)
  }
}
