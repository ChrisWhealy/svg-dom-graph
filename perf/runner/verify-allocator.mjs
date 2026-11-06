// Verifies the counting allocator in a real browser, and says whether its figures can be used.
//
//   node perf/runner/verify-allocator.mjs [--out FILE]
//
// The design does not trust the allocator until it has counted a deliberately retained allocation, a release and a leaked
// closure, each repeated many times. The checks themselves are in `perf-harness-wasm/src/allocator_check`. This runs them
// in Chrome and prints each one. It exits with 1 if any fails.
import fs from 'node:fs'
import { openHarnessPage } from './lib/harness-page.mjs'

const out = process.argv.includes('--out') ? process.argv[process.argv.indexOf('--out') + 1] : null
const page = await openHarnessPage()
const result = JSON.parse(await page.evaluate('window.perfHarness.verify_allocator()'))
await page.close()

for (const step of result.steps) {
  console.log(`${step.pass ? 'ok  ' : 'FAIL'} ${step.name}: ${step.expectation}`)
  if (!step.pass) console.log(`     observed ${JSON.stringify(step.observed)}`)
}
console.log(
  `\n${result.pass ? 'the allocator is verified' : 'the allocator is NOT verified: its figures must not be used'}`
)
if (out) fs.writeFileSync(out, JSON.stringify(result, null, 2) + '\n')
process.exit(result.pass ? 0 : 1)
