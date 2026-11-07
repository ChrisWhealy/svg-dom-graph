# The discrepancy ledger

The design asks for a record of every place where the Rust engine and the oracle disagree, and of how each was settled.
This page is that record, and it also records what was found while building the comparison, since an honest ledger says what the comparison could and could not see.
The text of design revision 6 decides every discrepancy, and neither engine is changed to agree with the other until it does.

## Result

| Comparison | Scenarios | Pops | Slice checkpoints | Disagreements |
|---|---|---|---|---|
| Native Rust against the oracle | 1,228 | 399,490 | 234,760 | 0 |
| 32-bit Rust (`wasm32-wasip1`) against the oracle | 1,228 | 399,490 | 234,760 | 0 |

The scenarios are the six known-answer fixtures, the twenty-one refusal cases, 600 seeds of the panel, 600 seeds of the disturbed panel and the frozen revision `mixed_outage/7`.
A disagreement is any difference in a hash, a pop, a slice, a counter or a final state.

The two engines never disagreed on a pop, a slice or a hash, at any point of the build.
That is a statement about what was tested, and it needs three qualifications, which the last section sets out.
Findings 6 and 7 below are defects that both engines had, and that agreement could not show.

## What was found while building the comparison

| # | Found | Kind | Settled by |
|---|---|---|---|
| 1 | The first differential run reported all 606 scenarios as different, on `counters` only | A defect of the comparison, and not of an engine | The comparison read JSON text, and Rust writes an object's keys in sorted order. It now compares with the keys sorted. As `counters` is the last field compared, the pops, the slices and every hash had already agreed in all 606 |
| 2 | The first panel never reached a failure of a server that is down, a recovery of one that is up, a scripted arrival, or a cancel of a key that never arrives | A gap in the coverage | `dess/s1/coverage.mjs` counted the seeds that reach each rule. The disturbed panel `panel/2` was added to the contract, and it reaches the duplicate failure in 464 seeds and the duplicate recovery in 536 |
| 3 | Rule R6 said that a job's token rises when it reaches a terminal state, and no comparison can see that | An unobservable rule | The mutation check kept a mutation of it, and reported it as equivalent. R6 now says so. Both engines keep the increase |
| 4 | The design's "stale/active bit" in the event-pop hash has no value for a no-op | A gap in the design, settled in stage S0 | Three outcomes, as `dess/s0/docs/pop-outcomes.md` records. Revision 7 of the design should say so |
| 5 | The design does not say in what order the events that one pop creates take their sequence numbers | A rule left open | Contract rules R1 and R2, fixed before either engine was coded. The S0 pins, which another implementation produced, confirm them, since they include the sequence numbers |
| 6 | Two arrivals with the same key made the second overwrite the first job's entry, so a later cancel of the key reached the wrong job | A defect that both engines shared, found by review | Rule R13: keys are unique across the script and the generator, and a violation is refused before the run, in the same words by both engines. See below |
| 7 | The engine did not check that the generator's parameters and its own configuration agree, so a generated outage could name a server that does not exist. Scripted events were checked only when they popped | A gap that both engines shared, found by review | Rule R14: the figures must agree, and scripted events are well formed. Both are refused before the run, in the same words by both engines. See below |
| 8 | The coverage report said that it counted a cancel or a timeout while a job waits to retry, and a rejection while a server is idle, and it read only counters, outcomes and final states | A report that claimed more than it measured | The oracle has an `observe` hook that gives the state before each pop, and the report now counts those transitions from it. See below |

## Finding 6: duplicate arrival keys

A review found that the engine bound a key to a job with an insert that replaced any earlier binding.
With J0 arriving as `X` and running, and J1 arriving as `X` and queued, a later `Cancel(X)` cancelled J1, and J0 ran on.
A scripted arrival could also take a key that the generator makes, such as `g0`.
The oracle did the same, so the two engines agreed on the wrong answer, and no comparison could have seen it.
The panels use `S` and a number for scripted keys and `g` and a number for generated ones, so they never collided.

The design says a cancel names a job by its key, and so a key has to name one job.
The text decides it, and rule R13 now says so.

- **Refused before the run.** Two scripted arrivals that share a key, and a scripted key that is `g` and a number below the generator's arrival count with no leading zero.
- **Refused at the arrival pop.** A pop that finds its key already bound is an error and never rebinds it.
  The check before the run refuses first, so this stays in case a scenario arrives some other way.
- **Not refused.** `g5` when the generator makes five arrivals, `g01`, `G0`, and `g0` when there is no generator.

The same refusals are in `dess/s1/error-fixtures.json`, with their messages, and both engines are compared on them.
Fixture E3-KEY is the scenario with distinct keys: the cancel reaches X, and Y runs on.
Two more refusals that the oracle gave in other words, a nonexistent server and a duration that is not positive, were made to match at the same time.

The sensitivity check has a mutation for it.
Removing the check before the run is caught, since the engine then refuses in the words of the guard.
Removing the guard is equivalent, since the check before the run refuses first.

## Finding 7: parameters and configuration, and scripted events

A review found that the engine validated its configuration and the generator's parameters separately, and never compared them.
Parameters that describe three servers and a configuration that allocates two were accepted, and a generated outage could then name a server that does not exist.
The oracle did the same, and the differential scenarios always supplied matching values.
The oracle's own test for a run with no arrivals did it too, and nobody noticed: its configuration had two servers and its parameters had four.

Scripted events had the related gap.
A failure of a missing server, a duration that is zero, and a deadline that is not after the arrival were refused only when the event popped.
A job that was rejected on arrival never reached its deadline check, so a bad deadline on it was never refused at all.

