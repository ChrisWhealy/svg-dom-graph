// The recovery runner's sweeps, in the page, next to the library they break.
//
// For each fault point that can be reached by failing a DOM call, an operation is run once without a fault to learn how many
// DOM calls it makes and what the scene looks like when it succeeds. It is then run again once for every one of those calls,
// with that call made to fail (`perf/page/fault-injector.js`), and what is left behind is compared with what the library says
// it leaves behind (`perf/fault-contracts.json`). So the failure is put at every place that the operation can have one, and not
// at a few that were thought of.
//
// Three states are compared for each failure, as checkpoints and as the document itself:
//   before  the scene as it was, read back from the document;
//   after   the scene when the failed call has returned;
//   ref     the scene of a twin that ran the same operation with no fault, which is what a recovery has to reach.
// The reference model is asked too: the twin has to agree with it, so that `ref` is not just the library agreeing with itself.
//
// A point whose contract is `undocumented` is not tested against a guess. What was seen is recorded, as a finding.
//
// Nothing is timed inside a check. A recovery time is taken around the calls that recover, with the clock read before the first
// and after the last, and the snapshots that compare states are taken after it.

const parse = JSON.parse
const OPEN = 'perf-recovery-'
// The DOM calls whose bindings return an object. When one of them throws, wasm-bindgen's glue has already put its result in the
// table of externrefs, and nothing frees the slot, so each failure of one costs a slot for good. The ones that return nothing
// (setAttribute, addEventListener) do not. See the finding that `runRecovery` reports, which measures it.
const RETURNS_AN_OBJECT = new Set(['createElementNS', 'createElement', 'appendChild', 'insertBefore', 'getBBox'])
const methodOf = row => (row.call ?? '').split(' ')[0]

/** What an edge-construction failure changed, in words, from what was observed of it. */
const leftBy = observed => [
  ...(observed.sections_changed ?? []),
  ...(observed.checkpoint_readable === false ? ['a scene that could not be read back'] : []),
  ...(observed.extra_elements ? [`${observed.extra_elements} extra elements`] : []),
  ...(observed.paths_changed ? ['the connector paths'] : []),
  ...(observed.ends_named ? [`the names of ${observed.ends_named} of the ends`] : [])
]

