# Conway's Game of Life

A board of cells that live and die by Conway's rules, which a person draws on and sets going.
The panel is in its own group, "Cellular Automata".
It adds no library API: the scene is a library `Scene`, the Play, Step and pattern buttons are `svg_dom_graph::controls::svg_buttons`, and the speed is a slider from `svg_dom_graph::controls::sliders`.
The board itself is drawn by the page, as SVG in the scene's layer.

## The rules

Each cell has eight neighbours: the cells above, below, left and right, and the four diagonals.
All the cells change at once, from the generation before:

* A live cell with two or three live neighbours survives.
  With fewer it dies of loneliness, and with more it dies of crowding.
* A dead cell with exactly three live neighbours is born.

The edges either wrap round or stop.
Where they wrap, the board is the surface of a doughnut, and a cell on one edge is next to the cells on the opposite edge.
Where they stop, a place beyond the edge is dead.
The page starts with the edges wrapped.
A board is at least one cell across and one down, and a size of 0 is made 1, since where the edges wrap a cell is counted by its place modulo the size, which a size of nothing has no answer for.
So no method of a board can fail, whatever size it was asked to be.
The page uses 40 by 30.

## The scene

The scene is 560 pixels wide and has three parts.

* **The strip above the board.**<br>
  The buttons Play or Pause, Step, Random, Clear and Wrap edges, and under them the slider for the speed.
  The slider goes from 1 to 30 generations in each second, in steps of 1, and starts at 8.
  Wrap edges is a toggle button, and says which it is with `aria-pressed`.
* **The board.**<br>
  40 cells across and 30 down, each 14 pixels square, so 560 by 420 pixels.
  The grid is one path, and so are the live cells, which are drawn as one rectangle for each run of cells in a row.
  The page therefore sets one attribute for a generation, and not one for each cell.
* **The strip under the board.**<br>
  Two rows of buttons, 80 pixels in all.
  The first row has a button for each pattern, which replaces the board and starts the count of the generations again.
  The second row has a button for each spaceship, which adds one to what is on the board.

The generation and the number of live cells are written above the scene, as `Generation 12: 53 live cells`.

## Drawing on the board

* A click or a touch brings a cell to life or kills it.
* A drag gives every cell that it crosses the state that the first cell was changed to.
  So a drag that starts on a dead cell brings cells to life, and one that starts on a live cell kills them.
* The browser reports the pointer only now and then, and during a fast drag it can have crossed several cells between two reports.
  So a line of cells, by the algorithm of Bresenham, joins the cell that was painted last to the one that the pointer is over now, and every cell on it is given the state.
  A drag that is reported only at its two ends therefore paints every cell between them, along a row, a column or a diagonal.
* The pointer is captured on the scene, so a drag that leaves the board goes on until it is let go, and comes back in at the same state.
  While it is off the board nothing is painted, and the cell that it comes back to starts a new line, so a drag does not draw across the board from where it left to where it returned.
* A drag ends in one place, whichever way it ends: the pointer is let go, the browser cancels it, or the pointer capture is lost.
  The page listens for `pointerup`, `pointercancel` and `lostpointercapture`, and each does the same thing, which can be done twice without harm, since the capture is lost after every `pointerup`.
* A drag belongs to the pointer that began it, by its pointer id.
  Another finger or a mouse cannot move it on or end it, and cannot start a drag of its own while it goes on.
* If a drag is recorded but its pointer no longer holds the capture, its end was never reported, so the next pointer that goes down ends it and starts its own.
* A press in the strips above and below the board changes nothing.
* The scene has `touch-action: none`, so that a finger that draws does not scroll the page.

## The keyboard

The board is one group that can take the keyboard focus, with the role of an application.
With the focus on it, a red cursor outlines one cell.

* The arrow keys move the cursor a cell, and it goes round to the other side at an edge, whether or not the edges of the board wrap.
* The Space and Enter keys bring the cell under the cursor to life, or kill it.
* The cursor starts in the middle, and a click moves it to the cell that was clicked.

Assistive technology is told what is under the cursor, as `Column 22, row 15, alive`, with the columns and rows counted from 1.
That is a live region of its own, which is only there for assistive technology.
The generation and the population are a live region too.
It is announced when the generations are stopped and is not while they go, since it would be read out every time it changes.

## When a write to the page fails

