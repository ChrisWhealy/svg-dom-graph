// Whether a baseline may block a change, and what its runs say about the spread of the runner that made them.
//
// A baseline blocks only when two things are true: it has the runs to block with (`RUNS_TO_BLOCK`, one commit, one environment), and a person
// has looked at the spread of those runs and approved it, which `perf/runner/qualify.mjs --approve` records in `qualification.json` beside
// `promotion.json`. Neither is enough alone. The runs say how much the runner varies by itself, and approving is the decision that this
// is little enough for a regression to be told from it. Nothing here sets a threshold: the comparison's own interval comes from the same runs.
import fs from 'node:fs'
import path from 'node:path'
import { RUNS_TO_BLOCK, readPromotion, resultFiles } from './baselines.mjs'

/**
 * The widest spread between the repetition medians of a baseline that is still approved without `--accept-noise`. It is a guard against
 * approving a baseline from a runner that was too busy to measure anything, and not a threshold for a regression: a ratio of 2 means that
 * the slowest run took twice as long as the fastest, which no timing verdict could be trusted through. It is provisional, and is to be revisited
 * once there are baselines of 10 runs to see how wide an ordinary runner's spread is.
 */
export const NOISE_LIMIT = 2

export const qualificationFile = dir => path.join(dir, 'qualification.json')
export const readQualification = dir => {
  const file = qualificationFile(dir)
  return fs.existsSync(file) ? JSON.parse(fs.readFileSync(file, 'utf8')) : null
}

const quantile = (sorted, q) => sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))]

/** For each timing of a baseline's runs, the median of each run and how far apart those medians are. */
export const spreadOf = dir => {
  const byMetric = new Map()
  for (const file of resultFiles(dir)) {
    const doc = JSON.parse(fs.readFileSync(file, 'utf8'))
    if (doc.mode !== 'performance') continue
    for (const metric of doc.metrics) {
      if (metric.status !== 'measured' || !['latency', 'batch_throughput'].includes(metric.kind)) continue
      const key = `${metric.layer} ${metric.name}`
      if (!byMetric.has(key))
        byMetric.set(key, { name: metric.name, layer: metric.layer, unit: metric.unit, medians: [], limited: false })
      const entry = byMetric.get(key)
      entry.medians.push(metric.summary.p50)
      entry.limited ||= metric.resolution_limited === true
    }
  }

  return [...byMetric.values()]
    .map(({ medians, ...rest }) => {
      const sorted = [...medians].sort((a, b) => a - b)
      const mean = sorted.reduce((a, b) => a + b, 0) / sorted.length
      const deviation = Math.sqrt(sorted.reduce((a, b) => a + (b - mean) ** 2, 0) / Math.max(1, sorted.length - 1))
      return {
        ...rest,
        runs: sorted.length,
        median: quantile(sorted, 0.5),
        // Highest over lowest, as the harness's own reports give the spread of repetitions.
        spread: sorted[0] > 0 ? sorted[sorted.length - 1] / sorted[0] : Infinity,
        coefficient_of_variation: mean > 0 ? deviation / mean : Infinity
      }
    })
    .sort((a, b) => b.spread - a.spread)
}

/** What a baseline lacks before it may be approved. Empty if nothing. */
export const shortfalls = (dir, spread = spreadOf(dir)) => {
  const promotion = readPromotion(dir)
  if (!promotion) return ['it has no promotion.json, so it is not a baseline']
  const found = []
  if (promotion.runs.length < RUNS_TO_BLOCK)
    found.push(`it has ${promotion.runs.length} runs and approving needs ${RUNS_TO_BLOCK}`)
  const noisy = spread.filter(m => !m.limited && m.spread > NOISE_LIMIT)
  if (noisy.length > 0)
    found.push(
      `${noisy.length} timing(s) vary by more than ${NOISE_LIMIT} between runs, the worst being ${noisy[0].name} at ${noisy[0].spread.toFixed(2)}`
    )
  return found
}

/** Whether the baseline in `dir` is approved, for the runs that it has now. Promoting over it again voids the approval. */
export const isQualified = dir => {
  const promotion = readPromotion(dir)
  const record = readQualification(dir)
  if (!promotion || !record) return false
  return (
    record.schema_version === 1 &&
    promotion.runs.length >= RUNS_TO_BLOCK &&
    JSON.stringify(record.runs) === JSON.stringify(promotion.runs)
  )
}
