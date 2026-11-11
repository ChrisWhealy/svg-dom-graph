// P0-A: can a scene be built and staged in a connected host that nobody can see, reach or hear?
//
//   node dess/demo-panel/p0/staging.mjs [--out dess/demo-panel/p0/evidence/staging.json]
//
// The cross-size scenario switch builds the new scene while the old one is still the live picture, so it needs a host that the surface can be built
// into (it has to be connected, laid out and measurable) and that a person cannot see, focus, click or hear until the switch commits. This probe tries
// two candidates, with the real simulator surface and controller built into each under its CSS:
//
//   opacity_inert           `opacity:0; pointer-events:none` and the `inert` attribute, in the flow of the panel.
//   opacity_inert_absolute  the same, taken out of the flow with `position:absolute`, so that it occupies no room.
//   offscreen_inert  the same container moved far off the screen, with `inert`, and a fixed width so that it keeps its geometry.
//
// and a control with no staging, which has to be found by every check below. A check that the control does not pass is a probe that cannot see, and
// fails the run. Neither candidate ever uses `display:none` or `visibility:hidden`, which make a host unmeasurable.
//
// Gates, for each candidate:
//   S1  presence   connected, `checkVisibility()` true, and the host's own presence test says Live.
//   S2  geometry   the surface built and drew, with finite measurements: a box, one content group, every text measurable, the data node's setter
//                  called and the page equal to the projection.
//   S3  isolation  Tab never reaches the candidate, scripted focus on it does not take, a real click reaches none of it, and the browser's
//                  accessibility tree holds none of its names.
//   S5  layout     the staged host does not move the live scene, what follows it or the height of the page, since a host in the flow of the panel
//                  occupies its own height even when nobody can see it.
//   S4  publish    removing the staging exposes the scene to Tab, a click and the accessibility tree, with presence still Live, and staging it again
//                  isolates it as before, so that a publication can be undone.
//
// The strategy that is selected is the first candidate that passes every gate. The result, with the browser, the build and every observation,
// is written to dess/demo-panel/p0/evidence/staging.json. Chrome is the release gate. It exits with 1 if neither candidate passes or the control does
// not behave as a control.
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { openHarnessPage } from '../../../perf/runner/lib/harness-page.mjs'
import { press, release, pause } from '../../../perf/lib/input.mjs'

