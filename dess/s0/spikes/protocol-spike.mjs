// S0 decision 5: does the Layer C protocol of design revision 6 section 8.3 work, and what does it cost?
//
//   node dess/s0/spikes/protocol-spike.mjs [--repetitions 3] [--inject-late-pause] [--out FILE]
//
// A page that is shaped like the product's controller (`protocol/page/product.js`) is driven with real CDP input in a fresh Chrome for
// each repetition, and the frames that carry its commits are found in a trace with the harness's own attribution rule.
//
//   Next:  10 untimed and 30 timed real clicks. The mark is written when the click is delivered, by a harness listener.
//   Play:  10 untimed and 30 timed one-tick cycles at the slowest speed, 1 slice a second. Each is Play, one timer tick and its
//          presentation, then Pause. The mark is written by the harness's scheduler as the timer callback is entered, so it
//          measures scheduled-commit-to-presentation and leaves the interval out. Pause comes after the presentation.
//
// It checks, for every cycle, that exactly one slice was committed, that the epoch advanced, that no timer is pending, and that the
// product's file holds no mark. With `--inject-late-pause` it delays one Pause until a second tick has committed, which has to be
// caught as `invalid_protocol_extra_tick` and has to reject the whole repetition. At the end the three repetitions are given to the
// real comparison, as a baseline and as a candidate, which has to call every metric diagnostic, since three is fewer than five.
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { spawnSync } from 'node:child_process'
import { launch, serve } from '../../../perf/lib/chrome-session.mjs'
import { traced } from '../../../perf/lib/trace.mjs'
import { attribute } from '../../../perf/lib/frames.mjs'
import { press, release, pause } from '../../../perf/lib/input.mjs'
import { compareCollections } from '../../../perf/lib/baselines.mjs'

const here = import.meta.dirname
const root = path.resolve(here, '../../..')
const argument = name => (process.argv.includes(name) ? process.argv[process.argv.indexOf(name) + 1] : null)
const REPETITIONS = Number(argument('--repetitions') ?? 3)
const INJECT = process.argv.includes('--inject-late-pause')
const out = argument('--out')

const PRIMING = 256
const WARMUP = 10
const MEASURED = 30
const INTERVAL_MS = 1000
const PRESENT_WAIT_MS = 150
const QUIET_MS = 1300

const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'dess-protocol-'))
fs.mkdirSync(path.join(dir, 'prototype'))
for (const f of ['index.html', 'product.js']) fs.copyFileSync(path.join(here, 'protocol/page', f), path.join(dir, f))
for (const f of ['engine.mjs', 'generator.mjs', 'splitmix.mjs']) fs.copyFileSync(path.join(root, 'dess/s0/prototype', f), path.join(dir, 'prototype', f))

