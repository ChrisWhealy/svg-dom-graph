// The gallery adapter: loads every panel of the built demo gallery in a real browser, revisits it, and walks the ones that can be
// stepped.
//
// A panel is selected through its menu entry with a real click, so the adapter works for every panel without a change to any of
// them. What it reports for each is what a person would wait for and what they would leave behind:
//
//   * the build time of the first visit and the time of a revisit, each from the click to the end of the frame that shows the
//     panel, found in a trace by the rule of `perf/lib/frames.mjs`, so the wasm that builds the panel is in the first and not in
//     the second;
//   * the DOM elements and effective listener registrations before and after the first visit;
//   * the console errors and uncaught exceptions during the visit;
//   * for a panel with Next, Previous or Restart controls found by accessible name, a walk forward, back and restart, each click
//     timed the same way, and whether the panel changed. A panel with a manual clock is advanced by the hook in
//     `perf/gallery-overrides.json`. A control that is missing downgrades the panel to a load and a revisit and does not fail
//     the sweep.
//
// The gallery builds each panel once and later only hides it (Phase 0), so a visit and a revisit are not a teardown. The sweep
// says so and does not claim one.
import fs from 'node:fs'
import path from 'node:path'
import { launch, serve } from '../../lib/chrome-session.mjs'
import { attribute } from '../../lib/frames.mjs'
import { pause } from '../../lib/input.mjs'
import { traced } from '../../lib/trace.mjs'
import { judgeWalk, unwalked } from './walk-verdict.mjs'

const root = path.resolve(import.meta.dirname, '../../..')
const tracker = fs.readFileSync(path.join(root, 'perf/page/listener-tracker.js'), 'utf8')

const DEFAULT_NAMES = {
  next: '^next\\b',
  previous: '^(previous|prev)\\b',
  // "Reset zoom" is a view control and not a restart of a walk, so the name has to begin with the word.
  restart: '^restart\\b',
}
/** The most steps a walk makes forward, and then back. */
const MAX_STEPS = 6

// The page's side: a recorder like Layer C's for the click, and the finders that a real click needs a place from.
export const PAGE = `
;(() => {
  const events = []
  addEventListener('click', event => {
    const n = events.length
    performance.mark('perf-g-' + n)
    events.push({ n, stamp: performance.now(), type: event.type })
  }, { capture: true, passive: true })
  let readyAt = null
  new MutationObserver((records, observer) => {
    if (document.querySelector('#stage .section.active svg')) { readyAt = performance.now(); observer.disconnect() }
  }).observe(document, { childList: true, subtree: true })

  const name = element => (element.getAttribute('aria-label') ?? element.textContent ?? '').trim().replace(/\\s+/g, ' ')
  const centre = element => {
    element.scrollIntoView({ block: 'center', inline: 'center' })
    const r = element.getBoundingClientRect()
    return { x: r.left + r.width / 2, y: r.top + r.height / 2 }
  }
  const section = () => document.querySelector('#stage .section.active')
  window.perfG = {
    events: () => events,
    clear: () => { events.length = 0 },
    readyAt: () => readyAt,
    menu: () => [...document.querySelectorAll('#menu .menu-item')].map(item => ({ id: item.dataset.target, label: item.textContent.trim(), group: item.closest('details')?.querySelector('summary')?.textContent.trim() ?? '' })),
    where: id => {
      const item = document.querySelector('#menu .menu-item[data-target="' + id + '"]')
      const group = item.closest('details')
      if (group) group.open = true
      return centre(item)
    },
    active: () => document.querySelector('#menu .menu-item[aria-current="page"]')?.dataset.target ?? null,
    counts: () => ({ elements: document.querySelectorAll('*').length, listeners: window.__listeners?.snapshot().total ?? null }),
    // The controls of the active panel, by accessible name, and whether each is disabled.
    controls: () => [...(section()?.querySelectorAll('[role="button"], button, [aria-label]') ?? [])]
      .map(element => ({ name: name(element), role: element.getAttribute('role') ?? element.localName, disabled: element.getAttribute('aria-disabled') === 'true' || element.disabled === true }))
      .filter(control => control.name),
    find: source => {
      const pattern = new RegExp(source, 'i')
      const element = [...(section()?.querySelectorAll('[role="button"], button') ?? [])].find(candidate => pattern.test(name(candidate)))
      return element ? { ...centre(element), name: name(element), disabled: element.getAttribute('aria-disabled') === 'true' || element.disabled === true } : null
    },
    // What the panel looks like, as numbers cannot say it exactly: positions come from measured text widths, which drift in the
    // fifth decimal place from one layout to the next, so every number is rounded to a tenth of a pixel first.
    parts: () => {
      const round = text => text.replace(/-?\\d+\\.\\d{2,}/g, number => String(Math.round(Number(number) * 10) / 10))
      const s = section()
      return s ? [...s.querySelectorAll('*')].map(element => element.localName + '|' + (element.getAttribute('class') ?? '') + '|' + round(element.getAttribute('transform') ?? '') + '|' + (element.getAttribute('fill') ?? '') + '|' + round((element.getAttribute('d') ?? '').slice(0, 60)) + '|' + round(element.getAttribute('aria-label') ?? '') + '|' + (element.children.length === 0 ? round(element.textContent) : '')) : []
    },
    // The parts as one number, so that two states can be compared without keeping both.
    fingerprint: () => {
      const parts = window.perfG.parts()
      if (parts.length === 0) return null
      let hash = 0
      for (const text of parts) for (let i = 0; i < text.length; i++) hash = (hash * 31 + text.charCodeAt(i)) | 0
      return hash
    },
  }
})()
`

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const click = async (page, at) => {
  await page.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: at.x, y: at.y })
  await page.send('Input.dispatchMouseEvent', { type: 'mousePressed', x: at.x, y: at.y, button: 'left', buttons: 1, clickCount: 1 })
  await page.send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: at.x, y: at.y, button: 'left', buttons: 0, clickCount: 1 })
}

