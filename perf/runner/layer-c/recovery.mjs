// The recovery scenarios that need a real browser: a view change that cannot be written in a frame, a scene dropped with a frame
// pending, and a scene dropped between a pointer down and a pointer up.
//
// Each is a fault point of the recovery workload (design section 7.1), and each is held to what the library says it leaves behind
// (`perf/fault-contracts.json`):
//
//   scheduled_frame    partly committed. A wheel event updates the view at once and leaves the DOM write to the next animation
//                      frame, so if the browser refuses the frame, or the write in it fails, the scene's view has changed and
//                      the document has not. The next event asks for a frame again, and the document catches up.
//   teardown           nothing committed. Dropping the last handle cancels the pending frame before its closure goes, writes the
//                      view that was waiting, and releases the scene's listeners. Removing the host then leaves the page as
//                      it was.
//   mid_drag_teardown  nothing committed. The pointer handlers hold the scene weakly, so after it is dropped they find nothing and
//                      do nothing, the grabbing cursor is taken off the node, and removing the host leaves the page as it was.
//
// Frames are refused, held back, and made to fail by `perf/page/fault-injector.js`, and the input is real, through
// `Input.dispatchMouseEvent`. Every scenario is run `REPETITIONS` times, in a scene of its own, and must hold every time.
import { spawnSync } from 'node:child_process'
import { dragPath, pause, wheel } from '../../lib/input.mjs'
import { openLayerCPage } from './lib/c-page.mjs'

const REPETITIONS = 10
const STEP_GAP_MS = 24
const MECHANISMS = {
  scheduled_frame:
    "performance.now() in the page, from the first capture-phase listener for the event that retries (after a frame was refused, or its write failed) to the moment the content layer's transform became the scene's own view, which a MutationObserver reports",
  teardown:
    'performance.now() in the page around the drop of the last Scene handle with a frame pending, which cancels the frame, writes the view that was waiting and releases the listeners',
  mid_drag_teardown:
    'performance.now() in the page around the drop of the last Scene handle between a pointer down and a pointer up'
}

const near = (a, b) => Math.abs(a - b) < 1e-6
const same = (a, b) => a && b && near(a.scale, b.scale) && near(a.tx, b.tx) && near(a.ty, b.ty)

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Runs the scenarios of `points`, each as many times as `repetitions` says, and gives an outcome in the shape of the other
 * layers', with the findings and the table of what each run did.
 */
