// The gallery panel's acceptance tests G01 to G10: the real gallery route, the real controls, one Chrome.
//
//   node dess/demo-panel/test-gallery.mjs [--stage target/demo-stage] [--only G04,G06] [--out dess/demo-panel/evidence/gallery.json]
//
// The gallery is the one that `cargo run -p demo-server -- --build-only` stages, and it is served as it is: no second build, no test fixture, no hook in
// the wasm. A test only ever does what a person can. It clicks and types with real input events, and it reads what the page shows. The oracle is the
// independent JavaScript engine (`dess/reference-js`), which shares no code with the engine in the page, and never a state machine written for the test.
//
// Failures are made by the page-level injector (`perf/page/fault-injector.js`), which is injected before the gallery's own script. It holds timers and
// frames, so that no test waits for a wall-clock interval, and it makes a DOM call fail. Time is never a gate. One test, G06, lets the browser's own
// timer run once at 1 Hz, to see that the real scheduler is the one in use, and it says how long that took and nothing else.
//
// G05's rapid arrow keys are run on the 12 snapshots of the walkthrough. Its run on the 5,464 of the W6 reference trace needs the scenario to be
// switched, which is the lifecycle stage (P3), and it is reported as deferred and not as a pass.
import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { launch, serve } from '../../perf/lib/chrome-session.mjs'
import { press, release, move, pause, wheel } from '../../perf/lib/input.mjs'
import { run } from '../reference-js/oracle.mjs'
import { MIXED_OUTAGE_7 } from '../reference-js/generator.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const stageDir = path.resolve(argument('--stage') ?? path.join(root, 'target/demo-stage'))
const only = argument('--only')?.split(',') ?? null
/** The nightly runs: the long paths G18 and G19, and the cycles of G20 in the hundreds. */
const nightly = process.argv.includes('--nightly')
const CYCLES = Number(argument('--cycles') ?? (nightly ? 40 : 3))
const out = argument('--out') ? path.resolve(argument('--out')) : null
const PANEL = 'panel-service-simulator'
const COUNTER_KEYS = [
  'arrived',
  'admitted',
  'rejected',
  'completed',
  'timed-out',
  'cancelled',
  'failed',
  'retrying',
  'stale',
  'no-op'
]
const COUNTER_NAMES = [
  'Arrived',
  'Admitted',
  'Rejected',
  'Completed',
  'Timed out',
  'Cancelled',
  'Failed',
  'Waiting to retry',
  'Stale pops',
  'No-op pops'
]

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// The oracle: the state of E0 after k slices, from the independent engine, in the form that the page shows it.
const fixtures = JSON.parse(fs.readFileSync(path.join(root, 'dess/s1/fixtures.json'), 'utf8'))
const E0_SLICES = fixtures.E0.expect.slices
/** The five-server scenario's parameters, which are the frozen generator's with five servers and its own revision. */
const FIVE = { ...MIXED_OUTAGE_7, servers: 5, arrivals: 360, revision: 'gallery/five-servers' }
/** The scenarios of the catalogue, as the independent engine runs them, with what the panel shows of their shape. */
const SCENARIOS = {
  E0: {
    servers: 2,
    capacity: 3,
    slices: 12,
    run: { configId: 'fixture/E0', seed: 0, config: fixtures.E0.config, script: fixtures.E0.script }
  },
  'E3-NOOP': {
    servers: 1,
    capacity: 2,
    slices: fixtures['E3-NOOP'].expect.slices,
    run: {
      configId: 'fixture/E3-NOOP',
      seed: 0,
      config: fixtures['E3-NOOP'].config,
      script: fixtures['E3-NOOP'].script
    }
  },
  five: {
    servers: 5,
    capacity: MIXED_OUTAGE_7.queueCapacity,
    slices: 493,
    run: {
      configId: 'gallery/five-servers:20261106',
      seed: 20261106,
      config: FIVE,
      params: FIVE
    }
  },
  scaled: {
    servers: 3,
    capacity: MIXED_OUTAGE_7.queueCapacity,
    slices: 487,
    run: {
      configId: 'mixed_outage/7',
      seed: 20261106,
      config: MIXED_OUTAGE_7,
      params: { ...MIXED_OUTAGE_7, arrivals: 360 }
    }
  },
  W6: {
    servers: 3,
    capacity: MIXED_OUTAGE_7.queueCapacity,
    slices: 5464,
    run: { configId: 'mixed_outage/7', seed: 20261106, config: MIXED_OUTAGE_7, params: MIXED_OUTAGE_7 }
  }
}
const E0S = SCENARIOS.E0
const oracleCache = new Map()
/** The state of a scenario after k slices, from the independent engine, in the form that the page shows it. */
const oracleAt = (scenario, k) => {
  if (k === 0) return { overlay: Array(10).fill(0), data: Array(8).fill(0), tick: 0, waiting: 0, busy: 0 }
  const cached = oracleCache.get(`${scenario.run.configId}/${k}`)
  if (cached) return cached
  const r = run(scenario.run, { stopAfterSlices: k })
  const c = r.counters
  const s = r.slices[k - 1]
  const retry = r.jobs.filter(j => j[2].startsWith('retrywait')).length
  const state = {
    overlay: [
      c.arrivals,
      c.admitted,
      c.rejected,
      c.completed,
      c.timedOut,
      c.cancelled,
      c.failedAttempts + c.failedDeferrals,
      retry,
      c.stale,
      c.noops
    ],
    data: [k, s.tick, s.waiting, s.busy, c.completed, c.timedOut, c.cancelled, c.rejected],
    tick: s.tick,
    waiting: s.waiting,
    busy: s.busy
  }
  oracleCache.set(`${scenario.run.configId}/${k}`, state)
  return state
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const { server, port } = await serve(stageDir)
const chrome = await launch()
const { send } = chrome
const evaluate = async expression =>
  chrome.evaluate(`JSON.stringify(${expression})`).then(v => (v === undefined ? undefined : JSON.parse(v)))
const exceptions = []
chrome.on('Runtime.exceptionThrown', p =>
  exceptions.push(p.exceptionDetails.exception?.description ?? p.exceptionDetails.text)
)
chrome.on('Runtime.consoleAPICalled', p => {
  if (p.type === 'error') exceptions.push(`console.error: ${p.args.map(a => a.value ?? a.description).join(' ')}`)
})
await send('Runtime.enable')
await send('Emulation.setDeviceMetricsOverride', { width: 1500, height: 1000, deviceScaleFactor: 1, mobile: false })
// Before the gallery's own script: the listener tracker, the fault injector, and a count of what the page asks the browser for.
for (const file of ['perf/page/listener-tracker.js', 'perf/page/fault-injector.js'])
  await send('Page.addScriptToEvaluateOnNewDocument', { source: fs.readFileSync(path.join(root, file), 'utf8') })
await send('Page.addScriptToEvaluateOnNewDocument', {
  source: `;(() => {
    window.__seen = { frames: 0, timers: [], observers: [], captured: { timers: [], frames: [] } }
    const requestFrame = window.requestAnimationFrame
    window.requestAnimationFrame = callback => { window.__seen.frames++; window.__seen.captured.frames.push(callback); return requestFrame(callback) }
    const setTimer = window.setTimeout
    window.setTimeout = (callback, ...rest) => { window.__seen.timers.push(rest[0] ?? 0); window.__seen.captured.timers.push(callback); return setTimer(callback, ...rest) }
    const observe = MutationObserver.prototype.observe
    MutationObserver.prototype.observe = function (target, options) {
      window.__seen.observers.push({ id: target.id ?? target.tagName, filter: options?.attributeFilter ?? null })
      window.__liveObservers.add(this)
      return observe.call(this, target, options)
    }
    // The observers that are still watching, which is what an owner that is taken down must leave none of.
    window.__liveObservers = new Set()
    const disconnect = MutationObserver.prototype.disconnect
    MutationObserver.prototype.disconnect = function () {
      window.__liveObservers.delete(this)
      return disconnect.call(this)
    }
    // The wasm instances, so that the table of externrefs that wasm-bindgen keeps can be read. Nothing in the gallery is changed for this.
    window.__wasmInstances = []
    for (const name of ['instantiate', 'instantiateStreaming']) {
      const native = WebAssembly[name]
      WebAssembly[name] = async (...args) => {
        const result = await native.apply(WebAssembly, args)
        window.__wasmInstances.push(result.instance ?? result)
        return result
      }
    }
    window.__counts = () => {
      const exports = window.__wasmInstances.at(-1)?.exports
      return {
        elements: document.getElementsByTagName('*').length,
        listeners: __listeners.snapshot().total,
        observers: window.__liveObservers.size,
        externrefs: exports?.__wbindgen_externrefs?.length ?? null,
        held: { timers: __timers.pending().length, frames: __frames.pending() }
      }
    }
  })()`
})

const url = `http://127.0.0.1:${port}/`
/** Loads the gallery on the panel, in a page of its own, and waits for the panel to say that it is built. */
const fresh = async (hash = PANEL) => {
  await send('Page.navigate', { url: 'about:blank' })
  await pause(50)
  await send('Page.navigate', { url: `${url}#${hash}` })
  for (let waited = 0; waited < 20000; waited += 50) {
    if ((await evaluate(`document.getElementById('${PANEL}')?.dataset.panelState ?? null`)) === 'ready') {
      // The controls are in the scene, so they leave the document with it. Their elements are kept, to be read while the scene is detached.
      await evaluate(
        `(window.__byId = Object.fromEntries([...document.querySelectorAll('#dess-toolbar [id], #dess-side [id]')].map(e => [e.id, e])), 0)`
      )
      return
    }
    await pause(50)
  }
  throw new Error('the panel did not come up')
}
const until = async (expression, ms = 4000) => {
  for (let waited = 0; waited < ms; waited += 25) {
    if (await evaluate(expression)) return true
    await pause(25)
  }
  return false
}
const centre = selector =>
  evaluate(
    `(() => { const e = document.querySelector(${JSON.stringify(selector)}); e.scrollIntoView({ block: 'center' }); const r = e.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 } })()`
  )
/** A real click: the pointer moves to the middle of the element, and presses and releases there. */
const click = async selector => {
  const { x, y } = await centre(selector)
  await move(chrome, x, y)
  await press(chrome, x, y)
  await release(chrome, x, y)
  await pause(20)
}
const KEYS = {
  Tab: { key: 'Tab', code: 'Tab', vk: 9 },
  Enter: { key: 'Enter', code: 'Enter', vk: 13, text: '\r' },
  Space: { key: ' ', code: 'Space', vk: 32, text: ' ' },
  ArrowRight: { key: 'ArrowRight', code: 'ArrowRight', vk: 39 },
  ArrowLeft: { key: 'ArrowLeft', code: 'ArrowLeft', vk: 37 },
  ArrowDown: { key: 'ArrowDown', code: 'ArrowDown', vk: 40 },
  KeyN: { key: 'n', code: 'KeyN', vk: 78, text: 'n' }
}
/** A real key press: the browser decides what it does to the focused element. */
const key = async name => {
  const k = KEYS[name]
  await send('Input.dispatchKeyEvent', {
    type: k.text ? 'keyDown' : 'rawKeyDown',
    key: k.key,
    code: k.code,
    windowsVirtualKeyCode: k.vk,
    text: k.text
  })
  await send('Input.dispatchKeyEvent', { type: 'keyUp', key: k.key, code: k.code, windowsVirtualKeyCode: k.vk })
  await pause(20)
}
const focus = selector =>
  evaluate(
    `(() => { document.querySelector(${JSON.stringify(selector)}).focus(); return document.activeElement.id })()`
  )
const setSpeed = ms =>
  evaluate(
    `(() => { const s = document.getElementById('dess-speed'); s.value = '${ms}'; s.dispatchEvent(new Event('change', { bubbles: true })); return s.value })()`
  )

/** What the page shows, read from the page's own elements. */
const read = () =>
  evaluate(
    `(() => {
    const text = id => document.getElementById(id)?.textContent ?? null
    const svg = document.querySelector('#dess-stage svg')
    const slots = {}
    for (const g of svg.querySelectorAll('[data-ss-key]'))
      slots[g.getAttribute('data-ss-key')] = { text: g.querySelector('text')?.textContent ?? '', cls: g.getAttribute('class') ?? '', label: g.getAttribute('aria-label') ?? '', hidden: g.getAttribute('aria-hidden') === 'true' }
    const content = svg.querySelector(':scope > g.svg-dom-graph-content')
    const nodes = [...content.querySelectorAll(':scope > g')].filter(g => !g.classList.contains('ss-overlay'))
    const data = [...nodes[nodes.length - 1].querySelectorAll('text')].map(t => t.textContent.trim())
    const attr = (id, name) => document.getElementById(id).getAttribute(name)
    return {
      slots, data, transform: content.getAttribute('transform'),
      snapshot: text('dess-snapshot'), iSnapshot: text('dess-i-snapshot'), tick: text('dess-i-tick'), waiting: text('dess-i-waiting'), busy: text('dess-i-busy'),
      selected: text('dess-i-selected'), state: text('dess-i-state'), display: text('dess-i-display'), live: text('dess-live'),
      counters: COUNTER_KEYS.map(k => text('dess-c-' + k)),
      seek: { value: document.getElementById('dess-seek').value, max: document.getElementById('dess-seek').max, valuetext: attr('dess-seek', 'aria-valuetext'), text: text('dess-seek-text'), description: text('dess-seek-desc') },
      play: { text: text('dess-play'), pressed: attr('dess-play', 'aria-pressed') },
      prev: { disabled: attr('dess-prev', 'aria-disabled'), hasDisabled: document.getElementById('dess-prev').hasAttribute('disabled'), reason: text('dess-prev-reason') },
      next: { disabled: attr('dess-next', 'aria-disabled'), hasDisabled: document.getElementById('dess-next').hasAttribute('disabled'), reason: text('dess-next-reason') },
      errorHidden: document.getElementById('dess-error').hidden, errorText: text('dess-error'),
      notice: { hidden: document.getElementById('dess-notice').hidden, text: text('dess-notice') }
    }
  })()`.replace('COUNTER_KEYS', JSON.stringify(COUNTER_KEYS))
  )
const slice = async () => Number((await read()).iSnapshot.split(' ')[0])
const hold = async on => {
  await evaluate(`(__timers.hold(${on}), __frames.hold(${on}), 0)`)
}
const accessibility = async () => {
  await send('Accessibility.enable')
  const tree = await send('Accessibility.getFullAXTree')
  return tree.result.nodes.filter(n => !n.ignored)
}

const pending = () => evaluate('({ timers: __timers.pending(), frames: __frames.pending() })')

/** Chooses a scenario in the list, which is the request for it. The answer is read from the page afterwards. */
const chooseScenario = id =>
  evaluate(
    `(() => { const s = document.getElementById('dess-scenario'); s.value = '${id}'; s.dispatchEvent(new Event('change', { bubbles: true })); return s.value })()`
  )
/** What the page says about the scenario that is on it, and what is in the DOM for it. */
const scenarioState = () =>
  evaluate(`(() => {
    const hosts = [...document.querySelectorAll('[id^="dess-host-"]')]
    return {
      select: document.getElementById('dess-scenario').value,
      hosts: hosts.map(h => ({ id: h.id, staged: h.hasAttribute('inert') || h.hasAttribute('style') })),
      svgs: document.querySelectorAll('#dess-stage svg').length,
      servers: [...document.querySelectorAll('#dess-stage [data-ss-key]')].filter(g => /^S\\d+$/.test(g.getAttribute('data-ss-key'))).length,
      max: document.getElementById('dess-seek').max,
      snapshot: document.getElementById('dess-i-snapshot').textContent,
      errorShown: !document.getElementById('dess-error').hidden, error: document.getElementById('dess-error').textContent,
      transform: document.querySelector('#dess-stage svg > g.svg-dom-graph-content')?.getAttribute('transform') ?? null
    }
  })()`)
/** How many scenes the accessibility tree exposes: the overlay is a group named Service simulator, and there is one in each scene. */
const scenes = async () =>
  (await accessibility()).filter(n => n.role?.value === 'group' && n.name?.value === 'Service simulator').length
/** Where the scene is on the screen, and how far the page and every container above the scene are scrolled. A staged host that took room, or a focus that scrolled, moves one of them. */
const layoutOf = () =>
  evaluate(`(() => {
    const svg = document.querySelector('#dess-stage svg')
    const r = svg.getBoundingClientRect()
    const scrolls = [[window.scrollX, window.scrollY]]
    for (let e = svg.parentElement; e; e = e.parentElement) scrolls.push([e.scrollLeft, e.scrollTop])
    return { rect: [r.left, r.top, r.width, r.height], scrolls, document: document.documentElement.scrollHeight }
  })()`)
const unmoved = (a, b) =>
  a.rect.every((v, i) => Math.abs(v - b.rect[i]) <= 0.5) && same(a.scrolls, b.scrolls)
const routeAway = () => click('#menu .menu-item[data-target="panel-petri-net"]')
const routeBack = () => click(`#menu .menu-item[data-target="${PANEL}"]`)
/** What the page says without reading the scene, for when the scene is not in the document. */
const status = () =>
  evaluate(`(() => {
    const text = id => (document.getElementById(id) ?? window.__byId[id]).textContent
    return { snapshot: text('dess-i-snapshot'), state: text('dess-i-state'), display: text('dess-i-display'), live: text('dess-live'), error: text('dess-error'), errorHidden: document.getElementById('dess-error').hidden }
  })()`)
const census = () =>
  evaluate(
    `({ observers: __seen.observers.filter(o => o.id === 'stage').length, listeners: __listeners.snapshot().total, hosts: [...document.querySelectorAll('[id^="dess-host-"]')].map(e => e.id) })`
  )

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const results = []
let failures = 0
let current = null
const check = (ok, text) => {
  if (!ok) failures++
  current.checks.push({ ok: Boolean(ok), text })
  console.log(`  ${ok ? 'ok  ' : 'FAIL'} ${text}`)
}
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b)
const test = async (id, title, body, { long = false } = {}) => {
  if (only ? !only.includes(id) : long && !nightly) return
  exceptions.length = 0
  current = { id, title, checks: [], deferred: null }
  results.push(current)
  console.log(`${id} ${title}`)
  try {
    await body()
  } catch (error) {
    failures++
    current.checks.push({ ok: false, text: `the test stopped: ${error.stack ?? error}` })
    console.log(`  FAIL the test stopped: ${error.stack ?? error}`)
  }
}