The board in the code is the truth, and what is on the page is drawn from the whole of it every time.
A step, a click, a pattern or a key changes the board first and then draws it.
If a write to the page fails, the board is right and the page is out of date.
The demo does not undo the change.
It remembers that the page is out of date, and the next button, key, drag, frame or move of the slider draws the board again before it does anything else.
Since every draw is of the whole board, that is enough to put the page right, and the error is then hidden.

* A failed write stops the generations, and the Play button says `Play` again and the readout is announced again, as when a person pauses them.
* The error says that what is shown may be out of date, and that the next button, key or drawing draws it again.
* The error is above the board, so showing or hiding it moves the board on the page.
  While a drag goes on, the browser would report the pointer over cells that it had not crossed, and they would be painted.
  So a change to the error waits until the drag ends.

## Speed

The generations are timed by `requestAnimationFrame`, and not by a timer, so they stop when the page is in the background.
Each frame adds the time that has passed to what was carried over, and a generation is worked out for every period, which is a thousand milliseconds over the speed.
The times and the period must be finite, and the period above 0, or nothing is due and nothing is carried, so that time that cannot be made sense of does not build up for a later frame.
A speed that is not finite and above 0 has no period, which the page treats as nothing being due.
No more than four are worked out in one frame, and a gap longer than a quarter of a second counts as a quarter.
So a page that has been in the background goes on from where it was, and does not race to catch up.
Someone who has asked their system for less motion starts with the generations stopped.

## The patterns

| Button | Cells | What it does |
|---|---|---|
| Glider | 5 | Crawls diagonally, one cell every four generations. |
| Blinker | 3 | Turns between a row and a column, with a period of 2. |
| R-pentomino | 5 | Takes 1103 generations to settle down on a board without limits, and sends out gliders on the way. |
| Acorn | 7 | Takes 5206 generations to settle down on a board without limits. |
| Glider gun | 36 | Bill Gosper's gun, which sends out a glider every 30 generations. |

A pattern is put in the middle of the board.
The generation count starts again at 0, and whether the generations are going is not changed.
On this small board the edges that wrap bring the gliders back to the pattern that made them, so a gun, an R-pentomino and an acorn do not do what they would on a board without limits.
Turn off Wrap edges to see more of it.

## Spaceships

The second row of buttons under the board adds a light, medium or heavy weight spaceship, which are Conway's three that fly along a row.
They have 9, 11 and 13 cells.
Each repeats its shape every 4 generations, 2 cells to the right.
They are drawn the other way round from how they are usually shown, so that they fly to the right, into the board from the left.

A spaceship is added, and does not replace the board.
What is on the board stays, so a ship can be added to a gun or a glider, the cells that are already alive stay so, and the count of the generations goes on.
Whether the generations are going is not changed.
The ship is put at the left edge, level with the cursor, and moved up or down as far as it needs to be to be wholly on the board.
So the arrow keys choose its height.
The cursor starts at row 16, and a ship of 4 rows is then at rows 14 to 17.
Where the edges wrap, the ship goes round and comes back.
Where they stop, it runs into the edge and is broken up.

## How it is checked

`game_of_life/board/unit_tests.rs` checks the rules one at a time and that every cell changes at once, a block as a still life, a blinker with a period of 2, a glider that moves a cell down and across in 4 generations, that on a board of 8 with wrapping edges it comes back to where it began in 32, and that the Gosper gun makes a glider every 30 generations.
It checks that each spaceship goes 2 cells to the right every 4 generations without changing shape, that it goes round a board with wrapping edges, and where one is put.
It checks that each spaceship goes 2 cells to the right every 4 generations without changing shape, that it goes round a board with wrapping edges, and where one is put.
It also checks the edges, the patterns, the random board and the pace.
`game_of_life/view/unit_tests.rs` checks which cell a point is in, the line of cells between two cells (both ends, no gap, the same length either way, and one cell in each column or row), the paths that are drawn, the cursor, the buttons and the writing.
`game_of_life.rs` in the CDP tests checks writes that fail, by making the browser throw for them: a cell, a step, a frame and a key that cannot be drawn, and that the next interaction draws the board and hides the error.
It checks drags that the browser cancels or whose capture is lost, a pointer that is not making the drag, a second pointer that goes down during one, and a drag whose end was never reported.
It also checks the scene, Pause, Play and Step, a blinker that turns on a step and back on the next, drawing by click and drag, the keys and what assistive technology is told, each pattern, the edges that wrap and stop, and the speed.
