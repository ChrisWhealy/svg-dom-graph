// The generator of a seed, written for the oracle from `dess/s0/docs/generator-and-qualification.md` and the contract.
import { stream } from './prng.mjs'
import { sha256 } from './canonical.mjs'

/** The streams of a seed. The first five make the arrivals one at a time, and the sixth is drawn once for the outages. */
export class Generator {
  constructor(seed, params) {
    this.params = params
    this.gap = stream(seed, 1)
    this.priority = stream(seed, 2)
    this.duration = stream(seed, 3)
    this.slack = stream(seed, 4)
    this.cancel = stream(seed, 5)
    this.count = params.arrivals
    this.made = 0
  }

  /** The draws of the next arrival, in the order of the contract. */
  nextArrival() {
    const p = this.params
    const gap = this.gap.between(p.gap[0], p.gap[1])
    const priority = this.priority.below(100) < p.highPercent ? 1 : 0
    const durations = []
    for (let i = 0; i < p.maxAttempts; i++) durations.push(this.duration.between(p.duration[0], p.duration[1]))
    const slack = this.slack.between(p.slack[0], p.slack[1])
    let cancelAfter = 0
    if (this.cancel.below(100) < p.cancelPercent) cancelAfter = this.cancel.between(p.cancelAfter[0], p.cancelAfter[1])
    this.made++
    return { gap, priority, durations, slack, cancelAfter }
  }

  states() {
    return [this.gap, this.priority, this.duration, this.slack, this.cancel].map(s => s.state)
  }
}

/** The scripted outages of a seed: a failure, a recovery some ticks later, and a pause before the next. */
export const outages = (seed, params) => {
  const s = stream(seed, 6)
  const horizon = params.arrivals * Math.ceil((params.gap[0] + params.gap[1]) / 2)
  const events = []
  let at = s.between(params.outage.first[0], params.outage.first[1])
  while (at < horizon) {
    const server = s.below(params.servers)
    const length = s.between(params.outage.length[0], params.outage.length[1])
    events.push({ kind: 'fail', at, server }, { kind: 'recover', at: at + length, server })
    at += length + s.between(params.outage.spacing[0], params.outage.spacing[1])
  }
  return events
}

/** The text of everything the generator produces for a seed, and its hash. */
export const inputStreamHash = (seed, params) => {
  const lines = outages(seed, params).map(e => `${e.kind},${e.at},${e.server}`)
  const g = new Generator(seed, params)
  for (let i = 0; i < g.count; i++) {
    const a = g.nextArrival()
    lines.push(`arrive,${i},${a.gap},${a.priority},${a.durations.join('+')},${a.slack},${a.cancelAfter}`)
  }
  return sha256(lines.join('\n'))
}

/** The scenario of a seed in the differential panel, derived from stream 7 in the order the contract gives. */
export const panelParams = seed => {
  const s = stream(seed, 7)
  const b = (lo, hi) => s.between(lo, hi)
  const servers = b(2, 4)
  const queueCapacity = b(1, 16)
  const maxAttempts = b(1, 3)
  const deferralCap = b(0, 6)
  const arrivals = b(0, 200)
  const gap = [1, b(1, 6)]
  const highPercent = b(0, 100)
  const durLo = b(1, 4)
  const duration = [durLo, b(durLo, 12)]
  const slackLo = b(1, 10)
  const slack = [slackLo, b(slackLo, 60)]
  const cancelPercent = b(0, 20)
  const cancelAfter = [1, b(1, 40)]
  const firstLo = b(1, 30)
  const first = [firstLo, b(firstLo, 200)]
  const lenLo = b(1, 5)
  const length = [lenLo, b(lenLo, 30)]
  const spLo = b(1, 20)
  const spacing = [spLo, b(spLo, 100)]
  return {
    revision: 'panel/1',
    servers,
    queueCapacity,
    maxAttempts,
    deferralCap,
    arrivals,
    gap,
    highPercent,
    duration,
    slack,
    cancelPercent,
    cancelAfter,
    outage: { first, length, spacing }
  }
}

/** The parameters of the frozen S0 revision. */
export const MIXED_OUTAGE_7 = {
  revision: 'mixed_outage/7',
  servers: 3,
  queueCapacity: 12,
  maxAttempts: 2,
  deferralCap: 4,
  arrivals: 4000,
  gap: [1, 2],
  highPercent: 30,
  duration: [2, 12],
  slack: [6, 30],
  cancelPercent: 4,
  cancelAfter: [1, 40],
  outage: { first: [100, 200], length: [5, 30], spacing: [80, 240] }
}

/**
 * The script of a disturbed panel seed (`panel/2`): scripted arrivals, and failures, recoveries and cancels at random ticks, drawn
 * from stream 8 in the order of the contract.
 */
export const disturbances = (seed, params) => {
  const s = stream(seed, 8)
  const horizon = 2 * params.arrivals + 300
  const script = []
  const arrivals = s.between(0, 10)
  for (let i = 0; i < arrivals; i++) {
    const at = s.between(1, horizon)
    const priority = s.below(2)
    const durations = []
    for (let a = 0; a < params.maxAttempts; a++) durations.push(s.between(params.duration[0], params.duration[1]))
    const deadline = at + s.between(1, 60)
    script.push({ kind: 'arrive', at, key: `S${i}`, priority, durations, deadline })
  }
  const events = s.between(0, 40)
  for (let i = 0; i < events; i++) {
    const kind = s.below(3)
    const at = s.between(1, horizon)
    if (kind < 2) script.push({ kind: kind === 0 ? 'fail' : 'recover', at, server: s.below(params.servers) })
    else {
      const generated = s.below(2) === 0
      script.push({
        kind: 'cancel',
        at,
        key: generated ? `g${s.between(0, params.arrivals + 5)}` : `S${s.between(0, 12)}`
      })
    }
  }
  return script
}
