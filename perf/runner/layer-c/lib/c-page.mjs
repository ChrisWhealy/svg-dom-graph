// Opens the Layer C page in a headless Chrome of its own, served with the headers that make the browser's clock finer, and
// gives back what a runner needs to drive it, including a way to open a further tab for a case that has to be on its own.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { launch, serve } from '../../../lib/chrome-session.mjs'
import { wasmPackage } from '../../lib/harness-page.mjs'

const root = path.resolve(import.meta.dirname, '../../../..')
const tracker = fs.readFileSync(path.join(root, 'perf/page/listener-tracker.js'), 'utf8')

// Waits until the page says it is ready. The page's own clock gives the time to ready, from the navigation's start.
const ready = async page => {
  for (let waited = 0; waited < 30000; waited += 50) {
    if (await page.evaluate('Boolean(window.perfC)')) return page.evaluate('window.perfC.readyAt')
    await new Promise(resolve => setTimeout(resolve, 50))
  }
  throw new Error('the Layer C page did not start')
}

export const openLayerCPage = async ({ isolated = true } = {}) => {
  if (!fs.existsSync(path.join(wasmPackage, 'perf_harness_wasm.js'))) {
    throw new Error('the wasm package is not built: run `wasm-pack build perf-harness-wasm --target web --release --out-dir pkg`')
  }
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-layer-c-'))
  fs.copyFileSync(path.join(import.meta.dirname, '../page/index.html'), path.join(dir, 'index.html'))
  fs.cpSync(wasmPackage, path.join(dir, 'pkg'), { recursive: true })
  const { server, port } = await serve(dir, { isolated })
  const url = `http://127.0.0.1:${port}/`
  const chrome = await launch()

  const start = async page => {
    await page.send('Page.addScriptToEvaluateOnNewDocument', { source: tracker })
    await page.send('Page.navigate', { url })
    return ready(page)
  }
  const coldStartMs = await start(chrome)

  // A tab of its own, brought to the front, since only the tab in front receives a real wheel.
  const openTab = async () => {
    const tab = await chrome.newPage('about:blank')
    const readyAt = await start(tab)
    await tab.send('Page.bringToFront')
    tab.coldStartMs = readyAt
    return tab
  }
  const close = async () => {
    await chrome.close()
    server.close()
    fs.rmSync(dir, { recursive: true, force: true })
  }
  return { ...chrome, openTab, close, coldStartMs, crossOriginIsolated: await chrome.evaluate('crossOriginIsolated') }
}
