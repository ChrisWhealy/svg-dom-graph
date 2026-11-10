// The Layer C runner for workload W6: the simulator's real Next and Play controls, in a real browser, from input to presentation.
//
// The product's controller is built on its real surface with its real controls, in a fresh headless Chrome for each repetition, and is driven
// with `Input.dispatchMouseEvent`. Two series are measured, and they are two metrics that are never pooled. Each starts again from the cursor
// after slice 256, and takes 10 untimed commits and then 30 measured ones, which end at slice 296.
//
//   Next  A real click on the accessible Next button. The mark is written by a harness listener in the capture phase of the document, as the
//         click is delivered and before the product's own listener runs. What is measured is input delivery to the presentation of the frame
//         that carries the commit. A click commits synchronously in its handler, so its frame holds only a repaint, and the frame is found
//         with the wider `update` qualifier, which the result records and labels as the upper bound of a window and not a cause.
//   Play  One tick at the slowest speed, 1 slice a second. Each observation is a real click on Play, exactly one timer-driven slice and its
//         presentation, and then a real click on Pause, outside the timed interval. The mark is written by the harness's own scheduler as the
//         product's timer callback is entered, so what is measured is the scheduled commit to its presentation. The 1,000 ms interval and the
//         wait from the click to the callback are not in it, and are recorded on their own.
//
// A second tick before Pause is `invalid_protocol_extra_tick`, and rejects the whole repetition: it is not retried, it is not counted twice and
// no tick is dropped. After every commit the controller's own engine is held to the independent engine's hash, since a checkpoint that
// does not match makes the timings worthless. Failed, hidden and stale ticks never enter a latency series.
//
// Play costs at least 40 seconds of timer delay a repetition. It is run only when asked for (`series` includes `play`) and when the manifest
// allows the wall time, and a series that is not run is reported as not run, never as a pass.
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { DEFINITION, attribute } from '../../lib/frames.mjs'
import { pause, press, release } from '../../lib/input.mjs'
import { traced } from '../../lib/trace.mjs'
import { openHarnessPage } from '../lib/harness-page.mjs'

const PRIMING = 256
const WARMUP = 10
const MEASURED = 30
const INTERVAL_MS = 1000
const PRESENT_WAIT_MS = 150
const QUIET_MS = 1300
/** Play needs at least this much wall time a repetition: 40 cycles at 1 Hz is 40 seconds, and the rest is the browser, the trace and the teardown. */
export const PLAY_MINIMUM_WALL_MS = 90_000

const describeSamples = (binary, name, unit, values, mechanism) => {
  if (values.length === 0) return { name, layer: 'C', status: 'unavailable', reason: 'no observation was made' }
  const run = spawnSync(binary, ['metric', '--name', name, '--layer', 'C', '--unit', unit, '--mechanism', mechanism], {
    input: JSON.stringify(values),
    encoding: 'utf8'
  })
  if (run.status !== 0) throw new Error(`describing ${name}: ${run.stderr}`)
  return JSON.parse(run.stdout)
}
const count = (name, unit, value, mechanism) => ({
  name,
  layer: 'C',
  status: 'measured',
  unit,
  kind: 'count',
  value,
  mechanism
})
const unavailable = (name, reason) => ({ name, layer: 'C', status: 'unavailable', reason })

/**
 * One repetition. `trace` is the independent engine's trace (`expected` of service-trace.mjs), `series` is a list of `next` and `play`, and
 * `injectLatePause` delays the Pause of one Play cycle past the interval, which has to be rejected for the extra tick and for nothing else.
 */