Rule R14 moves all of it to construction, before any event is created.

- **The figures agree.** Servers, queue capacity, maximum attempts and retry deferral cap are compared in that order, and the first difference is the error.
- **A scripted failure or recovery names a server that exists.**
- **A scripted arrival has a duration, none of them zero, and a deadline after its tick.** An arrival with no durations is refused with the zero-duration message.

The checks at the pop stay as a backstop.
Ten new refusal cases cover each figure, each kind of scripted event, a zero duration on a later attempt, and the rejected arrival with the bad deadline.
Two cases show that agreeing figures and the last server are accepted.
The sensitivity check has a mutation for each of the two checks, and both are caught.

## Finding 8: what the coverage report measures

A review found that the report named two conditions that it could not establish.
It counted counters, pop outcomes and final job states, and none of those says what state a job or a server was in just before an event.
Fixtures E1-RC and E2-RT do reach the retry-wait cases, so the checks were covered, and the report was not.

The oracle's `run` now takes an `observe(state, event)` hook, which is called before each pop and changes nothing.
The report is in two parts and says so in its header.
The first part is from the trace, as before.
The second part, marked `transition:`, is from the state before each pop.

| Pair of an event and a state | First panel | Disturbed panel |
|---|---|---|
| A cancel reaches a job that waits to retry | 16 seeds | 18 seeds |
| A deadline reaches a job that waits to retry | 83 seeds | 119 seeds |
| An arrival is rejected while a server is idle | 127 seeds | 175 seeds |
| An arrival is rejected while no server is idle | 110 seeds | 172 seeds |
| A failure reaches a server that is idle, busy or down | 425, 438 and none | 579, 495 and 464 seeds |

The report also gives a cancel and a deadline for a job in each other state.
A pair that no seed reaches is absent from the list, and not shown as zero.
The first panel never fails a server that is down, which is why the disturbed panel exists.

The oracle's trace is unchanged, so the golden hashes and the differential run are unaffected.

## Rules that the design left open

The contract lists R1 to R14.
The ones that could have gone either way are these.

- **R2, creation order inside a pop.** A generated arrival creates the next arrival, then its deadline, then its cancel.
  Nothing in the design fixes it.
  It changes sequence numbers, and so the event-pop hash, and it is never visible in the state.
- **R6, tokens.** The design says that a stale completion is discarded after checking the job, the attempt and a generation.
  The contract uses one token for all three.
- **R8, dispatch.** The design orders waiting jobs by priority, queue entry tick and job number.
  The contract says that a retry's queue entry tick is the tick of its return to the queue, which the design implies and does not state.
- **R11, same-tick events.** No event is created for the tick being processed.
  This makes a slice's events known when it begins, and both engines rely on it.

## Does the comparison see a bug?

A comparison that agrees proves nothing until it is known to notice a disagreement.
`dess/s1/mutation-check.mjs` breaks the Rust engine in fifteen small ways, one at a time, and runs the differential run on 150 seeds of each panel.

| Mutation | Caught by |
|---|---|
| The deferral cap is one too few | The event-pop hash, in a panel seed |
| Dispatch ignores the queue entry tick | The event-pop hash, in fixture E0 |
| The backoff is half what it should be | The event-pop hash, in fixture E0 |
| A cancel with nothing to act on counts as stale | The event-pop hash, in fixture E3-CT |
| An aborted attempt keeps its token | A mismatch in the panels |
| A cancel runs after a deadline | The initial checkpoint, in fixture E0 |
| A failure of a server that is down counts as effective | The event-pop hash, in fixture E3-NOOP |
| The checkpoint leaves out the deferral count | The slice checkpoint, in fixture E0 |
| A retry that is queued keeps its deferral count | The slice checkpoint, in a panel seed |
| The queue holds one job too many | The event-pop hash, in fixture E0 |
| A duplicate scripted key is not refused before the run | The refusal cases, which then differ in their words |
| The guard against a rebound key is off | Not caught, and equivalent: the check before the run refuses first (finding 6) |
| The parameters are not compared with the configuration | The refusal cases, which then run |
| Scripted events are not checked before the run | The refusal cases, which then run |
| A terminal job keeps its token | Not caught, and equivalent: it cannot be observed (finding 3) |

Thirteen of the fifteen were caught, and the other two are unobservable.
The record is `dess/s1/evidence/mutation-check.json`.

## What the agreement does not show

1. **The same author wrote both engines.** The oracle was written first, from the design and the contract, and the Rust engine second, from the same text.
   They share no code.
   They can still share a misreading of the design, and agreement cannot reveal one.
   Findings 6 and 7 are two, and a reviewer found both.
   The known answers of the design and the S0 pins, which another implementation produced, are the only checks that do not depend on the author.
   A reviewer who did not write either engine should read the contract against the design.
2. **The panels are random.** They reach the rules that the coverage report lists, and a rule that is reached in few seeds is tested by few.
   A failure by retry deferrals is reached in 10 seeds of the first panel and 15 of the second.
3. **The state hash covers what the contract says it covers.** Terminal jobs are not in the state, and their terminal states are compared at the end of each run.
   A bug that changed only a terminal job's history, and nothing else, would be seen only through the counters.

## Adjudication rule for the next discrepancy

If the engines disagree, find the first differing pop or slice, and read the design's text for the rule involved.
If the text decides, the engine that disagrees with it is wrong, and a fixture is added that fails on it.
If the text does not decide, the choice is written into the contract as a numbered rule first, and only then are the engines changed.
Neither engine is ever changed only to match the other.
