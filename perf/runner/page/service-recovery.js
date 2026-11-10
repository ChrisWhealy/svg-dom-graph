// The service-simulation target of the recovery workload: the sweeps that break a simulator's controller, in the page, and judge what is left.
//
// The graph target (recovery.js) breaks a scene and compares it with a twin and a reference model of a graph. This one breaks a controller
// on its surface, and what it compares is different in kind:
//
//   logical  the committed slice, the engine's hash of its state at that slice, and the hash of the projection that it committed;
//   page     the surface as it can be read back, and the play control's ARIA, which are a projection and never the source of truth;
//   alive    the elements, the closures and the waiting timers that a controller holds.
//
// The model hash is compared with the hash that an independent engine, in JavaScript, made for the same slice (`testing.oracle`), so an
// engine that went wrong cannot agree with itself and a controller that committed the wrong slice is seen. A DOM call is made to fail, one
// at a time, at every place that an operation has one, by perf/page/fault-injector.js, exactly as the graph target does.
//
// A point is judged against the contract in perf/fault-contracts.json. nothing_committed means that the state afterwards is the state
// before. partly_committed means that the logical state is unchanged and the page or the control may differ until a defined repair, which
// is run, and has to restore it. Nothing is timed inside a check, and a recovery time is taken around the call that recovers.

const parse = JSON.parse
const MECHANISM = 'the page clock read before and after the call that recovers, with the checkpoints read after it'
// The DOM calls whose bindings return an object. When one of them throws, wasm-bindgen's glue has already put its result in the table of
// externrefs, and nothing frees the slot, so each failure of one costs a slot for good. See `RETURNS_AN_OBJECT` in recovery.js.
const RETURNS_AN_OBJECT = new Set(['createElementNS', 'createElement', 'appendChild', 'insertBefore', 'getBBox'])

const wait = ms => new Promise(resolve => setTimeout(resolve, ms))

/** The smallest step that the page's clock makes, which every time in a result has to say. */
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

