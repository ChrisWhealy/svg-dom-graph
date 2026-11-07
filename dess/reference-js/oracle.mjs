// The oracle: a second engine for the discrete-event service simulator, written from design revision 6 and the S1 contract
// (`dess/s1/docs/contract.md`) and from nothing else. It is deliberately plain, so that it is easy to read against the design:
//
//   * the future events are one array, kept sorted by (tick, rank, sequence), and a slice is the prefix at the smallest tick;
//   * each kind of event has a function of its own, which is the transition that the design's rank table describes;
//   * nothing is shared with the Rust engine, and nothing is shared with the S0 prototype.
//
// It takes a scenario and gives a trace: every pop, every slice with its checkpoint hash, and the final state.
import { sha256, Writer, canonicalJson } from './canonical.mjs'
import { Generator, inputStreamHash, outages } from './generator.mjs'

const RANK = { complete: 0, fail: 1, cancel: 2, deadline: 3, recover: 4, retry: 5, arrive: 6 }
const KIND_CODE = { complete: 0, fail: 1, cancel: 2, deadline: 3, recover: 4, retry: 5, arrive: 6 }
const STATE_CODE = { queued: 1, running: 2, retrywait: 3 }
const COUNTERS = [
  'pops',
  'stale',
  'noops',
  'slices',
  'arrivals',
  'admitted',
  'rejected',
  'completed',
  'timedOut',
  'cancelled',
  'failedAttempts',
  'failedDeferrals'
]
const ACTIVE = job => job.state === 'queued' || job.state === 'running' || job.state === 'retrywait'

const compare = (a, b) => a.tick - b.tick || a.rank - b.rank || a.seq - b.seq

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** An error with a kind, which the differential run compares with the Rust engine's. */
export class ScenarioError extends Error {
  constructor(kind, message) {
    super(message)
    this.kind = kind
  }
}

/** The keys of the script and the generator have to name one job each, and are checked before anything runs (rule R13). */
const validateKeys = (script, params) => {
  const seen = new Set()
  for (const e of script) {
    if (e.kind !== 'arrive') continue
    if (seen.has(e.key)) throw new ScenarioError('invalid', `the arrival key ${e.key} is used more than once`)
    seen.add(e.key)
    const generated = /^g(0|[1-9][0-9]*)$/.exec(e.key)
    if (params && generated && Number(generated[1]) < params.arrivals)
      throw new ScenarioError('invalid', `the arrival key ${e.key} is one that the generator uses`)
  }
}

const validate = config => {
  const inRange = (name, v, lo, hi) => {
    if (!Number.isInteger(v) || v < lo || v > hi)
      throw new ScenarioError('invalid', `${name} ${v} is outside ${lo} to ${hi}`)
  }
  inRange('servers', config.servers, 1, 4)
  inRange('queue capacity', config.queueCapacity, 1, 64)
  inRange('maximum attempts', config.maxAttempts, 1, 3)
  inRange('retry deferral cap', config.deferralCap, 0, 16)
}

/** Creates an event, and puts it where it belongs in the sorted array. The sequence is the next number of the counter. */
const create = (s, tick, kind, payload) => {
  if (!Number.isSafeInteger(tick)) throw new Error('a tick overflowed')
  const event = { tick, rank: RANK[kind], seq: s.nextSeq++, kind, payload }
  let lo = 0
  let hi = s.events.length
  while (lo < hi) {
    const mid = (lo + hi) >> 1
    if (compare(s.events[mid], event) < 0) lo = mid + 1
    else hi = mid
  }
  s.events.splice(lo, 0, event)
}

const createGeneratedArrival = (s, from) => {
  if (!s.generator || s.generator.made >= s.generator.count) return
  const index = s.generator.made
  const a = s.generator.nextArrival()
  create(s, from + a.gap, 'arrive', {
    key: `g${index}`,
    generated: true,
    priority: a.priority,
    durations: a.durations,
    deadlineOrSlack: a.slack,
    cancelDelay: a.cancelAfter
  })
}

const finish = (s, job, state, reason, counter) => {
  job.state = state
  job.reason = reason ?? null
  job.token += 1
  s.counters[counter] += 1
}

