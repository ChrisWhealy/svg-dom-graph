// The known-answer fixtures of the design, as inputs to the prototype engine. Priority 1 is high and 0 is low.
// `durations` is what each attempt takes, and the last one is used again if there are more attempts than entries.
export const E0 = {
  config: { servers: 2, queueCapacity: 3, maxAttempts: 2, deferralCap: 4 },
  inputs: {
    script: [
      { kind: 'arrive', at: 0, key: 'J0', priority: 1, durations: [4, 2], deadline: 10 },
      { kind: 'arrive', at: 0, key: 'J1', priority: 0, durations: [6], deadline: 20 },
      { kind: 'arrive', at: 1, key: 'J2', priority: 1, durations: [2], deadline: 8 },
      { kind: 'arrive', at: 2, key: 'J3', priority: 0, durations: [2], deadline: 5 },
      { kind: 'arrive', at: 4, key: 'J4', priority: 1, durations: [1], deadline: 9 },
      { kind: 'arrive', at: 5, key: 'J5', priority: 0, durations: [3], deadline: 15 },
      { kind: 'fail', at: 2, server: 0 },
      { kind: 'recover', at: 7, server: 0 },
      { kind: 'cancel', at: 6, key: 'J5' }
    ]
  },
  expect: {
    pops: 19,
    slices: 12,
    snapshots: 13,
    stale: 5,
    arrivals: 6,
    admitted: 5,
    rejected: 1,
    states: { J0: 'completed', J1: 'completed', J2: 'completed', J3: 'timedout', J4: 'rejected', J5: 'cancelled' }
  }
}

// One server, capacity two, two attempts. J0 starts at tick 0 for 5 ticks, the server fails at tick 1, and the retry is
// eligible at tick 3. It is cancelled at tick 2, while it waits to retry.
export const E1_RC = {
  config: { servers: 1, queueCapacity: 2, maxAttempts: 2, deferralCap: 4 },
  inputs: {
    script: [
      { kind: 'arrive', at: 0, key: 'J0', priority: 1, durations: [5], deadline: 20 },
      { kind: 'fail', at: 1, server: 0 },
      { kind: 'cancel', at: 2, key: 'J0' }
    ]
  },
  expect: { pops: 6, slices: 6, snapshots: 7, stale: 3, states: { J0: 'cancelled' }, servers: ['down'] }
}

// The same, with a deadline at tick 2 and no cancel: the job times out while it waits to retry.
export const E2_RT = {
  config: { servers: 1, queueCapacity: 2, maxAttempts: 2, deferralCap: 4 },
  inputs: {
    script: [
      { kind: 'arrive', at: 0, key: 'J0', priority: 1, durations: [5], deadline: 2 },
      { kind: 'fail', at: 1, server: 0 }
    ]
  },
  expect: { pops: 5, slices: 5, snapshots: 6, stale: 2, states: { J0: 'timedout' }, servers: ['down'] }
}

export const FIXTURES = { E0, 'E1-RC': E1_RC, 'E2-RT': E2_RT }
