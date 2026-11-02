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
* The pointer is captured on the scene, so a drag that leaves the board goes on until it is let go, and comes back in at the same state.
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

## Speed

The generations are timed by `requestAnimationFrame`, and not by a timer, so they stop when the page is in the background.
Each frame adds the time that has passed to what was carried over, and a generation is worked out for every period, which is a thousand milliseconds over the speed.
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
`game_of_life/view/unit_tests.rs` checks which cell a point is in, the paths that are drawn, the cursor, the buttons and the writing.
`game_of_life.rs` in the CDP tests checks the scene, Pause, Play and Step, a blinker that turns on a step and back on the next, drawing by click and drag, the keys and what assistive technology is told, each pattern, the edges that wrap and stop, and the speed.
