// Tests listener-tracker.js in a real Chrome. Run it from anywhere: `node perf/page/test-listener-tracker.mjs`.
//
// Each case follows one rule of the DOM's "add an event listener", and the last case checks the tally against Chrome's
// own `JSEventListeners` after a collection, which is the independent count that the tracker has to agree with.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { serve, launch } from '../lib/chrome-session.mjs'

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'listener-test-'))
fs.writeFileSync(path.join(dir, 'index.html'), '<!doctype html><body><div id="host"></div></body>')
const { server, port } = await serve(dir)
const chrome = await launch()
await chrome.send('Page.addScriptToEvaluateOnNewDocument', {
  source: fs.readFileSync(new URL('./listener-tracker.js', import.meta.url), 'utf8'),
})
await chrome.send('Page.navigate', { url: `http://127.0.0.1:${port}/` })
await new Promise(resolve => setTimeout(resolve, 500))

let failures = 0
// Each case runs in a function of its own, so `const` names cannot collide, and it counts only the registrations that it
// made itself. `mine()` is the tally minus what the page held when the case began.
const expect = async (label, body, expected) => {
  const expression = `(() => {
    const start = window.__listeners.snapshot().total
    const mine = () => window.__listeners.snapshot().total - start
    const t = document.createElement("div")
    document.body.append(t)
    const f = () => {}
    const g = () => {}
    ${body}
  })()`
  const actual = await chrome.evaluate(expression)
  const ok = JSON.stringify(actual) === JSON.stringify(expected)
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${label}${ok ? '' : `: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`}`)
}
const total = 'window.__listeners.snapshot().total'

await expect('the same listener added twice counts once', 't.addEventListener("click", f); t.addEventListener("click", f); return mine()', 1)
await expect(
  'a different capture flag is a different registration',
  't.addEventListener("click", f); t.addEventListener("click", f, true); return mine()',
  2,
)
await expect(
  'passive does not change identity',
  't.addEventListener("click", f, true); t.addEventListener("click", f, { capture: true, passive: true }); return mine()',
  1,
)
await expect(
  'removing with the wrong capture flag removes nothing, and the right one removes it',
  't.addEventListener("click", f, true); t.removeEventListener("click", f, { capture: false }); const a = mine(); t.removeEventListener("click", f, true); return [a, mine()]',
  [1, 0],
)
await expect('removing something never added changes nothing', 't.removeEventListener("click", g); return mine()', 0)
await expect(
  'a different type is a different registration',
  't.addEventListener("click", f); t.addEventListener("keydown", f); return mine()',
  2,
)
await expect(
  'a different callback is a different registration',
  't.addEventListener("click", f); t.addEventListener("click", g); return mine()',
  2,
)
await expect('a null callback is ignored', 't.addEventListener("click", null); return mine()', 0)
await expect(
  'a handleEvent object counts once',
  'const h = { handleEvent() {} }; t.addEventListener("click", h); t.addEventListener("click", h); return mine()',
  1,
)
await expect(
  'a once listener stops counting after it fires, and a second dispatch changes nothing',
  't.addEventListener("click", f, { once: true }); const a = mine(); t.dispatchEvent(new Event("click")); const b = mine(); t.dispatchEvent(new Event("click")); return [a, b, mine()]',
  [1, 0, 0],
)
await expect(
  'a once listener added again after it fired is a new registration',
  't.addEventListener("click", f, { once: true }); t.dispatchEvent(new Event("click")); t.addEventListener("click", f, { once: true }); return mine()',
  1,
)
await expect(
  'a listener whose signal aborts stops counting',
  'const c = new AbortController(); t.addEventListener("click", f, { signal: c.signal }); const a = mine(); c.abort(); return [a, mine()]',
  [1, 0],
)
await expect(
  'a listener with an already-aborted signal is never counted',
  'const c = new AbortController(); c.abort(); t.addEventListener("click", f, { signal: c.signal }); return mine()',
  0,
)
await expect(
  'a listener on a detached node counts as detached, and as connected once attached',
  'const d = document.createElement("div"); d.addEventListener("click", f); const a = window.__listeners.snapshot(); document.body.append(d); const b = window.__listeners.snapshot(); return [a.detached >= 1, b.connected - a.connected]',
  [true, 1],
)

// The independent check: Chrome's own count, after a collection, moves by the same amount as the tally.
const base = await (async () => {
  await chrome.collectGarbage()
  return { chrome: (await chrome.metrics()).JSEventListeners, tally: await chrome.evaluate(total) }
})()
await chrome.evaluate(`window.fns = Array.from({ length: 50 }, () => () => {})
  window.fns.forEach(fn => document.body.addEventListener("click", fn))
  window.fns.forEach(fn => document.body.addEventListener("click", fn))`)
await chrome.collectGarbage()
const added = {
  chrome: (await chrome.metrics()).JSEventListeners - base.chrome,
  tally: (await chrome.evaluate(total)) - base.tally,
}
await chrome.evaluate('window.fns.forEach(fn => document.body.removeEventListener("click", fn))')
await chrome.collectGarbage()
const removed = {
  chrome: (await chrome.metrics()).JSEventListeners - base.chrome,
  tally: (await chrome.evaluate(total)) - base.tally,
}
const agree = added.chrome === 50 && added.tally === 50 && removed.chrome === 0 && removed.tally === 0
if (!agree) failures++
console.log(`${agree ? 'ok  ' : 'FAIL'} 50 listeners each added twice: Chrome and the tally both move by`, added, 'and back to', removed)

await chrome.close()
server.close()
fs.rmSync(dir, { recursive: true, force: true })
console.log(failures ? `\n${failures} failed` : '\nall passed')
process.exit(failures ? 1 : 0)
