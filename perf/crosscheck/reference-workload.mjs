// A second, independent implementation of the workload generator, for checking `perf-harness`.
//
// It is written from the same description but shares no code with the Rust crate: another language, 64-bit values as
// BigInt, and Node's own SHA-256. If both produce the same operation count and digests for a manifest, the stream does not
// depend on the language, the platform or the integer width that produced it.
//
// It reads a workload report that `perf-harness generate` wrote, regenerates the stream from the report's manifest, and
// compares the counts and digests.
//
//   node perf/crosscheck/reference-workload.mjs report.json
import crypto from 'node:crypto'
import fs from 'node:fs'

const MASK = (1n << 64n) - 1n

class SplitMix64 {
  constructor(seed) {
    this.state = BigInt(seed) & MASK
  }

  next() {
    this.state = (this.state + 0x9e3779b97f4a7c15n) & MASK
    let z = this.state
    z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & MASK
    z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & MASK
    return z ^ (z >> 31n)
  }

  // A number in 0..n, redrawing when the draw falls in the short tail that would make some values likelier.
  below(n) {
    const bound = BigInt(n)
    const limit = MASK - (MASK % bound)
    for (;;) {
      const draw = this.next()
      if (draw < limit) return Number(draw % bound)
    }
  }
}

const derive = (seed, stream) => new SplitMix64(new SplitMix64(BigInt(seed) ^ ((BigInt(stream) * 0xd6e8feb86659fd93n) & MASK)).next())

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const NODE_W = 80
const NODE_H = 36
const MARGIN = 20
const PITCH_X = 140
const PITCH_Y = 90

