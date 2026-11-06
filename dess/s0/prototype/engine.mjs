// The S0 prototype of the engine, written from section 3 of the design (revision 6) and from nothing else. It exists to measure
// what S0 has to measure (the activity of a generated trace, and what replay costs), and to be a third reading of the rules
// beside the Rust engine and the JavaScript oracle that S1 produces. It is not the S1 engine.
//
// An event is (tick, rank, sequence, payload) in a min-heap. Every event at the smallest tick is popped, in rank order, and then
// jobs are dispatched once. A pop that finds its job or attempt already gone is stale, and is still a pop.

export const RANK = { complete: 0, fail: 1, cancel: 2, deadline: 3, recover: 4, retry: 5, arrive: 6 }
const ACTIVE = new Set(['queued', 'running', 'retrywait'])

class Heap {
  constructor() {
    this.items = []
  }
  get size() {
    return this.items.length
  }
  static less(a, b) {
    return a.tick - b.tick || a.rank - b.rank || a.seq - b.seq
  }
  push(item) {
    const items = this.items
    let i = items.length
    items.push(item)
    while (i > 0) {
      const parent = (i - 1) >> 1
      if (Heap.less(items[i], items[parent]) >= 0) break
      ;[items[i], items[parent]] = [items[parent], items[i]]
      i = parent
    }
  }
  peek() {
    return this.items[0]
  }
  pop() {
    const items = this.items
    const top = items[0]
    const last = items.pop()
    if (items.length > 0) {
      items[0] = last
      let i = 0
      for (;;) {
        const l = 2 * i + 1
        const r = l + 1
        let m = i
        if (l < items.length && Heap.less(items[l], items[m]) < 0) m = l
        if (r < items.length && Heap.less(items[r], items[m]) < 0) m = r
        if (m === i) break
        ;[items[i], items[m]] = [items[m], items[i]]
        i = m
      }
    }
    return top
  }
}

/**
 * Runs a scenario to its end and reports every timestamp slice.
 *
 * `config`: servers, queueCapacity, maxAttempts, deferralCap.
 * `inputs.script`: events that are known before the run (fail, recover, cancel by key, and fixed arrivals), preloaded in order.
 * `inputs.source`: for a generated run, `next(index)` gives `{ gap, priority, durations, slack, cancelAfter }` for the nth arrival,
 *   and the engine schedules exactly one next arrival when the previous generated one pops.
 * `options.onSlice(slice)`: called after each slice with the figures that the activity preflight needs.
 * `options.stopAfterSlices`: stops once this many slices are done, for a replay to a cursor.
 * `options.hash`: an object with `update(text)` and `digest('hex')`, such as `crypto.createHash('sha256')`, to hash the pop sequence.
 *   It is passed in so that this file loads in a browser as well as in Node.
 */