const root = path.resolve(import.meta.dirname, '../../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const out = path.resolve(argument('--out') ?? path.join(root, 'dess/demo-panel/p0/evidence/staging.json'))
const first = (command, args) => spawnSync(command, args, { cwd: root, encoding: 'utf8' }).stdout.trim().split('\n')[0]
const SEED = '20261106'

// The CSS that each mode puts on the container, which is also what a publication removes and a rollback puts back.
const STAGING_STYLE = {
  opacity_inert: 'opacity:0;pointer-events:none',
  opacity_inert_absolute: 'position:absolute;left:0;top:0;width:1000px;opacity:0;pointer-events:none',
  opacity_aria_hidden: 'opacity:0;pointer-events:none',
  offscreen_inert: 'position:absolute;left:-20000px;top:0;width:1000px'
}

const page = await openHarnessPage()
const evaluate = async expression => JSON.parse(await page.evaluate(`JSON.stringify(${expression})`))
const key = async (type, name, code, vk) =>
  page.send('Input.dispatchKeyEvent', { type, key: name, code, windowsVirtualKeyCode: vk })
const tab = async () => {
  await key('rawKeyDown', 'Tab', 'Tab', 9)
  await key('keyUp', 'Tab', 'Tab', 9)
  await pause(30)
}

/** Everything that is read from the page about the staged scene, at one moment. */
const observe = () =>
  evaluate(`(() => {
    const c = document.getElementById('perf-sim-container')
    const svg = document.getElementById('perf-sim-svg')
    const box = e => { const r = e.getBoundingClientRect(); return { x: r.x, y: r.y, width: r.width, height: r.height } }
    const finite = b => Object.values(b).every(Number.isFinite)
    const texts = [...svg.querySelectorAll('text')]
    const bboxes = texts.map(t => { try { const b = t.getBBox(); return [b.x, b.y, b.width, b.height].every(Number.isFinite) } catch { return false } })
    const labels = [...new Set([...svg.querySelectorAll('[aria-label]')].map(e => e.getAttribute('aria-label')).filter(Boolean))]
    const state = JSON.parse(window.perfHarness.sim_state())
    return {
      connected: svg.isConnected,
      checkVisibility: svg.checkVisibility(),
      containerStyle: c.getAttribute('style'),
      inert: c.inert,
      box: box(svg),
      boxIsFinite: finite(box(svg)),
      viewBox: svg.getAttribute('viewBox'),
      contentGroups: svg.querySelectorAll(':scope > g.svg-dom-graph-content').length,
      textCount: texts.length,
      textMeasurable: bboxes.every(Boolean) && bboxes.length > 0,
      labelCount: labels.length,
      presence: state.presence,
      dirty: state.dirty,
      setterCalls: state.setter_calls,
      dataCells: state.data_cells.length,
      pageIsProjection: state.page_is_projection,
      committedSlice: state.committed_slice
    }
  })()`)

/** Puts two things inside the staged region that can take focus, since a scene may offer none: an SVG element and an HTML button. */
const addProbes = () =>
  page.evaluate(`(() => {
    const c = document.getElementById('perf-sim-container')
    const svg = document.getElementById('perf-sim-svg')
    const ns = 'http://www.w3.org/2000/svg'
    const g = document.createElementNS(ns, 'g')
    g.id = 'p0-svg-probe'
    g.setAttribute('tabindex', '0')
    g.setAttribute('role', 'button')
    g.setAttribute('aria-label', 'P0 SVG probe')
    const r = document.createElementNS(ns, 'rect')
    r.setAttribute('x', '10'); r.setAttribute('y', '10'); r.setAttribute('width', '120'); r.setAttribute('height', '40')
    g.appendChild(r)
    svg.appendChild(g)
    const b = document.createElement('button')
    b.id = 'p0-html-probe'
    b.textContent = 'P0 HTML probe'
    // At the top of the container, so that it is in view when the container is scrolled to, and a real click can reach it.
    c.insertBefore(b, c.firstChild)
    window.__p0clicks = 0
    for (const type of ['mousedown', 'mouseup', 'click']) c.addEventListener(type, () => window.__p0clicks++, true)
  })()`)

const insideContainer =
  'Boolean(document.activeElement && document.activeElement.closest && document.activeElement.closest("#perf-sim-container"))'

/** Tab from the control before the scene, as far as the end of the page, and say whether any stop is inside the scene's container. */
const tabbing = async () => {
  await page.evaluate(`document.getElementById('perf-sim-seek').focus()`)
  const stops = []
  for (let i = 0; i < 12; i++) {
    await tab()
    stops.push(
      await evaluate(
        `({ id: document.activeElement?.id ?? '', tag: document.activeElement?.tagName ?? '', inside: ${insideContainer} })`
      )
    )
  }
  return {
    reachedScene: stops.some(s => s.inside),
    probesReached: [...new Set(stops.filter(s => s.id.startsWith('p0-')).map(s => s.id))],
    stops: stops.map(s => `${s.tag}#${s.id}${s.inside ? '*' : ''}`)
  }
}

/** A scripted `focus()` on each probe, which an inert subtree must ignore. */
const scriptedFocus = () =>
  evaluate(`(() => {
    document.getElementById('perf-sim-seek').focus()
    const taken = {}
    for (const id of ['p0-svg-probe', 'p0-html-probe']) {
      document.getElementById(id).focus()
      taken[id] = document.activeElement?.id === id
    }
    return taken
  })()`)

/** A real mouse click, by the browser's input pipeline, on the HTML probe and on the SVG probe, as far as either is on the screen. */
const clicking = async () => {
  await page.evaluate(`document.getElementById('perf-sim-container').scrollIntoView({ block: 'start' })`)
  const spots = await evaluate(`(() => {
    const spot = id => {
      const r = document.getElementById(id).getBoundingClientRect()
      const x = r.x + r.width / 2
      const y = r.y + r.height / 2
      return { id, x, y, onScreen: x >= 0 && y >= 0 && x <= innerWidth && y <= innerHeight }
    }
    return [spot('p0-html-probe'), spot('p0-svg-probe')]
  })()`)
  await page.evaluate('window.__p0clicks = 0')
  const clicked = []
  for (const s of spots) {
    if (!s.onScreen) {
      clicked.push({ id: s.id, onScreen: false })
      continue
    }
    const before = await evaluate('window.__p0clicks')
    await press(page, s.x, s.y)
    await release(page, s.x, s.y)
    await pause(30)
    clicked.push({ id: s.id, onScreen: true, events: (await evaluate('window.__p0clicks')) - before })
  }
  return clicked
}

/** The browser's accessibility tree: which of the scene's own names, and the probes', are in it. */
const hearing = async () => {
  const names = await evaluate(
    `[...new Set([...document.getElementById('perf-sim-svg').querySelectorAll('[aria-label]')].map(e => e.getAttribute('aria-label')).filter(Boolean)), 'P0 HTML probe']`
  )
  const tree = (await page.send('Accessibility.getFullAXTree')).result.nodes.filter(n => !n.ignored)
  const heard = new Set(tree.map(n => String(n.name?.value ?? '')))
  const exposed = names.filter(n => heard.has(n))
  return {
    names: names.length,
    exposed: exposed.length,
    exposedNames: exposed.slice(0, 8),
    tabbableProbe: heard.has('P0 SVG probe')
  }
}

const setStaging = (mode, on) =>
  page.evaluate(`(() => {
    const c = document.getElementById('perf-sim-container')
    const ariaOnly = ${JSON.stringify(mode === 'opacity_aria_hidden')}
    if (${JSON.stringify(on)}) {
      c.setAttribute('style', ${JSON.stringify(STAGING_STYLE[mode] ?? '')})
      if (ariaOnly) c.setAttribute('aria-hidden', 'true')
      else c.inert = true
    } else {
      c.removeAttribute('style')
      c.removeAttribute('aria-hidden')
      c.inert = false
    }
  })()`)

// The accessibility tree is read first, before anything takes focus: Chrome exposes content that has the focus even under `aria-hidden`, so a reading
// taken after the keyboard has been into the scene would say that the scene was never hidden.
/**
 * Whether a staged host moves the live scene or what follows it. Two stand-ins sit around the container in the panel's flow: a block as tall as the
 * scene, for the live scene, and one after, for the rest of the page. Their positions and the height of the page are measured with the staged host
 * shown to layout and with it taken out of layout for the length of the measurement only (nothing is built while it is out). A host in the flow
 * pushes what follows it down by its own height, and one out of the flow does not.
 */
const layout = () =>
  evaluate(`(() => {
    const c = document.getElementById('perf-sim-container')
    const stage = c.parentElement
    const live = document.createElement('div')
    live.id = 'p0-live'
    live.style.height = '480px'
    stage.insertBefore(live, c)
    const below = document.createElement('div')
    below.id = 'p0-below'
    below.style.height = '40px'
    stage.appendChild(below)
    const measure = () => ({
      liveY: live.getBoundingClientRect().y + scrollY,
      belowY: below.getBoundingClientRect().y + scrollY,
      pageHeight: document.documentElement.scrollHeight
    })
    const shown = measure()
    const display = c.style.display
    c.style.display = 'none'
    const without = measure()
    c.style.display = display
    live.remove()
    below.remove()
    return {
      liveShift: shown.liveY - without.liveY,
      belowShift: shown.belowY - without.belowY,
      pageHeightShift: shown.pageHeight - without.pageHeight
    }
  })()`)

const exposure = async () => {
  const ax = await hearing()
  return { ax, tab: await tabbing(), focus: await scriptedFocus(), clicks: await clicking() }
}
const isolated = e =>
  !e.tab.reachedScene &&
  Object.values(e.focus).every(taken => !taken) &&
  e.clicks.every(c => !c.onScreen || c.events === 0) &&
  e.ax.exposed === 0
const exposed = e =>
  e.tab.probesReached.length === 2 &&
  Object.values(e.focus).every(Boolean) &&
  e.clicks.some(c => c.onScreen && c.events > 0) &&
  e.ax.exposed > 0

const run = async mode => {
  const built = JSON.parse(
    await page.evaluate(
      `window.perfHarness.sim_build(JSON.stringify({ seed: ${JSON.stringify(SEED)}, product: true, staging: ${JSON.stringify(mode)} }))`
    )
  )
  if (!built.ok) return { mode, built: false, error: built.error }
  await addProbes()
  const result = { mode: mode ?? 'none (control)', built: true, elements: built.elements }
  result.observed = await observe()
  if (mode) result.layout = await layout()
  result.staged = await exposure()
  if (mode) {
    // The publication: the staging comes off, and the scene has to be live and reachable. Then it goes back on, and it has to be isolated again.
    await setStaging(mode, false)
    result.published = { observed: await observe(), ...(await exposure()) }
    await setStaging(mode, true)
    result.restaged = await exposure()
  }
  await page.evaluate('window.perfHarness.sim_teardown()')
  return result
}

const gates = (r, control) => {
  const o = r.observed
  const live = x => x.connected && x.checkVisibility && x.presence === 'Live' && !x.dirty
  return {
    S1_presence: live(o) && (!r.published || live(r.published.observed)),
    S2_geometry:
      o.boxIsFinite &&
      o.box.width > 0 &&
      o.box.height > 0 &&
      o.contentGroups === 1 &&
      o.textMeasurable &&
      o.setterCalls >= 1 &&
      o.dataCells === 8 &&
      o.pageIsProjection,
    S3_isolation: isolated(r.staged),
    // The staged host does not move the live scene, what follows it, or the height of the page.
    S5_no_layout_shift: Boolean(r.layout) && r.layout.liveShift === 0 && Math.abs(r.layout.belowShift) < 1 && Math.abs(r.layout.pageHeightShift) < 1,
    S4_publish: Boolean(r.published) && exposed(r.published) && isolated(r.restaged)
  }
}

try {
  const control = await run(null)
  // The control is the scene as it is shown. If the probes do not find it, they would not have found a leak, and nothing that follows means anything.
  const controlSees = control.built && exposed(control.staged)
  const results = {}
  const CANDIDATES = ['opacity_inert', 'opacity_inert_absolute', 'offscreen_inert']
  for (const mode of CANDIDATES) {
    const r = await run(mode)
    results[mode] = r.built ? { ...r, gates: gates(r) } : r
    results[mode].passes = r.built && Object.values(results[mode].gates).every(Boolean)
  }
  // The negative control: `aria-hidden` without `inert`. It hides the scene from the accessibility tree and not from the keyboard, so the isolation
  // check has to fail on it. If it does not, the probe cannot see a keyboard leak, and a pass of `inert` proves nothing.
  const negative = await run('opacity_aria_hidden')
  const negativeLeaks =
    negative.built && !isolated(negative.staged) && negative.staged.tab.reachedScene && negative.staged.ax.exposed === 0
  const selected = CANDIDATES.find(m => results[m].passes) ?? null
  const evidence = {
    identity: {
      measured_at: new Date().toISOString(),
      git_sha: first('git', ['rev-parse', 'HEAD']),
      git_dirty: spawnSync('git', ['status', '--porcelain'], { cwd: root, encoding: 'utf8' }).stdout.trim().length > 0,
      browser: await page.evaluate('navigator.userAgent'),
      rustc: first('rustc', ['--version']),
      wasm_pack: first('wasm-pack', ['--version']),
      node: process.version
    },
    css: STAGING_STYLE,
    control: { sees_the_scene: controlSees, observations: control },
    candidates: results,
    negative_control: { aria_hidden_without_inert_leaks_to_the_keyboard: negativeLeaks, observations: negative },
    selected,
    pass: controlSees && negativeLeaks && selected !== null
  }
  fs.mkdirSync(path.dirname(out), { recursive: true })
  fs.writeFileSync(out, JSON.stringify(evidence, null, 2) + '\n')

  console.log(
    `the control, which is shown: ${controlSees ? 'the probes find it (Tab, scripted focus, a click and the accessibility tree)' : 'THE PROBES DO NOT FIND IT, so they prove nothing'}`
  )
  for (const [mode, r] of Object.entries(results)) {
    if (!r.built) console.log(`${mode}: the scene could not be built: ${r.error}`)
    else
      console.log(
        `${mode}: ${Object.entries(r.gates)
          .map(([g, ok]) => `${g} ${ok ? 'pass' : 'FAIL'}`)
          .join(', ')}`
      )
  }
  console.log(
    `negative control, aria-hidden without inert: ${negativeLeaks ? 'the probe finds the keyboard leak, as it must (Tab reaches the scene while the accessibility tree has none of it)' : 'THE PROBE DOES NOT SEE A LEAK, so it proves nothing'}`
  )
  console.log(`selected staging: ${selected ?? 'none'}`)
  console.log(`evidence: ${path.relative(root, out)}`)
  process.exit(evidence.pass ? 0 : 1)
} finally {
  await page.close()
}