export const makeRecovery = (h, now, testing = {}) => {
  const faults = () => window.__faults
  const call = (fn, ...args) => parse(fn(...args))
  const act = (slot, op) => call(h.recovery_act, slot, JSON.stringify(op))
  const intend = (slot, op) => call(h.recovery_intend, slot, JSON.stringify(op))
  const snapshot = (slot, name) => call(h.recovery_snapshot, slot, name)
  const differences = (expected, actual) => call(h.recovery_differences, expected, actual)

  // The document itself: what a checkpoint does not read. Counts of elements and of listeners, the positions of the labels,
  // the connectors' paths, and the style of every node, so that a stray element, a dangling listener or a half-written label
  // shows even though the model has no place for it.
  const raw = slot => {
    const svg = document.getElementById(OPEN + slot)
    if (!svg) return null
    const group = svg.querySelector('g.svg-dom-graph-content')
    return {
      elements: svg.querySelectorAll('*').length,
      listeners: window.__listeners?.snapshot().total ?? null,
      labels: [...svg.querySelectorAll('[data-svg-dom-graph-edge-label]')]
        .map(label => label.getAttribute('transform'))
        .join('|'),
      paths: [...svg.querySelectorAll('path')].map(path => path.getAttribute('d')).join('|'),
      styles: [...(group?.children ?? [])].map(child => child.getAttribute('style') ?? '').join('|'),
      // What the nodes with content write: the text of every cell and every accessible name, and the state of the nested scene's own host.
      texts: [...svg.querySelectorAll('text')].map(text => text.textContent).join('|'),
      names: [...(group?.children ?? [])].map(child => child.getAttribute('aria-label') ?? '').join('|'),
      hosts: [...document.querySelectorAll(`svg[id^="${OPEN}${slot}-child"]`)]
        // What the host does, and not how it is written: hiding it and showing it again leaves a style that was not there, and a person sees the same.
        .map(
          host =>
            `${getComputedStyle(host).visibility}/${getComputedStyle(host).display}/${host.getAttribute('aria-hidden') ?? ''}`
        )
        .join('|')
    }
  }
  const FIELDS = ['elements', 'listeners', 'labels', 'paths', 'styles']
  const MORE_FIELDS = [...FIELDS, 'texts', 'names', 'hosts']
  const sameRaw = (a, b, fields = FIELDS) => fields.every(field => a[field] === b[field])
  const rawDifferences = (a, b, fields = FIELDS) => fields.filter(field => a[field] !== b[field])

  /** The accessible names of the nodes called `keys` on `slot`, which is where a relationship clause is written. */
  const endNames = (slot, keys) => {
    const groups = [...document.getElementById(OPEN + slot).querySelectorAll('g.svg-dom-graph-content > g')].filter(
      g => !g.hasAttribute('data-svg-dom-graph-edge-label')
    )
    return keys.map(
      key => groups.find(g => g.querySelector('text')?.textContent === key)?.getAttribute('aria-label') ?? ''
    )
  }

  const build = (manifest, slot, labelled) => {
    const built = call(h.recovery_build, JSON.stringify(manifest), slot, labelled)
    if (!built.ok) throw new Error(`building slot ${slot}: ${built.error}`)
    return built
  }

  // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
  /** Runs `op` on `slot` with no fault and counts and describes the DOM calls it made. */
  const dryRun = (slot, op) => {
    faults().record(true)
    const result = act(slot, op)
    faults().record(false)
    return { result, calls: faults().take() }
  }

  /** Runs `op` on `slot` with the nth DOM call made to fail. Gives the result, the call that failed, and how long it took. */
  const faulted = (slot, op, index) => {
    faults().arm(index)
    // For the self-test only: removal that does nothing, for the failed call alone, so that the harness can still clean up after it.
    if (testing.suppressRemoval) faults().suppressRemoval(true)
    const started = now()
    const result = act(slot, op)
    const elapsed = now() - started
    faults().suppressRemoval(false)
    const fired = faults().disarm()
    return { result, fired: fired?.fired ?? null, counted: fired?.counted ?? 0, elapsed }
  }

  /** Runs `op` on `slot` as the recovery, and gives how long it took. */
  const retried = (slot, op) => {
    const started = now()
    const result = act(slot, op)
    return { result, elapsed: now() - started }
  }

  const phaseOf = fired => {
    if (!fired) return 'none'
    if (fired.method === 'setAttribute' && fired.name === 'transform' && fired.tag === 'g' && fired.label)
      return 'label_update'
    if (fired.method === 'setAttribute' && fired.name === 'transform' && fired.tag === 'g') return 'dom_transform'
    if (fired.tag === 'path') return 'connector_update'
    return 'other'
  }
  const describeCall = call =>
    `${call.method} ${call.tag}${call.name ? ` ${call.name}` : ''}${call.label ? ' (a label)' : ''}`

  // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
  // One row per failure, and what to say about it.
  const row = (point, index, fired, extra) => ({ point, index, call: fired ? describeCall(fired) : 'none', ...extra })

  /** Every DOM call of an operation, or in an endurance cycle the one that this cycle breaks, taken in turn from cycle to cycle. */
  const indicesFor = (calls, options) =>
    options.cycle === undefined
      ? Array.from({ length: calls }, (_, index) => index + 1)
      : [((options.cycle - 1) % calls) + 1]

  // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
  /**
   * Breaks one operation at each of its DOM calls and reads what is left, as every sweep above does, for the operations that follow. The
   * twin (slot 1) makes the call with no fault and is the reference, and a fresh scene (slot 0) is made for each failure.
   *
   *   setup(slot)               builds the scene on the slot, with whatever the operation needs beside it, and says what it made
   *   op(made)                  the operation, from what the twin made
   *   pointOf(fired, dry)       the fault point that the failed call belongs to
   *   retryOp(made, point)      what recovers from it. The operation again if it is not given
   *   experiments(slot, made)   further things done to the healed scene, whose results are kept in the row
   *   partial(before, after)    whether what a failure left is a state that the library documents as partly committed
   */
  const transaction = (manifest, options, spec) => {
    const made = spec.setup(1)
    const op = spec.op(made)
    intend(1, op)
    const dry = dryRun(1, op)
    if (!dry.result.ok) throw new Error(`the unfaulted twin could not do ${op.op}: ${dry.result.error}`)
    snapshot(1, 'ref')
    const reference = raw(1)
    const rows = []
    const times = {}
    for (const index of indicesFor(dry.calls.length, options)) {
      spec.setup(0)
      snapshot(0, 'before')
      const before = raw(0)
      const failure = faulted(0, op, index)
      const snap = snapshot(0, 'after')
      const after = raw(0)
      const unchanged = snap.ok && differences('before', 'after').count === 0 && sameRaw(before, after, MORE_FIELDS)
      const point = spec.pointOf(failure.fired, dry, index)
      const retry = spec.retryOp ? spec.retryOp(made, point) : op
      const recovery = retried(0, retry)
      const healed = snapshot(0, 'healed')
      const now_ = raw(0)
      const reached = healed.ok && differences('ref', 'healed').count === 0 && sameRaw(now_, reference, MORE_FIELDS)
      const extra = spec.experiments?.(0, made, { before, now_ }) ?? {}
      ;(times[point] ??= []).push(failure.elapsed + recovery.elapsed)
      rows.push(
        row(point, index, failure.fired, {
          error_surfaced: !failure.result.ok,
          outcome: unchanged
            ? 'nothing_committed'
            : spec.partial?.(before, after, point)
              ? 'partly_committed'
              : 'changed',
          left: unchanged
            ? []
            : [
                ...(snap.ok ? differences('before', 'after').sections : ['unreadable']),
                ...rawDifferences(before, after, MORE_FIELDS)
              ],
          retry: spec.retryName?.(point) ?? 'the same call again',
          retry_ok: recovery.result.ok,
          retry_reaches_reference: reached,
          ...extra
        })
      )
    }
    return { rows, times, agrees: true, calls: dry.calls.length }
  }

  /** The position of the last call of `dry` that attaches an element, which is where a drawn label or node is put in the document. */
  const lastAttach = dry =>
    dry.calls.map(c => c.method).reduce((last, m, i) => (m === 'appendChild' || m === 'insertBefore' ? i + 1 : last), 0)
  const writesLabelPlace = fired =>
    Boolean(fired && fired.method === 'setAttribute' && fired.name === 'transform' && fired.tag === 'g' && fired.label)

  const fresh = (manifest, slot, kind = null, labelled = true) => {
    const built = build(manifest, slot, labelled)
    if (kind) {
      const prepared = call(h.recovery_prepare, slot, kind)
      if (!prepared.ok) throw new Error(`preparing ${kind} on slot ${slot}: ${prepared.error}`)
    }
    return built
  }

  /** What each operation of the sweeps above needs, said once, so that its failures and the failures of its rollback are broken the same way. */
  const operations = manifest => ({
    node_construction: {
      setup: slot => build(manifest, slot, false),
      op: () => ({ op: 'add_node', key: 'fault-node', x: 40, y: 600, w: 80, h: 36 })
    },
    edge_construction: {
      setup: slot => build(manifest, slot, false),
      op: built => {
        const [from, to] = built.nodes
          .map(a => built.nodes.map(b => [a, b]))
          .flat()
          .find(([a, b]) => a !== b && !built.edges.some(([x, y]) => x === a && y === b))
        return { op: 'add_edge', from, to }
      }
    },
    edge_label: {
      setup: slot => fresh(manifest, slot),
      // The scene labels every second edge, so the first one it left alone is the one to label.
      op: () => ({ op: 'set_edge_label', edge: 1, text: 'w-new' }),
      pointOf: (fired, dry, index) => (index <= lastAttach(dry) ? 'edge_label_draw' : 'edge_label_placement'),
      partial: (before, after, point) => point === 'edge_label_placement' && after.elements > before.elements,
      retryName: point =>
        point === 'edge_label_placement' ? 'the same text again, which places the label' : 'the same call again'
    },
    connector_type: {
      setup: slot => fresh(manifest, slot),
      // Edge 0 has a label, so the change places one.
      op: () => ({ op: 'set_connector_type', edge: 0, connector: 'elbow' }),
      pointOf: fired =>
        fired?.tag === 'path' ? 'connector_type_redraw' : writesLabelPlace(fired) ? 'connector_type_label' : 'other',
      partial: (before, after, point) => point === 'connector_type_label' && after.paths !== before.paths,
      // Asking for the type the edge has is a no-op, so the label is placed again by labelling the edge with the text it has.
      retryOp: (made, point) =>
        point === 'connector_type_label'
          ? { op: 'set_edge_label', edge: 0, text: 'w0' }
          : { op: 'set_connector_type', edge: 0, connector: 'elbow' },
      retryName: point =>
        point === 'connector_type_label' ? 'the label again with its own text, which places it' : 'the same call again'
    },
    data_node_construction: {
      setup: slot => fresh(manifest, slot),
      op: () => ({
        op: 'add_data_node',
        key: 'D1',
        x: 20,
        y: 120,
        values: [5_000_000, 6_000_000, 7_000_000, 8_000_000, 9_000_000, 1_100_000]
      }),
      pointOf: () => 'data_node_construction'
    },
    data_values_update: {
      setup: slot => fresh(manifest, slot, 'data'),
      op: () => ({ op: 'set_data_values', key: 'D0', values: [1_000_001, 2_000_002, 3_000_003, 4_000_004] }),
      pointOf: () => 'data_values_update',
      partial: (before, after) => after.texts !== before.texts,
      retryName: () => 'the same values again, which writes every cell',
      experiments: (slot, made, { before }) => {
        // The values that the node held before put the node back, in one call, whatever the failed call wrote.
        const back = act(slot, {
          op: 'set_data_values',
          key: 'D0',
          values: [1_000_000, 2_000_000, 3_000_000, 4_000_000]
        })
        return { old_values_put_back: back.ok && raw(slot).texts === before.texts }
      }
    },
    operator_construction: {
      setup: slot => fresh(manifest, slot, 'data'),
      op: () => ({ op: 'add_unary_operator', key: 'op0', input: 'D0', x: 220, y: 20, values: [4293967295] }),
      pointOf: () => 'operator_construction'
    },
    container_construction: {
      setup: slot => fresh(manifest, slot, 'container'),
      op: () => ({ op: 'add_container', key: 'box', x: 20, y: 600, w: 120, h: 80 }),
      pointOf: () => 'container_construction',
      // The documented exception: the box could not be attached after the nested scene's root was hidden, and showing it again failed too.
      rollbackPartial: fields => fields.length > 0 && fields.every(field => field === 'hosts')
    }
  })

  /**
   * Breaks an operation at each DOM call, and then breaks the rollback of that failure at each DOM call that the rollback makes. How many
   * calls the rollback makes is read from the first failure: the calls counted after it, until the operation returns. An operation whose
   * rollback is only the removal of elements makes none that can fail, and has no rows, which is said.
   */
  const rollbackSweep = (manifest, options, spec, point) => {
    const made = spec.setup(1)
    const op = spec.op(made)
    intend(1, op)
    const dry = dryRun(1, op)
    if (!dry.result.ok) throw new Error(`the unfaulted twin could not do ${op.op}: ${dry.result.error}`)
    snapshot(1, 'ref')
    const reference = raw(1)
    const rows = []
    const times = {}
    let rollbackCalls = 0
    // The calls that failed in the probes, which no row records but which the glue pays for all the same.
    const probed = []
    for (const first of indicesFor(dry.calls.length, options)) {
      spec.setup(0)
      faults().arm(first)
      act(0, op)
      const probe = faults().disarm()
      if (probe?.fired) probed.push(describeCall(probe.fired))
      const following = probe ? probe.counted - first : 0
      rollbackCalls += following
      for (let gap = 1; gap <= following; gap++) {
        spec.setup(0)
        snapshot(0, 'before')
        const before = raw(0)
        faults().arm(first, 1, gap)
        const started = now()
        const failure = act(0, op)
        const elapsed = now() - started
        const fired = faults().disarm()
        const snap = snapshot(0, 'after')
        const after = raw(0)
        const counts = call(h.recovery_counts, 0)
        const unchanged = snap.ok && differences('before', 'after').count === 0 && sameRaw(before, after, MORE_FIELDS)
        const recovery = retried(0, op)
        const healed = snapshot(0, 'healed')
        const reached = healed.ok && differences('ref', 'healed').count === 0 && sameRaw(raw(0), reference, MORE_FIELDS)
        ;(times[point] ??= []).push(elapsed + recovery.elapsed)
        rows.push(
          row(point, first, fired?.fired, {
            gap,
            second_call: fired?.all?.[1] ? describeCall(fired.all[1]) : 'none',
            all_calls: (fired?.all ?? []).map(describeCall),
            error_surfaced: !failure.ok,
            outcome: unchanged
              ? 'nothing_committed'
              : spec.rollbackPartial?.(rawDifferences(before, after, MORE_FIELDS))
                ? 'partly_committed'
                : 'changed',
            retry: 'the same call again',
            observed: {
              checkpoint_readable: snap.ok,
              checkpoint_error: snap.ok ? undefined : snap.error,
              sections_changed: snap.ok ? differences('before', 'after').sections : [],
              fields_changed: rawDifferences(before, after, MORE_FIELDS),
              extra_elements: after.elements - before.elements,
              paths_changed: after.paths !== before.paths,
              edge_confirmed_by_the_call: counts.nodes + counts.edges
            },
            retry_ok: recovery.result.ok,
            retry_reaches_reference: reached
          })
        )
      }
    }
    return { rows, times, agrees: true, calls: dry.calls.length, rollbackCalls, probed }
  }

  /** The sweeps that break one operation at every DOM call. Each gives the rows of its failures, and the times of its recoveries. */
  const sweeps = {
    // Adding a node: a failure anywhere leaves nothing behind.
    node_construction: (manifest, options = {}) => {
      const op = { op: 'add_node', key: 'fault-node', x: 40, y: 600, w: 80, h: 36 }
      build(manifest, 1, false)
      intend(1, op)
      const dry = dryRun(1, op)
      if (!dry.result.ok) throw new Error(`the unfaulted twin could not add a node: ${dry.result.error}`)
      snapshot(1, 'ref')
      call(h.recovery_predicted, 1, 'model')
      const agrees = differences('model', 'ref')
      const reference = raw(1)
      const rows = []
      const times = []
      for (const index of indicesFor(dry.calls.length, options)) {
        build(manifest, 0, false)
        snapshot(0, 'before')
        const before = raw(0)
        const failure = faulted(0, op, index)
        const snap = snapshot(0, 'after')
        const after = raw(0)
        const unchanged = snap.ok && differences('before', 'after').count === 0 && sameRaw(before, after)
        const recovery = retried(0, op)
        const healed = snapshot(0, 'healed')
        const reached =
          healed.ok &&
          differences('ref', 'healed').count === 0 &&
          after.elements !== null &&
          raw(0).elements === reference.elements
        times.push(failure.elapsed + recovery.elapsed)
        rows.push(
          row('node_construction', index, failure.fired, {
            error_surfaced: !failure.result.ok,
            outcome: unchanged ? 'nothing_committed' : 'changed',
            left: unchanged
              ? []
              : [
                  ...(snap.ok ? differences('before', 'after').sections : ['unreadable']),
                  ...rawDifferences(before, after)
                ],
            retry_ok: recovery.result.ok,
            retry_reaches_reference: reached
          })
        )
      }
      return { rows, times, agrees: agrees.count === 0, calls: dry.calls.length }
    },

    // Moving a node: the write of its transform, then its connectors, then its labels.
    move_node: (manifest, options = {}) => {
      const built = build(manifest, 0, true)
      const key = built.edges[0][0]
      const start = call(h.recovery_node_rect, 0, key)
      const target = { x: start.x + 45, y: start.y + 30 }
      const elsewhere = { x: target.x - 25, y: target.y + 40 }
      const moveTo = at => ({ op: 'move_node', key, ...at })

      // The twin makes the move cleanly, and then a second one to somewhere else, so that both the state a recovery by the same
      // move reaches and the state a recovery by another move reaches are known.
      build(manifest, 1, true)
      intend(1, moveTo(target))
      const dry = dryRun(1, moveTo(target))
      if (!dry.result.ok) throw new Error(`the unfaulted twin could not move a node: ${dry.result.error}`)
      snapshot(1, 'ref')
      call(h.recovery_predicted, 1, 'model')
      const agrees = differences('model', 'ref')
      const reference = raw(1)
      act(1, moveTo(elsewhere))
      intend(1, moveTo(elsewhere))
      snapshot(1, 'ref2')
      const reference2 = raw(1)

      const rows = []
      const times = { dom_transform: [], connector_update: [], label_update: [] }
      for (const index of indicesFor(dry.calls.length, options)) {
        build(manifest, 0, true)
        snapshot(0, 'before')
        const before = raw(0)
        const failure = faulted(0, moveTo(target), index)
        const snap = snapshot(0, 'after')
        const after = raw(0)
        const phase = phaseOf(failure.fired)
        const where = call(h.recovery_node_rect, 0, key)
        const moved = where.x === target.x && where.y === target.y
        const unchanged = snap.ok && differences('before', 'after').count === 0 && sameRaw(before, after)

        let recovery
        let detail = {}
        if (phase === 'label_update') {
          // A move to the origin it already has retries the label that failed to place.
          recovery = retried(0, moveTo(target))
          const healed = snapshot(0, 'healed')
          detail.retry = 'a move to the origin the node already has'
          detail.retry_reaches_reference =
            healed.ok &&
            differences('ref', 'healed').count === 0 &&
            raw(0).labels === reference.labels &&
            raw(0).paths === reference.paths
        } else if (phase === 'connector_update') {
          // The same origin redraws no connector. That is documented too, so it is checked: the paths must not change.
          const pathsBefore = raw(0).paths
          const same = act(0, moveTo(target))
          detail.same_origin_redraws_nothing = same.ok && raw(0).paths === pathsBefore
          // A move to a different origin recomputes every incident connector from where the nodes now are.
          recovery = retried(0, moveTo(elsewhere))
          const healed = snapshot(0, 'healed')
          detail.retry = 'a move to a different origin'
          const checkpointDiff = healed.ok ? differences('ref2', 'healed') : { count: -1, first: ['unreadable'] }
          const now_ = raw(0)
          detail.parts = {
            checkpoint_differences: checkpointDiff.count,
            first: checkpointDiff.first,
            paths_equal: now_.paths === reference2.paths,
            labels_equal: now_.labels === reference2.labels
          }
          // Connectors are a function of where the nodes are, which is what the contract is about. A label is kept where it is while
          // it still can be, so where it ends up depends on the moves before, and a clean run is not a reference for it.
          detail.retry_reaches_reference = checkpointDiff.count === 0 && now_.paths === reference2.paths
        } else {
          recovery = retried(0, moveTo(target))
          const healed = snapshot(0, 'healed')
          detail.retry = 'the same move again'
          detail.retry_reaches_reference =
            healed.ok && differences('ref', 'healed').count === 0 && sameRaw(raw(0), reference)
        }
        if (times[phase]) times[phase].push(failure.elapsed + recovery.elapsed)
        rows.push(
          row(phase, index, failure.fired, {
            error_surfaced: !failure.result.ok,
            outcome: unchanged ? 'nothing_committed' : moved ? 'partly_committed' : 'inconsistent',
            node_moved: moved,
            left: unchanged
              ? []
              : [
                  ...(snap.ok ? differences('before', 'after').sections : ['unreadable']),
                  ...rawDifferences(before, after)
                ],
            retry_ok: recovery.result.ok,
            ...detail
          })
        )
      }
      return { rows, times, agrees: agrees.count === 0, calls: dry.calls.length }
    },

    // Making a node draggable: four listeners and a style, and a failure anywhere leaves it as it was.
    drag_install: (manifest, options = {}) => {
      const built = build(manifest, 1, false)
      const key = built.nodes[0]
      const op = { op: 'make_draggable', key }
      const before1 = raw(1)
      const dry = dryRun(1, op)
      if (!dry.result.ok) throw new Error(`the unfaulted twin could not make a node draggable: ${dry.result.error}`)
      const reference = raw(1)
      const added = reference.listeners - before1.listeners
      const rows = []
      const times = []
      for (const index of indicesFor(dry.calls.length, options)) {
        build(manifest, 0, false)
        const before = raw(0)
        const failure = faulted(0, op, index)
        const after = raw(0)
        const dangling = after.listeners - before.listeners
        const unchanged = sameRaw(before, after)
        // A node that the failed call left marked as draggable refuses a second installation.
        const recovery = retried(0, op)
        const marked = !recovery.result.ok && /draggable/i.test(recovery.result.error ?? '')
        const final = raw(0)
        times.push(failure.elapsed + recovery.elapsed)
        rows.push(
          row('drag_install', index, failure.fired, {
            error_surfaced: !failure.result.ok,
            outcome: unchanged ? 'nothing_committed' : 'changed',
            left: unchanged ? [] : rawDifferences(before, after),
            listeners_left: dangling,
            marked_draggable_by_the_failed_call: marked,
            retry_ok: recovery.result.ok,
            retry_reaches_reference:
              recovery.result.ok && final.listeners - before.listeners === added && final.styles === reference.styles
          })
        )
      }
      return { rows, times, agrees: true, calls: dry.calls.length }
    },

    // Adding an edge: nothing documents what a failure part way leaves, so each is observed and written down.
    edge_construction: (manifest, options = {}, kind = { point: 'edge_construction', connector: 'straight' }) => {
      const built = build(manifest, 1, false)
      const pair = built.nodes
        .map(from => built.nodes.map(to => [from, to]))
        .flat()
        .find(([from, to]) => from !== to && !built.edges.some(([a, b]) => a === from && b === to))
      const op = { op: 'add_edge', from: pair[0], to: pair[1], connector: kind.connector }
      intend(1, op)
      const dry = dryRun(1, op)
      if (!dry.result.ok) throw new Error(`the unfaulted twin could not add an edge: ${dry.result.error}`)
      snapshot(1, 'ref')
      const reference = raw(1)
      const referenceEnds = endNames(1, pair)
      const rows = []
      const times = []
      for (const index of indicesFor(dry.calls.length, options)) {
        build(manifest, 0, false)
        snapshot(0, 'before')
        const before = raw(0)
        const endsBefore = endNames(0, pair)
        const failure = faulted(0, op, index)
        const snap = snapshot(0, 'after')
        const after = raw(0)
        const counts = call(h.recovery_counts, 0)
        // What each end of the edge says about itself, which is where the second half of the call writes. It is compared with what
        // it said before, since the names of other nodes that happen to mention the same node are not the ends of this edge.
        const ends = endNames(0, pair)
        const named = pair.filter((key, position) => ends[position] !== endsBefore[position]).length
        const recovery = retried(0, op)
        const healed = snapshot(0, 'healed')
        times.push(failure.elapsed + recovery.elapsed)
        rows.push(
          row(kind.point, index, failure.fired, {
            error_surfaced: !failure.result.ok,
            outcome:
              snap.ok && differences('before', 'after').count === 0 && sameRaw(before, after) && named === 0
                ? 'nothing_committed'
                : 'changed',
            observed: {
              checkpoint_readable: snap.ok,
              checkpoint_error: snap.ok ? undefined : snap.error,
              sections_changed: snap.ok ? differences('before', 'after').sections : [],
              extra_elements: after.elements - before.elements,
              paths_changed: after.paths !== before.paths,
              ends_named: named,
              edge_confirmed_by_the_call: counts.edges
            },
            retry_ok: recovery.result.ok,
            // The checkpoint and the document both have to be the reference's. A stray connector is an element that a checkpoint
            // does not see, and a name that is half written is one that it sees only through the clauses.
            retry_reaches_reference:
              healed.ok &&
              differences('ref', 'healed').count === 0 &&
              raw(0).elements === reference.elements &&
              raw(0).paths === reference.paths &&
              JSON.stringify(endNames(0, pair)) === JSON.stringify(referenceEnds)
          })
        )
      }
      return { rows, times, agrees: true, calls: dry.calls.length }
    },

    // Labelling an edge: a label that cannot be drawn is not left behind, and one that cannot be placed is where it was.
    edge_label: (manifest, options = {}) => transaction(manifest, options, operations(manifest).edge_label),

    // Adding an edge that is an elbow: the same transaction as a straight one, with the routing of an elbow in it.
    elbow_edge_construction: (manifest, options = {}) =>
      sweeps.edge_construction(manifest, options, { point: 'elbow_edge_construction', connector: 'elbow' }),

    // Changing a connector's type: a failure of the new path leaves the edge as it was, and one of its label's place leaves the new type drawn.
    connector_type: (manifest, options = {}) => transaction(manifest, options, operations(manifest).connector_type),

    // A data node: drawing is a transaction, and a failure of a data node's own cells leaves the scene as it was.
    data_node_construction: (manifest, options = {}) =>
      transaction(manifest, options, operations(manifest).data_node_construction),

    // New values for a data node: the model keeps the old ones until every cell is written, and calling again heals it, with the same
    // values, or with the old ones, which puts the node back.
    data_values_update: (manifest, options = {}) =>
      transaction(manifest, options, operations(manifest).data_values_update),

    // An operator node: it is drawn, and its input edge wired, and a failure of either takes the node away again.
    operator_construction: (manifest, options = {}) =>
      transaction(manifest, options, operations(manifest).operator_construction),

    // A container node, which owns a nested scene: a failure leaves both scenes as they were, and the nested one visible.
    container_construction: (manifest, options = {}) =>
      transaction(manifest, options, operations(manifest).container_construction),

    // A failure inside the rollback of a failure. Each operation that undoes itself is broken at every DOM call, and then, for each of those,
    // at every DOM call that the undoing makes. Nothing documents what is left, so each is observed and written down.
    rollback_node_construction: (manifest, options = {}) =>
      rollbackSweep(manifest, options, operations(manifest).node_construction, 'rollback_node_construction'),
    rollback_edge_construction: (manifest, options = {}) =>
      rollbackSweep(manifest, options, operations(manifest).edge_construction, 'rollback_edge_construction'),
    rollback_edge_label_draw: (manifest, options = {}) =>
      rollbackSweep(manifest, options, operations(manifest).edge_label, 'rollback_edge_label_draw'),
    rollback_data_node_construction: (manifest, options = {}) =>
      rollbackSweep(manifest, options, operations(manifest).data_node_construction, 'rollback_data_node_construction'),
    rollback_operator_construction: (manifest, options = {}) =>
      rollbackSweep(manifest, options, operations(manifest).operator_construction, 'rollback_operator_construction'),
    rollback_container_construction: (manifest, options = {}) =>
      rollbackSweep(manifest, options, operations(manifest).container_construction, 'rollback_container_construction')
  }

  // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
  /** The host is taken out of the document while the scene is alive, and the scene is used. Nothing documents what happens. */
  const removedHost = manifest => {
    const built = build(manifest, 0, true)
    const svg = document.getElementById(OPEN + '0')
    svg.remove()
    const key = built.nodes[0]
    const start = call(h.recovery_node_rect, 0, key)
    const tries = [
      { op: 'add_node', key: 'late', x: 40, y: 600, w: 80, h: 36 },
      { op: 'move_node', key, x: start.x + 45, y: start.y + 30 },
      { op: 'add_edge', from: key, to: 'late' },
      { op: 'make_draggable', key: built.nodes[1] },
      { op: 'set_edge_label', edge: 1, text: 'late label' }
    ]
    const outcomes = tries.map(op => {
      intend(0, op)
      const result = act(0, op)
      return { op: op.op, ok: result.ok, error: result.error }
    })
    document.body.appendChild(svg)
    const back = snapshot(0, 'reattached')
    call(h.recovery_predicted, 0, 'model')
    const agrees = back.ok ? differences('model', 'reattached') : null
    return {
      outcomes,
      readable_when_back: back.ok,
      readable_error: back.ok ? undefined : back.error,
      agrees_with_the_model_when_back: agrees ? agrees.count === 0 : null,
      first_differences: agrees?.first ?? []
    }
  }

  // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
  /** What the contract says should be true of the failures at a point, and the rows that break it. */
  const judge = (point, expect, rows) => {
    const mine = rows.filter(r => r.point === point)
    if (mine.length === 0)
      return {
        status: 'inconclusive',
        detail: `no DOM call of the operation fell in the ${point} phase, so nothing was broken`
      }
    const broken = []
    for (const r of mine) {
      if (!r.error_surfaced) broken.push(`call ${r.index} (${r.call}) failed and the operation reported success`)
      else if (expect === 'nothing_committed') {
        if (r.outcome !== 'nothing_committed')
          broken.push(
            `call ${r.index} (${r.call}) left changes in ${
              (r.left ?? leftBy(r.observed ?? {})).join(', ') || 'the scene'
            }`
          )
        else if (!r.retry_ok) broken.push(`call ${r.index} (${r.call}): the retry failed too`)
        else if (r.retry_reaches_reference === false)
          broken.push(`call ${r.index} (${r.call}): the retry did not reach the reference`)
        else if (r.marked_draggable_by_the_failed_call)
          broken.push(`call ${r.index} (${r.call}) left the node marked as draggable`)
      } else if (expect === 'partly_committed') {
        if (r.outcome !== 'partly_committed')
          broken.push(`call ${r.index} (${r.call}) left the state "${r.outcome}", and a committed move was documented`)
        else if (!r.retry_ok) broken.push(`call ${r.index} (${r.call}): the defined retry failed`)
        else if (!r.retry_reaches_reference)
          broken.push(`call ${r.index} (${r.call}): the defined retry (${r.retry}) did not reach the reference`)
        else if (r.same_origin_redraws_nothing === false)
          broken.push(
            `call ${r.index} (${r.call}): a move to the same origin redrew a connector, and the documentation says it does not`
          )
      }
    }
    return broken.length === 0
      ? {
          status: 'pass',
          detail: `${mine.length} failures, each ${expect.replace('_', ' ')} as documented${expect === 'nothing_committed' ? ', and the retry reached the reference' : ', and the defined retry reached the reference'}`
        }
      : { status: 'fail', detail: `${broken.length} of ${mine.length}: ${broken[0]}`, broken }
  }

  return { sweeps, removedHost, judge, raw, build, act, intend, snapshot, differences, faults, call }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** The points that need real input or a browser frame, and so are broken by Layer C and not here. */
const LAYER_C = new Set(['scheduled_frame', 'teardown', 'mid_drag_teardown'])
/** The sweeps and the fault points each of them breaks. */
const GROUPS = [
  { sweep: 'node_construction', points: ['node_construction'] },
  { sweep: 'move_node', points: ['dom_transform', 'connector_update', 'label_update'] },
  { sweep: 'drag_install', points: ['drag_install'] },
  { sweep: 'edge_construction', points: ['edge_construction'] },
  { sweep: 'edge_label', points: ['edge_label_draw', 'edge_label_placement'] },
  { sweep: 'elbow_edge_construction', points: ['elbow_edge_construction'] },
  { sweep: 'connector_type', points: ['connector_type_redraw', 'connector_type_label'] },
  { sweep: 'data_node_construction', points: ['data_node_construction'] },
  { sweep: 'data_values_update', points: ['data_values_update'] },
  { sweep: 'operator_construction', points: ['operator_construction'] },
  { sweep: 'container_construction', points: ['container_construction'] },
  { sweep: 'rollback_node_construction', points: ['rollback_node_construction'] },
  { sweep: 'rollback_edge_construction', points: ['rollback_edge_construction'] },
  { sweep: 'rollback_edge_label_draw', points: ['rollback_edge_label_draw'] },
  { sweep: 'rollback_data_node_construction', points: ['rollback_data_node_construction'] },
  { sweep: 'rollback_operator_construction', points: ['rollback_operator_construction'] },
  { sweep: 'rollback_container_construction', points: ['rollback_container_construction'] }
]
const MECHANISM =
  'performance.now() in the page around the failed call and the call that recovers from it, with no check inside the interval'

// The section that a failure is filed under in a result, which has a fixed list of them.
const sectionOf = row => {
  const left = (row.left ?? []).filter(item =>
    ['topology', 'dom_identity', 'geometry', 'content', 'interaction', 'accessibility', 'paint_order'].includes(item)
  )
  return left[0] ?? (row.outcome === 'inconsistent' ? 'geometry' : 'dom_identity')
}

/**
 * Breaks every fault point of `manifest.recovery` that this layer can reach, and gives an outcome in the shape of the other
 * layers', with the findings and the table of failures as well.
 */
export const runRecovery = (h, manifest, testing = {}) => {
  const R = makeRecovery(h, () => performance.now(), testing)
  const outcome = {
    status: 'ok',
    metrics: [],
    gates: [],
    notes: [],
    findings: [],
    cases: [],
    samples: {},
    checkpoints_passed: 0,
    checkpoints_failed: [],
    stream: null
  }
  const wanted = manifest.recovery.points
  // The smallest step the page's clock makes, which every time in a result has to say.
  outcome.timer_resolution_us = (() => {
    let smallest = Infinity
    let last = performance.now()
    for (let probe = 0; probe < 200000; probe++) {
      const at = performance.now()
      const step = (at - last) * 1000
      if (step > 0 && step < smallest) smallest = step
      last = at
    }
    return Number.isFinite(smallest) ? smallest : 0
  })()
  const expectOf = point => wanted.find(p => p.point === point)?.expect
  const metric = (name, microseconds) =>
    parse(h.describe_metric(name, 'B', 'us', JSON.stringify(microseconds), MECHANISM))
  const first = JSON.parse(h.recovery_build(JSON.stringify(manifest), 0, false))
  if (first.ok) {
    outcome.stream = {
      scope: 'full',
      operations: first.operations,
      sha256: first.sha256,
      expected_sha256: null,
      matches: null
    }
    h.recovery_teardown(0)
  }

  for (const { point } of wanted.filter(p => LAYER_C.has(p.point)))
    outcome.notes.push(`${point} needs real input and a frame, and is broken by Layer C`)

  const rows = []
  for (const group of GROUPS) {
    const points = group.points.filter(point => wanted.some(p => p.point === point))
    if (points.length === 0) continue
    h.recovery_forget()
    let swept
    try {
      swept = R.sweeps[group.sweep](manifest)
    } catch (error) {
      outcome.status = 'error'
      outcome.error = `${group.sweep}: ${error.message}`
      continue
    }
    rows.push(...swept.rows)
    const twin = swept.agrees
    outcome.gates.push({
      name: `recovery.${group.sweep}.reference_agrees_with_the_model`,
      status: twin ? 'pass' : 'fail',
      detail: twin
        ? 'the twin that made the call with no fault matches the reference model'
        : 'the twin that made the call with no fault differs from the reference model, so it is not a reference'
    })
    outcome.notes.push(`${group.sweep}: the operation makes ${swept.calls} DOM calls, and each was made to fail`)
    for (const point of points) {
      const expect = expectOf(point)
      const mine = swept.rows.filter(r => r.point === point)
      const times = Array.isArray(swept.times) ? swept.times : (swept.times[point] ?? [])
      if (expect === 'undocumented') {
        outcome.findings.push(summariseFinding(point, mine))
        if (times.length) {
          outcome.metrics.push(
            metric(
              `recovery.${point}.time`,
              times.map(t => t * 1000)
            )
          )
          outcome.samples[`recovery.${point}.time`] = times.map(t => t * 1000)
        }
        continue
      }
      const verdict = R.judge(point, expect, swept.rows)
      outcome.gates.push({
        name: `recovery.${point}.matches_the_contract`,
        status: verdict.status,
        detail: verdict.detail
      })
      for (const r of mine) {
        const fine = !verdict.broken?.some(b => b.startsWith(`call ${r.index} `))
        if (fine) outcome.checkpoints_passed++
        else
          outcome.checkpoints_failed.push({
            section: sectionOf(r),
            detail: `${point}: call ${r.index} (${r.call}): ${(verdict.broken ?? []).find(b => b.startsWith(`call ${r.index} `))}`
          })
      }
      if (times.length) outcome.samples[`recovery.${point}.time`] = times.map(t => t * 1000)
      outcome.metrics.push(
        times.length
          ? metric(
              `recovery.${point}.time`,
              times.map(t => t * 1000)
            )
          : {
              name: `recovery.${point}.time`,
              layer: 'B',
              status: 'unavailable',
              reason: 'no failure fell in this phase'
            }
      )
      outcome.metrics.push({
        name: `recovery.${point}.failures_injected`,
        layer: 'B',
        status: 'measured',
        unit: 'count',
        kind: 'count',
        value: mine.length,
        mechanism: 'DOM calls of the operation made to fail one at a time, by perf/page/fault-injector.js'
      })
    }
  }

  // What a failed DOM call costs the glue. This is not a fault point of the library, and it is found here because failing the library's
  // calls is how it shows. It is measured, and reported as a finding.
  outcome.findings.push(slotLossFinding(h, R, manifest))

  if (wanted.some(p => p.point === 'removed_host')) {
    const seen = R.removedHost(manifest)
    outcome.findings.push({
      point: 'removed_host',
      contract: 'undocumented',
      summary: `with the host <svg> out of the document, ${seen.outcomes.filter(o => o.ok).length} of ${seen.outcomes.length} operations succeeded (${seen.outcomes.map(o => `${o.op} ${o.ok ? 'ok' : `failed: ${o.error}`}`).join('; ')}); put back, the scene ${seen.readable_when_back ? `was readable and ${seen.agrees_with_the_model_when_back ? 'agreed' : 'did not agree'} with the model` : 'could not be read'}`,
      details: seen
    })
  }
  R.build(manifest, 0, false)
  h.recovery_teardown(0)
  h.recovery_teardown(1)
  outcome.cases = rows
  if (outcome.checkpoints_failed.length > 0 && outcome.status === 'ok') outcome.status = 'invalid_incorrect'
  return outcome
}

// What was seen at an undocumented point, grouped, so that it reads as a finding and not as a table of rows.
const summariseFinding = (point, rows) => {
  if (rows.length === 0 && point.startsWith('rollback_'))
    return {
      point,
      contract: 'undocumented',
      summary:
        'the rollback of this operation makes no DOM call that can fail, since it only removes elements, so there is no failure of the rollback to inject',
      details: { observed: [] }
    }
  const groups = new Map()
  for (const r of rows) {
    const o = r.observed ?? {}
    const signature = [
      `the error was ${r.error_surfaced ? 'reported' : 'NOT reported'}`,
      o.checkpoint_readable === false
        ? `the scene could not be read back (${String(o.checkpoint_error).slice(0, 80)})`
        : `sections changed: ${(o.sections_changed ?? []).join(', ') || 'none'}`,
      ...(r.gap === undefined ? [] : [`a second failure fell ${r.gap} calls after the first, inside the rollback`]),
      ...(o.fields_changed === undefined ? [] : [`document fields changed: ${o.fields_changed.join(', ') || 'none'}`]),
      `${o.extra_elements} extra elements`,
      `connector paths ${o.paths_changed ? 'changed' : 'unchanged'}`,
      ...(o.ends_named === undefined ? [] : [`${o.ends_named} of the 2 ends of the edge say so`]),
      `the call confirmed ${o.edge_confirmed_by_the_call} ${r.gap === undefined ? 'edges' : 'nodes and edges'}`,
      `a retry ${r.retry_reaches_reference ? 'reaches the reference' : 'does NOT reach the reference'}`
    ].join('; ')
    const entry = groups.get(signature) ?? { signature, calls: [] }
    entry.calls.push(
      r.gap === undefined ? `${r.index} ${r.call}` : `${r.index}+${r.gap} ${r.call} then ${r.second_call}`
    )
    groups.set(signature, entry)
  }
  const observed = [...groups.values()].map(g => ({
    what_was_left: g.signature,
    failures: g.calls.length,
    calls: g.calls
  }))
  return {
    point,
    contract: 'undocumented',
    summary: `${rows.length} failures, one at each DOM call of the operation, left ${observed.length} different states: ${observed.map(o => `${o.failures} at ${o.calls.map(c => c.split(' ')[0]).join(', ')} (${o.what_was_left})`).join('. ')}`,
    details: { observed }
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const resolutionOf = () => {
  let smallest = Infinity
  let last = performance.now()
  for (let probe = 0; probe < 200000; probe++) {
    const at = performance.now()
    const step = (at - last) * 1000
    if (step > 0 && step < smallest) smallest = step
    last = at
  }
  return Number.isFinite(smallest) ? smallest : 0
}

/**
 * Endurance under deliberate failures: the same cycles as the recovery sweeps, one failure at a time, over and over.
 *
 * Every cycle builds a scene and its twin, breaks each fault point the manifest names at one of its DOM calls (the call is
 * different from cycle to cycle, so that the cycles together reach all of them), recovers as the contract says, takes both scenes
 * down, and reads what is left: the document's elements and listeners, and the counting allocator's live bytes. After the warm-up,
 * nothing may have grown. A failure must not leak what a success would not, and a recovery must not leave anything behind.
 */
export const runEndurance = (h, manifest, testing = {}) => {
  const R = makeRecovery(h, () => performance.now())
  const { warmup_iterations: warmup, measurement_iterations: measured } = manifest.execution
  const total = warmup + measured
  const allowance = manifest.limits.live_growth_allowance_bytes ?? 0
  const wanted = manifest.recovery.points
  const outcome = {
    status: 'ok',
    metrics: [],
    gates: [],
    notes: [],
    findings: [],
    cases: [],
    samples: {},
    checkpoints_passed: 0,
    checkpoints_failed: [],
    stream: null,
    timer_resolution_us: resolutionOf()
  }
  const first = JSON.parse(h.recovery_build(JSON.stringify(manifest), 0, false))
  if (first.ok)
    outcome.stream = {
      scope: 'full',
      operations: first.operations,
      sha256: first.sha256,
      expected_sha256: null,
      matches: null
    }
  h.recovery_teardown(0)

  const reading = () => ({
    ...parse(h.resource_reading()),
    listeners: window.__listeners?.snapshot().total ?? null,
    elements: document.querySelectorAll('*').length,
    handles: h.held_handles?.().held ?? null
  })
  // The slab of externref slots is made as large as the run will need before anything is read, since it doubles as slots are lost
  // (see RETURNS_AN_OBJECT) and a doubling is not growth that the library caused. A cycle makes a handful of failures, and every
  // one of them can lose a slot, so twelve a cycle is more than enough and the run checks that it was.
  h.reserve_handles(2 ** Math.ceil(Math.log2(total * 12 + 512)))
  h.recovery_forget()
  const baseline = reading()
  const slotsAtStart = h.held_handles().undefinedSlots
  let objectCallsFailed = 0
  const series = []
  const unmet = []
  let judged = 0
  for (let cycle = 1; cycle <= total; cycle++) {
    const rows = []
    const probedCalls = []
    for (const group of GROUPS) {
      if (!group.points.some(point => wanted.some(p => p.point === point))) continue
      // For the self-test and for finding where a leak comes from: the same DOM call is broken in every cycle.
      const swept = R.sweeps[group.sweep](manifest, { cycle: testing.pin ?? cycle })
      rows.push(...swept.rows)
      probedCalls.push(...(swept.probed ?? []))
    }
    objectCallsFailed += probedCalls.filter(c => RETURNS_AN_OBJECT.has(methodOf({ call: c }))).length
    objectCallsFailed += rows.reduce(
      (sum, r) => sum + (r.all_calls ?? [r.call]).filter(c => RETURNS_AN_OBJECT.has(methodOf({ call: c }))).length,
      0
    )
    for (const { point, expect } of wanted.filter(p => p.expect !== 'undocumented' && !LAYER_C.has(p.point))) {
      const verdict = R.judge(point, expect, rows)
      if (verdict.status === 'inconclusive') continue
      judged++
      if (verdict.status === 'fail') unmet.push(`cycle ${cycle}, ${point}: ${verdict.detail}`)
    }
    h.recovery_teardown(0)
    h.recovery_teardown(1)
    h.recovery_forget()
    // For the self-test only: something that every cycle leaves behind, which the gates have to notice.
    if (testing.leakBytes) h.leak_bytes(testing.leakBytes)
    if (testing.leakListener) document.body.addEventListener('perf-leak', () => {})
    if (testing.leakElement) document.body.appendChild(document.createElement('div'))
    // A failed call of the harness's own, which no row records: it loses a slot that nothing explains.
    if (testing.unexplainedSlotLoss) {
      window.__faults.arm(1)
      h.recovery_build(JSON.stringify(manifest), 2, false)
      window.__faults.disarm()
    }
    const now = reading()
    series.push({
      cycle,
      measured: cycle > warmup,
      live_bytes: now.live_bytes,
      capacity: now.capacity,
      listeners: now.listeners,
      elements: now.elements,
      handles: now.handles
    })
  }

  const verdict = parse(
    h.endurance_verdict(
      JSON.stringify(
        series.map(({ cycle, measured: m, live_bytes, capacity }) => ({ cycle, measured: m, live_bytes, capacity }))
      ),
      allowance
    )
  )
  const mechanism = 'the counting allocator and the document, read after both scenes of a cycle were torn down'
  const count = (name, unit, value, how) => ({
    name,
    layer: 'B',
    status: 'measured',
    unit,
    kind: 'count',
    value,
    mechanism: how ?? mechanism
  })
  const trend = (name, unit, value, how) => ({
    name,
    layer: 'B',
    status: 'measured',
    unit,
    kind: 'trend',
    value,
    mechanism: how ?? mechanism
  })
  outcome.gates.push({ name: 'endurance.live_bytes_do_not_grow', status: verdict.status, detail: verdict.detail })
  outcome.gates.push({
    name: 'endurance.recoveries_succeed',
    status: unmet.length === 0 && judged > 0 ? 'pass' : judged === 0 ? 'inconclusive' : 'fail',
    detail:
      unmet.length === 0
        ? `${judged} recoveries over ${total} cycles each reached the state the contract promises`
        : `${unmet.length} of ${judged} recoveries did not: ${unmet[0]}`
  })
  const back = (name, field, what) => {
    const off = series.filter(point => point[field] !== baseline[field])
    outcome.gates.push({
      name,
      status: off.length === 0 ? 'pass' : 'fail',
      detail:
        off.length === 0
          ? `${what} was ${baseline[field]} before the first cycle and after each of the ${total} teardowns`
          : `${off.length} of ${total} teardowns left ${what} at a count other than ${baseline[field]}, the first being cycle ${off[0].cycle} with ${off[0][field]}`
    })
  }
  back('endurance.listeners_return_to_baseline', 'listeners', 'the effective listener count')
  back('endurance.dom_returns_to_baseline', 'elements', "the document's element count")
  // The slots that wasm-bindgen lost are counted exactly, and have to be the ones that the failed object-returning calls explain.
  const slotsLost = h.held_handles().undefinedSlots - slotsAtStart
  outcome.gates.push({
    name: 'endurance.lost_externref_slots_are_explained',
    status: slotsLost === objectCallsFailed ? 'pass' : 'fail',
    detail:
      slotsLost === objectCallsFailed
        ? `${slotsLost} externref slots were lost over ${total} cycles, one for each of the ${objectCallsFailed} failures of a DOM call that returns an object, and none for the others`
        : `${slotsLost} externref slots were lost, and ${objectCallsFailed} failures of a call that returns an object explain only those`
  })
  outcome.metrics.push(
    count(
      'wasm.externref_slots_lost',
      'count',
      slotsLost,
      "slots of wasm-bindgen's externref table that hold undefined and are never freed, counted by reading the table before the first cycle and after the last"
    )
  )
  outcome.metrics.push(count('lifecycle.measured_cycles_sampled', 'count', verdict.measured_points))
  if (verdict.net_live_change !== null)
    outcome.metrics.push(trend('wasm.live_bytes_net_change_after_warmup', 'bytes', verdict.net_live_change))
  else
    outcome.metrics.push({
      name: 'wasm.live_bytes_net_change_after_warmup',
      layer: 'B',
      status: 'unavailable',
      reason: 'no measured cycle differs from the reference, or a live-bytes reading was missing'
    })
  if (verdict.live_slope_per_cycle !== null)
    outcome.metrics.push(trend('wasm.live_bytes_slope_per_cycle', 'bytes', verdict.live_slope_per_cycle))
  else
    outcome.metrics.push({
      name: 'wasm.live_bytes_slope_per_cycle',
      layer: 'B',
      status: 'unavailable',
      reason: 'fewer than 3 measured cycles were sampled, which is not a trend'
    })
  if (verdict.grow_events !== null)
    outcome.metrics.push(
      count(
        'wasm.memory_grow_events_after_warmup',
        'count',
        verdict.grow_events,
        'WebAssembly.Memory.buffer.byteLength after each teardown'
      )
    )
  outcome.metrics.push(
    count(
      'recovery.cycles',
      'count',
      total,
      'cycles in which every fault point of the manifest was broken and recovered from'
    )
  )
  outcome.samples['resource.after_teardown.cycle'] = series.map(point => point.cycle)
  outcome.samples['resource.after_teardown.measured'] = series.map(point => Number(point.measured))
  outcome.samples['resource.live_bytes.after_teardown.per_cycle'] = series.map(point => point.live_bytes)
  outcome.samples['resource.listeners.after_teardown.per_cycle'] = series.map(point => point.listeners)
  outcome.samples['resource.dom_elements.after_teardown.per_cycle'] = series.map(point => point.elements)
  outcome.samples['resource.capacity.after_teardown.per_cycle'] = series.map(point => point.capacity)
  outcome.samples['resource.js_handles_held.after_teardown.per_cycle'] = series.map(point => point.handles)
  outcome.notes.push(
    `${total} cycles (${warmup} warm-up), each breaking the fault points of the manifest at one DOM call, a different one from cycle to cycle`
  )
  outcome.checkpoints_passed = judged - unmet.length
  for (const line of unmet) outcome.checkpoints_failed.push({ section: 'dom_identity', detail: line })
  if (outcome.checkpoints_failed.length > 0) outcome.status = 'invalid_incorrect'
  return outcome
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Fails the first DOM call of each method that `add_node` and `make_draggable` make, many times, and counts the externref slots that
 * were lost: those holding `undefined`, which the glue filled and nothing frees.
 */
const slotLossFinding = (h, R, manifest) => {
  const FAILURES = 40
  const built = R.build(manifest, 0, false)
  const ops = [
    { op: 'add_node', key: 'probe', x: 40, y: 600, w: 80, h: 36 },
    { op: 'make_draggable', key: built.nodes[0] }
  ]
  const seen = new Map()
  for (const op of ops) {
    const probe = R.faults()
    probe.record(true)
    R.build(manifest, 1, false)
    probe.take()
    R.act(1, op)
    probe.record(false)
    for (const [index, call] of probe.take().entries())
      if (!seen.has(call.method)) seen.set(call.method, { op, index: index + 1 })
  }
  const table = []
  for (const [method, { op, index }] of seen) {
    R.build(manifest, 0, false)
    const before = h.held_handles().undefinedSlots
    let failed = 0
    for (let n = 0; n < FAILURES; n++) {
      R.faults().arm(index)
      if (!R.act(0, op).ok) failed++
      R.faults().disarm()
      // A failed add_node of this kind leaves nothing to build on, and a draggable node is refused a second time, so each is built afresh.
      R.build(manifest, 0, false)
    }
    table.push({
      method,
      returns_an_object: RETURNS_AN_OBJECT.has(method),
      failures: failed,
      slots_lost: h.held_handles().undefinedSlots - before
    })
  }
  h.recovery_teardown(0)
  h.recovery_teardown(1)
  const losing = table.filter(row => row.slots_lost > 0)
  const exact = table.every(row => row.slots_lost === (row.returns_an_object ? row.failures : 0))
  return {
    point: 'externref_slot_per_failed_call',
    contract: 'undocumented',
    summary: `a DOM call that fails costs wasm-bindgen's glue one externref slot for good if the call returns an object, and nothing if it returns nothing: ${table.map(row => `${row.method} ${row.slots_lost} lost in ${row.failures} failures`).join(', ')}. ${exact ? 'Exactly one slot per failure, for every method that returns an object, and none for the others.' : 'The loss was not exactly one slot per failure of a call that returns an object.'} The slot holds undefined and is never freed, so the table, and the list of free slots beside it, grow for as long as failures happen. It is in the generated glue (wasm-bindgen ${'0.2.129'}) and not in the library, since the harness's own web-sys calls lose slots in the same way.`,
    details: {
      failures_per_method: FAILURES,
      table,
      losing: losing.map(row => row.method),
      exactly_one_slot_per_failed_object_call: exact
    }
  }
}
