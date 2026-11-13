// Reports on whether the baselines may block a change, and records the decision that one may.
//
//   node perf/runner/qualify.mjs                        every baseline: its runs, its spread, and what it lacks
//   node perf/runner/qualify.mjs --approve <baseline directory> --reason "..." [--accept-noise "..."]
//
// Approving writes `qualification.json` beside the baseline's `promotion.json`, naming its runs, the spread that was seen and why it was
// accepted. It refuses a baseline with fewer than 10 runs, and one whose runs vary by more than `NOISE_LIMIT` unless `--accept-noise` says
// why. It writes nothing else and commits nothing. Promoting a baseline again voids its approval, since the runs are then others.
// See perf/docs/qualification.md for what the whole qualification asks for and where each part stands.
import fs from 'node:fs'
import path from 'node:path'
import { RUNS_TO_BLOCK, baselinesDir, readPromotion, root } from '../lib/baselines.mjs'
import {
  NOISE_LIMIT,
  isQualified,
  qualificationFile,
  readQualification,
  shortfalls,
  spreadOf
} from '../lib/qualification.mjs'

/** The hours that a soak has to run, with every endurance gate passing, before endurance counts as qualified. */
const ENDURANCE_HOURS = 3

const baselineDirs = () => {
  const found = []
  const walk = dir => {
    if (!fs.existsSync(dir)) return
    if (fs.existsSync(path.join(dir, 'promotion.json'))) return found.push(dir)
    for (const entry of fs.readdirSync(dir, { withFileTypes: true }))
      if (entry.isDirectory()) walk(path.join(dir, entry.name))
  }
  walk(baselinesDir)
  return found.sort()
}

const argument = name => {
  const at = process.argv.indexOf(name)
  return at < 0 ? null : process.argv[at + 1]
}

if (process.argv.includes('--approve')) {
  const dir = path.resolve(argument('--approve') ?? '')
  const reason = argument('--reason')
  const noise = argument('--accept-noise')
  if (!reason) throw new Error('--reason is required: say why this baseline is the one to block against')
  const promotion = readPromotion(dir)
  if (!promotion) throw new Error(`${dir} is not a baseline: it has no promotion.json`)
  const spread = spreadOf(dir)
  const lacking = shortfalls(dir, spread).filter(text => !(noise && text.includes('vary by more than')))
  if (lacking.length > 0) {
    console.error(`not approved: ${lacking.join('; ')}`)
    process.exit(1)
  }
  const record = {
    schema_version: 1,
    approved_at: new Date().toISOString(),
    reason,
    accepted_noise: noise ?? null,
    runs_to_block: RUNS_TO_BLOCK,
    runs: promotion.runs,
    spread: spread.map(({ name, layer, unit, runs, median, spread: ratio, coefficient_of_variation }) => ({
      name,
      layer,
      unit,
      runs,
      median,
      highest_over_lowest: ratio,
      coefficient_of_variation
    }))
  }
  fs.writeFileSync(qualificationFile(dir), JSON.stringify(record, null, 2) + '\n')
  console.log(
    `approved ${path.relative(baselinesDir, dir)}: it blocks from the next comparison. Review and commit ${path.relative(process.cwd(), qualificationFile(dir))}.`
  )
  process.exit(0)
}

/** The detection record in perf/evidence/detection/ that shows the comparison working against exactly this baseline, or null. */
const detectionRecordFor = (dir, promotion) => {
  const folder = path.join(root, 'perf/evidence/detection')
  if (!fs.existsSync(folder)) return null
  for (const name of fs.readdirSync(folder).filter(n => n.endsWith('.json'))) {
    const record = JSON.parse(fs.readFileSync(path.join(folder, name), 'utf8'))
    if (
      record.outcome === 'holds' &&
      record.baseline?.dir === path.relative(baselinesDir, dir) &&
      JSON.stringify(record.baseline.runs) === JSON.stringify(promotion.runs)
    )
      return name
  }
  return null
}

const dirs = baselineDirs()
if (dirs.length === 0) console.log('no baselines are promoted yet')
let ready = 0
for (const dir of dirs) {
  const promotion = readPromotion(dir)
  const spread = spreadOf(dir)
  const lacking = shortfalls(dir, spread)
  const state = isQualified(dir)
    ? 'APPROVED, blocks'
    : readQualification(dir)
      ? 'approval void: the runs changed'
      : lacking.length === 0
        ? 'ready to approve'
        : 'reports only'
  if (state === 'ready to approve') ready++
  console.log(`\n${path.relative(baselinesDir, dir)}\n  ${promotion.runs.length} runs, ${state}`)
  if (isQualified(dir)) {
    const proof = detectionRecordFor(dir, promotion)
    console.log(
      proof
        ? `  detection: proven by ${proof}`
        : '  detection: no record for these runs (perf-detection.yml, then collect-detection.mjs)'
    )
  }
  for (const text of lacking) console.log(`  lacks: ${text}`)
  for (const m of spread.slice(0, 3))
    console.log(
      `  spread ${m.spread.toFixed(2)} (cv ${(m.coefficient_of_variation * 100).toFixed(1)}%) ${m.layer} ${m.name}${m.limited ? ' (resolution limited)' : ''}`
    )
}
// Endurance: soaks kept as evidence by perf-endurance.yml, each an `endurance-record.json`.
const soaks = path.join(root, 'perf/evidence/endurance')
const records = fs.existsSync(soaks)
  ? fs
      .readdirSync(soaks)
      .filter(name => name.endsWith('.json'))
      .map(name => JSON.parse(fs.readFileSync(path.join(soaks, name), 'utf8')))
  : []
console.log('\nEndurance')
if (records.length === 0)
  console.log(
    `  no soak is recorded: run the perf-endurance workflow and keep each endurance-record.json in perf/evidence/endurance/`
  )
for (const r of records)
  console.log(
    `  ${r.manifest}: ${r.hours_run} hours, ${r.measurement_iterations} cycles${r.capped_by_memory ? ' (capped by memory)' : ''}, ${r.passed ? 'no growth' : 'FAILED'} (${r.host?.cpu ?? 'unknown processor'})`
  )
const longest = Math.max(0, ...records.filter(r => r.passed).map(r => r.hours_run))
console.log(
  `  ${longest >= ENDURANCE_HOURS ? 'qualified' : 'not qualified'}: the longest soak with no growth is ${longest} hours, and ${ENDURANCE_HOURS} are asked for`
)

console.log(
  `\n${dirs.filter(isQualified).length} of ${dirs.length} baselines block, ${ready} more are ready to approve (limit ${NOISE_LIMIT}, ${RUNS_TO_BLOCK} runs)`
)
