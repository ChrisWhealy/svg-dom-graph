# The resource risks, measured

`docs/security.md` lists the ways a graph description can exhaust a browser, and says that the library sets no limits of its own.
The coverage audit of 4 November 2026 called the missing budget a gap, "no measured scale budget identified".
This page is the measurement.
It takes each risk in turn, says what was run and what it found, and gives the budget that follows.

Every figure is from one browser on one machine, and says how the cost scales and not what it is elsewhere.

| What | Value |
|---|---|
| Browser | Headless Chrome 157 |
| Machine | A Mac with 12 cores and 32 GB of memory |
| Clock | A step of 5 µs in the test runner, and 100 µs in a served page that is not cross-origin isolated |
| Records | `spmv/evidence/*.jsonl`, written by `node spmv/measure.mjs` |

A timing is a measurement and never a gate.
No other browser, and no slower or smaller device, was measured, so nothing here is a limit for one.

## How the risks were tested

- **A scene of the matrix.** Every column of the matrix is a node and every row is a node, and every nonzero is an edge.
  Two data nodes show windows of the vector and of the product, and are rewritten at each step.
- **Independent knobs.** The number of entities, the adjacency (flat or hub-heavy), the length of a label, the cells of a data node and the entries changed in a step are set separately.
- **A product that is checked.** The product is exact integer arithmetic.
  The JavaScript oracle agrees with the Rust core on 625 scenarios, so a figure is for a scene that shows the right numbers.
- **A budget first.** A scenario is checked against a budget, and its matrix generated, before a single scene call.
  Only a scenario that passed can be built.

## The risks

### 1. The total number of nodes and edges

| Entities | Edges | Page elements | Build | Of which nodes |
|---|---|---|---|---|
| 100 | 148 | 599 | 14 ms | 7 ms |
| 1,000 | 1,498 | 5,891 | 59 ms | 36 ms |
| 5,000 | 7,498 | 27,768 | 513 ms | 426 ms |
| 10,000 | 14,998 | 55,172 | 1,804 ms | 1,617 ms |
| 14,000 | 20,998 | 77,070 | 3,378 ms | 3,141 ms |
| 20,000 | 29,998 | 109,909 | 7,106 ms | 6,789 ms |

- **Edges are cheap.** An edge costs 9 to 12 µs at 1,000 entities and over, and a page holds about one element for each edge.
- **Nodes are not.** A node costs 66 µs among 100 and 85 µs among 5,000.
  The cost of making all of them is about 16 to 17 nanoseconds times the square of their number.
  That constant is the same at 10,000, 14,000 and 20,000 entities.
- **The cost is the library's.** The core generates a 5,000-entity matrix in about 4 ms, so the time is in the scene.
- **Beyond what was measured.** The runner allows a test about 20 seconds, so 28,000 and 40,000 entities were not built.
  The fit predicts about 14 s and 27 s, and these are predictions and not measurements.

Page elements are about 4 for a node, 1 for an edge and 2 for a data cell, and the budget's model uses that.

### 2. The cells of a data node

| Cells in a node | Page elements in both nodes | Build of both nodes | A step, p50 | A step, p99 |
|---|---|---|---|---|
| 10 | about 40 | 7 ms | 0.01 ms | 0.34 ms |
| 100 | about 400 | 9 ms | 0.07 ms | 0.19 ms |
| 1,000 | about 4,000 | 28 ms | 1.0 ms | 1.6 ms |

- **Each step changes half the cells**, and the library rewrites only the cells that changed.
- **A step grows with the cells**, about 1 ms for 1,000 cells, and stays under a frame.
- **A cell's size is fixed by its type.** The greatest figures the product can reach stay inside the data node's frame, and a test asserts it.

### 3. Node degree

A hub is a column joined to every row, so its degree is the number of rows.
Each edge appends a relationship clause to the node at both ends, and the library writes the whole accumulated description again.

| Degree | Description, 16-character labels | Edges build | A drag move, p50 |
|---|---|---|---|
| 1,000 | 28 KB | 17 ms | 1.25 ms |
| 5,000 | 140 KB | 114 ms | 5.5 ms |
| 10,000 | 280 KB | 330 ms | 11.5 ms |
| 20,000 | 560 KB | 1,075 ms | 25 ms |

- **The description is linear in the degree.** It is about 28 bytes an edge at 16 characters.
  A test asserts that a hub's description is longer than any other node's and at least the degree times a label.
- **The build of the edges is worse than linear.** Doubling the degree from 10,000 to 20,000 multiplies it by 3.3, which is what rewriting a growing string for each edge does.
- **A drag move is linear.** It takes 1.1 to 1.25 µs for each edge at every degree, because a move redraws every connector on the node, and only those.
  A test counts the connectors that change and finds exactly the degree.
- **A frame is 16.7 ms.** A move of the busiest node is under it at a degree of 10,000, with a p99 of 15.8 ms.
  It is over it at 20,000, with a p50 of 25 ms.
  At the measured cost for each edge the median would cross a frame near 14,000 edges, which is an extrapolation.

