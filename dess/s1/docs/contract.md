# The S1 contract

Two engines implement stage S1: the Rust crate `service-sim-engine`, and a JavaScript oracle in `dess/reference-js/`.
Design revision 6 fixes most of the rules, and leaves some open.
This page fixes the rest, once, before either engine was coded, so that neither can set a rule by accident.
Where the design is silent, the choice made here is a binding rule, and every one is listed in section 1.

## 1. Rules the design leaves open

| Rule | Binding choice |
|---|---|
| R1 Insertion sequence | A number from one counter, allocated when an event is created. Scripted events are created first, in the order they are authored. Then the generator's scripted outages, in the order drawn. Then the first generated arrival |
| R2 Creation order inside one pop | On a generated arrival: the next generated arrival, then the job's deadline, then its cancel. On a failed attempt: the retry eligibility. On a deferred retry: the next retry. After the pops of a slice, dispatch creates the completions in server order |
| R3 Deadline | A scripted arrival gives an absolute tick. A generated one gives a slack, and the deadline is the admission tick plus the slack. The deadline has to be later than the admission tick |
| R4 Queue entry tick | The tick at which the job was admitted, or at which a retry put it back in the queue. A deferral that finds the queue full leaves it unchanged |
| R5 Attempt duration | The nth attempt takes the nth entry of the job's durations, and the last entry if there are fewer |
| R6 Tokens | A job's token starts at 0. It increases by one when a running attempt is aborted by a failed server, and also when the job reaches a terminal state. A completion or a retry eligibility carries the token the job had when it was created, and is valid only if the job is still in the matching state with that token. The increase on reaching a terminal state cannot be observed, since a terminal job fails the state check of every late event and is not in a checkpoint. Both engines keep it, so that the rule reads as the design does |
| R7 Job identity | The next number, from 0, allocated when an arrival pops, including an arrival that is rejected. A key is bound to its number then |
| R8 Dispatch order | Waiting jobs by priority (high first), then queue entry tick, then job number. Idle servers by index. Dispatch runs once, after the pops of a slice, and only if the slice had at least one pop |
| R9 Pop outcome | Effective, stale or no-op, as in `dess/s0/docs/pop-outcomes.md` |
| R10 Run end | The run ends when the future heap is empty |
| R11 Same-tick creation | No event is created for the tick being processed. A retry deferral, a duration and a backoff are at least one tick, and a generated gap is at least one tick |
| R12 Limits | A configuration is validated before anything is allocated, with checked arithmetic. A limit that is exceeded is an error and not a truncation |

## 2. Inputs

A scenario is a configuration and some inputs.

The configuration has the number of servers (1 to 4), the queue capacity (1 to 64), the maximum attempts (1 to 3) and the retry deferral cap (0 to 16).
One server is allowed for the probes E1-RC and E2-RT.
The product supports 2 to 4.

The inputs are a script, a generator, or both.
A script event is one of four kinds.

| Kind | Fields |
|---|---|
| `arrive` | `at`, `key`, `priority` (0 or 1), `durations`, `deadline` (absolute) |
| `cancel` | `at`, `key` |
| `fail` | `at`, `server` |
| `recover` | `at`, `server` |

A generator makes the arrivals and cancels of a seed, and the outages, as `dess/s0/docs/generator-and-qualification.md` describes.
Its six streams are `derive(seed, 1)` to `derive(seed, 6)`.
The scripted outages are all drawn before the run, and the arrivals one at a time.

## 3. The event-pop hash and the input-stream hash

These are the S0 formats, so that the S0 pins stay valid.

The event-pop hash is the SHA-256 of the text of every pop in order.
A pop is `tick:kind:seq:outcome;`, where the kind is `complete`, `fail`, `cancel`, `deadline`, `recover`, `retry` or `arrive`, and the outcome is `e`, `s` or `n`.

The input-stream hash is the SHA-256 of the generator's output for a seed: one line `kind,at,server` for each outage event, then one line `arrive,index,gap,priority,durations,slack,cancelAfter` for each arrival, joined by newlines.
The durations are joined by `+`.

The parameter hash is the SHA-256 of the parameters as canonical JSON, with every key sorted and no spaces.

## 4. The canonical state

The model checkpoint after a slice is the SHA-256 of this byte string.
Integers are little-endian and unsigned.
A string is a `u32` length and then its UTF-8 bytes.
A list is a `u32` count and then its items.