const planGraph = (prng, nodeCount, edgeCount, maxDegree) => {
  let columns = 1
  while (columns * columns < nodeCount) columns++
  const rows = Math.ceil(nodeCount / columns)
  const nodes = Array.from({ length: nodeCount }, (_, i) => ({
    key: `n${i}`,
    x: MARGIN + (i % columns) * PITCH_X,
    y: MARGIN + Math.floor(i / columns) * PITCH_Y,
  }))
  const limit = maxDegree ?? Infinity
  const degree = new Array(nodeCount).fill(0)
  const seen = new Set()
  const edges = []
  let attempts = 0
  const allowed = 100 * edgeCount + 1000
  while (edges.length < edgeCount) {
    if (++attempts > allowed) throw new Error('the edges could not be placed')
    const from = prng.below(nodeCount)
    const to = prng.below(nodeCount)
    if (from === to || degree[from] >= limit || degree[to] >= limit || seen.has(`${from},${to}`)) continue
    seen.add(`${from},${to}`)
    degree[from]++
    degree[to]++
    edges.push([from, to])
  }
  let hub = 0
  for (let i = 0; i < nodeCount; i++) if (degree[i] > degree[hub]) hub = i
  return { nodes, edges, fieldW: MARGIN + columns * PITCH_X, fieldH: MARGIN + rows * PITCH_Y, hub }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The canonical encoding: a tag byte for each value, little-endian fixed-width integers, and strings behind a u32 length.
const u64 = value => {
  const b = Buffer.alloc(9)
  b[0] = 0x01
  b.writeBigUInt64LE(BigInt(value), 1)
  return b
}
const i64 = value => {
  const b = Buffer.alloc(9)
  b[0] = 0x02
  b.writeBigInt64LE(BigInt(value), 1)
  return b
}
const str = value => {
  const bytes = Buffer.from(value, 'utf8')
  const head = Buffer.alloc(5)
  head[0] = 0x03
  head.writeUInt32LE(bytes.length, 1)
  return Buffer.concat([head, bytes])
}
const op = (code, ...parts) => ({ code, bytes: Buffer.concat([Buffer.from([code]), ...parts]) })

const ops = {
  createScene: () => op(1),
  dropScene: () => op(2),
  removeHost: () => op(3),
  addNode: (key, x, y, w, h, label) => op(4, str(key), i64(x), i64(y), i64(w), i64(h), str(label)),
  addEdge: (from, to) => op(5, str(from), str(to)),
  moveNode: (key, x, y) => op(6, str(key), i64(x), i64(y)),
  phase: measure => op(7, u64(measure ? 1 : 0)),
  checkpoint: index => op(8, u64(index)),
}
const isControl = o => o.code === 7 || o.code === 8

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
export const generate = manifest => {
  const { seed, workload, graph, execution, measurement } = manifest
  const warmup = execution.warmup_iterations
  const total = warmup + execution.measurement_iterations
  const every = measurement.checkpoint_every ?? null
  const out = []
  let nextCheckpoint = 0
  let lastWasCheckpoint = false
  const emit = o => {
    out.push(o)
    lastWasCheckpoint = o.code === 8
  }
  const checkpoint = () => emit(ops.checkpoint(nextCheckpoint++))
  const build = plan => {
    for (const n of plan.nodes) emit(ops.addNode(n.key, n.x, n.y, NODE_W, NODE_H, n.key))
    for (const [from, to] of plan.edges) emit(ops.addEdge(plan.nodes[from].key, plan.nodes[to].key))
  }
  const marker = step => {
    if (step === 0 && warmup > 0) emit(ops.phase(false))
    if (step === warmup) emit(ops.phase(true))
  }
  const planFor = stream => planGraph(derive(seed, stream), graph.nodes, graph.edges ?? 0, graph.maximum_degree ?? null)

  if (workload === 'connector_rerouting') {
    const plan = planFor(0)
    const prng = derive(seed, 1)
    emit(ops.createScene())
    build(plan)
    checkpoint()
    for (let step = 0; step < total; step++) {
      marker(step)
      const aimAtHub = prng.below(4) === 0
      const any = prng.below(plan.nodes.length)
      const x = prng.below(plan.fieldW - NODE_W + 1)
      const y = prng.below(plan.fieldH - NODE_H + 1)
      emit(ops.moveNode(plan.nodes[aimAtHub ? plan.hub : any].key, x, y))
      if (every !== null && (step + 1) % every === 0) checkpoint()
    }
    if (!lastWasCheckpoint) checkpoint()
  } else {
    const fixed = workload === 'scene_construction' ? planFor(0) : null
    for (let cycle = 0; cycle < total; cycle++) {
      marker(cycle)
      const plan = fixed ?? planFor(0x1000 + cycle)
      emit(ops.createScene())
      build(plan)
      if ((every !== null && (cycle + 1) % every === 0) || cycle + 1 === total) checkpoint()
      emit(ops.dropScene())
      emit(ops.removeHost())
    }
    checkpoint()
  }
  return out
}

export const digests = stream => {
  const full = crypto.createHash('sha256').update(Buffer.from([1]))
  const mutations = crypto.createHash('sha256').update(Buffer.from([1]))
  let mutationCount = 0
  for (const o of stream) {
    full.update(o.bytes)
    if (!isControl(o)) {
      mutations.update(o.bytes)
      mutationCount++
    }
  }
  return { operations: stream.length, mutations: mutationCount, sha256: full.digest('hex'), mutations_sha256: mutations.digest('hex') }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// A seed can be larger than 2**53, which a JSON number read as a double cannot hold. The reviver takes it from the source text.
export const readReport = text => JSON.parse(text, (key, value, context) => (key === 'seed' ? BigInt(context.source) : value))

export const compare = report => {
  const mine = digests(generate(report.manifest.canonical))
  const theirs = report.stream
  const differences = Object.keys(mine).filter(key => mine[key] !== theirs[key])
  return { mine, differences }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const report = readReport(fs.readFileSync(process.argv[2], 'utf8'))
  const { mine, differences } = compare(report)
  console.log(JSON.stringify(mine))
  process.exit(differences.length ? 1 : 0)
}
