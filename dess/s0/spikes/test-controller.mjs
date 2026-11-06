// Tests the stand-in controller of the protocol spike for the races that its Play and Pause cycles cannot reach.
//
//   node dess/s0/spikes/test-controller.mjs
//
// The spike's own late-callback check runs after playback has stopped. These run a cancelled callback while a newer playback is
// live, which is the case that a stale callback can damage: it must not touch the handle of the timer that is pending now. They use
// the scheduler's held-callback test mode, which keeps a cancelled callback and runs it on request. It needs Chrome.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { launch, serve } from '../../../perf/lib/chrome-session.mjs'
import { pause } from '../../../perf/lib/input.mjs'

const here = import.meta.dirname
const root = path.resolve(here, '../../..')
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'dess-controller-'))
fs.mkdirSync(path.join(dir, 'prototype'))
for (const f of ['index.html', 'product.js']) fs.copyFileSync(path.join(here, 'protocol/page', f), path.join(dir, f))
for (const f of ['engine.mjs', 'generator.mjs', 'splitmix.mjs'])
  fs.copyFileSync(path.join(root, 'dess/s0/prototype', f), path.join(dir, 'prototype', f))

let failures = 0
const check = (ok, text) => {
  if (!ok) failures++
  console.log(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}

const { server, port } = await serve(dir)
const chrome = await launch()
try {
  await chrome.send('Page.navigate', { url: `http://127.0.0.1:${port}/` })
  for (let i = 0; i < 100 && !(await chrome.evaluate('window.__dess?.ready === true')); i++) await pause(50)
  const state = () => chrome.evaluate('window.__dess.state()')
  const run = code => chrome.evaluate(code)
  await run('window.__dess.reset(256); window.__dess.setInterval(60000)') // no tick can fire by itself in the test

  // Play, Pause, Play, and then the callback of the first Play is forced to run.
  await run('window.__dess.holdCancelled(true)')
  await run('window.__dess.play()') //  H1
  await run('window.__dess.pause()') //  cancels H1, and the epoch advances
  await run('window.__dess.play()') //  H2
  const before = await state()
  check(
    before.pending === 1 && before.playing,
    `two generations: H2 is pending after Play, Pause, Play (${before.pending} pending)`
  )
  const ran = await run('window.__dess.runHeld()')
  const after = await state()
  check(ran === 1, `the cancelled callback of H1 was kept, and ran (${ran})`)
  check(
    after.commits === before.commits && after.ticks === before.ticks,
    'and it committed nothing and counted no tick'
  )
  check(after.pending === 1, `H2 is still pending after the stale callback ran (${after.pending} pending)`)
  await run('window.__dess.pause()')
  const stopped = await state()
  check(
    stopped.pending === 0 && !stopped.playing,
    `Pause cancels H2 at once, so nothing is left pending (${stopped.pending} pending)`
  )

  // The case that the spike already had: the callback runs after the playback it belonged to has stopped.
  await run('window.__dess.holdCancelled(true)')
  await run('window.__dess.play()')
  await run('window.__dess.pause()')
  const late = await state()
  await run('window.__dess.runHeld()')
  const lateAfter = await state()
  check(
    lateAfter.commits === late.commits && lateAfter.pending === 0,
    'a cancelled callback that runs after a stop commits nothing and leaves nothing pending'
  )
  await run('window.__dess.holdCancelled(false)')
} finally {
  await chrome.close()
  server.close()
  fs.rmSync(dir, { recursive: true, force: true })
}
console.log(
  failures ? `\n${failures} did not behave` : '\nthe controller survives a stale callback from an earlier playback'
)
process.exit(failures ? 1 : 0)
