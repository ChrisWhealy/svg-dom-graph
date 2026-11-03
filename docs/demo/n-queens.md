# The N-Queens puzzle

The whole depth-first search of the N-Queens puzzle, one event at a time, on a board of 4 to 9.
The panel is in its own group, "Backtracking".
It adds no library API.
Everything is demo-local, in `demo-app/src/n_queens/`.

## The search

The search tries the rows in order, and the columns of a row in order.
It never skips a branch because of a symmetry, so the trace is the real exhaustive search.
Each square is a `Try`, which is followed at once by exactly one `Reject` or `Place`.
A `Reject` says which line an earlier queen attacks the square along.
When several lines apply, the reason is the first of column, main diagonal and anti-diagonal.
The event also holds how many of the three lines have a queen on them, which is the count of directions and not of queens.
A `Place` puts a queen on the square, and a `Remove` takes it off again when the search comes back.
A board with a queen in every row is a `Place`, a `Solution` and a `Remove`, in that order.
The search ends with one `Exhausted`.

Snapshot 0 is the empty board, and event `k` makes snapshot `k`, so there is one more snapshot than there are events.
The numbers that follow can be checked against any other solver:

| N | Events | Solutions | On the page |
|---|---|---|---|
| 4 | 139 | 2 | yes |
| 8 | 33,589 | 92 | yes |
| 9 | 153,502 | 352 | yes |
| 10 | 732,563 | 724 | no, counted only |

A trace is limited to 250,000 events.
A search that needs more fails with `EventBudgetExceeded`, and is never cut short and then ended with an `Exhausted`.
A board of 0 is `InvalidBoardSize`, and one of more than 9 is `UnsupportedBoardSize`.
Both are refused before anything is allocated or any array is indexed.
The board of 10 is counted, by a separate function, so that its 732,563 events and 724 solutions are tested without being kept.
That function finds the free squares with bit masks and not with the builder's own check, so it is a cross-check of the builder.
The literal numbers in the tests are the reference.

## What a trace holds

A trace is three vectors.

* The events, 4 bytes each.
* A snapshot for each index from 0 to the last, as an array of 9 columns and a depth, which is 10 bytes.
  A row with no queen holds `0xFF`, and nothing else is a sentinel.
  A depth of 0 means no queens.
* The indices of the solutions, in order, which the buttons search by binary search.

A snapshot never holds a `Vec`, a string or a matrix.
A board of 8 takes about 0.9 MB by the capacity of its vectors, and a board of 9 about 3.5 MB.
The page shows the figure for the board that is on it.

## What the page shows

Nothing on the page is kept between steps.
`projection::project` makes a `View` from the trace and an index, and the page draws only what differs from the view it drew last.
So going back is the same call as going forward.

* **A Try** outlines its square with a solid blue outline, and draws three rays through it: its column and both diagonals, from edge to edge of the board.
  The rays are the same whether or not the square turns out to be free, and whether or not a queen is in the way, so a Try never gives the outcome away.
* **A Reject** outlines the square with a dashed red outline and a cross.
  The three rays stay, and the one that is the reason is drawn thick and solid while the other two are thin and dashed, even if they also conflict.
  The status names the reason, and adds "first of K conflicting directions" only when K is more than 1.
* **A Place** draws the queen, and removes the outline and the rays.
* **A Solution** draws a heavy dashed frame round the whole board.
  The frame comes with a solution and nothing else, and so never meets an outline.
* **A Remove** outlines the square the queen has left with a dotted violet outline.

There are no tints for squares that are attacked, and no state for them in the page.
The only thing drawn for an attack is the three rays.

### The layers

The board is drawn in layers, back to front, so that the rays go under the queens and the outline over the rays:

1. The squares, as a chessboard.
2. The three rays, which take no pointer or focus.
3. The queens, as one glyph for each square, shown or hidden.
4. The outline, which is one element that moves.
5. The solution frame.
6. The grid of squares that a person can focus, which are transparent.
7. The focus ring.

A square is one gridcell, which has an id such as `nq-r3-c5` that counts from 0.
Its fill, its glyph and its outline are separate elements, so the page does not need one element to hold all three.
The page never adds or removes an element between steps.
A new board size builds the new layers apart from the page, draws the first snapshot in them, and then swaps them in at once.

Every state differs by its shape, its dash or its stroke width as well as its colour.
In forced-colours mode the style sheet uses system colours.

## The controls

The controls are drawn in the scene, in a strip above the board, so that they are part of the same SVG as the board.
The scene is 640 pixels wide.

* The two selectors, for the size of the board and the speed, are the browser's own `<select>` in a `<foreignObject>`, as the library's sliders are.
* The six buttons are the library's `svg_dom_graph::controls::svg_buttons`.
* The range is the library's slider, which is a native `<input type="range">` in a `<foreignObject>`, with its name and the step written above it.
  This keeps the native behaviour of the range.

`layout/` says where each of them is, and has unit tests that check that none is over another or outside the scene.
The board is in its own group below them, and a new size of board replaces only that group.

