// What the gallery panel does in Firefox, looked at through geckodriver, and written down as observations (design D5).
//
//   node dess/demo-panel/p4/firefox-observe.mjs [--out dess/demo-panel/p4/evidence/firefox-observations.json]
//
// Chrome is the release gate and Firefox is not: nothing here is a pass claim, and no CI job depends on it. The script serves the staged gallery
// (`cargo run -p demo-server -- --build-only` first), starts a headless Firefox through the geckodriver that wasm-pack has cached, and does what a
// person would do in a short visit: load the panel, step, play for a moment, choose a scenario of another size, and read what the page shows. Each
// thing is recorded as what was seen, with the Firefox version. A step that cannot be done is recorded as that. It is a look, and not a test suite.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawn, spawnSync } from 'node:child_process'
import { serve } from '../../../perf/lib/chrome-session.mjs'

const root = path.resolve(import.meta.dirname, '../../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const out = path.resolve(argument('--out') ?? path.join(root, 'dess/demo-panel/p4/evidence/firefox-observations.json'))
const FIREFOX = process.env.FIREFOX ?? '/Applications/Firefox.app/Contents/MacOS/firefox'
const cache = path.join(os.homedir(), 'Library/Caches/.wasm-pack')
const driver =
  process.env.GECKODRIVER ??
  fs
    .readdirSync(cache)
    .filter(d => d.startsWith('geckodriver'))
    .map(d => path.join(cache, d, 'geckodriver'))
    .find(f => fs.existsSync(f))
if (!driver || !fs.existsSync(FIREFOX)) {
  console.error(
    'Firefox or geckodriver was not found: set FIREFOX and GECKODRIVER, or run `wasm-pack test --headless --firefox service-sim` once so that wasm-pack caches the driver'
  )
  process.exit(2)
}
const version = spawnSync(FIREFOX, ['--version'], { encoding: 'utf8' }).stdout.trim()

const { server, port } = await serve(path.join(root, 'target/demo-stage'))
const PORT = 4444 + Math.floor(Math.random() * 1000)
const geckodriver = spawn(driver, ['--port', String(PORT)], { stdio: 'ignore' })
const call = async (method, route, body) => {
  const response = await fetch(`http://127.0.0.1:${PORT}${route}`, {
    method,
    headers: { 'content-type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined
  })
  const json = await response.json()
  if (json.value?.error) throw new Error(`${json.value.error}: ${json.value.message}`)
  return json.value
}
for (let i = 0; i < 50; i++) {
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
let session
try {
  const created = await call('POST', '/session', {
    capabilities: {
      alwaysMatch: {
        browserName: 'firefox',
        'moz:firefoxOptions': { binary: FIREFOX, args: ['-headless', '-width', '1500', '-height', '1000'] }
      }
    }
  })
  session = created.sessionId
  const run = (script, ...args) => call('POST', `/session/${session}/execute/sync`, { script, args })
  const wait = async (script, ms = 15000) => {
    for (let waited = 0; waited < ms; waited += 100) {
      if (await run(`return Boolean(${script})`)) return true
      await new Promise(r => setTimeout(r, 100))
    }
    return false
  }
  await call('POST', `/session/${session}/url`, { url: `http://127.0.0.1:${port}/#panel-service-simulator` })
  const ready = await wait(`document.getElementById('panel-service-simulator')?.dataset.panelState === 'ready'`)
  note('the panel is built when it is selected', ready ? 'yes' : 'no: it did not come up in 15 seconds')
  if (!ready) throw new Error('the panel did not come up')
  note('the first snapshot', await run(`return document.getElementById('dess-snapshot').textContent`))
  note(
    'the controls are in the scene',
    (await run(
      `return ['dess-prev','dess-next','dess-play','dess-scenario','dess-seek'].every(id => document.getElementById(id).closest('foreignObject'))`
    ))
      ? "yes, in a foreignObject of the scene's svg"
      : 'no'
  )
  const colours = { idle: 'rgb(238, 244, 255)', busy: 'rgb(187, 247, 208)', down: 'rgb(254, 202, 202)' }
  const servers = () =>
    run(
      `const svg = document.querySelector('#dess-stage svg'); return [...svg.querySelectorAll('[data-ss-key^="S"]')].filter(g => /^S\\d+$/.test(g.getAttribute('data-ss-key'))).map(g => { const k = g.getAttribute('data-ss-key'); const s = ['busy','down','idle'].find(c => g.classList.contains('ss-' + c)); return { key: k, state: s, fill: getComputedStyle(svg.querySelector('.svg-dom-graph-content > g[aria-label^="' + k + '."] > rect')).fill } })`
    )
  for (let i = 0; i < 4; i++) await run(`document.getElementById('dess-next').click()`)
  const afterFour = await servers()
  note('the walkthrough after four clicks of Next', {
    snapshot: await run(`return document.getElementById('dess-i-snapshot').textContent`),
    servers: afterFour
  })
  note(
    'the server boxes are the colour of their state (the :has() rules)',
    afterFour.every(s => s.fill === colours[s.state]) ? 'yes' : `no: ${JSON.stringify(afterFour)}`
  )
  note(
    'the data node holds the eight numbers',
    await run(
      `const svg = document.querySelector('#dess-stage svg'); const nodes = [...svg.querySelectorAll('.svg-dom-graph-content > g:not(.ss-overlay)')]; return [...nodes.at(-1).querySelectorAll('text')].map(t => t.textContent.trim())`
    )
  )
  await run(`document.getElementById('dess-play').click()`)
  await new Promise(r => setTimeout(r, 2600))
  await run(`document.getElementById('dess-play').click()`)
  note('Play for 2.6 seconds at 1 Hz, then Pause', {
    snapshot: await run(`return document.getElementById('dess-i-snapshot').textContent`),
    button: await run(
      `return document.getElementById('dess-play').textContent + ' / ' + document.getElementById('dess-play').getAttribute('aria-pressed')`
    )
  })
  for (const id of ['E3-NOOP', 'five', 'E0']) {
    await run(
      `const s = document.getElementById('dess-scenario'); s.value = arguments[0]; s.dispatchEvent(new Event('change', { bubbles: true }))`,
      id
    )
    note(
      `a switch to ${id}`,
      await run(
        `return { snapshot: document.getElementById('dess-i-snapshot').textContent, scenes: document.querySelectorAll('#dess-stage svg').length, servers: [...document.querySelectorAll('#dess-stage [data-ss-key]')].filter(g => /^S\\d+$/.test(g.getAttribute('data-ss-key'))).length, hosts: [...document.querySelectorAll('[id^="dess-host-"]')].map(h => h.id + (h.hasAttribute('inert') ? ' (staged)' : '')), refusal: document.getElementById('dess-error').hidden ? 'none' : document.getElementById('dess-error').textContent }`
      )
    )
  }
  await run(`document.getElementById('dess-stage').scrollIntoView()`)
  const layout = await run(
    `const r = e => { const b = document.getElementById(e).getBoundingClientRect(); return [Math.round(b.left), Math.round(b.top), Math.round(b.width), Math.round(b.height)] }; return { toolbar: r('dess-toolbar'), side: r('dess-side'), svg: (() => { const b = document.querySelector('#dess-stage svg').getBoundingClientRect(); return [Math.round(b.left), Math.round(b.top), Math.round(b.width), Math.round(b.height)] })() }`
  )
  note('the controls and inspector are inside the scene (left, top, width, height)', layout)
  const screenshot = await call('GET', `/session/${session}/screenshot`)
  fs.writeFileSync(out.replace(/\.json$/, '.png'), Buffer.from(screenshot, 'base64'))
  note('a screenshot was kept', path.relative(root, out.replace(/\.json$/, '.png')))
} catch (error) {
  note('the visit stopped', String(error.message ?? error))
} finally {
  if (session) await call('DELETE', `/session/${session}`).catch(() => {})
  geckodriver.kill()
  server.close()
}
fs.mkdirSync(path.dirname(out), { recursive: true })
fs.writeFileSync(
  out,
  `${JSON.stringify({ observed_at: new Date().toISOString(), browser: version, note: 'Observations, not a pass claim. Chrome is the release gate.', observations }, null, 2)}\n`
)
console.log(`\nrecorded in ${path.relative(root, out)}`)
