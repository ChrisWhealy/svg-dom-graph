# Sparse Matrix: Scale and Limits

The panel is in the gallery's Scale group, and it opens the demo in a window of its own, as large as the screen allows.
The demo draws a sparse matrix times a vector.
Every column of the matrix is a node on the left, and every row is a node on the right.
Every nonzero is an edge from its column to its row.
Two data nodes show the first cells of the vector and of the product, and a small square shows where the nonzeros are.

It is the demonstrator of scenario D of the coverage audit, and its purpose is to find where the library's documented resource limits are.
The figures and the reasons for the budget are in [`spmv/docs/resource-risks.md`](../../spmv/docs/resource-risks.md).

## Table of Contents

- [The window](#the-window)
- [Reading the graph](#reading-the-graph)
- [Panning and zooming](#panning-and-zooming)
- [The controls](#the-controls)
- [What a step does](#what-a-step-does)
- [Refusals](#refusals)
- [Reading the readout](#reading-the-readout)
- [Tests](#tests)

## The window

The window is one `<svg>`, and all of it is in the scene.
The controls are above the graph, and the readout is below it, and each is the browser's own HTML in a `<foreignObject>`.
Every list and every button is in one of them, and none is on the page round the `<svg>`.
The graph between them is a nested `<svg>` that pans and zooms, so the controls and the readout stay where they are.
The controls look as those of the Petri net demo do.
A list is the browser's own `<select>` with its name beside it.
A button has the white fill, the slate outline and the teal pressed state of the library's own SVG buttons.

## What the window says

The readout under the picture opens with four short paragraphs, written for a person who does not know the task.

- **What this shows.** A computer often multiplies a table of numbers, the matrix A, by a list of numbers, x, to make a new list, y.
- **How to read it.** Each y is made from one row of A: multiply every number in the row by the x in its column, then add the products.
- **What a step does.** A never changes.
  A step changes one number of x, and only the y that use it are worked out again.
- **Why it matters.** Most big tables are mostly zeros, so a computer keeps only the nonzeros, and a larger size shows how much a page can draw.

The line under it says how the steps will go, in order or at random, before the first one.

## Reading the multiplication

The first scenario has 16 entities, which are 8 columns and 8 rows, and it is drawn as the multiplication itself.
A matrix of up to 12 rows and 12 columns is drawn this way, and a larger one is drawn as a graph, which is described below.

The picture reads from left to right as `A × x = y`.

- **A is a grid.** Each cell is a number of the matrix, blue if it is positive and red if it is negative.
  A zero is a faint 0 on a pale cell, so the nonzeros stand out.
- **x and y are columns.** A vector is a list and not a grid, so each is a single column with its entries one under another.
- **One row is worked.** The row that is selected is outlined in A, and its nonzeros are lit.
  The entries of x that it uses are lit in x, and its entry in y is lit in y.
- **The operations are drawn out.** For each nonzero of the row there is a `MUL` node.
  Its two inputs are the number of A and the entry of x, each named beside it, and it shows their product.
  The products are added one after another by `ADD` nodes, and the last sum is the entry of y.
- **Click another row.** Click any cell of a row of A, or an entry of y, or choose it in the list, to work that row.
  A row of more than six nonzeros draws six and folds the rest into one node, so the sum is still the whole row.
- **y is also drawn as bars.** There is one bar for each entry of y, level with it, growing right for a positive number and left for a negative one.
  The longest bar fills its half of the width, and a note says what that is.
- **A step changes x, in order.** By default a step changes the next entry of x: x0, then x1, and so on round.
  The numbers in x, in the operations, in y and in the bars are written in place, and what changed has a dashed green outline.
  The numbers of A that the changed entry was multiplied by have a dashed green outline too.
  The green line under the controls says which entries changed, from what, to what, and which comes next.
- **The speed of a run is chosen.** The Run speed list has four choices.
  Slow takes a step every 3 s, medium every 1.5 s, fast every 0.5 s, and flat out every frame.
  Medium is the default here, and flat out is the default for the measured graph.
- **Random is still there.** The Step order list can choose the seeded random stream that the contract defines, which is what the measurements use.

Every number in the picture is a number that the core holds, and the sum for a row always ends at the entry of y.
The readout also writes the row out as an equation.

```text
y2 = 1·x2 + 2·x7 = 1·31 + 2·82 = 195
```

## Reading the graph

A matrix that is larger than 12 on a side is drawn as a graph, with a node for each column and each row and an edge for each nonzero.
A graph of up to 64 nodes on a side is two single columns with a wide channel between them.
Each edge is then a line that can be followed from one end to the other.
A larger graph is there to be measured, and is packed into blocks.

- **A colour for the sign.** A blue edge has a positive value and a red edge has a negative one.
- **A width for the size.** The greater the value, the thicker the edge.
- **A label for the value.** Up to 60 edges, each edge says its value on it.
- **Words under the boxes.** Each of the three boxes above the graph has a caption.
  The grid of grey squares shows where the matrix is nonzero, and a matrix of up to 24 on a side is shown in full.
- **Participation.** Selecting a row or column puts a band across it, lights its cells, and highlights the entries of **x** and **y** that it uses or makes.
- **What a step changed.** After a step, the entries of **x** and **y** that changed have a dashed green outline.
- **A click for the arithmetic.** Click a node, or choose one in the list, to light its edges and the nodes at their other ends.
  The readout then writes out what the node stands for.
- **Any node from the keyboard.** The list holds only the first 100 columns and the first 100 rows.
  The Go to a node box takes any node, such as `x250` or `y400`, and selects it when Enter is pressed.
  A name that is not a node, or is out of range, is told in the banner with the range that exists, and the selection stays.

```text
x0 = 74 feeds 3 rows: y0 (×6), y3 (×−4), y5 (×−4)
```

A row of more than six terms writes six and says how many more there are.
Above 60 edges only the selected node's edges are strong, and the rest are faint, since a graph of hundreds of edges drawn alike is a tangle.
The edges are a picture and take no part in the pointer.
A click reaches a node however many edges are drawn over it, and a drag that begins on an edge pans the scene.
A graph of more than 30,000 edges is not tagged, so a node cannot be selected in it.

## Panning and zooming

- **Drag the background** to pan.
- **Hold Ctrl or Cmd and turn the wheel** to zoom about the pointer.
- **Zoom in, Zoom out and Reset view** are buttons in the controls, and zoom about the centre of the graph's room.

A tall graph is not drawn smaller than its text can be read, so a graph of 100 entities begins at the top and is panned to the rest.

## The controls

| Control | What it sets |
|---|---|
| Graph entities | The number of column and row nodes together, from 16 to a hostile 2,000,000 |
| Adjacency | Flat, where every column is alike, or hub-heavy, where 20 columns take 60% of the nonzeros |
| Label length | The length of every node's label, from the natural `x0` to 1,024 |
| Cells in a data node | 10, 100 or 1,000 |
| Entries changed in a step | 1, 10 or 100 of the vector |
| Edges | Automatic, every edge shown, or only the selected node's |
| Highlight a node | The first 100 columns and rows, as an alternative to a click |

Build makes the scene the controls describe.
Step runs one step, and Run steps once for each frame until it is stopped, or until the window is hidden.
Drag the busiest column moves the node with the most edges back and forth, and Rebuild 10 times builds and tears down the scene ten times.
Tear down removes the scene and leaves the controls.

## What a step does

A step changes some entries of the vector and keeps the product up to date from the change.
The two data nodes are rewritten and the graph is left alone, and no element is added to the page.
When a node is selected, what it computes is written out again from the new vector.
When the matrix has no more than 20,000 nonzeros, every step is also checked against a full recomputation of the product.

The product is exact integer arithmetic, so the oracle in `spmv/reference-js/` agrees with it without any tolerance.

## Refusals

Before any `Scene` call is made, the size is checked against a budget.
A size over it is refused with a reason, in a message styled apart from the readout of a drawing.
The check is made before anything is changed, so a graph that was there stays, with its readouts and its view, and the message says so.
Rebuild 10 times asked for after a refusal does nothing, so it cannot take that graph away.

| Choice | Refusal |
|---|---|
| 20,000 entities | `nodes 20000 is over the budget of 10000` |
| 2,000,000 entities | `nodes 2000000 is over the budget of 10000`, answered at once |
| A label of 1,024 | `label length 1024 is over the budget of 256` |

## Reading the readout

- **Plan.** The nodes, edges and data cells, the busiest node's edges, and the estimate of the page's elements against the budget.
- **Page elements.** What the build added to the page, against the estimate.
  A small graph also has labels on its edges, which the estimate leaves out.
- **Build.** The time in all, and for the nodes, the edges, the data nodes, the pattern square and the styling that makes the graph readable.
- **JS heap.** Before and after the build, where the browser reports it.
- **Step.** The latency of the last steps as p50, p95, p99 and worst.
- **Dragging and Rebuilding.** What a move of the busiest node costs, and whether ten rebuilds left the page as it was.

The page's clock has a step, which the demo measures and shows.
A time shorter than a step is told as under one step and not as zero.

## Tests

| Command | Runs |
|---|---|
| `wasm-pack test --headless --chrome spmv-scene --test browser` | The scene on a page: its shape, the values after every step, the edges' signs, a selection, a refusal, teardown and the documented costs |
| `node spmv/panel-smoke.mjs` | The window, as a person would use it, after `cargo run -p demo-server -- --build-only` |

See [`spmv/README.md`](../../spmv/README.md).
