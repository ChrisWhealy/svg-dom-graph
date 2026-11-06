// Phase 0 spike for the performance harness (design-specs/performance_harness.pdf, section 10).
// It answers four questions about a built wasm page, using Chrome over the DevTools Protocol.
//
//   1. What timer resolution does the page get, with and without cross-origin isolation?
//   2. Which counters does Chrome's `Performance.getMetrics` give?
//   3. Does a wrapper on `addEventListener` agree with Chrome's own listener count?
//   4. What do the element ids, counters and listeners do when the page builds a second scene?
//
// Run it from the repository root with the fixture built, and give it the directory that holds `index.html`:
//
//   node perf/spike/phase0-spike.mjs cdp-test-fixture [isolated]
//
// Set CHROME to the path of a Chrome binary if it is not the default macOS install. It needs Node 22 or later.
import http from 'node:http'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawn } from 'node:child_process'

const dir = process.argv[2]
const isolated = process.argv[3] === 'isolated'
const chromePath = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'

if (!dir) {
  console.error('usage: node phase0-spike.mjs <directory with index.html> [isolated]')
  process.exit(2)
}

const types = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.wasm': 'application/wasm'
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Serves the directory. COOP and COEP headers are added only when asked, so both timer resolutions can be compared.
const server = http
  .createServer((req, res) => {
    const file = path.join(dir, req.url === '/' ? 'index.html' : req.url.split('?')[0])

    if (!fs.existsSync(file)) return res.writeHead(404).end()

    const headers = {
      'content-type': types[path.extname(file)] ?? 'application/octet-stream'
    }

    if (isolated) {
      headers['cross-origin-opener-policy'] = 'same-origin'
      headers['cross-origin-embedder-policy'] = 'require-corp'
    }

    res.writeHead(200, headers).end(fs.readFileSync(file))
  })
  .listen(0)

const port = server.address().port
const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-spike-'))
const chrome = spawn(
  chromePath,
  ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', 'about:blank'],
  { stdio: ['ignore', 'ignore', 'pipe'] }
)

const wsUrl = await new Promise(resolve => {
  let log = ''

  chrome.stderr.on('data', chunk => {
    log += chunk
    const found = log.match(/ws:\/\/\S+/)
    if (found) resolve(found[0])
  })
})

const host = wsUrl.match(/ws:\/\/([^/]+)/)[1]
const tabs = await (await fetch(`http://${host}/json`)).json()
const socket = new WebSocket(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl)

await new Promise(resolve => (socket.onopen = resolve))

let nextId = 0
const pending = new Map()

socket.onmessage = event => {
  const message = JSON.parse(event.data)

  if (message.id && pending.has(message.id)) {
    pending.get(message.id)(message)
    pending.delete(message.id)
  }
}

const send = (method, params = {}) =>
  new Promise(resolve => {
    const id = ++nextId
    pending.set(id, resolve)
    socket.send(JSON.stringify({ id, method, params }))
  })

const evaluate = async expression => {
  const reply = await send('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true
  })

  return reply.result.result?.value
}

const metrics = async () => {
  const reply = await send('Performance.getMetrics')

  return Object.fromEntries(reply.result.metrics.map(metric => [metric.name, metric.value]))
}

const pick = m => ({
  Nodes: m.Nodes,
  JSEventListeners: m.JSEventListeners,
  JSHeapUsedSize: m.JSHeapUsedSize
})

await send('Performance.enable')
await send('Page.enable')
await send('HeapProfiler.enable')

// A naive tally, installed before any page script runs. It counts calls, not effective registrations, which is the
// weakness question 3 is checking.
await send('Page.addScriptToEvaluateOnNewDocument', {
  source: `
    window.__tally = { add: 0, remove: 0 }
    const add = EventTarget.prototype.addEventListener
    const remove = EventTarget.prototype.removeEventListener
    EventTarget.prototype.addEventListener = function (type, listener, options) {
      __tally.add++
      return add.call(this, type, listener, options)
    }
    EventTarget.prototype.removeEventListener = function (type, listener, options) {
      __tally.remove++
      return remove.call(this, type, listener, options)
    }`
})
await send('Page.navigate', { url: `http://127.0.0.1:${port}/` })
await new Promise(resolve => setTimeout(resolve, 2500))

const out = {
  isolated,
  crossOriginIsolated: await evaluate('crossOriginIsolated')
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Question 1: the smallest non-zero step of performance.now() over a tight loop.
out.timer_resolution_us = await evaluate(`(() => {
  let smallest = Infinity
  let last = performance.now()
  for (let i = 0; i < 2e6; i++) {
    const now = performance.now()
    const step = now - last
    if (step > 0 && step < smallest) smallest = step
    last = now
  }
  return smallest * 1000
})()`)
out.user_agent = await evaluate('navigator.userAgent')

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Question 2: which counters exist.
const first = await metrics()

out.metric_names = Object.keys(first).filter(name =>
  /Nodes|Listeners|JSHeap|LayoutCount|RecalcStyleCount|ScriptDuration/.test(name)
)
out.before = pick(first)

const describePage = () =>
  evaluate(`({
  scene_children: document.querySelector('#diagram').childElementCount,
  elements: document.querySelectorAll('#diagram *').length,
  ids: [...document.querySelectorAll('#diagram [id]')].map(element => element.id),
  tally: { add: __tally.add, remove: __tally.remove, net: __tally.add - __tally.remove },
})`)
out.first_scene = await describePage()

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Question 4: the fixture's `run` builds a scene on the same <svg>. Calling it again is a rebuild without a reload.
out.second_run = await evaluate(
  `(() => { try { window.fixture.run(); return 'ok' } catch (e) { return 'threw: ' + e } })()`
)
out.second_scene = await describePage()
out.after_second_run_before_gc = pick(await metrics())

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Question 3: compare the tally with Chrome's count once a collection has run.
await send('HeapProfiler.collectGarbage')
out.after_second_run_after_gc = pick(await metrics())

await evaluate(`document.querySelector('#diagram').remove()`)
await send('HeapProfiler.collectGarbage')
await new Promise(resolve => setTimeout(resolve, 300))
out.after_removing_the_svg = pick(await metrics())

console.log(JSON.stringify(out, null, 2))

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Chrome may still be writing to its profile directory, so the clean-up waits for it to exit and then retries.
const exited = new Promise(resolve => chrome.once('exit', resolve))

chrome.kill()
await exited
server.close()

fs.rmSync(profile, {
  recursive: true,
  force: true,
  maxRetries: 5,
  retryDelay: 200
})

process.exit(0)