/** What the page shows for snapshot k, compared with what the independent engine says that it is. */
const matchesOracle = (page, k, scenario = E0S) => {
  const o = oracleAt(scenario, k)
  const problems = []
  if (page.iSnapshot !== `${k} of ${scenario.slices}`) problems.push(`snapshot ${page.iSnapshot}`)
  if (page.tick !== String(o.tick)) problems.push(`tick ${page.tick} not ${o.tick}`)
  if (page.waiting !== `${o.waiting} of ${scenario.capacity}`) problems.push(`waiting ${page.waiting} not ${o.waiting}`)
  if (page.busy !== `${o.busy} of ${scenario.servers}`) problems.push(`busy ${page.busy} not ${o.busy}`)
  if (!same(page.counters, o.overlay.map(String))) problems.push(`inspector counters ${page.counters} not ${o.overlay}`)
  const overlay = COUNTER_KEYS.map(k2 => page.slots[k2].text)
  if (!same(overlay, o.overlay.map(String))) problems.push(`overlay counters ${overlay} not ${o.overlay}`)
  const labels = COUNTER_KEYS.map((k2, i) => page.slots[k2].label)
  if (
    !same(
      labels,
      COUNTER_NAMES.map((n, i) => `${n}: ${o.overlay[i]}`)
    )
  )
    problems.push(`overlay labels ${labels}`)
  if (!same(page.data, o.data.map(String))) problems.push(`data cells ${page.data} not ${o.data}`)
  if (page.slots.clock.text !== `tick ${o.tick} · slice ${k}`) problems.push(`clock ${page.slots.clock.text}`)
  const shown = Object.keys(page.slots).filter(s => /^Q\d+$/.test(s) && !page.slots[s].hidden).length
  if (shown !== Math.min(o.waiting, 12)) problems.push(`${shown} queue slots shown, not ${o.waiting}`)
  return problems
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
await test('G01', 'the menu, the deep link and the source frame agree with the registry', async () => {
  await fresh()
  const menu = await evaluate(`(() => {
    const items = [...document.querySelectorAll('#menu .menu-item')]
    const at = items.findIndex(i => i.dataset.target === '${PANEL}')
    return { at, before: items[at - 1]?.dataset.target, after: items[at + 1]?.dataset.target, label: items[at]?.textContent.trim(), current: items[at]?.getAttribute('aria-current'),
             group: items[at]?.closest('details.menu-group')?.querySelector('summary')?.textContent.trim() }
  })()`)
  check(menu.before === 'panel-petri-net', `the menu puts the panel immediately after Petri Net (after ${menu.before})`)
  check(menu.group?.toLowerCase().includes('concurrency'), `in the Concurrency group (${menu.group})`)
  check(menu.current === 'page', 'a deep link to the panel selects it and marks its menu item as the current page')
  const section = await evaluate(
    `(() => { const s = document.getElementById('${PANEL}'); return { active: s.classList.contains('active'), tag: s.tagName, state: s.dataset.panelState, label: s.dataset.label } })()`
  )
  check(section.active && section.state === 'ready', 'the section of that id is the active one, and its panel is built')
  check(
    section.label === menu.label || menu.label.includes(section.label),
    `the menu item says what the section's data-label says (${section.label})`
  )
  const lib = fs.readFileSync(path.join(root, 'demo-app/src/lib.rs'), 'utf8')
  const registered = lib.match(new RegExp(`"${PANEL}" => ([a-z_]+)::([a-z_]+)( \\+ [a-z_:A-Z]+)?`))
  check(
    registered?.[1] === 'service_sim' && registered[2] === 'build_service_simulator_demo',
    `demo_gallery! registers ${PANEL} (${registered?.[0]})`
  )
  const frames = await evaluate(
    `[...document.querySelectorAll('#${PANEL} details.source')].map(d => d.querySelector('summary')?.textContent.trim())`
  )
  const files = (
    fs.readFileSync(path.join(root, 'demo-app/src/service_sim/mod.rs'), 'utf8').match(/source_file!\(/g) ?? []
  ).length
  check(
    frames[0]?.includes('build_service_simulator_demo'),
    `the first source frame is the function that built it (${frames[0]})`
  )
  check(
    frames.length === 1 + files,
    `and there is one frame for each of the ${files} files that the registry lists (${frames.length - 1})`
  )
  const order = fs.readdirSync(path.join(root, 'demo/panels/16-concurrency')).sort()
  check(
    order.indexOf('02-service-simulator.html') === order.indexOf('01-petri-net.html') + 1,
    'the fragments are numbered 01-petri-net and 02-service-simulator'
  )
})

await test('G02', 'the panel is built once, and a revisit adds no owner, observer, SVG, listener or id', async () => {
  await fresh()
  const census = () =>
    evaluate(`(() => {
      const ids = [...document.querySelectorAll('[id]')].map(e => e.id)
      const duplicates = ids.filter((id, i) => ids.indexOf(id) !== i)
      const section = document.getElementById('${PANEL}')
      return {
        svgs: section.querySelectorAll('svg').length, hosts: [...document.querySelectorAll('[id^="dess-host-"]')].map(e => e.id),
        overlays: section.querySelectorAll('.ss-overlay').length, duplicates, listeners: __listeners.snapshot().total,
        observers: __seen.observers.filter(o => o.id === 'stage').length, buttons: section.querySelectorAll('button').length
      }
    })()`)
  const first = await census()
  check(
    first.svgs === 1 && same(first.hosts, ['dess-host-0']) && first.overlays === 1,
    `the first visit builds one SVG, one host and one overlay (${first.svgs}, ${first.hosts}, ${first.overlays})`
  )
  check(first.observers === 1, `and one watch on the gallery's stage (${first.observers})`)
  check(first.duplicates.length === 0, 'and no id twice in the page')
  // The first round trip lets the other panel make its own listeners, and is the baseline.
  await click('#menu .menu-item[data-target="panel-petri-net"]')
  await until(`document.getElementById('panel-petri-net').dataset.panelState === 'ready'`)
  await click(`#menu .menu-item[data-target="${PANEL}"]`)
  await pause(100)
  const baseline = await census()
  for (let i = 0; i < 3; i++) {
    await click('#menu .menu-item[data-target="panel-petri-net"]')
    await click(`#menu .menu-item[data-target="${PANEL}"]`)
  }
  await pause(100)
  const after = await census()
  check(
    after.svgs === 1 && same(after.hosts, ['dess-host-0']) && after.overlays === 1,
    'three more round trips leave one SVG, the same host and one overlay'
  )
  check(
    after.listeners === baseline.listeners,
    `and the same number of effective listeners (${baseline.listeners} then ${after.listeners})`
  )
  check(after.observers === 1, `and still one watch on the stage (${after.observers})`)
  check(
    after.duplicates.length === 0 && after.buttons === baseline.buttons,
    'and no duplicate id and no second set of controls'
  )
  const state = await evaluate(`document.getElementById('${PANEL}').dataset.panelState`)
  check(state === 'ready', 'and the panel still says that it was built once')
})

await test('G03', 'the first draw is the walkthrough at snapshot 0', async () => {
  await fresh()
  const page = await read()
  check(page.snapshot === 'Snapshot 0 of 12', `the heading says ${page.snapshot}`)
  const counts =
    await evaluate(`(() => { const svg = document.querySelector('#dess-stage svg'); return { svgs: document.querySelectorAll('#dess-stage svg').length, overlays: svg.querySelectorAll('.ss-overlay').length,
    queue: [...svg.querySelectorAll('[data-ss-key]')].filter(g => /^Q\\d+$/.test(g.getAttribute('data-ss-key'))).length,
    servers: [...svg.querySelectorAll('[data-ss-key]')].filter(g => /^S\\d+$/.test(g.getAttribute('data-ss-key'))).length,
    viewBox: svg.getAttribute('viewBox'), contents: svg.querySelectorAll(':scope > g.svg-dom-graph-content').length } })()`)
  check(
    counts.svgs === 1 && counts.overlays === 1 && counts.contents === 1,
    'one SVG, one content group and one overlay'
  )
  check(
    counts.queue === 12 && counts.servers === 2,
    `twelve queue slots and two servers (${counts.queue}, ${counts.servers})`
  )
  check(counts.viewBox === '0 -150 1340 630', `the fixed view box (${counts.viewBox})`)
  const problems = matchesOracle(page, 0)
  check(
    problems.length === 0,
    `the data cells, the ten counters, the clock and the queue are the independent engine's snapshot 0 ${problems.join('; ')}`
  )
  check(
    page.slots.S0.text === 'idle' && page.slots.S1.text === 'idle',
    `both servers are idle (${page.slots.S0.text}, ${page.slots.S1.text})`
  )
  check(
    page.seek.max === '12' && page.seek.valuetext === 'Snapshot 0 of 12',
    `the range runs to the preflight's 12 (${page.seek.max}, ${page.seek.valuetext})`
  )
  check(page.play.text === 'Play' && page.play.pressed === 'false', 'Play reads Play and is not pressed')
  check(page.state === 'Paused / visible' && page.display === 'Up to date', `${page.state}, ${page.display}`)
  check(page.errorHidden, 'and no error is shown')
})

await test('G04', 'twelve Next clicks match the independent engine at every snapshot', async () => {
  await fresh()
  const ordinals = [0]
  for (let k = 1; k <= E0_SLICES; k++) {
    await click('#dess-next')
    const page = await read()
    const problems = matchesOracle(page, k)
    ordinals.push(Number(page.data[0]))
    if (problems.length) check(false, `snapshot ${k}: ${problems.join('; ')}`)
  }
  check(
    ordinals.every((n, i) => n === i),
    `the data node's ordinal advanced by one at each commit (${ordinals})`
  )
  const last = await read()
  check(matchesOracle(last, E0_SLICES).length === 0, 'and snapshot 12 is the end of the run: every counter agrees')
  const expected = fixtures.E0.expect
  check(
    last.slots.S0.text === 'idle' && last.slots.S1.text === 'idle',
    'both servers are idle at the end, as the fixture says'
  )
  check(
    same(last.counters.map(Number).slice(0, 7), [
      expected.arrivals,
      expected.admitted,
      expected.rejected,
      expected.completed,
      expected.timedOut,
      expected.cancelled,
      expected.failed
    ]),
    "and the totals are the fixture's own known answers"
  )
  check(
    last.next.disabled === 'true' && last.next.reason === 'Already at last snapshot',
    'Next says that it is at the last snapshot, and is aria-disabled'
  )
})

await test('G05', 'Previous, Restart and a seek equal a full redraw, and each arrow key is a request', async () => {
  await fresh()
  // After each move, the page is read, repaired in full by a click, and read again. A delta that is not the full redraw would differ.
  const settle = async k => {
    await until(`document.getElementById('dess-seek-text').textContent === 'Committed ${k}'`)
    const before = await read()
    await click('#dess-repair')
    const after = await read()
    const { slots: a, data: b, ...restBefore } = before
    check(
      same(before.slots, after.slots) && same(before.data, after.data),
      `snapshot ${k}: the page equals its own full redraw`
    )
    check(
      matchesOracle(after, k).length === 0,
      `snapshot ${k}: and the independent engine's ${matchesOracle(after, k).join('; ')}`
    )
    return restBefore
  }
  for (let i = 0; i < 6; i++) await click('#dess-next')
  await click('#dess-prev')
  await settle(5)
  await focus('#dess-seek')
  for (const target of [9, 2, 12, 0, 7]) {
    await evaluate(
      `(() => { const s = document.getElementById('dess-seek'); s.value = '${target}'; s.dispatchEvent(new Event('input', { bubbles: true })) })()`
    )
    await settle(target)
  }
  await click('#dess-restart')
  await settle(0)
  // Rapid arrow keys: each press is one request, and the last one wins.
  await focus('#dess-seek')
  await evaluate('__seen.frames = 0')
  await hold(true)
  for (let i = 0; i < 5; i++) await key('ArrowRight')
  const during = await read()
  const held = await pending()
  check(
    during.seek.value === '5' && during.seek.valuetext === 'Snapshot 5 of 12',
    `after five presses the range asks for 5 and says ${during.seek.valuetext}`
  )
  check(
    during.iSnapshot === '0 of 12' && during.seek.text === 'Requested 5, committed 0',
    `while the committed snapshot is still 0 (${during.seek.text})`
  )
  check(
    during.seek.description === 'Snapshot 0 is still the committed one.',
    `and the description names it (${during.seek.description})`
  )
  const requests = await evaluate('__seen.frames')
  check(requests === 5, `each press asked the browser for a frame of its own (${requests})`)
  check(held.frames === 1, `and the last one is the only one that is still waiting (${held.frames})`)
  await evaluate('__frames.release()')
  await hold(false)
  await until(`document.getElementById('dess-seek-text').textContent === 'Committed 5'`)
  const done = await read()
  check(
    done.iSnapshot === '5 of 12' && done.seek.valuetext === 'Snapshot 5 of 12',
    `the last request is the one that committed (${done.iSnapshot})`
  )
  const ax = await accessibility()
  const slider = ax.find(n => n.role?.value === 'slider')
  check(
    slider?.name?.value === 'Snapshot' && slider?.value?.value === 5,
    `the accessibility tree has a slider named Snapshot, at 5 (${slider?.name?.value}, ${slider?.value?.value})`
  )
  check(done.seek.valuetext === 'Snapshot 5 of 12', `and its aria-valuetext says ${done.seek.valuetext}`)
  const described = slider?.properties?.find(p => p.name === 'describedby')?.value?.value
  check(
    described === 'dess-seek-desc',
    `and it is described by the element that names the committed snapshot (${described})`
  )
  current.limits = [
    "Chrome's DevTools accessibility tree reports a range's value as its number and ignores aria-valuetext: a custom role=slider that has it is reported with an empty valuetext in the same Chrome (157). So what is announced is checked at the attribute, and a screen reader's own reading is a manual observation for P4."
  ]
  console.log(`  note ${current.limits[0]}`)
  current.deferred = 'the same arrow keys on the 5,464 snapshots of the W6 reference trace are G05W6, below'
  console.log(`  note ${current.deferred}`)
})

await test('G06', 'playback takes one slice per timer at each rate, and Play at the end starts nothing', async () => {
  await fresh()
  await hold(true)
  for (const [ms, name] of [
    [1000, '1 Hz'],
    [500, '2 Hz'],
    [200, '5 Hz'],
    [100, '10 Hz']
  ]) {
    await click('#dess-restart')
    await setSpeed(ms)
    await click('#dess-play')
    let p = await pending()
    const page = await read()
    check(
      p.timers.length === 1 && p.timers[0].delay === ms,
      `${name}: Play holds one timer, of ${ms} ms (${JSON.stringify(p.timers)})`
    )
    check(
      page.play.text === 'Pause' && page.play.pressed === 'true',
      `${name}: and the button reads Pause and is pressed`
    )
    const before = await slice()
    await evaluate(`__timers.fire(${p.timers[0].id})`)
    p = await pending()
    check(
      (await slice()) === before + 1 && p.timers.length === 1,
      `${name}: one firing commits one slice and asks for the next timer only (${p.timers.length} held)`
    )
    await click('#dess-play')
    p = await pending()
    check(
      p.timers.length === 0 && (await read()).play.pressed === 'false',
      `${name}: Pause takes the timer back and unpresses the button`
    )
  }
  // The rate is replaced while playing.
  await click('#dess-restart')
  await setSpeed(1000)
  await click('#dess-play')
  await setSpeed(200)
  let p = await pending()
  check(
    p.timers.length === 1 && p.timers[0].delay === 200,
    `a new rate replaces the waiting timer and does not add one (${JSON.stringify(p.timers)})`
  )
  await click('#dess-play')
  // At the known end: seek there, and press Play.
  await hold(false)
  await focus('#dess-seek')
  await evaluate(
    `(() => { const s = document.getElementById('dess-seek'); s.value = '12'; s.dispatchEvent(new Event('input', { bubbles: true })) })()`
  )
  await until(`document.getElementById('dess-seek-text').textContent === 'Committed 12'`)
  await hold(true)
  const clean = await pending()
  await click('#dess-play')
  p = await pending()
  let page = await read()
  check(
    p.timers.length === clean.timers.length && p.frames === clean.frames,
    `Play at the end on a clean page schedules no timer and no frame (${p.timers.length}, ${p.frames})`
  )
  check(
    page.play.text === 'Play' && page.play.pressed === 'false' && page.state.startsWith('Paused, end of scenario'),
    `and the button stays Play and unpressed (${page.state})`
  )
  check(
    page.live === 'End of scenario' && page.iSnapshot === '12 of 12',
    `and says End of scenario, with the cursor where it was (${page.live})`
  )
  // A dirty page at the end: a repair that fails leaves it dirty, and Play is then the request that repairs it.
  await evaluate('__faults.arm(1)')
  await click('#dess-repair')
  await evaluate('__faults.disarm()')
  page = await read()
  check(page.display === 'Needs repair', `a failed repair leaves the page marked dirty (${page.display})`)
  await click('#dess-play')
  p = await pending()
  page = await read()
  check(
    p.timers.length === clean.timers.length && page.display === 'Up to date',
    `Play at the end of a dirty page repairs it first and schedules nothing (${page.display})`
  )
  check(
    page.play.pressed === 'false' && matchesOracle(page, 12).length === 0,
    'and the toggle is unpressed and the picture is snapshot 12'
  )
  await hold(false)
  // One real interval, at 1 Hz, on the browser's own timer.
  await click('#dess-restart')
  await setSpeed(1000)
  const started = Date.now()
  await click('#dess-play')
  const advanced = await until(`document.getElementById('dess-i-snapshot').textContent.startsWith('1 ')`, 4000)
  const elapsed = Date.now() - started
  await click('#dess-play')
  check(advanced, `the real timer, once at 1 Hz, committed a slice (${elapsed} ms, which is told and not judged)`)
  check((await read()).play.pressed === 'false', 'and Pause stopped it')
})

await test('G07', 'a callback that was taken back cannot commit, and the last request wins', async () => {
  await fresh()
  await hold(true)
  // Pause takes the timer back, and a timer that is not there cannot be fired.
  await setSpeed(500)
  await click('#dess-play')
  let p = await pending()
  const [timer] = p.timers
  await click('#dess-play')
  const late = await evaluate(`__timers.fire(${timer.id})`)
  p = await pending()
  let page = await read()
  check(
    late.ran === false && p.timers.length === 0 && page.iSnapshot === '0 of 12',
    `a timer that Pause took back is gone, and commits nothing (${JSON.stringify(late)}, ${page.iSnapshot})`
  )
  // A seek that Next replaced leaves no frame.
  await focus('#dess-seek')
  await evaluate(
    `(() => { const s = document.getElementById('dess-seek'); s.value = '8'; s.dispatchEvent(new Event('input', { bubbles: true })) })()`
  )
  check((await pending()).frames === 1, 'a seek asks for one frame')
  await click('#dess-next')
  p = await pending()
  page = await read()
  check(
    p.frames === 0 && page.iSnapshot === '1 of 12' && page.seek.text === 'Committed 1',
    `Next takes the seek's frame back and steps from where the page was (${p.frames} held, ${page.iSnapshot})`
  )
  // Of three seeks, the last wins, and only its frame is left.
  await evaluate(
    `(() => { const s = document.getElementById('dess-seek'); for (const v of ['4', '9', '6']) { s.value = v; s.dispatchEvent(new Event('input', { bubbles: true })) } })()`
  )
  p = await pending()
  check(p.frames === 1, `three seeks leave one frame waiting (${p.frames})`)
  await evaluate('__frames.release()')
  await hold(false)
  await until(`document.getElementById('dess-seek-text').textContent === 'Committed 6'`)
  page = await read()
  check(
    page.iSnapshot === '6 of 12' && matchesOracle(page, 6).length === 0,
    `the last of them is the one that commits (${page.iSnapshot})`
  )
  // Play replaces a seek that is waiting.
  await hold(true)
  await evaluate(
    `(() => { const s = document.getElementById('dess-seek'); s.value = '11'; s.dispatchEvent(new Event('input', { bubbles: true })) })()`
  )
  await click('#dess-play')
  p = await pending()
  page = await read()
  check(
    p.frames === 0 && p.timers.length === 1 && page.iSnapshot === '6 of 12',
    `Play takes a waiting seek back and asks for a timer (${p.frames} frames, ${p.timers.length} timers, ${page.iSnapshot})`
  )
  await evaluate(`__timers.fire(${p.timers[0].id})`)
  check((await slice()) === 7, 'and the timer commits one slice from where the page was')
  await click('#dess-play')
  check((await pending()).timers.length === 0, 'and Pause leaves nothing waiting')
  await hold(false)
  const errors = exceptions.filter(e => !/favicon/.test(e))
  check(errors.length === 0, `and the page raised no exception (${errors.length})`)
  current.deferred =
    'A callback that the browser still calls after it was taken back is shown by the native controller tests, with a scheduler that keeps cancelled tasks. The page scheduler frees a taken-back closure, so the real page cannot call one. Pause leaves a seek that is waiting alone is also native: the page has no way to pause while a seek is waiting, since a seek stops playback'
  console.log(`  note ${current.deferred}`)
})

await test('G08', 'Repair redraws the committed snapshot and changes nothing else', async () => {
  await fresh()
  for (let i = 0; i < 5; i++) await click('#dess-next')
  const before = await read()
  await click('#dess-repair')
  let page = await read()
  check(
    same(page.slots, before.slots) && same(page.data, before.data),
    'on a clean page the picture and the data cells are what they were'
  )
  check(
    page.iSnapshot === '5 of 12' && page.selected === before.selected && page.transform === before.transform,
    'and the cursor, the selection and the view are unmoved'
  )
  check(page.play.text === 'Play' && page.play.pressed === 'false', 'and Play is not started')
  // Damage to what a person sees: an overlay slot and the Play control.
  await evaluate(`(() => {
    document.querySelector('#dess-stage svg [data-ss-key="arrived"] text').textContent = '999'
    const play = document.getElementById('dess-play')
    play.textContent = 'Pause'
    play.setAttribute('aria-pressed', 'true')
  })()`)
  await click('#dess-repair')
  page = await read()
  check(matchesOracle(page, 5).length === 0, `a repair restores the overlay (${matchesOracle(page, 5).join('; ')})`)
  check(page.play.text === 'Play' && page.play.pressed === 'false', 'and the Play control and its ARIA')
  check(page.iSnapshot === '5 of 12', 'without moving the cursor')
  // A draw that fails part way, at each of several of its writes, leaves the page torn and dirty, and a repair makes it the committed snapshot again.
  let fired = 0
  for (const at of [1, 2, 3, 5, 8]) {
    await evaluate(`__faults.arm(${at}, 100000)`)
    await click('#dess-next')
    const report = await evaluate('__faults.disarm()')
    if (report.fired === null) continue
    fired++
    page = await read()
    check(
      page.display === 'Needs repair' && page.iSnapshot === '5 of 12',
      `a failure at write ${at} is not a commit: still 5 of 12, ${page.display} (${page.iSnapshot})`
    )
    check(page.live === 'The display could not be updated. Use Repair display.', `and says so, in words (${page.live})`)
    if (fired === 1)
      check(
        !page.errorHidden && page.errorText.length > 0,
        `and the first failure of an episode is written in the error area (${page.errorText.slice(0, 50)})`
      )
    await click('#dess-repair')
    page = await read()
    check(
      page.display === 'Up to date' &&
        page.errorHidden &&
        page.iSnapshot === '5 of 12' &&
        matchesOracle(page, 5).length === 0,
      `a repair makes it snapshot 5 again, with the data cells and overlay of the independent engine ${matchesOracle(page, 5).join('; ')}`
    )
  }
  check(fired === 5, `every one of the five writes was reached inside the draw (${fired})`)
  // A write of the Play control that fails while playback starts leaves playback going under a button that says Play. The controller is not dirty, and the
  // page still has to say that it needs repair, and a repair has to put the button right without stopping playback.
  await click('#dess-restart')
  await hold(true)
  await evaluate('__faults.arm(1)')
  await click('#dess-play')
  await evaluate('__faults.disarm()')
  page = await read()
  check(
    page.play.text === 'Play' && page.play.pressed === 'false',
    `the failed write left the button saying Play (${page.play.text}, ${page.play.pressed})`
  )
  check(
    (await pending()).timers.length === 1 && page.state.startsWith('Playing'),
    `while playback is going (${page.state})`
  )
  check(page.display === 'Needs repair', `and the inspector does not say Up to date (${page.display})`)
  await click('#dess-repair')
  page = await read()
  check(
    page.play.text === 'Pause' && page.play.pressed === 'true' && page.display === 'Up to date',
    `a repair puts the button right (${page.play.text}, ${page.play.pressed}, ${page.display})`
  )
  check((await pending()).timers.length === 1 && page.state.startsWith('Playing'), 'without stopping playback')
  await click('#dess-play')
  await hold(false)
  await click('#dess-restart')
  for (let i = 0; i < 5; i++) await click('#dess-next')
  // A write of a boundary button that fails leaves Next saying that it can still go on, after the step to the last snapshot. The page says that it needs
  // repair, though the slice committed, and a repair writes the attribute again. The injector fails the nth call of any kind, and a refresh writes the
  // attribute twice, so the failure is put on that one attribute alone, in the page, and taken off again.
  await evaluate(`(() => {
    const set = Element.prototype.setAttribute
    window.__restoreSet = () => { Element.prototype.setAttribute = set }
    Element.prototype.setAttribute = function (name, value) {
      if (window.__failBoundary && this.id === 'dess-next' && name === 'aria-disabled') throw new DOMException('an injected failure of the boundary write', 'InvalidStateError')
      return set.call(this, name, value)
    }
    return 0
  })()`)
  await evaluate(
    `(() => { const s = document.getElementById('dess-seek'); s.value = '11'; s.dispatchEvent(new Event('input', { bubbles: true })) })()`
  )
  await until(`document.getElementById('dess-seek-text').textContent === 'Committed 11'`)
  await evaluate('window.__failBoundary = true')
  await click('#dess-next')
  await evaluate('window.__failBoundary = false')
  page = await read()
  check(
    page.iSnapshot === '12 of 12' && page.next.disabled !== 'true',
    `the slice committed, and Next was not marked as the last (${page.iSnapshot}, ${page.next.disabled})`
  )
  check(page.display === 'Needs repair', `and the page says that it needs repair (${page.display})`)
  await click('#dess-repair')
  page = await read()
  check(
    page.next.disabled === 'true' && page.display === 'Up to date' && page.iSnapshot === '12 of 12',
    `a repair marks Next again and clears the status (${page.next.disabled}, ${page.display})`
  )
  await evaluate('__restoreSet()')
  await click('#dess-restart')
  for (let i = 0; i < 5; i++) await click('#dess-next')
  // A failure that the controller recovers from by itself is still said, and the cursor did not move.
  await evaluate('__faults.arm(1)')
  await click('#dess-next')
  await evaluate('__faults.disarm()')
  page = await read()
  check(
    page.iSnapshot === '5 of 12' && page.live === 'The display could not be updated. Use Repair display.',
    `a failure that was recovered from is still said, and is not a commit (${page.iSnapshot}, ${page.live})`
  )
  check(matchesOracle(page, 5).length === 0, 'and the page that the recovery left is snapshot 5')
  await click('#dess-repair')
  await click('#dess-next')
  check(matchesOracle(await read(), 6).length === 0, 'and after a repair the next slice commits')
})

await test(
  'G09',
  'selecting and clearing a job agree across the scene, the inspector and the accessible label',
  async () => {
    await fresh()
    for (let i = 0; i < 3; i++) await click('#dess-next')
    const before = await read()
    const options = await evaluate(
      `[...document.getElementById('dess-job').options].map(o => ({ value: o.value, label: o.textContent }))`
    )
    check(
      options[0].value === '' && options[0].label === 'None' && options.length > 1,
      `the list offers None and the jobs that can be seen (${options.length - 1})`
    )
    const queued = options.find(o => o.label.includes('queued'))
    await focus('#dess-job')
    await evaluate(
      `(() => { const s = document.getElementById('dess-job'); s.value = '${queued.value}'; s.dispatchEvent(new Event('change', { bubbles: true })) })()`
    )
    let page = await read()
    const key = queued.label.split(':')[0]
    const outlined = Object.entries(page.slots)
      .filter(([, s]) => s.cls.split(' ').includes('ss-selected'))
      .map(([k]) => k)
    check(
      outlined.length === 1 && page.slots[outlined[0]].text.startsWith(key),
      `exactly one slot is outlined, and it holds ${key} (${outlined})`
    )
    check(
      page.selected.startsWith(key) && page.selected.includes('queued'),
      `the inspector names it and its state (${page.selected})`
    )
    check(
      page.slots.selected.label.includes(key) && page.slots.selected.text.includes(key),
      `and so does the selected-job line (${page.slots.selected.label})`
    )
    check(
      page.slots[outlined[0]].label.includes(key),
      `and the outlined slot's accessible name (${page.slots[outlined[0]].label})`
    )
    check(
      page.iSnapshot === before.iSnapshot && same(page.data, before.data) && page.transform === before.transform,
      'the cursor, the data cells and the view are unchanged'
    )
    await focus('#dess-job')
    await evaluate(
      `(() => { const s = document.getElementById('dess-job'); s.value = ''; s.dispatchEvent(new Event('change', { bubbles: true })) })()`
    )
    page = await read()
    check(
      Object.values(page.slots).every(s => !s.cls.split(' ').includes('ss-selected')),
      'clearing the selection takes every outline off'
    )
    check(
      page.selected === 'none' && page.slots.selected.label === 'No job is selected',
      `and the inspector and the line say none (${page.selected})`
    )
    check(
      page.iSnapshot === before.iSnapshot && page.transform === before.transform,
      'with the cursor and the view still unmoved'
    )
    // A selection survives a step, and follows the job.
    await focus('#dess-job')
    await evaluate(
      `(() => { const s = document.getElementById('dess-job'); s.value = '${queued.value}'; s.dispatchEvent(new Event('change', { bubbles: true })) })()`
    )
    await click('#dess-next')
    page = await read()
    check(
      page.selected.startsWith(key) &&
        Object.values(page.slots).filter(s => s.cls.split(' ').includes('ss-selected')).length <= 2,
      `the selection is still the same job after a step (${page.selected})`
    )
  }
)

await test(
  'G10',
  'native keyboard controls, names, focus and the boundary rule pass the accessibility tree',
  async () => {
    await fresh()
    const ax = await accessibility()
    const named = (role, name) => ax.find(n => n.role?.value === role && n.name?.value === name)
    const property = (node, name) => node?.properties?.find(p => p.name === name)?.value?.value
    for (const [role, name] of [
      ['button', 'Previous'],
      ['button', 'Next'],
      ['button', 'Restart'],
      ['button', 'Play'],
      ['button', 'Repair display'],
      ['slider', 'Snapshot']
    ])
      check(named(role, name), `the accessibility tree has a ${role} named ${name}`)
    check(
      ax.some(n => n.role?.value === 'combobox' && n.name?.value === 'Speed'),
      'and a combobox named Speed'
    )
    check(
      ax.some(n => n.role?.value === 'combobox' && n.name?.value === 'Inspect a job'),
      'and one named Inspect a job'
    )
    check(property(named('button', 'Play'), 'pressed') === 'false', 'Play is a toggle that is not pressed')
    const previous = named('button', 'Previous')
    check(property(previous, 'disabled') === true, 'Previous is exposed as disabled at snapshot 0, by aria-disabled')
    const page = await read()
    check(
      !page.prev.hasDisabled && !page.next.hasDisabled,
      'and neither boundary button has the HTML disabled attribute'
    )
    check(
      previous?.description?.value === 'Already at first snapshot',
      `and its reason is read with it (${previous?.description?.value})`
    )
    check(
      ax.filter(n => n.properties?.some(p => p.name === 'live') || n.role?.value === 'status').length >= 1,
      'and there is a polite live region'
    )
    // The click still reaches the controller.
    await click('#dess-prev')
    check(
      (await read()).live === 'Already at the first snapshot',
      `a click on the dimmed Previous still reaches the controller, which says so (${(await read()).live})`
    )
    // Tab order and keys, on native controls.
    await focus('#dess-prev')
    const order = []
    for (let i = 0; i < 6; i++) {
      await key('Tab')
      order.push(await evaluate('document.activeElement.id'))
    }
    check(
      same(order, ['dess-next', 'dess-restart', 'dess-play', 'dess-speed', 'dess-repair', 'dess-seek']),
      `Tab visits the controls in order (${order})`
    )
    await focus('#dess-next')
    await key('Enter')
    check(
      (await slice()) === 1 && (await evaluate('document.activeElement.id')) === 'dess-next',
      'Enter on Next steps once, and the focus stays on Next'
    )
    await key('Space')
    check(
      (await slice()) === 2 && (await evaluate('document.activeElement.id')) === 'dess-next',
      'and Space does the same'
    )
    await focus('#dess-seek')
    await key('ArrowRight')
    await until(`document.getElementById('dess-seek-text').textContent === 'Committed 3'`)
    check(
      (await slice()) === 3 && (await evaluate('document.activeElement.id')) === 'dess-seek',
      'an arrow key on the range is one seek, and keeps the focus'
    )
    await hold(true)
    await focus('#dess-play')
    await key('Space')
    const playing = await read()
    check(
      playing.play.pressed === 'true' && (await evaluate('document.activeElement.id')) === 'dess-play',
      'Space on Play starts playback and keeps the focus'
    )
    await key('Space')
    check((await read()).play.pressed === 'false', 'and Space again pauses it')
    await hold(false)
    // No wrapper-level or document-wide shortcut (D4).
    await evaluate('document.activeElement.blur()')
    const at = await slice()
    for (const name of ['KeyN', 'Space', 'ArrowRight', 'Enter']) await key(name)
    await pause(100)
    check((await slice()) === at, 'keys pressed with no control focused do nothing: there is no shortcut')
    const errors = exceptions.filter(e => !/favicon/.test(e))
    check(
      errors.length === 0,
      `and the page raised no exception or console error (${errors.length}: ${errors.join(' | ').slice(0, 300)})`
    )
  }
)

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
await test(
  'G05W6',
  'five arrow keys on the 5,464 snapshots of the advanced scenario are five requests, and the last one wins',
  async () => {
    await fresh()
    await chooseScenario('W6')
    const state = await scenarioState()
    check(
      state.max === '5464' && state.snapshot === '0 of 5464',
      `the advanced scenario runs to the preflight's 5,464 (${state.max}, ${state.snapshot})`
    )
    check(
      matchesOracle(await read(), 0, SCENARIOS.W6).length === 0,
      "and starts at the independent engine's snapshot 0"
    )
    await focus('#dess-seek')
    await evaluate('__seen.frames = 0')
    await hold(true)
    for (let i = 0; i < 5; i++) await key('ArrowRight')
    const during = await read()
    const held = await pending()
    check(
      during.seek.value === '5' && during.seek.valuetext === 'Snapshot 5 of 5464',
      `the range asks for 5 and says ${during.seek.valuetext}`
    )
    check(
      during.iSnapshot === '0 of 5464' && during.seek.text === 'Requested 5, committed 0',
      `while snapshot 0 is still the committed one (${during.seek.text})`
    )
    check(
      (await evaluate('__seen.frames')) === 5 && held.frames === 1,
      `each press asked for a frame, and only the last is waiting (${held.frames})`
    )
    await evaluate('__frames.release()')
    await hold(false)
    await until(`document.getElementById('dess-seek-text').textContent === 'Committed 5'`)
    check(
      matchesOracle(await read(), 5, SCENARIOS.W6).length === 0,
      "the last request committed snapshot 5, which is the independent engine's"
    )
    // A long jump, to the end, and one step beyond it.
    await evaluate(
      `(() => { const s = document.getElementById('dess-seek'); s.value = '5464'; s.dispatchEvent(new Event('input', { bubbles: true })) })()`
    )
    const arrived = await until(`document.getElementById('dess-seek-text').textContent === 'Committed 5464'`, 20000)
    check(arrived, 'a seek to the last snapshot arrives')
    const end = await read()
    const problems = matchesOracle(end, 5464, SCENARIOS.W6)
    check(problems.length === 0, `and it is the independent engine's last snapshot ${problems.join('; ')}`)
    await click('#dess-next')
    check(
      (await read()).live === 'End of scenario' && (await slice()) === 5464,
      'and Next there is the end of the scenario, with nothing committed'
    )
  }
)

await test(
  'G11',
  'routing away with a held Play timer stops it through the one gallery observer, and routing back stays paused',
  async () => {
    await fresh()
    // The first visit to the other panel builds it, and its listeners are not this panel's.
    await routeAway()
    await until(`document.getElementById('panel-petri-net').dataset.panelState === 'ready'`)
    await routeBack()
    await hold(true)
    await setSpeed(1000)
    const baseline = await census()
    await click('#dess-play')
    let p = await pending()
    const [timer] = p.timers
    check(p.timers.length === 1 && timer.delay === 1000, `Play holds one 1 Hz timer (${JSON.stringify(p.timers)})`)
    await routeAway()
    p = await pending()
    let page = await read()
    check(p.timers.length === 0, 'the observer told the current controller, which took the timer back')
    const late = await evaluate(`__timers.fire(${timer.id})`)
    check(
      late.ran === false && (await slice()) === 0,
      `a timer that was taken back is gone and commits nothing (${JSON.stringify(late)})`
    )
    check(
      page.play.pressed === 'false' && page.state === 'Paused / hidden',
      `the hidden panel says that it is paused and hidden (${page.state})`
    )
    check(
      !page.notice.hidden && page.notice.text === 'Paused while hidden. Press Play to resume.',
      `and says why (${page.notice.text})`
    )
    check(page.live === 'Paused while hidden. Press Play to resume.', 'and says it once, in the live region')
    await routeBack()
    page = await read()
    p = await pending()
    check(
      page.play.pressed === 'false' && page.state === 'Paused / visible' && p.timers.length === 0,
      `routing back does not resume (${page.state}, ${p.timers.length} timers)`
    )
    check(
      !page.notice.hidden && page.iSnapshot === '0 of 12',
      'and the reason is still shown, at the snapshot where it stopped'
    )
    const after = await census()
    check(
      after.observers === baseline.observers &&
        after.listeners === baseline.listeners &&
        same(after.hosts, baseline.hosts),
      `with the same observer, listeners and host (${after.observers}, ${after.listeners})`
    )
    await click('#dess-play')
    p = await pending()
    page = await read()
    check(
      page.notice.hidden && p.timers.length === 1 && page.play.pressed === 'true',
      'a deliberate Play clears the reason and starts one timer'
    )
    await click('#dess-play')
    page = await read()
    check(page.notice.hidden && page.play.pressed === 'false', 'and a Pause after it does not bring the reason back')
    await hold(false)
  }
)

await test(
  'G12',
  'a hidden page stops Play and stays stopped, and a hide and a show in one batch may continue',
  async () => {
    await fresh()
    await hold(true)
    await setSpeed(1000)
    const setHidden = hidden =>
      evaluate(
        `(() => { Object.defineProperty(document, 'hidden', { configurable: true, get: () => ${hidden} }); document.dispatchEvent(new Event('visibilitychange')); return 0 })()`
      )
    await click('#dess-play')
    await setHidden(true)
    let p = await pending()
    let page = await read()
    check(
      p.timers.length === 0 && page.play.pressed === 'false',
      `a hidden transition of the document stops Play (${p.timers.length} timers)`
    )
    check(!page.notice.hidden, 'and says that it paused')
    await setHidden(false)
    p = await pending()
    check(p.timers.length === 0 && (await read()).play.pressed === 'false', 'a later visible event does not resume it')
    // A hide and a show in one task are two events, and the first one stops Play.
    await click('#dess-play')
    await evaluate(
      `(() => { Object.defineProperty(document, 'hidden', { configurable: true, get: () => true }); document.dispatchEvent(new Event('visibilitychange')); Object.defineProperty(document, 'hidden', { configurable: true, get: () => false }); document.dispatchEvent(new Event('visibilitychange')) })()`
    )
    p = await pending()
    check(
      p.timers.length === 0 && (await read()).play.pressed === 'false',
      'a hide and a show of the document in one task still stop it'
    )
    // The gallery's own hide and show of the section in one task are one batch for the observer, which sees only the panel that is shown.
    await click('#dess-play')
    const before = await census()
    const start = await slice()
    await evaluate(
      `(() => { const s = document.getElementById('${PANEL}'); s.classList.remove('active'); s.classList.add('active') })()`
    )
    await pause(50)
    p = await pending()
    page = await read()
    check(
      p.timers.length <= 1,
      `after a hide and a show in one batch at most one timer is waiting (${p.timers.length})`
    )
    if (p.timers.length === 1) {
      await evaluate(`__timers.fire(${p.timers[0].id})`)
      const next = await pending()
      check(
        (await slice()) === start + 1 && next.timers.length === 1,
        `and one firing is one tick, with the next one asked for once (${await slice()}, ${next.timers.length})`
      )
    } else check(page.play.pressed === 'false', 'or it stopped, and says so')
    const after = await census()
    check(
      after.observers === before.observers && after.listeners === before.listeners && same(after.hosts, before.hosts),
      'with no new observer, listener or host'
    )
    await click('#dess-play')
    await hold(false)
    const errors = exceptions.filter(e => !/favicon/.test(e))
    check(errors.length === 0, `and the page raised no exception (${errors.length})`)
  }
)

await test(
  'G13',
  'hiding the panel during a seek that is waiting leaves no frame, and a later seek succeeds',
  async () => {
    await fresh()
    await hold(true)
    await focus('#dess-seek')
    await evaluate(
      `(() => { const s = document.getElementById('dess-seek'); s.value = '8'; s.dispatchEvent(new Event('input', { bubbles: true })) })()`
    )
    let p = await pending()
    check(p.frames === 1, `a seek is waiting for its frame (${p.frames})`)
    await routeAway()
    p = await pending()
    let page = await read()
    check(
      p.frames === 0 && page.iSnapshot === '0 of 12',
      `hiding the panel took the frame back, and nothing was committed (${p.frames}, ${page.iSnapshot})`
    )
    await routeBack()
    page = await read()
    check(
      page.seek.text === 'Committed 0' && page.iSnapshot === '0 of 12',
      `routing back shows the committed snapshot and no request (${page.seek.text})`
    )
    await focus('#dess-seek')
    await evaluate(
      `(() => { const s = document.getElementById('dess-seek'); s.value = '6'; s.dispatchEvent(new Event('input', { bubbles: true })) })()`
    )
    await evaluate('__frames.release()')
    await hold(false)
    await until(`document.getElementById('dess-seek-text').textContent === 'Committed 6'`)
    page = await read()
    check(
      page.iSnapshot === '6 of 12' && matchesOracle(page, 6).length === 0,
      "a later seek commits the independent engine's snapshot 6"
    )
  }
)

await test(
  'G14',
  'a host detached before or in the middle of a draw commits nothing, and a full repair follows',
  async () => {
    await fresh()
    for (let i = 0; i < 3; i++) await click('#dess-next')
    // In the middle of a draw: the injector detaches the host just after its second write.
    await evaluate(`__faults.armDetach({ svgId: 'dess-svg-0', method: 'setAttribute', nth: 2 })`)
    await click('#dess-next')
    const report = await evaluate('__faults.disarmDetach()')
    check(
      report?.fired === true && report.firings === 1 && report.detached === true,
      `the injection fired once, at ${report?.site}, and the host left (${JSON.stringify(report)})`
    )
    let page = await status()
    check(page.snapshot === '3 of 12', `no slice was committed (${page.snapshot})`)
    check(
      page.state === 'Paused / detached' && page.display === 'Needs repair',
      `and the page says that it is detached and needs repair (${page.state}, ${page.display})`
    )
    check(
      page.live === 'The panel is hidden or detached, so nothing changed.',
      `in words, as unavailable and not as a failed write (${page.live})`
    )
    check(await evaluate('__faults.reattach()'), 'the host was put back')
    await click('#dess-repair')
    page = await read()
    check(
      page.display === 'Up to date' && matchesOracle(page, 3).length === 0,
      `a full repair makes it the independent engine's snapshot 3 (${matchesOracle(page, 3).join('; ')})`
    )
    await click('#dess-next')
    check(matchesOracle(await read(), 4).length === 0, 'and the next slice then commits')
    // Before a draw: the host is taken out of the document by the test.
    await evaluate(
      `(() => { const svg = document.getElementById('dess-svg-0'); window.__svgEl = svg; window.__parent = svg.parentNode; window.__next = svg.nextSibling; svg.remove() })()`
    )
    // The controls are in the scene, and went with it, so the request is made by the click handler directly.
    await evaluate('(window.__byId["dess-next"].click(), 0)')
    page = await status()
    check(
      page.snapshot === '4 of 12' && page.live === 'The panel is hidden or detached, so nothing changed.',
      `a request on a host that is not in the document commits nothing (${page.snapshot})`
    )
    await evaluate('(window.__parent.insertBefore(window.__svgEl, window.__next), 0)')
    await click('#dess-repair')
    page = await read()
    check(
      page.display === 'Up to date' && matchesOracle(page, 4).length === 0,
      'put back, a repair makes it snapshot 4 again'
    )
    current.checks.push({
      ok: true,
      text: 'a native fake host covers the same boundaries without a browser, in the coordinator tests'
    })
  }
)

await test(
  'G15',
  'a switch to a scenario of the same size publishes snapshot 0 whole, and a failure leaves the old one as it was',
  async () => {
    await fresh()
    await chooseScenario('scaled')
    await evaluate(`(window.__svg = document.querySelector('#dess-stage svg'), 0)`)
    for (let i = 0; i < 3; i++) await click('#dess-next')
    const options = await evaluate(
      `[...document.getElementById('dess-job').options].map(o => ({ value: o.value, label: o.textContent }))`
    )
    await evaluate(
      `(() => { const s = document.getElementById('dess-job'); s.value = '${options[1].value}'; s.dispatchEvent(new Event('change', { bubbles: true })) })()`
    )
    const before = await read()
    check(
      before.selected !== 'none' && before.iSnapshot === '3 of 487',
      `a scenario of three servers, at snapshot 3, with a job selected (${before.iSnapshot}, ${before.selected})`
    )
    // A failure at several writes of the draw: the prior scenario, selection and picture are kept, and a repair restores the picture exactly.
    let failures = 0
    for (const at of [1, 3, 8]) {
      await evaluate(`__faults.arm(${at}, 100000)`)
      await chooseScenario('W6')
      const report = await evaluate('__faults.disarm()')
      const state = await scenarioState()
      const same_svg = await evaluate('document.querySelector("#dess-stage svg") === window.__svg')
      if (report.fired === null) {
        check(false, `the injection at write ${at} was not reached`)
        continue
      }
      failures++
      check(
        state.errorShown && state.error.startsWith('The scenario could not be changed'),
        `a failure at write ${at} is said, and says that the previous scenario is still shown (${state.error.slice(0, 70)})`
      )
      check(
        state.select === 'scaled' && state.max === '487' && same_svg,
        `the list, the range and the scene are the old scenario's (${state.select}, ${state.max}, same scene ${same_svg})`
      )
      await click('#dess-repair')
      const after = await read()
      check(
        same(after.slots, before.slots) &&
          same(after.data, before.data) &&
          after.selected === before.selected &&
          after.iSnapshot === before.iSnapshot,
        `a repair restores the same selection, snapshot and projection (${after.iSnapshot})`
      )
    }
    check(failures === 3, `all three injections fired (${failures})`)
    // The same switch, with nothing wrong: atomic, in place, at snapshot 0.
    await chooseScenario('W6')
    const state = await scenarioState()
    const page = await read()
    check(
      await evaluate('document.querySelector("#dess-stage svg") === window.__svg'),
      'a scenario of the same size is shown in the same scene, which is not replaced'
    )
    check(
      state.select === 'W6' && state.max === '5464' && !state.errorShown && state.hosts.length === 1,
      `the list and the range are the new scenario's (${state.select}, ${state.max})`
    )
    check(
      page.selected === 'none' && matchesOracle(page, 0, SCENARIOS.W6).length === 0,
      'at snapshot 0 with nothing selected, as the independent engine has it'
    )
  }
)

await test(
  'G16',
  'the sizes 2, 1, 3 and 2 each keep one scene, and a failure at any write leaves the old one',
  async () => {
    await fresh()
    // A person's zoom is the scene's own state, and goes with the scene that replaces it.
    const { x, y } = await centre('#dess-stage svg')
    for (let i = 0; i < 3; i++) await wheel(chrome, x, y, -200, { ctrl: true })
    const zoomed = (await scenarioState()).transform
  // The scene is brought into view, and where it is and how far the page is scrolled are kept, since neither may move while another scene is staged.
  await evaluate(`document.querySelector('#dess-stage').scrollIntoView({ block: 'center' })`)
  const layout0 = await layoutOf()
  check(layout0.scrolls.some(([x, y]) => y > 0), `the page is scrolled, so that a scroll that moved would show (${JSON.stringify(layout0.scrolls.filter(([x, y]) => x || y))})`)
    check(
      zoomed !== null && !/scale\(1\)|matrix\(1,0,0,1,0,0\)/.test(zoomed),
      `the scene was zoomed by real wheel input (${zoomed})`
    )
    for (const id of ['E3-NOOP', 'scaled', 'E0']) {
      await evaluate(`document.querySelector('#dess-stage [data-ss-key="S0"]').focus()`)
      const focused = await evaluate('document.activeElement.getAttribute("data-ss-key")')
      await chooseScenario(id)
      const state = await scenarioState()
      const exposed = await scenes()
      check(
        state.svgs === 1 && state.hosts.length === 1 && !state.hosts[0].staged && exposed === 1,
        `${id}: one scene in the document and one in the accessibility tree (${state.svgs}, ${state.hosts.length}, ${exposed})`
      )
      check(
        state.servers === SCENARIOS[id].servers &&
          state.snapshot === `0 of ${SCENARIOS[id].slices}` &&
          state.max === String(SCENARIOS[id].slices),
        `${id}: ${state.servers} servers, ${state.snapshot}`
      )
      check(state.transform === zoomed, `${id}: the zoom went with it (${state.transform})`)
    check(unmoved(layout0, await layoutOf()), `${id}: the scene did not move and the page did not scroll`)
      check(
        focused === 'S0' && (await evaluate('document.activeElement.id')) === 'dess-scenario',
        `${id}: a focus that was inside the old scene is on the scenario list`
      )
      check(
        matchesOracle(await read(), 0, SCENARIOS[id]).length === 0,
        `${id}: and its first draw is the independent engine's snapshot 0`
      )
    }
    // Failures, at writes through the whole of a switch from 2 servers to 1. The count of writes comes from a switch that succeeds.
    await evaluate('__faults.record(true)')
    await chooseScenario('E3-NOOP')
    const writes = (await evaluate('__faults.take()')).length
    await evaluate('__faults.record(false)')
    await chooseScenario('E0')
    check(writes > 50, `a switch of size makes ${writes} counted DOM writes`)
    const points = [
      ...new Set([
        1,
        2,
        3,
        5,
        10,
        20,
        40,
        Math.floor(writes / 2),
        writes - 8,
        writes - 6,
        writes - 4,
        writes - 3,
        writes - 2,
        writes - 1,
        writes
      ])
    ].filter(n => n >= 1 && n <= writes)
    // The writes of the build come first, then those of the first draw, and then the handoff, so a stride through the middle reaches the draw as well.
    for (let n = 60; n < writes - 8; n += 37) if (!points.includes(n)) points.push(n)
    const stages = new Set()
    let rolled = 0
    let committed = 0
    for (const at of points) {
      const before = await scenarioState()
      // Where the scene is now, after the clicks of the last point, which scroll what they click into view.
      const layoutBefore = await layoutOf()
      await evaluate(`__faults.arm(${at}, 3)`)
      await chooseScenario('E3-NOOP')
      await evaluate('__faults.disarm()')
      const state = await scenarioState()
      if (!state.errorShown) {
        // The failure fell on a write that nothing depends on, so the switch committed. Back to where the next point starts.
        committed++
        await chooseScenario('E0')
        continue
      }
      rolled++
      {
        const layoutAfter = await layoutOf()
        if (!unmoved(layoutBefore, layoutAfter)) check(false, `a failure at write ${at} moved the scene or scrolled the page: ${JSON.stringify(layoutBefore)} then ${JSON.stringify(layoutAfter)}`)
      }
      stages.add(/(Build|Validate|Publish) failed/.exec(state.error)?.[1] ?? 'other')
      const exposed = await scenes()
      const ok =
        state.select === 'E0' &&
        state.svgs === 1 &&
        state.hosts.length === 1 &&
        state.hosts[0].id === before.hosts[0].id &&
        !state.hosts[0].staged &&
        exposed === 1 &&
        state.snapshot === before.snapshot &&
        state.max === before.max &&
        state.transform === before.transform
      if (!ok)
        check(
          false,
          `a failure at write ${at} did not leave the old scene as it was: ${JSON.stringify({ before, state, exposed })}`
        )
      await click('#dess-repair')
      const page = await read()
      if (page.display !== 'Up to date' || matchesOracle(page, 0).length !== 0)
        check(
          false,
          `after a failure at write ${at} a repair did not give snapshot 0 (${page.display}, ${matchesOracle(page, 0).join('; ')})`
        )
    }
    check(
      rolled >= 6,
      `${rolled} of the ${points.length} injections rolled back, and each left the old scene, one scene in the accessibility tree, its zoom and a repaired picture (${committed} fell on a write that nothing depends on)`
    )
    check(
      stages.has('Build') && stages.has('Validate') && stages.has('Publish'),
      `and the failures were at the stages ${[...stages].join(', ')}`
    )
    await chooseScenario('E3-NOOP')
    const last = await scenarioState()
    check(
      last.select === 'E3-NOOP' && last.svgs === 1 && (await scenes()) === 1 && !last.errorShown,
      'and an ordinary switch afterwards still works'
    )
  }
)

await test('G16F', 'a switch that fails at the handoff gives the focus back to where it was', async () => {
  await fresh()
  // The page makes the candidate's `unstage` fail: its removal of the staging style throws, as a write that failed part way through a handoff would.
  const failUnstage = shown =>
    evaluate(`(() => {
      const remove = Element.prototype.removeAttribute
      window.__restoreRemove = () => { Element.prototype.removeAttribute = remove }
      Element.prototype.removeAttribute = function (name) {
        if (name === 'style' && /^dess-host-/.test(this.id) && this.id !== '${shown}') throw new DOMException('an injected failure of the handoff', 'InvalidStateError')
        return remove.call(this, name)
      }
      return 0
    })()`)
  const cases = [
    [
      'the scenario list, as when a person used the keyboard',
      `(document.getElementById('dess-scenario').focus(), document.activeElement.id)`,
      'document.activeElement.id',
      'dess-scenario'
    ],
    [
      'the Next button',
      `(document.getElementById('dess-next').focus(), document.activeElement.id)`,
      'document.activeElement.id',
      'dess-next'
    ],
    [
      'a node inside the old scene',
      `(document.querySelector('#dess-stage [data-ss-key="S0"]').focus(), document.activeElement.getAttribute('data-ss-key'))`,
      'document.activeElement.getAttribute("data-ss-key")',
      'S0'
    ]
  ]
  for (const [name, focusIt, read, expected] of cases) {
    const before = await scenarioState()
    const hostId = before.hosts[0].id
    await evaluate(`document.querySelector('#dess-stage').scrollIntoView({ block: 'center' })`)
    check((await evaluate(focusIt)) === expected, `the focus is on ${name}`)
    // Taking the focus there may have scrolled it into view, and that is the test's own doing, so the layout is kept after it.
    const layout = await layoutOf()
    await failUnstage(hostId)
    await chooseScenario('E3-NOOP')
    await evaluate('__restoreRemove()')
    const state = await scenarioState()
    check(
      state.errorShown && /Publish failed/.test(state.error),
      `the switch fails at the handoff (${state.error.slice(0, 80)})`
    )
    check(
      state.select === 'E0' &&
        state.hosts.length === 1 &&
        state.hosts[0].id === hostId &&
        !state.hosts[0].staged &&
        state.snapshot === before.snapshot,
      'and the old scenario, scene and cursor are as they were'
    )
    check((await evaluate(read)) === expected, `and the focus is back on ${name} (${await evaluate(read)})`)
    {
      const after = await layoutOf()
      check(unmoved(layout, after), `and giving the focus back did not scroll the page or move the scene (${JSON.stringify(layout)} then ${JSON.stringify(after)})`)
    }
    check(
      await evaluate('document.getElementById("dess-next").closest("#' + hostId + '") !== null'),
      'with the controls in the old scene'
    )
  }
  // A handoff that works leaves no saved focus to be given back later: the focus is where a person can use it, and a later failure does not move it.
  await evaluate(`(document.getElementById('dess-next').focus(), 0)`)
  await chooseScenario('E3-NOOP')
  const ok = await scenarioState()
  check(
    !ok.errorShown && ok.select === 'E3-NOOP' && (await evaluate('document.activeElement.id')) === 'dess-next',
    'a handoff that works keeps the focus on the control that had it'
  )
})

await test('G16R', 'a handoff that fails the controller\'s check, or whose focus move does nothing, is undone with the focus where it was', async () => {
  await fresh()
  const focusCases = [
    ['the scenario list', `(document.getElementById('dess-scenario').focus(), document.activeElement.id)`, 'document.activeElement.id', 'dess-scenario'],
    ['the Next button', `(document.getElementById('dess-next').focus(), document.activeElement.id)`, 'document.activeElement.id', 'dess-next'],
    ['a node inside the old scene', `(document.querySelector('#dess-stage [data-ss-key="S0"]').focus(), document.activeElement.getAttribute('data-ss-key'))`, 'document.activeElement.getAttribute("data-ss-key")', 'S0']
  ]
  const restoreAll = () => evaluate('(window.__restoreAll?.(), 0)')
  // 1. The host's readback passes and the controller's own check then fails, because the document reads as hidden at the last step of the handoff, which leaves the focus alone.
  const hideAfterShowing = shown =>
    evaluate(`(() => {
      const remove = Element.prototype.removeAttribute
      window.__restoreAll = () => { Element.prototype.removeAttribute = remove; delete document.hidden }
      Element.prototype.removeAttribute = function (name) {
        const result = remove.call(this, name)
        // The document reads as hidden from here, which the controller's check of the host sees as Hidden and no focus is affected by.
        if (name === 'inert' && /^dess-host-/.test(this.id) && this.id !== '${shown}') Object.defineProperty(document, 'hidden', { configurable: true, get: () => true })
        return result
      }
      return 0
    })()`)
  // 2. A call of focus that returns normally and does nothing, once, which is a handoff whose focus move silently failed. The undo's call works.
  const failFocusOnce = id =>
    evaluate(`(() => {
      const focus = HTMLElement.prototype.focus
      let skipped = 0
      window.__restoreAll = () => { HTMLElement.prototype.focus = focus }
      HTMLElement.prototype.focus = function (...args) {
        if (this.id === '${id}' && skipped++ === 0) return undefined
        return focus.apply(this, args)
      }
      return 0
    })()`)
  const scenarios = [
    ['the controller\'s check fails after the readback passes', async hostId => hideAfterShowing(hostId), /Hidden/, ['the scenario list', 'the Next button', 'a node inside the old scene']],
    ['the focus move silently does nothing', async () => failFocusOnce('dess-scenario'), /focus did not reach/, ['a node inside the old scene']],
    ['the return of the focus to a control silently does nothing', async () => failFocusOnce('dess-next'), /focus did not reach/, ['the Next button']]
  ]
  for (const [what, arm, pattern, cases] of scenarios) {
    for (const [name, focusIt, read, expected] of focusCases.filter(c => cases.includes(c[0]))) {
      const before = await scenarioState()
      const hostId = before.hosts[0].id
      check((await evaluate(focusIt)) === expected, `${what}, with the focus on ${name}: it is there to begin with`)
      await arm(hostId)
      await chooseScenario('E3-NOOP')
      await restoreAll()
      const state = await scenarioState()
      check(state.errorShown && /Publish failed/.test(state.error) && pattern.test(state.error), `the switch is refused at the handoff (${state.error.slice(0, 100)})`)
      check(state.select === 'E0' && state.hosts.length === 1 && state.hosts[0].id === hostId && !state.hosts[0].staged && state.snapshot === before.snapshot && (await scenes()) === 1, 'and the old scenario, scene and cursor are as they were, with one scene in the accessibility tree')
      check((await evaluate(read)) === expected, `and the focus is back on ${name} (${await evaluate(read)})`)
    }
  }
})

await test(
  'G16P',
  'a handoff that does nothing for one of the two staging writes is found, and rolled back',
  async () => {
    await fresh()
    // The page makes the old scene's write of `inert`, or of its style, return normally and do nothing, as a handoff that silently failed would.
    for (const skipped of ['inert', 'style']) {
      const before = await scenarioState()
      const hostId = before.hosts[0].id
      await evaluate(`(() => {
      const set = Element.prototype.setAttribute
      window.__restoreSet = () => { Element.prototype.setAttribute = set }
      Element.prototype.setAttribute = function (name, value) {
        if (this.id === '${hostId}' && name === '${skipped}') return undefined
        return set.call(this, name, value)
      }
      return 0
    })()`)
      await chooseScenario('E3-NOOP')
      await evaluate('__restoreSet()')
      const state = await scenarioState()
      check(
        state.errorShown && /Publish failed/.test(state.error) && /old scene/.test(state.error),
        `without the old scene's ${skipped}, the switch is refused at the handoff (${state.error.slice(0, 90)})`
      )
      check(
        state.select === 'E0' &&
          state.svgs === 1 &&
          state.hosts.length === 1 &&
          state.hosts[0].id === hostId &&
          !state.hosts[0].staged,
        `and the old scene is still the one shown (${state.hosts.map(h => h.id)})`
      )
      check((await scenes()) === 1, 'with one scene in the accessibility tree')
      check(
        await evaluate(`Boolean(document.getElementById('dess-next').closest('#${hostId}'))`),
        'and the controls are in it'
      )
    }
    await chooseScenario('E3-NOOP')
    const last = await scenarioState()
    check(
      last.select === 'E3-NOOP' && !last.errorShown && (await scenes()) === 1,
      'and the same switch with nothing wrong still goes through'
    )
  }
)

await test(
  'G17',
  'the no-op scenario is nine slices that match the independent engine, and its end commits nothing',
  async () => {
    await fresh()
    await chooseScenario('E3-NOOP')
    const E3 = SCENARIOS['E3-NOOP']
    const ordinals = [Number((await read()).data[0])]
    for (let k = 1; k <= E3.slices; k++) {
      await click('#dess-next')
      const page = await read()
      ordinals.push(Number(page.data[0]))
      const problems = matchesOracle(page, k, E3)
      if (problems.length) check(false, `snapshot ${k}: ${problems.join('; ')}`)
    }
    check(
      ordinals.every((n, i) => n === i),
      `the data node's ordinal advanced by one at each of nine commits (${ordinals})`
    )
    let page = await read()
    const expected = fixtures['E3-NOOP'].expect
    check(
      page.counters[8] === String(expected.stale) && page.counters[9] === String(expected.noops),
      `the stale and no-op counts are the fixture's, ${expected.stale} and ${expected.noops} (${page.counters[8]}, ${page.counters[9]})`
    )
    check(
      page.next.disabled === 'true' && page.next.reason === 'Already at last snapshot',
      'Next is marked as at the last snapshot'
    )
    await click('#dess-next')
    page = await read()
    check(
      page.iSnapshot === '9 of 9' && page.data[0] === '9' && page.live === 'End of scenario',
      `Next at the end commits nothing (${page.iSnapshot}, ${page.data[0]})`
    )
    check(matchesOracle(page, E3.slices, E3).length === 0, "and the page is still the independent engine's snapshot 9")
  }
)

await test(
  'L01',
  'the controls and the inspector are in the scene, the nodes are laid out as designed, and the counters and cells have room',
  async () => {
    await fresh()
    const geometry = () =>
      evaluate(`(() => {
      const svg = document.querySelector('#dess-stage svg')
      const content = svg.querySelector(':scope > g.svg-dom-graph-content')
      const box = e => { const r = e.getBoundingClientRect(); return { left: r.left, right: r.right, top: r.top, bottom: r.bottom, centre: (r.top + r.bottom) / 2 } }
      const nodes = {}
      for (const g of content.querySelectorAll(':scope > g:not(.ss-overlay)')) {
        const label = g.querySelector('text')?.textContent?.trim()
        const rect = g.querySelector('rect')
        if (label && rect && /^(Job dispatch|S\\d+)$/.test(label)) nodes[label] = box(rect)
      }
      const ends = [...content.querySelectorAll('path')].filter(p => p.getAttribute('marker-end') || p.closest('[class*="edge"]')).map(p => {
        const at = p.getPointAtLength(p.getTotalLength())
        const m = p.getScreenCTM()
        return { x: m.a * at.x + m.c * at.y + m.e, y: m.b * at.x + m.d * at.y + m.f }
      })
      const captions = [...content.querySelectorAll('.ss-overlay .ss-caption:not(.ss-legend)')].map(c => ({ text: c.textContent, ...box(c) }))
      const values = Object.fromEntries(['arrived', 'retrying', 'no-op'].map(k => [k, box(content.querySelector('[data-ss-key="' + k + '"] text'))]))
      const legend = [...content.querySelectorAll('.ss-legend')].map(c => c.textContent)
      const probe = document.createElementNS('http://www.w3.org/2000/svg', 'text'); probe.textContent = '999999'; content.appendChild(probe)
      const cellDigits = probe.getBoundingClientRect().width; probe.remove()
      const dataGroup = [...content.querySelectorAll(':scope > g:not(.ss-overlay)')].at(-1)
      const cells = [...dataGroup.querySelectorAll('rect')].map(r => ({ width: r.getBoundingClientRect().width }))
      const inFo = id => { const e = document.getElementById(id); return Boolean(e && e.closest('foreignObject') && e.closest('svg') === svg) }
      return { nodes, ends, captions, values, legend, cells, cellDigits, inFo: Object.fromEntries(['dess-prev', 'dess-next', 'dess-restart', 'dess-play', 'dess-speed', 'dess-repair', 'dess-seek', 'dess-scenario', 'dess-job', 'dess-snapshot', 'dess-i-snapshot', 'dess-counters'].map(id => [id, inFo(id)])),
               toolbar: box(document.getElementById('dess-toolbar')), side: box(document.getElementById('dess-side')), svg: box(svg), outside: document.querySelectorAll('#panel-service-simulator > .dess-wrap, #panel-service-simulator > .dess-head, #panel-service-simulator > .dess-transport').length }
    })()`)
    const near = (a, b) => Math.abs(a - b) <= 1.5
    for (const id of ['E0', 'E3-NOOP', 'scaled', 'five']) {
      await chooseScenario(id)
      const g = await geometry()
      const servers = SCENARIOS[id].servers
      const names = Array.from({ length: servers }, (_, i) => `S${i}`)
      const w = g.nodes['Job dispatch']
      const expected = (g.nodes.S0.top + g.nodes[names.at(-1)].bottom) / 2
      check(
        near(w.centre, expected),
        `${id}: the dispatch node's midline is the common midline of the ${servers} servers (${w.centre.toFixed(1)}, ${expected.toFixed(1)})`
      )
      const entering = names.map((n, i) =>
        g.ends.some(e => near(e.x, g.nodes[n].left) && e.y >= g.nodes[n].top - 1 && e.y <= g.nodes[n].bottom + 1)
      )
      check(entering.every(Boolean), `${id}: every connector enters its server on the west side (${entering})`)
      const widest = Math.max(
        ...g.captions
          .filter(c =>
            /^(Arrived|Admitted|Rejected|Completed|Timed out|Cancelled|Failed|Waiting to retry|Stale pops|No-op pops)$/.test(
              c.text
            )
          )
          .map(c => c.right)
      )
      const narrowest = Math.min(...Object.values(g.values).map(v => v.left))
      check(
        names.every(n => g.nodes[n].bottom <= g.svg.bottom - 1),
        `${id}: every server box is inside the scene (${names.map(n => g.nodes[n].bottom.toFixed(0))} of ${g.svg.bottom.toFixed(0)})`
      )
      check(
        widest + 4 <= narrowest,
        `${id}: the longest counter name ends at ${widest.toFixed(1)}, before the values at ${narrowest.toFixed(1)}`
      )
    }
    const g = await geometry()
    check(
      Object.values(g.inFo).every(Boolean),
      `every control and the inspector is in a foreignObject of the scene (${Object.entries(g.inFo)
        .filter(([, v]) => !v)
        .map(([k]) => k)})`
    )
    check(g.outside === 0, 'and no control is left in the panel outside the scene')
    check(
      g.legend.length === 4 && g.legend[0] === 'snapshot · tick' && g.legend[3] === 'cancelled · rejected',
      `each row of the eight cells is named beside it (${g.legend})`
    )
    check(
      g.cells.every(c => c.width >= g.cellDigits),
      `and each cell is wide enough for six digits (${g.cells.map(c => c.width.toFixed(0))} of ${g.cellDigits.toFixed(0)})`
    )
    // However narrow the window, the controls and the inspector stay with the scene and are not moved below it.
    await send('Emulation.setDeviceMetricsOverride', { width: 640, height: 1000, deviceScaleFactor: 1, mobile: false })
    await pause(150)
    const narrow = await geometry()
    await send('Emulation.setDeviceMetricsOverride', { width: 1500, height: 1000, deviceScaleFactor: 1, mobile: false })
    check(
      narrow.side.left > narrow.toolbar.right - 1 &&
        near(narrow.side.top, narrow.toolbar.top) &&
        narrow.side.bottom <= narrow.svg.bottom + 1,
      `at 640 px wide the inspector is still beside the scene and inside it (${narrow.side.left.toFixed(0)} to ${narrow.side.bottom.toFixed(0)})`
    )
  }
)

await test('L02', 'a server box is green while it runs a job, red while it is down, and blue while it is idle', async () => {
  await fresh()
  const COLOURS = { idle: 'rgb(238, 244, 255)', busy: 'rgb(187, 247, 208)', down: 'rgb(254, 202, 202)' }
  const seen = new Set()
  const boxes = () =>
    evaluate(`(() => {
      const svg = document.querySelector('#dess-stage svg')
      return [...svg.querySelectorAll('[data-ss-key^="S"]')].filter(g => /^S\\d+$/.test(g.getAttribute('data-ss-key'))).map(g => {
        const key = g.getAttribute('data-ss-key')
        const state = ['busy', 'down', 'idle'].find(s => g.classList.contains('ss-' + s))
        const rect = [...svg.querySelectorAll('.svg-dom-graph-content > g[aria-label^="' + key + '."] > rect')][0]
        return { key, state, fill: getComputedStyle(rect).fill, text: g.querySelector('text')?.textContent }
      })
    })()`)
  for (let k = 0; k <= E0_SLICES; k++) {
    if (k > 0) await click('#dess-next')
    for (const box of await boxes()) {
      seen.add(box.state)
      if (box.fill !== COLOURS[box.state]) check(false, `snapshot ${k}: ${box.key} is ${box.state} and its box is ${box.fill}, not ${COLOURS[box.state]}`)
      if (box.state === 'down' && box.text !== 'down') check(false, `snapshot ${k}: ${box.key} is down and says ${box.text}`)
    }
  }
  check(seen.has('idle') && seen.has('busy') && seen.has('down'), `the walkthrough shows all three states (${[...seen]}), and each box was the colour of its state at every snapshot`)
  // The five-server scenario, where the fifth box is the one that the old layout had no room for.
  await chooseScenario('five')
  for (let i = 0; i < 6; i++) await click('#dess-next')
  const five = await boxes()
  check(five.length === 5 && five.every(b => b.fill === COLOURS[b.state]), `five servers, each box the colour of its state (${five.map(b => b.state)})`)
  check(matchesOracle(await read(), 6, SCENARIOS.five).length === 0, 'and snapshot 6 of that scenario is the independent engine\'s')
})

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// P4: the long paths, and the cycles that show that nothing is left behind.

/** A small seeded generator, so that a run's "random" seeks are the same on every run and a failure can be repeated. */
const seeded = seed => {
  let a = seed >>> 0
  return () => {
    a = (a + 0x6d2b79f5) >>> 0
    let x = Math.imul(a ^ (a >>> 15), 1 | a)
    x = (x + Math.imul(x ^ (x >>> 7), 61 | x)) ^ x
    return ((x ^ (x >>> 14)) >>> 0) / 4294967296
  }
}
/** A hash of what the page shows of the simulation: every slot of the overlay and the cells of the data node. */
const domHash = async () => {
  const page = await read()
  return crypto.createHash('sha256').update(JSON.stringify([page.slots, page.data])).digest('hex')
}
const seekTo = async target => {
  await evaluate(`(() => { const s = document.getElementById('dess-seek'); s.value = '${target}'; s.dispatchEvent(new Event('input', { bubbles: true })) })()`)
  return until(`document.getElementById('dess-seek-text').textContent === 'Committed ${target}'`, 30000)
}

await test(
  'G18',
  'the scaled scenario: a walk of all 487 snapshots, random seeks and rate changes agree with the independent engine',
  async () => {
    await fresh()
    await chooseScenario('scaled')
    const S = SCENARIOS.scaled
    const inStage = () => evaluate(`document.querySelectorAll('#dess-stage *').length`)
    const elements = await inStage()
    let bad = 0
    const report = text => {
      if (bad++ < 5) check(false, text)
    }
    const confirm = async (k, route) => {
      const page = await read()
      const problems = matchesOracle(page, k, S)
      if (problems.length) report(`${route}, snapshot ${k}: ${problems.join('; ')}`)
      const before = await domHash()
      await click('#dess-repair')
      if ((await domHash()) !== before) report(`${route}, snapshot ${k}: the page is not its own full redraw`)
    }
    for (let k = 1; k <= S.slices; k++) {
      await click('#dess-next')
      if (k % 25 === 0 || k === S.slices) await confirm(k, 'by Next')
    }
    check(bad === 0, `all 487 snapshots were walked by Next, and each of the ${Math.floor(S.slices / 25) + 1} that were read matched the independent engine and its own full redraw`)
    check((await inStage()) === elements, `and no element was added to the scene by 487 draws (${elements} then ${await inStage()})`)
    const random = seeded(20261106)
    bad = 0
    for (let i = 0; i < 30; i++) {
      const k = Math.floor(random() * (S.slices + 1))
      if (!(await seekTo(k))) report(`the seek to ${k} did not arrive`)
      else await confirm(k, 'by seek')
    }
    check(bad === 0, 'thirty random seeks, in a fixed order, each matched the independent engine and its own full redraw')
    // The rate is changed while playing, and each tick is one slice.
    await seekTo(100)
    await hold(true)
    bad = 0
    await click('#dess-play')
    let at = 100
    for (const ms of [1000, 500, 200, 100, 500, 1000]) {
      await setSpeed(ms)
      for (let i = 0; i < 4; i++) {
        const p = await pending()
        if (p.timers.length !== 1 || p.timers[0].delay !== ms) report(`at ${ms} ms there are ${JSON.stringify(p.timers)} timers`)
        await evaluate(`__timers.fire(${p.timers[0]?.id})`)
        at++
        if ((await slice()) !== at) report(`a tick at ${ms} ms did not commit slice ${at}`)
      }
    }
    await confirm(at, 'after the rate changes')
    await click('#dess-play')
    await hold(false)
    check(bad === 0, 'six rates in turn, four ticks at each, were one slice a tick with one timer waiting, and the data node followed each commit')
    const final = await read()
    check(Number(final.data[0]) === at && final.iSnapshot === `${at} of ${S.slices}`, `the data node's ordinal is the snapshot (${final.data[0]}, ${final.iSnapshot})`)
  },
  { long: true }
)

await test(
  'G19',
  'the advanced scenario: a thousand commits by Play, and seeks across the 5,464 snapshots, agree with the independent engine',
  async () => {
    await fresh()
    await chooseScenario('W6')
    const S = SCENARIOS.W6
    let bad = 0
    const report = text => {
      if (bad++ < 5) check(false, text)
    }
    await hold(true)
    await setSpeed(100)
    await click('#dess-play')
    // A thousand ticks, each fired by the test, and each one slice. The ordinal of the data node is read straight after the tick, since it is written
    // by the draw itself.
    for (let batch = 0; batch < 10; batch++) {
      const result = await evaluate(`(() => {
        const cell = () => {
          const nodes = [...document.querySelectorAll('#dess-stage svg > g.svg-dom-graph-content > g:not(.ss-overlay)')]
          return Number(nodes.at(-1).querySelector('text').textContent.trim())
        }
        const problems = []
        for (let i = 0; i < 100; i++) {
          const timers = __timers.pending()
          if (timers.length !== 1) { problems.push('there are ' + timers.length + ' timers after ' + i); break }
          const before = cell()
          const fired = __timers.fire(timers[0].id)
          if (fired.threw) { problems.push('a tick threw: ' + fired.threw); break }
          if (cell() !== before + 1) { problems.push('a tick moved the data node from ' + before + ' to ' + cell()); break }
        }
        return problems
      })()`)
      if (result.length) report(`batch ${batch}: ${result.join('; ')}`)
      const k = (batch + 1) * 100
      const page = await read()
      const problems = matchesOracle(page, k, S)
      if (problems.length) report(`after ${k} commits: ${problems.join('; ')}`)
    }
    check(bad === 0, 'a thousand ticks were a thousand commits, one timer at a time, and every hundredth matched the independent engine')
    await click('#dess-play')
    await hold(false)
    check((await slice()) === 1000 && (await read()).play.pressed === 'false', 'and Pause stopped it at snapshot 1,000')
    const random = seeded(4000)
    const targets = [...Array.from({ length: 12 }, () => Math.floor(random() * (S.slices + 1))), S.slices, 0]
    bad = 0
    for (const k of targets) {
      if (!(await seekTo(k))) {
        report(`the seek to ${k} did not arrive`)
        continue
      }
      const page = await read()
      const problems = matchesOracle(page, k, S)
      if (problems.length) report(`the seek to ${k}: ${problems.join('; ')}`)
      const before = await domHash()
      await click('#dess-repair')
      if ((await domHash()) !== before) report(`the seek to ${k} is not its own full redraw`)
    }
    check(bad === 0, `${targets.length} seeks, to the end and the start and at random, each matched the independent engine and its own full redraw`)
    current.limits = ['Nothing here is a measurement of speed. The advanced scenario is a long run to check, and it is not a benchmark.']
  },
  { long: true }
)

await test('G20', `${CYCLES} cycles of switching, hiding, taking the panel down and building it again leave nothing behind`, async () => {
  await fresh()
  const counts = () => evaluate('__counts()')
  const keepElements = () =>
    evaluate(`(window.__byId = Object.fromEntries([...document.querySelectorAll('#dess-toolbar [id], #dess-side [id]')].map(e => [e.id, e])), 0)`)
  const ready = () => until(`document.getElementById('${PANEL}').dataset.panelState === 'ready'`, 10000)
  const module = `import('/pkg/svg_dom_graph_demo.js')`
  const cycle = async () => {
    for (const id of ['E3-NOOP', 'scaled', 'five', 'E0']) {
      await chooseScenario(id)
      await click('#dess-next')
    }
    await routeAway()
    await routeBack()
    await hold(true)
    await setSpeed(500)
    await click('#dess-play')
    const p = await pending()
    const firstFrame = await evaluate('__seen.captured.frames.length')
    await evaluate(`(() => { const s = document.getElementById('dess-seek'); s.value = '9'; s.dispatchEvent(new Event('input', { bubbles: true })) })()`)
    // A timer and a frame are waiting when the panel is taken down, and the page's copies of them are kept.
    const callbacks = await evaluate(`(window.__kept = { timers: __seen.captured.timers.slice(-3), frames: __seen.captured.frames.slice(${firstFrame}) }, 0)`)
    const before = await evaluate('document.getElementsByTagName("*").length')
    await evaluate(`${module}.then(m => (m.dispose_panel('${PANEL}'), m.dispose_panel('${PANEL}'), 0))`)
    await hold(false)
    const gone = await evaluate(`({ hosts: document.querySelectorAll('[id^="dess-host-"]').length, observers: [...window.__liveObservers].filter(o => o).length, state: document.getElementById('${PANEL}').dataset.panelState ?? null,
      controlsHome: document.getElementById('dess-toolbar').parentElement.classList.contains('dess-holder'), frames: document.querySelectorAll('#${PANEL} details.source').length, svgs: document.querySelectorAll('#${PANEL} svg').length })`)
    // The kept callbacks are called as a browser that still held them would call them.
    const retained = await evaluate(`(() => {
      const outcome = { ran: 0, threw: 0 }
      const elements = document.getElementsByTagName('*').length
      for (const f of [...window.__kept.timers, ...window.__kept.frames]) { try { f(performance.now()); outcome.ran++ } catch { outcome.threw++ } }
      outcome.elementsMoved = document.getElementsByTagName('*').length !== elements
      return outcome
    })()`)
    await evaluate(`${module}.then(m => (m.init_panel('${PANEL}'), 0))`)
    await ready()
    await keepElements()
    return { p, gone, retained, callbacks, before }
  }
  const warm = await cycle()
  const baseline = await counts()
  check(baseline.externrefs !== null, `the table of externrefs can be read (${baseline.externrefs} slots)`)
  check(warm.gone.hosts === 0 && warm.gone.observers === 0 && warm.gone.svgs === 0 && warm.gone.state === null && warm.gone.controlsHome && warm.gone.frames === 0, `taking the panel down leaves no scene, no observer, no source frame and no state, and the controls are back where they are written (${JSON.stringify(warm.gone)})`)
  check(!warm.retained.elementsMoved, `callbacks that were kept and are called afterwards change nothing in the page (${warm.retained.ran} ran, ${warm.retained.threw} refused)`)
  let drift = null
  for (let n = 1; n <= CYCLES && !drift; n++) {
    const done = await cycle()
    const now = await counts()
    if (!same(now, baseline)) drift = { n, baseline, now }
    if (done.gone.hosts !== 0 || done.gone.observers !== 0 || done.retained.elementsMoved) drift = { n, gone: done.gone, retained: done.retained }
  }
  check(drift === null, drift ? `after cycle ${drift.n} the counts are not the baseline: ${JSON.stringify(drift)}` : `after ${CYCLES} further cycles the elements, listeners, observers, externref slots and waiting callbacks are exactly the baseline (${JSON.stringify(baseline)})`)
  const page = await read()
  check(page.iSnapshot === '0 of 12' && matchesOracle(page, 0).length === 0, 'and the panel that was built last works: it is at snapshot 0 of the walkthrough')
  await click('#dess-next')
  check(matchesOracle(await read(), 1).length === 0, 'and steps to the independent engine\'s snapshot 1')
  const errors = exceptions.filter(e => !/favicon/.test(e))
  check(errors.length === 0, `and the page raised no exception (${errors.length}: ${errors.join(' | ').slice(0, 200)})`)
})

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const first = (command, args) => spawnSync(command, args, { cwd: root, encoding: 'utf8' }).stdout.trim().split('\n')[0]
const version = await evaluate('navigator.userAgent')
await chrome.close()
server.close()
const counted = results.flatMap(r => r.checks)
console.log(`\n${results.length} tests, ${counted.length} checks, ${failures} failed`)
if (out) {
  fs.mkdirSync(path.dirname(out), { recursive: true })
  fs.writeFileSync(
    out,
    `${JSON.stringify({ measured_at: new Date().toISOString(), git_sha: first('git', ['rev-parse', 'HEAD']), git_dirty: first('git', ['status', '--porcelain']).length > 0, browser: version, tests: results }, null, 2)}\n`
  )
}
process.exit(failures ? 1 : 0)