### 4. String lengths

| Label length | Description of a hub of 200 edges | Build |
|---|---|---|
| 4 | 3 KB | 20 ms |
| 128 | 28 KB | 13 ms |
| 1,024 | 208 KB | 33 ms |

At low degree a long label is cheap.
The description is about the label length plus 12, times the degree, so the two multiply.

| Hub of degree 10,000 | Description | Edges build | Build in all |
|---|---|---|---|
| 16-character labels | 280 KB | 330 ms | 2.0 s |
| 128-character labels | 1.4 MB | 1,305 ms | 3.1 s |
| 512-character labels | 5.2 MB | 5,217 ms | 7.2 s |

- **The cost of a degree and a label together is quadratic in the degree and rises with the label.**
- **The heap is churned.** The heap after such a build was 128 to 294 MB for the larger hubs.
  The figure depends on when the collector last ran, so it says that memory is touched and not what a peak is.
- **A hub of 20,000 edges with 128-character labels** was predicted at 12 s and not built, for the reason given above.

### 5. Memory when a scene is rebuilt

A scene of 1,000 entities was built and torn down 80 times.

| | Result |
|---|---|
| Page elements after each teardown | 4 every time, the page's own |
| JS heap, median of the first 20 cycles | 2.66 MB |
| JS heap, median of the last 20 cycles | 3.00 MB |
| JS heap, range over all cycles | 1.2 to 7.4 MB |
| A build | 47 ms at p50, 68 ms at worst |

No accumulating trend can be told from the collector's noise in 80 cycles.
This is not an endurance result, and not a count of listeners, which were not measured here.

### 6. A budget, and a refusal

- **The budget is checked first.** A scenario that fails it builds nothing, and the page is untouched.
  A test builds a host, tries a refused scenario and finds the host empty.
- **A hostile size is answered at once.** 2,000,000 entities with 64 draws a row is refused by the first stage, before any matrix is generated, in under a millisecond.
- **A refusal is reported apart from a drawing,** with its code and its reason, in the gallery panel and in the data the oracle compares.

## The budget

The default budget is a policy for this class of device, and each figure has the basis shown.

| Limit | Value | Basis |
|---|---|---|
| Nodes | 10,000 | Builds in 1.8 s. Beyond it the cost of making nodes grows with the square |
| Edges | 60,000 | An edge is cheap, and the page-element limit bounds the rest |
| Degree of a node | 2,000 | A drag move is 2.3 ms, 14% of a frame, and the description stays under 0.5 MB |
| Cells in one data node | 1,000 | A step is 1 ms |
| Cells in all data nodes | 2,000 | Two nodes of 1,000 |
| Label length | 256 | A hub at the degree limit keeps a description near 0.5 MB |
| Page elements | 120,000 | The largest page built and driven was 100,174 |

The model of the page is a least-squares fit of 38 measured pages, rounded up so that it is never below the page: 60 and then 4, 1 and 2.
It was above the measured page in all 38 cases by 49 to 823 elements, and a test keeps it between the page and a third over it.
The product of degree and label length is not limited on its own, and the defaults keep it near half a megabyte.

## What making the graph readable costs

A graph that is made to be read has its edges given a class for their sign and size and its nodes tagged, so that a node can be selected.
That is one write for each edge and each node, and it was measured in the demo's window.

| Entities | Edges | Readable styling | Build in all |
|---|---|---|---|
| 1,000 | 1,498 | 1.5 ms | 44 to 75 ms |
| 5,000 | 7,498 | 7 to 8 ms | about 500 ms |
| 10,000 | 14,998 | 14 to 16 ms | 1.7 to 1.8 s |

It costs about a microsecond an edge, under 1% of a build, and the cost of making nodes is still the whole of the rest.
Selecting a node writes only that node's edges, so its cost is the node's degree and not the size of the graph.
Edge labels are made only up to 60 edges, since each is more elements.
The probes above build without any of this, so their figures are those of the scene alone.

## What was not measured

- **Any other browser.** Firefox and Safari were not run.
- **Any other device.** A phone or a slow laptop will have a lower envelope, and this page cannot say how much lower.
- **A person dragging.** The drag figures are from calls to `move_node`, and not from pointer events, so they leave out the browser's input handling and its painting.
- **First paint.** The build time ends when the scene has been made, and the browser's paint after it was not timed.
- **A peak of the heap.** Only the heap before and after was read.
- **Listeners and long endurance.** The page's elements were counted after each teardown, and listeners and hours of repetition were not.
- **Builds over twenty seconds.** The runner does not allow a test that long, so the largest sizes are predictions.

## Running it

```sh
node spmv/measure.mjs                       # the measurements, about two minutes, which need Chrome
node spmv/measure.mjs --only degree         # one of them
wasm-pack test --headless --chrome spmv-scene --test browser   # the tests of the scene
```