| Field | Encoding |
|---|---|
| Version | `u8` 1 |
| Configuration id | A string |
| Seed | `u64` |
| Tick | `u64`, the tick of the slice, or 0 for the initial state |
| Batch index | `u64`, the slice number, or 0 for the initial state |
| Generator | `u8` 0 if there is none. Otherwise 1, then the count of arrivals generated, and the states of the gap, priority, duration, slack and cancel streams, each `u64` |
| Counters | Twelve `u64`: pops, stale, no-ops, slices, arrivals, admitted, rejected, completed, timed out, cancelled, failed by attempts, failed by deferrals |
| Live jobs | A list, by job number. Each is the number `u64`, the state `u8` (1 queued, 2 running, 3 waiting to retry), the key, the priority `u8`, the attempt `u32`, the failures `u32`, the token `u64`, the deferrals `u32`, the queue entry tick `u64`, the server `u64` if running, and the durations as a list of `u64` |
| Servers | A list in index order. Each is a state `u8` (0 idle, 1 busy, 2 down), and for a busy server the job number `u64` and the due tick `u64` |
| Waiting | A list of job numbers in dispatch order |
| Future events | A list in order of tick, rank and sequence. Each is the tick `u64`, the rank `u8`, the sequence `u64`, a kind `u8`, and the payload |
| Next sequence | `u64` |

The payload of a future event is as follows.

| Kind | Payload |
|---|---|
| 0 complete | the job number `u64`, the token `u64` |
| 1 fail | the server `u64` |
| 2 cancel | the key |
| 3 deadline | the job number `u64` |
| 4 recover | the server `u64` |
| 5 retry | the job number `u64`, the token `u64` |
| 6 arrive | the key, a generated flag `u8`, the priority `u8`, the durations, the deadline or slack `u64`, the cancel delay `u64` |

For an arrival, the `u64` is the absolute deadline if the flag is 0, and the slack if it is 1.
The cancel delay is 0 for a scripted arrival and for a generated one without a cancel.

Terminal jobs are not in the state.
They are in the counters, and every terminal state is compared at the end of a run.

## 5. The differential panel

The panel is a set of seeds, and each seed derives its own scenario from `derive(seed, 7)`, so that the two engines also test their agreement on the derivation.
The parameters are drawn in this order, each with `between(lo, hi)`.

| Draw | Range |
|---|---|
| Servers | 2 to 4 |
| Queue capacity | 1 to 16 |
| Maximum attempts | 1 to 3 |
| Retry deferral cap | 0 to 6 |
| Arrivals | 0 to 200 |
| Gap | 1 to a draw of 1 to 6 |
| High priority percent | 0 to 100 |
| Duration | a draw of 1 to 4, to a draw of that to 12 |
| Slack | a draw of 1 to 10, to a draw of that to 60 |
| Cancel percent | 0 to 20 |
| Cancel after | 1 to a draw of 1 to 40 |
| First outage | a draw of 1 to 30, to a draw of that to 200 |
| Outage length | a draw of 1 to 5, to a draw of that to 30 |
| Outage pause | a draw of 1 to 20, to a draw of that to 100 |

The revision is `panel/1`, and the configuration id is `panel/1:` and the seed.
The parameters hash is the hash of the canonical JSON of these parameters, with the same keys as `mixed_outage/7`.

### The disturbed panel

The first panel never reaches several rules.
One is a failure of a server that is already down, and another is a recovery of one that is up.
The others are an arrival that collides with a generated one, and a cancel of a key that never arrives.
Generated outages do not overlap, and generated cancels follow their own arrivals.
So a second panel, `panel/2`, runs the same derived scenarios with a script added.

The script is drawn from `derive(seed, 8)` after the parameters of the first panel are known, in this order.

| Draw | Range |
|---|---|
| Scripted arrivals | 0 to 10 |
| Scripted events | 0 to 40 |

Each scripted arrival draws, in order, its tick (1 to `2 * arrivals + 300`) and its priority (0 or 1).
It then draws one duration for each attempt, in the panel's duration range, and its deadline as its tick plus a draw of 1 to 60.
Its key is `S` and its number among the scripted arrivals.
Each scripted event draws a kind (0 fail, 1 recover, 2 cancel) and then its tick (1 to `2 * arrivals + 300`).
A failure or a recovery then draws a server (0 to servers minus 1), and a cancel draws a key.
The key of a cancel is `g` and a draw of 0 to `arrivals + 5` with probability one half, and otherwise `S` and a draw of 0 to 12.
Some of these keys are never created, and some are created later than the cancel.

The scripted arrivals are authored first and the scripted events after them, and the script is created before the generator's outages (rule R1).
The configuration id is `panel/2:` and the seed, and the revision is `panel/2`.

## 6. What is compared

For every seed, the two engines produce the same trace.

- The parameters, input-stream and event-pop hashes.
- Every pop: its tick, kind, sequence, outcome and subject.
- Every slice: its number, tick, pop count, waiting count, busy count and checkpoint hash.
- The initial checkpoint hash.
- The final counters, the final state of every job, and the final state of every server.

The first difference is the discrepancy, and it is recorded in the ledger with its cause.
