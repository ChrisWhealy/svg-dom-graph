# The discrepancy ledger

The Rust core and the JavaScript oracle were written from `spmv/docs/contract.md`, the oracle second and from the contract alone.
This page records every place where building or comparing them found something, and how it was settled.
The text of the contract decides, and neither side is changed to agree with the other until it does.

## Result

| Comparison | Scenarios | Steps | Refusals | Disagreements |
|---|---|---|---|---|
| Native Rust against the oracle | 625 | 3,970 | 10 | 0 |
| 32-bit Rust (`wasm32-wasip1`) against the oracle | 625 | 3,970 | 10 | 0 |

The scenarios are 25 cases whose outcomes were derived by hand and 600 seeds of the panel.
A disagreement is any difference in a field of the outcome, a hash, a statistic or a refusal's code or message.

## What was found

| # | Found | Kind | Settled by |
|---|---|---|---|
| 1 | Two checks of the second budget stage could never fail: the actual edge count cannot exceed rows times draws, and the page estimate does not fall as edges rise | A flaw in the contract, found while writing the cases | Contract decision 5 removed them, in the contract and in both implementations, so that no check exists that a test cannot reach |
| 2 | The first comparison did not cover node labels, which `docs/security.md` names as a risk | A gap in what was compared | A hash of every label was added to the contract, the core and the oracle, and a mutation of the padding is caught by it |
| 3 | The mutation that cut a label to its length was caught by a panel seed and by no hand-derived case | A thin spot in the cases | A case was added with a label length shorter than the label's own text, which the contract says is kept whole |
| 4 | The page model of the budget, written before any measurement, overestimated the page by about 3.7 times | A guess that the data corrected | The model was fitted to 38 measured pages and rounded up, and a browser test keeps it between the page and a third over it |
| 5 | The design of a data node's size was guessed at 84 by 22 for a cell, and Chrome draws about 50 by 36 | A guess that a geometry test corrected | The layout was set from measured sizes from 1 to 1,000 cells, which do not depend on the values |
| 6 | The first demo was unintelligible: the scene was too small to read and could not be panned or zoomed, its edges overlapped so that the aim of the demo was lost, and its controls were beside the scene and not in it | A failure of the design, found by review | The demo was rebuilt. It has a window of its own, two single columns for a graph that is made to be read, edges coloured by sign and sized by value, only a selected node's edges strong in a large graph, a click that writes out the arithmetic, and every list and button in a `<foreignObject>` of the scene's `<svg>` |
| 7 | The library places its toolbar from the rendered box of the scene's `<svg>`, which for a nested `<svg>` is the box of everything it draws, so the toolbar of a tall graph was centred off the screen | A limit of the library, found while building the fix | Not changed in the library. The window has no toolbar, and has its own Zoom in, Zoom out and Reset view buttons, with the library's own dragging of the background and Ctrl-wheel zoom turned on |
| 8 | A view box with a negative origin, used to centre the graph, moved the toolbar into the middle of it, since the library fixes the toolbar to the origin of the scene's space | The same limit | The view box begins at the origin, and the graph is moved by an offset in the layout |
| 9 | After edge labels were added, the mutation check reported a mutation as missed: the contents of the pattern square were tested by nothing | A gap in the tests, found by the check | A test now asserts that the square's path is one cell for each nonzero, in the order of the rows |
| 10 | The page estimate of the budget leaves out the labels on the edges of a small graph, so the demo's readout showed more elements than it had estimated | A limit that was shown and not explained | The readout says that the estimate leaves out the labels |
| 11 | In a graph of 1,000 entities 847 of the 1,000 nodes could not be clicked: the connectors are drawn after the nodes, so a dense graph covers them and a click lands on a connector, which does not register it. The cursor over an edge was an arrow and not the hand that says the background drags | A defect that no test had looked for, found by a user | The scene's connectors take no part in the pointer, by a rule in the stylesheet. Every node of 1,000, 5,000 and 10,000 entities is now under the pointer at its centre, and the smoke test clicks column 250 with a real mouse and drags from the top of an edge |
| 12 | The two boxes of figures and the grid of grey squares had no explanation, so a person could not tell what they were | A gap in the design, found by a user | Each has a caption under it in the scene, and a test checks what the caption says, that it is under its own box, and that it overlaps no node, no other box and no other caption |
| 13 | The matrix itself was not shown, so the values in the two vectors could not be read against anything, and nothing showed which values took part in a calculation | A gap in the design, found by a user | A graph of up to 24 by 24 draws A in full, and a selection lights its cells and the entries of x and y it uses; a step outlines what it changed. Chrome tests check each |
| 14 | A non-expert could not tell why x and y changed on a step, nor what the numbers on the connectors meant, since correct values were shown with no explanation | A gap in the design, found by a user | Each node shows its value beside it, a step is explained in words, and a caption says the connector numbers are the fixed matrix. Tests check each |
| 15 | The demo never stated the task, did not say how an input turns into an output, and a caption on the connectors was garbled | A gap in the design, found by a user | The task is stated in plain words, selecting an input writes out each sum it is a term of, and the connector caption gives a concrete example from the matrix. Tests check each |
| 16 | The window drew a graph with the numbers beside it, so that a person who did not know the task could not see it done, and x was drawn as a grid | A design that put the data before the idea, found by a user | A matrix up to 12 on a side is drawn as `A × x = y`, with x and y as columns and the operations of the selected row drawn as `MUL` and `ADD` nodes. Chrome tests check the layout, every row, a folded long row and the numbers after a step |
| 17 | A step changed an entry of x chosen at random, so the order looked like a mistake, a run changed the picture too fast to follow, the chain of operations was large, and y was not drawn as a graph | A gap in the design, found by a user | A worked matrix steps in order by default (the contract's random order stays for measurements), a run takes a step every 1.5 s, the operand nodes are plain with their names beside them, and y is drawn as bars. Tests check the order, the bars and what a step marks |
| 18 | The window did not say what the demo is for or how steps are ordered, in words a person who does not know the task can follow, and the speed of a run could not be changed | A gap in the design, found by a user | The readout opens with what the demo shows, how to read it, what a step does and why it matters, and the story line says how steps go. A Run speed list chooses slow, medium, fast or flat out. The smoke test checks the text and that fast takes more steps than slow |

No disagreement between the two implementations was found at any point of the build.
That is a statement about what was compared, and it needs two qualifications.

## Hand-derived cases

The 25 cases in `spmv/cases.json` have expectations derived by hand from the contract, not taken from either implementation.
They cover these.

- An empty matrix, a one-by-one matrix, and a matrix whose every nonzero is in one column.
- Steps with no updates, no steps at all, and a window as wide as the vector.
- A label longer than its length, and each malformed field.
- Each of the seven refusals, with its message and its place in the order.

Both implementations gave every expected outcome the first time.

A matrix written out by hand, with its product and its hash, is also pinned in both languages.
The oracle computed the hash and the Rust core asserts it, so the two agree on bytes that neither generated.

## Does the comparison see a bug?

`spmv/mutation-check.mjs` breaks the core and the scene in 43 small ways, one at a time, and the differential run or a test, native or in Chrome, has to notice each.
The record is `spmv/evidence/mutation-check.json`, and all 43 are caught.

| Part | Mutations | Found by |
|---|---|---|
| The matrix and its hashes | A kept duplicate, a hub draw, a sign, an unsorted row, a hash without values, a label's padding and its cut | The differential run, in a case or a panel seed |
| The vector and the product | A shifted start, an update beyond the window, a skipped nonzero, a changed entry that is not stored | The differential run |
| The budget | An exclusive limit, a degree that ignores columns, an edge bound without the draws | The differential run |
| The arithmetic of a node and the index of an edge | A plus sign for a negative term, a long row that hides what it left out, an index one too high | The native unit tests |
| The layout | Too little room for the data nodes, a small graph that is not two single columns | The tests in Chrome and the native unit tests |
| The scene | The wrong values in a data node, a missing edge, a shifted label, a host left in the page, a move that does not move, a missing or incomplete pattern square | The tests in Chrome |
| What a person reads | An edge's sign reversed, a selection that does not put out the last one, a column that loses its first edge | The tests in Chrome |

## What the agreement does not show

1. **The same author wrote both.** They share no code, and they can share a misreading of the contract.
   The hand-derived cases and the cross-language pins are the only checks that do not depend on the author.
2. **The contract is not the audit.** It is a reading of scenario D, and a reader of the audit who wanted another reading of "sparse matrix" would build another.
3. **The panel was seen by one person.** The smoke test checks what the panel says and leaves behind, and a screenshot was looked at, and nobody else has looked at it.
