// The Layer C runner for workload W4: real input, from the pointer to the frame.
//
// A scene is built in a fresh headless Chrome with every node draggable and pan and wheel zoom switched on. The plan of
// `perf-harness gestures` is then played into it through `Input.dispatchMouseEvent`, a case at a time, one for every scale of
// the view and every gesture. What is measured is the browser's: the first capture-phase listener for each input event stamps
// the page's own clock and writes a user-timing mark, and the frame that carries the change is found in a trace by the rule of
// `perf/lib/frames.mjs`. The interaction time is from that stamp to the end of the frame's presentation. How long CDP took to
// deliver the input is a metric of its own and is never used as a start.
//
// A gesture that did not do what it was meant to do is not a measurement. After every gesture the scene is read back and
// compared with what the plan said it would be, and a difference makes the run invalid. A sample the trace cannot place in
// a frame is `unavailable`, counted, and its reason is kept, and it is never given a guessed time.
//
// The wheel cases are each run in a tab of their own that has been brought to the front, since a real wheel reaches only the
// tab in front.
import { spawnSync } from 'node:child_process'
import { DEFINITION, attribute } from '../../lib/frames.mjs'
import { dragPath, pause, wheel } from '../../lib/input.mjs'
import { traced } from '../../lib/trace.mjs'
import { MAX_NODES, checkAccessibility, expectations } from './accessibility.mjs'
import { openLayerCPage } from './lib/c-page.mjs'

/** How long between one pointer move and the next. A frame is 16.7 ms at 60 Hz, so each move gets a frame of its own. */
export const STEP_GAP_MS = 24
/** How long to wait for the last frame of a gesture before the scene is read back. */
const SETTLE_MS = 80