export const runSimulationC = async ({ binary, manifest, seed, trace, series = ['next'], injectLatePause = false }) => {
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
  // The seed is the exact decimal text of the manifest's, which a number can round, and a runner without it cannot build the controller.
  if (typeof seed !== 'string' || !/^\d+$/.test(seed))
    throw new Error(`the Layer C runner needs the seed as its decimal text, and was given ${JSON.stringify(seed)}`)
  const fail = (section, detail) => outcome.checkpoints_failed.push({ section, detail })
  const pass = () => outcome.checkpoints_passed++
  const playWanted = series.includes('play')
  if (playWanted && manifest.limits.wall_ms < PLAY_MINIMUM_WALL_MS) {
    outcome.status = 'refused'
    outcome.error = `the Play series needs a wall_ms of at least ${PLAY_MINIMUM_WALL_MS} a repetition, since 40 cycles at 1 Hz take 40 seconds before the browser's own costs, and the manifest allows ${manifest.limits.wall_ms}`
    return outcome
  }
  const page = await openHarnessPage()
  outcome.crossOriginIsolated = page.crossOriginIsolated
  const controls = (method, ...args) =>
    page.evaluate(`window.perfHarness.sim_c.${method}(${args.map(a => JSON.stringify(a)).join(', ')})`)
  const state = () => controls('state')
  const click = async id => {
    const at = await controls('rect', id)
    if (!at) throw new Error(`there is no control ${id}`)
    await press(page, at.x, at.y)
    await release(page, at.x, at.y)
  }
  // Every state that the controller's own engine was held to the independent engine at, so that the digest of what ran is of what was read.
  const heldAt = []
  /** The controller's own engine against the independent engine's hash at the slice it is at. */
  const held = (s, k, what) => {
    heldAt.push([k, s.engine_sha256])
    if (s.committed_slice !== k || s.engine_slice !== k || s.engine_sha256 !== trace.checkpoints[k] || s.dirty) {
      fail(
        'content',
        `${what}: the controller is at slice ${s.committed_slice} with the engine at ${s.engine_slice}, and slice ${k} with the independent engine's hash was expected`
      )
      return false
    }
    pass()
    return true
  }
  const started = performance.now()
  try {
    outcome.timer_resolution_us = await page.evaluate(`(() => {
      let smallest = Infinity
      let last = performance.now()
      for (let probe = 0; probe < 200000; probe++) {
        const at = performance.now()
        const step = (at - last) * 1000
        if (step > 0 && step < smallest) smallest = step
        last = at
      }
      return Number.isFinite(smallest) ? smallest : 5
    })()`)
    await controls('install')
    let nextQualifier = null
    const next = []
    const play = []
    const clickToTimer = []
    const unattributed = {}
    const lose = reason => {
      const key = reason.replace(/\d+/g, 'N')
      unattributed[key] = (unattributed[key] ?? 0) + 1
    }

    // Next.
    if (series.includes('next')) {
      let s = await controls('reset', seed, PRIMING, manifest.simulation.arrivals)
      held(s, PRIMING, 'the start of the Next series')
      for (let i = 0; i < WARMUP; i++) {
        await click('perf-sim-next')
        await pause(PRESENT_WAIT_MS)
      }
      const before = await state()
      held(before, PRIMING + WARMUP, 'after the Next warm-up')
      const { trace: captured } = await traced(page, async () => {
        for (let i = 0; i < MEASURED; i++) {
          await click('perf-sim-next')
          await pause(PRESENT_WAIT_MS)
        }
      })
      const after = await state()
      if (after.committed_slice !== before.committed_slice + MEASURED)
        fail(
          'interaction',
          `Next moved the cursor from ${before.committed_slice} to ${after.committed_slice}, and ${MEASURED} was meant`
        )
      held(after, PRIMING + WARMUP + MEASURED, 'after the Next series')
      for (let n = before.next_marks + 1; n <= after.next_marks; n++) {
        const frame = attribute(captured, `sim-next-${n}`, { boundMs: 250, qualify: 'update' })
        if (frame.status === 'measured') {
          nextQualifier = frame.qualify
          next.push(frame.latencyMs)
        } else lose(frame.reason)
      }
      if (next.length !== MEASURED)
        fail(
          'interaction',
          `${next.length} Next observations were attributed, and ${MEASURED} were meant: ${JSON.stringify(unattributed)}`
        )
      else pass()
      if (nextQualifier !== 'update')
        fail('interaction', `the Next frames were found with the qualifier ${nextQualifier}, and update was meant`)
    }

    // Play: one tick a cycle, at the slowest speed.
    let rejected = null
    if (playWanted) {
      await controls('reset', seed, PRIMING, manifest.simulation.arrivals)
      await controls('setSpeed', INTERVAL_MS)
      const cycle = async late => {
        const before = await state()
        const clicked = performance.now()
        await click('perf-sim-play')
        let at = await state()
        for (let i = 0; i < 400 && at.timer_entries === before.timer_entries; i++) {
          await pause(10)
          at = await state()
        }
        const waited = performance.now() - clicked
        if (at.timer_entries !== before.timer_entries + 1 || at.committed_slice !== before.committed_slice + 1)
          return {
            problem: `Play did not commit one slice: ${before.committed_slice} then ${at.committed_slice}, with ${at.timer_entries - before.timer_entries} timer callbacks`
          }
        await pause(late ? INTERVAL_MS + 150 : PRESENT_WAIT_MS)
        await click('perf-sim-play')
        const after = await state()
        if (after.timer_entries !== before.timer_entries + 1)
          return {
            problem: `invalid_protocol_extra_tick: ${after.timer_entries - before.timer_entries} ticks in a cycle, and one was meant`
          }
        if (after.committed_slice !== before.committed_slice + 1)
          return {
            problem: `the cursor moved from ${before.committed_slice} to ${after.committed_slice}, and one slice was meant`
          }
        if (after.playing || after.waiting_tasks !== 0)
          return { problem: `after Pause, playing ${after.playing} and ${after.waiting_tasks} timers waiting` }
        if (after.epoch <= before.epoch) return { problem: 'the epoch did not advance on Pause' }
        if (!held(after, before.committed_slice + 1, 'a Play cycle'))
          return { problem: "the controller was not at the independent engine's state after a Play cycle" }
        return { waited }
      }
      for (let i = 0; i < WARMUP && !rejected; i++) rejected = (await cycle(false)).problem ?? null
      const before = await state()
      let captured = null
      if (!rejected) {
        captured = (
          await traced(page, async () => {
            for (let i = 0; i < MEASURED && !rejected; i++) {
              const result = await cycle(injectLatePause && i === 4)
              if (result.problem) rejected = result.problem
              else clickToTimer.push(result.waited)
            }
          })
        ).trace
      }
      const after = await state()
      if (rejected) {
        // The whole repetition is rejected and its trace is kept. Nothing is retried, and no tick is dropped or counted twice.
        fail('interaction', rejected)
        clickToTimer.length = 0
      } else {
        for (let n = before.timer_entries + 1; n <= after.timer_entries; n++) {
          const frame = attribute(captured, `sim-tick-${n}`, { boundMs: 250 })
          if (frame.status !== 'measured') {
            lose(frame.reason)
            continue
          }
          play.push(frame.latencyMs)
          // Pause is outside the timed interval: its mark comes after the presentation that is timed.
          const mark = captured.marks.get(`sim-pause-${before.pause_marks + (n - before.timer_entries)}`)
          if (!mark || !(mark.startMs > frame.presentedAtPageMs))
            fail('interaction', `the Pause of tick ${n} was not marked after its presentation`)
        }
        if (play.length !== MEASURED)
          fail(
            'interaction',
            `${play.length} Play observations were attributed, and ${MEASURED} were meant: ${JSON.stringify(unattributed)}`
          )
        else pass()
        held(after, PRIMING + WARMUP + MEASURED, 'after the Play series')
        // Nothing happens after the last Pause: no commit, no callback, no timer.
        await pause(QUIET_MS)
        const quiet = await state()
        if (
          quiet.timer_entries !== after.timer_entries ||
          quiet.committed_slice !== after.committed_slice ||
          quiet.waiting_tasks !== 0
        )
          fail('interaction', 'something ran after the last Pause')
        else pass()
      }
    }

    const mechanism = interval =>
      `${DEFINITION}. It runs from ${interval} to the end of the compositor's PipelineReporter for the frame that the correlation rule attributes to it, in a trace of blink.user_timing, devtools.timeline and cc; input from Input.dispatchMouseEvent`
    if (series.includes('next'))
      outcome.metrics.push(
        describeSamples(
          binary,
          'simulation.next.input_to_presentation',
          'ms',
          next,
          mechanism(
            'the delivery of a real click on the accessible Next button, stamped by a capture-phase listener of the harness, found with the update qualifier, which makes the figure the upper bound of a window and not a cause'
          )
        )
      )
    else
      outcome.metrics.push(
        unavailable('simulation.next.input_to_presentation', 'not run: the Next series was not asked for')
      )
    if (playWanted && !rejected)
      outcome.metrics.push(
        describeSamples(
          binary,
          'simulation.play.timer_to_presentation',
          'ms',
          play,
          mechanism(
            "the entry of the product's timer callback, stamped by the harness's scheduler as it is invoked, so that the 1,000 ms interval and the wait from the click are not in it"
          )
        )
      )
    else
      outcome.metrics.push(
        unavailable(
          'simulation.play.timer_to_presentation',
          rejected
            ? `the repetition was rejected: ${rejected}`
            : 'not run: Play costs at least 40 seconds a repetition and is scheduled on the nightly or manual profile until the CI budget is verified'
        )
      )
    if (playWanted && !rejected) {
      outcome.metrics.push(
        count(
          'simulation.play.interval',
          'ms',
          INTERVAL_MS,
          'the configured playback interval, recorded apart from the latency'
        )
      )
      outcome.metrics.push(
        describeSamples(
          binary,
          'simulation.play.click_to_timer_entry',
          'ms',
          clickToTimer,
          'the wall clock from the real Play click to the entry of the timer callback, which includes the interval and is never part of the latency'
        )
      )
      outcome.samples['simulation.play.click_to_timer_entry'] = clickToTimer
    }
    if (next.length) outcome.samples['simulation.next.input_to_presentation'] = next
    if (play.length) outcome.samples['simulation.play.timer_to_presentation'] = play
    outcome.notes.push(`Next frames were found with the ${nextQualifier ?? 'update'} qualifier`)
    outcome.cases.push({
      next_qualifier: nextQualifier,
      series,
      rejected,
      unattributed,
      seconds: (performance.now() - started) / 1000
    })
    // What ran is the engine hashes that were read at the checkpoints above, against the independent engine's at the same slices.
    const sha = text => createHash('sha256').update(text).digest('hex')
    const executed = sha(heldAt.map(([, hash]) => hash).join('\n'))
    const wanted = sha(heldAt.map(([k]) => trace.checkpoints[k]).join('\n'))
    outcome.stream = {
      scope: 'mutations',
      operations: Math.max(0, ...heldAt.map(([k]) => k)),
      sha256: executed,
      expected_sha256: wanted,
      matches: executed === wanted
    }
  } finally {
    await controls('teardown').catch(() => {})
    await page.close()
  }
  if (outcome.checkpoints_failed.length > 0 && outcome.status === 'ok') outcome.status = 'invalid_incorrect'
  return outcome
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Correctness in a real browser: real Next clicks and a real Play and Pause, with focus, the view and the accessibility tree held still.
 *
 * Every real Next click has to commit exactly one slice, leave the controller's engine at the independent engine's hash, and leave the focus
 * on the control that was clicked and the scene's view as it was: a pointer never advances time on its own and never moves the view. A real
 * Play commits one slice at 1 Hz and a real Pause stops it with nothing pending, and a real Seek reaches the slice that its control chose.
 * The browser's own accessibility tree has to expose the controls by name, with the Play control's pressed state.
 */
export const runSimulationCCorrectness = async ({ manifest, seed, trace }) => {
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
    timer_resolution_us: 5
  }
  // The seed is the exact decimal text of the manifest's, which a number can round, and a runner without it cannot build the controller.
  if (typeof seed !== 'string' || !/^\d+$/.test(seed))
    throw new Error(`the Layer C runner needs the seed as its decimal text, and was given ${JSON.stringify(seed)}`)
  const fail = (section, detail) => outcome.checkpoints_failed.push({ section, detail })
  const pass = () => outcome.checkpoints_passed++
  const page = await openHarnessPage()
  outcome.crossOriginIsolated = page.crossOriginIsolated
  const controls = (method, ...args) =>
    page.evaluate(`window.perfHarness.sim_c.${method}(${args.map(a => JSON.stringify(a)).join(', ')})`)
  const state = () => controls('state')
  const click = async id => {
    const at = await controls('rect', id)
    await press(page, at.x, at.y)
    await release(page, at.x, at.y)
    await pause(60)
  }
  const view = () =>
    page.evaluate(`(() => {
    const svg = document.getElementById('perf-sim-svg')
    const group = svg.querySelector('g')
    return JSON.stringify({ viewBox: svg.getAttribute('viewBox'), transform: group?.getAttribute('transform') ?? null, active: document.activeElement?.id ?? null })
  })()`)
  // What a pointer must not change: the view of the scene, and which control has the focus, which is the one that was used.
  const invariants = async (before, expectedActive, what) => {
    const now = JSON.parse(await view())
    if (now.viewBox !== before.viewBox || now.transform !== before.transform)
      fail('interaction', `${what} moved the view`)
    else pass()
    if (now.active !== expectedActive)
      fail('interaction', `${what} left the focus on ${now.active} and not on ${expectedActive}`)
    else pass()
  }
  /** All three controls in the browser's own accessibility tree, each by its role and name, and the Play control's pressed state. */
  const controlsInTree = async (what, pressed) => {
    const nodes = (await page.send('Accessibility.getFullAXTree')).result.nodes
    const named = (role, pattern) =>
      nodes.find(n => n.role?.value === role && pattern.test(String(n.name?.value ?? '')))
    const next = named('button', /^Next slice$/)
    const play = named('button', /^(Play|Pause)$/)
    const seek = named('slider', /^Seek to slice$/)
    if (!next) fail('accessibility', `${what}: no button named Next slice is in the accessibility tree`)
    else pass()
    if (!seek) fail('accessibility', `${what}: no slider named Seek to slice is in the accessibility tree`)
    else pass()
    if (!play) fail('accessibility', `${what}: no button named Play or Pause is in the accessibility tree`)
    else {
      const state = String(play.properties?.find(p => p.name === 'pressed')?.value?.value ?? '')
      const name = play.name.value
      if (state !== pressed || name !== (pressed === 'true' ? 'Pause' : 'Play'))
        fail(
          'accessibility',
          `${what}: the Play control is named ${name} and pressed ${state}, and ${pressed === 'true' ? 'Pause' : 'Play'} and pressed ${pressed} were meant`
        )
      else pass()
    }
  }
  const heldAt = []
  try {
    await controls('install')
    let s = await controls('reset', seed, 0, manifest.simulation.arrivals)
    heldAt.push([0, s.engine_sha256])
    await page.send('Accessibility.enable')
    await controlsInTree('at the start', 'false')
    for (let k = 1; k <= 12; k++) {
      const before = JSON.parse(await view())
      await page.evaluate("document.getElementById('perf-sim-next').focus()")
      await click('perf-sim-next')
      const after = await state()
      heldAt.push([k, after.engine_sha256])
      if (
        after.committed_slice !== k ||
        after.engine_slice !== k ||
        after.engine_sha256 !== trace.checkpoints[k] ||
        after.dirty
      )
        fail(
          'interaction',
          `a real Next click did not commit slice ${k} with the independent engine's hash: the controller is at ${after.committed_slice}`
        )
      else pass()
      const now = JSON.parse(await view())
      if (now.viewBox !== before.viewBox || now.transform !== before.transform)
        fail('interaction', `a Next click moved the view at slice ${k}`)
      else pass()
      if (now.active !== 'perf-sim-next')
        fail('interaction', `focus was on ${now.active} and not on the control that was clicked, after slice ${k}`)
      else pass()
    }
    // A real Play commits one slice, and a real Pause stops with nothing pending. While it plays the control is pressed and named Pause, and
    // the view and the focus are as they were after each of the two clicks.
    await controls('setSpeed', 1000)
    const before = await state()
    const viewBeforePlay = JSON.parse(await view())
    await click('perf-sim-play')
    await invariants(viewBeforePlay, 'perf-sim-play', 'a real Play click')
    await controlsInTree('during playback', 'true')
    let at = await state()
    for (let i = 0; i < 400 && at.timer_entries === before.timer_entries; i++) {
      await pause(10)
      at = await state()
    }
    await pause(150)
    await invariants(viewBeforePlay, 'perf-sim-play', 'a timer tick of playback')
    await click('perf-sim-play')
    await invariants(viewBeforePlay, 'perf-sim-play', 'a real Pause click')
    await controlsInTree('after Pause', 'false')
    const after = await state()
    heldAt.push([after.committed_slice, after.engine_sha256])
    if (
      after.timer_entries !== before.timer_entries + 1 ||
      after.committed_slice !== before.committed_slice + 1 ||
      after.playing ||
      after.waiting_tasks !== 0 ||
      after.engine_sha256 !== trace.checkpoints[after.committed_slice]
    )
      fail(
        'interaction',
        `a real Play and Pause did not commit exactly one slice and stop with nothing pending: ${JSON.stringify({ before: before.committed_slice, after: after.committed_slice, ticks: after.timer_entries - before.timer_entries, playing: after.playing, waiting: after.waiting_tasks })}`
      )
    else pass()
    const pressed = await page.evaluate("document.getElementById('perf-sim-play').getAttribute('aria-pressed')")
    if (pressed !== 'false') fail('accessibility', `the Play control says pressed ${pressed} after Pause`)
    else pass()
    // A real Seek: a real click on the range control, which the browser turns into a value, and a replay to that slice at a frame. The cursor has
    // to arrive there, the engine has to hold the independent engine's hash for it, and a Next after it has to commit the slice that follows.
    await page.evaluate("document.getElementById('perf-sim-seek').focus()")
    const viewBeforeSeek = JSON.parse(await view())
    const sliderAt = await controls('rect', 'perf-sim-seek')
    await press(page, sliderAt.x, sliderAt.y)
    await release(page, sliderAt.x, sliderAt.y)
    const target = Number(await page.evaluate("document.getElementById('perf-sim-seek').value"))
    let seeked = await state()
    for (let i = 0; i < 300 && seeked.committed_slice !== target; i++) {
      await pause(10)
      seeked = await state()
    }
    heldAt.push([seeked.committed_slice, seeked.engine_sha256])
    await invariants(viewBeforeSeek, 'perf-sim-seek', 'a real Seek')
    await controlsInTree('after Seek', 'false')
    const arrived =
      target > 0 &&
      seeked.committed_slice === target &&
      seeked.engine_sha256 === trace.checkpoints[target] &&
      !seeked.dirty &&
      !seeked.playing
    if (arrived) pass()
    else
      fail(
        'interaction',
        `a real click on the Seek control chose slice ${target}, and the controller is at ${seeked.committed_slice}`
      )
    const viewBeforeNext = JSON.parse(await view())
    await click('perf-sim-next')
    await invariants(viewBeforeNext, 'perf-sim-next', 'a Next click after a Seek')
    const afterSeek = await state()
    heldAt.push([afterSeek.committed_slice, afterSeek.engine_sha256])
    if (afterSeek.committed_slice === target + 1 && afterSeek.engine_sha256 === trace.checkpoints[target + 1]) pass()
    else fail('interaction', `Next after a Seek to ${target} committed slice ${afterSeek.committed_slice}`)
    outcome.gates.push({
      name: 'simulation.real_seek_was_exercised',
      status: arrived ? 'pass' : 'fail',
      detail: `a real click on the Seek control chose slice ${target}, which was reached with the independent engine's hash, and Next then committed slice ${afterSeek.committed_slice}`
    })
    const sha = text => createHash('sha256').update(text).digest('hex')
    const executed = sha(heldAt.map(([, hash]) => hash).join('\n'))
    const wanted = sha(heldAt.map(([k]) => trace.checkpoints[k]).join('\n'))
    outcome.stream = {
      scope: 'mutations',
      operations: Math.max(...heldAt.map(([k]) => k)),
      sha256: executed,
      expected_sha256: wanted,
      matches: executed === wanted
    }
  } finally {
    await controls('teardown').catch(() => {})
    await page.close()
  }
  if (outcome.checkpoints_failed.length > 0) outcome.status = 'invalid_incorrect'
  return outcome
}
