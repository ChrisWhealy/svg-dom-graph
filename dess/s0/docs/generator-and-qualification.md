# The generator and its qualification

Design revision 6 requires a seed-qualification gate before any W6 timing.
The generator runs on 500 predetermined seeds with a frozen parameter set.
At least 475 of them must qualify, and the seed in the manifest must qualify as well.
S0 defined the generator, froze its parameters, and ran the gate.

## The generator

The scenario is `mixed_outage`, and its code is `dess/s0/prototype/generator.mjs`.
The nth arrival takes its draws from six streams of the seed.
A stream is `derive(seed, n)`, which is the harness's own derivation from `perf-harness/src/prng`.
A test pins the PRNG to the published vectors that the Rust tests use.
Every draw is an integer, and changing how one thing is drawn cannot move another.

| Stream | Draws |
|---|---|
| 1 | The gap before the next arrival, in ticks |
| 2 | Whether the job is high priority |
| 3 | The duration of each attempt, one draw for each of `max_attempts` |
| 4 | The deadline slack after admission |
| 5 | Whether the job is cancelled, and after how long |
| 6 | The scripted outages, all drawn before the run |

A generated arrival schedules the next one when it pops, as the design says.
Its cancel, if it has one, is scheduled when the arrival is admitted.
An outage is a failure of one server, and a recovery some ticks later, and the next outage follows after a pause.

### The frozen parameters

The revision is `mixed_outage/7`.
Changing any figure is a new revision, and the whole panel is run again.

| Parameter | Value |
|---|---|
| Servers, queue capacity, max attempts, retry deferral cap | 3, 12, 2, 4 |
| Arrivals | 4,000 |
| Gap between arrivals | 1 to 2 ticks |
| High priority | 30% |
| Duration of an attempt | 2 to 12 ticks |
| Deadline slack | 6 to 30 ticks |
| Cancelled | 4%, after 1 to 40 ticks |
| First outage at | Tick 100 to 200 |
| Outage length, and pause before the next | 5 to 30 ticks, and 80 to 240 ticks |

## The bounds

The bounds are the ones in the design, and are frozen in `dess/s0/prototype/preflight.mjs`.
They apply to the measured window, which is slices 297 to 536 after 256 priming slices and 40 warm-up slices.

| Test | Bound |
|---|---|
| Mean occupancy, which is the waiting count divided by the capacity after dispatch | 0.25 to 0.90 |
| Slices with a nonempty queue | At least 50% |
| Mean pops a slice, stale pops included | 1.5 to 32 |
| Slices with a busy server | At least 50% |
| The two halves of 120 slices, mean occupancy | Differ by at most 0.20 |
| The two halves, mean pops a slice | Differ by at most 25% of the whole |

A trace also needs at least 536 nonempty slices and 10,000 pops.

## The candidates

The first candidate failed, and the design says to change the generator's parameters and run the whole panel again.
It also says never to change the bounds to keep a seed.
Seven candidates were run, and every result is kept in `dess/s0/evidence/candidates/`, the failures included.

| Candidate | Change | Qualified of 500 | Manifest seed | Why the others failed |
|---|---|---|---|---|
| 1 | The first guess: gap 1 to 4, slack 12 to 60, durations 1 to 12 | 1 | Refused | Median occupancy 0.093, so far too little load |
| 2 | Gap 1 to 3 | 340 | Qualifies | The halves differ too much in 124 seeds, and occupancy is low in 48 |
| 3 | Shorter and rarer outages | 309 | Qualifies | Worse, so the outages were not the cause |
| 4 | Slack 6 to 30 | 10 | Refused | Median occupancy 0.157, since the queue is too short |
| 5 | Durations 2 to 12 | 78 | Qualifies | Median occupancy 0.199 |
| 6 | Slack 4 to 20 | 0 | Refused | Median occupancy 0.099 |
| 7 | Gap 1 to 2, slack 6 to 30, durations 2 to 12 | **490** | **Qualifies** | The halves differ too much in 10 seeds |

Candidate 3 taught the useful thing.
At a utilisation near 1 the queue is a reflected random walk, and nothing pulls it back.
The halves of the window then differ by chance.
Short deadlines give the pull, since a waiting job abandons at a rate that grows with the queue.
They give enough only when the load is high enough to keep a queue.

## The result

490 of 500 seeds qualify, against the 475 needed.
The refused seeds are 85, 108, 123, 152, 218, 226, 312, 338, 363 and 474, all on the half-window occupancy test.

The manifest seed 20261106 qualifies, and well inside the bounds.
Its mean occupancy is 0.497, its queue is nonempty in all slices, and it has 2.06 pops a slice.
Its halves differ by 0.039 in occupancy and 6.5% in pops a slice.

| Figure over the 500 seeds | Minimum | Median | Maximum |
|---|---|---|---|
| Mean occupancy of the measured window | 0.332 | 0.483 | 0.645 |
| Pops a slice | 1.93 | 2.07 | 2.23 |
| Pops in the whole trace | 11,159 | 11,255 | 11,369 |
| Arrivals rejected because the queue was full | 0 | 3 | 14 |

Every trace is above the 10,000-pop minimum, and a server is busy in every slice of the median seed.
The queue is rarely full, so the trace exercises rejection a little and timeouts and cancellations a lot.

## Reproducing it

```sh
node dess/s0/prototype/test-prototype.mjs
node dess/s0/prototype/qualify.mjs
node dess/s0/prototype/qualify.mjs --params dess/s0/evidence/candidates/mixed_outage-3.params.json
```

The first checks the prototype against the design's known answers.
The second runs the panel, and exits with 1 if fewer than 475 seeds qualify or the manifest seed does not.
The third reruns a recorded candidate, and the candidate files hold the full parameter set.
`dess/s0/evidence/qualification.json` is the record of the frozen revision.

## What this shows and does not show

- The figures are for the prototype engine, which was written from the design and passes E0, E1-RC and E2-RT.
  It is not the S1 engine.
- S1's Rust engine has to reproduce this generator draw for draw.
  The streams, the order and the parameters above are the contract.
- The qualification says the window is busy and steady enough to time.
  It does not say the trace is representative of anything.
