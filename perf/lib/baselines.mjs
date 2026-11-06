// What the comparison, the promotion and perf-smoke share: where a baseline lives, how a comparison is made and checked, and how
// one is written down for a person to read.
import fs from 'node:fs'
import path from 'node:path'
import { createRequire } from 'node:module'
import { spawnSync } from 'node:child_process'

export const root = path.resolve(import.meta.dirname, '../..')
export const binary = path.join(root, 'target/release/perf-harness')
// The directory can be moved with PERF_BASELINES_DIR, to try a promotion without touching the committed baselines.
export const baselinesDir = process.env.PERF_BASELINES_DIR
  ? path.resolve(process.env.PERF_BASELINES_DIR)
  : path.join(root, 'perf/baselines')

/** Raised when the statistic or the way a baseline is recorded changes in a way that makes old baselines incomparable. */
export const METHOD_VERSION = '1'
/** The baseline runs a comparison needs before its verdicts may block a change (design section 8.1). */
export const RUNS_TO_BLOCK = 10
/** The fewest runs that a baseline may be promoted from. Fewer than the comparison needs for a verdict is allowed, and says so. */
export const MIN_PROMOTION_RUNS = 3

const requireFromSchemas = createRequire(path.join(root, 'perf/schemas/package.json'))
const Ajv = requireFromSchemas('ajv/dist/2020.js')
const addFormats = requireFromSchemas('ajv-formats')
const ajv = new Ajv({ strict: true, strictTypes: false, strictRequired: false, allErrors: true })
addFormats(ajv)
for (const name of ['comparison', 'baseline-promotion'])
  ajv.addSchema(JSON.parse(fs.readFileSync(path.join(root, `perf/schemas/${name}.schema.json`), 'utf8')))
const validators = {
  comparison: ajv.getSchema('urn:svg-dom-graph:perf:comparison:1'),
  promotion: ajv.getSchema('urn:svg-dom-graph:perf:baseline-promotion:1')
}

/** Checks a document against a schema and throws with every reason if it does not satisfy it. */
export const validate = (kind, document, what) => {
  const check = validators[kind]
  if (!check(document)) throw new Error(`${what} does not satisfy the ${kind} schema: ${ajv.errorsText(check.errors)}`)
  return document
}

export const run = (command, args, options = {}) =>
  spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 512 * 1024 * 1024, ...options })

export const must = (result, what) => {
  if (result.status !== 0) throw new Error(`${what} failed:\n${result.stderr || result.stdout}`)
  return result
}

/** The result documents in a directory, which are the files that carry a `run_id`. */
export const resultFiles = dir =>
  fs
    .readdirSync(dir)
    .filter(name => name.endsWith('.json') && !name.endsWith('.cases.json'))
    .map(name => path.join(dir, name))
    .filter(file => {
      try {
        return 'run_id' in JSON.parse(fs.readFileSync(file, 'utf8'))
      } catch {
        return false
      }
    })
    .sort()

/** The name of the environment a result was measured in, which is the name of the directory its baseline is kept in. */
export const environmentKey = file => must(run(binary, ['baseline-key', file]), `reading the environment of ${file}`).stdout.trim()

/** Where the baseline of a manifest, a layer and an environment is kept. */
export const baselineDirFor = (manifestId, layer, key) => path.join(baselinesDir, manifestId, layer, key)

