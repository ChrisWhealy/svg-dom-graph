// How much of the rule space the differential panel exercises: for each rule, in how many seeds it happens at all.
//
//   node dess/s1/coverage.mjs [--seeds 600]
//
// A panel that agrees proves nothing about a rule that it never reaches. Two kinds of figure are given, and they are measured in
// different ways.
//
//   From the trace: the seeds in which each counter is above zero, a retry that is deferred at least once, a job that fails by its
//   deferrals or by its attempts, and a stale or no-op pop of each kind. These read the counters, the pop outcomes and the final
//   states of the jobs, and say nothing of the state before a pop.
//
//   From the state before each pop ("transition:"): the oracle's `observe` hook gives the state that a pop acts on, and these count
//   a cancel or a deadline that reaches a job in each of its states, a failure of a server in each of its states, and an arrival
//   that is rejected while a server is idle. A figure of this kind is the pairs of an event and a state that were reached.
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
    const reached = new Set()
    const observe = (s, ev) => {
      if (ev.kind === 'cancel' || ev.kind === 'deadline') {
        const job = ev.kind === 'cancel' ? s.byKey.get(ev.payload.key) : s.jobs[ev.payload.job]
        reached.add(`${ev.kind} reaches ${job ? (job.state ?? 'a job not yet placed') : 'a key with no job'}`)
      } else if (ev.kind === 'fail') {
        reached.add(`fail reaches a server that is ${s.servers[ev.payload.server].state}`)
      } else if (ev.kind === 'arrive' && s.waiting.length >= s.config.queueCapacity) {
        reached.add(
          s.servers.some(sv => sv.state === 'idle')
            ? 'an arrival is rejected while a server is idle'
            : 'an arrival is rejected while no server is idle'
        )
      }
    }
    const t = run(scenarioOf(seed), { observe })
    for (const name of reached) bump(`transition: ${name}`)
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