* **Previous** and **Next** move one event, and **Restart** goes to snapshot 0.
* **Previous solution** and **Next solution** go to the nearest solution that is strictly before or after the step.
  From a solution, the next one is the one after it, never itself.
* **The range** goes straight to any step, and never draws the steps between.
  Moving it asks for a draw at the next animation frame, and a burst of moves between two frames is drawn once, at the last place.
  Letting it rest draws that place at once, and then says where it is, once.
  The left and right arrow keys move it by one step, and the Page keys move it by whatever the browser does.
  At 9 by 9 there are 153,502 steps, so dragging with the pointer is coarse by nature.
* **Play** shows the next step after a delay of 1000 divided by the speed, and the speed is 1, 5, 20 or 50 events a second.
  The tick after that is asked for only when the step has been drawn, so a slow page gets a lower rate and never a backlog.
  Play stops at the last step.
  Play at the last step does nothing, and does not start again.
  Every other control stops it, and so does a new size of board or a write that fails.

A button that is at an end is marked `aria-disabled` and is dimmed, and not made `disabled`.
So a button that has the keyboard focus keeps it when the search reaches the end.
Pressing it says why nothing happened, for example "Already at the last step."

## A write that fails

The controller changes the selected index only after the page has been drawn.
If a write fails part way, the page is marked dirty, the index stays where it was, Play stops, and the failure is shown above the board.
The next thing a person does, whatever it is, first draws the selected snapshot again in full.
That includes a button with nowhere to go, such as Next at the last step, and a seek to where the range already is.
If that draw fails too, nothing else happens, and the page stays dirty.
A new size of board replaces the whole board, so it makes a dirty page right.

A new size is a transaction.
Everything that can fail is done before the old board goes.
The new board is made and drawn apart from the page, and its active square and focus ring are set on it.
Then the controls are written, with the end of the range last, so a failure never leaves the range going further than the old trace.
If a control cannot be written, the controls are put back as far as they can be, and the old board, its trace and its step stay.
The page is then marked dirty, because the controls may be part way between the two boards, and the next action draws the old board again in full.
The selector goes back to the size of the board that is on the page.
Only when all of that has worked is the new board swapped in, and nothing after the swap can fail.

## Playback and the clock

All timing goes through a small `Scheduler` trait.
The page uses the browser's timers.
A test can ask for a scheduler that runs a tick only when it is told to, by putting `?manual-clock` in the address or setting `window.__nQueensManualClock` before the panel is built.
Then `window.__nQueensAdvance(n)` runs up to `n` ticks, and `window.__nQueensPending()` says how many are waiting.
A tick carries the number of the run of playback that asked for it, so one that was already on its way when playback stopped does nothing.

## Accessibility

* The board is a `role="grid"` of `role="row"` and `role="gridcell"`.
  The grid gives the row and the column, so the name of a square says only its state: "empty", "queen", "candidate", "rejected: main diagonal" or "removed".
* The board has one tab stop.
  Exactly one square has `tabindex="0"`, and the rest have `-1`.
  The arrow keys move between squares without wrapping, Home and End go to the ends of the row, and Control with them goes to the corners.
  The Page keys, Enter and Space are not handled.
  Nothing a person does on the board changes the search, and a click moves the focus and nothing else.
* The focus ring is drawn by the page, as two nested outlines on top of every other layer.
  It shows while the grid has the focus, including after a mouse click, and hides when the focus leaves.
* A polite live region says one sentence for each step that a person takes by hand.
  While Play runs, it says only a solution and the end, and a pause says where the search is.
  A move of the range is silent until it rests.
* Rows, columns and solutions are counted from 1 on the page and from 0 in the code and the tests.
  The two solutions of the 4 by 4 board are `[1, 3, 0, 2]` and `[2, 0, 3, 1]` in the code, and `[2, 4, 1, 3]` and `[3, 1, 4, 2]` on the page.

## Where the code is

* `solver/` is the search, the trace and the count, with unit tests.
* `projection/` is what each step shows, the status text, the geometry of the rays and the keys, with unit tests.
* `controller/` is the index, the buttons, the dirty page and the playback, behind two traits, with unit tests.
  A fake surface and a manual scheduler test every rule without a browser.
* `mod.rs` builds the panel and attaches the listeners.
  It holds `build_n_queens_demo`, which the page shows as the panel's source.
* `layout/` says where every control and the board are in the scene, with unit tests.
* `controls.rs` draws the selectors, the buttons and the range.
* `board.rs` makes the layers of one size of board, and writes one snapshot into them.
* `page.rs` is what the controller draws on: the board that is on the page, the view that was drawn last, the active square and the focus ring.
* `chrome.rs` finds the elements that the panel's own HTML declares.
* `scheduler.rs` runs the playback's ticks, with the browser's timers or with the clock that a test advances.
* `dom.rs` has the few helpers that make and write SVG elements.

The CDP tests in `cdp-integration-test/tests/cdp/n_queens.rs` drive the built page with a real browser.
They check the layers and the grid, what each kind of step draws, the range, the buttons at their ends, playback on a manual clock, the keys and the ring, a new size, and writes that fail.
