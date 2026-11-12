// What the gallery panel does in another browser, driven through that browser's own WebDriver, and written down as observations (design D5).
//
//   node dess/demo-panel/p4/webdriver-observe.mjs --browser firefox|safari [--viewport 390x844] [--out FILE]
//
// Chrome is the release gate, and this script is not a test suite: it records what a person would see and do in a visit, and says plainly when a step
// could not be done. It serves the staged gallery (`cargo run -p demo-server -- --build-only` first). The driver is geckodriver, which wasm-pack caches, or
// Safari's own `safaridriver`, which needs Safari's "Allow Remote Automation" (Develop menu) and opens a visible window.
//
// What it looks at, in the order that a person would meet it:
//   - the panel is built, and its controls are in a foreignObject of the scene's svg;
//   - what the browser computes as the role and name of each control, which is what a screen reader is given;
//   - the pointer path: a click on each control, and a real pointer sequence through the driver's actions;
//   - the keyboard path: the order of Tab, Enter and Space on a button, the arrow keys on the range, and a choice in the scenario list;
//   - playback interrupted by a press of Next, and by routing to another panel and back, with what was ticking meanwhile;
//   - a switch of scenario, and where the focus is after it, after a route away and back, and after a failure;
//   - a failed write (the page-level fault injector) and the repair that follows, for a step and for a switch;
//   - the layout in the viewport, which `--viewport` makes narrow.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawn, spawnSync } from 'node:child_process'
import { serve } from '../../../perf/lib/chrome-session.mjs'

