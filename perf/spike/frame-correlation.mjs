// Phase 3 spike for the performance harness (design-specs/performance_harness.pdf, sections 3.2 and 10).
//
//   node perf/spike/frame-correlation.mjs [--out FILE]
//
// Phase 3 is conditional on showing that the correlation rule of section 3.2 attributes frames correctly on a known workload.
// The page keeps its own stamps of the frame callbacks that carry its mutations, which is an answer that does not come from
// the trace. Each sample's attributed frame is then checked against it. The rule is judged by how often it is right where it
// says anything, and by whether it says nothing where it should.
//
//   1. Known cost, one mutation at a time. Each frame carries a frame callback that spins for a known time.
//   2. The same with unrelated work making frames of its own all the while.
//   3. Mutations made back to back, so that earlier frames are still on their way. The rule is run beside the naive one, the
//      next presentation after the mark, to show what the rule is for.
//   4. Where it must say nothing: a mutation that changes nothing, a trace whose buffer is too small, and a mark that is lost.
//   5. A change whose layout is forced in its own task, which has no style step left for the frame that shows it. The style rule
//      says nothing, and the wider qualifier is held to the same stamps.
//   6. Real input from `Input.dispatchMouseEvent`, delivered at once and deferred to a frame, to find where the browser puts
//      the input in relation to the frame, since the rule has to hold for it.
//
// Set CHROME to the path of a Chrome binary if it is not the default macOS install. It needs Node 22 or later.
import fs from 'node:fs'
import path from 'node:path'
import { launch, serve } from '../lib/chrome-session.mjs'
import { attribute, toPageMs } from '../lib/frames.mjs'
import { traced } from '../lib/trace.mjs'

const root = path.resolve(import.meta.dirname, '../..')
const outIndex = process.argv.indexOf('--out')
const out = outIndex > 0 ? process.argv[outIndex + 1] : null
const results = { chrome: null, experiments: {} }
const log = []
const say = text => {
  log.push(text)
  console.log(text)
}

const { server, port } = await serve(path.join(root, 'perf/spike/frame-correlation'))
const chrome = await launch()
results.chrome = (await chrome.send('Browser.getVersion')).result?.product ?? null
const open = async () => {
  await chrome.send('Page.navigate', { url: `http://localhost:${port}/` })
  await new Promise(resolve => setTimeout(resolve, 400))
}
const pause = ms => new Promise(resolve => setTimeout(resolve, ms))

// The frame that the page's own stamps say carried a sample: the one whose span holds its frame callback.
const holds = (trace, name, attributed, record) => {
  const began = toPageMs(trace, name, attributed.frame.beganAt)
  const ended = toPageMs(trace, name, attributed.frame.endedAt)
  return began <= record.rafStart + 0.25 && record.rafEnd <= ended + 0.25
}

