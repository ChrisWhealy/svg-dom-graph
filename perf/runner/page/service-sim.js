// Workload W6 on Layer B: the simulator's controller on its surface, run cleanly and timed, in the page.
//
// The page's wasm gives primitives (service_perf.rs) and this orchestrates them. The trace is fixed before anything runs, by the independent
// JavaScript engine, and what it says each slice's hash is comes in as `trace.checkpoints`, with the popped events of each slice. The
// controller's own engine is read where it is (`engine_sha256`) and is held to that hash after every slice that a check looks at, so a
// controller that committed the wrong slice of the wrong history is seen whatever it drew.
//
//   correctness  every slice of the trace is committed with the controller as a person's page has it. Each commit has to leave the controller's
//                engine at the independent engine's hash, the page changed (a stale-only slice still changes the data node's counters), the
//                data node's setter called once more, and the page and a full redraw of it equal at the slices that are checked.
//   performance  slices 1 to 256 are untimed priming. The next 10 batches of four slices are untimed warm-up and the 60 after them are
//                measured. The four slices of a batch are worked out first. Then the clock is read, four `next` calls are made and the clock is
//                read again, and every check is made after. No layout is read inside it, and no fault is injected anywhere.
//   endurance    the page is built, driven, hidden, shown and removed again and again, and what it leaves behind is counted.
//
// A mismatch makes the run invalid_incorrect, and its timings are then never used.

const parse = JSON.parse
const WARMUP_BATCHES_SLICES = [257, 296]
const PRIMING = 256
const BATCH = 4

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

const microtasks = async () => {
  for (let turn = 0; turn < 3; turn++) await Promise.resolve()
}

const newOutcome = () => ({
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
})

const sha256Of = async text => {
  const bytes = new TextEncoder().encode(text)
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes))
  return [...digest].map(b => b.toString(16).padStart(2, '0')).join('')
}

/** The chain hash of a list of checkpoints, as the orchestrator and the native runner make it. */
const chainOf = checkpoints => sha256Of(checkpoints.join('\n'))

