# The sparse-matrix contract

This page fixes what the Rust core and the JavaScript oracle both implement.
It is written from the coverage audit of 4 November 2026, scenario D, and from `docs/security.md`, and it is written before either implementation.
Where the audit leaves a choice open, the choice is made here and numbered.

A scenario is a sparse matrix A of R rows and C columns, a vector x of C integers, and a stream of updates to x.
Each step applies the updates and recomputes y = A·x.
The bipartite dependency graph has one node for each column and each row, and one edge from column j to row i for each nonzero A(i,j).

## 1. The specification

| Field | Meaning | Allowed |
|---|---|---|
| `seed` | The seed of every draw | Any `u64` |
| `rows` R | Rows, which are output nodes | 1 to 1,000,000 |
| `cols` C | Columns, which are input nodes | 1 to 1,000,000 |
| `per_row` k | Draws of a column for each row | 0 to 64 |
| `hub_cols` H | The first H columns are hubs | 0 to C |
| `hub_percent` P | The chance in percent that a draw is a hub | 0 to 100 |
| `label_len` L | The length of a node's label | 1 to 100,000 |
| `cells` N | Cells in each of the two data nodes | 1 to the smaller of R and C |
| `updates` U | Entries of x changed in a step | 0 to N |
| `steps` | Steps after the initial one | 0 to 100,000 |

A specification outside these ranges is **invalid**, with the message `NAME V is outside LO to HI`, where NAME is the field's name in the table and V its value.
The checks are made in the order of the table.
An invalid specification is a malformed one, and it is refused for that reason and not for a budget.

## 2. The streams

A named stream is derived from the seed as the harness derives it, and as the simulator's contract does (`dess/s1/docs/contract.md`).
Stream n is SplitMix64 seeded with the first output of SplitMix64 seeded with `seed XOR (n * 0xD6E8FEB86659FD93)`, all in wrapping 64-bit arithmetic.
`below(n)` is a draw in 0 to n−1 with no modulo bias: a draw in the short last interval of the range is thrown away.
`between(lo, hi)` is `lo + below(hi − lo + 1)`.

| Stream | Used for |
|---|---|
| 1 | The columns of the nonzeros |
| 2 | The values of the nonzeros |
| 3 | The initial vector x |
| 4 | The updates |
| 7 | The specification of a panel seed (section 7) |

## 3. The matrix

For each row i in order, from stream 1: for t from 0 to k−1, draw `u = below(100)`, then draw a column.
If u < P and H > 0 the column is `below(H)`.
Otherwise it is `below(C)`.
Both draws are made for every t, so the stream advances the same way whatever the outcome.
A column that is already in the row's list is dropped, and the first draw of it is kept.
The row's list is then sorted ascending.

The values are drawn from stream 2, row by row, and in ascending column order within a row.
For each nonzero draw `v = between(1, 9)` and then `s = below(2)`.
The value is −v if s is 1, and v otherwise.
No value is ever zero, and no row has more than k nonzeros.

**Decision 1.** Duplicates are dropped and not redrawn.
A redraw would make the number of draws depend on the data, and the audit asks for a fixed workload.

## 4. The vector and the updates

The initial vector is `x(j) = between(0, 198) − 99` for j from 0 to C−1, drawn in order from stream 3.

A step applies U updates in order, from stream 4.
Each is `j = below(N)` and then `v = between(0, 198) − 99`, and sets `x(j) = v`.
A later update of the same j in the same step replaces the earlier one.

**Decision 2.** An update changes only the first N entries of x, which are the ones the x data node shows.
Every update is therefore visible, and the update rate is U cells a step.
An update may set the value that the entry already has.

**Presentation order.** The window that shows the multiplication can ask a run for `InOrder` updates.
Then `j` is x0, x1 and so on round the first N entries, and `v` is still drawn from stream 4.
That order is for a person watching.
The oracle, the hashes and every measurement use the order above.

## 5. The product

`y(i)` is the sum over the nonzeros of row i of `value × x(column)`, as a signed 64-bit integer.
With |value| ≤ 9, |x| ≤ 99 and k ≤ 64 the greatest magnitude is 57,024, so no sum can overflow.