// The page's clock is coarsened to 0.1 ms, so a stamp may be that much early or late against the trace.
// What fraction of samples were given a frame, how many of those frames hold the callback, and how long the frames were.
const judge = (label, trace, records, prefix = 'perf-') => {
  let measured = 0
  let right = 0
  let presentedAfter = 0
  const unavailable = {}
  const wrong = []
  for (const record of records) {
    const name = `${prefix}${record.index}`
    const result = attribute(trace, name, { boundMs: 100 })
    if (result.status !== 'measured') {
      unavailable[result.reason] = (unavailable[result.reason] ?? 0) + 1
      continue
    }
    measured++
    if (holds(trace, name, result, record)) right++
    else wrong.push(record.index)
    if (toPageMs(trace, name, result.presentedAt) >= record.rafEnd) presentedAfter++
  }
  const summary = { label, samples: records.length, measured, right, presented_after_callback: presentedAfter, unavailable, wrong_indices: wrong }
  say(`${label}: ${records.length} samples, ${measured} given a frame, ${right} of those hold the sample's own callback, ${presentedAfter} presented after it ends${Object.keys(unavailable).length ? `, unavailable: ${JSON.stringify(unavailable)}` : ''}`)
  return summary
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
say(`Chrome ${results.chrome}\n`)
const costs = [0, 2, 5, 9, 14, 20, 30]
const plan = Array.from({ length: 70 }, (_, index) => ({ index, cost: costs[index % costs.length] }))

await open()
{
  const { result: records, trace } = await traced(chrome, () => chrome.evaluate(`(async()=>{const r=[];for(const s of ${JSON.stringify(plan)}){r.push(await spike.mutate(s.index,s.cost))}return r})()`))
  results.experiments.known_cost = judge('1 known cost, one at a time', trace, records)
  const durations = records.map(record => {
    const result = attribute(trace, `perf-${record.index}`, { boundMs: 100 })
    return result.status === 'measured' ? { cost: record.cost, latency: result.latencyMs } : null
  }).filter(Boolean)
  // A frame carrying a callback that spins for `cost` cannot be presented sooner than that after the mark.
  const tooSoon = durations.filter(d => d.latency < d.cost)
  say(`  none presented sooner than the callback's own cost: ${tooSoon.length === 0}; latency by cost (ms): ${costs.map(cost => { const l = durations.filter(d => d.cost === cost).map(d => d.latency); return `${cost}→${(l.reduce((a, b) => a + b, 0) / l.length).toFixed(1)}` }).join(' ')}`)
  results.experiments.known_cost.presented_sooner_than_cost = tooSoon.length
}

await open()
await chrome.evaluate('spike.startUnrelated(5)')
{
  const { result: records, trace } = await traced(chrome, () => chrome.evaluate(`(async()=>{const r=[];for(const s of ${JSON.stringify(plan)}){r.push(await spike.mutate(s.index,s.cost))}return r})()`))
  results.experiments.unrelated_work = judge('2 unrelated work making frames at 5 ms', trace, records)
}
await chrome.evaluate('spike.stopUnrelated()')

await open()
{
  // Back to back: a new mutation every 6 ms, with the frames of the earlier ones still in flight.
  const { result: records, trace } = await traced(chrome, () => chrome.evaluate(`(async()=>{const r=[];for(let i=0;i<80;i++){r.push(spike.mutateNow(i,i%3===0?6:0));await new Promise(f=>setTimeout(f,6))}await new Promise(f=>setTimeout(f,120));return r})()`))
  results.experiments.back_to_back = judge('3 back to back, 6 ms apart', trace, records)
  // The naive rule: the first presentation after the mark. Its answer is judged against the same stamps.
  let naiveRight = 0
  let naiveMeasured = 0
  for (const record of records) {
    const mark = trace.marks.get(`perf-${record.index}`)
    const next = trace.pipelines.filter(p => p.pid === mark?.pid && /PRESENTED/.test(p.state ?? '') && p.end > mark.ts).sort((a, b) => a.end - b.end)[0]
    const frame = next && trace.frames.find(f => f.id === next.sequence && f.pid === next.pid)
    if (!frame) continue
    naiveMeasured++
    const began = toPageMs(trace, `perf-${record.index}`, frame.ts)
    const ended = toPageMs(trace, `perf-${record.index}`, frame.end)
    if (began <= record.rafStart + 0.25 && record.rafEnd <= ended + 0.25) naiveRight++
  }
  results.experiments.back_to_back.naive = { measured: naiveMeasured, right: naiveRight }
  say(`  the naive rule, the next presentation after the mark: ${naiveMeasured} given a frame, ${naiveRight} of them right`)
}

await open()
{
  const { result: records, trace } = await traced(chrome, () => chrome.evaluate(`(async()=>{const r=[];for(let i=0;i<20;i++){r.push(await spike.noop(i))}return r})()`))
  const measured = records.filter(record => attribute(trace, `perf-${record.index}`, { boundMs: 100 }).status === 'measured').length
  results.experiments.changes_nothing = { samples: records.length, measured }
  say(`4a a mutation that changes nothing: ${records.length} samples, ${measured} given a frame (it should be none)`)
}
await open()
{
  const { result: records, trace, dataLossOccurred } = await traced(chrome, () => chrome.evaluate(`(async()=>{const r=[];for(let i=0;i<70;i++){r.push(await spike.mutate(i,2))}return r})()`), { bufferKb: 16 })
  const summary = judge('4b a trace with a 16 KB buffer', trace, records)
  results.experiments.small_buffer = { ...summary, data_loss_reported: dataLossOccurred }
  say(`  Chrome reported data loss: ${dataLossOccurred}; the marks that survived: ${trace.marks.size} of ${records.length}`)
}

await open()
{
  // The style and layout are forced in the task, as when a panel is built and measured, so only the frame's paint is left.
  const { result: records, trace } = await traced(chrome, () => chrome.evaluate(`(async()=>{const r=[];for(let i=0;i<40;i++){r.push(await spike.mutateForced(i))}return r})()`))
  const strict = records.filter(record => attribute(trace, `perf-${record.index}`, { boundMs: 100 }).status === 'measured').length
  let wide = 0
  let wideRight = 0
  for (const record of records) {
    const result = attribute(trace, `perf-${record.index}`, { boundMs: 100, qualify: 'update' })
    if (result.status !== 'measured') continue
    wide++
    if (holds(trace, `perf-${record.index}`, result, record)) wideRight++
  }
  results.experiments.forced_layout = { samples: records.length, strict_measured: strict, update_measured: wide, update_right: wideRight }
  say(`6 layout forced in the task: ${records.length} samples, ${strict} given a frame by the style rule, ${wide} by the wider rule, of which ${wideRight} hold the callback`)
}

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
// Real input. The page's own stamp is taken in the first listener, and CDP's round trip is kept apart from it.
for (const deferred of [false, true]) {
  await open()
  await chrome.evaluate(`spike.armInput(${deferred})`)
  const roundTrips = []
  const { trace } = await traced(chrome, async () => {
    for (let i = 0; i < 40; i++) {
      const sent = performance.now()
      await chrome.send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: 100 + i, y: 100 })
      roundTrips.push(performance.now() - sent)
      await pause(45)
    }
  })
  const records = await chrome.evaluate('spike.log')
  const label = `5 real input, ${deferred ? 'deferred to the frame' : 'written at once'}`
  const summary = judge(label, trace, records, 'perf-in-')
  let inside = 0
  let before = 0
  for (const record of records) {
    const mark = trace.marks.get(`perf-in-${record.index}`)
    const frame = mark && trace.frames.find(f => f.pid === mark.pid && f.ts <= mark.ts && mark.ts <= f.end)
    if (frame) inside++
    else before++
  }
  summary.mark_inside_a_frame = inside
  summary.mark_before_a_frame = before
  summary.cdp_round_trip_ms_median = [...roundTrips].sort((a, b) => a - b)[roundTrips.length >> 1]
  results.experiments[deferred ? 'input_deferred' : 'input_immediate'] = summary
  say(`  the mark was inside a main frame ${inside} times and before one ${before} times; CDP round trip median ${summary.cdp_round_trip_ms_median.toFixed(2)} ms`)
}

