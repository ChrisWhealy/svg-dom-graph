// Starts a static file server and a headless Chrome, and gives back a small CDP client.
//
// The harness scripts share it, so each of them says only what it measures. It needs Node 22 or later. Set CHROME to the
// path of a Chrome binary if it is not the default macOS install.
import http from 'node:http'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawn } from 'node:child_process'

const types = { '.html': 'text/html', '.js': 'text/javascript', '.wasm': 'application/wasm' }

// Serves `dir`. COOP and COEP headers are sent only when `isolated` is set, which is what gives a finer timer.
export const serve = (dir, { isolated = false } = {}) =>
  new Promise(resolve => {
    const server = http
      .createServer((req, res) => {
        const file = path.join(dir, req.url === '/' ? 'index.html' : req.url.split('?')[0])
        if (!fs.existsSync(file)) return res.writeHead(404).end()
        const headers = { 'content-type': types[path.extname(file)] ?? 'application/octet-stream' }
        if (isolated) {
          headers['cross-origin-opener-policy'] = 'same-origin'
          headers['cross-origin-embedder-policy'] = 'require-corp'
        }
        res.writeHead(200, headers).end(fs.readFileSync(file))
      })
      .listen(0, () => resolve({ server, port: server.address().port }))
  })

export const launch = async () => {
  const chromePath = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'
  const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-chrome-'))
  const chrome = spawn(
    chromePath,
    ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', 'about:blank'],
    { stdio: ['ignore', 'ignore', 'pipe'] },
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
    const reply = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })
    if (reply.result.exceptionDetails) throw new Error(reply.result.exceptionDetails.exception?.description)
    return reply.result.result?.value
  }
  const metrics = async () => {
    const reply = await send('Performance.getMetrics')
    return Object.fromEntries(reply.result.metrics.map(metric => [metric.name, metric.value]))
  }
  const collectGarbage = async () => {
    await send('HeapProfiler.collectGarbage')
    await new Promise(resolve => setTimeout(resolve, 100))
  }
  const close = async () => {
    // Chrome may still be writing to its profile directory, so the clean-up waits for it to exit and then retries.
    const exited = new Promise(resolve => chrome.once('exit', resolve))
    chrome.kill()
    await exited
    fs.rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 })
  }

  await send('Performance.enable')
  await send('Page.enable')
  await send('HeapProfiler.enable')
  return { send, evaluate, metrics, collectGarbage, close }
}
