// How much of the rule space the differential panel exercises: for each rule, in how many seeds it happens at all.
//
//   node dess/s1/coverage.mjs [--seeds 600]
//
// A panel that agrees proves nothing about a rule that it never reaches. This counts, over the oracle's traces of the panel, the
// seeds in which each counter is above zero, and the seeds that reach the rarer paths of the design: a retry that is deferred
// because the queue is full, a job that fails by its deferrals, a second attempt, a cancel or a timeout while waiting to retry,
// and a rejection while a server is idle.
import { run } from '../reference-js/oracle.mjs'
import { disturbances, panelParams } from '../reference-js/generator.mjs'

const N = Number(process.argv.includes('--seeds') ? process.argv[process.argv.indexOf('--seeds') + 1] : 600)
const KIND = ['complete', 'fail', 'cancel', 'deadline', 'recover', 'retry', 'arrive']
const seeds = {}
let empty = 0
let bump = () => {}
const panels = [
  [
    'panel',
    seed => {
      const p = panelParams(seed)
      return { configId: `panel/1:${seed}`, seed, config: p, params: p }
    }
  ],
  [
    'disturbed panel',
    seed => {
      const p = { ...panelParams(seed), revision: 'panel/2' }
      return { configId: `panel/2:${seed}`, seed, config: p, params: p, script: disturbances(seed, p) }
    }
  ]
]
for (const [panelName, scenarioOf] of panels) {
  bump = name => {
    const key = `${panelName}: ${name}`
    seeds[key] = (seeds[key] ?? 0) + 1
  }
  for (let seed = 0; seed < N; seed++) {
    const t = run(scenarioOf(seed))
    if (t.counters.pops === 0) empty++
    for (const [name, v] of Object.entries(t.counters))
      if (v > 0 && !['pops', 'slices'].includes(name)) bump(`counter ${name} > 0`)
    const retries = new Map()
    let deferred = 0
    for (const [, kind, , outcome, subject] of t.pops) {
      if (KIND[kind] === 'retry' && outcome === 0) retries.set(subject, (retries.get(subject) ?? 0) + 1)
    }
    for (const n of retries.values()) if (n > 1) deferred++
    if (deferred > 0) bump('a retry deferred at least once')
    if (t.jobs.some(([, , s]) => s === 'failed(retry_deferrals_exhausted)')) bump('a job failed by its deferrals')
    if (t.jobs.some(([, , s]) => s === 'failed(attempts_exhausted)')) bump('a job failed by its attempts')
    const stalePerKind = new Set(t.pops.filter(q => q[3] === 1).map(q => KIND[q[1]]))
    for (const k of stalePerKind) bump(`a stale ${k}`)
    const noopPerKind = new Set(t.pops.filter(q => q[3] === 2).map(q => KIND[q[1]]))
    for (const k of noopPerKind) bump(`a no-op ${k}`)
  }
}
console.log(`${N} seeds in each panel, ${empty} with no pops at all`)
for (const [name, count] of Object.entries(seeds).sort((a, b) => a[0].localeCompare(b[0])))
  console.log(`  ${name.padEnd(40)} ${String(count).padStart(4)} seeds`)
