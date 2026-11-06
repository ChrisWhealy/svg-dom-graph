// Checks that the fault contracts are well formed, that every source they cite exists, and that the recovery manifests expect
// what they say. A contract that cites a line that has gone is a contract nobody can check, so it fails here.
//
//   node perf/lib/test-contracts.mjs
import fs from 'node:fs'
import path from 'node:path'
import { createRequire } from 'node:module'

const root = path.resolve(import.meta.dirname, '../..')
const { parse } = createRequire(path.join(root, 'perf/schemas/package.json'))('smol-toml')
const contracts = JSON.parse(fs.readFileSync(path.join(root, 'perf/fault-contracts.json'), 'utf8'))
const schema = JSON.parse(fs.readFileSync(path.join(root, 'perf/schemas/manifest.schema.json'), 'utf8'))
const ids = schema.properties.recovery.properties.points.items.properties.point.enum

let failures = 0

const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

check(
  JSON.stringify(contracts.points.map(p => p.id).sort()) === JSON.stringify([...ids].sort()),
  `the contracts name the fault points of the manifest schema (${ids.length})`
)

for (const point of contracts.points) {
  const documented = point.contract !== 'undocumented'
  check(
    documented === (point.documented !== 'none'),
    `${point.id}: it is called documented exactly when its contract is not "undocumented" (${point.contract}, ${point.documented})`
  )
  check(!documented || point.sources.length > 0, `${point.id}: a documented contract cites where`)
  check(
    point.contract !== 'partly_committed' || (point.retry && point.differs?.length > 0),
    `${point.id}: a partly committed contract names the sections that differ and the retry`
  )
  for (const source of point.sources) {
    const [file, range] = source.split(':')
    const [from, to] = range.split('-').map(Number)
    const lines = fs.existsSync(path.join(root, file))
      ? fs.readFileSync(path.join(root, file), 'utf8').split('\n').length
      : 0
    check(lines >= to && from >= 1 && from <= to, `${point.id}: ${source} is in a file of ${lines} lines`)
  }
}
for (const name of fs.readdirSync(path.join(root, 'perf/manifests')).filter(name => name.startsWith('w5-'))) {
  const manifest = parse(fs.readFileSync(path.join(root, 'perf/manifests', name), 'utf8'))
  for (const { point, expect } of manifest.recovery.points) {
    check(contracts.points.find(p => p.id === point)?.contract === expect, `${name}: ${point} expects ${expect}`)
  }
}
console.log(
  failures ? `\n${failures} did not hold` : '\nthe contracts are well formed, cited, and what the manifests expect'
)
process.exit(failures ? 1 : 0)
