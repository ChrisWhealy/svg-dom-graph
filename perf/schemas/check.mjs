// Checks the performance-harness schemas. Run it with `npm install && npm test` in this directory.
//
// Every valid example must pass, and every mutation in `rejected` must fail. A schema that is too permissive therefore
// fails this script, not just a malformed document. Phase 1 should reuse the same cases against the Rust validator.
import fs from 'node:fs'
import assert from 'node:assert/strict'
import Ajv from 'ajv/dist/2020.js'
import addFormats from 'ajv-formats'
import { parse as parseToml } from 'smol-toml'

const read = file => JSON.parse(fs.readFileSync(new URL(file, import.meta.url), 'utf8'))
// A condition names `properties` without a `type`, and a `then` requires properties that the parent schema defines.
// Ajv's strictTypes and strictRequired reject both, so those two checks are off. Every condition has a case below.
const ajv = new Ajv({
  strict: true,
  strictTypes: false,
  strictRequired: false,
  allErrors: true
})
addFormats(ajv)

const names = [
  'manifest',
  'checkpoint',
  'result',
  'comparison',
  'baseline-promotion',
  'workload-report',
  'gallery-report'
]
for (const name of names) ajv.addSchema(read(`${name}.schema.json`))
const validators = Object.fromEntries(names.map(name => [name, ajv.getSchema(`urn:svg-dom-graph:perf:${name}:1`)]))

const clone = value => structuredClone(value)

// A shared case is a list of operations on a path of keys: `set` puts a value there, and `delete` takes the key away.
const applyOps = (doc, ops) => {
  for (const op of ops) {
    const path = [...(op.set ?? op.delete)]
    const last = path.pop()
    const parent = path.reduce((object, key) => object[key], doc)
    if (op.set) parent[last] = op.value
    else delete parent[last]
  }
}
const valid = {
  // The parser's objects have another prototype, so a JSON round trip makes them comparable with the JSON examples.
  manifest: JSON.parse(
    JSON.stringify(parseToml(fs.readFileSync(new URL('manifest.example.toml', import.meta.url), 'utf8')))
  ),
  checkpoint: read('examples/checkpoint.json'),
  result: read('examples/result.json'),
  comparison: read('examples/comparison.json'),
  'baseline-promotion': read('examples/baseline-promotion.json'),
  'workload-report': read('examples/workload-report.json'),
  'gallery-report': read('examples/gallery-report.json')
}