export const runRecoveryC = async ({
  binary,
  canonical,
  manifest,
  report,
  repetitions = REPETITIONS,
  endurance = null,
  faults = {}
}) => {
  let reps = repetitions
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
    timer_resolution_us: 0
  }
  const page = await openLayerCPage({ faults: true })
  const problems = []
  try {
    await page.send('Runtime.enable')
    page.on('Runtime.exceptionThrown', params =>
      problems.push(params.exceptionDetails.exception?.description ?? params.exceptionDetails.text)
    )
    page.on('Runtime.consoleAPICalled', params => {
      if (params.type === 'error') problems.push(params.args.map(arg => arg.value ?? arg.description).join(' '))
    })
    outcome.crossOriginIsolated = page.crossOriginIsolated
    // For the self-test only: a cancel that does not cancel.
    if (faults.suppressCancel) await page.evaluate('__frames.suppressCancel(true)')
    outcome.timer_resolution_us = await page.evaluate(
      `(() => { let s = Infinity, l = performance.now(); for (let i = 0; i < 200000; i++) { const n = performance.now(); const d = (n - l) * 1000; if (d > 0 && d < s) s = d; l = n } return Number.isFinite(s) ? s : 0 })()`
    )

    const view = async () => JSON.parse(await page.evaluate('perfC.interactive_view()'))
    const domView = id => page.evaluate(`perfC.domView(${JSON.stringify(id)})`)
    const counts = () => page.evaluate('({ elements: perfC.countElements(), listeners: perfC.listeners() })')
    const baseline = await counts()
    const build = async () => {
      const built = JSON.parse(await page.evaluate(`perfC.interactive_build(${JSON.stringify(canonical)}, 100)`))
      if (built.error) throw new Error(`building the scene: ${built.error}`)
      await page.send('Emulation.setDeviceMetricsOverride', {
        width: built.host[0] + 40,
        height: built.host[1] + 40,
        deviceScaleFactor: 1,
        mobile: false
      })
      await page.evaluate('perfC.settle(80)')
      const origin = await page.evaluate(`perfC.hostRect(${JSON.stringify(built.svg_id)})`)
      if (outcome.stream === null)
        outcome.stream = {
          scope: 'full',
          operations: built.operations,
          sha256: built.sha256,
          expected_sha256: report.stream.sha256,
          matches: built.sha256 === report.stream.sha256
        }
      return { ...built, origin }
    }
    const wheelAt = (scene, deltaY = -100) => wheel(page, scene.origin.left + 300, scene.origin.top + 200, deltaY)

    // - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
    const scenarios = {
      // A view change that cannot be written: first the browser refuses the frame, then the write in the frame fails.
      scheduled_frame: async () => {
        const times = []
        const rows = []
        for (let run = 0; run < reps; run++) {
          const scene = await build()
          const id = scene.svg_id
          // One event with nothing wrong, to learn which of the DOM calls it makes is the write of the view.
          await page.evaluate('__faults.record(true)')
          await wheelAt(scene)
          await pause(120)
          await page.evaluate('__faults.record(false)')
          const calls = await page.evaluate('__faults.take()')
          const write = calls.findIndex(call => call.method === 'setAttribute' && call.name === 'transform') + 1
          await page.evaluate(`perfC.interactive_reset(100, '{}')`)
          await page.evaluate('perfC.settle(80)')

          for (const way of ['refused', 'write_fails']) {
            if (way === 'refused') await page.evaluate('__frames.refuseNext()')
            else await page.evaluate(`__faults.arm(${write})`)
            await wheelAt(scene)
            await pause(150)
            const fired = way === 'refused' ? null : await page.evaluate('__faults.disarm()')
            const model = await view()
            const dom = await domView(id)
            const stale = !same(model, dom)
            const moved = model.scale > 1.2
            // The next event asks for a frame again, and the document catches up to the latest view.
            await page.evaluate('perfC.clear()')
            await wheelAt(scene)
            const stamp = (await page.evaluate('perfC.events()')).find(event => event.type === 'wheel')?.stamp
            const settled = await page.evaluate(`perfC.untilConsistent(${JSON.stringify(id)}, 1000)`)
            const after = await view()
            const row = {
              point: 'scheduled_frame',
              run,
              way,
              injected: way === 'refused' ? true : fired?.fired !== null && fired?.fired !== undefined,
              model_changed_at_once: moved,
              document_stale: stale,
              document_caught_up_after_the_next_event: settled !== null && same(after, await domView(id)),
              view_after_two_events: after.scale
            }
            rows.push(row)
            if (settled !== null && stamp !== undefined) times.push(settled - stamp)
            await page.evaluate(`perfC.interactive_reset(100, '{}')`)
            await page.evaluate('perfC.settle(60)')
          }
          await page.evaluate('perfC.interactive_teardown()')
        }
        const broken = rows.filter(
          r =>
            !(
              r.injected &&
              r.model_changed_at_once &&
              r.document_stale &&
              r.document_caught_up_after_the_next_event &&
              near(r.view_after_two_events, 1.5625)
            )
        )
        return {
          rows,
          times,
          broken,
          describe: r =>
            `${r.way}: injected ${r.injected}, the model changed at once ${r.model_changed_at_once}, the document was stale ${r.document_stale}, it caught up after the next event ${r.document_caught_up_after_the_next_event}`
        }
      },

      // The scene is dropped while a frame is pending.
      teardown: async () => {
        const times = []
        const rows = []
        for (let run = 0; run < reps; run++) {
          const scene = await build()
          const id = scene.svg_id
          await page.evaluate('__frames.clear(); __frames.hold(true)')
          await wheelAt(scene)
          await pause(80)
          const pending = await page.evaluate('__frames.pending()')
          const model = await view()
          const staleBefore = !same(model, await domView(id))
          const cancelledBefore = await page.evaluate('__frames.cancelled()')
          const took = await page.evaluate('perfC.timedDrop()')
          const cancelled = (await page.evaluate('__frames.cancelled()')) - cancelledBefore
          const stillPending = await page.evaluate('__frames.pending()')
          const flushed = same(model, await domView(id))
          const afterDrop = await counts()
          // A frame that was cancelled has nothing left to run, and one that was not would throw here.
          const released = await page.evaluate('__frames.release()')
          await page.evaluate('perfC.settle(60)')
          await page.evaluate('perfC.interactive_teardown()')
          await page.evaluate('perfC.settle(40)')
          const final = await counts()
          times.push(took)
          rows.push({
            point: 'teardown',
            run,
            a_frame_was_pending: pending === 1,
            document_was_behind_the_view: staleBefore,
            frames_cancelled_by_the_drop: cancelled,
            frames_pending_after_the_drop: stillPending,
            view_written_when_dropped: flushed,
            frames_that_ran_late: released.ran,
            late_frames_that_threw: released.threw,
            listeners_after_the_drop: afterDrop.listeners - baseline.listeners,
            page_back_to_baseline: final.elements === baseline.elements && final.listeners === baseline.listeners
          })
        }
        const broken = rows.filter(
          r =>
            !(
              r.a_frame_was_pending &&
              r.frames_cancelled_by_the_drop === 1 &&
              r.frames_pending_after_the_drop === 0 &&
              r.view_written_when_dropped &&
              r.frames_that_ran_late === 0 &&
              r.late_frames_that_threw.length === 0 &&
              r.listeners_after_the_drop === 0 &&
              r.page_back_to_baseline
            )
        )
        return {
          rows,
          times,
          broken,
          describe: r =>
            `pending ${r.a_frame_was_pending}, cancelled ${r.frames_cancelled_by_the_drop}, view written ${r.view_written_when_dropped}, ran late ${r.frames_that_ran_late}, threw ${r.late_frames_that_threw.length}, listeners left ${r.listeners_after_the_drop}, back to baseline ${r.page_back_to_baseline}`
        }
      },

      // The scene is dropped between a pointer down and a pointer up.
      mid_drag_teardown: async () => {
        const times = []
        const rows = []
        for (let run = 0; run < reps; run++) {
          const scene = await build()
          const id = scene.svg_id
          const key = scene.nodes[run % scene.nodes.length]
          const hint = [0, 0]
          const press = await page.evaluate(
            `perfC.pressPoint(${JSON.stringify(id)}, 'drag', ${JSON.stringify(key)}, ${JSON.stringify(hint)})`
          )
          if (!press) throw new Error(`no place on ${key} reaches it`)
          const errorsBefore = problems.length
          // Down, and some of the way across, with real input.
          const from = [scene.origin.left + press.x, scene.origin.top + press.y]
          await dragPartly(page, from, 30, 18, 3, STEP_GAP_MS)
          const moving = await page.evaluate(`perfC.nodeStyle(${JSON.stringify(id)}, ${JSON.stringify(key)})`)
          const took = await page.evaluate('perfC.timedDrop()')
          const atDrop = await page.evaluate(`perfC.nodeStyle(${JSON.stringify(id)}, ${JSON.stringify(key)})`)
          const afterDrop = await counts()
          // The pointer carries on across the node that is still in the document, and comes up.
          await finishDrag(page, from, 30, 18, 3, STEP_GAP_MS)
          await pause(80)
          const later = await page.evaluate(`perfC.nodeStyle(${JSON.stringify(id)}, ${JSON.stringify(key)})`)
          await page.evaluate('perfC.interactive_teardown()')
          await page.evaluate('perfC.settle(40)')
          const final = await counts()
          times.push(took)
          rows.push({
            point: 'mid_drag_teardown',
            run,
            node: key,
            the_drag_was_under_way: moving.transform !== '' && /grabbing/.test(moving.style),
            node_moved_by_the_drag_after_the_drop: later.transform !== atDrop.transform,
            listeners_after_the_drop: afterDrop.listeners - baseline.listeners,
            page_errors: problems.slice(errorsBefore),
            page_back_to_baseline: final.elements === baseline.elements && final.listeners === baseline.listeners,
            // The node is not being dragged any more, so it must not show the cursor of a drag, either straight after the drop or
            // once the pointer has come up.
            grabbing_cursor_left: /grabbing/.test(atDrop.style) || /grabbing/.test(later.style),
            style_while_dragging: moving.style,
            style_after_the_drop_and_the_pointer_up: later.style
          })
        }
        const broken = rows.filter(
          r =>
            !(
              r.the_drag_was_under_way &&
              !r.node_moved_by_the_drag_after_the_drop &&
              r.listeners_after_the_drop === 0 &&
              !r.grabbing_cursor_left &&
              r.page_errors.length === 0 &&
              r.page_back_to_baseline
            )
        )
        return {
          rows,
          times,
          broken,
          describe: r =>
            `drag under way ${r.the_drag_was_under_way}, moved after the drop ${r.node_moved_by_the_drag_after_the_drop}, listeners left ${r.listeners_after_the_drop}, grabbing cursor left ${r.grabbing_cursor_left}, errors ${r.page_errors.length}, back to baseline ${r.page_back_to_baseline}`
        }
      }
    }

    // Endurance: the same scenarios, one run of each in every cycle, and what the browser holds after each cycle is read once
    // the page has settled and the garbage has been collected.
    if (endurance) {
      reps = 1
      const points = wanted.filter(p => p.point in scenarios)
      const reading = async () => {
        await page.evaluate('perfC.settle(60)')
        await page.collectGarbage()
        await page.evaluate('perfC.settle(30)')
        const browser = await page.metrics()
        return {
          ...(await counts()),
          heap: browser.JSHeapUsedSize,
          chromeListeners: browser.JSEventListeners,
          nodes: browser.Nodes
        }
      }
      const start = await reading()
      const series = []
      const unmet = []
      const total = endurance.warmup + endurance.measured
      for (let cycle = 1; cycle <= total; cycle++) {
        for (const { point } of points) {
          const ran = await scenarios[point]()
          if (ran.broken.length > 0) unmet.push(`cycle ${cycle}, ${point}: ${ran.describe(ran.broken[0])}`)
        }
        series.push({ cycle, measured: cycle > endurance.warmup, ...(await reading()) })
      }
      const mechanism =
        'read after each cycle once the page had settled and HeapProfiler.collectGarbage had run, so the heap is a Chrome diagnostic and the counts are the gate'
      const back = (name, field, what) => {
        const off = series.filter(point => point[field] !== start[field])
        outcome.gates.push({
          name,
          status: off.length === 0 ? 'pass' : 'fail',
          detail:
            off.length === 0
              ? `${what} was ${start[field]} before the first cycle and after each of the ${total} cycles`
              : `${off.length} of ${total} cycles ended with ${what} at a count other than ${start[field]}, the first being cycle ${off[0].cycle} with ${off[0][field]}`
        })
      }
      back('endurance.listeners_return_to_baseline', 'listeners', 'the effective listener count')
      back('endurance.dom_returns_to_baseline', 'elements', "the document's element count")
      outcome.gates.push({
        name: 'endurance.recoveries_succeed',
        status: unmet.length === 0 ? 'pass' : 'fail',
        detail:
          unmet.length === 0
            ? `${total * points.length} scenario runs over ${total} cycles each held to the contract`
            : `${unmet.length} did not: ${unmet[0]}`
      })
      const measured = series.filter(point => point.measured)
      const slope = values => {
        const n = values.length
        const mx = (n - 1) / 2
        const my = values.reduce((a, b) => a + b, 0) / n
        return n < 3
          ? null
          : values.reduce((s, v, i) => s + (i - mx) * (v - my), 0) / values.reduce((s, _, i) => s + (i - mx) ** 2, 0)
      }
      const heapSlope = slope(measured.map(point => point.heap))
      outcome.metrics.push({
        name: 'lifecycle.measured_cycles_sampled',
        layer: 'C',
        status: 'measured',
        unit: 'count',
        kind: 'count',
        value: measured.length,
        mechanism
      })
      outcome.metrics.push(
        heapSlope === null
          ? {
              name: 'browser.js_heap_slope_per_cycle',
              layer: 'C',
              status: 'unavailable',
              reason: 'fewer than 3 measured cycles'
            }
          : {
              name: 'browser.js_heap_slope_per_cycle',
              layer: 'C',
              status: 'measured',
              unit: 'bytes',
              kind: 'trend',
              value: heapSlope,
              mechanism: `the least-squares slope of JSHeapUsedSize against the cycle over the measured cycles, ${mechanism}. A diagnostic of Chrome only, and not a gate`
            }
      )
      outcome.metrics.push({
        name: 'browser.js_heap_net_change_after_warmup',
        layer: 'C',
        status: 'measured',
        unit: 'bytes',
        kind: 'trend',
        value: measured.at(-1).heap - measured[0].heap,
        mechanism: `JSHeapUsedSize of the last measured cycle less the first, ${mechanism}`
      })
      outcome.samples['resource.after_cycle.cycle'] = series.map(point => point.cycle)
      outcome.samples['resource.after_cycle.measured'] = series.map(point => Number(point.measured))
      outcome.samples['resource.js_heap.after_cycle.per_cycle'] = series.map(point => point.heap)
      outcome.samples['resource.listeners.after_cycle.per_cycle'] = series.map(point => point.listeners)
      outcome.samples['resource.dom_elements.after_cycle.per_cycle'] = series.map(point => point.elements)
      outcome.samples['resource.chrome_listeners.after_cycle.per_cycle'] = series.map(point => point.chromeListeners)
      outcome.notes.push(
        `${total} cycles (${endurance.warmup} warm-up), each running ${points.map(p => p.point).join(', ')} once`
      )
      outcome.checkpoints_passed = total * points.length - unmet.length
      for (const line of unmet) outcome.checkpoints_failed.push({ section: 'dom_identity', detail: line })
    } else {
      for (const { point, expect } of wanted.filter(p => p.point in scenarios)) {
        const ran = await scenarios[point]()
        outcome.cases.push(...ran.rows)
        if (ran.times.length) {
          outcome.samples[`recovery.${point}.time`] = ran.times
          const described = spawnSync(
            binary,
            [
              'metric',
              '--name',
              `recovery.${point}.time`,
              '--layer',
              'C',
              '--unit',
              'ms',
              '--mechanism',
              MECHANISMS[point]
            ],
            { input: JSON.stringify(ran.times), encoding: 'utf8' }
          )
          outcome.metrics.push(JSON.parse(described.stdout))
        }
        const ok = ran.broken.length === 0
        outcome.gates.push({
          name: `recovery.${point}.matches_the_contract`,
          status: ok ? 'pass' : 'fail',
          detail: ok
            ? `${ran.rows.length} runs, each ${expect.replace('_', ' ')} as documented`
            : `${ran.broken.length} of ${ran.rows.length} runs broke it: ${ran.describe(ran.broken[0])}`
        })
        outcome.checkpoints_passed += ran.rows.length - ran.broken.length
        for (const bad of ran.broken)
          outcome.checkpoints_failed.push({
            section: point === 'scheduled_frame' ? 'interaction' : 'dom_identity',
            detail: `${point} run ${bad.run}: ${ran.describe(bad)}`
          })
      }

      if (problems.length > 0)
        outcome.notes.push(
          `the page reported ${problems.length} errors during the scenarios, the first being: ${problems[0].slice(0, 160)}`
        )
    }
  } catch (error) {
    outcome.status = 'error'
    outcome.error = String(error?.message ?? error)
  } finally {
    await page.close()
  }
  if (outcome.status === 'ok' && outcome.checkpoints_failed.length > 0) outcome.status = 'invalid_incorrect'
  return outcome
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// A drag in two halves, so that something can be done to the scene between them. `dragPath` plays a whole one.
const send = (page, params) => page.send('Input.dispatchMouseEvent', params)