const repetition = async (number, injectAt) => {
  const { server, port } = await serve(dir, { isolated: true })
  const chrome = await launch()
  const started = performance.now()
  const result = { repetition: number, next: [], play: [], unattributed: {}, failures: [], rejected: null, seconds: {}, pauseAfterPresentation: 0, pauseChecked: 0 }
  try {
    await chrome.send('Page.navigate', { url: `http://127.0.0.1:${port}/` })
    for (let i = 0; i < 100 && !(await chrome.evaluate('window.__dess?.ready === true')); i++) await pause(50)
    const state = () => chrome.evaluate('window.__dess.state()')
    const click = async id => {
      const { x, y } = await chrome.evaluate(`window.__dess.buttonRect(${JSON.stringify(id)})`)
      await press(chrome, x, y)
      await release(chrome, x, y)
    }
    const fail = text => result.failures.push(text)

    // The product's file names no mark, so everything that measures is the harness's.
    // Comments are taken out first, since the file's own comments say why it holds none.
    const source = (await chrome.evaluate('window.__dess.productSource()')).replace(/\/\*[\s\S]*?\*\//g, '').replace(/^\s*\/\/.*$/gm, '')
    if (/performance\.|PerformanceObserver|dess-|__dess/.test(source)) fail('the product file holds measurement code')

    // Next.
    let t0 = performance.now()
    await chrome.evaluate(`window.__dess.reset(${PRIMING})`)
    for (let i = 0; i < WARMUP; i++) {
      await click('next')
      await pause(PRESENT_WAIT_MS)
    }
    const beforeNext = await state()
    const { trace: nextTrace } = await traced(chrome, async () => {
      for (let i = 0; i < MEASURED; i++) {
        await click('next')
        await pause(PRESENT_WAIT_MS)
      }
    })
    const afterNext = await state()
    if (afterNext.cursor !== beforeNext.cursor + MEASURED) fail(`Next moved the cursor from ${beforeNext.cursor} to ${afterNext.cursor}, and ${MEASURED} was meant`)
    if (process.env.DESS_DEBUG) {
      const m = nextTrace.marks.get(`dess-next-${beforeNext.nextMarks + 1}`)
      const near = nextTrace.frames.filter(f => f.pid === m.pid && f.tid === m.tid && f.ts > m.ts - 100000 && f.ts < m.ts + 300000)
      console.log('mark', m.ts, 'frames near it:', near.map(f => ({ at: f.ts - m.ts, dur: f.end - f.ts, style: f.styleAt.map(s => s - m.ts), update: f.updateAt.length })))
    }
    for (let n = beforeNext.nextMarks + 1; n <= afterNext.nextMarks; n++) {
      // A commit made in the handler of a click has its style and layout done in that task, so the frame that shows it has only a
      // paint. The wider qualifier finds that frame, and the harness labels it as the upper bound of a window and not a cause.
      const frame = attribute(nextTrace, `dess-next-${n}`, { boundMs: 250, qualify: 'update' })
      if (frame.status === 'measured') result.nextQualifier = frame.qualify
      if (frame.status === 'measured') result.next.push(frame.latencyMs)
      else result.unattributed[frame.reason.replace(/\d+/g, 'N')] = (result.unattributed[frame.reason.replace(/\d+/g, 'N')] ?? 0) + 1
    }
    result.seconds.next = (performance.now() - t0) / 1000

    // Play: one tick a cycle, at 1 slice a second.
    t0 = performance.now()
    await chrome.evaluate(`window.__dess.reset(${PRIMING}); window.__dess.setInterval(${INTERVAL_MS})`)
    const cycle = async (late) => {
      const before = await state()
      await click('play')
      // Exactly one successful timer-driven slice.
      let at = await state()
      for (let i = 0; i < 400 && at.commits === before.commits; i++) {
        await pause(10)
        at = await state()
      }
      if (at.commits !== before.commits + 1) return `Play did not commit one slice: ${before.commits} then ${at.commits}`
      await pause(late ? INTERVAL_MS + 150 : PRESENT_WAIT_MS)
      await click('pause')
      const after = await state()
      if (after.ticks !== before.ticks + 1) return `invalid_protocol_extra_tick: ${after.ticks - before.ticks} ticks in a cycle, and one was meant`
      if (after.cursor !== before.cursor + 1) return `the cursor moved from ${before.cursor} to ${after.cursor}, and one slice was meant`
      if (after.playing || after.pending !== 0) return `after Pause, playing ${after.playing} and ${after.pending} timers pending`
      if (after.epoch <= before.epoch) return 'the epoch did not advance on Pause'
      return null
    }
    for (let i = 0; i < WARMUP && !result.rejected; i++) {
      const problem = await cycle(false)
      if (problem) result.rejected = problem
    }
    const beforePlay = await state()
    let tracePlay = null
    if (!result.rejected) {
      tracePlay = (
        await traced(chrome, async () => {
          for (let i = 0; i < MEASURED && !result.rejected; i++) {
            const problem = await cycle(injectAt === i)
            if (problem) result.rejected = problem
          }
        })
      ).trace
    }
    const afterPlay = await state()
    if (!result.rejected) {
      const first = beforePlay.tickMarks + 1
      for (let n = first; n <= afterPlay.tickMarks; n++) {
        const frame = attribute(tracePlay, `dess-tick-${n}`, { boundMs: 250 })
        if (frame.status === 'measured') {
          result.play.push(frame.latencyMs)
          // Pause is outside the timed interval: its mark comes after the presentation that is timed.
          const pauseMark = tracePlay.marks.get(`dess-pause-${n - beforePlay.tickMarks + beforePlay.pauseMarks}`)
          if (pauseMark) {
            result.pauseChecked++
            if (pauseMark.startMs > frame.presentedAtPageMs) result.pauseAfterPresentation++
          }
        } else result.unattributed[frame.reason.replace(/\d+/g, 'N')] = (result.unattributed[frame.reason.replace(/\d+/g, 'N')] ?? 0) + 1
      }
      if (afterPlay.cursor !== PRIMING + WARMUP + MEASURED) fail(`the cursor ended at ${afterPlay.cursor}, and ${PRIMING + WARMUP + MEASURED} was meant`)
      // Nothing happens after the last Pause: no commit, no callback, no timer. A cancelled callback that runs anyway is inert.
      await pause(QUIET_MS)
      const quiet = await state()
      if (quiet.ticks !== afterPlay.ticks || quiet.commits !== afterPlay.commits || quiet.pending !== 0) fail('something ran after the last Pause')
      if ((await chrome.evaluate('window.__dess.lateCallbackCommits()')) !== 0) fail('a cancelled callback that ran late committed a slice')
    }
    result.seconds.play = (performance.now() - t0) / 1000
    result.seconds.total = (performance.now() - started) / 1000
    result.browser = (await chrome.evaluate('navigator.userAgent')).match(/Chrome\/([0-9.]+)/)?.[1]
  } finally {
    await chrome.close()
    server.close()
  }
  return result
}

const results = []
for (let r = 0; r < REPETITIONS; r++) {
  const rep = await repetition(r, INJECT && r === 0 ? 4 : -1)
  results.push(rep)
  const q = a => (a.length ? [...a].sort((x, y) => x - y)[Math.floor(a.length / 2)].toFixed(1) : 'n/a')
  console.log(
    `repetition ${r}: ${rep.rejected ? `REJECTED (${rep.rejected})` : 'valid'}; Next ${rep.next.length} observations, median ${q(rep.next)} ms; ` +
      `Play ${rep.play.length} observations, median ${q(rep.play)} ms; unattributed ${JSON.stringify(rep.unattributed)}; ` +
      `Pause after the presentation in ${rep.pauseAfterPresentation} of ${rep.pauseChecked}; ${rep.seconds.total.toFixed(1)} s (Next ${rep.seconds.next.toFixed(1)}, Play ${rep.seconds.play.toFixed(1)})`
  )
  for (const f of rep.failures) console.log(`    FAIL ${f}`)
}

// The three repetitions, as a baseline and as a candidate, through the real comparison.
const valid = results.filter(r => !r.rejected)
let classification = null
if (valid.length === REPETITIONS && !INJECT) {
  const sha = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).stdout.trim()
  const work = fs.mkdtempSync(path.join(os.tmpdir(), 'dess-compare-'))
  const write = (name, withPromotion) => {
    const d = path.join(work, name)
    fs.mkdirSync(d)
    for (const rep of valid) {
      const id = `w6-smoke-C-performance-r${rep.repetition}`
      const samples = `${id}.samples.ndjson`
      fs.writeFileSync(path.join(d, samples), [['w6.next.input_to_presentation', rep.next], ['w6.play.timer_to_presentation', rep.play]].map(([metric, values]) => JSON.stringify({ metric, values })).join('\n') + '\n')
      fs.writeFileSync(
        path.join(d, `${id}.json`),
        JSON.stringify({
          schema_version: 1, run_id: id, repetition: rep.repetition,
          environment: { kind: 'browser', browser: 'Chrome', browser_version: rep.browser, os: os.type(), cpu: os.cpus()[0].model, runner_class: 'local', protocol_version: '1', git_sha: sha, git_dirty: false },
          manifest: { id: 'w6-smoke', sha256: 'a'.repeat(64), operation_stream_sha256: 'b'.repeat(64) },
          mode: 'performance', status: 'ok',
          metrics: [{ name: 'w6.next.input_to_presentation', layer: 'C', status: 'measured' }, { name: 'w6.play.timer_to_presentation', layer: 'C', status: 'measured' }],
          raw_samples: samples
        })
      )
    }
    if (withPromotion)
      fs.writeFileSync(path.join(d, 'promotion.json'), JSON.stringify({ promotion: { promoted_at: new Date().toISOString(), commit: sha, workload_hash: 'a'.repeat(64), environment_key: 'spike', method_version: '1' } }))
    return d
  }
  const baseline = write('baseline', true)
  const candidate = write('candidate', false)
  const report = compareCollections({ baselineDir: baseline, candidateDirs: [candidate], layer: 'C', mode: 'performance', out: path.join(work, 'comparison.json') })
  classification = Object.fromEntries(report.comparisons.map(c => [`${c.metric} ${c.statistic}`, { verdict: c.verdict, has_interval: c.ratio_interval !== undefined, ratio: c.ratio }]))
  console.log('\nthe three repetitions as a baseline and a candidate, through the real comparison:')
  for (const [name, c] of Object.entries(classification)) console.log(`  ${name}: ${c.verdict}${c.has_interval ? ' WITH AN INTERVAL' : ', no interval'}, ratio ${c.ratio?.toFixed(3)}`)
  console.log(`  reason: ${report.comparisons[0].reason}`)
  fs.rmSync(work, { recursive: true, force: true })
}
fs.rmSync(dir, { recursive: true, force: true })

const record = { repetitions: REPETITIONS, injected_late_pause: INJECT, interval_ms: INTERVAL_MS, results, classification }
if (out) fs.writeFileSync(out, JSON.stringify(record, null, 2) + '\n')
const bad = INJECT ? !(results[0].rejected?.startsWith('invalid_protocol_extra_tick')) : results.some(r => r.rejected || r.failures.length)
process.exit(bad ? 1 : 0)
