# The controller

The design asks S3 for a manual clock, epochs and tokens, a synchronous hidden predicate, checks before and after every draw, and a forced mid-draw detach.
This page says what each is, and what the tests prove.

## What the controller holds

| Field | Meaning |
|---|---|
| The committed slice | The slice that was drawn and committed. It moves only after a draw that succeeded while the host was still there |
| The committed projection | What the page should hold. It is what a repair draws |
| The scenario generation | Counted up by every change of history |
| The playback epoch | Counted up by every stop of playback |
| The seek token | Counted up by every replacement of a seek |
| Dirty | The page may differ from the committed projection, because a draw failed or the host went |
| The engine | Staged ahead of the committed slice after a draw that failed, so that a retry does not step it twice |

The controller holds no browser.
It is generic over a `Host`, which draws and says whether it may, and a `Scheduler`, which runs a task later.
So every rule is tested natively, with a fake host and a clock that a test moves.

## The commit

A slice is committed in this order.

1. Ask the host whether it is live.
   If not, stop, invalidate what was scheduled, and say once that the simulator is not available.
2. If the page is dirty, draw the committed slice again in full.
   If that fails, stop.
3. Ask again, and then draw.
   The draw is synchronous.
4. Ask again after the draw.
   A host that left in the middle of it is detected here, even though a detached element accepts the writes and reports no error.
5. If the draw failed, stop, mark the page dirty, report once, and make one immediate full redraw of the committed slice.
   It is not retried.
6. Only now move the cursor.

## Navigation has one owner

A manual request is a next, a previous, a restart, a seek, a retry or a play.
Each one first cancels playback and any seek that is pending, and only then looks at its own target.
So a request that ends at a boundary has still replaced what came before it, and the last request that a person made is the one that wins.
A pause is not a navigation.
It stops playback and leaves a pending seek alone, which is the distinction that is kept on purpose.
Playback's own ticks do not come through this rule, since a tick must not stop the playback that it belongs to.

The review that found this gave the case of a seek to slice 10 followed by a seek to slice 0 while the committed slice was 0.
The second returned at a boundary and did not cancel the first, whose frame then committed slice 10.

## Selection and scenario switches are transactions

What the controller calls committed is what was shown.
A selection is staged with the projection that it makes, and the view is published by the commit and not before it.
A draw that fails leaves the selection that was on the page, so a later slice is not drawn with one that never was.
A scenario switch stages the new engine and draws its first slice in full.
It replaces the old scenario only if that draw succeeded while the host was still there.
Otherwise the old scenario is still committed, the page is repaired to it, and the switch can be asked for again.
A switch is a navigation, so playback and a pending seek are cancelled whether or not it succeeds.

The review that found this noted that the switch was not transactional.
It installed the new scenario and then drew it, so a failed draw left a scenario installed that had never been shown.
That is not an intended exception, and it is now tested.

## Tokens

A callback carries the epoch, the generation or the seek token that it was scheduled under, and holds only a weak reference to the controller.
It does nothing if any has moved, if the host is hidden or detached, or if the controller was disposed.
It is not promised that no callback runs while the panel is hidden.
It is promised that one that does is inert.

## Hiding

| Signal | What happens |
|---|---|
| The page is hidden or left | `visibilitychange` and `pagehide` stop playback and seeks by the same path. `pagehide` stops without asking, since a page that is being left may still read as visible. A hide and a show in one task are told one after the other, so playback is stopped at the hide and not resumed at the show. `pageshow` repairs a dirty page and never resumes |
| The panel is hidden | The hint stops playback, takes back the timer and the frame, and moves the epoch and the token. A callback that was already queued runs once and finds the host not live |
| The panel is shown | A dirty page is drawn again in full. Playback stays stopped until a person asks |
| The host leaves the document | The same as hidden, and nothing is drawn on a detached host |
| Disposal | The controller is marked first, then what it scheduled is taken back |

The predicate is synchronous and is asked at the entry of every callback, before scheduling, and before and after every draw.
It checks that the host is connected, that the document is visible, and that `checkVisibility()` says the host is rendered.
A hidden gallery panel is still in the document, so attachment alone is not enough.

## Seeking

A seek replays the run from its first slice into a new engine, at an animation frame.
Staging happens on that engine and never touches the committed state, so a seek that is replaced half way changes nothing that is shown.
A forced test mode replays one slice a frame, to make the races of cancellation reachable.
A replay that ends because the run has no more slices has found the last slice, and records it.
A replay that ends because the engine refused to go on has found nothing, and records nothing.
It reports the error once, and the cursor, the committed projection and the last slice stay as they were.
A seek whose frame cannot be asked for keeps the cursor and offers a retry, and no synchronous fallback hides it.

## What the tests prove

| Test | Proves |
|---|---|
| Full, step and seek | A slice reached by stepping, by a seek from the start, and by a seek from another slice gives the same projection and the same page, for E0 and for generated runs |
| Playback and the clock | One slice a delay, the next timer scheduled only after a commit, one timer at a time, a stop at the end |
| One callback queued before a hide | It is inert with the hint and without it, draws nothing, and is announced once |
| Showing again | A dirty page is repaired, playback does not resume, and no delta is written onto a page that is not known |
| A host that leaves in the middle of a draw | On a fake host, in a seek, and on the real surface after each of its overlay writes and its setter, the slice is not committed, nothing is written after the departure, and a return repairs the page |
| A failed write | Nothing is committed, one report, one repair and no retry, and a repair that fails leaves the page dirty until the next request, even one at a boundary |
| Replacement | A seek, a speed change and a scenario switch each leave a kept callback of the old one inert |
| Selection and switch transactions | A selection or a scenario switch whose draw fails, is refused because the host is hidden, or has the host leave in the middle of its draw, is not published. The committed projection, the selection, the generation and the old scenario's next slice are as they were, and the page is repaired to them |
| An engine error | The engine is made to refuse the sixth slice by a limit. A seek, a step and playback each leave the cursor, the committed projection, the page and the unknown last slice as they were, and report once. A natural end still records the last slice |
| Page lifecycle | A hide and a show before the next tick stop playback for good, on a real page with the real listeners. `pagehide` stops a page that still reads as visible, `pageshow` does not resume it, and a dropped listener is not called |
| Navigation ownership | Seek then seek back to the committed slice, restart and previous at the first slice, play then next, play then seek, seek then play, and seek then next, each with the callbacks of what was cancelled kept, leave the last request in force. A pause leaves a seek alone |
| No cycle | A timer and a frame that are waiting hold no strong reference to the controller |
| Closures | Over 60 cycles on the page's own timers, every closure is dropped, whether the task ran or was cancelled |
| The real gallery | A timer queued before the gallery's own route hides a panel runs after the hint, finds the panel hidden, and finds its probe still connected |

## The sensitivity check

`dess/s3/mutation-check.mjs` breaks the code in twenty-nine ways, one at a time, and runs the tests.
The record is `dess/s3/evidence/mutation-check.json`, and all twenty-nine are caught.
The first run caught fifteen and missed three, and the tests that close them are these.

| Mutation that was missed | Test added |
|---|---|
| A frame of a replaced seek is not stale | A held frame of the first seek takes no step for the second, so it commits in the fourth frame and not the third |
| A staged slice is stepped again | The run is not made again after a failed draw, counted by the engines that were built |
| The host is announced every time | Seven requests to a hidden host give one announcement, and a second episode gives another |
