# The discrepancy ledger

The design asks for a record of every place where the Rust engine and the oracle disagree, and of how each was settled.
This page is that record, and it also records what was found while building the comparison, since an honest ledger says what the comparison could and could not see.
The text of design revision 6 decides every discrepancy, and neither engine is changed to agree with the other until it does.

## Result

| Comparison | Scenarios | Pops | Slice checkpoints | Disagreements |
|---|---|---|---|---|
| Native Rust against the oracle | 1,206 | 399,483 | 234,754 | 0 |
| 32-bit Rust (`wasm32-wasip1`) against the oracle | 1,206 | 399,483 | 234,754 | 0 |

The scenarios are the five known-answer fixtures, 600 seeds of the panel, 600 seeds of the disturbed panel and the frozen revision `mixed_outage/7`.
A disagreement is any difference in a hash, a pop, a slice, a counter or a final state.

The two engines never disagreed on a pop, a slice or a hash, at any point of the build.
That is a statement about what was tested, and it needs three qualifications, which the last section sets out.

## What was found while building the comparison

| # | Found | Kind | Settled by |
|---|---|---|---|
| 1 | The first differential run reported all 606 scenarios as different, on `counters` only | A defect of the comparison, and not of an engine | The comparison read JSON text, and Rust writes an object's keys in sorted order. It now compares with the keys sorted. As `counters` is the last field compared, the pops, the slices and every hash had already agreed in all 606 |
| 2 | The first panel never reached a failure of a server that is down, a recovery of one that is up, a scripted arrival, or a cancel of a key that never arrives | A gap in the coverage | `dess/s1/coverage.mjs` counted the seeds that reach each rule. The disturbed panel `panel/2` was added to the contract, and it reaches the duplicate failure in 464 seeds and the duplicate recovery in 536 |
| 3 | Rule R6 said that a job's token rises when it reaches a terminal state, and no comparison can see that | An unobservable rule | The mutation check kept a mutation of it, and reported it as equivalent. R6 now says so. Both engines keep the increase |
| 4 | The design's "stale/active bit" in the event-pop hash has no value for a no-op | A gap in the design, settled in stage S0 | Three outcomes, as `dess/s0/docs/pop-outcomes.md` records. Revision 7 of the design should say so |
| 5 | The design does not say in what order the events that one pop creates take their sequence numbers | A rule left open | Contract rules R1 and R2, fixed before either engine was coded. The S0 pins, which another implementation produced, confirm them, since they include the sequence numbers |

## Rules that the design left open

The contract lists R1 to R12.
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
`dess/s1/mutation-check.mjs` breaks the Rust engine in eleven small ways, one at a time, and runs the differential run on 150 seeds of each panel.

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
| A terminal job keeps its token | Not caught, and equivalent: it cannot be observed (finding 3) |

Ten of the eleven were caught, and the eleventh is unobservable.
The record is `dess/s1/evidence/mutation-check.json`.

## What the agreement does not show

1. **The same author wrote both engines.** The oracle was written first, from the design and the contract, and the Rust engine second, from the same text.
   They share no code.
   They can still share a misreading of the design, and agreement cannot reveal one.
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
