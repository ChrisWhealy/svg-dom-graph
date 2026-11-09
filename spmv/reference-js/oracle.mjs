// The oracle of the sparse-matrix scale demonstrator, written from `spmv/docs/contract.md` and from nothing else. It keeps each row
// as a plain array of pairs, multiplies in the plainest way, and keeps the product incrementally with a search of the rows, so that
// it shares no structure with the Rust core's compressed form.
import { createHash } from 'node:crypto'
import { stream } from './prng.mjs'

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** A malformed specification, a refusal, or a disagreement of the product with itself. */
export class Failure extends Error {
  constructor(outcome, code, message) {
    super(message)
    this.outcome = outcome
    this.code = code
  }
}

const RANGES = [
  ['rows', () => 1, () => 1_000_000],
  ['cols', () => 1, () => 1_000_000],
  ['per_row', () => 0, () => 64],
  ['hub_cols', () => 0, s => s.cols],
  ['hub_percent', () => 0, () => 100],
  ['label_len', () => 1, () => 100_000],
  ['cells', () => 1, s => Math.min(s.rows, s.cols)],
  ['updates', () => 0, s => s.cells],
  ['steps', () => 0, () => 100_000]
]

/** Checks the ranges in the order of the table in section 1. */
export const validate = spec => {
  for (const [name, lo, hi] of RANGES) {
    const v = spec[name]
    if (!Number.isInteger(v) || v < 0)
      throw new Failure('invalid', 'invalid', `${name} is missing or is not a non-negative integer`)
    if (v < lo(spec) || v > hi(spec))
      throw new Failure('invalid', 'invalid', `${name} ${v} is outside ${lo(spec)} to ${hi(spec)}`)
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** The specification of a panel seed (section 7). */
export const panelSpec = seed => {
  const s = stream(seed, 7)
  const rows = s.between(1, 120)
  const cols = s.between(1, 120)
  const per_row = s.between(0, 8)
  const hub_cols = s.below(2) === 0 ? 0 : s.between(1, cols)
  const hub_percent = s.between(0, 100)
  const label_len = s.between(1, 40)
  const cells = s.between(1, Math.min(rows, cols))
  const updates = s.between(0, cells)
  const steps = s.between(1, 12)
  return { seed, rows, cols, per_row, hub_cols, hub_percent, label_len, cells, updates, steps }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const BUDGET_FIELDS = [
  'max_nodes',
  'max_edges',
  'max_degree',
  'max_cells_per_node',
  'max_total_cells',
  'max_label_len',
  'max_dom_elements',
  'dom_base',
  'dom_per_node',
  'dom_per_edge',
  'dom_per_cell'
]
const U64_MAX = (1n << 64n) - 1n

/** The estimate of the page's elements. BigInt, so that a sum saturates at the greatest u64 as the contract says. */
const estimate = (b, nodes, edges, cells) => {
  const sat = v => (v > U64_MAX ? U64_MAX : v)
  let total = BigInt(b.dom_base)
  total = sat(total + sat(BigInt(b.dom_per_node) * BigInt(nodes)))
  total = sat(total + sat(BigInt(b.dom_per_edge) * BigInt(edges)))
  total = sat(total + sat(BigInt(b.dom_per_cell) * BigInt(cells)))
  return total
}

const over = (code, what, value, limit) => {
  if (BigInt(value) > BigInt(limit))
    throw new Failure('refused', code, `${what} ${value} is over the budget of ${limit}`)
}

export const checkSpec = (spec, b) => {
  const nodes = spec.rows + spec.cols
  const cells = 2 * spec.cells
  const upper = BigInt(spec.rows) * BigInt(spec.per_row)
  over('nodes', 'nodes', nodes, b.max_nodes)
  over('cells', 'cells in a data node', spec.cells, b.max_cells_per_node)
  over('total_cells', 'cells in all data nodes', cells, b.max_total_cells)
  over('label_len', 'label length', spec.label_len, b.max_label_len)
  over('edges', 'edges', upper, b.max_edges)
  over('dom', 'estimated page elements', estimate(b, nodes, upper, cells), b.max_dom_elements)
}

export const checkMatrix = (spec, stats, b) => {
  const nodes = spec.rows + spec.cols
  const cells = 2 * spec.cells
  const degree = Math.max(stats.max_row_degree, stats.max_col_degree)
  over('degree', 'degree of a node', degree, b.max_degree)
  const est = estimate(b, nodes, stats.nnz, cells)
  return { nodes, edges: stats.nnz, cells, max_degree: degree, est_dom: Number(est) }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** The matrix as an array of rows, each an array of [column, value] sorted by column (sections 3 and 4). */
export const generate = spec => {
  const columns = stream(spec.seed, 1)
  const values = stream(spec.seed, 2)
  const rows = []
  for (let i = 0; i < spec.rows; i++) {
    const chosen = []
    for (let t = 0; t < spec.per_row; t++) {
      const u = columns.below(100)
      const c = u < spec.hub_percent && spec.hub_cols > 0 ? columns.below(spec.hub_cols) : columns.below(spec.cols)
      if (!chosen.includes(c)) chosen.push(c)
    }
    chosen.sort((a, b) => a - b)
    rows.push(
      chosen.map(c => {
        const v = values.between(1, 9)
        return [c, values.below(2) === 1 ? -v : v]
      })
    )
  }
  return rows
}

export const statistics = (rows, cols) => {
  const colDegree = new Array(cols).fill(0)
  let nnz = 0
  let maxRow = 0
  let emptyRows = 0
  for (const row of rows) {
    nnz += row.length
    maxRow = Math.max(maxRow, row.length)
    if (row.length === 0) emptyRows++
    for (const [c] of row) colDegree[c]++
  }
  return {
    nnz,
    max_row_degree: maxRow,
    max_col_degree: Math.max(0, ...colDegree),
    empty_rows: emptyRows,
    empty_cols: colDegree.filter(d => d === 0).length
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const i64 = v => {
  const b = Buffer.alloc(8)
  b.writeBigInt64LE(BigInt(v))
  return b
}
const u64 = v => {
  const b = Buffer.alloc(8)
  b.writeBigUInt64LE(BigInt(v))
  return b
}
const u32 = v => {
  const b = Buffer.alloc(4)
  b.writeUInt32LE(v)
  return b
}
const sha = parts => createHash('sha256').update(Buffer.concat(parts)).digest('hex')

export const matrixHash = (rows, cols) => {
  const nnz = rows.reduce((n, r) => n + r.length, 0)
  const parts = [Buffer.from([1]), u64(rows.length), u64(cols), u64(nnz)]
  for (const row of rows) {
    parts.push(u32(row.length))
    for (const [c, v] of row) parts.push(u64(c), i64(v))
  }
  return sha(parts)
}

export const vectorHash = v => sha([Buffer.from([1]), u64(v.length), ...v.map(i64)])

/** The node label of section 7. */
export const label = (kind, index, len) => `${kind}${index}`.padEnd(len, '-')

/** The hash of every node label: the columns' and then the rows', each followed by a line feed. */
export const labelsHash = spec => {
  const parts = []
  for (let j = 0; j < spec.cols; j++) parts.push(`${label('x', j, spec.label_len)}\n`)
  for (let i = 0; i < spec.rows; i++) parts.push(`${label('y', i, spec.label_len)}\n`)
  return createHash('sha256').update(parts.join('')).digest('hex')
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** The plainest product: every row, every pair. */
export const multiply = (rows, x) => rows.map(row => row.reduce((sum, [c, v]) => sum + v * x[c], 0))

/** Runs a scenario to its end and gives the value that the Rust core's `trace` gives. Throws a [`Failure`] if it cannot. */
export const trace = (spec, budget) => {
  validate(spec)
  checkSpec(spec, budget)
  const rows = generate(spec)
  const stats = statistics(rows, spec.cols)
  const plan = checkMatrix(spec, stats, budget)
  const s3 = stream(spec.seed, 3)
  const x = Array.from({ length: spec.cols }, () => s3.between(0, 198) - 99)
  let y = multiply(rows, x)
  const out = { x0: vectorHash(x), y0: vectorHash(y), steps: [] }
  const s4 = stream(spec.seed, 4)
  for (let step = 1; step <= spec.steps; step++) {
    for (let u = 0; u < spec.updates; u++) {
      const j = s4.below(spec.cells)
      const v = s4.between(0, 198) - 99
      // Keep the product incrementally: find every nonzero of column j by searching the rows.
      const delta = v - x[j]
      rows.forEach((row, i) => {
        for (const [c, value] of row) if (c === j) y[i] += value * delta
      })
      x[j] = v
    }
    const full = multiply(rows, x)
    if (full.some((v, i) => v !== y[i]))
      throw new Failure('internal', 'internal', `the incremental product differs from the full one at step ${step}`)
    out.steps.push({ step, x_sha256: vectorHash(x), y_sha256: vectorHash(y) })
  }
  return {
    outcome: 'ok',
    plan,
    stats,
    matrix_sha256: matrixHash(rows, spec.cols),
    labels_sha256: labelsHash(spec),
    x0_sha256: out.x0,
    y0_sha256: out.y0,
    steps: out.steps
  }
}

/** [`trace`] with a failure turned into the value that the Rust core reports. */
export const outcome = (spec, budget) => {
  try {
    return trace(spec, budget)
  } catch (error) {
    if (!(error instanceof Failure)) throw error
    return { outcome: error.outcome, code: error.code, message: error.message }
  }
}

export { BUDGET_FIELDS }
