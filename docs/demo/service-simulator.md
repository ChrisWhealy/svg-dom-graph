# Service simulator

A queue, one to five servers, outages, retries and cancellations, simulated one tick at a time and drawn on one long-lived scene.
The panel is in the group "Concurrency", after the Petri net.
It adds no library API.
The simulation is the `service-sim-engine` crate, and the page is `demo-app/src/service_sim/`, with the projection, the surface, the controller and the session coordinator in `service-sim`.

## What it simulates

Jobs arrive at a service.
A job that finds an idle server starts at once, and one that does not waits in a queue of fixed size.
A job that arrives to a full queue is rejected.
A high priority job is taken from the queue before a low priority one.
A server can go down, which interrupts the job that it holds, and a job whose attempt fails waits to retry until it has no attempt left.
A job that is still not done at its deadline times out, and a job can be cancelled while it waits or runs.
Time is counted in whole ticks.
A snapshot is the state after one more tick in which something happened, and snapshot 0 is the state before anything has.
Nothing in a run is random, so a scenario is the same every time it is chosen.

## The scenarios

| Scenario | Servers | Snapshots | Pops | What it is |
| --- | --- | --- | --- | --- |
| Walkthrough | 2 | 12 | 19 | The engine's own known-answer scenario E0, with a rejection, a time-out, a cancel and stale events |
| No-op and stale events | 1 | 9 | 9 | E3-NOOP, where most events find nothing to do |
| Busy service / outages | 3 | 487 | 1,009 | The frozen generator `mixed_outage/7` at its manifest seed, scaled to 360 arrivals |
| Five servers | 5 | 493 | 1,103 | The same generator with five servers, which is the most that the scene draws |
| W6 reference trace (advanced) | 3 | 5,464 | 11,214 | The same generator at 4,000 arrivals, which the performance harness uses |

The first scenario is the one that the panel opens with.
The advanced scenario is a long run to look at, and it is not a benchmark.
Before a scenario is built, a separate engine replays it to its end without drawing anything.
That gives the exact number of snapshots, which is what the range runs to, and it is checked against the totals above.
A scenario that disagrees is refused, with a reason, and the one on the page stays.

## The page

The controls are in the scene's own `<svg>`, above the picture, and the inspector is beside it.
They are native HTML in `foreignObject`s, so they do not zoom or pan with the scene, and they scale with it at any width.

| Control | Does |
| --- | --- |
| Scenario | Starts another scenario at snapshot 0 |
| Previous, Next | Move one snapshot |
| Restart | Goes back to snapshot 0 |
| Play, Pause | One toggle. Steps at the speed chosen, and stops at the end. It never loops |
| Speed | 1, 2, 5 or 10 snapshots a second |
| Snapshot (the range) | Goes straight to a snapshot. Each arrow key is a request of its own, and the last one wins |
| Repair display | Draws the snapshot that you are on again, in full, and moves nothing |
| Inspect a job | Outlines a job in the picture and says what state it is in |

A button that has nowhere to go is dimmed with `aria-disabled`, and is never `disabled`.
It can still be pressed, and then says so, so a click is never lost.
At the end of a scenario, Play does not start a timer.
It says "End of scenario", and the button stays Play.

## Reading the picture

- **The queue** is a column of twelve boxes, in the order that the jobs will be taken, with a line that says how many more wait than fit.
- **Job dispatch** takes the job at the front of the queue and gives it to a server that is idle.
- **A server** says `idle`, `down`, or the job that it runs.
  Its box is blue while it is idle, green while it runs a job, and red while it is down.
  The word is always written as well, so the colour is never the only sign.
- **The counters** are the totals so far: arrived, admitted, rejected, completed, timed out, cancelled, failed, waiting to retry, stale pops and no-op pops.
- **Snapshot in eight numbers** sums up the snapshot that you are on, two cells to a row.
  The rows are the snapshot and the tick, how many wait and how many servers are busy, the totals completed and timed out, and the totals cancelled and rejected.
  It is one of the library's data nodes, and it is rewritten once for each snapshot.
- **The selected job** is outlined wherever it is, and its line at the bottom says where.

The inspector says the same in words, and the picture is also read aloud by name, so nothing depends on colour or position.

## Hiding the panel

Hiding the panel, or the page, stops Play, and nothing starts it again for you.
The page says "Paused while hidden.
Press Play to resume." and keeps saying it until you act.
A seek that is still on its way is cancelled by a hide.
If the panel is hidden and shown again within one task, the browser tells the page only that it is shown, so playback may go on, and nothing is doubled.

## When something goes wrong

A write to the page that fails does not count as a commit.
The snapshot stays where it was, the page says that it needs repair, and Repair display draws the committed snapshot again in full.
A scenario that cannot be shown leaves the one that you had exactly as it was, with its snapshot, its selection and its picture, and says which step failed.
A scene with another number of servers is built out of sight and read back before it replaces the old one.
A failure at any step puts the old one back, with the focus where it was.

## How it is tested

| Test | Shows |
| --- | --- |
| `cargo test -p service-sim` | The projection, the controller, the coordinator and the catalogue, natively, with a fake host |
| `wasm-pack test --headless --chrome service-sim` | The real surface and controller on a real page |
| `dess/demo-panel/test-gallery.mjs` | The real gallery in Chrome, with real clicks and keys, against the independent JavaScript engine |
| `dess/demo-panel/test-gallery.mjs --nightly` | The long paths: the 487 and 5,464 snapshot scenarios, and many cycles that must leave nothing behind |

The independent engine in `dess/reference-js` shares no code with the Rust engine, and the two agree on 1,228 scenarios.
Failures are made by a page-level injector that fails or detaches a DOM call, so the gallery is one wasm build with no test hook in it.
`dess/gather-evidence.mjs` runs every check in turn, and `dess/demo-panel/p4/README.md` says what each phase shows.

## Browsers

Chrome is the browser that the tests run in.
Firefox and Safari are not tested, and nothing here claims that they work.
The server colours use the CSS selector `:has()`, which needs a recent version of either.
What has been seen in them is recorded in `dess/demo-panel/p4/browser-observations.md`.