let failures = 0
const report = (ok, message) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${message}`)
}

for (const name of names) {
  const check = validators[name]
  report(check(valid[name]), `valid ${name}${check.errors ? ' ' + ajv.errorsText(check.errors) : ''}`)
}

// A connector between boxes that touch has no path, so its coordinates are empty.
{
  const touching = clone(valid.checkpoint)
  touching.geometry.elements[1].coordinates = []
  const check = validators.checkpoint
  report(
    check(touching),
    `valid checkpoint with an undrawn connector${check.errors ? ' ' + ajv.errorsText(check.errors) : ''}`
  )
}

// A native run, such as Layer A, has no browser, and is a result like any other.
{
  const native = read('examples/result-native.json')
  const check = validators.result
  report(check(native), `valid native result${check.errors ? ' ' + ajv.errorsText(check.errors) : ''}`)
}

// The result's copy of its manifest must be the example manifest, so the two cannot drift apart.
try {
  assert.deepEqual(valid.result.manifest.canonical, valid.manifest)
  report(true, 'the result example carries the example manifest')
} catch {
  report(false, 'the result example carries the example manifest')
}

// Each case takes a copy of a valid document, breaks one thing, and must then be rejected.
const rejected = {
  // Shared with the Rust manifest reader, which refuses the same cases.
  manifest: Object.fromEntries(
    read('examples/rejected-manifests.json').cases.map(c => [c.label, d => applyOps(d, c.ops)])
  ),
  'gallery-report': {
    'an unknown top-level key': d => {
      d.extra = 1
    },
    'a report that does not say what its times are': d => {
      delete d.time_definition
    },
    'a report with no summary': d => {
      delete d.summary
    },
    'a summary that is missing a count': d => {
      delete d.summary.walked
    },
    'a panel with no status': d => {
      delete d.panels[0].status
    },
    'an unknown panel status': d => {
      d.panels[0].status = 'fine'
    },
    'a panel id that is not a panel': d => {
      d.panels[0].id = 'tree'
    },
    'a build with both a time and a reason': d => {
      d.panels[0].build = { ms: 1, unavailable: 'no' }
    },
    'a negative time': d => {
      d.panels[0].build = { ms: -1 }
    },
    'a walk with no verdict': d => {
      delete d.panels[1].walk.verdict
    },
    'an unknown verdict': d => {
      d.panels[1].walk.verdict = 'ok'
    },
    'a walk with no checks': d => {
      d.panels[1].walk.checks = []
    },
    'a check with an unknown outcome': d => {
      d.panels[1].walk.checks[0].outcome = 'ok'
    },
    'a check with no detail': d => {
      delete d.panels[1].walk.checks[0].detail
    },
    'an expectation with no reason': d => {
      delete d.panels[1].walk.expect.because
    },
    'an unwalked panel with no verdict': d => {
      delete d.panels[0].walk.verdict
    },
    'a downgraded walk with no reason': d => {
      delete d.panels[0].walk.reason
    },
    'a walked panel with no clicks': d => {
      delete d.panels[1].walk.clicks
    },
    'a walked panel with a reason': d => {
      d.panels[1].walk.reason = 'why'
    },
    'an unknown walk status': d => {
      d.panels[1].walk.status = 'ran'
    },
    'a control with no name': d => {
      d.panels[1].controls[0].name = ''
    },
    'a count that is not whole': d => {
      d.panels[0].counts.elements_after = 1.5
    },
    'a page error of an unknown kind': d => {
      d.page_errors = [{ kind: 'warning', text: 'x' }]
    },
    'a viewport of three numbers': d => {
      d.viewport = [1, 2, 3]
    }
  },
  checkpoint: {
    'a connector with two numbers': d => {
      d.geometry.elements[1].coordinates = [1, 2]
    },
    'no nodes list': d => {
      delete d.topology.nodes
    },
    'no edges list': d => {
      delete d.topology.edges
    },
    'no dom_identity': d => {
      delete d.dom_identity
    },
    'no geometry tolerance': d => {
      delete d.geometry.tolerance
    },
    'zero tolerance': d => {
      d.geometry.tolerance = 0
    },
    'unknown section': d => {
      d.extra = {}
    },
    'unknown key in topology': d => {
      d.topology.extra = 1
    },
    'node box with three numbers': d => {
      d.geometry.elements[0].coordinates = [0, 0, 1]
    },
    'label with four numbers': d => {
      d.geometry.elements[2].coordinates = [0, 0, 1, 1]
    },
    'duplicate paint-order key': d => {
      d.paint_order = ['a', 'a']
    },
    'zero view scale': d => {
      d.interaction.view.scale = 0
    },
    'view missing a field': d => {
      delete d.interaction.view.x
    },
    'a hash for geometry': d => {
      d.section_hashes.geometry = 'f'.repeat(64)
    },
    'a short hash': d => {
      d.section_hashes.topology = 'abc'
    },
    'an edge without an end': d => {
      delete d.topology.edges[0].to
    },
    'a non-string content value': d => {
      d.content.a = 1
    }
  },
  result: {
    'a gate with an unknown status': d => {
      d.gates = [{ name: 'g', status: 'maybe', detail: 'x' }]
    },
    'a gate with no detail': d => {
      d.gates = [{ name: 'g', status: 'pass' }]
    },
    'a gate with an unknown key': d => {
      d.gates = [{ name: 'g', status: 'pass', detail: 'x', typo: 1 }]
    },
    'a resolution flag of false': d => {
      d.metrics[0].resolution_limited = false
    },
    'a resolution flag on a count': d => {
      d.metrics[1].resolution_limited = true
    },
    'a resolution flag on a batch': d => {
      d.metrics[0].resolution_limited = true
      d.metrics[0].kind = 'batch_throughput'
      d.metrics[0].batch_size = 5
    },
    'no environment kind': d => {
      delete d.environment.kind
    },
    'an unknown environment kind': d => {
      d.environment.kind = 'cloud'
    },
    'a browser run with no browser': d => {
      delete d.environment.browser
    },
    'a browser run with no browser version': d => {
      delete d.environment.browser_version
    },
    'a browser run with no cross-origin flag': d => {
      delete d.environment.cross_origin_isolated
    },
    'an empty build profile': d => {
      d.environment.build_profile = ''
    },
    'no stream scope': d => {
      delete d.manifest.operation_stream_scope
    },
    'an unknown stream scope': d => {
      d.manifest.operation_stream_scope = 'everything'
    },
    'unknown top-level key': d => {
      d.typo = 1
    },
    'missing environment': d => {
      delete d.environment
    },
    'a short git sha': d => {
      d.environment.git_sha = 'abc'
    },
    'a major-only browser version': d => {
      d.environment.browser_version = '157'
    },
    'missing manifest copy': d => {
      delete d.manifest.canonical
    },
    'an invalid manifest copy': d => {
      d.manifest.canonical.execution.repetitions = 0
    },
    'a short manifest hash': d => {
      d.manifest.sha256 = 'abc'
    },
    'a measured metric with no unit': d => {
      delete d.metrics[0].unit
    },
    'a measured metric with no mechanism': d => {
      delete d.metrics[0].mechanism
    },
    'a latency with no samples': d => {
      delete d.metrics[0].samples
    },
    'a latency with no summary': d => {
      delete d.metrics[0].summary
    },
    'a latency with no within-run spread': d => {
      delete d.metrics[0].within_run
    },
    'a latency in bytes': d => {
      d.metrics[0].unit = 'bytes'
    },
    'a latency with a value': d => {
      d.metrics[0].value = 1
    },
    'a latency without p50': d => {
      delete d.metrics[0].summary.p50
    },
    'a latency batch size of one': d => {
      d.metrics[0].batch_size = 1
    },
    'a batch throughput without a batch size': d => {
      d.metrics[0].kind = 'batch_throughput'
    },
    'a count with no value': d => {
      delete d.metrics[1].value
    },
    'a count with a summary': d => {
      d.metrics[1].summary = { min: 1, p50: 1, max: 1 }
    },
    'a count in ms': d => {
      d.metrics[1].unit = 'ms'
    },
    'a capacity in counts': d => {
      d.metrics[2].unit = 'count'
    },
    'a capacity with a summary': d => {
      d.metrics[2].summary = { min: 1, p50: 1, max: 1 }
    },
    'a capacity with no value': d => {
      delete d.metrics[2].value
    },
    'a capacity with a batch size': d => {
      d.metrics[2].batch_size = 2
    },
    'an unavailable metric with no reason': d => {
      delete d.metrics[5].reason
    },
    'an unavailable metric with a value': d => {
      d.metrics[5].value = 1
    },
    'an unavailable metric with a summary': d => {
      d.metrics[5].summary = { min: 1, p50: 1, max: 1 }
    },
    'a measured metric with a reason': d => {
      d.metrics[0].reason = 'x'
    },
    'an unknown metric key': d => {
      d.metrics[0].typo = 1
    },
    'ok with an error': d => {
      d.error = 'x'
    },
    'error without text': d => {
      d.status = 'error'
    },
    'timed out without text': d => {
      d.status = 'timed_out'
    },
    'invalid_incorrect without failed checkpoints': d => {
      d.status = 'invalid_incorrect'
    },
    'correctness without checkpoint counts': d => {
      d.mode = 'correctness'
      delete d.raw_samples
    },
    'performance ok without raw samples': d => {
      delete d.raw_samples
    },
    'a non-date start': d => {
      d.started_at = 'yesterday'
    }
  },
  'workload-report': {
    'unknown top-level key': d => {
      d.typo = 1
    },
    'no stream': d => {
      delete d.stream
    },
    'an unsupported encoding version': d => {
      d.stream.encoding_version = 2
    },
    'a short stream digest': d => {
      d.stream.sha256 = 'abc'
    },
    'no mutations digest': d => {
      delete d.stream.mutations_sha256
    },
    'no operations': d => {
      d.stream.operations = 0
    },
    'no checkpoints': d => {
      d.checkpoints = []
    },
    'a checkpoint with no model digest': d => {
      delete d.checkpoints[0].model_sha256
    },
    'a checkpoint with no section digests': d => {
      delete d.checkpoints[0].section_sha256
    },
    'a checkpoint missing one section digest': d => {
      delete d.checkpoints[0].section_sha256.topology
    },
    'a digest for geometry, which is compared within a tolerance': d => {
      d.checkpoints[0].section_sha256.geometry = 'f'.repeat(64)
    },
    'a negative operation index': d => {
      d.checkpoints[0].op_index = -1
    },
    'an invalid manifest copy': d => {
      d.manifest.canonical.execution.repetitions = 0
    },
    'no section report': d => {
      delete d.sections
    },
    'an unknown key in the manifest block': d => {
      d.manifest.typo = 1
    }
  },
  'baseline-promotion': {
    'an unknown key': d => {
      d.typo = 1
    },
    'no reason': d => {
      delete d.reason
    },
    'a short commit': d => {
      d.promotion.commit = 'abc'
    },
    'no record of what it replaced': d => {
      delete d.previous
    },
    'an accepted regression with no reason': d => {
      d.accepted_regression = { metrics: ['mutation.move_node'] }
    },
    'an accepted regression of no metric': d => {
      d.accepted_regression = { reason: 'x', metrics: [] }
    },
    'no runs': d => {
      d.runs = []
    },
    'a method that is not this one': d => {
      d.comparison_method.statistic = 'mean'
    }
  },
  comparison: {
    'unknown top-level key': d => {
      d.typo = 1
    },
    'a gating block that is not one of the two states': d => {
      d.gating.statistical_gates = 'maybe'
    },
    'no gating block': d => {
      delete d.gating
    },
    'a browser version that is neither a version nor native': d => {
      d.compatibility.key.browser_version = 'latest'
    },
    'no baseline promotion record': d => {
      delete d.baseline.promotion
    },
    'a short baseline commit': d => {
      d.baseline.git_sha = 'abc'
    },
    'no candidate runs': d => {
      d.candidate.run_ids = []
    },
    'a repeated run id': d => {
      d.candidate.run_ids = ['r1', 'r1']
    },
    'too few bootstrap resamples': d => {
      d.method.bootstrap_resamples = 10
    },
    'no ratio tolerance': d => {
      delete d.method.ratio_tolerance
    },
    'confidence of one': d => {
      d.method.confidence = 1
    },
    'a regression with no ratio': d => {
      delete d.comparisons[0].ratio
    },
    'a regression with no interval': d => {
      delete d.comparisons[0].ratio_interval
    },
    'a regression with no candidate side': d => {
      delete d.comparisons[0].candidate
    },
    'an inconclusive verdict with no reason': d => {
      d.comparisons[0].verdict = 'inconclusive'
    },
    'a diagnostic verdict with an interval': d => {
      d.comparisons[0].verdict = 'diagnostic'
      d.comparisons[0].reason = 'x'
    },
    'a not_comparable verdict carrying numbers': d => {
      d.comparisons[0].verdict = 'not_comparable'
      d.comparisons[0].reason = 'x'
    },
    'comparable false with no reason': d => {
      d.compatibility.comparable = false
    },
    'incompatible yet a regression verdict': d => {
      d.compatibility.comparable = false
      d.compatibility.reason = 'x'
    },
    'a side with no between-run spread': d => {
      delete d.comparisons[0].baseline.between_run
    },
    'an unknown verdict': d => {
      d.comparisons[0].verdict = 'worse'
    },
    'a missing compatibility key field': d => {
      delete d.compatibility.key.cpu
    }
  }
}

for (const [name, cases] of Object.entries(rejected)) {
  for (const [label, mutate] of Object.entries(cases)) {
    const doc = clone(valid[name])
    mutate(doc)
    report(!validators[name](doc), `${name} rejects ${label}`)
  }
}

// An incompatible pair must be accepted when every verdict is not_comparable and carries no numbers.
{
  const doc = clone(valid.comparison)
  doc.compatibility.comparable = false
  doc.compatibility.reason = 'the browser version differs'
  doc.comparisons = [
    {
      metric: 'mutation.move_node',
      layer: 'B',
      statistic: 'p50',
      verdict: 'not_comparable',
      reason: 'the browser version differs'
    }
  ]
  report(validators.comparison(doc), 'comparison accepts an incompatible pair with only not_comparable verdicts')
}

// The service target of the recovery workload has no graph, and it is told from the graph target by one key. Its manifests are read as the
// graph target's are, and each way of getting the pair of target and points wrong is refused.
{
  const read = name =>
    JSON.parse(JSON.stringify(parseToml(fs.readFileSync(new URL(`../manifests/${name}`, import.meta.url), 'utf8'))))
  const service = read('w5-service-recovery-small.toml')
  const graph = read('w5-recovery-small.toml')
  const check = validators.manifest
  report(check(service), `valid service recovery manifest${check.errors ? ' ' + ajv.errorsText(check.errors) : ''}`)
  report(check(graph), 'valid graph recovery manifest, which names no target')
  const refuses = (label, doc) => report(!check(doc), `manifest rejects ${label}`)
  const edit = (base, ops) => {
    const doc = clone(base)
    applyOps(doc, ops)
    return doc
  }
  refuses('a mistyped target', edit(service, [{ set: ['recovery', 'target'], value: 'service_simulator' }]))
  refuses('the service target with no scenario', edit(service, [{ delete: ['recovery', 'scenario'] }]))
  refuses('an empty scenario', edit(service, [{ set: ['recovery', 'scenario'], value: '' }]))
  refuses(
    'a graph point on the service target',
    edit(service, [{ set: ['recovery', 'points', 0, 'point'], value: 'node_construction' }])
  )
  refuses(
    'a service point on the graph target',
    edit(graph, [{ set: ['recovery', 'points', 0, 'point'], value: 'service_overlay_write' }])
  )
  refuses('a scenario on the graph target', edit(graph, [{ set: ['recovery', 'scenario'], value: 'E0' }]))
  refuses('a graph with the service target', edit(service, [{ set: ['graph'], value: { nodes: 4, edges: 3 } }]))
  refuses('the graph target with no graph', edit(graph, [{ delete: ['graph'] }]))
  refuses('a mistyped key of the recovery table', edit(service, [{ set: ['recovery', 'tarjet'], value: 'graph' }]))
}

// Workload W6 is the simulator run cleanly. It has no graph and no checkpoint policy, and a [simulation] table that is its own, and a
// manifest of any other workload is told from it by the workload key.
{
  const read = name =>
    JSON.parse(JSON.stringify(parseToml(fs.readFileSync(new URL(`../manifests/${name}`, import.meta.url), 'utf8'))))
  const simulation = read('w6-service-simulation-smoke.toml')
  const check = validators.manifest
  report(check(simulation), `valid simulator manifest${check.errors ? ' ' + ajv.errorsText(check.errors) : ''}`)
  const refuses = (label, doc) => report(!check(doc), `manifest rejects ${label}`)
  const edit = (base, ops) => {
    const doc = clone(base)
    applyOps(doc, ops)
    return doc
  }
  refuses('the simulator workload with no table', edit(simulation, [{ delete: ['simulation'] }]))
  refuses(
    'a simulation table on another workload',
    edit(simulation, [{ set: ['workload'], value: 'scene_construction' }])
  )
  refuses('a graph with the simulator workload', edit(simulation, [{ set: ['graph'], value: { nodes: 4 } }]))
  refuses(
    'a checkpoint policy with the simulator workload',
    edit(simulation, [{ set: ['checkpoint'], value: { view: 'unchanged' } }])
  )
  refuses('a mistyped key of the simulation table', edit(simulation, [{ set: ['simulation', 'batches'], value: 4 }]))
  refuses('a simulation table with no queue capacity', edit(simulation, [{ delete: ['simulation', 'queue_capacity'] }]))
  refuses('a zero pop minimum', edit(simulation, [{ set: ['simulation', 'min_pops'], value: 0 }]))
  refuses('too few arrivals', edit(simulation, [{ set: ['simulation', 'arrivals'], value: 10 }]))
  refuses('too many arrivals', edit(simulation, [{ set: ['simulation', 'arrivals'], value: 2000000 }]))
  report(
    check(edit(simulation, [{ set: ['simulation', 'arrivals'], value: 36000 }])),
    'valid scaled simulator manifest'
  )
}

console.log(failures ? `\n${failures} failed` : '\nall passed')
process.exit(failures ? 1 : 0)