**Decision 3.** All arithmetic is exact integer arithmetic.
The audit asks for the computed values to be compared with an oracle, and an integer product needs no tolerance.
A floating-point variant would need a rounding policy and a tolerance, and is out of scope.

An implementation also keeps y incrementally.
For an update that sets x(j) to a new value, it adds `A(i,j) × (new − old)` to y(i) for every nonzero in column j.
The incremental y has to equal a full recomputation after every step, and an implementation that disagrees with itself reports an internal error.

## 6. The canonical bytes and the hashes

All integers are little-endian.
A matrix is `u8 1`, `u64 R`, `u64 C`, `u64 nnz`, and then for each row `u32 count` followed by `count` pairs of `u64 column` and `i64 value`.
A vector is `u8 1`, `u64 length`, and the entries as `i64`.
A hash is the SHA-256 of the bytes, as lower-case hexadecimal.

| Name | Hash of |
|---|---|
| `matrix_sha256` | The matrix |
| `x0_sha256`, `y0_sha256` | The initial vectors x and y |
| `x_sha256`, `y_sha256` of a step | The vectors after that step's updates |
| `labels_sha256` | The labels of section 7: the column labels in order, then the row labels in order, each followed by a line feed |

The statistics are `nnz`, `max_row_degree`, `max_col_degree`, `empty_rows` and `empty_cols`, where a degree is a count of nonzeros.

## 7. Labels, and the specification of a panel seed

A node's label is its kind and index, `x` and the column for a column node and `y` and the row for a row node, written in decimal.
It is padded on the right with `-` up to L characters.
A label whose text is already longer than L is not cut.
Every label is ASCII.

A panel seed makes a small specification, for the differential run, with stream 7 in this order:

| Field | Draw |
|---|---|
| R, C | `between(1, 120)` each |
| k | `between(0, 8)` |
| H | 0 if `below(2)` is 0, otherwise `between(1, C)` |
| P | `between(0, 100)` |
| L | `between(1, 40)` |
| N | `between(1, min(R, C))` |
| U | `between(0, N)` |
| steps | `between(1, 12)` |

The panel's `seed` is the seed itself.

## 8. Budgets

The audit requires budgets to be enforced before a hostile or oversized scene is built, and `docs/security.md` lists them.
A valid specification is checked against a budget in two stages, and the checks of each stage are made in the order given.
The first stage needs only the specification.
The second needs the generated matrix, and is made before any scene call.

A budget has seven limits and a model of the page: `max_nodes`, `max_edges`, `max_degree`, `max_cells_per_node`, `max_total_cells`, `max_label_len`, `max_dom_elements`, and `dom_base`, `dom_per_node`, `dom_per_edge` and `dom_per_cell`.
The estimate of the page's elements is `dom_base + dom_per_node × nodes + dom_per_edge × edges + dom_per_cell × cells`, where nodes is R + C and cells is 2N.

| Stage | Check | Code | Message |
|---|---|---|---|
| 1 | R + C over `max_nodes` | `nodes` | `nodes V is over the budget of LIMIT` |
| 1 | N over `max_cells_per_node` | `cells` | `cells in a data node V is over the budget of LIMIT` |
| 1 | 2N over `max_total_cells` | `total_cells` | `cells in all data nodes V is over the budget of LIMIT` |
| 1 | L over `max_label_len` | `label_len` | `label length V is over the budget of LIMIT` |
| 1 | R × k over `max_edges` | `edges` | `edges V is over the budget of LIMIT` |
| 1 | The estimate with R × k edges over `max_dom_elements` | `dom` | `estimated page elements V is over the budget of LIMIT` |
| 2 | The larger of the greatest row and column degree over `max_degree` | `degree` | `degree of a node V is over the budget of LIMIT` |

A refusal is not an error of the specification.
It is reported separately from a successful run, as the audit requires, with its code and its message.
All sums saturate at the greatest `u64`.

**Decision 4.** The first stage uses R × k as an upper bound on the edges.
A specification which could only produce too many edges is then refused before any matrix is generated.

**Decision 5.** The second stage checks only the degree.
The actual edge count cannot exceed R × k, and the estimate does not fall as edges rise, so neither could fail there once the first stage had passed.
A check that cannot fail cannot be tested, and is not made.
The plan that a passing scenario reports uses the actual edge count.