await chrome.close()
server.close()

// - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
const e = results.experiments
const verdicts = [
  [e.known_cost.measured === e.known_cost.samples && e.known_cost.right === e.known_cost.measured, 'one at a time, every sample is given the frame that holds its callback'],
  [e.unrelated_work.measured > 0 && e.unrelated_work.right === e.unrelated_work.measured, 'with unrelated work, every sample that is given a frame is given the right one'],
  [e.back_to_back.measured > 0 && e.back_to_back.right === e.back_to_back.measured, 'back to back, every sample that is given a frame is given the right one'],
  [e.back_to_back.naive.right < e.back_to_back.naive.measured, 'the naive rule is wrong back to back, which is what the rule is for'],
  [e.changes_nothing.measured === 0, 'a mutation that changes nothing is never given a frame'],
  [e.small_buffer.right === e.small_buffer.measured, 'a trace that lost events gives no wrong frame'],
  [e.forced_layout.strict_measured < e.forced_layout.samples && e.forced_layout.update_measured >= 0.9 * e.forced_layout.samples && e.forced_layout.update_right === e.forced_layout.update_measured, 'a change with its layout forced in the task is missed by the style rule and placed right by the wider one'],
  [e.input_immediate.measured > 0 && e.input_immediate.right === e.input_immediate.measured, 'real input written at once is given the right frame'],
  [e.input_deferred.measured > 0 && e.input_deferred.right === e.input_deferred.measured, 'real input deferred to a frame is given the right frame'],
]
say('')
let failed = 0
for (const [ok, text] of verdicts) {
  if (!ok) failed++
  say(`${ok ? 'ok  ' : 'FAIL'} ${text}`)
}
say(failed ? `\n${failed} did not hold: the rule must not be relied on` : '\nthe rule attributes frames correctly wherever it gives an answer')
results.verdicts = verdicts.map(([ok, text]) => ({ ok, text }))
if (out) {
  fs.mkdirSync(path.dirname(out), { recursive: true })
  fs.writeFileSync(out, JSON.stringify(results, null, 2) + '\n')
  fs.writeFileSync(out.replace(/\.json$/, '.txt'), log.join('\n') + '\n')
}
process.exit(failed ? 1 : 0)
