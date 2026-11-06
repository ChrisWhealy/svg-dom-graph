// The seeded generator of the scenario `mixed_outage`, and its frozen parameter set.
//
// Every draw comes from one of six streams of the seed (`derive(seed, stream)`), so that changing how one thing is drawn cannot
// move another. All draws are integers. The nth generated arrival takes its draws from each stream in turn, in arrival order,
// and the outages are all drawn before the run starts, since they do not depend on it.
import { derive } from './splitmix.mjs'

export const STREAM = { gap: 1, priority: 2, duration: 3, slack: 4, cancel: 5, outage: 6 }

/**
 * The parameter set, frozen after seven candidates (`dess/s0/evidence/candidates/`). Changing any figure is a new revision of the
 * generator, and the qualification panel is run again. The first three candidates had too little load, or no pull that kept the
 * queue from wandering, and the seventh is the first that qualified.
 */
export const MIXED_OUTAGE = {
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

export const configOf = p => ({
  servers: p.servers,
  queueCapacity: p.queueCapacity,
  maxAttempts: p.maxAttempts,
  deferralCap: p.deferralCap
})

/** The nth arrival's draws, as a source for the engine. Calls must be made in order 0, 1, 2, and so on. */
export const makeSource = (seed, p) => {
  const gap = derive(seed, STREAM.gap)
  const priority = derive(seed, STREAM.priority)
  const duration = derive(seed, STREAM.duration)
  const slack = derive(seed, STREAM.slack)
  const cancel = derive(seed, STREAM.cancel)
  return {
    count: p.arrivals,
    next: () => {
      const spec = {
        gap: gap.between(...p.gap),
        priority: priority.below(100) < p.highPercent ? 1 : 0,
        durations: Array.from({ length: p.maxAttempts }, () => duration.between(...p.duration)),
        slack: slack.between(...p.slack),
        cancelAfter: 0
      }
      if (cancel.below(100) < p.cancelPercent) spec.cancelAfter = cancel.between(...p.cancelAfter)
      return spec
    }
  }
}

/** The scripted outages: a server fails, and recovers a while later, and the next outage comes some time after that. */
export const makeOutages = (seed, p) => {
  const prng = derive(seed, STREAM.outage)
  const horizon = p.arrivals * Math.ceil((p.gap[0] + p.gap[1]) / 2)
  const events = []
  let at = prng.between(...p.outage.first)
  while (at < horizon) {
    const server = prng.below(p.servers)
    const length = prng.between(...p.outage.length)
    events.push({ kind: 'fail', at, server }, { kind: 'recover', at: at + length, server })
    at += length + prng.between(...p.outage.spacing)
  }
  return events
}

export const inputsFor = (seed, p = MIXED_OUTAGE) => ({ script: makeOutages(seed, p), source: makeSource(seed, p) })