export const readPromotion = dir => {
  const file = path.join(dir, 'promotion.json')
  return fs.existsSync(file) ? JSON.parse(fs.readFileSync(file, 'utf8')) : null
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Compares the runs of `layer` and `mode` in `candidateDirs` with those in `baselineDir`, writes the report to `out`, and gives it
 * back checked against `comparison.schema.json`.
 */
export const compareCollections = ({ baselineDir, candidateDirs, layer, mode, out, settings = {} }) => {
  const promotion = readPromotion(baselineDir)
  if (!promotion) throw new Error(`${baselineDir} has no promotion.json, so it is not a baseline`)
  const args = [
    'compare',
    '--baseline', baselineDir,
    ...candidateDirs.flatMap(dir => ['--candidate', dir]),
    '--promotion', path.join(baselineDir, 'promotion.json'),
    '--layer', layer,
    '--mode', mode,
    '--generated-at', new Date().toISOString(),
    '--runs-to-block', String(RUNS_TO_BLOCK),
    '--out', out
  ]
  for (const [flag, value] of Object.entries(settings)) args.push(`--${flag}`, String(value))
  must(run(binary, args), 'the comparison')
  return validate('comparison', JSON.parse(fs.readFileSync(out, 'utf8')), `the comparison report ${out}`)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const show = value => (Math.abs(value) >= 100 ? value.toFixed(0) : Math.abs(value) >= 10 ? value.toFixed(1) : value.toFixed(2))

/** The verdicts of a report as lines of Markdown, for a person and for a CI job's summary. */
export const summarise = (report, title) => {
  const lines = [`### ${title}`, '']
  const { baseline, candidate, compatibility, method, gating } = report
  lines.push(
    `Baseline \`${baseline.git_sha.slice(0, 10)}\` (promoted ${baseline.promotion.promoted_at.slice(0, 10)}, ${baseline.run_ids.length} runs) ` +
      `against candidate \`${candidate.git_sha.slice(0, 10)}\` (${candidate.run_ids.length} runs). ` +
      `Ratio tolerance ${(method.ratio_tolerance * 100).toFixed(0)}%, ${(method.confidence * 100).toFixed(0)}% interval, ` +
      `${method.bootstrap_resamples} resamples, seed ${method.bootstrap_seed}.`,
    ''
  )
  if (!compatibility.comparable) {
    lines.push(`**Not comparable:** ${compatibility.reason}`, '')
    return lines
  }
  lines.push(
    gating.statistical_gates === 'blocking'
      ? `Statistical verdicts block a change: the baseline has ${gating.baseline_runs} runs.`
      : `Statistical verdicts only report: the baseline has ${gating.baseline_runs} runs, and blocking needs ${gating.baseline_runs_to_block}.`,
    ''
  )
  const counts = Object.groupBy(report.comparisons, c => c.verdict)
  lines.push(
    Object.entries(counts)
      .map(([verdict, list]) => `${list.length} ${verdict.replaceAll('_', ' ')}`)
      .join(', ') || 'nothing was compared',
    ''
  )
  const interesting = report.comparisons.filter(c => ['regression', 'improvement', 'inconclusive'].includes(c.verdict))
  if (interesting.length > 0) {
    lines.push('| metric | statistic | baseline | candidate | ratio | interval | verdict |', '|---|---|---|---|---|---|---|')
    for (const c of interesting)
      lines.push(
        `| ${c.metric} | ${c.statistic} | ${show(c.baseline.median)} | ${show(c.candidate.median)} | ${c.ratio.toFixed(3)} | ` +
          `${c.ratio_interval.low.toFixed(3)} to ${c.ratio_interval.high.toFixed(3)} | **${c.verdict.replaceAll('_', ' ')}** |`
      )
    lines.push('')
  }
  const diagnostic = report.comparisons.filter(c => c.verdict === 'diagnostic')
  if (diagnostic.length > 0) {
    lines.push(`Diagnostic only, with no interval and no verdict: ${diagnostic[0].reason}.`, '')
    lines.push('| metric | statistic | baseline median | candidate median | ratio |', '|---|---|---|---|---|')
    for (const c of diagnostic.slice(0, 12))
      lines.push(`| ${c.metric} | ${c.statistic} | ${show(c.baseline.median)} | ${show(c.candidate.median)} | ${c.ratio?.toFixed(3) ?? 'n/a'} |`)
    if (diagnostic.length > 12) lines.push(`| … ${diagnostic.length - 12} more in the report | | | | |`)
    lines.push('')
  }
  return lines
}

/** The regressions in a report, which only block a change once the baseline has the runs to set a threshold from. */
export const regressions = report => report.comparisons.filter(c => c.verdict === 'regression')
