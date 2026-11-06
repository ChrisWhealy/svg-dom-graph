# Stale pops and no-op pops

Every event that the engine pops is counted, and is included in the event-pop hash.
Design revision 6 uses two words for a pop that changes nothing, and does not say how the hash records the difference.
S0 settled it, with the prototype engine and two known-answer fixtures.

## The question

A cancel that finds its job already completed or timed out is ignored.
The prototype did not set `stale` for it, which a review found inconsistent with the definition that a stale pop finds its job or attempt already gone.

## What the design says

| Pop | The design's word | Where |
|---|---|---|
| A completion, a deadline or a retry eligibility whose job or attempt is gone | stale | Section 3.2 and 3.4, and E0, E1-RC and E2-RT |
| A cancel of a missing, rejected or terminal job | no-op | The rank table, row 2, and section 3.3 |
| A failure of a server that is already down, or a recovery of one that is up | no-op | The rank table, rows 1 and 4, and section 3.4 |

So the design already keeps the two apart.
A stale pop belongs to a token that was superseded, and has a cause inside the run: a cancel, a timeout or a failure that came first.
A no-op pop names something that had nothing to act on, and its cause is the input.
E3 asks for a "no-op ledger", and section 3.4 says outage no-ops "count as heap pops and are included in the event-pop hash".

The gap is the hash.
Its definition, "processed event key/payload + stale/active bit", has one bit, and a no-op is neither.

## Decision

A pop has one of three outcomes, and each is recorded.

| Outcome | Meaning | Counter | In the hash |
|---|---|---|---|
| Effective | It changed something | none | `e` |
| Stale | It carries a token and finds its job or attempt gone | `stale` | `s` |
| No-op | It has nothing to act on | `noops` | `n` |

- A cancel of an unfinished job is effective, and a cancel of anything else is a no-op, and not stale.
- A failure of a busy or idle server is effective, and of a down one is a no-op.
- A recovery of a down server is effective, and of an up one is a no-op.
- The stale count keeps its meaning, so E0 is still 5 stale pops.
- A reader of the results sees the stale fraction as the share of pops that wasted superseded work, and the no-op count as the share that the script wasted.

The alternative, counting an ignored cancel as stale, was rejected.
It would put a script event and a superseded token in one figure, and the design reports "stale fractions" for the second.

## The fixtures

| Fixture | Setup | Expected |
|---|---|---|
| E3-NOOP | One server. J0 completes at tick 2. Then a cancel of J0, a cancel of a key that never arrived, a failure of the idle server twice, and a recovery twice. J0's deadline at tick 10 | 9 pops, 9 slices, 10 snapshots, 4 effective, 4 no-ops, 1 stale. J0 completed, the server idle |
| E3-CT | One server, capacity one. J0 and J1 arrive together, and J1 is rejected. J0 times out while running at tick 3. A cancel of J1 at tick 1 and of J0 at tick 4. J0's completion at tick 10 | 6 pops, 5 slices, 6 snapshots, 2 no-ops, 1 stale. J0 timed out, J1 rejected |

The expected counts were derived by hand before the prototype ran, and it gave the same figures the first time.
They are design derivations until the Rust engine and the JavaScript oracle agree, as for E1-RC and E2-RT.
E0, E1-RC and E2-RT now state 0 no-ops, so a pop that is wrongly classed as one fails them.

## The pin moved

The hash format changed, so `event_pop_sha256` in `dess/s0/prototype/frozen.json` was written again.
The parameter hash and the input-stream hash did not move, which is what the pins are for: only the engine changed.
The manifest seed's run is now pinned as 11,214 pops, of which 3,002 are stale and 107 are no-ops.

## For the design

Revision 7 of the design should say three things.

- Replace "stale/active bit" in the event-pop hash with the three outcomes above.
- Say that a stale pop and a no-op pop are counted separately, and that results report both.
- Add E3-NOOP and E3-CT, with their counts, to the companion fixtures.

The S1 Rust engine has to reproduce the same outcome for every pop.