const leave = (s, job) => {
  if (job.state === 'queued') s.waiting.splice(s.waiting.indexOf(job), 1)
  else if (job.state === 'running') {
    s.servers[job.server] = { state: 'idle' }
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// One function for each kind of event. Each gives the outcome of the pop ('e', 's' or 'n') and the subject that the trace records.
const transitions = {
  arrive(s, ev) {
    const p = ev.payload
    if (s.byKey.has(p.key)) throw new ScenarioError('invalid', `the arrival key ${p.key} is already bound to a job`)
    const job = {
      id: s.jobs.length,
      key: p.key,
      priority: p.priority,
      durations: p.durations,
      state: null,
      reason: null,
      attempt: 0,
      failures: 0,
      token: 0,
      defers: 0,
      entered: 0,
      server: -1
    }
    s.jobs.push(job)
    s.byKey.set(p.key, job)
    s.counters.arrivals += 1
    if (p.generated) createGeneratedArrival(s, ev.tick)
    if (s.waiting.length < s.config.queueCapacity) {
      job.state = 'queued'
      job.entered = ev.tick
      s.waiting.push(job)
      s.counters.admitted += 1
      const deadline = p.generated ? ev.tick + p.deadlineOrSlack : p.deadlineOrSlack
      if (deadline <= ev.tick) throw new ScenarioError('invalid', `the deadline of ${p.key} is not after its admission`)
      create(s, deadline, 'deadline', { job: job.id })
      if (p.generated && p.cancelDelay > 0) create(s, ev.tick + p.cancelDelay, 'cancel', { key: p.key })
    } else {
      job.state = 'rejected'
      s.counters.rejected += 1
    }
    return ['e', job.id]
  },

  complete(s, ev) {
    const job = s.jobs[ev.payload.job]
    if (job.state === 'running' && job.token === ev.payload.token) {
      s.servers[job.server] = { state: 'idle' }
      finish(s, job, 'completed', null, 'completed')
      return ['e', job.id]
    }
    return ['s', job.id]
  },

  deadline(s, ev) {
    const job = s.jobs[ev.payload.job]
    if (ACTIVE(job)) {
      leave(s, job)
      finish(s, job, 'timedout', null, 'timedOut')
      return ['e', job.id]
    }
    return ['s', job.id]
  },

  cancel(s, ev) {
    const job = s.byKey.get(ev.payload.key)
    if (job && ACTIVE(job)) {
      leave(s, job)
      finish(s, job, 'cancelled', null, 'cancelled')
      return ['e', job.id]
    }
    return ['n', job ? job.id : -1]
  },

  fail(s, ev) {
    const index = ev.payload.server
    if (index >= s.servers.length) throw new ScenarioError('invalid', `server ${index} does not exist`)
    const server = s.servers[index]
    if (server.state === 'busy') {
      const job = server.job
      s.servers[index] = { state: 'down' }
      job.token += 1
      job.failures += 1
      if (job.failures < s.config.maxAttempts) {
        job.state = 'retrywait'
        create(s, ev.tick + 2 * 2 ** (job.failures - 1), 'retry', { job: job.id, token: job.token })
      } else finish(s, job, 'failed', 'attempts_exhausted', 'failedAttempts')
      return ['e', index]
    }
    if (server.state === 'idle') {
      s.servers[index] = { state: 'down' }
      return ['e', index]
    }
    return ['n', index]
  },

  recover(s, ev) {
    const index = ev.payload.server
    if (index >= s.servers.length) throw new ScenarioError('invalid', `server ${index} does not exist`)
    if (s.servers[index].state === 'down') {
      s.servers[index] = { state: 'idle' }
      return ['e', index]
    }
    return ['n', index]
  },

  retry(s, ev) {
    const job = s.jobs[ev.payload.job]
    if (job.state === 'retrywait' && job.token === ev.payload.token) {
      if (s.waiting.length < s.config.queueCapacity) {
        job.state = 'queued'
        job.entered = ev.tick
        job.defers = 0
        s.waiting.push(job)
      } else {
        job.defers += 1
        if (job.defers > s.config.deferralCap) finish(s, job, 'failed', 'retry_deferrals_exhausted', 'failedDeferrals')
        else create(s, ev.tick + 1, 'retry', { job: job.id, token: job.token })
      }
      return ['e', job.id]
    }
    return ['s', job.id]
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** After the pops of a slice: the waiting jobs go to the idle servers, once. */
const dispatch = (s, tick) => {
  s.waiting.sort((a, b) => b.priority - a.priority || a.entered - b.entered || a.id - b.id)
  for (let index = 0; index < s.servers.length; index++) {
    if (s.servers[index].state !== 'idle' || s.waiting.length === 0) continue
    const job = s.waiting.shift()
    job.attempt += 1
    job.state = 'running'
    job.server = index
    const duration = job.durations[job.attempt - 1] ?? job.durations[job.durations.length - 1]
    if (duration < 1) throw new ScenarioError('invalid', 'a duration is not positive')
    s.servers[index] = { state: 'busy', job, due: tick + duration }
    create(s, tick + duration, 'complete', { job: job.id, token: job.token })
  }
}

const encodePayload = (w, ev) => {
  const p = ev.payload
  switch (ev.kind) {
    case 'complete':
    case 'retry':
      w.u64(p.job).u64(p.token)
      break
    case 'deadline':
      w.u64(p.job)
      break
    case 'fail':
    case 'recover':
      w.u64(p.server)
      break
    case 'cancel':
      w.string(p.key)
      break
    case 'arrive':
      w.string(p.key)
        .u8(p.generated ? 1 : 0)
        .u8(p.priority)
        .u64List(p.durations)
        .u64(p.deadlineOrSlack)
        .u64(p.generated ? p.cancelDelay : 0)
      break
  }
}

/** The canonical encoding of the state, and its hash. `tick` and `batch` are those of the slice just done, or 0 and 0. */
const checkpoint = (s, tick, batch) => {
  const w = new Writer().u8(1).string(s.configId).u64(s.seed).u64(tick).u64(batch)
  if (s.generator) {
    w.u8(1).u64(s.generator.made)
    for (const state of s.generator.states()) w.u64(state)
  } else w.u8(0)
  for (const name of COUNTERS) w.u64(s.counters[name])
  const live = s.jobs.filter(ACTIVE)
  w.u32(live.length)
  for (const j of live) {
    w.u64(j.id)
      .u8(STATE_CODE[j.state])
      .string(j.key)
      .u8(j.priority)
      .u32(j.attempt)
      .u32(j.failures)
      .u64(j.token)
      .u32(j.defers)
      .u64(j.entered)
    if (j.state === 'running') w.u64(j.server)
    w.u64List(j.durations)
  }
  w.u32(s.servers.length)
  for (const sv of s.servers) {
    if (sv.state === 'idle') w.u8(0)
    else if (sv.state === 'busy') w.u8(1).u64(sv.job.id).u64(sv.due)
    else w.u8(2)
  }
  w.u64List(s.waiting.map(j => j.id))
  w.u32(s.events.length)
  for (const ev of s.events) {
    w.u64(ev.tick).u8(ev.rank).u64(ev.seq).u8(KIND_CODE[ev.kind])
    encodePayload(w, ev)
  }
  w.u64(s.nextSeq)
  return w.digest()
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Runs a scenario to the end and gives its trace.
 *
 * `scenario`: `{ configId, seed, config, script, params }`. `params` is present for a generated run, and then `config` is its
 * servers, queue capacity, attempts and deferral cap. `script` is a list of events.
 */
export const run = (scenario, options = {}) => {
  const { configId, seed, config, script = [], params = null } = scenario
  validate(config)
  validateKeys(script, params)
  const s = {
    configId,
    seed,
    config,
    events: [],
    nextSeq: 0,
    jobs: [],
    byKey: new Map(),
    waiting: [],
    servers: Array.from({ length: config.servers }, () => ({ state: 'idle' })),
    counters: Object.fromEntries(COUNTERS.map(n => [n, 0])),
    generator: params ? new Generator(seed, params) : null
  }
  for (const e of script) {
    if (e.kind === 'arrive')
      create(s, e.at, 'arrive', {
        key: e.key,
        generated: false,
        priority: e.priority,
        durations: e.durations,
        deadlineOrSlack: e.deadline,
        cancelDelay: 0
      })
    else if (e.kind === 'cancel') create(s, e.at, 'cancel', { key: e.key })
    else create(s, e.at, e.kind, { server: e.server })
  }
  if (params) {
    for (const e of outages(seed, params)) create(s, e.at, e.kind, { server: e.server })
    createGeneratedArrival(s, 0)
  }

  const popText = []
  const pops = []
  const slices = []
  const initial = checkpoint(s, 0, 0)
  while (s.events.length > 0) {
    const tick = s.events[0].tick
    let count = 0
    while (count < s.events.length && s.events[count].tick === tick) count++
    const batch = s.events.splice(0, count)
    for (const ev of batch) {
      const [outcome, subject] = transitions[ev.kind](s, ev)
      s.counters.pops += 1
      if (outcome === 's') s.counters.stale += 1
      if (outcome === 'n') s.counters.noops += 1
      popText.push(`${ev.tick}:${ev.kind}:${ev.seq}:${outcome};`)
      pops.push([ev.tick, KIND_CODE[ev.kind], ev.seq, outcome === 'e' ? 0 : outcome === 's' ? 1 : 2, subject])
    }
    dispatch(s, tick)
    s.counters.slices += 1
    slices.push({
      index: s.counters.slices,
      tick,
      pops: count,
      waiting: s.waiting.length,
      busy: s.servers.filter(x => x.state === 'busy').length,
      checkpoint: checkpoint(s, tick, s.counters.slices)
    })
    if (options.stopAfterSlices && s.counters.slices >= options.stopAfterSlices) break
  }

  return {
    seed,
    config_id: configId,
    params_sha256: params ? sha256(canonicalJson(params)) : null,
    input_stream_sha256: params ? inputStreamHash(seed, params) : null,
    event_pop_sha256: sha256(popText.join('')),
    initial_checkpoint: initial,
    counters: s.counters,
    slices,
    pops,
    jobs: s.jobs.map(j => [j.id, j.key, j.state + (j.reason ? `(${j.reason})` : '')]),
    servers: s.servers.map(x => x.state)
  }
}
