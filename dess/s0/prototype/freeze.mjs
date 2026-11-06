// Computes the pins of the current revision, and writes them with `--write`.
//
//   node dess/s0/prototype/freeze.mjs [--write]
//
// Run it after a deliberate change to the parameters or the engine, with a new `revision` in `generator.mjs`, and after the
// qualification panel has passed for that revision. Without `--write` it only prints, and says whether the pins on file still match.
import fs from 'node:fs'
import { FROZEN_FILE, checkPins, computePins, readFrozen } from './pin.mjs'
import { MIXED_OUTAGE } from './generator.mjs'

const MANIFEST_SEED = 20261106
const pins = computePins(MIXED_OUTAGE, MANIFEST_SEED)
console.log(JSON.stringify(pins, null, 2))
if (process.argv.includes('--write')) {
  fs.writeFileSync(FROZEN_FILE, JSON.stringify(pins, null, 2) + '\n')
  console.log(`\nwritten to ${FROZEN_FILE.pathname}`)
} else {
  const check = fs.existsSync(FROZEN_FILE)
    ? checkPins(MIXED_OUTAGE, readFrozen())
    : { ok: false, problems: ['there is no frozen.json'] }
  console.log(check.ok ? '\nthe pins on file match' : `\nthe pins on file do not match: ${check.problems.join('; ')}`)
}
