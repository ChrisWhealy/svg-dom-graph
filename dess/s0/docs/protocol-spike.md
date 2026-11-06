# The Layer C protocol spike

Design revision 6 specifies how Layer C measures Next and Play.
A harness-owned scheduler stamps the marks, and Play is a one-tick Play and Pause cycle at 1 slice a second.
Any repetition with an extra tick is rejected.
S0 tried that protocol before any runner exists.
The spike is `dess/s0/spikes/protocol-spike.mjs`.

## What the spike is

The page is a stand-in for the product, and not the product.
`protocol/page/product.js` has a controller, a surface and a scheduler shaped as the design's section 6 has them.
The controller reaches time only through a scheduler with `schedule(callback, ms)` and `cancel(handle)`.
Play schedules one timer, and the next only after a commit.
Every stop advances an epoch, and a callback from an earlier epoch does nothing.
The surface writes 8 counter cells and 12 queue slots on each commit, with the slice number among them.
It steps through the first 700 slices of the manifest seed's run, which the prototype engine produced in the page.

There is no measurement code in the product file.
The harness's page, `protocol/page/index.html`, builds the controller with a scheduler of its own.
That scheduler stamps a user-timing mark immediately before it calls what the product scheduled.
A harness listener in the capture phase stamps a mark when a Next click is delivered, and when Pause is.
The runner reads the product file, takes out its comments, and fails if it finds `performance.`, a mark name or the harness's hook.

## The protocol that was run

Each repetition is a fresh Chrome, with real input through `Input.dispatchMouseEvent`.

| Series | Cycles | What each does |
|---|---|---|
| Next | 10 untimed, then 30 timed | A real click on the Next button, and 150 ms for the presentation |
| Play | 10 untimed, then 30 timed | A real click on Play, one timer tick at 1 slice a second, its presentation, then a real click on Pause |

Both series start from the cursor after slice 256.
Each Play cycle checks that exactly one slice committed, that the epoch advanced, that no timer is pending, and that the cursor moved by one.
After the last cycle a quiet window of 1.3 seconds checks that nothing ran, and a cancelled callback that is forced to run commits nothing.
The cursor ends at slice 296, as the design says.

## Results

Three valid repetitions, each with 30 Next and 30 Play observations, every one attributed to a frame.

| Repetition | Next median | Play median | Pause after the presentation | Wall time |
|---|---|---|---|---|
| 0 | 10.6 ms | 3.6 ms | 30 of 30 | 55.1 s |
| 1 | 11.1 ms | 3.6 ms | 30 of 30 | 55.0 s |
| 2 | 13.2 ms | 3.7 ms | 30 of 30 | 55.1 s |

Next is input to presentation.
Play is the timer callback to presentation, and the 1,000 ms interval is not in it.
The two are different metrics and are never pooled.

### The runtime budget

A repetition costs about 55 seconds, of which Play is 48.4 and Next is 6.6.
Three repetitions are about 2.75 minutes, and with browser start-up they fit in the design's six minutes.
A `wall_ms` of 90,000 for a repetition has a margin of 35 seconds over what was measured.
The cost is the 1 Hz interval, and it cannot be cut without risking the extra-tick race.

### Next needs the wider frame rule

The default attribution found no frame for any of the 30 Next clicks.
The click's handler commits synchronously, and Chrome does the style and layout in that same task.
The frame that shows the change then holds only a paint.
This is the case that the Phase 3 findings describe as a forced layout leaving no style step.

So the Next series uses `qualify: 'update'`, which finds a frame with a style step, a layout step or a paint.
The harness labels such a figure as the upper bound of a window and not a cause, and the result records the qualifier.
Play commits in a timer callback, so its style step is in the frame, and the stricter default works.
The product could commit Next in an animation frame instead, but that would change what Next does.

### The extra-tick control

With `--inject-late-pause` one Pause is delayed past the interval, so that a second tick commits first.
The runner reports `invalid_protocol_extra_tick: 2 ticks in a cycle, and one was meant`, and rejects the whole repetition.
It does not retry the cycle, drop a tick or count two.

### The three-repetition classification

The three repetitions were given to the real comparison as a baseline and as a candidate.
Both metrics came out `diagnostic`, with no interval, because three is fewer than five repetitions.
The medians and the ratio are shown, and the report says why there is no verdict.

## What it means for the product

- The controller takes its scheduler as a constructor argument, and the harness builds it with a wrapper.
  So the harness page must be a composition root that can build the product's controller, and the controller cannot be private to the gallery.
- The mark is the harness's, so no hook is added to a release build.
- Next commits are measured with the wider qualifier, and a note on every Next figure says so.
- Play is nightly or manual until the CI cost of 55 seconds a repetition is accepted.
  A missing Play series is reported as not run.

## What it does not show

- The product's library calls.
  The surface writes plain DOM and does not call `set_data_values`, so the figures say nothing about the library's cost.
  That is stage S2 and S5.
- Any browser but Chrome 157, on one machine.
- A stale tick, a hidden tick or a failed tick in a latency series.
  The spike rejects a repetition for the first and fails it for the others, and the real controller will have more ways to produce them.