const makeChecks = (outcome, trace) => {
  const fail = (section, detail) => {
    if (outcome.checkpoints_failed.length < 50) outcome.checkpoints_failed.push({ section, detail })
    else
      outcome.checkpoints_failed.length === 50 &&
        outcome.notes.push('more failed checkpoints than the 50 that are kept')
  }
  const pass = () => outcome.checkpoints_passed++
  /** The controller's own engine against the independent engine's hash at slice `k`. */
  const engineAt = (state, k, what) => {
    let good = true
    if (state.committed_slice !== k) {
      fail('content', `${what}: the committed slice is ${state.committed_slice} and not ${k}`)
      good = false
    }
    if (state.engine_slice !== k || state.engine_recorded_at !== k) {
      fail(
        'content',
        `${what}: the controller's engine is at slice ${state.engine_slice} and it records ${state.engine_recorded_at}, and ${k} was expected`
      )
      good = false
    }
    if (state.engine_sha256 !== trace.checkpoints[k]) {
      fail('content', `${what}: the live engine's hash is not the independent engine's at slice ${k}`)
      good = false
    }
    if (state.dirty || !state.surface_trusted) {
      fail('content', `${what}: the page is dirty or its surface is not trusted after slice ${k}`)
      good = false
    }
    if (good) pass()
    return good
  }
  return { fail, pass, engineAt }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** Every slice of the trace, committed one at a time, with the controller as a person's page has it. */
export const runSimulationCorrectness = async (h, manifest, trace) => {
  const outcome = newOutcome()
  const { fail, pass, engineAt } = makeChecks(outcome, trace)
  const slices = trace.counters.slices
  const built = parse(
    h.sim_build(JSON.stringify({ seed: manifest.seed_text, arrivals: manifest.simulation.arrivals, product: true }))
  )
  if (!built.ok) {
    outcome.status = 'error'
    outcome.error = built.error
    return outcome
  }
  try {
    let previous = parse(h.sim_state())
    engineAt(previous, 0, 'the first slice')
    const initialSetter = previous.setter_calls
    const executed = [previous.engine_sha256]
    let staleOnly = 0
    let staleOnlyChanged = 0
    let deltaChecks = 0
    let deltaEqualsFull = 0
    // A full redraw writes the data node once more, and it is a repair and not a displayed slice, so the setter count that is expected moves
    // by one for each that was asked for.
    let repairs = 0
    for (let k = 1; k <= slices; k++) {
      const stepped = parse(h.sim_step(1))
      if (!stepped.ok || stepped.last !== 'done') {
        fail('content', `slice ${k}: next reported ${stepped.last ?? stepped.error}`)
        break
      }
      const state = parse(h.sim_state())
      engineAt(state, k, `slice ${k}`)
      if (!state.page_is_projection)
        fail('content', `slice ${k}: the page, read back from its elements, is not the projection that was committed`)
      else pass()
      executed.push(state.engine_sha256)
      // Every slice changes the page, and a slice of stale pops only still changes the data node, since a slice is counted there. A
      // content mutation that the attribution cannot see would make a measurement of nothing.
      if (state.page_sha256 === previous.page_sha256) fail('content', `slice ${k}: the page did not change`)
      else pass()
      const onlyStale = trace.slice_pops[k - 1] === trace.slice_stale[k - 1]
      if (onlyStale) {
        staleOnly++
        if (state.data_cells.join(',') !== previous.data_cells.join(',')) staleOnlyChanged++
        else fail('content', `slice ${k}: every pop was stale and the data node did not change`)
      }
      if (state.setter_calls !== initialSetter + k + repairs)
        fail(
          'content',
          `slice ${k}: the data node's setter has returned ${state.setter_calls - initialSetter - repairs} times for ${k} slices`
        )
      else pass()
      // A delta draw and a full redraw of the same slice have to leave the same page, which is checked often enough to cover each
      // window's ends and every kind of slice, and not at each slice, since a redraw is far dearer than a delta.
      if (k % 16 === 0 || k === PRIMING || k === WARMUP_BATCHES_SLICES[1] || k === 536) {
        deltaChecks++
        const repaired = parse(h.sim_repair())
        repairs++
        const after = parse(h.sim_state())
        if (
          repaired.outcome !== 'done' ||
          after.page_sha256 !== state.page_sha256 ||
          after.projection_sha256 !== state.projection_sha256
        ) {
          fail('content', `slice ${k}: a full redraw does not leave the page that the delta draws left`)
        } else {
          deltaEqualsFull++
          pass()
        }
        previous = after
      } else previous = state
    }
    const end = parse(h.sim_state())
    const stream = await chainOf(executed)
    const wanted = await chainOf(trace.checkpoints)
    outcome.stream = {
      scope: 'full',
      operations: slices,
      sha256: stream,
      expected_sha256: wanted,
      matches: stream === wanted
    }
    const displayed = slices
    const setterForSlices = end.setter_calls - initialSetter - repairs
    outcome.gates.push({
      name: 'simulation.setter_updates_every_displayed_slice',
      status: setterForSlices === displayed ? 'pass' : 'fail',
      detail: `the data node's setter returned without an error ${setterForSlices} times for ${displayed} displayed slices`
    })
    // The release gate asks for at least 1,000 successful commits. A trace of about 1,000 pops has fewer slices than that and does not make the
    // claim, so the gate is made only by a manifest that asks for the 10,000 pops of the smoke trace or more.
    if (manifest.simulation.min_pops >= 10000)
      outcome.gates.push({
        name: 'simulation.setter_commits_reach_the_release_minimum',
        status: setterForSlices >= 1000 ? 'pass' : 'fail',
        detail: `${setterForSlices} successful commits of the data node's setter, and the release gate requires at least 1000`
      })
    outcome.gates.push({
      name: 'simulation.stale_only_slices_change_the_page',
      status: staleOnly === 0 ? 'inconclusive' : staleOnly === staleOnlyChanged ? 'pass' : 'fail',
      detail:
        staleOnly === 0
          ? 'the trace has no slice of stale pops only, so the claim was not tested'
          : `${staleOnlyChanged} of the ${staleOnly} slices of stale pops only changed the data node's counters`
    })
    outcome.gates.push({
      name: 'simulation.delta_draw_equals_full_redraw',
      status: deltaChecks > 0 && deltaChecks === deltaEqualsFull ? 'pass' : 'fail',
      detail: `${deltaEqualsFull} of ${deltaChecks} checked slices left the same page after a full redraw as after a delta draw`
    })
    outcome.metrics.push({
      name: 'simulation.actual_pops',
      layer: 'B',
      status: 'measured',
      unit: 'count',
      kind: 'count',
      value: trace.counters.pops,
      mechanism:
        "the independent engine's count of popped heap events, stale ones included, over the whole trace that was committed"
    })
  } finally {
    h.sim_teardown()
  }
  if (outcome.checkpoints_failed.length > 0) outcome.status = 'invalid_incorrect'
  return outcome
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const MECHANISM =
  'performance.now read before the first of four synchronous next calls and after the last, the four slices having been worked out first: the data node setter, the overlay writes, the connectedness guards and the commit bookkeeping are inside it, and no layout is read'

/** The batches of four slices of the fixed windows, timed. */
export const runSimulationPerformance = async (h, manifest, trace, testing = {}) => {
  const outcome = newOutcome()
  const { fail, pass, engineAt } = makeChecks(outcome, trace)
  const warmup = manifest.execution.warmup_iterations
  const measured = manifest.execution.measurement_iterations
  // `testing.product` builds the fixture as a person's page has it, whose test of visibility reads layout, to show that the gate on that bites.
  const built = parse(
    h.sim_build(
      JSON.stringify({
        seed: manifest.seed_text,
        arrivals: manifest.simulation.arrivals,
        product: testing.product === true
      })
    )
  )
  if (!built.ok) {
    outcome.status = 'error'
    outcome.error = built.error
    return outcome
  }
  const microseconds = []
  const executed = []
  try {
    const start = parse(h.sim_state())
    executed.push(start.engine_sha256)
    engineAt(start, 0, 'the first slice')
    // The priming slices are not timed and are not a steady state. Each is read as it is committed, so that the digest of what ran covers them.
    let state = start
    for (let k = 1; k <= PRIMING; k++) {
      const stepped = parse(h.sim_step(1))
      if (!stepped.ok || stepped.done !== 1) {
        outcome.status = 'error'
        outcome.error = `priming stopped at slice ${k}: ${stepped.last ?? stepped.error}`
        return outcome
      }
      state = parse(h.sim_state())
      executed.push(state.engine_sha256)
    }
    engineAt(state, PRIMING, 'the end of priming')
    let cursor = PRIMING
    let firstMeasured = null
    const readsLayout = state.reads_layout
    let setterBefore = state.setter_calls
    let poppedInBatches = 0
    let engineSteppedInBatches = 0
    for (let number = 0; number < warmup + measured; number++) {
      const timed = number >= warmup
      if (timed && firstMeasured === null) firstMeasured = cursor + 1
      const result = parse(h.sim_batch(BATCH))
      if (!result.ok) {
        outcome.status = 'error'
        outcome.error = `batch ${number}: ${result.error}`
        return outcome
      }
      if (result.engine_stepped_in_interval) engineSteppedInBatches++
      const slices = Array.from({ length: BATCH }, (_, i) => cursor + 1 + i)
      cursor += BATCH
      state = parse(h.sim_state())
      const good = engineAt(state, cursor, `the batch ending at slice ${cursor}`)
      // What the batch drew is read back from the page, outside the timed interval, and held to the projection that was committed. A draw that
      // skipped a write and still returned success would otherwise make the timing cheaper without failing anything.
      if (!state.page_is_projection)
        fail(
          'content',
          `the batch ending at slice ${cursor}: the page, read back from its elements, is not the projection that was committed`
        )
      else pass()
      if (result.outcomes.some(o => o !== 'done'))
        fail('content', `batch ending at slice ${cursor}: ${result.outcomes.join(', ')}`)
      const setterCalls = state.setter_calls - setterBefore
      setterBefore = state.setter_calls
      if (setterCalls !== BATCH)
        fail(
          'content',
          `batch ending at slice ${cursor}: the data node's setter returned ${setterCalls} times for ${BATCH} slices`
        )
      else pass()
      const pops = slices.reduce((n, k) => n + trace.slice_pops[k - 1], 0)
      const stale = slices.reduce((n, k) => n + trace.slice_stale[k - 1], 0)
      poppedInBatches += pops
      // The engine's own hash after each slice of the batch, and each is held to the independent engine's, so a slice inside a batch is seen
      // and not only the last. The digest of what ran is of these, and never of the trace's own.
      slices.forEach((k, i) => {
        const actual = result.checkpoints?.[i]
        if (actual !== trace.checkpoints[k])
          fail(
            'content',
            `the batch ending at slice ${cursor}: the engine's hash after slice ${k} is not the independent engine's`
          )
        else pass()
        executed.push(actual ?? '')
      })
      if (timed) {
        const us = result.elapsed_ms * 1000
        microseconds.push(us)
        outcome.cases.push({
          observation: number - warmup,
          slices,
          pops,
          stale_pops: stale,
          setter_calls: setterCalls,
          end_page_sha256: state.page_sha256,
          end_engine_sha256: state.engine_sha256,
          microseconds: us
        })
      }
    }
    const executedChain = await chainOf(executed)
    const wantedChain = await chainOf(trace.checkpoints.slice(0, cursor + 1))
    outcome.stream = {
      scope: 'mutations',
      operations: cursor,
      sha256: executedChain,
      expected_sha256: wantedChain,
      matches: executedChain === wantedChain
    }
    const metric = parse(
      h.describe_metric('simulation.render_commit.four_slices', 'B', 'us', JSON.stringify(microseconds), MECHANISM)
    )
    if (
      metric.status === 'measured' &&
      metric.kind === 'latency' &&
      metric.unit === 'us' &&
      outcome.timer_resolution_us > 0 &&
      metric.summary.p50 < 50 * outcome.timer_resolution_us
    )
      metric.resolution_limited = true
    outcome.metrics.push(metric)
    outcome.samples['simulation.render_commit.four_slices'] = microseconds
    outcome.metrics.push({
      name: 'simulation.actual_pops_in_batches',
      layer: 'B',
      status: 'measured',
      unit: 'count',
      kind: 'count',
      value: poppedInBatches,
      mechanism:
        "the independent engine's popped heap events, stale ones included, over every warm-up and measured batch"
    })
    // The windows are fixed, and the trace says where: the warm-up one begins at slice 257, and the measured one after the manifest's warm-up
    // batches. The page's own priming count is held to that, and not to itself, so a window that is wrapped, reused or shifted is seen.
    const startsAt = trace.windows.warmup[0] + warmup * BATCH
    const endsAt = startsAt + measured * BATCH - 1
    outcome.gates.push({
      name: 'simulation.windows_are_not_wrapped',
      status: firstMeasured === startsAt && cursor === endsAt && cursor <= 536 ? 'pass' : 'fail',
      detail: `${warmup} warm-up and ${measured} measured batches of ${BATCH} slices: the measured ones began at slice ${firstMeasured} and ended at ${cursor}, and the fixed windows have them begin at ${startsAt} and end at ${endsAt}, inside the 536 slices`
    })
    outcome.gates.push({
      name: 'simulation.engine_is_not_stepped_in_a_timed_interval',
      status: engineSteppedInBatches === 0 ? 'pass' : 'fail',
      detail:
        engineSteppedInBatches === 0
          ? 'the four slices of every batch were worked out before the clock was read, so the interval is the render and the commit'
          : `the engine was stepped inside the timed interval of ${engineSteppedInBatches} batches, so those timings hold engine work`
    })
    outcome.gates.push({
      name: 'simulation.no_layout_is_read_in_a_timed_interval',
      status: readsLayout === false ? 'pass' : 'fail',
      detail:
        readsLayout === false
          ? 'the controller under test asks whether the host may be drawn on by a cached activation flag and isConnected, and never by checkVisibility or a bounding box'
          : 'the controller under test reads layout to decide whether it may draw, so a timed batch would include it'
    })
  } finally {
    h.sim_teardown()
  }
  if (outcome.checkpoints_failed.length > 0) outcome.status = 'invalid_incorrect'
  return outcome
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** Builds, uses, hides, shows and removes the page again and again, and counts what it leaves behind. */
export const runSimulationEndurance = async (h, manifest, trace) => {
  const outcome = newOutcome()
  const { fail, pass, engineAt } = makeChecks(outcome, trace)
  const { warmup_iterations: warmup, measurement_iterations: measured } = manifest.execution
  const total = warmup + measured
  const allowance = manifest.limits.live_growth_allowance_bytes ?? 0
  const reading = () => ({
    ...parse(h.resource_reading()),
    listeners: window.__listeners?.snapshot().total ?? null,
    elements: document.querySelectorAll('*').length,
    handles: h.held_handles?.().held ?? null
  })
  h.sim_teardown()
  h.reserve_handles(2 ** Math.ceil(Math.log2(total * 400 + 4096)))
  const baseline = reading()
  const slotsAtStart = h.held_handles().undefinedSlots
  const series = []
  const slicesPerCycle = 64
  const cycleEnds = []
  for (let cycle = 1; cycle <= total; cycle++) {
    const built = parse(
      h.sim_build(JSON.stringify({ seed: manifest.seed_text, arrivals: manifest.simulation.arrivals, product: true }))
    )
    if (!built.ok) {
      outcome.status = 'error'
      outcome.error = `cycle ${cycle}: ${built.error}`
      return outcome
    }
    const stepped = parse(h.sim_step(slicesPerCycle))
    if (!stepped.ok || stepped.done !== slicesPerCycle)
      fail('content', `cycle ${cycle}: ${stepped.done} of ${slicesPerCycle} slices were committed`)
    const atEnd = parse(h.sim_state())
    engineAt(atEnd, slicesPerCycle, `cycle ${cycle}`)
    cycleEnds.push(atEnd.engine_sha256)
    // Play from the real control, and a hide of the page that the gallery's hint has to see, and a show. Playback must stop, and showing the
    // page must not start it again.
    document.getElementById('perf-sim-play').click()
    const playing = parse(h.sim_state())
    if (!playing.playing) fail('content', `cycle ${cycle}: the Play control did not start playback`)
    document.getElementById('perf-sim-stage').style.display = 'none'
    document.getElementById('perf-sim-stage').classList.toggle('hidden')
    await microtasks()
    document.getElementById('perf-sim-stage').style.display = ''
    document.getElementById('perf-sim-stage').classList.toggle('hidden')
    await microtasks()
    const settled = parse(h.sim_state())
    if (settled.dirty) fail('content', `cycle ${cycle}: the page was dirty after it was hidden and shown`)
    if (settled.playing) fail('content', `cycle ${cycle}: playback was running after the page was hidden and shown`)
    h.sim_teardown()
    const gone = reading()
    series.push({
      cycle,
      measured: cycle > warmup,
      live_bytes: gone.live_bytes,
      capacity: gone.capacity,
      listeners: gone.listeners,
      elements: gone.elements,
      handles: gone.handles
    })
    if (document.getElementById('perf-sim'))
      fail('dom_identity', `cycle ${cycle}: the page is still in the document after its teardown`)
    else pass()
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
  // No fault is injected here, so no slot is lost and none has an explanation.
  const slotsLost = h.held_handles().undefinedSlots - slotsAtStart
  outcome.gates.push({
    name: 'endurance.lost_externref_slots_are_explained',
    status: slotsLost === 0 ? 'pass' : 'fail',
    detail:
      slotsLost === 0
        ? `no externref slot was lost over ${total} cycles, and no fault was injected`
        : `${slotsLost} externref slots were lost, and no fault was injected to explain one`
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
  outcome.samples['resource.after_teardown.cycle'] = series.map(point => point.cycle)
  outcome.samples['resource.after_teardown.measured'] = series.map(point => Number(point.measured))
  outcome.samples['resource.live_bytes.after_teardown.per_cycle'] = series.map(point => point.live_bytes)
  outcome.samples['resource.listeners.after_teardown.per_cycle'] = series.map(point => point.listeners)
  outcome.samples['resource.dom_elements.after_teardown.per_cycle'] = series.map(point => point.elements)
  outcome.samples['resource.capacity.after_teardown.per_cycle'] = series.map(point => point.capacity)
  outcome.samples['resource.js_handles_held.after_teardown.per_cycle'] = series.map(point => point.handles)
  // What every cycle reached is the independent engine's hash at the slice it stopped at, so the digest of the cycles is that hash repeated.
  const executed = await chainOf(cycleEnds)
  const wanted = await chainOf(cycleEnds.map(() => trace.checkpoints[slicesPerCycle]))
  outcome.stream = {
    scope: 'mutations',
    operations: total * slicesPerCycle,
    sha256: executed,
    expected_sha256: wanted,
    matches: executed === wanted
  }
  if (outcome.checkpoints_failed.length > 0) outcome.status = 'invalid_incorrect'
  return outcome
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * What Layer C drives. The product fixture has a real Next button and a real Play and Pause button, and the harness stamps what it measures
 * from outside them: a capture-phase listener on the document writes the mark of a Next click and of a Pause as the input is delivered,
 * before the product's own listener runs, and the harness's scheduler writes the mark of each timer callback's entry (service_perf.rs).
 * Nothing in the product writes a mark.
 */
export const simulationControls = h => {
  const marks = { next: 0, pause: 0 }
  let installed = false
  const install = () => {
    if (installed) return
    installed = true
    document.addEventListener(
      'click',
      event => {
        const target = event.target
        if (target?.id === 'perf-sim-next') performance.mark(`sim-next-${++marks.next}`)
        // A click on the control while it says that playback is running is a Pause, and one while it says otherwise is a Play.
        else if (target?.id === 'perf-sim-play' && target.getAttribute('aria-pressed') === 'true')
          performance.mark(`sim-pause-${++marks.pause}`)
      },
      { capture: true }
    )
  }
  const state = () => ({ ...parse(h.sim_state()), next_marks: marks.next, pause_marks: marks.pause })
  const rect = id => {
    const box = document.getElementById(id)?.getBoundingClientRect()
    return box ? { x: box.left + box.width / 2, y: box.top + box.height / 2 } : null
  }
  /** A fresh product fixture at slice `slice`, reached by untimed calls of next. The marks are not reset: they only ever count up. */
  const reset = (seed, slice, arrivals) => {
    h.sim_teardown()
    const built = parse(h.sim_build(JSON.stringify({ seed, arrivals, product: true })))
    if (!built.ok) throw new Error(built.error)
    const stepped = parse(h.sim_step(slice))
    if (!stepped.ok || stepped.done !== slice)
      throw new Error(`reaching slice ${slice}: ${stepped.last ?? stepped.error}`)
    return state()
  }
  return { install, state, rect, reset, setSpeed: ms => parse(h.sim_set_speed(ms)), teardown: () => h.sim_teardown() }
}
