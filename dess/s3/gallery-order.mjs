// S3: the order test that S0 ran on a copy of the gallery's markup, run on the real gallery.
//
//   node dess/s3/gallery-order.mjs [--stage DIR] [--out FILE]
//
// The controller is told that a panel was hidden by a MutationObserver on the stage, and it tests whether it may draw with
// `checkVisibility()` at the entry of every callback. This checks, on the gallery that the demo really serves, what S0 measured on a
// page with the same markup and rules: that a timer queued before a panel is hidden runs after the hint, and finds the panel hidden,
// though the panel is still in the document. The panel is switched by the gallery's own `hashchange` route, so it is the gallery's
// `selectDemo` that toggles the classes.
import fs from 'node:fs'
import path from 'node:path'
import { launch, serve } from '../../perf/lib/chrome-session.mjs'
import { pause } from '../../perf/lib/input.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const stage = path.resolve(argument('--stage') ?? path.join(root, 'target/demo-stage'))
const out = argument('--out')

let failures = 0
const check = (ok, text) => {
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
  if (!ok) failures += 1
}

const { server, port } = await serve(stage)
const chrome = await launch()
const evidence = { checks: [] }
try {
  await chrome.send('Runtime.enable')
  await chrome.send('Page.navigate', { url: `http://127.0.0.1:${port}/` })
  const ready = async () => chrome.evaluate(`!!document.querySelector('#stage .section.active')`)
  for (let waited = 0; waited < 20000 && !(await ready()); waited += 50) await pause(50)
  check(await ready(), 'the real gallery is up with a panel shown')

  // A probe in the shown panel, and the ids of two panels to switch between.
  const setup = await chrome.evaluate(`(() => {
    const shown = document.querySelector('#stage .section.active')
    const other = [...document.querySelectorAll('#menu .menu-item')].map(i => i.dataset.target).find(t => t !== shown.id)
    const probe = document.createElementNS('http://www.w3.org/2000/svg', 'svg')
    probe.setAttribute('id', 's3-probe'); probe.setAttribute('width', '40'); probe.setAttribute('height', '40')
    shown.appendChild(probe)
    window.__s3 = { log: [], shown: shown.id, other }
    return { shown: shown.id, other }
  })()`)
  check(Boolean(setup.other), `two panels to switch between (${setup.shown} and ${setup.other})`)

  const visible = () => chrome.evaluate(`document.getElementById('s3-probe').checkVisibility()`)
  check((await visible()) === true, 'the probe in the shown panel is visible')

  // One task: the hint is installed, a timer is queued, and then the gallery's own route hides the panel. The hint is a microtask of
  // this task, so it comes before the timer, which finds the panel hidden although the probe is still in the document.
  const hide = await chrome.evaluate(`(() => {
    const s = window.__s3
    const probe = document.getElementById('s3-probe')
    const observer = new MutationObserver(() => s.log.push('hint:visible=' + probe.checkVisibility()))
    observer.observe(document.getElementById('stage'), { attributes: true, subtree: true, attributeFilter: ['class'] })
    s.observer = observer
    setTimeout(() => s.log.push('timer:visible=' + probe.checkVisibility() + ',connected=' + probe.isConnected), 0)
    history.replaceState(null, '', '#' + s.other)
    window.dispatchEvent(new Event('hashchange'))
    return { syncVisible: probe.checkVisibility(), shownNow: document.querySelector('#stage .section.active')?.id }
  })()`)
  check(
    hide.syncVisible === false && hide.shownNow === setup.other,
    `the gallery's own route hid the panel in the same task (${JSON.stringify(hide)})`
  )
  await pause(200)
  const log = await chrome.evaluate(`window.__s3.log`)
  evidence.hide = { log }
  check(log[0] === 'hint:visible=false', `the hint came first and found the panel hidden (${log[0]})`)
  check(
    log[1] === 'timer:visible=false,connected=true',
    `a timer queued before the hide ran after it, found the panel hidden, and the probe still connected (${log[1]})`
  )

  // Showing again is a hint, and the panel is visible, and nothing says to resume.
  await chrome.evaluate(
    `(() => { window.__s3.log.length = 0; history.replaceState(null, '', '#' + window.__s3.shown); window.dispatchEvent(new Event('hashchange')) })()`
  )
  await pause(100)
  const shownLog = await chrome.evaluate(`window.__s3.log`)
  check(
    shownLog.length === 1 && shownLog[0] === 'hint:visible=true',
    `showing the panel again is a hint, and the probe is visible (${shownLog})`
  )
  evidence.show = { log: shownLog }
  await chrome.evaluate(`window.__s3.observer.disconnect()`)
} finally {
  await chrome.close()
  server.close()
}
console.log(
  failures === 0
    ? 'the real gallery orders the hint, the timer and the test as the design needs'
    : `${failures} checks failed`
)
if (out) fs.writeFileSync(path.resolve(root, out), `${JSON.stringify(evidence, null, 2)}\n`)
process.exit(failures === 0 ? 0 : 1)
