// Runs the measurements of the resource risks in headless Chrome and writes one record file for each.
//
//   node spmv/measure.mjs [--only NAME]
//
// The measurements are the ignored tests of `spmv-scene/tests/measure.rs`. Each prints records on lines that begin `SPMV-REPORT `,
// and this keeps them in `spmv/evidence/NAME.jsonl`. They need Chrome, as the tests do, and a run takes about a minute. The figures
// are those of the browser and machine in each file's first record, and a timing here is a measurement and never a gate.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

const root = path.resolve(import.meta.dirname, '..')
const only = process.argv.includes('--only') ? process.argv[process.argv.indexOf('--only') + 1] : null
const NAMES = [
  'sizes',
  'degree',
  'label_lengths',
  'cells',
  'rebuilds',
  'limits_nodes',
  'limits_hub_short_labels',
  'limits_hub_medium_labels',
  'limits_hub_long_labels'
]

fs.mkdirSync(path.join(root, 'spmv/evidence'), { recursive: true })
let failed = 0
for (const name of NAMES.filter(n => !only || n === only)) {
  const run = spawnSync(
    'wasm-pack',
    [
      'test',
      '--headless',
      '--chrome',
      'spmv-scene',
      '--test',
      'measure',
      '--',
      '--nocapture',
      '--include-ignored',
      name
    ],
    { cwd: root, encoding: 'utf8', maxBuffer: 1 << 28, timeout: 600_000 }
  )
  const records = `${run.stdout}${run.stderr}`
    .split('\n')
    .filter(line => line.includes('SPMV-REPORT '))
    .map(line => line.slice(line.indexOf('SPMV-REPORT ') + 'SPMV-REPORT '.length))
  const ok = run.status === 0 && records.length > 0
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}: ${records.length} records`)
  if (!ok) {
    failed++
    continue
  }
  fs.writeFileSync(path.join(root, 'spmv/evidence', `${name}.jsonl`), `${records.join('\n')}\n`)
}
process.exit(failed ? 1 : 0)