export const run = (config, inputs, options = {}) => {
  const { servers, queueCapacity, maxAttempts, deferralCap } = config
  const heap = new Heap()
  let seq = 0
  const push = (tick, kind, payload) => heap.push({ tick, kind, rank: RANK[kind], seq: seq++, payload })
  const jobs = []
  const byKey = new Map()
  const srv = Array.from({ length: servers }, () => ({ state: 'idle', job: null }))
  const waiting = []
  const hash = options.hash ?? null
  const counters = {
    pops: 0,
    stale: 0,
    slices: 0,
    arrivals: 0,
    admitted: 0,
    rejected: 0,
    completed: 0,
    timedOut: 0,
    cancelled: 0,
    failed: 0
  }

  for (const event of inputs.script ?? []) {
    if (event.kind === 'arrive') push(event.at, 'arrive', { key: event.key, spec: event })
    else push(event.at, event.kind, event)
  }
  let generated = 0
  const scheduleGenerated = from => {
    if (!inputs.source || generated >= inputs.source.count) return
    const spec = inputs.source.next(generated)
    push(from + spec.gap, 'arrive', { key: `g${generated}`, spec, generated: true })
    generated++
  }
  scheduleGenerated(0)

  const finish = (job, state, reason) => {
    job.state = state
    job.reason = reason
    job.token++
    if (state === 'completed') counters.completed++
    else if (state === 'timedout') counters.timedOut++
    else if (state === 'cancelled') counters.cancelled++
    else if (state === 'failed') counters.failed++
  }
  const leave = job => {
    if (job.state === 'queued') waiting.splice(waiting.indexOf(job), 1)
    else if (job.state === 'running') {
      srv[job.server].state = 'idle'
      srv[job.server].job = null
    }
  }

  let sliceCount = 0
  while (heap.size > 0) {
    const tick = heap.peek().tick
    let pops = 0
    while (heap.size > 0 && heap.peek().tick === tick) {
      const ev = heap.pop()
      pops++
      counters.pops++
      let stale = false
      const p = ev.payload
      switch (ev.kind) {
        case 'arrive': {
          const job = {
            id: jobs.length,
            key: p.key,
            priority: p.spec.priority ?? 0,
            state: null,
            attempt: 0,
            failures: 0,
            token: 0,
            defers: 0,
            durations: p.spec.durations ?? [p.spec.duration],
            entered: 0,
            server: -1,
            reason: null
          }
          jobs.push(job)
          byKey.set(p.key, job)
          counters.arrivals++
          if (p.generated) scheduleGenerated(tick)
          if (waiting.length < queueCapacity) {
            job.state = 'queued'
            job.entered = tick
            waiting.push(job)
            counters.admitted++
            push(p.spec.deadline ?? tick + p.spec.slack, 'deadline', { job })
            if (p.generated && p.spec.cancelAfter) push(tick + p.spec.cancelAfter, 'cancel', { key: p.key })
          } else {
            job.state = 'rejected'
            counters.rejected++
          }
          break
        }
        case 'complete': {
          const job = p.job
          if (job.state === 'running' && job.token === p.token) {
            srv[job.server].state = 'idle'
            srv[job.server].job = null
            finish(job, 'completed')
          } else stale = true
          break
        }
        case 'deadline': {
          const job = p.job
          if (ACTIVE.has(job.state)) {
            leave(job)
            finish(job, 'timedout')
          } else stale = true
          break
        }
        case 'cancel': {
          const job = byKey.get(p.key)
          if (job && ACTIVE.has(job.state)) {
            leave(job)
            finish(job, 'cancelled')
          }
          break
        }
        case 'fail': {
          const sv = srv[p.server]
          if (sv.state === 'busy') {
            const job = sv.job
            sv.state = 'down'
            sv.job = null
            job.token++
            job.failures++
            if (job.failures < maxAttempts) {
              job.state = 'retrywait'
              push(tick + 2 * 2 ** (job.failures - 1), 'retry', { job, token: job.token })
            } else finish(job, 'failed', 'attempts_exhausted')
          } else if (sv.state === 'idle') sv.state = 'down'
          break
        }
        case 'recover': {
          if (srv[p.server].state === 'down') srv[p.server].state = 'idle'
          break
        }
        case 'retry': {
          const job = p.job
          if (job.state === 'retrywait' && job.token === p.token) {
            if (waiting.length < queueCapacity) {
              job.state = 'queued'
              job.entered = tick
              job.defers = 0
              waiting.push(job)
            } else if (++job.defers > deferralCap) finish(job, 'failed', 'retry_deferrals_exhausted')
            else push(tick + 1, 'retry', { job, token: job.token })
          } else stale = true
          break
        }
      }
      if (stale) counters.stale++
      if (hash) hash.update(`${tick}:${ev.kind}:${ev.seq}:${stale ? 1 : 0};`)
    }
    // Dispatch, once: high priority first, then the tick the job entered the queue, then its id. Idle servers by index.
    waiting.sort((a, b) => b.priority - a.priority || a.entered - b.entered || a.id - b.id)
    for (const [index, sv] of srv.entries()) {
      if (sv.state === 'idle' && waiting.length > 0) {
        const job = waiting.shift()
        job.attempt++
        sv.state = 'busy'
        sv.job = job
        job.server = index
        job.state = 'running'
        const duration = job.durations[job.attempt - 1] ?? job.durations[job.durations.length - 1]
        push(tick + duration, 'complete', { job, token: job.token })
      }
    }
    sliceCount++
    counters.slices = sliceCount
    options.onSlice?.({
      index: sliceCount,
      tick,
      pops,
      waiting: waiting.length,
      busy: srv.filter(s => s.state === 'busy').length,
      states: options.wantStates ? jobs.map(j => j.state) : null
    })
    if (options.stopAfterSlices && sliceCount >= options.stopAfterSlices) break
  }
  return {
    counters,
    hash: hash?.digest('hex') ?? null,
    states: Object.fromEntries(jobs.map(j => [j.key, j.state + (j.reason ? `(${j.reason})` : '')])),
    servers: srv.map(s => s.state)
  }
}
