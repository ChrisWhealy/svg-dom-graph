// Checks that the measured counts that the simulator's Layer C runner builds are ones the result schema accepts, and that a time cannot
// be recorded as a count. The Play series runs only on the nightly profile, so a count in the wrong unit would otherwise be found there.
//
//   node perf/runner/test-service-metrics.mjs
import fs from 'node:fs'
import { createRequire } from 'node:module'
import path from 'node:path'
import { COUNT_UNITS, count } from './layer-c/service.mjs'

const schemas = path.resolve(import.meta.dirname, '../schemas')
const require = createRequire(path.join(schemas, 'check.mjs'))
const Ajv = require('ajv/dist/2020.js').default
const schema = JSON.parse(fs.readFileSync(path.join(schemas, 'result.schema.json'), 'utf8'))
const validate = new Ajv({ strict: false }).compile({
  $schema: 'https://json-schema.org/draft/2020-12/schema',
  ...schema.properties.metrics.items
})

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

for (const unit of ['ms', 'us', 'count', 'bytes']) {
  const metric = { name: 'x', layer: 'C', status: 'measured', unit, kind: 'count', value: 1, mechanism: 'm' }
  check(
    validate(metric) === COUNT_UNITS.includes(unit),
    `the schema and the runner agree that a count in ${unit} is ${COUNT_UNITS.includes(unit) ? 'allowed' : 'refused'}`
  )
}
check(validate(count('simulation.example', 'count', 3, 'an example')), 'a count the runner builds is valid')
let refused = false
try {
  count('simulation.play.interval', 'ms', 1000, 'a time is not a count')
} catch {
  refused = true
}
check(refused, 'a count in milliseconds is refused when it is built')
process.exit(failures ? 1 : 0)