const dragPartly = async (page, from, dx, dy, steps, gapMs) => {
  await send(page, { type: 'mouseMoved', x: from[0], y: from[1], button: 'none', buttons: 0 })
  await send(page, { type: 'mousePressed', x: from[0], y: from[1], button: 'left', buttons: 1, clickCount: 1 })
  await pause(gapMs)
  for (let step = 1; step <= steps; step++) {
    await send(page, {
      type: 'mouseMoved',
      x: Math.round(from[0] + (dx * step) / (steps * 2)),
      y: Math.round(from[1] + (dy * step) / (steps * 2)),
      button: 'left',
      buttons: 1
    })
    await pause(gapMs)
  }
}

const finishDrag = async (page, from, dx, dy, steps, gapMs) => {
  for (let step = 1; step <= steps; step++) {
    await send(page, {
      type: 'mouseMoved',
      x: Math.round(from[0] + dx / 2 + (dx * step) / (steps * 2)),
      y: Math.round(from[1] + dy / 2 + (dy * step) / (steps * 2)),
      button: 'left',
      buttons: 1
    })
    await pause(gapMs)
  }
  await send(page, {
    type: 'mouseReleased',
    x: Math.round(from[0] + dx),
    y: Math.round(from[1] + dy),
    button: 'left',
    buttons: 0,
    clickCount: 1
  })
}

export { dragPath }
