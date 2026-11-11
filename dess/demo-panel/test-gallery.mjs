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
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { launch, serve } from '../../perf/lib/chrome-session.mjs'
import { press, release, move, pause } from '../../perf/lib/input.mjs'
import { run } from '../reference-js/oracle.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const stageDir = path.resolve(argument('--stage') ?? path.join(root, 'target/demo-stage'))
const only = argument('--only')?.split(',') ?? null
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
const E0 = { configId: 'fixture/E0', seed: 0, config: fixtures.E0.config, script: fixtures.E0.script }
const E0_SLICES = fixtures.E0.expect.slices
const oracleAt = k => {
  if (k === 0) return { overlay: Array(10).fill(0), data: Array(8).fill(0), tick: 0, waiting: 0, busy: 0 }
  const r = run(E0, { stopAfterSlices: k })
  const c = r.counters
  const s = r.slices[k - 1]
  const retry = r.jobs.filter(j => j[2].startsWith('retrywait')).length
  return {
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
      return observe.call(this, target, options)
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
    if ((await evaluate(`document.getElementById('${PANEL}')?.dataset.panelState ?? null`)) === 'ready') return
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
      errorHidden: document.getElementById('dess-error').hidden, errorText: text('dess-error')
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
const test = async (id, title, body) => {
  if (only && !only.includes(id)) return
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
const matchesOracle = (page, k) => {
  const o = oracleAt(k)
  const problems = []
  if (page.iSnapshot !== `${k} of ${E0_SLICES}`) problems.push(`snapshot ${page.iSnapshot}`)
  if (page.tick !== String(o.tick)) problems.push(`tick ${page.tick} not ${o.tick}`)
  if (page.waiting !== `${o.waiting} of 3`) problems.push(`waiting ${page.waiting} not ${o.waiting}`)
  if (page.busy !== `${o.busy} of 2`) problems.push(`busy ${page.busy} not ${o.busy}`)
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
  check(counts.viewBox === '0 0 1000 480', `the fixed view box (${counts.viewBox})`)
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
  current.deferred =
    'the same arrow keys on the 5,464 snapshots of the W6 reference trace need the scenario to be switched, which is P3'
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