const MECHANISM =
  `${DEFINITION}, found with the style-or-layout qualifier. It runs from the first capture-phase listener for the input event, stamped with performance.now() in the page, to the end of the ` +
  `compositor's PipelineReporter for the frame that the correlation rule attributes to it, in a trace of blink.user_timing, ` +
  `devtools.timeline and cc; input from Input.dispatchMouseEvent every ${STEP_GAP_MS} ms`

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** The edges of a manifest's scene, as `[from, to]` pairs, from the operation stream that `perf-harness stream` writes. */
export const edgesOf = (binary, manifestPath) => {
  const run = spawnSync(binary, ['stream', manifestPath], { encoding: 'utf8', maxBuffer: 1 << 28 })
  if (run.status !== 0) throw new Error(`reading the stream: ${run.stderr}`)
  return run.stdout.split('\n').filter(Boolean).map(line => JSON.parse(line)).filter(op => op.op === 'add_edge').map(op => [op.from, op.to])
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const describeSamples = (binary, name, unit, values, mechanism) => {
  if (values.length === 0) return { name, layer: 'C', status: 'unavailable', reason: 'no observation was made' }
  const run = spawnSync(binary, ['metric', '--name', name, '--layer', 'C', '--unit', unit, '--mechanism', mechanism], {
    input: JSON.stringify(values),
    encoding: 'utf8',
  })
  if (run.status !== 0) throw new Error(`describing ${name}: ${run.stderr}`)
  return JSON.parse(run.stdout)
}

const count = (name, unit, value, mechanism) => ({ name, layer: 'C', status: 'measured', unit, kind: 'count', value, mechanism })
const unavailable = (name, reason) => ({ name, layer: 'C', status: 'unavailable', reason })

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** What the page can say about itself and the browser can say about the page, after a settle step. */
const resources = async page => {
  await page.evaluate('perfC.settle(30)')
  await page.collectGarbage()
  await page.evaluate('perfC.settle(30)')
  const browser = await page.metrics()
  return {
    dom_elements: await page.evaluate('perfC.countElements()'),
    listeners: await page.evaluate('perfC.listeners()'),
    js_heap_used_bytes: browser.JSHeapUsedSize,
  }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// What a gesture has to have done, read back from the scene and compared with the plan's prediction.
const near = (a, b, tolerance) => Math.abs(a - b) <= tolerance

const verifyEffect = async (page, item, scalePercent) => {
  const scale = scalePercent / 100
  const view = JSON.parse(await page.evaluate('perfC.interactive_view()'))
  if (view.error) return view.error
  if (item.gesture === 'drag') {
    const node = JSON.parse(await page.evaluate(`perfC.interactive_node(${JSON.stringify(item.node)})`))
    if (node.error) return node.error
    // The pointer moves in whole pixels, and the scene moves by a pixel over the scale.
    const tolerance = 1 / scale + 0.01
    if (!near(node.x, item.expected[0], tolerance) || !near(node.y, item.expected[1], tolerance)) {
      return `${item.node} ended at (${node.x}, ${node.y}), the plan said (${item.expected[0]}, ${item.expected[1]})`
    }
    if (!near(view.tx, 0, 1e-6) || !near(view.ty, 0, 1e-6) || !near(view.scale, scale, 1e-9)) return `a drag changed the view to ${JSON.stringify(view)}`
    return null
  }
  const want = item.expected
  const tolerance = item.gesture === 'pan' ? 1 : 0.5
  if (!near(view.scale, want.scale, 1e-6) || !near(view.tx, want.tx, tolerance) || !near(view.ty, want.ty, tolerance)) {
    return `the view ended at ${JSON.stringify(view)}, the plan said ${JSON.stringify(want)}`
  }
  return null
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/** Plays one gesture, and gives how long CDP took over each command. */
const play = async (tab, item, origin, svgId) => {
  if (item.gesture === 'wheel_zoom') {
    const trip = await wheel(tab, origin.left + item.at[0], origin.top + item.at[1], item.delta_y)
    return { trips: [trip], moved: 0 }
  }
  // The press is made where the pointer reaches what the gesture is for, which is the planned place unless a connector is over it.
  const kind = item.gesture === 'drag' ? 'drag' : 'pan'
  const press = await tab.evaluate(`perfC.pressPoint(${JSON.stringify(svgId)}, ${JSON.stringify(kind)}, ${JSON.stringify(item.node ?? null)}, ${JSON.stringify(item.from)})`)
  if (!press) return { trips: [], moved: 0, unplaceable: true }
  // A drag is made from the node's own box, so a node that is dragged a distance moves by the distance whatever the press.
  const trips = await dragPath(tab, [origin.left + press.x, origin.top + press.y], item.dx, item.dy, item.steps, STEP_GAP_MS)
  return { trips: trips.moves, moved: press.moved > 0.5 ? 1 : 0 }
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Runs one case in `tab`, and gives what it measured.
 *
 * With `timed` off the gestures are played and checked and no trace is made, which is the correctness mode.
 */
const runCase = async (tab, kase, canonical, timed, faults) => {
  const result = { failures: [], latencies: [], inputDelays: [], trips: [], unattributed: {}, eventTiming: 0, checks: 0, build: null, relocated: 0 }
  const build = JSON.parse(await tab.evaluate(`perfC.interactive_build(${JSON.stringify(canonical)}, ${kase.scale_percent})`))
  if (build.error) throw new Error(`building the scene: ${build.error}`)
  result.build = build
  // The whole host has to be on screen, or a gesture planned for a part of it that is not has nothing to land on. A wheel in a
  // tab that has not been sized lands outside the page and does nothing.
  await tab.send('Emulation.setDeviceMetricsOverride', { width: build.host[0] + 40, height: build.host[1] + 40, deviceScaleFactor: 1, mobile: false })
  if (!build.checkpoint?.passed) {
    result.failures.push({ section: 'build', detail: `the scene to be driven through is not the one meant: ${(build.checkpoint?.differences ?? []).join('; ')}` })
  } else {
    result.checks++
  }
  // The faults of the self-test. Each is put in after the scene is built, and none is there in a run that is not a self-test.
  if (faults.slowdownMs) await tab.evaluate(`perfC.slowdown(${faults.slowdownMs})`)
  if (faults.blockInput) await tab.evaluate('perfC.block()')
  if (faults.cdpExtraMs) {
    const send = tab.send
    tab.send = async (method, params) => {
      const reply = await send(method, params)
      if (method.startsWith('Input.')) await pause(faults.cdpExtraMs)
      return reply
    }
  }
  await tab.evaluate('perfC.clear()')
  await tab.evaluate('perfC.settle(100)')

  let gestureId = 0
  const gestures = [...kase.warmup.map(item => ({ item, measured: false })), ...kase.measured.map(item => ({ item, measured: true }))]
  const work = async () => {
    for (const { item, measured } of gestures) {
      // Every gesture starts from the same scene, so that one does not depend on the one before.
      const positions = item.gesture === 'drag' ? { [item.node]: item.start } : {}
      const reset = JSON.parse(await tab.evaluate(`perfC.interactive_reset(${kase.scale_percent}, ${JSON.stringify(JSON.stringify(positions))})`))
      if (reset.error) throw new Error(`resetting the scene: ${reset.error}`)
      await tab.evaluate('perfC.settle(40)')
      const origin = await tab.evaluate(`perfC.hostRect(${JSON.stringify(build.svg_id)})`)
      await tab.evaluate(`perfC.begin(${gestureId++}, ${measured})`)
      const { trips, moved, unplaceable } = await play(tab, item, origin, build.svg_id)
      result.relocated += moved ?? 0
      if (unplaceable) result.failures.push({ section: 'interaction', detail: `${kase.gesture} ${gestureId - 1} at ${kase.scale_percent}%: no place was found at which the pointer reaches what the gesture is for` })
      await pause(SETTLE_MS)
      await tab.evaluate('perfC.end()')
      await tab.evaluate('perfC.settle(20)')
      if (measured) result.trips.push(...trips)
      const wrong = await verifyEffect(tab, item, kase.scale_percent)
      result.checks++
      if (wrong) result.failures.push({ section: 'interaction', detail: `${kase.gesture} ${gestureId - 1} at ${kase.scale_percent}%: ${wrong}` })
    }
  }

  let trace = null
  if (timed) trace = (await traced(tab, work)).trace
  else await work()

  const events = await tab.evaluate('perfC.events()')
  // The input events that change the scene are the moves with the button down, and the wheel. The press and the release start
  // and end a gesture, and the click that follows a release is the browser's.
  const counted = event => event.measured && (kase.gesture === 'wheel_zoom' ? event.type === 'wheel' : event.type === 'pointermove' && event.buttons === 1)
  for (const event of events.filter(counted)) {
    result.inputDelays.push(event.stamp - event.hardware)
    if (!trace) continue
    const frame = attribute(trace, `perf-c-${event.n}`, { boundMs: 250 })
    if (frame.status === 'measured') result.latencies.push(frame.latencyMs)
    else result.unattributed[frame.reason.replace(/\d+/g, 'N')] = (result.unattributed[frame.reason.replace(/\d+/g, 'N')] ?? 0) + 1
  }
  result.eventTiming = (await tab.evaluate('perfC.eventTiming()')).length
  return result
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Runs the interaction workload once in a fresh Chrome, in `mode`, and gives an outcome in the shape of the other layers'.
 *
 * `plan` is the output of `perf-harness gestures`, and `report` the workload report of the manifest, whose digests the stream
 * that the page executed is held to. `faults` is for the self-test alone: `slowdownMs`, `blockInput`, `cdpExtraMs` and `accessibility`.
 * `edges` are the scene's edges, which the accessibility check holds the tree to.
 */
export const runInteraction = async ({ binary, mode, canonical, plan, report, edges, faults = {} }) => {
  const timed = mode === 'performance'
  const outcome = {
    status: 'ok',
    metrics: [],
    samples: {},
    checkpoints_passed: 0,
    checkpoints_failed: [],
    gates: [],
    notes: [],
    stream: null,
    timer_resolution_us: 0,
  }
  const page = await openLayerCPage()
  try {
    outcome.crossOriginIsolated = page.crossOriginIsolated
    outcome.timer_resolution_us = await page.evaluate(`(() => { let s = Infinity, l = performance.now(); for (let i = 0; i < 200000; i++) { const n = performance.now(); const d = (n - l) * 1000; if (d > 0 && d < s) s = d; l = n } return Number.isFinite(s) ? s : 0 })()`)
    // One observation, which the metric says by its sample count. A cold start cannot be repeated inside a run.
    outcome.metrics.push(describeSamples(binary, 'cold_start.page_ready', 'ms', [page.coldStartMs], 'performance.now() in the page when the wasm module was ready, which is from the start of the navigation'))
    outcome.notes.push(`plan ${plan.sha256}; input every ${STEP_GAP_MS} ms through Input.dispatchMouseEvent; a wheel case is run in a tab of its own, in front`)

    // One build and one teardown, with the counts taken at each end after a settle step.
    const base = await resources(page)
    const first = plan.plan.cases[0]
    const built = JSON.parse(await page.evaluate(`perfC.interactive_build(${JSON.stringify(canonical)}, ${first.scale_percent})`))
    if (built.error) throw new Error(built.error)
    const whileBuilt = await resources(page)
    await page.evaluate('perfC.interactive_teardown()')
    const hostGone = !(await page.evaluate('perfC.interactive_host_present()'))
    const after = await resources(page)
    for (const [moment, figures] of [['baseline', base], ['while_built', whileBuilt], ['after_teardown', after]]) {
      for (const [name, unit, mechanism] of [
        ['dom_elements', 'count', 'document.querySelectorAll(*).length'],
        ['listeners', 'count', 'the effective-registration tracker'],
        ['js_heap_used_bytes', 'bytes', 'Performance.getMetrics JSHeapUsedSize after a forced collection, and a diagnostic of Chrome only'],
      ]) {
        outcome.metrics.push(figures[name] === null || figures[name] === undefined ? unavailable(`resource.${name}.${moment}`, `${mechanism} gave no reading`) : count(`resource.${name}.${moment}`, unit, figures[name], mechanism))
      }
    }
    const gate = (name, status, detail) => outcome.gates.push({ name, status, detail })
    gate('interaction.listeners_return_to_baseline', after.listeners === base.listeners ? 'pass' : 'fail', `${base.listeners} before the build, ${whileBuilt.listeners} while it stood, ${after.listeners} after the teardown`)
    // A gate that passes because nothing was ever counted proves nothing, so the listeners of the draggable nodes have to be seen.
    gate('interaction.listeners_are_seen_while_built', whileBuilt.listeners > base.listeners ? 'pass' : 'inconclusive', `${whileBuilt.listeners - base.listeners} listeners were added by a scene whose nodes are draggable`)
    gate('interaction.dom_returns_to_baseline', after.dom_elements === base.dom_elements && hostGone ? 'pass' : 'fail', `${base.dom_elements} elements before, ${whileBuilt.dom_elements} while built, ${after.dom_elements} after, host present ${!hostGone}`)

    // The cases.
    for (const kase of plan.plan.cases) {
      const key = `${kase.gesture}.scale_${kase.scale_percent}`
      const isolated = kase.gesture === 'wheel_zoom'
      const tab = isolated ? await page.openTab() : page
      let result
      try {
        result = await runCase(tab, kase, canonical, timed, faults)
        await tab.evaluate('perfC.interactive_teardown()')
      } finally {
        if (isolated) await tab.closeTab()
      }
      outcome.checkpoints_passed += result.checks - result.failures.length
      outcome.checkpoints_failed.push(...result.failures)
      if (!outcome.stream) {
        const scope = timed ? 'mutations' : 'full'
        const sha256 = timed ? result.build.mutations_sha256 : result.build.sha256
        const expected = timed ? report.stream.mutations_sha256 : report.stream.sha256
        outcome.stream = { scope, operations: result.build.operations, sha256, expected_sha256: expected, matches: sha256 === expected }
      }
      if (!timed) continue
      outcome.metrics.push(describeSamples(binary, `interaction.${key}.latency`, 'ms', result.latencies, MECHANISM))
      outcome.metrics.push(describeSamples(binary, `interaction.${key}.input_delay`, 'ms', result.inputDelays, 'event.timeStamp, which is when the browser made the event, to the first capture-phase listener, both in the page clock'))
      outcome.metrics.push(describeSamples(binary, `cdp.${key}.round_trip`, 'ms', result.trips, 'how long Input.dispatchMouseEvent took to answer, which is never used as a start time'))
      const lost = Object.values(result.unattributed).reduce((a, b) => a + b, 0)
      outcome.metrics.push(count(`interaction.${key}.frames_unattributed`, 'count', lost, lost ? `the trace could not place these in a frame: ${JSON.stringify(result.unattributed)}` : 'every measured input event was placed in a frame'))
      outcome.metrics.push(count(`interaction.${key}.presses_relocated`, 'count', result.relocated, 'gestures whose press was moved from the planned point to the nearest one that reaches the node or the pan surface, since connectors are drawn over nodes'))
      outcome.metrics.push(count(`interaction.${key}.event_timing_entries`, 'count', result.eventTiming, 'Event Timing entries of 16 ms or more, rounded to 8 ms by the browser, kept beside the figures above'))
      outcome.samples[`interaction.${key}.latency`] = result.latencies
      outcome.samples[`interaction.${key}.input_delay`] = result.inputDelays
      outcome.samples[`cdp.${key}.round_trip`] = result.trips
    }
    if (!timed) outcome.metrics.push(unavailable('interaction.latency', 'the correctness mode keeps no timing'))

    // The accessibility tree, last, and in tabs of their own, since keeping it up to date costs time on every change.
    if (edges) await accessibilityPass({ page, plan, canonical, edges, outcome, fault: faults.accessibility ?? null })
  } catch (error) {
    outcome.status = 'error'
    outcome.error = String(error?.message ?? error)
  } finally {
    await page.close()
  }
  if (outcome.status === 'ok' && (outcome.checkpoints_failed.length > 0 || outcome.stream?.matches === false)) outcome.status = 'invalid_incorrect'
  return outcome
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Checks Chrome's accessibility tree at each scale of the plan, in a tab of its own with the scene built, before and after one
 * drag, one pan and one wheel zoom, and puts what it found in `outcome`. See `accessibility.mjs`.
 */
const accessibilityPass = async ({ page, plan, canonical, edges, outcome, fault }) => {
  const expected = expectations(edges)
  const scales = [...new Set(plan.plan.cases.map(kase => kase.scale_percent))]
  let largest = 0
  let inspected = 0
  let checked = 0
  for (const scalePercent of scales) {
    const tab = await page.openTab()
    try {
      const build = JSON.parse(await tab.evaluate(`perfC.interactive_build(${JSON.stringify(canonical)}, ${scalePercent})`))
      if (build.error) throw new Error(`building the scene for the accessibility check: ${build.error}`)
      await tab.send('Emulation.setDeviceMetricsOverride', { width: build.host[0] + 40, height: build.host[1] + 40, deviceScaleFactor: 1, mobile: false })
      await tab.evaluate('perfC.settle(100)')
      // The first measured gesture of each kind at this scale, with the wheel last so that the graph view's name has to change.
      const gestures = ['drag', 'pan', 'wheel_zoom']
        .map(kind => plan.plan.cases.find(kase => kase.scale_percent === scalePercent && kase.gesture === kind)?.measured[0])
        .filter(Boolean)
      const play_ = async item => {
        const positions = item.gesture === 'drag' ? { [item.node]: item.start } : {}
        await tab.evaluate(`perfC.interactive_reset(${scalePercent}, ${JSON.stringify(JSON.stringify(positions))})`)
        await tab.evaluate('perfC.settle(40)')
        const origin = await tab.evaluate(`perfC.hostRect(${JSON.stringify(build.svg_id)})`)
        await play(tab, item, origin, build.svg_id)
        await pause(SETTLE_MS)
      }
      const { checks, stats } = await checkAccessibility({ tab, expected, scalePercent, gestures, play: play_, label: `accessibility at ${scalePercent}%`, fault })
      for (const check of checks) {
        if (check.ok) outcome.checkpoints_passed++
        else outcome.checkpoints_failed.push({ section: 'accessibility', detail: check.detail })
      }
      checked += checks.length
      largest = Math.max(largest, stats.ax_nodes)
      inspected = Math.max(inspected, stats.nodes_inspected)
    } finally {
      await tab.closeTab()
    }
  }
  const how = 'Accessibility.getFullAXTree in a tab of its own, after the timed cases'
  outcome.metrics.push(count('accessibility.checks', 'count', checked, `${how}: roles, names and relationships against the model, focus, and consistency after input`))
  outcome.metrics.push(count('accessibility.ax_nodes_largest_tree', 'count', largest, how))
  outcome.metrics.push(count('accessibility.nodes_inspected', 'count', inspected, `${how}, at most ${MAX_NODES} nodes from those that have an edge`))
}
