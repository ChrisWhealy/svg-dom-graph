// Loads, revisits and walks every panel of the built demo gallery in a real browser, and writes a report.
//
//   node perf/runner/gallery-sweep.mjs [--stage DIR] [--out DIR] [--only panel-a,panel-b]
//
// The gallery is built by `cargo run -p demo-server -- --build-only`, which stages it in `target/demo-stage`. The report is
// checked against `perf/schemas/gallery-report.schema.json` before it is written. The sweep fails if a panel does not come up
// when it is selected, or if the page throws, and it never fails on a time.
import fs from 'node:fs'
import path from 'node:path'
import { createRequire } from 'node:module'
import { spawnSync } from 'node:child_process'
import { sweepGallery } from './layer-c/gallery.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const argument = name => {
  const at = process.argv.indexOf(name)
  return at > 0 ? process.argv[at + 1] : null
}
const stageDir = path.resolve(argument('--stage') ?? path.join(root, 'target/demo-stage'))
const outDir = path.resolve(argument('--out') ?? path.join(root, 'target/perf/gallery'))
const only = argument('--only')?.split(',') ?? null
const overrides = JSON.parse(fs.readFileSync(path.join(root, 'perf/gallery-overrides.json'), 'utf8')).panels

const requireFromSchemas = createRequire(path.join(root, 'perf/schemas/package.json'))
const Ajv = requireFromSchemas('ajv/dist/2020.js')
const ajv = new Ajv({ strict: true, strictTypes: false, strictRequired: false, allErrors: true })
const validate = ajv.compile(JSON.parse(fs.readFileSync(path.join(root, 'perf/schemas/gallery-report.schema.json'), 'utf8')))

const first = (command, args) => spawnSync(command, args, { cwd: root, encoding: 'utf8' }).stdout?.trim().split('\n')[0] ?? null
const report = await sweepGallery({ stageDir, overrides, only, log: line => console.log(line) })
report.git_sha = first('git', ['rev-parse', 'HEAD'])
report.git_dirty = spawnSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }).stdout.trim().length > 0

if (!validate(report)) {
  console.error(`the report does not satisfy its schema: ${ajv.errorsText(validate.errors)}`)
  process.exit(1)
}
fs.mkdirSync(outDir, { recursive: true })
const file = path.join(outDir, `gallery-report-${new Date().toISOString().replace(/[-:.]/g, '').slice(0, 15)}.json`)
fs.writeFileSync(file, JSON.stringify(report, null, 2) + '\n')

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const summary = report.summary
const median = values => (values.length ? [...values].sort((a, b) => a - b)[values.length >> 1] : null)
const panels = report.panels
const built = panels.filter(panel => panel.build.ms !== undefined).map(panel => panel.build.ms)
const revisited = panels.filter(panel => panel.revisit?.ms !== undefined).map(panel => panel.revisit.ms)
console.log(`\n${panels.length} panels in ${path.relative(root, file)}`)
console.log(`  cold start ${report.cold_start_ms.toFixed(0)} ms`)
console.log(`  first visit: ${built.length} timed, median ${median(built)?.toFixed(1)} ms, slowest ${Math.max(...built).toFixed(1)} ms (${panels.find(panel => panel.build.ms === Math.max(...built))?.id})`)
console.log(`  revisit: ${revisited.length} timed, median ${median(revisited)?.toFixed(1)} ms`)
console.log(`  swept ${summary.panels} of the ${summary.panels_in_gallery} panels in the gallery: ${summary.loaded} loaded, ${summary.failed_to_load} failed to load, ${summary.with_errors} with page errors`)
console.log(`  walked ${summary.walked}, downgraded ${summary.downgraded}, skipped ${summary.skipped}`)

let failed = 0
const gate = (ok, text) => {
  if (!ok) failed++
  console.log(`  ${ok ? 'ok  ' : 'FAIL'} ${text}`)
}
gate(summary.panels === summary.panels_in_gallery || only !== null, `the whole gallery was swept (${summary.panels} of ${summary.panels_in_gallery})`)
gate(summary.failed_to_load === 0, 'every panel came up when it was selected, on the first visit and on the revisit')
gate(report.page_errors.length === 0, `the page threw nothing and logged no error (${report.page_errors.length})`)
console.log(failed ? `\n${failed} deterministic gate(s) failed` : '\nevery deterministic gate passed')
process.exit(failed ? 1 : 0)
