// S0 decision 3: how a panel in the gallery is known to be hidden, and what asking costs.
//
//   node dess/s0/spikes/predicate-bench.mjs [--out FILE]
//
// The gallery (`demo/index.template.html`) keeps every panel in one document as a `<div class="section">`, which is shown by the
// rule `.section.active { display: block }` and hidden by `.section { display: none }`. `selectDemo` toggles the class in the
// handler of a click, and nothing else tells a panel that it was hidden. This page has the same markup and the same two rules,
// with an SVG in each panel, and it answers three questions in Chrome:
//
//   1. Is each candidate predicate right when the panel is shown and when it is hidden?
//   2. What does one call cost, with nothing waiting to be recalculated and with an SVG write just made?
//   3. In what order do a timer, an animation frame and each kind of observer run after the class is toggled?
//
// The third decides what a hint can promise. A MutationObserver runs as a microtask at the end of the task that changed the
// class, so no timer callback can run between the change and the hint. An IntersectionObserver runs at the next rendering
// opportunity, which comes after a timer that was already queued.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { launch, serve } from '../../../perf/lib/chrome-session.mjs'

const out = process.argv.includes('--out') ? process.argv[process.argv.indexOf('--out') + 1] : null

const page = `<!doctype html><meta charset="utf-8"><title>gallery predicate</title>
<style>
  #stage { position: relative; }
  .section { display: none; }
  .section.active { display: block; }
</style>
<nav id="menu"></nav>
<div id="stage">
  ${['a', 'b', 'c', 'd']
    .map(
      id => `<div class="section" id="${id}"><h2>${id}</h2>
    <svg id="svg-${id}" width="400" height="260"><g class="svg-dom-graph-content">${Array.from({ length: 60 }, (_, i) => `<rect x="${(i % 10) * 30}" y="${Math.floor(i / 10) * 30}" width="24" height="24"/>`).join('')}</g></svg></div>`
    )
    .join('\n  ')}
</div>
<script>
  // The same toggle as selectDemo in the gallery.
  const select = target => { for (const s of document.querySelectorAll('#stage .section')) s.classList.toggle('active', s.id === target) }
  window.select = select
  select('a')
</script>`

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'dess-predicate-'))
fs.writeFileSync(path.join(dir, 'index.html'), page)
const { server, port } = await serve(dir, { isolated: true })
const chrome = await launch()
await chrome.send('Page.navigate', { url: `http://127.0.0.1:${port}/` })
await new Promise(resolve => setTimeout(resolve, 600))

const result = await chrome.evaluate(`(async () => {
  const section = id => document.getElementById(id)
  const probe = (id) => document.querySelector('#svg-' + id + ' rect')
  const predicates = {
    'cached flag (a variable the hint sets)': () => window.__flag,
    'isConnected': el => el.isConnected,
    'ancestor has class active': el => el.closest('.section').classList.contains('active'),
    "closest('.section:not(.active)') is null": el => el.closest('.section:not(.active)') === null,
    'checkVisibility()': el => el.checkVisibility(),
    'getClientRects().length > 0': el => el.getClientRects().length > 0,
    'getBoundingClientRect().width > 0': el => el.getBoundingClientRect().width > 0,
  }

  // 1. Right answers.
  const truth = {}
  for (const state of ['shown', 'hidden']) {
    select(state === 'shown' ? 'a' : 'b')
    const el = probe('a')
    window.__flag = state === 'shown'
    truth[state] = {}
    for (const [name, fn] of Object.entries(predicates)) truth[state][name] = fn(el)
  }
  select('a')
  window.__flag = true

  // 2. Cost. Each loop is timed whole, and a loop that makes the write and calls nothing is taken away.
  const el = probe('a')
  const N = 4000
  const loop = (write, call) => {
    const start = performance.now()
    for (let i = 0; i < N; i++) { if (write) el.setAttribute('x', String(i % 7)); if (call) call(el) }
    return (performance.now() - start) * 1000 / N
  }
  const cost = {}
  for (const [name, fn] of Object.entries(predicates)) {
    loop(false, fn); loop(true, fn) // warm up
    const best = (write) => Math.min(...Array.from({ length: 5 }, () => loop(write, fn)))
    const writeOnly = Math.min(...Array.from({ length: 5 }, () => loop(true, null)))
    cost[name] = { clean_us: +best(false).toFixed(3), after_an_svg_write_us: +(best(true) - writeOnly).toFixed(3) }
  }

  // 3. Order after the class is toggled, in one task.
  const order = async (mode) => {
    select('a')
    await new Promise(r => requestAnimationFrame(() => setTimeout(r, 50)))
    const log = []
    let observer
    const done = new Promise(r => setTimeout(r, 200))
    if (mode === 'mutation') {
      observer = new MutationObserver(() => log.push('mutation observer'))
      observer.observe(document.getElementById('stage'), { subtree: true, attributes: true, attributeFilter: ['class'] })
    } else if (mode === 'intersection') {
      observer = new IntersectionObserver(() => log.push('intersection observer'))
      observer.observe(document.getElementById('a'))
      await new Promise(r => setTimeout(r, 100)) // the first delivery, for the initial state
      log.length = 0
    }
    setTimeout(() => log.push('timer queued before the toggle'), 0)
    requestAnimationFrame(() => log.push('animation frame'))
    Promise.resolve().then(() => log.push('microtask queued before the toggle'))
    select('b') // the toggle
    log.push('end of the task that toggled')
    await done
    observer?.disconnect()
    return log
  }
  const orders = { none: await order('none'), mutation: await order('mutation'), intersection: await order('intersection') }
  return { truth, cost, orders, chrome: navigator.userAgent.match(/Chrome\\/([0-9.]+)/)?.[1], crossOriginIsolated, hasCheckVisibility: typeof Element.prototype.checkVisibility }
})()`)

await chrome.close()
server.close()
fs.rmSync(dir, { recursive: true, force: true })

console.log(
  `Chrome ${result.chrome}, cross-origin isolated ${result.crossOriginIsolated}, checkVisibility is a ${result.hasCheckVisibility}\n`
)
console.log('Right answers (the panel is shown, then hidden):')
for (const name of Object.keys(result.truth.shown))
  console.log(
    `  ${name.padEnd(44)} shown ${String(result.truth.shown[name]).padEnd(6)} hidden ${result.truth.hidden[name]}`
  )
console.log('\nCost of one call, in microseconds (best of 5 loops of 4,000):')
console.log(`  ${'predicate'.padEnd(44)} ${'clean'.padStart(9)} ${'after an SVG write'.padStart(20)}`)
for (const [name, c] of Object.entries(result.cost))
  console.log(
    `  ${name.padEnd(44)} ${c.clean_us.toFixed(3).padStart(9)} ${c.after_an_svg_write_us.toFixed(3).padStart(20)}`
  )
console.log('\nOrder after the class is toggled in one task:')
for (const [mode, log] of Object.entries(result.orders)) console.log(`  ${mode.padEnd(13)} ${log.join('  ->  ')}`)
if (out) fs.writeFileSync(out, JSON.stringify(result, null, 2) + '\n')