export const makeServiceRecovery = (h, now, testing = {}) => {
  const faults = () => window.__faults
  const call = (fn, ...args) => parse(fn(...args))
  const build = (slot, scenario) => call(h.service_build, JSON.stringify({ scenario }), slot)
  const act = (slot, op) => call(h.service_act, slot, JSON.stringify(op))
  const cp = slot => {
    const result = call(h.service_checkpoint, slot)
    if (!result.ok) throw new Error(`reading slot ${slot}: ${result.error}`)
    return result
  }
  const oracle = testing.oracle?.checkpoints ?? null
  // What the controller holds is read from the controller, and what it should hold comes from outside it: the oracle, and a replay of the
  // run. The live engine may be a slice ahead of the committed one after a failed draw, and then it is compared at its own slice.
  const liveEngineBroken = checkpoint => {
    const broken = []
    if (!checkpoint.committed_model_matches_reference)
      broken.push('the model that the controller holds is not the model of a replay')
    if (checkpoint.engine_recorded_at !== checkpoint.engine_slice)
      broken.push(
        `the controller thinks its engine is at slice ${checkpoint.engine_recorded_at} and the engine is at ${checkpoint.engine_slice}`
      )
    if (![checkpoint.committed_slice, checkpoint.committed_slice + 1].includes(checkpoint.engine_slice))
      broken.push(`the engine is at slice ${checkpoint.engine_slice} and the page is at ${checkpoint.committed_slice}`)
    if (oracle && oracle[checkpoint.engine_slice] !== checkpoint.engine_sha256)
      broken.push('the live engine does not hold the state that the oracle says it should at its slice')
    return broken
  }
  const agreesWithOracle = checkpoint =>
    liveEngineBroken(checkpoint).length > 0
      ? false
      : oracle
        ? oracle[checkpoint.committed_slice] === checkpoint.model_sha256
        : null
  const logical = checkpoint => ({
    committed_slice: checkpoint.committed_slice,
    model_sha256: checkpoint.model_sha256,
    projection_sha256: checkpoint.projection_sha256
  })
  const sameLogical = (a, b) => JSON.stringify(logical(a)) === JSON.stringify(logical(b))
  const until = async (predicate, ms = 3000) => {
    for (let waited = 0; waited < ms; waited += 10) {
      if (predicate()) return true
      await wait(10)
    }
    return predicate()
  }

  /** A controller on `slot` at slice `slice`, reached by clean steps. */
  const at = (slot, scenario, slice) => {
    const built = build(slot, scenario)
    if (!built.ok) throw new Error(`building slot ${slot}: ${built.error}`)
    for (let i = 0; i < slice; i++) {
      const stepped = act(slot, { op: 'next' })
      if (stepped.outcome !== 'done')
        throw new Error(`stepping slot ${slot} to slice ${i + 1}: ${JSON.stringify(stepped)}`)
    }
    return cp(slot)
  }

  const describeCall = c => (c ? [c.method, c.tag, c.name].filter(Boolean).join(' ') : '')
  const methodOf = row => (row.call ?? '').split(' ')[0]

  /** Runs `op` with no fault and counts and describes the DOM calls it made. */
  const dryRun = (slot, op) => {
    faults().record(true)
    const result = act(slot, op)
    faults().record(false)
    return { result, calls: faults().take() }
  }

  /** Runs `op` with the nth DOM call, and `times - 1` after it, made to fail. */
  const faulted = (slot, op, index, times = 1) => {
    const before = h.held_handles().undefinedSlots
    faults().arm(index, times)
    const result = act(slot, op)
    const fired = faults().disarm()
    const slots = h.held_handles().undefinedSlots - before
    const explained = (fired?.all ?? []).reduce((n, c) => n + expectedSlots(c), 0)
    return { result, fired: fired?.fired ?? null, counted: fired?.counted ?? 0, slots, explained }
  }

  const timed = (slot, op) => {
    const started = now()
    const result = act(slot, op)
    return { result, elapsed: now() - started }
  }

  // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
  /** What a call of a draw is, by where it wrote: the data node's setter writes the names of its cells, and the overlay writes its slots. */
  const classify = c =>
    c.tag === 'text' && c.name === 'aria-label'
      ? 'service_data_setter'
      : c.tag === 'g'
        ? 'service_overlay_write'
        : c.tag === 'button'
          ? 'service_control_aria'
          : 'other'

  const expectedSlots = c => (RETURNS_AN_OBJECT.has(c?.method) ? 1 : 0)

  const row = (point, index, c, observed, broken) => ({
    point,
    index,
    call: describeCall(c),
    observed,
    broken,
    ok: broken.length === 0
  })

  const SLICE = 1

  /**
   * Which of the DOM calls of an operation to break. All of them, or in a cycle of an endurance run one call of each fault point that the
   * calls are classified as, a different one from cycle to cycle, so that every point is broken in every cycle and a point that the
   * operation no longer reaches is found missing.
   */
  const pickCalls = (calls, options) => {
    const all = calls.map((_, i) => i + 1)
    if (!options.cycle) return all
    const groups = new Map()
    for (const index of all) {
      const point = classify(calls[index - 1])
      if (point !== 'other') groups.set(point, [...(groups.get(point) ?? []), index])
    }
    return [...groups.values()].map(group => group[(options.cycle - 1) % group.length])
  }

  // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
  /**
   * The draw of the next slice, broken at each of its DOM calls. Every call is the overlay's or the data node's setter's, and a failure at
   * one is repaired at once by the controller, so each is broken twice: once so that the repair works, and once so that the repair meets a
   * failure too, which leaves the page dirty and stopped until a person asks for the repair.
   */
  const drawFrom = (scenario, base, options = {}) => {
    const rows = []
    // The time of a recovery, by the fault point that the call that failed was classified as, so that each point has only its own.
    const times = {}
    const reference = at(1, scenario, base)
    const twin = dryRun(1, { op: 'next' })
    const after = cp(1)
    const calls = twin.calls
    const agrees =
      twin.result.outcome === 'done' && after.committed_slice === base + 1 && agreesWithOracle(after) !== false
    const pick = pickCalls(calls, options)
    for (const index of pick) {
      const c = calls[index - 1]
      const point = classify(c)
      if (point === 'other') continue
      for (const double of [false, true]) {
        const before = at(0, scenario, base)
        const failed = faulted(0, { op: 'next' }, index, double ? 2 : 1)
        const slotsLost = failed.slots
        const now1 = cp(0)
        const broken = []
        const kept = sameLogical(before, now1)
        if (failed.result.outcome !== 'failed') broken.push(`the call reported ${failed.result.outcome} and not failed`)
        if (!kept) broken.push('the committed slice, the model or the projection changed')
        if (agreesWithOracle(now1) === false) broken.push('the committed model differs from the independent oracle')
        if (now1.playing) broken.push('playback was left running')
        if (!double) {
          // The controller's own immediate repair worked, so the page is the committed slice again.
          if (now1.page_sha256 !== before.page_sha256)
            broken.push('the page is not the committed slice after the immediate repair')
          if (now1.dirty) broken.push('the page is still dirty after a repair that worked')
        } else {
          if (!now1.dirty) broken.push('the repair met a failure too and the page was not marked dirty')
        }
        if (failed.slots !== failed.explained) {
          broken.push(
            `${failed.slots} externref slots were lost, and the ${double ? 'failures' : 'failure'} of ${failed.fired?.method} explain ${failed.explained}`
          )
        }
        // The recovery: a person asks for the repair, and then for the slice again, which has to reach what a clean run reaches.
        const recovery = timed(0, { op: 'repair' })
        ;(times[point] ??= []).push(recovery.elapsed)
        const repaired = cp(0)
        if (recovery.result.outcome !== 'done') broken.push(`the repair reported ${recovery.result.outcome}`)
        if (repaired.page_sha256 !== before.page_sha256 || repaired.dirty)
          broken.push('the repair did not put the page back to the committed slice')
        const again = act(0, { op: 'next' })
        const reached = cp(0)
        if (again.outcome !== 'done' || reached.committed_slice !== base + 1)
          broken.push('the next slice could not be committed after the repair')
        if (reached.page_sha256 !== after.page_sha256)
          broken.push('the next slice, after the repair, is not the page that a clean run draws')
        if (reached.data_cells.join(',') !== after.data_cells.join(','))
          broken.push('the data node does not hold what a clean run holds')
        if (reached.elements !== reference.elements)
          broken.push(`the page has ${reached.elements} elements and a clean run has ${reference.elements}`)
        rows.push(
          row(
            point,
            index,
            c,
            {
              double,
              base,
              outcome: failed.result.outcome,
              dirty: now1.dirty,
              kept,
              slots_lost: slotsLost,
              slot_explained: failed.explained
            },
            broken
          )
        )
      }
    }
    teardownAll()
    return { rows, calls: calls.length, agrees, times }
  }

  // The slices that the draws are broken from. A slice changes some of the slots and some of the cells of the data node, so a draw from
  // one slice has DOM calls that a draw from another does not, and the setter is reached at its first, middle and last cell only by
  // drawing from several.
  const BASES = [1, 4, 8]

  /** The draw of the next slice, from each base, and the full redraw of a repair, each broken at every one of its DOM calls. */
  const drawWrites = (scenario, options = {}) => {
    const rows = []
    const times = {}
    let calls = 0
    let agrees = true
    // The data node's setter writes only the cells that changed, so a draw from one base may not reach it, and a cycle breaks all three.
    for (const base of BASES) {
      const swept = drawFrom(scenario, base, options)
      rows.push(...swept.rows)
      for (const [point, taken] of Object.entries(swept.times)) (times[point] ??= []).push(...taken)
      calls += swept.calls
      agrees = agrees && swept.agrees
    }
    // The full redraw that a repair makes writes every field of every slot, and it can meet a failure too: it is broken at each call, and
    // asked for again.
    const repaired = fullRepair(scenario, options)
    return {
      rows: [...rows, ...repaired.rows],
      calls: calls + repaired.calls,
      agrees: agrees && repaired.agrees,
      // The repair that follows a failed repair is a different operation from the repair that follows a failed draw, so its times are
      // reported under a name of their own, and are not added to the points that the failures of the repair's calls are classified as.
      times: { ...times, full_repair: repaired.times }
    }
  }

  // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
  /** The full redraw of the committed slice, which is what a repair is, broken at each of its DOM calls and then asked for again. */
  const fullRepair = (scenario, options = {}) => {
    const rows = []
    const times = []
    const before = at(1, scenario, SLICE)
    const twin = dryRun(1, { op: 'repair' })
    const calls = twin.calls
    const agrees = twin.result.outcome === 'done' && cp(1).page_sha256 === before.page_sha256
    const pick = pickCalls(calls, options)
    for (const index of pick) {
      const c = calls[index - 1]
      const point = classify(c)
      if (point === 'other') continue
      const start = at(0, scenario, SLICE)
      const failed = faulted(0, { op: 'repair' }, index)
      const mid = cp(0)
      const broken = []
      if (failed.result.outcome !== 'failed') broken.push(`the repair reported ${failed.result.outcome} and not failed`)
      if (!sameLogical(start, mid)) broken.push('the committed slice, the model or the projection changed')
      if (!mid.dirty) broken.push('the page was not left dirty after a repair that failed')
      if (mid.playing) broken.push('playback was started by a repair')
      const again = timed(0, { op: 'repair' })
      times.push(again.elapsed)
      const healed = cp(0)
      if (again.result.outcome !== 'done' || healed.dirty) broken.push('asking for the repair again did not succeed')
      if (healed.page_sha256 !== start.page_sha256 || healed.data_cells.join(',') !== start.data_cells.join(','))
        broken.push('the repair that was asked for again did not put the page back')
      if (healed.control_pressed !== 'false' || healed.controls_stale) broken.push('the control was not put right')
      if (failed.slots !== failed.explained)
        broken.push(`${failed.slots} externref slots were lost, and the failure explains ${failed.explained}`)
      rows.push(
        row(
          point,
          index,
          c,
          { repair: true, outcome: failed.result.outcome, slots_lost: failed.slots, slot_explained: failed.explained },
          broken
        )
      )
    }
    teardownAll()
    return { rows, calls: calls.length, agrees, times }
  }

  // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
  /** The building of the surface, broken at each DOM call. A build that fails builds nothing, and leaves nothing in the page. */
  const construction = (scenario, options = {}) => {
    const rows = []
    const baseline = document.body.querySelectorAll('*').length
    const listenersBaseline = window.__listeners?.snapshot().total ?? null
    const twin = (() => {
      faults().record(true)
      const built = build(2, scenario)
      faults().record(false)
      return { built, calls: faults().take() }
    })()
    const clean = twin.built.ok ? cp(2) : null
    call(h.service_teardown, 2)
    const calls = twin.calls
    const agrees = twin.built.ok && clean !== null && clean.committed_slice === 0 && agreesWithOracle(clean) !== false
    // A failure of any call that is not one of the build's own is a draw, so a cycle goes on from the call that it started at until a call
    // has made the build fail, which is the case that this sweep is for.
    const pick = options.cycle
      ? calls.map((_, k) => ((k + options.cycle - 1) % calls.length) + 1)
      : calls.map((_, i) => i + 1)
    for (const index of pick) {
      const c = calls[index - 1]
      const slotsBefore = h.held_handles().undefinedSlots
      faults().arm(index)
      const result = build(2, scenario)
      const fired = faults().disarm()
      const slotsLost = h.held_handles().undefinedSlots - slotsBefore
      const broken = []
      const listenersNow = () => window.__listeners?.snapshot().total ?? null
      let point = 'service_data_node_construction'
      let observed = { built: result.ok, slots_lost: slotsLost, slot_explained: expectedSlots(fired?.fired) }
      if (!result.ok) {
        // The build failed, so it built nothing: no fixture, no element, no listener.
        const nothing = call(h.service_checkpoint, 2)
        if (nothing.ok) broken.push('a fixture exists after a build that failed')
        if (document.body.querySelectorAll('*').length !== baseline)
          broken.push('the failed build left elements in the page')
        if (listenersNow() !== listenersBaseline) broken.push('the failed build left listeners on the page')
      } else {
        // The surface was built and the failure fell in the first draw of the controller, which is a draw like any other: the controller
        // is dirty and says so, its logical state is the first slice, and a repair writes the page right.
        point = classify(c) === 'other' ? 'service_overlay_write' : classify(c)
        const mid = cp(2)
        if (!mid.dirty) broken.push('a write of the first draw failed and the controller was not marked dirty')
        if (mid.committed_slice !== 0 || agreesWithOracle(mid) === false)
          broken.push('the first draw changed the committed slice or model')
        const repair = act(2, { op: 'repair' })
        const healed = cp(2)
        if (repair.outcome !== 'done' || healed.dirty || healed.page_sha256 !== clean?.page_sha256)
          broken.push('the repair did not give what a clean build gives')
        observed = { ...observed, dirty: mid.dirty }
      }
      if (slotsLost !== expectedSlots(fired?.fired))
        broken.push(
          `${slotsLost} externref slots were lost, and one failure of ${fired?.fired?.method} explains ${expectedSlots(fired?.fired)}`
        )
      call(h.service_teardown, 2)
      if (document.body.querySelectorAll('*').length !== baseline)
        broken.push('tearing it down left elements in the page')
      if (listenersNow() !== listenersBaseline) broken.push('tearing it down left listeners on the page')
      if (!result.ok) {
        const retried = build(2, scenario)
        const retriedState = retried.ok ? cp(2) : null
        if (!retried.ok || retriedState.page_sha256 !== clean?.page_sha256)
          broken.push('building again did not give what a clean build gives')
        call(h.service_teardown, 2)
      }
      rows.push(row(point, index, c, observed, broken))
      if (options.cycle && point === 'service_data_node_construction') break
    }
    return { rows, calls: calls.length, agrees, times: [] }
  }

  // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
  /** The writes of the Play control, which Play and Pause make, broken at each. The cursor stays, and a repair writes the control again. */
  const controlWrites = (scenario, options = {}) => {
    const rows = []
    const times = []
    const cases = [
      { name: 'play', prepare: slot => at(slot, scenario, SLICE), op: { op: 'play' }, then: 'running' },
      {
        name: 'pause',
        prepare: slot => {
          const s = at(slot, scenario, SLICE)
          act(slot, { op: 'set_speed', ms: 5000 })
          act(slot, { op: 'play' })
          return s
        },
        op: { op: 'pause' },
        then: 'stopped'
      }
    ]
    let calls = 0
    for (const kind of cases) {
      kind.prepare(1)
      const twin = dryRun(1, kind.op)
      const buttonCalls = twin.calls.map((c, i) => ({ c, i: i + 1 })).filter(x => x.c.tag === 'button')
      calls += buttonCalls.length
      act(1, { op: 'pause' })
      const expectPressed = kind.then === 'running' ? 'true' : 'false'
      const pick = options.cycle
        ? buttonCalls.filter((_, k) => k === (options.cycle - 1) % buttonCalls.length)
        : buttonCalls
      for (const { c, i } of pick) {
        const before = kind.prepare(0)
        if (kind.op.op === 'play') act(0, { op: 'set_speed', ms: 5000 })
        const failed = faulted(0, kind.op, i)
        const broken = []
        const mid = cp(0)
        if (!sameLogical(before, mid)) broken.push('a failed write of the control changed the cursor or the model')
        if (!mid.controls_stale) broken.push('the failed write of the control was not remembered')
        if (mid.control_pressed === expectPressed)
          broken.push('the control already says what it should, so the failed write wrote it')
        const recovery = timed(0, { op: 'repair' })
        times.push(recovery.elapsed)
        const repaired = cp(0)
        if (recovery.result.outcome !== 'done') broken.push(`the repair reported ${recovery.result.outcome}`)
        if (repaired.control_pressed !== expectPressed)
          broken.push(
            `after the repair the control says aria-pressed=${repaired.control_pressed}, and the controller says ${expectPressed}`
          )
        if (repaired.controls_stale) broken.push('the control is still stale after the repair')
        if (repaired.playing !== (kind.then === 'running')) broken.push('the repair started or stopped playback')
        if (!sameLogical(before, repaired)) broken.push('the repair moved the cursor')
        if (failed.slots !== failed.explained)
          broken.push(`${failed.slots} externref slots were lost, and the failure explains ${failed.explained}`)
        act(0, { op: 'pause' })
        rows.push(
          row(
            'service_control_aria',
            i,
            c,
            { op: kind.name, stale: mid.controls_stale, slots_lost: failed.slots, slot_explained: failed.explained },
            broken
          )
        )
      }
    }
    teardownAll()
    return { rows, calls, agrees: true, times: { service_control_aria: times } }
  }

  // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
  /** A page that will not give a timer or a frame, and the host that is detached before a draw or in the middle of one. */
  const hostAndScheduling = async scenario => {
    const rows = []
    const point = (name, index, observed, broken) => rows.push(row(name, index, null, observed, broken))

    // service_timer_request: the browser refuses the timer that Play asks for.
    {
      const before = at(0, scenario, SLICE)
      act(0, { op: 'set_speed', ms: 20 })
      window.__timers.refuseNext()
      const refused = act(0, { op: 'play' })
      window.__timers.clear()
      const mid = cp(0)
      const broken = []
      if (refused.outcome !== 'failed') broken.push(`Play reported ${refused.outcome} and not failed`)
      if (!sameLogical(before, mid)) broken.push('the cursor or the model changed')
      if (mid.playing || mid.waiting_tasks !== 0) broken.push('playback or a timer was left')
      if (!/not|could|refused/.test(mid.announcement ?? '')) broken.push('the failure was not reported to a person')
      const again = act(0, { op: 'play' })
      // This is the browser's own timer, and not a held one: it has to tick twice, so that the timer that a tick asks for is a real one too.
      const moved = await until(() => cp(0).committed_slice > SLICE + 1)
      if (again.outcome !== 'done' || !moved) broken.push('Play did not work once the browser gave a timer')
      act(0, { op: 'pause' })
      point('service_timer_request', 1, { refused: refused.outcome }, broken)
    }

    // service_timer_request, again, for the two other ways that a timer is asked for: the timer that replaces one when the speed changes during
    // playback, and the timer that follows a slice that playback committed. Both have to leave the same state as a refused first timer:
    // playback stopped, no timer waiting, and the display and the model as the last slice left them. The timers are held, so the tick is
    // run by the test and no interval is waited for.
    for (const path of ['a new speed', 'the next slice']) {
      const before = at(0, scenario, SLICE)
      window.__timers.hold(true)
      act(0, { op: 'set_speed', ms: 100 })
      act(0, { op: 'play' })
      const queued = window.__timers.pending()
      const broken = []
      if (queued.length !== 1) broken.push(`${queued.length} timers were queued by Play, and one was expected`)
      window.__timers.refuseNext()
      let outcome = null
      let expectSlice = before.committed_slice
      if (path === 'a new speed') outcome = act(0, { op: 'set_speed', ms: 50 }).outcome
      else {
        window.__timers.fire(queued[0].id)
        expectSlice = before.committed_slice + 1
      }
      window.__timers.clear()
      const mid = cp(0)
      if (outcome !== null && outcome !== 'failed') broken.push(`the change of speed reported ${outcome} and not failed`)
      if (mid.playing) broken.push('playback was left marked as running without a timer')
      if (mid.waiting_tasks !== 0) broken.push(`${mid.waiting_tasks} timers are waiting, and none should be`)
      if (mid.committed_slice !== expectSlice) broken.push(`the committed slice is ${mid.committed_slice} and not ${expectSlice}`)
      if (mid.control_pressed !== 'false') broken.push('the Play control still shows that playback is running')
      if (!/not|could|refused/.test(mid.announcement ?? '')) broken.push('the failure was not reported to a person')
      if (agreesWithOracle(mid) === false) broken.push('the model differs from the oracle')
      point('service_timer_request', path === 'a new speed' ? 2 : 3, { path }, broken)
    }

    // service_frame_request: the browser refuses the frame that a seek asks for.
    {
      const before = at(0, scenario, SLICE)
      window.__frames.refuseNext()
      const refused = act(0, { op: 'seek', target: SLICE + 3 })
      const mid = cp(0)
      const broken = []
      if (refused.outcome !== 'failed') broken.push(`the seek reported ${refused.outcome} and not failed`)
      if (!sameLogical(before, mid)) broken.push('the cursor or the model changed')
      if (mid.pending_retry !== SLICE + 3) broken.push('no retry was offered for the seek that failed')
      if (mid.waiting_tasks !== 0) broken.push('a frame was left waiting')
      const retried = act(0, { op: 'retry' })
      const arrived = await until(() => cp(0).committed_slice === SLICE + 3)
      const end = cp(0)
      if (retried.outcome !== 'pending' || !arrived) broken.push('the retry did not reach the slice')
      if (agreesWithOracle(end) === false) broken.push('the slice that was reached differs from the oracle')
      point('service_frame_request', 1, { refused: refused.outcome }, broken)
    }

    // service_detached_before_draw: the host is out of the document when a draw is asked for.
    {
      const before = at(0, scenario, SLICE)
      const svg = document.getElementById(`perf-service-svg-0`)
      svg.remove()
      const blocked = act(0, { op: 'next' })
      const mid = cp(0)
      const broken = []
      if (blocked.outcome !== 'unavailable') broken.push(`the draw reported ${blocked.outcome} and not unavailable`)
      if (!sameLogical(before, mid)) broken.push('the cursor or the model changed while the host was away')
      if (mid.page_sha256 !== before.page_sha256)
        broken.push('something was written to a host that was not in the document')
      call(h.service_attach, 0)
      act(0, { op: 'presence' })
      const back = cp(0)
      if (back.dirty || back.page_sha256 !== before.page_sha256)
        broken.push('the committed slice was not drawn again on return')
      if (back.playing) broken.push('playback started by itself on return')
      const next = act(0, { op: 'next' })
      if (next.outcome !== 'done' || cp(0).committed_slice !== SLICE + 1)
        broken.push('the next slice could not be committed on return')
      point('service_detached_before_draw', 1, { outcome: blocked.outcome }, broken)
    }

    // service_detached_during_draw: the host leaves after each point of a draw in turn.
    {
      at(0, scenario, SLICE)
      call(h.service_hook, 0, null, false)
      act(0, { op: 'next' })
      const points = parse(h.service_hook_calls(0)).calls
      for (let leave = 0; leave < points; leave++) {
        const before = at(0, scenario, SLICE)
        call(h.service_hook, 0, leave, false)
        const left = act(0, { op: 'next' })
        call(h.service_hook, 0, null, true)
        const mid = cp(0)
        const broken = []
        if (left.outcome !== 'unavailable') broken.push(`the draw reported ${left.outcome} and not unavailable`)
        if (!sameLogical(before, mid)) broken.push('the slice that was drawn as the host left was committed')
        if (!mid.dirty) broken.push('the page was not marked dirty')
        call(h.service_attach, 0)
        act(0, { op: 'presence' })
        const back = cp(0)
        if (back.dirty || back.page_sha256 !== before.page_sha256)
          broken.push('the committed slice was not drawn again on return')
        const next = act(0, { op: 'next' })
        if (next.outcome !== 'done' || cp(0).committed_slice !== SLICE + 1)
          broken.push('the next slice could not be committed on return')
        point('service_detached_during_draw', leave + 1, { leave_after: leave, outcome: left.outcome }, broken)
      }
    }

    // The three that follow decide which callback runs and when. The timers and the frames are held by the injector and run one at a time, so
    // no interval of the wall clock is waited for, and a stale callback is run on purpose, in the order that a test chooses. A microtask
    // turn is waited for, which is how the scheduler releases a closure that has run, and which does not depend on the load of the page.
    const microtasks = async () => {
      for (let turn = 0; turn < 3; turn++) await Promise.resolve()
    }
    const hold = on => {
      window.__timers.hold(on)
      window.__frames.hold(on)
    }
    const release = () => {
      window.__timers.clear()
      window.__frames.clear()
      window.__frames.hold(false)
    }
    // Every held timer is run, in the order asked for, and every held frame.
    const runAll = () => {
      for (const { id } of window.__timers.pending()) window.__timers.fire(id)
      window.__frames.release()
    }

    // service_gallery_hidden: the panel is hidden while playing, with the timer taken back and with it kept, and the timer that was already
    // queued runs.
    for (const kept of [false, true]) {
      const before = at(0, scenario, SLICE)
      hold(true)
      act(0, { op: 'keep_cancelled', on: kept })
      act(0, { op: 'set_speed', ms: 30 })
      act(0, { op: 'play' })
      call(h.service_show_panel, 0, false)
      // The hint is a mutation observer, which is delivered as a microtask at the end of the task that hid the panel, before any timer.
      await microtasks()
      const hiddenAt = cp(0)
      const queued = window.__timers.pending().length
      runAll()
      await microtasks()
      const mid = cp(0)
      const broken = []
      if (queued !== (kept ? 1 : 0))
        broken.push(`${queued} timers were queued after the hide, and ${kept ? 1 : 0} were expected`)
      if (mid.playing || mid.waiting_tasks !== 0) broken.push('playback or a timer outlived the hide')
      if (mid.committed_slice !== hiddenAt.committed_slice || mid.page_sha256 !== hiddenAt.page_sha256)
        broken.push('a callback committed or wrote while the panel was hidden')
      if (window.__timers.pending().length !== 0) broken.push('a callback that ran asked for another timer')
      if (mid.live_closures !== 0) broken.push(`${mid.live_closures} closures are alive`)
      call(h.service_show_panel, 0, true)
      await microtasks()
      const shown = cp(0)
      if (shown.playing || shown.committed_slice !== mid.committed_slice)
        broken.push('showing the panel resumed playback')
      if (shown.dirty || shown.page_sha256 !== mid.page_sha256)
        broken.push('the page was not whole when the panel was shown')
      if (window.__timers.pending().length !== 0) broken.push('showing the panel asked for a timer')
      act(0, { op: 'keep_cancelled', on: false })
      release()
      void before
      point('service_gallery_hidden', kept ? 2 : 1, { kept_callback: kept }, broken)
    }

    // service_teardown_late_tick: the controller is disposed, and a timer and a frame that were already on their way run after it.
    {
      const before = at(0, scenario, SLICE)
      hold(true)
      act(0, { op: 'keep_cancelled', on: true })
      act(0, { op: 'set_speed', ms: 30 })
      act(0, { op: 'play' })
      act(0, { op: 'seek', target: SLICE + 4 })
      act(0, { op: 'dispose' })
      const onTheirWay = { timers: window.__timers.pending().length, frames: window.__frames.pending() }
      runAll()
      await microtasks()
      const mid = cp(0)
      const broken = []
      if (onTheirWay.timers === 0 || onTheirWay.frames === 0)
        broken.push(`${onTheirWay.timers} timers and ${onTheirWay.frames} frames were on their way, so no late callback was run`)
      if (!sameLogical(before, mid)) broken.push('a late callback committed a slice after the disposal')
      if (mid.playing || mid.waiting_tasks !== 0) broken.push('a timer or a frame is still waiting')
      if (window.__timers.pending().length !== 0 || window.__frames.pending() !== 0)
        broken.push('a late callback asked for a timer or a frame')
      if (mid.live_closures !== 0) broken.push(`${mid.live_closures} closures are alive after the callbacks ran`)
      release()
      point('service_teardown_late_tick', 1, { live_closures: mid.live_closures, ...onTheirWay }, broken)
    }

    // service_teardown_late_tick, again: a timer that a change of speed took back runs late, and its epoch is no longer the playback's. It
    // is run before the timer that replaced it, and then after.
    for (const staleFirst of [true, false]) {
      const before = at(0, scenario, SLICE)
      hold(true)
      act(0, { op: 'keep_cancelled', on: true })
      act(0, { op: 'set_speed', ms: 300 })
      act(0, { op: 'play' })
      act(0, { op: 'set_speed', ms: 320 })
      const [stale, current] = window.__timers.pending()
      const broken = []
      if (!stale || !current || stale.delay !== 300 || current.delay !== 320)
        broken.push('the timer that was taken back and the timer that replaced it were not both queued')
      else {
        const afterStale = () => cp(0).committed_slice
        const run = which => window.__timers.fire(which.id)
        if (staleFirst) {
          run(stale)
          if (afterStale() !== before.committed_slice) broken.push('the stale timer committed a slice')
          if (window.__timers.pending().length !== 1) broken.push('the stale timer asked for another timer')
          run(current)
        } else {
          run(current)
          run(stale)
        }
        const mid = cp(0)
        if (mid.committed_slice !== before.committed_slice + 1)
          broken.push(
            `${mid.committed_slice - before.committed_slice} slices were committed by two timers of which one was stale`
          )
        if (window.__timers.pending().length !== 1)
          broken.push(`${window.__timers.pending().length} timers are queued, and the playback should have exactly one`)
        if (agreesWithOracle(mid) === false) broken.push('the slice that was reached differs from the oracle')
      }
      act(0, { op: 'pause' })
      window.__timers.pending().forEach(({ id }) => window.__timers.fire(id))
      act(0, { op: 'keep_cancelled', on: false })
      release()
      point('service_teardown_late_tick', staleFirst ? 2 : 3, { stale_first: staleFirst }, broken)
    }
    // The browser's own timer and frame are used by the two sweeps above that wait for one: Play with a timer that the browser gave, and a
    // seek that is asked for again. They poll for the result with a generous limit, and do not assert on an interval.
    teardownAll()
    return { rows, calls: rows.length, agrees: true, times: [] }
  }

  const teardownAll = () => {
    for (const slot of [0, 1, 2]) h.service_teardown(slot)
  }

  const sweeps = { drawWrites, construction, controlWrites, hostAndScheduling }

  /** Judges the rows of one point against its contract. */
  const judge = (point, expect, rows) => {
    const mine = rows.filter(r => r.point === point)
    if (mine.length === 0) return { status: 'inconclusive', detail: `no failure was injected at ${point}`, broken: [] }
    const broken = mine.filter(r => !r.ok).flatMap(r => r.broken.map(b => `call ${r.index} (${r.call}): ${b}`))
    return broken.length === 0
      ? {
          status: 'pass',
          detail: `all ${mine.length} failures injected at ${point} left what ${expect} says, and ${expect === 'partly_committed' ? 'the repair put the rest right' : 'the state was the state before'}`,
          broken: []
        }
      : { status: 'fail', detail: `${broken.length} of ${mine.length} failures did not: ${broken[0]}`, broken }
  }

  return { sweeps, judge, teardownAll, faults, cp, at, act, build, classify }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const GROUPS = [
  { sweep: 'construction', points: ['service_data_node_construction'] },
  { sweep: 'drawWrites', points: ['service_overlay_write', 'service_data_setter'] },
  { sweep: 'controlWrites', points: ['service_control_aria'] },
  {
    sweep: 'hostAndScheduling',
    points: [
      'service_timer_request',
      'service_frame_request',
      'service_detached_before_draw',
      'service_detached_during_draw',
      'service_gallery_hidden',
      'service_teardown_late_tick'
    ]
  }
]

const sectionOf = row =>
  row.point === 'service_control_aria'
    ? 'accessibility'
    : row.broken.some(b => /element|closure|listener|slot/.test(b))
      ? 'dom_identity'
      : 'content'

/**
 * Breaks every service fault point of the manifest, and judges each against its contract. Async, since a timer and a frame are real.
 * `testing.oracle.checkpoints[k]` is the independent engine's hash of its state at slice k.
 */
export const runServiceRecovery = async (h, manifest, testing = {}) => {
  const R = makeServiceRecovery(h, () => performance.now(), testing)
  const scenario = manifest.recovery.scenario
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
    stream: null
  }
  outcome.timer_resolution_us = resolutionOf()
  const metric = (name, microseconds) =>
    parse(h.describe_metric(name, 'B', 'us', JSON.stringify(microseconds), MECHANISM))
  h.service_forget()
  const bodyBefore = document.body.querySelectorAll('*').length
  const listenersBefore = window.__listeners?.snapshot().total ?? null
  const slotsBefore = h.held_handles().undefinedSlots

  // The independent oracle has to agree with the engine for the slices that the sweeps reach, before any of them is judged.
  if (testing.oracle) {
    const built = R.build(0, scenario)
    const slices = []
    if (built.ok) {
      for (let k = 0; k < 8; k++) {
        const c = R.cp(0)
        slices.push(
          testing.oracle.checkpoints[c.committed_slice] === c.model_sha256 &&
            testing.oracle.checkpoints[c.engine_slice] === c.engine_sha256 &&
            c.committed_model_matches_reference
        )
        if (R.act(0, { op: 'next' }).outcome !== 'done') break
      }
    }
    h.service_teardown(0)
    outcome.gates.push({
      name: 'recovery.service.engine_agrees_with_the_independent_oracle',
      status: slices.length > 0 && slices.every(Boolean) ? 'pass' : 'fail',
      detail: `the engine's hash of its state agrees with the independent engine's at ${slices.filter(Boolean).length} of ${slices.length} slices that were reached`
    })
  } else {
    outcome.notes.push(
      'no independent oracle was supplied, so the model hash of a committed slice is not compared with one'
    )
  }

  // Every group is swept first, since a group can break a call that belongs to a point of another: the full redraw of a repair writes the
  // Play control, for one. The points are judged afterwards, over every row that was found for them.
  const rows = []
  const timesOf = {}
  for (const group of GROUPS) {
    if (!group.points.some(point => wanted.some(p => p.point === point))) continue
    let swept
    try {
      swept = await R.sweeps[group.sweep](scenario)
    } catch (error) {
      outcome.status = 'error'
      outcome.error = `${group.sweep}: ${error.message}`
      R.teardownAll()
      continue
    }
    rows.push(...swept.rows)
    for (const [point, times] of Object.entries(Array.isArray(swept.times) ? {} : swept.times))
      (timesOf[point] ??= []).push(...times)
    outcome.gates.push({
      name: `recovery.service.${group.sweep}.a_run_with_no_fault_agrees_with_the_oracle`,
      status: swept.agrees ? 'pass' : 'fail',
      detail: swept.agrees
        ? 'the run that made the operation with no fault commits the slice that the oracle expects'
        : 'the run with no fault does not reach what the oracle expects, so it is no reference'
    })
    outcome.notes.push(
      `${group.sweep}: ${swept.calls} ${group.sweep === 'hostAndScheduling' ? 'situations' : 'DOM calls'}, and each was broken`
    )
  }
  for (const { point, expect } of wanted) {
    const mine = rows.filter(r => r.point === point)
    const times = timesOf[point] ?? []
    if (expect === 'undocumented') {
      outcome.findings.push({
        point,
        contract: 'undocumented',
        summary: `${mine.length} failures were injected at ${point}, and ${mine.filter(r => !r.ok).length} left something other than what a clean run leaves`,
        details: { rows: mine }
      })
      continue
    }
    const verdict = R.judge(point, expect, rows)
    outcome.gates.push({
      name: `recovery.${point}.matches_the_contract`,
      status: verdict.status,
      detail: verdict.detail
    })
    for (const r of mine) {
      if (r.ok) outcome.checkpoints_passed++
      else
        outcome.checkpoints_failed.push({
          section: sectionOf(r),
          detail: `${point}: call ${r.index} (${r.call}): ${r.broken[0]}`
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
            reason:
              'this point has no repair to time: it is a situation, or nothing was committed and so nothing needed one'
          }
    )
    outcome.metrics.push({
      name: `recovery.${point}.failures_injected`,
      layer: 'B',
      status: 'measured',
      unit: 'count',
      kind: 'count',
      value: mine.length,
      mechanism: 'DOM calls and situations of the operation made to fail one at a time'
    })
  }

  // What the sweeps left behind: nothing, and the slots that failed object-returning calls explain, and no more.
  R.teardownAll()
  h.service_forget()
  const elementsAfter = document.body.querySelectorAll('*').length
  const listenersAfter = window.__listeners?.snapshot().total ?? null
  outcome.gates.push({
    name: 'recovery.service.dom_returns_to_baseline',
    status: elementsAfter === bodyBefore ? 'pass' : 'fail',
    detail:
      elementsAfter === bodyBefore
        ? `the document held ${bodyBefore} elements before the sweeps and after them`
        : `the document held ${bodyBefore} elements before the sweeps and ${elementsAfter} after them`
  })
  outcome.gates.push({
    name: 'recovery.service.listeners_return_to_baseline',
    status: listenersBefore === null ? 'inconclusive' : listenersAfter === listenersBefore ? 'pass' : 'fail',
    detail:
      listenersBefore === null
        ? 'the listener tracker is not in this page'
        : `the effective listener count was ${listenersBefore} before the sweeps and ${listenersAfter} after them`
  })
  // For the self-test: a failed call of the harness's own, which no row records, loses a slot that nothing explains.
  if (testing.unexplainedSlotLoss) {
    window.__faults.arm(1)
    R.build(2, scenario)
    window.__faults.disarm()
    h.service_teardown(2)
  }
  // The repair that follows a failed repair is timed under a name of its own, since it belongs to no single fault point.
  for (const [name, times] of Object.entries(timesOf)) {
    if (wanted.some(p => p.point === name) || times.length === 0) continue
    outcome.samples[`recovery.service.${name}.time`] = times.map(t => t * 1000)
    outcome.metrics.push(
      metric(
        `recovery.service.${name}.time`,
        times.map(t => t * 1000)
      )
    )
  }
  const slotsLost = h.held_handles().undefinedSlots - slotsBefore
  const explained = rows.reduce((n, r) => n + (r.observed?.slot_explained ?? 0), 0)
  const measured = rows.reduce((n, r) => n + (r.observed?.slots_lost ?? 0), 0)
  // Three quantities have to agree: what was lost over the whole run, what the rows measured, and what the failed calls explain. A slot
  // lost outside every row makes the first larger than the others, and no row sees it.
  const rowsClean = rows.every(r => !r.broken.some(b => /externref/.test(b)))
  const slotsAgree = slotsLost === explained && measured === explained
  outcome.gates.push({
    name: 'recovery.service.every_row_loses_exactly_the_slots_its_failure_explains',
    status: rowsClean && slotsAgree ? 'pass' : 'fail',
    detail: slotsAgree
      ? `${slotsLost} externref slots were lost over the run, ${measured} in the rows that measured it, and the failed calls that return an object explain ${explained}`
      : `${slotsLost} externref slots were lost over the run and ${measured} in the rows that measured it, and the failed calls that return an object explain ${explained}`
  })
  outcome.metrics.push({
    name: 'wasm.externref_slots_lost',
    layer: 'B',
    status: 'measured',
    unit: 'count',
    kind: 'count',
    value: slotsLost,
    mechanism:
      "slots of wasm-bindgen's externref table that hold undefined and are never freed, read before the first sweep and after the last"
  })
  outcome.cases = rows
  if (outcome.checkpoints_failed.length > 0 && outcome.status === 'ok') outcome.status = 'invalid_incorrect'
  return outcome
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Endurance: every fault point of the manifest is broken and recovered from in each cycle, a different DOM call from cycle to cycle, and
 * what the cycles leave behind is read after each is torn down. The gates are the harness's own: live memory does not grow, the
 * document and the listeners return to where they began, and the externref slots that were lost are exactly those that the failed calls
 * which return an object explain. A controller that kept a callback, a listener or an element would be seen here.
 */
export const runServiceEndurance = async (h, manifest, testing = {}) => {
  const R = makeServiceRecovery(h, () => performance.now(), testing)
  const { warmup_iterations: warmup, measurement_iterations: measured } = manifest.execution
  const total = warmup + measured
  const allowance = manifest.limits.live_growth_allowance_bytes ?? 0
  const wanted = manifest.recovery.points
  const scenario = manifest.recovery.scenario
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
  const reading = () => ({
    ...parse(h.resource_reading()),
    listeners: window.__listeners?.snapshot().total ?? null,
    elements: document.querySelectorAll('*').length,
    handles: h.held_handles?.().held ?? null
  })
  h.service_forget()
  // The slab of externref slots is made as large as the run will need before anything is read, since it doubles as slots are lost and a
  // doubling is not growth that the controller caused. A cycle breaks a few hundred calls at most.
  h.reserve_handles(2 ** Math.ceil(Math.log2(total * 400 + 4096)))
  const baseline = reading()
  const slotsAtStart = h.held_handles().undefinedSlots
  let slotsExplained = 0
  const series = []
  const unmet = []
  // How many failures were injected at each point over the run, so that the recoveries that are claimed can be reconciled with the
  // operations that were exercised.
  const injected = Object.fromEntries(wanted.filter(p => p.expect !== 'undocumented').map(p => [p.point, 0]))
  for (let cycle = 1; cycle <= total; cycle++) {
    const rows = []
    for (const group of GROUPS) {
      if (!group.points.some(point => wanted.some(p => p.point === point))) continue
      rows.push(...(await R.sweeps[group.sweep](scenario, { cycle: testing.pin ?? cycle })).rows)
    }
    slotsExplained += rows.reduce((n, r) => n + (r.observed?.slot_explained ?? 0), 0)
    if (testing.withholdPoint) rows.splice(0, rows.length, ...rows.filter(r => r.point !== testing.withholdPoint))
    for (const { point, expect } of wanted.filter(p => p.expect !== 'undocumented')) {
      const verdict = R.judge(point, expect, rows)
      injected[point] += rows.filter(r => r.point === point).length
      // A point that was not broken in a cycle is a gap in what the cycle exercised, and is a failure and not a skipped observation.
      if (verdict.status !== 'pass') unmet.push(`cycle ${cycle}, ${point}: ${verdict.detail}`)
    }
    R.teardownAll()
    h.service_forget()
    if (testing.leakListener) document.body.addEventListener('perf-leak', () => {})
    if (testing.leakElement) document.body.appendChild(document.createElement('div'))
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
  outcome.gates.push({ name: 'endurance.live_bytes_do_not_grow', status: verdict.status, detail: verdict.detail })
  const injectedTotal = Object.values(injected).reduce((n, count) => n + count, 0)
  const perPoint = Object.entries(injected)
    .map(([point, count]) => `${point.replace('service_', '')} ${count}`)
    .join(', ')
  outcome.gates.push({
    name: 'endurance.recoveries_succeed',
    status: unmet.length === 0 && injectedTotal > 0 ? 'pass' : 'fail',
    detail:
      unmet.length === 0
        ? `${injectedTotal} failures were injected over ${total} cycles (${perPoint}), every point was broken in every cycle, and each recovery reached the state the contract promises`
        : `${unmet.length} of ${total * Object.keys(injected).length} point-cycles did not pass, the first being ${unmet[0]}`
  })
  for (const [point, count] of Object.entries(injected)) {
    outcome.metrics.push({
      name: `recovery.${point}.failures_injected`,
      layer: 'B',
      status: 'measured',
      unit: 'count',
      kind: 'count',
      value: count,
      mechanism: 'DOM calls and situations of the operation made to fail one at a time, over every cycle'
    })
  }
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
  const slotsLost = h.held_handles().undefinedSlots - slotsAtStart
  outcome.gates.push({
    name: 'endurance.lost_externref_slots_are_explained',
    status: slotsLost === slotsExplained ? 'pass' : 'fail',
    detail:
      slotsLost === slotsExplained
        ? `${slotsLost} externref slots were lost over ${total} cycles, one for each of the ${slotsExplained} failures of a DOM call that returns an object, and none for the others`
        : `${slotsLost} externref slots were lost, and ${slotsExplained} failures of a call that returns an object explain only those`
  })
  outcome.metrics.push({
    name: 'wasm.externref_slots_lost',
    layer: 'B',
    status: 'measured',
    unit: 'count',
    kind: 'count',
    value: slotsLost,
    mechanism:
      "slots of wasm-bindgen's externref table that hold undefined and are never freed, read before the first cycle and after the last"
  })
  outcome.metrics.push({
    name: 'lifecycle.measured_cycles_sampled',
    layer: 'B',
    status: 'measured',
    unit: 'count',
    kind: 'count',
    value: verdict.measured_points,
    mechanism: 'the counting allocator and the document, read after the cycle was torn down'
  })
  outcome.metrics.push({
    name: 'recovery.cycles',
    layer: 'B',
    status: 'measured',
    unit: 'count',
    kind: 'count',
    value: total,
    mechanism: 'cycles in which every fault point of the manifest was broken and recovered from'
  })
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
  outcome.checkpoints_passed = Math.max(0, injectedTotal - unmet.length)
  for (const line of unmet) outcome.checkpoints_failed.push({ section: 'dom_identity', detail: line })
  if (outcome.checkpoints_failed.length > 0) outcome.status = 'invalid_incorrect'
  return outcome
}
