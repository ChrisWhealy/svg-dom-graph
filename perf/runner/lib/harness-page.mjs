// Opens the Layer B page in a headless Chrome of its own, ready for a runner to drive.
//
// The page is served from a temporary copy of the page and the built wasm package, with the headers that make the browser's
// clock finer. The effective-listener tracker is injected before any page script runs, so that it sees every registration.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { launch, serve } from '../../lib/chrome-session.mjs'

const root = path.resolve(import.meta.dirname, '../../..')

export const wasmPackage = path.join(root, 'perf-harness-wasm/pkg')

export const openHarnessPage = async ({ isolated = true } = {}) => {
  if (!fs.existsSync(path.join(wasmPackage, 'perf_harness_wasm.js'))) {
    throw new Error('the wasm package is not built: run `wasm-pack build perf-harness-wasm --target web --release --out-dir pkg`')
  }
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'perf-page-'))
  fs.copyFileSync(path.join(import.meta.dirname, '../page/index.html'), path.join(dir, 'index.html'))
  fs.cpSync(wasmPackage, path.join(dir, 'pkg'), { recursive: true })

  const { server, port } = await serve(dir, { isolated })
  const chrome = await launch()
  await chrome.send('Page.addScriptToEvaluateOnNewDocument', {
    source: fs.readFileSync(path.join(root, 'perf/page/listener-tracker.js'), 'utf8'),
  })
  await chrome.send('Page.navigate', { url: `http://127.0.0.1:${port}/` })
  for (let waited = 0; waited < 20000; waited += 100) {
    if (await chrome.evaluate('Boolean(window.perfHarness)')) break
    await new Promise(resolve => setTimeout(resolve, 100))
  }
  if (!(await chrome.evaluate('Boolean(window.perfHarness)'))) throw new Error('the harness page did not start')

  const close = async () => {
    await chrome.close()
    server.close()
    fs.rmSync(dir, { recursive: true, force: true })
  }
  return { ...chrome, close, crossOriginIsolated: await chrome.evaluate('crossOriginIsolated') }
}