const root = path.resolve(import.meta.dirname, '../../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const browser = argument('--browser')
if (!['firefox', 'safari'].includes(browser))
  throw new Error('usage: --browser firefox|safari [--viewport WxH] [--out FILE]')
const viewport = argument('--viewport')?.split('x').map(Number) ?? null
const out = path.resolve(
  argument('--out') ??
    path.join(root, `dess/demo-panel/p4/evidence/${browser}-interaction${viewport ? '-narrow' : ''}.json`)
)
const PANEL = 'panel-service-simulator'
const OTHER = 'panel-petri-net'
const ELEMENT = 'element-6066-11e4-a52e-4f735466cecf'
const KEYS = { enter: '', tab: '', right: '', left: '', down: '', up: '', space: ' ', escape: '' }

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const { server, port } = await serve(path.join(root, 'target/demo-stage'))
const PORT = 4444 + Math.floor(Math.random() * 1000)
let driver
let capabilities
let version
if (browser === 'firefox') {
  const FIREFOX = process.env.FIREFOX ?? '/Applications/Firefox.app/Contents/MacOS/Firefox'
  const cache = path.join(os.homedir(), 'Library/Caches/.wasm-pack')
  const geckodriver =
    process.env.GECKODRIVER ??
    fs
      .readdirSync(cache)
      .filter(d => d.startsWith('geckodriver'))
      .map(d => path.join(cache, d, 'geckodriver'))
      .find(f => fs.existsSync(f))
  if (!geckodriver || !fs.existsSync(FIREFOX))
    throw new Error('Firefox or geckodriver was not found: set FIREFOX and GECKODRIVER')
  version = spawnSync(FIREFOX, ['--version'], { encoding: 'utf8' }).stdout.trim()
  driver = spawn(geckodriver, ['--port', String(PORT)], { stdio: 'ignore' })
  capabilities = {
    browserName: 'firefox',
    'moz:firefoxOptions': { binary: FIREFOX, args: ['-headless', '-width', '1500', '-height', '1000'] }
  }
} else {
  version = `Safari ${spawnSync('defaults', ['read', '/Applications/Safari.app/Contents/Info', 'CFBundleShortVersionString'], { encoding: 'utf8' }).stdout.trim()} on macOS ${spawnSync('sw_vers', ['-productVersion'], { encoding: 'utf8' }).stdout.trim()}`
  driver = spawn('safaridriver', ['--port', String(PORT)], { stdio: 'ignore' })
  capabilities = { browserName: 'safari' }
}
const call = async (method, route, body) => {
  const response = await fetch(`http://127.0.0.1:${PORT}${route}`, {
    method,
    headers: { 'content-type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body)
  })
  const json = await response.json()
  if (json.value?.error) throw new Error(`${json.value.error}: ${String(json.value.message).split('\n')[0]}`)
  return json.value
}
for (let i = 0; i < 100; i++) {
  try {
    await fetch(`http://127.0.0.1:${PORT}/status`)
    break
  } catch {
    await new Promise(r => setTimeout(r, 100))
  }
}

const observations = []
const note = (name, seen) => {
  observations.push({ name, seen })
  console.log(`${name}: ${typeof seen === 'string' ? seen : JSON.stringify(seen)}`)
}
/** Looks at one thing. If the step cannot be done it is recorded as that, and the visit goes on. */
const observe = async (name, look) => {
  try {
    note(name, await look())
  } catch (error) {
    note(name, `could not be done: ${String(error.message ?? error).split('\n')[0]}`)
  }
}
const sleep = ms => new Promise(r => setTimeout(r, ms))

let session
try {
  session = (await call('POST', '/session', { capabilities: { alwaysMatch: capabilities } })).sessionId
  const s = route => `/session/${session}${route}`
  const run = (script, ...args) => call('POST', s('/execute/sync'), { script, args })
  const wait = async (expression, ms = 15000) => {
    for (let waited = 0; waited < ms; waited += 100) {
      if (await run(`return Boolean(${expression})`)) return true
      await sleep(100)
    }
    return false
  }
  const find = async css => (await call('POST', s('/element'), { using: 'css selector', value: css }))[ELEMENT]
  const click = async css => call('POST', s(`/element/${await find(css)}/click`), {})
  const focus = css => run(`document.querySelector(arguments[0]).focus()`, css)
  const active = () => run(`return document.activeElement?.id || document.activeElement?.tagName || null`)
  const press = async (...names) => {
    const actions = names.flatMap(n => [
      { type: 'keyDown', value: KEYS[n] ?? n },
      { type: 'keyUp', value: KEYS[n] ?? n }
    ])
    await call('POST', s('/actions'), { actions: [{ type: 'key', id: 'keyboard', actions }] })
    await call('DELETE', s('/actions'))
  }
  const text = id => run(`return document.getElementById(arguments[0]).textContent.trim()`, id)
  const snapshot = () => text('dess-i-snapshot')
  /** The snapshot once it differs from `before`, and how long it took, since a browser may answer a click before the page has drawn. */
  const changedFrom = async before => {
    const started = Date.now()
    while (Date.now() - started < 2000) {
      const now = await snapshot()
      if (now !== before) return { snapshot: now, milliseconds: Date.now() - started }
      await sleep(50)
    }
    return { snapshot: await snapshot(), milliseconds: null }
  }
  const setSpeed = ms =>
    run(
      `const e = document.getElementById('dess-speed'); e.value = arguments[0]; e.dispatchEvent(new Event('change', { bubbles: true })); return e.value`,
      String(ms)
    )
  // The menu's groups are collapsed, so its items are not in view for a pointer. The gallery routes on a click, and the page makes the same one.
  const menu = target =>
    run(`document.querySelector('#menu .menu-item[data-target="' + arguments[0] + '"]').click()`, target)
  const display = () => run(`return document.getElementById('dess-i-display').textContent.trim()`)
  const errorText = () =>
    run(`const e = document.getElementById('dess-error'); return e.hidden ? null : e.textContent.trim()`)
  const play = () =>
    run(
      `const b = document.getElementById('dess-play'); return b.textContent.trim() + ' / aria-pressed ' + b.getAttribute('aria-pressed')`
    )

  // Firefox is started at 1500 by 1000. Safari opens a window of its own size, so it is given the same one unless a narrow viewport was asked for.
  const size = viewport ?? (browser === 'safari' ? [1500, 1000] : null)
  if (size) await call('POST', s('/window/rect'), { x: 0, y: 0, width: size[0], height: size[1] }).catch(() => {})
  await call('POST', s('/url'), { url: `http://127.0.0.1:${port}/` })
  await wait(`document.getElementById('menu')`)
  // The fault injector is off until it is armed. It has to be in the page before the panel is built, so the panel is not selected yet.
  await run(fs.readFileSync(path.join(root, 'perf/page/fault-injector.js'), 'utf8'))
  await menu(PANEL)
  const ready = await wait(`document.getElementById('${PANEL}')?.dataset.panelState === 'ready'`)
  note('the panel is built when it is selected', ready ? 'yes' : 'no: it did not come up in 15 seconds')
  if (!ready) throw new Error('the panel did not come up')
  const window = await call('GET', s('/window/rect'))
  note('the window', {
    asked: viewport,
    got: [window.width, window.height],
    inner: await run(`return [innerWidth, innerHeight]`)
  })

  await observe('the controls are in the scene', () =>
    run(
      `return ['dess-prev','dess-next','dess-play','dess-restart','dess-scenario','dess-seek','dess-speed'].map(id => id + ': ' + Boolean(document.getElementById(id)?.closest('foreignObject')))`
    )
  )
  await observe('the role and name that the browser computes for each control', async () => {
    const seen = {}
    for (const id of [
      'dess-prev',
      'dess-next',
      'dess-play',
      'dess-restart',
      'dess-repair',
      'dess-scenario',
      'dess-seek',
      'dess-speed'
    ]) {
      const element = await find(`#${id}`)
      const role = await call('GET', s(`/element/${element}/computedrole`)).catch(
        e => `(${String(e.message).split(':')[0]})`
      )
      const label = await call('GET', s(`/element/${element}/computedlabel`)).catch(
        e => `(${String(e.message).split(':')[0]})`
      )
      seen[id] = `${role} / ${label}`
    }
    return seen
  })

  await observe('the server boxes are the colour of their state (the :has() rules)', async () => {
    const colours = { idle: 'rgb(238, 244, 255)', busy: 'rgb(187, 247, 208)', down: 'rgb(254, 202, 202)' }
    for (let i = 0; i < 4; i++) {
      const before = await snapshot()
      await run(`document.getElementById('dess-next').click()`)
      await changedFrom(before)
    }
    const servers = await run(
      `const svg = document.querySelector('#dess-stage svg'); return [...svg.querySelectorAll('[data-ss-key^="S"]')].filter(g => /^S\\d+$/.test(g.getAttribute('data-ss-key'))).map(g => { const k = g.getAttribute('data-ss-key'); const s = ['busy','down','idle'].find(c => g.classList.contains('ss-' + c)); return { key: k, state: s, fill: getComputedStyle(svg.querySelector('.svg-dom-graph-content > g[aria-label^="' + k + '."] > rect')).fill } })`
    )
    return { snapshot: await snapshot(), servers, allMatch: servers.every(x => x.fill === colours[x.state]) }
  })
  await run(`document.getElementById('dess-restart').click()`)

  // The pointer path.
  await observe('the pointer: three clicks of Next, then one of Previous', async () => {
    const seen = []
    for (const id of ['next', 'next', 'next', 'prev']) {
      const before = await snapshot()
      await click(`#dess-${id}`)
      seen.push({ clicked: id, before, ...(await changedFrom(before)) })
    }
    return seen
  })
  await observe('the pointer: a real pointer sequence on Next (move, press, release)', async () => {
    const before = await snapshot()
    const element = await find('#dess-next')
    await call('POST', s('/actions'), {
      actions: [
        {
          type: 'pointer',
          id: 'mouse',
          parameters: { pointerType: 'mouse' },
          actions: [
            { type: 'pointerMove', duration: 0, origin: { [ELEMENT]: element }, x: 0, y: 0 },
            { type: 'pointerDown', button: 0 },
            { type: 'pointerUp', button: 0 }
          ]
        }
      ]
    })
    await call('DELETE', s('/actions'))
    return { before, ...(await changedFrom(before)) }
  })

  // The keyboard path.
  await observe('the keyboard: Enter and then Space on Next', async () => {
    await focus('#dess-next')
    const before = await snapshot()
    await press('enter')
    const afterEnter = await snapshot()
    await press('space')
    return { before, afterEnter, afterSpace: await snapshot(), focus: await active() }
  })
  await observe('the keyboard: where Tab goes from Next, eight times', async () => {
    await focus('#dess-next')
    const order = []
    for (let i = 0; i < 8; i++) {
      await press('tab')
      order.push(await active())
    }
    return order
  })
  await observe('the keyboard: the arrow keys on the range', async () => {
    await focus('#dess-seek')
    const before = await run(`return document.getElementById('dess-seek').value`)
    await press('right', 'right')
    const after = await run(`return document.getElementById('dess-seek').value`)
    // A seek is made over frames, so the snapshot is read once it has arrived at the number that was asked for.
    await wait(
      `document.getElementById('dess-i-snapshot').textContent.startsWith(document.getElementById('dess-seek').value + ' ')`,
      5000
    )
    return { before, after, snapshot: await snapshot(), text: await text('dess-seek-text') }
  })
  await observe('the keyboard: a choice in the scenario list', async () => {
    // On macOS a closed list opens its popup for the arrow keys in some browsers, so the choice is also tried by typing the first letter of another option.
    await focus('#dess-scenario')
    const labels = await run(
      `return [...document.getElementById('dess-scenario').options].map(o => o.value + ': ' + o.text)`
    )
    const read = () => run(`return document.getElementById('dess-scenario').value`)
    const before = await read()
    await press('down')
    const afterArrow = await read()
    const other = labels
      .map(l => l.split(': ')[1])
      .find(
        l =>
          l[0].toLowerCase() !== (labels.find(x => x.startsWith(before + ':')) ?? '').split(': ')[1]?.[0]?.toLowerCase()
      )
    await press(other[0].toLowerCase())
    await sleep(300)
    return {
      labels,
      before,
      afterArrow,
      typed: other[0].toLowerCase(),
      afterTyping: await read(),
      scenes: await run(`return document.querySelectorAll('#dess-stage svg').length`),
      focus: await active(),
      refusal: await errorText()
    }
  })
  await run(
    `const s = document.getElementById('dess-scenario'); s.value = 'E0'; s.dispatchEvent(new Event('change', { bubbles: true }))`
  )

  // Playback interrupted.
  await observe('Play, then a press of Next while it runs', async () => {
    await click('#dess-restart')
    await setSpeed(1000)
    await click('#dess-play')
    await sleep(1300)
    const playing = { snapshot: await snapshot(), button: await play() }
    await focus('#dess-next')
    await press('enter')
    const afterNext = { snapshot: await snapshot(), button: await play() }
    await sleep(1500)
    return { playing, afterNext, oneAndAHalfSecondsLater: await snapshot() }
  })
  await observe('Play, then the panel is routed away from and back', async () => {
    await click('#dess-restart')
    await click('#dess-play')
    await sleep(1300)
    const before = await snapshot()
    await menu(OTHER)
    await wait(`document.getElementById('${OTHER}')?.dataset.panelState === 'ready'`)
    const during = await run(`return document.getElementById('${PANEL}').classList.contains('active')`)
    await sleep(2500)
    await menu(PANEL)
    await sleep(200)
    return {
      before,
      panelActiveWhileAway: during,
      afterReturn: await snapshot(),
      button: await play(),
      note: 'a snapshot that is the same before and after means nothing ticked while the panel was hidden, and the button not pressed means that it did not resume'
    }
  })
  await observe('focus after a route away and back', async () => {
    await focus('#dess-next')
    const before = await active()
    await menu(OTHER)
    await menu(PANEL)
    await sleep(200)
    return { before, after: await active() }
  })

  await observe('a switch of scenario, the five-server scenario and back', async () => {
    const seen = []
    for (const id of ['five', 'E0']) {
      await run(
        `const s = document.getElementById('dess-scenario'); s.value = arguments[0]; s.dispatchEvent(new Event('change', { bubbles: true }))`,
        id
      )
      seen.push({
        id,
        scenes: await run(`return document.querySelectorAll('#dess-stage svg').length`),
        servers: await run(
          `return [...document.querySelectorAll('#dess-stage [data-ss-key]')].filter(g => /^S\\d+$/.test(g.getAttribute('data-ss-key'))).length`
        ),
        refusal: await errorText(),
        focus: await active()
      })
    }
    return seen
  })

  // Failures, made by the page-level injector: the nth write of the library fails once.
  await observe('a failed write during a step, and the repair', async () => {
    await click('#dess-restart')
    await focus('#dess-next')
    await run(`__faults.arm(1)`)
    await click('#dess-next')
    const report = await run(`return __faults.disarm()`)
    const failed = {
      display: await display(),
      error: await errorText(),
      snapshot: await snapshot(),
      focus: await active(),
      injection: report
    }
    await click('#dess-repair')
    return {
      failed,
      afterRepair: {
        display: await display(),
        error: await errorText(),
        snapshot: await snapshot(),
        focus: await active()
      }
    }
  })
  await observe('a failed write during a switch of scenario, and a second try', async () => {
    const was = await run(`return document.getElementById('dess-scenario').value`)
    await run(`__faults.arm(3)`)
    await run(
      `const s = document.getElementById('dess-scenario'); s.value = 'five'; s.dispatchEvent(new Event('change', { bubbles: true }))`
    )
    const report = await run(`return __faults.disarm()`)
    const failed = {
      list: await run(`return document.getElementById('dess-scenario').value`),
      scenes: await run(`return document.querySelectorAll('#dess-stage svg').length`),
      refusal: await errorText(),
      injection: report
    }
    await run(
      `const s = document.getElementById('dess-scenario'); s.value = 'five'; s.dispatchEvent(new Event('change', { bubbles: true }))`
    )
    return {
      was,
      failed,
      secondTry: {
        list: await run(`return document.getElementById('dess-scenario').value`),
        refusal: await errorText(),
        scenes: await run(`return document.querySelectorAll('#dess-stage svg').length`)
      }
    }
  })

  await observe('the layout in the viewport', async () => {
    await run(`document.getElementById('dess-stage').scrollIntoView()`)
    return run(`
      const box = e => { const b = e.getBoundingClientRect(); return [Math.round(b.left), Math.round(b.top), Math.round(b.width), Math.round(b.height)] }
      const controls = ['dess-prev', 'dess-next', 'dess-play', 'dess-restart', 'dess-scenario', 'dess-seek', 'dess-speed'].map(id => [id, box(document.getElementById(id))])
      return {
        inner: [innerWidth, innerHeight],
        pageScrollsSideways: document.documentElement.scrollWidth > innerWidth,
        stage: box(document.getElementById('dess-stage')),
        svg: box(document.querySelector('#dess-stage svg')),
        controlsOutsideTheViewportSideways: controls.filter(([, b]) => b[0] < 0 || b[0] + b[2] > innerWidth).map(([id]) => id),
        smallestControl: Math.min(...controls.map(([, b]) => Math.min(b[2], b[3])))
      }`)
  })
  await observe('a screenshot was kept', async () => {
    fs.mkdirSync(path.dirname(out), { recursive: true })
    fs.writeFileSync(out.replace(/\.json$/, '.png'), Buffer.from(await call('GET', s('/screenshot')), 'base64'))
    return path.relative(root, out.replace(/\.json$/, '.png'))
  })
} catch (error) {
  note('the visit stopped', String(error.message ?? error))
} finally {
  if (session) await call('DELETE', `/session/${session}`).catch(() => {})
  driver.kill()
  server.close()
}
fs.mkdirSync(path.dirname(out), { recursive: true })
fs.writeFileSync(
  out,
  `${JSON.stringify({ observed_at: new Date().toISOString(), browser: version, viewport, note: 'Observations, not a pass claim. Chrome is the release gate.', observations }, null, 2)}\n`
)
console.log(`\nrecorded in ${path.relative(root, out)}`)
