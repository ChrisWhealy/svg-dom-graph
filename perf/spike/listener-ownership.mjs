// Finds out who owns the listeners that a scene registers, with the effective-registration tracker.
//
// Phase 0 saw a second scene add 44 listeners and remove 44, so the count did not move. This builds the fixture's scene
// twice with the tracker logging, and says which registrations the second build removed: its own, or the first build's.
//
//   node perf/spike/listener-ownership.mjs cdp-test-fixture
import fs from 'node:fs'
import { serve, launch } from '../lib/chrome-session.mjs'

const dir = process.argv[2]
if (!dir) {
  console.error('usage: node listener-ownership.mjs <directory with index.html>')
  process.exit(2)
}
const { server, port } = await serve(dir)
const chrome = await launch()
const tracker = fs.readFileSync(new URL('../page/listener-tracker.js', import.meta.url), 'utf8')
await chrome.send('Page.addScriptToEvaluateOnNewDocument', {
  source: `${tracker}\nwindow.__listeners.startLogging()`
})
await chrome.send('Page.navigate', { url: `http://127.0.0.1:${port}/` })
await new Promise(resolve => setTimeout(resolve, 2500))

const afterFirst = await chrome.evaluate(
  '({ snapshot: window.__listeners.snapshot(), sequence: window.__listeners.sequence(), logLength: window.__listeners.log.length })'
)
await chrome.evaluate('window.fixture.run()')
const afterSecond = await chrome.evaluate(
  '({ snapshot: window.__listeners.snapshot(), sequence: window.__listeners.sequence(), logLength: window.__listeners.log.length })'
)
const log = await chrome.evaluate('window.__listeners.log')

const first = log.slice(0, afterFirst.logLength)
const second = log.slice(afterFirst.logLength)
const count = (entries, key) =>
  entries.reduce((tally, entry) => ({ ...tally, [key(entry)]: (tally[key(entry)] ?? 0) + 1 }), {})
const bornIn = entry => (entry.born < afterFirst.sequence ? 'the first build' : 'the second build')

console.log('after the first build: ', JSON.stringify(afterFirst.snapshot))
console.log('after the second build:', JSON.stringify(afterSecond.snapshot))
console.log(
  '\nfirst build, by operation:',
  count(first, e => e.op)
)
console.log(
  'second build, by operation:',
  count(second, e => e.op)
)
console.log('\nsecond build, removals by who registered what was removed:')
console.log(
  count(
    second.filter(e => e.op === 'remove' || e.op === 'once-fired' || e.op === 'aborted'),
    e => `${e.op} -> ${bornIn(e)}`
  )
)
console.log('\nsecond build, removals by target and type:')
console.log(
  count(
    second.filter(e => e.op === 'remove'),
    e => `${e.kind} ${e.type}${e.capture ? ' (capture)' : ''}`
  )
)
console.log('\nregistrations that survive, by target and type:')
console.log(afterSecond.snapshot.byKind, afterSecond.snapshot.byType)
const firstRemoval = second.find(e => e.op === 'remove')
console.log('\none removal in the second build, with its call stack:')
console.log(JSON.stringify(firstRemoval, null, 2))
const firstAdd = second.find(e => e.op === 'add')
console.log('\none addition in the second build, with its call stack:')
console.log(JSON.stringify(firstAdd, null, 2))

await chrome.close()
server.close()
process.exit(0)