/** Waits for the frames that follow a click to reach the screen. */
const settle = async (page, ms = 120) => {
  await page.evaluate('new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))')
  await pause(ms)
}

/** The times from each recorded click to the end of the frame that shows what it did, or why one could not be found. */
const timesOf = async (page, trace) => {
  const events = await page.evaluate('perfG.events()')
  return events.map(event => {
    const frame = attribute(trace, `perf-g-${event.n}`, { boundMs: 2000, qualify: 'update' })
    return frame.status === 'measured' ? { ms: frame.latencyMs } : { reason: frame.reason }
  })
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
/**
 * Sweeps the gallery served from `stageDir`.
 *
 * `only` limits it to the named panels, and `overrides` is the parsed `perf/gallery-overrides.json`.
 */
export const sweepGallery = async ({ stageDir, overrides = {}, only = null, log = () => {} }) => {
  if (!fs.existsSync(path.join(stageDir, 'index.html')) || !fs.existsSync(path.join(stageDir, 'pkg'))) {
    throw new Error(`there is no built gallery in ${stageDir}: run \`cargo run -p demo-server -- --build-only\``)
  }
  const { server, port } = await serve(stageDir)
  const chrome = await launch()
  const errors = []
  try {
    await chrome.send('Runtime.enable')
    await chrome.send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 900, deviceScaleFactor: 1, mobile: false })
    chrome.on('Runtime.exceptionThrown', params => errors.push({ kind: 'exception', text: params.exceptionDetails.exception?.description ?? params.exceptionDetails.text }))
    chrome.on('Runtime.consoleAPICalled', params => {
      if (params.type === 'error') errors.push({ kind: 'console.error', text: params.args.map(arg => arg.value ?? arg.description).join(' ') })
    })
    await chrome.send('Page.addScriptToEvaluateOnNewDocument', { source: tracker })
    await chrome.send('Page.addScriptToEvaluateOnNewDocument', { source: PAGE })
    await chrome.send('Page.navigate', { url: `http://127.0.0.1:${port}/` })
    for (let waited = 0; waited < 60000 && (await chrome.evaluate('window.perfG?.readyAt() ?? null')) === null; waited += 50) await pause(50)
    const coldStartMs = await chrome.evaluate('perfG.readyAt()')
    if (coldStartMs === null) throw new Error('the gallery did not become ready')
    const version = (await chrome.send('Browser.getVersion')).result?.product ?? null

    const menu = (await chrome.evaluate('perfG.menu()')).filter(item => !only || only.includes(item.id))
    const panels = []

    // A visit: a real click on the menu entry, the trace around it, and the counts either side.
    const visit = async (item, label) => {
      const before = await chrome.evaluate('perfG.counts()')
      const errorsBefore = errors.length
      const at = await chrome.evaluate(`perfG.where(${JSON.stringify(item.id)})`)
      await chrome.evaluate('perfG.clear()')
      const { trace } = await traced(chrome, async () => {
        await click(chrome, at)
        await settle(chrome, 250)
      })
      const [time] = await timesOf(chrome, trace)
      const active = await chrome.evaluate('perfG.active()')
      return { label, before, after: await chrome.evaluate('perfG.counts()'), time, active, errors: errors.slice(errorsBefore) }
    }

    // First pass: the first visit of every panel, which builds it.
    for (const item of menu) {
      const override = overrides[item.id] ?? {}
      if (override.before_build) await chrome.evaluate(override.before_build)
      const first = await visit(item, 'first')
      const record = { id: item.id, label: item.label, group: item.group, status: 'ok', notes: [], errors: first.errors }
      if (first.active !== item.id) {
        record.status = 'error'
        record.notes.push(`selecting it left ${first.active ?? 'no panel'} current`)
      }
      record.build = first.time.ms !== undefined ? { ms: first.time.ms } : { unavailable: first.time.reason }
      record.counts = { elements_before: first.before.elements, elements_after: first.after.elements, listeners_before: first.before.listeners, listeners_after: first.after.listeners }
      record.controls = await chrome.evaluate('perfG.controls()')

      // The walk, if the panel has controls to walk with.
      const names = { ...DEFAULT_NAMES, ...Object.fromEntries(['next', 'previous', 'restart'].filter(key => override[key]).map(key => [key, override[key]])) }
      const found = {}
      for (const key of ['next', 'previous', 'restart']) found[key] = await chrome.evaluate(`perfG.find(${JSON.stringify(names[key])})`)
      // A clock that has no hook to advance it is noted and left alone, and does not fail the sweep.
      let hookMissing = false
      const advance = async () => {
        if (!override.advance || hookMissing) return
        try {
          await chrome.evaluate(override.advance)
        } catch {
          hookMissing = true
          record.notes.push(`the clock hook (${override.advance}) was not there`)
        }
      }
      if (override.skip) {
        record.walk = { status: 'skipped', reason: override.skip, ...unwalked(override.skip) }
      } else if (!found.next && !found.previous && !found.restart && !override.advance) {
        const reason = 'the panel has no Next, Previous or Restart control by accessible name, and no clock to advance'
        record.walk = { status: 'downgraded', reason, ...unwalked(reason) }
      } else {
        const steps = []
        const walkErrors = errors.length
        await chrome.evaluate('perfG.clear()')
        const fingerprint = () => chrome.evaluate('perfG.fingerprint()')
        const start = await fingerprint()
        const fingerprints = { start, forward: [], back: [], beforeRestart: null, afterRestart: null }
        // Where two states differ, as the first few elements that are not the same, so that a mismatch can be read and not just counted.
        const snapshots = { start: await chrome.evaluate('perfG.parts()') }
        // The first step at which Previous missed the state that Next had left from, and where the two differ.
        const firstRetraceMiss = () => {
          for (let index = 0; index < fingerprints.back.length - 1; index++) {
            const step = fingerprints.forward.length - index - 1
            if (fingerprints.back[index] !== fingerprints.forward[step - 1]) return `at step ${step}, ${differences(`forward${step}`, `back${index}`)}`
          }
          return ''
        }
        const differences = (from, to) => {
          const a = snapshots[from]
          const b = snapshots[to]
          if (!a || !b) return ''
          const seen = []
          for (let index = 0; index < Math.max(a.length, b.length) && seen.length < 3; index++) {
            if (a[index] !== b[index]) seen.push(`#${index}: "${(a[index] ?? '(none)').slice(0, 90)}" became "${(b[index] ?? '(none)').slice(0, 90)}"`)
          }
          return a.length === b.length ? `${seen.length}+ elements differ, first ${seen.join('; ')}` : `${a.length} elements became ${b.length}; ${seen.join('; ')}`
        }
        let taken = 0
        const press = async (key, phase = key) => {
          const at = await chrome.evaluate(`perfG.find(${JSON.stringify(names[key])})`)
          if (!at || at.disabled) return false
          const changedFrom = await fingerprint()
          await click(chrome, at)
          await advance()
          await settle(chrome, 120)
          steps.push({ label: key, phase, changed: (await fingerprint()) !== changedFrom })
          return true
        }
        const { trace } = await traced(chrome, async () => {
          // Every state is kept as it is left, so that Previous can be held to landing on the one that Next left from.
          while (taken < MAX_STEPS && (await press('next'))) {
            taken++
            fingerprints.forward.push(await fingerprint())
            snapshots[`forward${taken}`] = await chrome.evaluate('perfG.parts()')
          }
          for (let back = 0; back < taken; back++) {
            if (!(await press('previous'))) break
            fingerprints.back.push(await fingerprint())
            snapshots[`back${back}`] = await chrome.evaluate('perfG.parts()')
          }
          // Restart acts on a panel that has been moved, so the panel is moved once more if it is at its start.
          if (fingerprints.back.at(-1) === start || fingerprints.back.length === 0) await press('next', 'nudge')
          fingerprints.beforeRestart = await fingerprint()
          snapshots.beforeRestart = await chrome.evaluate('perfG.parts()')
          if (await press('restart')) {
            fingerprints.afterRestart = await fingerprint()
            snapshots.afterRestart = await chrome.evaluate('perfG.parts()')
          }
          if (override.advance) {
            await advance()
            await settle(chrome, 120)
          }
        })
        const times = await timesOf(chrome, trace)
        // The clicks that were made are the recorded events in the order they were made, and the steps say what each did.
        record.walk = {
          status: steps.length || override.advance ? 'walked' : 'downgraded',
          reason: steps.length || override.advance ? undefined : 'the controls were disabled or missing when the walk began',
          forward: steps.filter(step => step.label === 'next').length,
          back: steps.filter(step => step.label === 'previous').length,
          restarted: steps.some(step => step.label === 'restart'),
          clicks: steps.length,
          changed: steps.filter(step => step.changed).length,
          step_ms: times.filter(time => time.ms !== undefined).map(time => time.ms),
          steps_unattributed: times.filter(time => time.ms === undefined).length,
          unattributed: times.filter(time => time.ms === undefined).reduce((reasons, time) => ({ ...reasons, [time.reason.replace(/\d+/g, 'N')]: (reasons[time.reason.replace(/\d+/g, 'N')] ?? 0) + 1 }), {}),
          errors: errors.slice(walkErrors),
        }
        // What the walk found is judged against what is defined for the panel, and an observation that nothing defines is not a pass.
        const expect = override.expect ?? {}
        Object.assign(record.walk, judgeWalk({ found, steps, fingerprints, errors: record.walk.errors, expect, differences: { back: firstRetraceMiss(), restart: differences('start', 'afterRestart'), restartEffect: differences('beforeRestart', 'afterRestart') } }))
        if (override.expect) record.walk.expect = override.expect
        for (const key of ['next', 'previous', 'restart']) if (!found[key]) record.notes.push(`no ${key} control by accessible name`)
        if (!found.next && !found.previous && !found.restart) record.notes.push('advanced by its clock only')
      }
      panels.push(record)
      log(`${record.status === 'ok' ? 'ok  ' : 'FAIL'} ${item.id}: build ${record.build.ms?.toFixed(1) ?? 'n/a'} ms, ${record.walk.status}${record.walk.clicks ? `, ${record.walk.clicks} clicks` : ''}`)
    }

    // Second pass: a revisit of every panel, which only shows what is already built.
    for (const record of panels) {
      const revisit = await visit({ id: record.id }, 'revisit')
      record.revisit = revisit.time.ms !== undefined ? { ms: revisit.time.ms } : { unavailable: revisit.time.reason }
      record.counts.elements_after_revisit = revisit.after.elements
      record.counts.listeners_after_revisit = revisit.after.listeners
      if (revisit.active !== record.id) {
        record.status = 'error'
        record.notes.push(`revisiting it left ${revisit.active ?? 'no panel'} current`)
      }
      record.errors.push(...revisit.errors)
    }
    for (const record of panels) if (record.errors.length > 0 && record.status === 'ok') record.status = 'errors'
    const verdicts = verdict => panels.filter(panel => panel.walk.verdict === verdict).length
    const checks = outcome => panels.reduce((total, panel) => total + panel.walk.checks.filter(c => c.outcome === outcome).length, 0)

    const walks = status => panels.filter(panel => panel.walk.status === status).length
    return {
      schema_version: 1,
      // What the sweep found, in numbers, so that the report states its own completion. A panel has loaded if selecting it made it
      // the current panel, whether or not the page then complained, and it has failed if it did not.
      summary: {
        panels: panels.length,
        panels_in_gallery: (await chrome.evaluate('perfG.menu()')).length,
        loaded: panels.filter(panel => panel.status !== 'error').length,
        failed_to_load: panels.filter(panel => panel.status === 'error').length,
        with_errors: panels.filter(panel => panel.status === 'errors').length,
        walked: walks('walked'),
        downgraded: walks('downgraded'),
        skipped: walks('skipped'),
        walks_pass: verdicts('pass'),
        walks_inconclusive: verdicts('inconclusive'),
        walks_fail: verdicts('fail'),
        checks_pass: checks('pass'),
        checks_inconclusive: checks('inconclusive'),
        checks_fail: checks('fail'),
        first_visit_timed: panels.filter(panel => panel.build.ms !== undefined).length,
        revisit_timed: panels.filter(panel => panel.revisit?.ms !== undefined).length,
      },
      browser: version,
      viewport: [1280, 900],
      cold_start_ms: coldStartMs,
      cold_start_definition: 'performance.now() in the page when the first panel is on screen, which is from the start of the navigation',
      teardown: 'not measured: the gallery builds each panel once and only hides it afterwards, so a visit and a revisit are not a teardown',
      panels,
      page_errors: errors,
    }
  } finally {
    await chrome.close()
    server.close()
  }
}
