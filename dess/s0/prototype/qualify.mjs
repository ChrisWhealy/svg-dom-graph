// The S0 seed-qualification gate (design revision 6, section 8.3).
//
//   node dess/s0/prototype/qualify.mjs [--candidate NAME] [--out FILE] [--params FILE]
//
// Runs the generator on exactly the 500 predetermined seeds 0 to 499, with the frozen parameter set, and applies the activity and
// stability bounds of `preflight.mjs` to the measured window of each (slices 297 to 536). At least 475 of the 500 have to qualify,
// and the manifest seed (20261106) has to qualify as well, since that is the trace that gets timed. The result lists every seed with
// its reasons for refusal, and the distributions of the figures, whether the panel passed or not. If it does not pass, the
// generator's parameters are changed and the whole panel is run again; the bounds are never changed to keep a seed.
import fs from 'node:fs'
import { run } from './engine.mjs'
import { MIXED_OUTAGE, configOf, inputsFor } from './generator.mjs'
import { BOUNDS, MIN_POPS, MIN_SLICES, WINDOW, evaluate } from './preflight.mjs'

const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const params = argument('--params')
  ? { ...MIXED_OUTAGE, ...JSON.parse(fs.readFileSync(argument('--params'), 'utf8')) }
  : MIXED_OUTAGE
const SEEDS = Array.from({ length: 500 }, (_, i) => i)
const MANIFEST_SEED = 20261106
const REQUIRED = 475

const trace = seed => {
  const slices = []
  const result = run(configOf(params), inputsFor(seed, params), {
    onSlice: s => {
      if (s.index <= MIN_SLICES) slices.push({ index: s.index, pops: s.pops, waiting: s.waiting, busy: s.busy })
    }
  })
  return {
    ...evaluate(slices, params.queueCapacity, result.counters.pops),
    pops: result.counters.pops,
    slices: result.counters.slices,
    rejected: result.counters.rejected,
    arrivals: result.counters.arrivals
  }
}

const rows = SEEDS.map(seed => ({ seed, ...trace(seed) }))
const manifest = { seed: MANIFEST_SEED, ...trace(MANIFEST_SEED) }
const qualified = rows.filter(r => r.qualified).length

const quantiles = values => {
  const v = [...values].sort((a, b) => a - b)
  const at = q => v[Math.min(v.length - 1, Math.floor(q * v.length))]
  return { min: v[0], p5: at(0.05), median: at(0.5), p95: at(0.95), max: v[v.length - 1] }
}
const measured = rows.filter(r => r.metrics)
const distribution = Object.fromEntries(
  Object.keys(measured[0]?.metrics ?? {}).map(k => [k, quantiles(measured.map(r => r.metrics[k]))])
)
const reasonCounts = {}
for (const r of rows)
  for (const reason of r.reasons) {
    const key = reason.replace(/[0-9.]+/g, 'N')
    reasonCounts[key] = (reasonCounts[key] ?? 0) + 1
  }

const record = {
  candidate: argument('--candidate') ?? params.revision,
  parameters: params,
  bounds: BOUNDS,
  window: WINDOW,
  minimums: { slices: MIN_SLICES, pops: MIN_POPS },
  panel: { seeds: '0 to 499', qualified, required: REQUIRED, passed: qualified >= REQUIRED },
  manifest_seed: {
    seed: MANIFEST_SEED,
    qualified: manifest.qualified,
    reasons: manifest.reasons,
    metrics: manifest.metrics,
    pops: manifest.pops,
    slices: manifest.slices
  },
  refusal_reasons_by_kind: reasonCounts,
  distributions: distribution,
  pops: quantiles(rows.map(r => r.pops)),
  rejected_arrivals: quantiles(rows.map(r => r.rejected)),
  seeds: rows.map(r => ({
    seed: r.seed,
    qualified: r.qualified,
    reasons: r.reasons,
    metrics: r.metrics,
    pops: r.pops,
    slices: r.slices
  }))
}
if (argument('--out')) fs.writeFileSync(argument('--out'), JSON.stringify(record, null, 2) + '\n')

const f = x => (typeof x === 'number' ? x.toFixed(3) : x)
console.log(
  `${record.candidate}: ${qualified} of 500 seeds qualify (${REQUIRED} are needed): ${record.panel.passed ? 'PASS' : 'FAIL'}`
)
console.log(
  `manifest seed ${MANIFEST_SEED}: ${manifest.qualified ? 'qualifies' : `REFUSED: ${manifest.reasons.join('; ')}`}`
)
console.log('refusals by kind:', JSON.stringify(reasonCounts))
for (const [k, d] of Object.entries(distribution))
  console.log(
    `  ${k.padEnd(26)} min ${f(d.min)}  p5 ${f(d.p5)}  median ${f(d.median)}  p95 ${f(d.p95)}  max ${f(d.max)}`
  )
console.log(
  `  pops: median ${record.pops.median}, min ${record.pops.min}; rejected arrivals: median ${record.rejected_arrivals.median}`
)
process.exit(record.panel.passed && manifest.qualified ? 0 : 1)
